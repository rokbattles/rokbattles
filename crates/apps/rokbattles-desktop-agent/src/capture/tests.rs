use super::*;
use rokbattles_capture_ipc::{PacketBytes, UnavailableReason, write_record};
use rokbattles_capture_runtime::packet::{ClientTcpControl, FlowKey, SocketEstablishedEvidence};
use std::sync::Mutex;

struct Permission(AtomicBool);
impl Gate for Permission {
    async fn allowed(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Default)]
struct State {
    starts: usize,
    events: Vec<Event>,
    ended: bool,
    fail: bool,
    open: usize,
}
struct MockUpload(Arc<Mutex<State>>);
impl Upload for MockUpload {
    fn event(&mut self, event: Event) -> Result<(), Failure> {
        let mut state = self.0.lock().map_err(|_error| Failure::Lost)?;
        if state.fail {
            return Err(Failure::Lost);
        }
        match event {
            Event::Open { .. } => state.open += 1,
            Event::Close { .. } | Event::Abort { .. } => state.open -= 1,
            _ => {}
        }
        state.events.push(event);
        Ok(())
    }
    fn ended(&self) -> bool {
        self.0.lock().expect("state").ended
    }
    fn idle(&self) -> bool {
        self.0.lock().expect("state").open == 0
    }
    async fn finish(self) -> Result<(), Failure> {
        Ok(())
    }
}
type TestPipeline = Pipeline<MockUpload, Box<dyn FnMut() -> Result<MockUpload, Failure>>>;

fn setup() -> (TestPipeline, Arc<Mutex<State>>, Permission) {
    let state = Arc::new(Mutex::new(State::default()));
    let shared = Arc::clone(&state);
    let factory: Box<dyn FnMut() -> Result<MockUpload, Failure>> = Box::new(move || {
        shared.lock().expect("state").starts += 1;
        Ok(MockUpload(Arc::clone(&shared)))
    });
    (Pipeline::new(factory), state, Permission(AtomicBool::new(true)))
}
fn key() -> FlowKey {
    FlowKey {
        client: "10.0.0.2:40000".parse().expect("client"),
        server: "10.0.0.1:3101".parse().expect("server"),
    }
}
fn control(sequence: u32, acknowledgement: u32, flags: u8) -> Record {
    Record::ClientControl(ClientTcpControl { key: key(), sequence, acknowledgement, flags })
}
fn server(sequence: u32, acknowledgement: u32, flags: u8, payload: &[u8]) -> Record {
    let mut bytes = vec![0u8; 40 + payload.len()];
    bytes[0] = 0x45;
    let size = (bytes.len() as u16).to_be_bytes();
    bytes[2..4].copy_from_slice(&size);
    bytes[9] = 6;
    bytes[12..16].copy_from_slice(&[10, 0, 0, 1]);
    bytes[16..20].copy_from_slice(&[10, 0, 0, 2]);
    bytes[20..22].copy_from_slice(&3101u16.to_be_bytes());
    bytes[22..24].copy_from_slice(&40000u16.to_be_bytes());
    bytes[24..28].copy_from_slice(&sequence.to_be_bytes());
    bytes[28..32].copy_from_slice(&acknowledgement.to_be_bytes());
    bytes[32] = 5 << 4;
    bytes[33] = flags;
    bytes[40..].copy_from_slice(payload);
    Record::ServerPacket(PacketBytes::new(bytes))
}
fn native_backend() -> Backend {
    if cfg!(all(windows, target_arch = "x86_64")) { Backend::WinDivert } else { Backend::Pcap }
}
async fn established<U: Upload, F: FnMut() -> Result<U, Failure>>(
    pipeline: &mut Pipeline<U, F>,
    gate: &impl Gate,
) {
    pipeline
        .record(Record::Started(native_backend()), Duration::ZERO, gate)
        .await
        .expect("started");
    if permits_controls(native_backend()) {
        pipeline.record(control(10, 0, 2), Duration::ZERO, gate).await.expect("syn");
        pipeline.record(server(100, 11, 0x12, b""), Duration::ZERO, gate).await.expect("synack");
        pipeline.record(control(11, 101, 0x10), Duration::ZERO, gate).await.expect("ack");
    } else {
        pipeline
            .record(
                Record::SocketEstablished(SocketEstablishedEvidence {
                    key: key(),
                    client_initial_sequence: 10,
                    server_initial_sequence: 100,
                    generation: 1,
                }),
                Duration::ZERO,
                gate,
            )
            .await
            .expect("socket evidence");
    }
}

#[tokio::test]
async fn first_open_and_server_data_are_ordered_and_client_metadata_stays_local() {
    let (mut pipeline, state, gate) = setup();
    established(&mut pipeline, &gate).await;
    pipeline
        .record(server(101, 11, 0x10, b"server fixture"), Duration::from_secs(1), &gate)
        .await
        .expect("data");
    let state = state.lock().expect("state");
    assert_eq!(state.starts, 1);
    assert!(matches!(state.events.first(), Some(Event::Open { server_port: 3101, .. })));
    assert!(
        matches!(state.events.get(1),Some(Event::Data{offset:0,bytes,..}) if bytes == b"server fixture")
    );
    assert_eq!(state.events.len(), 2);
}

#[tokio::test]
async fn revocation_blocks_first_open_and_every_later_event() {
    let (mut pipeline, state, gate) = setup();
    gate.0.store(false, Ordering::Release);
    assert_eq!(
        pipeline.events(vec![Event::Open { id: 1, server_port: 3101 }], &gate).await,
        Err(Failure::Revoked)
    );
    assert_eq!(state.lock().expect("state").starts, 0);
    gate.0.store(true, Ordering::Release);
    pipeline.events(vec![Event::Open { id: 1, server_port: 3101 }], &gate).await.expect("open");
    gate.0.store(false, Ordering::Release);
    assert_eq!(
        pipeline
            .events(vec![Event::Data { id: 1, offset: 0, bytes: b"discard".to_vec() }], &gate)
            .await,
        Err(Failure::Revoked)
    );
    assert_eq!(state.lock().expect("state").events.len(), 1);
}

#[tokio::test]
async fn strict_start_backend_and_transport_failure_boundaries() {
    let (mut pipeline, _, gate) = setup();
    assert_eq!(pipeline.record(Record::Keepalive, Duration::ZERO, &gate).await, Err(Failure::Lost));
    pipeline.record(Record::Started(native_backend()), Duration::ZERO, &gate).await.expect("start");
    assert_eq!(
        pipeline.record(Record::Started(native_backend()), Duration::ZERO, &gate).await,
        Err(Failure::Lost)
    );
    assert_eq!(pipeline.record(Record::Gap, Duration::ZERO, &gate).await, Err(Failure::Lost));
    let wrong = if permits_controls(native_backend()) {
        Record::SocketEstablished(SocketEstablishedEvidence {
            key: key(),
            client_initial_sequence: 1,
            server_initial_sequence: 2,
            generation: 1,
        })
    } else {
        control(1, 0, 2)
    };
    assert_eq!(pipeline.record(wrong, Duration::ZERO, &gate).await, Err(Failure::Lost));
    assert_eq!(
        pipeline
            .record(Record::Unavailable(UnavailableReason::NativeBackend), Duration::ZERO, &gate)
            .await,
        Err(Failure::Unavailable)
    );
}

#[tokio::test]
async fn early_http_response_and_queue_failure_require_a_new_pipeline() {
    let (mut pipeline, state, gate) = setup();
    established(&mut pipeline, &gate).await;
    state.lock().expect("state").ended = true;
    assert_eq!(pipeline.tick(Duration::from_secs(1), &gate).await, Err(Failure::Lost));
    let (mut pipeline, state, gate) = setup();
    established(&mut pipeline, &gate).await;
    state.lock().expect("state").fail = true;
    assert_eq!(
        pipeline.record(server(101, 11, 0x10, b"fixture"), Duration::from_secs(1), &gate).await,
        Err(Failure::Lost)
    );
}

#[tokio::test]
async fn reader_overflow_and_truncated_frames_are_fatal_without_reparsing() {
    let (mut writer, input) = tokio::io::duplex(8192);
    let reader = Reader::start(input);
    for _ in 0..IPC_QUEUE + 1 {
        write_record(&mut writer, &Record::Keepalive).await.expect("write");
    }
    tokio::time::timeout(Duration::from_secs(1), async {
        while !reader.failed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("overflow");
    drop(reader);
    let (mut writer, input) = tokio::io::duplex(128);
    let mut reader = Reader::start(input);
    use tokio::io::AsyncWriteExt;
    writer.write_all(b"RKCI\x01").await.expect("partial");
    drop(writer);
    assert!(reader.records.recv().await.is_none());
    assert!(reader.failed());
}

#[tokio::test]
async fn persisted_consent_and_reader_loss_both_gate_transmission() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&temp.path().join("state")).await.expect("store");
    assert!(!store.allowed().await);
    store.set_capture_opt_in(true).await.expect("consent");
    assert!(store.allowed().await);
    let failure = AtomicBool::new(false);
    let gate = SessionGate { store: &store, failed: &failure };
    assert!(gate.allowed().await);
    failure.store(true, Ordering::Release);
    assert!(!gate.allowed().await);
    failure.store(false, Ordering::Release);
    store.set_paused(true).await.expect("pause");
    assert!(!gate.allowed().await);
    store.set_paused(false).await.expect("resume");
    store.set_maintenance_stop(true).await.expect("maintenance");
    assert!(!gate.allowed().await);
    store.close().await;
}

#[tokio::test]
async fn new_pipeline_does_not_resume_old_midstream_payload() {
    let (mut pipeline, state, gate) = setup();
    pipeline.record(Record::Started(native_backend()), Duration::ZERO, &gate).await.expect("start");
    pipeline
        .record(server(101, 11, 0x10, b"old ciphertext"), Duration::ZERO, &gate)
        .await
        .expect("ignored midstream");
    assert_eq!(state.lock().expect("state").starts, 0);
    assert!(state.lock().expect("state").events.is_empty());
}

#[tokio::test]
async fn settings_ticks_do_not_cancel_an_incomplete_ipc_read() {
    use tokio::io::AsyncWriteExt;
    let (mut writer, input) = tokio::io::duplex(128);
    let mut reader = Reader::start(input);
    writer.write_all(b"RKCI\x01").await.expect("prefix");
    for _ in 0..3 {
        tokio::select! {
            _ = reader.records.recv() => panic!("partial frame must not emit"),
            _ = tokio::time::sleep(Duration::from_millis(1)) => {},
        }
    }
    writer.write_all(&[22, 0, 0, 0, 0, 0, 0]).await.expect("rest");
    assert_eq!(reader.records.recv().await, Some(Record::Keepalive));
    assert!(!reader.failed());
}

#[tokio::test]
async fn eof_during_delayed_consent_read_prevents_the_next_upload_event() {
    struct Delayed {
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }
    impl Gate for Delayed {
        async fn allowed(&self) -> bool {
            self.entered.notify_one();
            self.release.notified().await;
            true
        }
    }
    let (mut pipeline, state, permission) = setup();
    established(&mut pipeline, &permission).await;
    let (writer, input) = tokio::io::duplex(128);
    let reader = Reader::start(input);
    let delayed =
        Delayed { entered: tokio::sync::Notify::new(), release: tokio::sync::Notify::new() };
    let gate = SessionGate { store: &delayed, failed: &reader.failed };
    let before = state.lock().expect("state").events.len();
    let (result, ()) = tokio::join!(
        pipeline.events(
            vec![Event::Data { id: 1, offset: 0, bytes: b"must not send".to_vec() }],
            &gate
        ),
        async {
            delayed.entered.notified().await;
            drop(writer);
            while !reader.failed() {
                tokio::task::yield_now().await;
            }
            delayed.release.notify_one();
        }
    );
    assert_eq!(result, Err(Failure::Revoked));
    assert_eq!(state.lock().expect("state").events.len(), before);
    assert_eq!(state.lock().expect("state").starts, 1);
}

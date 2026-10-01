//! SCM-only host. Enumerating sessions creates no capture handles; an authenticated
//! installed agent must explicitly send Start for every live capture connection.
use super::pump::{NativeRecord, Pump};
use crate::socket_evidence::{EvidenceEvent, POLL_INTERVAL, SessionGenerations, SocketEvidence};
use rokbattles_capture_ipc::{Backend, PacketBytes, UnavailableReason};
use rokbattles_capture_ipc::{
    ClientRequest, IO_DEADLINE, Record, SERVICE_NAME, read_request,
    windows::{Identity, InstalledFile, ProtectedInstallation, authenticate_client, create_server},
    write_record,
};
use std::{
    collections::BTreeMap,
    ffi::c_void,
    io,
    os::windows::io::{AsHandle, FromRawHandle, OwnedHandle},
    ptr,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
    time::Duration,
};
use tokio::net::windows::named_pipe::NamedPipeServer;
use windows_sys::Win32::{
    Foundation::{ERROR_CALL_NOT_IMPLEMENTED, ERROR_SUCCESS},
    System::{
        RemoteDesktop::{
            WTS_SESSION_INFOW, WTSActive, WTSEnumerateSessionsW, WTSFreeMemory, WTSQueryUserToken,
        },
        Services::*,
    },
};

static STOP: AtomicBool = AtomicBool::new(false);
static STATUS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
const MAX_SESSIONS: usize = 16;

/// Called only by the binary's exact --service mode. Never installs or starts SCM services.
pub fn dispatch() -> io::Result<()> {
    let mut name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(Some(0)).collect();
    let table = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: name.as_mut_ptr(),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW::default(),
    ];
    // SAFETY: terminated table and name live until SCM dispatcher returns; callbacks
    // have the exact Windows ABI and retain no borrowed pointer after dispatch.
    if unsafe { StartServiceCtrlDispatcherW(table.as_ptr()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
unsafe extern "system" fn service_main(_argc: u32, _argv: *mut *mut u16) {
    let name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(Some(0)).collect();
    // SAFETY: fixed name, ABI-correct callback, no user context pointer.
    let handle =
        unsafe { RegisterServiceCtrlHandlerExW(name.as_ptr(), Some(control), ptr::null()) };
    if handle.is_null() {
        return;
    }
    STATUS.store(handle, Ordering::Release);
    status(SERVICE_START_PENDING, 0);
    // The service itself must already live in the immutable protected install.
    let installed = ProtectedInstallation::open(InstalledFile::Helper);
    let runtime =
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build();
    let (Ok(_installation), Ok(runtime)) = (installed, runtime) else {
        status(SERVICE_STOPPED, 1);
        return;
    };
    status(SERVICE_RUNNING, 0);
    runtime.block_on(run());
    runtime.shutdown_timeout(Duration::from_secs(5));
    status(SERVICE_STOPPED, 0);
}
unsafe extern "system" fn control(
    code: u32,
    _event: u32,
    _data: *mut c_void,
    _context: *mut c_void,
) -> u32 {
    match code {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            STOP.store(true, Ordering::Release);
            status(SERVICE_STOP_PENDING, 0);
            ERROR_SUCCESS
        }
        SERVICE_CONTROL_INTERROGATE => ERROR_SUCCESS,
        _ => ERROR_CALL_NOT_IMPLEMENTED,
    }
}
fn status(state: u32, error: u32) {
    let handle = STATUS.load(Ordering::Acquire);
    if handle.is_null() {
        return;
    }
    let status = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: state,
        dwControlsAccepted: if state == SERVICE_RUNNING {
            SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN
        } else {
            0
        },
        dwWin32ExitCode: error,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint: u32::from(state == SERVICE_START_PENDING || state == SERVICE_STOP_PENDING),
        dwWaitHint: if state == SERVICE_START_PENDING || state == SERVICE_STOP_PENDING {
            10_000
        } else {
            0
        },
    };
    // SAFETY: SCM-issued handle remains valid during service_main; initialized status.
    unsafe { SetServiceStatus(handle, &status) };
}

struct SessionTask {
    identity: Identity,
    cancel: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}
async fn join_session(session: SessionTask) {
    let _sent = session.cancel.send(true);
    if !matches!(tokio::time::timeout(Duration::from_secs(8), session.task).await, Ok(Ok(()))) {
        // Never report SCM STOPPED while a native capture may still hold a driver
        // or installation file. Process termination closes every remaining handle.
        std::process::abort();
    }
}
async fn run() {
    let mut sessions: BTreeMap<u32, SessionTask> = BTreeMap::new();
    while !STOP.load(Ordering::Acquire) {
        let active = active_sessions().unwrap_or_default();
        let retired: Vec<u32> = sessions
            .iter()
            .filter(|(id, session)| {
                active.get(id) != Some(&session.identity) || session.task.is_finished()
            })
            .map(|(id, _)| *id)
            .collect();
        for id in retired {
            if let Some(session) = sessions.remove(&id) {
                join_session(session).await;
            }
        }
        for (id, identity) in active {
            if sessions.len() >= MAX_SESSIONS {
                break;
            }
            sessions.entry(id).or_insert_with(|| {
                let peer = identity.clone();
                let (cancel, receiver) = tokio::sync::watch::channel(false);
                SessionTask {
                    identity,
                    cancel,
                    task: tokio::spawn(async move { serve_logon(peer, receiver).await }),
                }
            });
        }
        for _ in 0..20 {
            if STOP.load(Ordering::Acquire) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    // Signal every capture before awaiting any one; all stop concurrently.
    for session in sessions.values() {
        let _sent = session.cancel.send(true);
    }
    for (_, session) in sessions {
        join_session(session).await;
    }
}

fn active_sessions() -> io::Result<BTreeMap<u32, Identity>> {
    let mut sessions: *mut WTS_SESSION_INFOW = ptr::null_mut();
    let mut count = 0;
    // SAFETY: local WTS server with documented version 1; outputs initialized.
    if unsafe { WTSEnumerateSessionsW(ptr::null_mut(), 0, 1, &mut sessions, &mut count) } == 0 {
        return Err(io::Error::last_os_error());
    }
    struct Allocation(*mut WTS_SESSION_INFOW);
    impl Drop for Allocation {
        fn drop(&mut self) {
            // SAFETY: owns the WTS allocation exactly once.
            unsafe { WTSFreeMemory(self.0.cast()) };
        }
    }
    let _allocation = Allocation(sessions);
    if count > 1024 || (count != 0 && sessions.is_null()) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "session enumeration unavailable"));
    }
    let mut active = BTreeMap::new();
    if count == 0 {
        return Ok(active);
    }
    // SAFETY: OS returned count initialized rows; count is capped before iteration.
    for session in unsafe { std::slice::from_raw_parts(sessions, count as usize) } {
        if session.State != WTSActive || session.SessionId == 0 {
            continue;
        }
        if active.len() >= MAX_SESSIONS {
            break;
        }
        let mut token = ptr::null_mut();
        // SAFETY: session came from OS enumeration; SYSTEM obtains this user's token.
        if unsafe { WTSQueryUserToken(session.SessionId, &mut token) } == 0 || token.is_null() {
            continue;
        }
        // SAFETY: WTSQueryUserToken transfers ownership of a fresh token handle.
        let token = unsafe { OwnedHandle::from_raw_handle(token) };
        if let Ok(identity) = Identity::from_token(token.as_handle()) {
            active.insert(session.SessionId, identity);
        }
    }
    Ok(active)
}

async fn serve_logon(identity: Identity, mut cancel: tokio::sync::watch::Receiver<bool>) {
    loop {
        if STOP.load(Ordering::Acquire) || *cancel.borrow() {
            return;
        }
        let Ok(mut server) = create_server(&identity) else {
            return;
        };
        tokio::select! {
            biased;
            _ = cancel.changed() => return,
            connected = server.connect() => if connected.is_err() { return; },
        }
        let start = tokio::select! {
            biased;
            _ = cancel.changed() => return,
            start = tokio::time::timeout(IO_DEADLINE, read_request(&mut server)) => start,
        };
        if matches!(start, Ok(Ok(ClientRequest::Start)))
            && let Ok(peer) = authenticate_client(&server, &identity)
        {
            // Peer handle remains pinned until the complete session ends.
            stream_session(&mut server, identity.clone(), peer, &mut cancel).await;
        }
        // Dropping the pipe ends consent. A new connection must authenticate and Start again.
        drop(server);
    }
}

async fn stream_session(
    server: &mut NamedPipeServer,
    identity: Identity,
    peer: rokbattles_capture_ipc::windows::ProcessIdentity,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) {
    let mut pump = Pump::start();
    let mut guard =
        crate::ownership::FlowGuard::new(super::WindowsOwnerLookup::new(identity.clone()));
    let started = std::time::Instant::now();
    let mut backend = None;
    let mut socket: Option<SocketEvidence> = None;
    let mut polling = tokio::time::interval(POLL_INTERVAL);
    polling.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let (mut reader, mut writer) = tokio::io::split(server);
    let mut stop_request = Box::pin(read_request(&mut reader));
    let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
    let mut watch_failure = true;
    let mut acknowledge_stop = false;
    loop {
        tokio::select! {
            biased;
            _ = cancel.changed() => break,
            changed = pump.failed.changed(), if watch_failure => {
                if *pump.failed.borrow() { break; }
                if changed.is_err() { watch_failure=false; }
            },
            result = &mut stop_request => {
                pump.cancel();
                acknowledge_stop = matches!(result, Ok(ClientRequest::Stop));
                break;
            }
            record = pump.records.recv() => {
                let Some(record) = record else { break; };
                let records = match record {
                    NativeRecord::Started(kind) => {
                        if backend.is_some() { break; }
                        if kind == Backend::Pcap {
                            match SocketEvidence::new(identity.clone(), SessionGenerations::default()) {
                                Ok(source) => socket=Some(source),
                                Err(_error) => { let _sent=write_record(&mut writer,&Record::Unavailable(UnavailableReason::OwnershipUnavailable)).await; break; }
                            }
                        }
                        backend=Some(kind);
                        vec![Record::Started(kind)]
                    },
                    NativeRecord::Packet(bytes) => {
                        if let Some(source)=socket.as_mut() {
                            if write_socket_packet(&mut writer,source,bytes,&started).await.is_err() {
                                let _sent=write_record(&mut writer,&Record::Gap).await;
                                break;
                            }
                            continue;
                        }
                        if backend != Some(Backend::WinDivert) { break; }
                        guard.server(bytes,started.elapsed())
                    },
                    #[cfg(target_arch = "x86_64")]
                    NativeRecord::ClientControl(control) => {
                        if backend != Some(Backend::WinDivert) { break; }
                        guard.client(control,started.elapsed())
                    },
                    NativeRecord::Unavailable(reason) => {
                        let _sent=write_record(&mut writer,&Record::Unavailable(reason)).await;
                        break;
                    }
                };
                let mut failed=false;
                for record in records {
                    if !guard.authorize_record(&record) {
                        let gap=guard.gap();
                        if write_record(&mut writer,&gap).await.is_err() { failed=true; }
                        break;
                    }
                    if write_record(&mut writer,&record).await.is_err() { failed=true;break; }
                    guard.record_written(&record);
                }
                if failed { break; }
            }
            _ = polling.tick(), if socket.is_some() => {
                let Some(source)=socket.as_mut() else { break; };
                match source.poll(started.elapsed()) {
                    Ok(events) => if write_socket_events(&mut writer,events).await.is_err() { break; },
                    Err(_error) => { let _sent=write_record(&mut writer,&Record::Gap).await; break; }
                }
            }

            _ = heartbeat.tick(), if backend.is_some() => {
                if !peer.is_alive() || STOP.load(Ordering::Acquire) { break; }
                if write_record(&mut writer, &Record::Keepalive).await.is_err() { break; }
            }
        }
    }
    pump.cancel();
    pump.finish().await;
    if acknowledge_stop {
        let stopped = if backend.is_some() {
            Record::Stopped
        } else {
            Record::Unavailable(UnavailableReason::SessionEnded)
        };
        let _sent = write_record(&mut writer, &stopped).await;
    }
}

async fn write_socket_events<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    events: Vec<EvidenceEvent>,
) -> io::Result<()> {
    for event in events {
        let record = match event {
            EvidenceEvent::Established(evidence) => Record::SocketEstablished(evidence),
            EvidenceEvent::Retired(evidence) => Record::SocketRetired(evidence),
        };
        write_record(writer, &record).await?;
    }
    Ok(())
}
async fn write_socket_packet<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    source: &mut SocketEvidence,
    bytes: PacketBytes,
    started: &std::time::Instant,
) -> io::Result<()> {
    let Some(packet) = rokbattles_capture_runtime::packet::parse(&bytes) else {
        return Err(io::Error::other("capture packet invalid"));
    };
    // Metadata writes may await. Repeat a fresh table/token authorization after
    // them, immediately before the packet write; never carry a queue-time grant.
    for _ in 0..128 {
        let decision = source
            .server(&packet, started.elapsed())
            .map_err(|_error| io::Error::other("socket ownership unavailable"))?;
        if !decision.events.is_empty() {
            write_socket_events(writer, decision.events).await?;
            continue;
        }
        if decision.allowed {
            write_record(writer, &Record::ServerPacket(bytes)).await?;
        }
        return Ok(());
    }
    Err(io::Error::other("socket ownership changed repeatedly"))
}

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

use core_tcp_stream::{
    CLIENT_PORT, CaptureConfig, CaptureError, CaptureEvent, CaptureSource, Handshake, StreamId,
    TcpStreamBatch, TcpStreamFragmentUpload, TrackerEvent, parse_handshake, run_capture_until,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::watcher::WatcherConfig;

const BATCH_FLUSH_INTERVAL: Duration = Duration::from_secs(75);
const BATCH_FRAGMENT_TARGET: usize = 4096;
const HANDSHAKE_ONLY_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const ACTIVE_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NetworkClientState {
    Disabled,
    Waiting,
    Connected,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkStatus {
    state: NetworkClientState,
    message: Option<String>,
}

impl NetworkStatus {
    fn disabled() -> Self {
        Self {
            state: NetworkClientState::Disabled,
            message: Some("Network introspection is disabled.".to_string()),
        }
    }
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Default)]
pub(crate) struct NetworkIntrospectionManager {
    task: tokio::sync::Mutex<Option<NetworkTask>>,
    status: Arc<Mutex<NetworkStatus>>,
}

impl NetworkIntrospectionManager {
    pub(crate) async fn start(&self, app: &AppHandle) {
        if !crate::app_config::get_experimental_network_introspection(app).unwrap_or(false) {
            self.set_status(app, NetworkStatus::disabled()).await;
            return;
        }

        let mut guard = self.task.lock().await;
        if guard.is_some() {
            return;
        }

        if let Err(message) = check_capture_runtime_prerequisite() {
            self.set_status(
                app,
                NetworkStatus { state: NetworkClientState::Error, message: Some(message) },
            )
            .await;
            return;
        }

        self.set_status(
            app,
            NetworkStatus {
                state: NetworkClientState::Waiting,
                message: Some("Waiting for client. If it is already open, restart it.".into()),
            },
        )
        .await;

        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::unbounded_channel();
        let api_url = WatcherConfig::default().api_tcp_stream_url.to_string();
        let app_for_upload = app.clone();
        let upload_handle =
            tauri::async_runtime::spawn(upload_loop(app_for_upload, rx, api_url, http_client()));

        let app_for_capture = app.clone();
        let status_for_capture = Arc::clone(&self.status);
        let running_for_capture = Arc::clone(&running);
        let capture_handle = tauri::async_runtime::spawn_blocking(move || {
            let mut capture_status = CaptureStatusTracker::default();
            let config = CaptureConfig {
                source: CaptureSource::Auto { probe_duration: Duration::from_secs(3) },
                port: CLIENT_PORT,
            };
            let result = run_capture_until(
                config,
                |event| {
                    handle_capture_event(
                        &app_for_capture,
                        &status_for_capture,
                        &tx,
                        &mut capture_status,
                        event,
                    );
                },
                || running_for_capture.load(Ordering::SeqCst),
            );

            if let Err(error) = result
                && !matches!(error, CaptureError::Stopped)
            {
                emit_network_status(
                    &app_for_capture,
                    &status_for_capture,
                    NetworkStatus {
                        state: NetworkClientState::Error,
                        message: Some(error_message(&error)),
                    },
                );
            }
        });

        *guard = Some(NetworkTask {
            running,
            capture_handle,
            upload_handle,
            shutdown_timeout: Duration::from_secs(3),
        });
    }

    pub(crate) async fn shutdown(&self) {
        let mut guard = self.task.lock().await;
        if let Some(task) = guard.take() {
            task.shutdown().await;
        }
    }

    pub(crate) async fn status(&self) -> NetworkStatus {
        self.status.lock().map_or_else(|_| NetworkStatus::default(), |status| status.clone())
    }

    pub(crate) async fn set_status(&self, app: &AppHandle, status: NetworkStatus) {
        emit_network_status(app, &self.status, status);
    }
}

struct NetworkTask {
    running: Arc<AtomicBool>,
    capture_handle: tauri::async_runtime::JoinHandle<()>,
    upload_handle: tauri::async_runtime::JoinHandle<()>,
    shutdown_timeout: Duration,
}

impl NetworkTask {
    async fn shutdown(self) {
        self.running.store(false, Ordering::SeqCst);

        let mut capture_handle = self.capture_handle;
        tokio::select! {
            _ = &mut capture_handle => {}
            _ = tokio::time::sleep(self.shutdown_timeout) => {
                capture_handle.abort();
            }
        }

        let mut upload_handle = self.upload_handle;
        tokio::select! {
            _ = &mut upload_handle => {}
            _ = tokio::time::sleep(self.shutdown_timeout) => {
                upload_handle.abort();
            }
        }
    }
}

#[derive(Debug)]
struct UploadState {
    capture_id: String,
    batch_index: u64,
    stream: StreamId,
    handshake: Handshake,
    fragments: Vec<TcpStreamFragmentUpload>,
    last_fragment_at: Instant,
}

async fn upload_loop(
    app: AppHandle,
    mut rx: mpsc::UnboundedReceiver<TrackerEvent>,
    api_url: String,
    client: reqwest::Client,
) {
    let mut streams: HashMap<StreamId, UploadState> = HashMap::new();
    let mut flush_interval = tokio::time::interval(BATCH_FLUSH_INTERVAL);

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Some(event) => {
                        handle_tracker_event(&app, &api_url, &client, &mut streams, event).await;
                    }
                    None => {
                        flush_all(&app, &api_url, &client, &mut streams, true).await;
                        break;
                    }
                }
            }
            _ = flush_interval.tick() => {
                flush_all(&app, &api_url, &client, &mut streams, false).await;
            }
        }
    }
}

async fn handle_tracker_event(
    app: &AppHandle,
    api_url: &str,
    client: &reqwest::Client,
    streams: &mut HashMap<StreamId, UploadState>,
    event: TrackerEvent,
) {
    match event {
        TrackerEvent::StreamAccepted { stream, handshake } => {
            streams.insert(
                stream.clone(),
                UploadState {
                    capture_id: capture_id(&stream, handshake),
                    batch_index: 0,
                    stream,
                    handshake,
                    fragments: Vec::new(),
                    last_fragment_at: Instant::now(),
                },
            );
        }
        TrackerEvent::StreamFragment { stream, fragment } => {
            let Some(state) = streams.get_mut(&stream) else {
                return;
            };
            state.fragments.push(TcpStreamFragmentUpload::from_payload(
                fragment.index,
                fragment.direction,
                &fragment.payload,
            ));
            state.last_fragment_at = Instant::now();
            if state.fragments.len() >= BATCH_FRAGMENT_TARGET {
                let _ = flush_stream(app, api_url, client, state, false).await;
            }
        }
        TrackerEvent::StreamEnded { stream } => {
            if let Some(mut state) = streams.remove(&stream)
                && !flush_stream(app, api_url, client, &mut state, true).await
            {
                streams.insert(stream, state);
            }
        }
        TrackerEvent::StreamIgnored { .. } => {}
    }
}

async fn flush_all(
    app: &AppHandle,
    api_url: &str,
    client: &reqwest::Client,
    streams: &mut HashMap<StreamId, UploadState>,
    stream_ended: bool,
) {
    let stream_ids = streams.keys().cloned().collect::<Vec<_>>();
    for stream in stream_ids {
        let Some(state) = streams.get_mut(&stream) else {
            continue;
        };
        if should_hold_handshake_only_stream(state, stream_ended) {
            continue;
        }
        if should_drop_handshake_only_stream(state, stream_ended) {
            emit_log(app, "Dropped idle handshake-only network stream");
            streams.remove(&stream);
            continue;
        }
        let _ = flush_stream(app, api_url, client, state, stream_ended).await;
    }
}

fn should_hold_handshake_only_stream(state: &UploadState, stream_ended: bool) -> bool {
    is_handshake_only_stream(state)
        && !stream_ended
        && state.last_fragment_at.elapsed() < HANDSHAKE_ONLY_IDLE_TIMEOUT
}

fn should_drop_handshake_only_stream(state: &UploadState, stream_ended: bool) -> bool {
    is_handshake_only_stream(state)
        && (stream_ended || state.last_fragment_at.elapsed() >= HANDSHAKE_ONLY_IDLE_TIMEOUT)
}

fn is_handshake_only_stream(state: &UploadState) -> bool {
    if state.batch_index != 0 || state.fragments.len() != 1 {
        return false;
    }
    let Some(fragment) = state.fragments.first() else {
        return false;
    };
    if fragment.direction != core_tcp_stream::Direction::ServerToClient {
        return false;
    }
    let Ok(payload) = fragment.payload() else {
        return false;
    };
    let Some((frame_body, consumed)) = first_frame(&payload) else {
        return false;
    };
    consumed == payload.len() && parse_handshake(frame_body).is_some()
}

fn first_frame(payload: &[u8]) -> Option<(&[u8], usize)> {
    let short = u16::from_be_bytes(payload.get(0..2)?.try_into().ok()?);
    let (length, body_start): (usize, usize) = if short == u16::MAX {
        (usize::try_from(u32::from_be_bytes(payload.get(2..6)?.try_into().ok()?)).ok()?, 6)
    } else {
        (usize::from(short), 2)
    };
    let body_end = body_start.checked_add(length)?;
    Some((payload.get(body_start..body_end)?, body_end))
}

async fn flush_stream(
    app: &AppHandle,
    api_url: &str,
    client: &reqwest::Client,
    state: &mut UploadState,
    stream_ended: bool,
) -> bool {
    if state.fragments.is_empty() && !stream_ended {
        return true;
    }

    let fragments = std::mem::take(&mut state.fragments);
    let fragment_count = fragments.len();
    let batch = TcpStreamBatch {
        capture_id: state.capture_id.clone(),
        batch_index: state.batch_index,
        stream_ended,
        stream: state.stream.clone(),
        handshake: state.handshake,
        fragments,
    };

    match post_tcp_stream_batch(client, api_url, &batch).await {
        Ok(()) => {
            emit_log(app, format!("Uploaded {} network fragments", fragment_count));
            state.batch_index = state.batch_index.saturating_add(1);
            true
        }
        Err(message) => {
            restore_failed_fragments(state, batch.fragments);
            emit_log(app, format!("Failed to upload network fragments: {message}"));
            false
        }
    }
}

fn restore_failed_fragments(
    state: &mut UploadState,
    failed_fragments: Vec<TcpStreamFragmentUpload>,
) {
    if state.fragments.is_empty() {
        state.fragments = failed_fragments;
    } else {
        let mut restored = failed_fragments;
        restored.append(&mut state.fragments);
        state.fragments = restored;
    }
}

async fn post_tcp_stream_batch(
    client: &reqwest::Client,
    api_url: &str,
    batch: &TcpStreamBatch,
) -> Result<(), String> {
    let response = client
        .post(api_url)
        .json(batch)
        .send()
        .await
        .map_err(|error| format!("failed to send tcp stream batch: {error}"))?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    Err(format!("API rejected tcp stream batch: {status} {body}"))
}

fn handle_capture_event(
    app: &AppHandle,
    status: &Arc<Mutex<NetworkStatus>>,
    tx: &mpsc::UnboundedSender<TrackerEvent>,
    capture_status: &mut CaptureStatusTracker,
    event: CaptureEvent,
) {
    match event {
        CaptureEvent::DeviceSelected { name } => {
            emit_log(app, format!("Network capture device selected: {name}"));
        }
        CaptureEvent::WaitingForClient => {
            if let Some(next_status) = capture_status.waiting_status() {
                emit_network_status(app, status, next_status);
            }
        }
        CaptureEvent::Tracker(event) => {
            if let Some(next_status) = capture_status.tracker_status(&event) {
                emit_network_status(app, status, next_status);
            }
            let _ = tx.send(event);
        }
    }
}

#[derive(Debug, Default)]
struct CaptureStatusTracker {
    active_streams: HashMap<StreamId, Instant>,
}

impl CaptureStatusTracker {
    fn waiting_status(&mut self) -> Option<NetworkStatus> {
        self.expire_idle_streams();
        if self.active_streams.is_empty() {
            Some(NetworkStatus {
                state: NetworkClientState::Waiting,
                message: Some("Waiting for client. If it is already open, restart it.".into()),
            })
        } else {
            None
        }
    }

    fn tracker_status(&mut self, event: &TrackerEvent) -> Option<NetworkStatus> {
        match event {
            TrackerEvent::StreamAccepted { stream, .. } => {
                self.active_streams.insert(stream.clone(), Instant::now());
                Some(NetworkStatus { state: NetworkClientState::Connected, message: None })
            }
            TrackerEvent::StreamFragment { stream, .. } => {
                self.active_streams.insert(stream.clone(), Instant::now());
                None
            }
            TrackerEvent::StreamEnded { stream } => {
                self.active_streams.remove(stream);
                if self.active_streams.is_empty() {
                    Some(NetworkStatus {
                        state: NetworkClientState::Disconnected,
                        message: Some("Disconnected. Waiting for reconnect.".to_string()),
                    })
                } else {
                    None
                }
            }
            TrackerEvent::StreamIgnored { reason, .. } => {
                if self.active_streams.is_empty() {
                    Some(NetworkStatus {
                        state: NetworkClientState::Waiting,
                        message: Some(format!("{reason}. Restart the client.")),
                    })
                } else {
                    None
                }
            }
        }
    }

    fn expire_idle_streams(&mut self) {
        self.active_streams
            .retain(|_stream, last_seen| last_seen.elapsed() < ACTIVE_STREAM_IDLE_TIMEOUT);
    }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(build_user_agent())
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .pool_idle_timeout(Some(Duration::from_secs(90)))
        .pool_max_idle_per_host(4)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .build()
        .expect("failed to build HTTP client")
}

fn build_user_agent() -> String {
    format!(
        "ROKBattles/{version} ({os}; {arch}) Tauri/{tauri}",
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        tauri = tauri::VERSION
    )
}

fn capture_id(stream: &StreamId, handshake: Handshake) -> String {
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis();
    format!(
        "{now_ms}-{}-{}-{}-{}-{}",
        stream.server_addr, stream.server_port, stream.client_port, handshake.key1, handshake.key2
    )
}

fn emit_network_status(
    app: &AppHandle,
    status_store: &Arc<Mutex<NetworkStatus>>,
    status: NetworkStatus,
) {
    if let Ok(mut current) = status_store.lock() {
        *current = status.clone();
    }
    let _ = app.emit("network-introspection", status);
}

#[derive(Debug, Clone, Serialize)]
struct LogPayload {
    message: String,
}

fn emit_log(app: &AppHandle, message: impl Into<String>) {
    let _ = app.emit("network-introspection-log", LogPayload { message: message.into() });
}

fn error_message(error: &CaptureError) -> String {
    if let Some(hint) = error.user_hint() {
        return format!("{error}. {hint}");
    }
    error.to_string()
}

#[cfg(target_os = "windows")]
fn check_capture_runtime_prerequisite() -> Result<(), String> {
    // This checks the Npcap/WinPcap DLL before any delay-loaded pcap import
    // is called, so missing runtimes become a recoverable status error.
    unsafe { libloading::Library::new("wpcap.dll") }.map(drop).map_err(|error| {
        format!(
            "Network introspection requires Npcap with WinPcap API-compatible mode enabled. Install Npcap, restart ROK Battles, then try again. Details: {error}"
        )
    })
}

#[cfg(not(target_os = "windows"))]
fn check_capture_runtime_prerequisite() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use core_tcp_stream::{Direction, IgnoreReason, TrackerEvent};

    use super::*;

    const HANDSHAKE_BODY: &[u8] = &[
        0x08, 0xf2, 0x42, 0x12, 0x0c, 0x08, 0x97, 0xd9, 0xd0, 0xaa, 0x02, 0x10, 0xd8, 0xb3, 0x98,
        0xf1, 0x03,
    ];

    #[test]
    fn capture_status_should_not_wait_while_stream_is_active() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();

        let connected = tracker
            .tracker_status(&TrackerEvent::StreamAccepted {
                stream: stream.clone(),
                handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
            })
            .expect("accepted stream should emit connected status");

        assert_eq!(connected.state, NetworkClientState::Connected);
        assert!(tracker.waiting_status().is_none());
    }

    #[test]
    fn capture_status_should_wait_after_last_stream_ends() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();
        let _ = tracker.tracker_status(&TrackerEvent::StreamAccepted {
            stream: stream.clone(),
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
        });

        let disconnected = tracker
            .tracker_status(&TrackerEvent::StreamEnded { stream })
            .expect("ending the last stream should emit disconnected status");
        let waiting = tracker.waiting_status().expect("no active streams should allow waiting");

        assert_eq!(disconnected.state, NetworkClientState::Disconnected);
        assert_eq!(waiting.state, NetworkClientState::Waiting);
    }

    #[test]
    fn capture_status_should_ignore_rejected_candidates_while_connected() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();
        let _ = tracker.tracker_status(&TrackerEvent::StreamAccepted {
            stream: stream.clone(),
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
        });

        let ignored = tracker.tracker_status(&TrackerEvent::StreamIgnored {
            stream,
            reason: IgnoreReason::CaptureStartedMidStream,
        });

        assert!(ignored.is_none());
    }

    #[test]
    fn capture_status_should_expire_idle_streams() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();
        tracker
            .active_streams
            .insert(stream, Instant::now() - ACTIVE_STREAM_IDLE_TIMEOUT - Duration::from_secs(1));

        let waiting = tracker.waiting_status().expect("idle active stream should be expired");

        assert_eq!(waiting.state, NetworkClientState::Waiting);
    }

    #[test]
    fn handshake_only_stream_should_wait_before_uploading() {
        let state = upload_state(vec![TcpStreamFragmentUpload::from_payload(
            0,
            Direction::ServerToClient,
            &prefixed(HANDSHAKE_BODY),
        )]);

        assert!(should_hold_handshake_only_stream(&state, false));
        assert!(!should_drop_handshake_only_stream(&state, false));
    }

    #[test]
    fn handshake_only_stream_should_drop_on_final_flush() {
        let state = upload_state(vec![TcpStreamFragmentUpload::from_payload(
            0,
            Direction::ServerToClient,
            &prefixed(HANDSHAKE_BODY),
        )]);

        assert!(!should_hold_handshake_only_stream(&state, true));
        assert!(should_drop_handshake_only_stream(&state, true));
    }

    #[test]
    fn multi_frame_first_fragment_should_upload_normally() {
        let mut payload = prefixed(HANDSHAKE_BODY);
        payload.extend_from_slice(&prefixed(&[0xaa]));
        let state = upload_state(vec![TcpStreamFragmentUpload::from_payload(
            0,
            Direction::ServerToClient,
            &payload,
        )]);

        assert!(!should_hold_handshake_only_stream(&state, false));
        assert!(!should_drop_handshake_only_stream(&state, false));
    }

    fn stream_id() -> StreamId {
        StreamId {
            client_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
            client_port: 56_380,
            server_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)),
            server_port: CLIENT_PORT,
        }
    }

    fn upload_state(fragments: Vec<TcpStreamFragmentUpload>) -> UploadState {
        UploadState {
            capture_id: "capture-1".to_string(),
            batch_index: 0,
            stream: stream_id(),
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
            fragments,
            last_fragment_at: Instant::now(),
        }
    }

    fn prefixed(body: &[u8]) -> Vec<u8> {
        let mut payload = Vec::from(u16::try_from(body.len()).unwrap().to_be_bytes());
        payload.extend_from_slice(body);
        payload
    }
}

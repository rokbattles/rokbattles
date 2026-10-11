mod config;
mod mail;
mod scan;
mod state;
mod store;
mod upload;

use std::{
    fs,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, SystemTime},
};

use rustc_hash::FxHashSet;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch};

pub(crate) use self::config::WatcherConfig;
use self::{
    mail::file_name_for_upload,
    scan::{apply_fs_event, next_file, refresh_scans_if_needed, sync_fs_watches},
    state::WatcherState,
    store::{QueuedUpload, StateFile, file_sig},
    upload::{UploadApiError, UploadStatus, post_file_to_api, upload_backoff},
};

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

// Diagnostics and snapshot validation must enforce the same persisted byte limit.
const MAX_REASON_BYTES: usize = 2048;

#[derive(Debug, Clone, Serialize)]
struct LogPayload {
    message: String,
}

fn build_user_agent() -> String {
    // Format: ROKBattles/<app_version> (<os>; <arch>) Tauri/<tauri_version>
    format!(
        "ROKBattles/{version} ({os}; {arch}) Tauri/{tauri}",
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        tauri = tauri::VERSION
    )
}

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        let user_agent = build_user_agent();
        reqwest::Client::builder()
            .user_agent(user_agent)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            .pool_max_idle_per_host(8)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .expect("failed to build HTTP client")
    })
}

fn emit_log(app: &AppHandle, message: impl Into<String>) {
    if let Err(error) = app.emit("rokbattles", LogPayload { message: message.into() }) {
        eprintln!("[rokbattles] failed to emit watcher log: {error}");
    }
}

fn now_epoch_ms() -> u128 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis()
}

pub fn reset_upload_state(app: &AppHandle) -> anyhow::Result<()> {
    StateFile::for_app(app)?.reset()
}

fn truncate_reason(mut reason: String) -> String {
    if reason.len() > MAX_REASON_BYTES {
        let suffix = " [truncated]";
        let mut end = MAX_REASON_BYTES - suffix.len();

        while !reason.is_char_boundary(end) {
            end -= 1;
        }

        reason.truncate(end);
        reason.push_str(suffix);
    }

    reason
}

fn apply_upload_result(
    state: &mut WatcherState,
    mut item: QueuedUpload,
    file_name: &str,
    result: Result<UploadStatus, UploadApiError>,
    now_ms: u128,
) -> String {
    match result {
        Ok(status) => {
            state.acknowledge_upload(&item.path);

            status.log_message(file_name)
        }
        Err(error) => {
            let reason = truncate_reason(error.to_string());

            if error.is_retryable() {
                item.attempts = item.attempts.saturating_add(1);

                let backoff = error
                    .retry_after()
                    .filter(|delay| !delay.is_zero())
                    .unwrap_or_else(|| upload_backoff(item.attempts));

                item.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));
                item.last_error = Some(reason.clone());
                state.requeue_upload(item);

                if error.is_rate_limited() && state.extend_rate_limit(now_ms, backoff) {
                    let wait_secs = (backoff.as_millis().saturating_add(999) / 1000).max(1);

                    return format!("Rate limited by API (429). Pausing uploads for {wait_secs}s.");
                }
            } else {
                state.reject_upload(item, reason.clone());
            }

            truncate_reason(format!("Failed to upload {file_name}: {reason}"))
        }
    }
}

pub struct WatcherTask {
    shutdown: watch::Sender<bool>,
    handle: tauri::async_runtime::JoinHandle<()>,
    shutdown_timeout: Duration,
}

impl WatcherTask {
    pub async fn shutdown(self, app: &AppHandle) {
        // No receiver means the watcher has already stopped; still join its task below.
        let _shutdown_result = self.shutdown.send(true);
        let mut handle = self.handle;
        tokio::select! {
            _ = &mut handle => {}
            _ = tokio::time::sleep(self.shutdown_timeout) => {
                emit_log(app, "Watcher shutdown timed out; aborting task");
                handle.abort();
            }
        }
    }
}

pub fn spawn_watcher(app: &AppHandle) -> WatcherTask {
    let app = app.clone();

    let config = WatcherConfig::default();
    let api_url = config.api_ingress_url.to_string();
    let shutdown_timeout = config.shutdown_timeout;
    let client = http_client();

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let (fs_tx, mut fs_rx) = mpsc::channel::<PathBuf>(config.fs_event_queue_capacity);
    let config_for_task = config.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let state_file = match StateFile::for_app(&app) {
            Ok(file) => file,
            Err(error) => {
                emit_log(&app, format!("Watcher stopped: {error:#}"));
                return;
            }
        };

        let uploads = match state_file.load() {
            Ok(uploads) => uploads,
            Err(error) => {
                emit_log(&app, format!("Watcher stopped; upload state retained: {error:#}"));
                return;
            }
        };

        let mut state = WatcherState::new(config_for_task, uploads);

        let mut fs_watcher = match notify::recommended_watcher({
            let fs_tx = fs_tx.clone();
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        // Periodic directory scans recover events dropped when the queue is full.
                        let _send_result = fs_tx.try_send(path);
                    }
                }
            }
        }) {
            Ok(w) => Some(w),
            Err(e) => {
                emit_log(&app, format!("Failed to initialize filesystem watcher: {}", e));
                None
            }
        };
        let mut fs_watched_dirs: FxHashSet<PathBuf> = FxHashSet::default();

        loop {
            if *shutdown_rx.borrow() {
                break;
            }

            let now_ms = now_epoch_ms();

            let _ = refresh_scans_if_needed(&app, &mut state).await;
            state.maybe_rescan_hot(now_ms);
            sync_fs_watches(&app, fs_watcher.as_mut(), &mut fs_watched_dirs, &state.dirs);

            for _ in 0..state.config.fs_event_budget {
                let Ok(path) = fs_rx.try_recv() else {
                    break;
                };
                apply_fs_event(&mut state, path, now_ms);
            }

            prefetch_uploads(&mut state, now_ms);

            // Persist discovery and earlier outcomes before starting another request.
            // A failed write leaves memory dirty and prevents unsaved work from being sent.
            if let Err(error) = state.flush(&state_file) {
                emit_log(&app, format!("Uploads paused; failed to save state: {error:#}"));

                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                    _ = shutdown_rx.changed() => {}
                }

                continue;
            }

            let now_ms = now_epoch_ms();

            if state.rate_limit_remaining_ms(now_ms).is_none()
                && let Some(item) = state.next_ready_upload(now_ms)
            {
                let message = process_upload(&mut state, client, &api_url, item, now_ms).await;

                emit_log(&app, message);

                continue;
            }

            let now_ms = now_epoch_ms();

            tokio::select! {
                _ = tokio::time::sleep(state.config.idle_sleep) => {}
                res = shutdown_rx.changed() => {
                    if res.is_ok() && *shutdown_rx.borrow() {
                        break;
                    }
                }
                maybe_path = fs_rx.recv() => {
                    if let Some(path) = maybe_path {
                        apply_fs_event(&mut state, path, now_ms);
                    }
                }
            }
        }

        if let Err(error) = state.flush(&state_file) {
            emit_log(&app, format!("Failed to save upload state during shutdown: {error:#}"));
        }
    });

    WatcherTask { shutdown: shutdown_tx, handle, shutdown_timeout }
}

fn prefetch_uploads(state: &mut WatcherState, now_ms: u128) {
    let target = state.config.upload_prefetch_target;
    let ready = state
        .uploads
        .pending
        .iter()
        .filter(|item| item.not_before_ms.is_none_or(|deadline| deadline <= now_ms))
        .take(target)
        .count();

    // Missing or backed-off sources must leave room to discover another usable copy.
    for _ in ready..target {
        let Some(item) = next_file(state) else {
            break;
        };

        state.enqueue_upload(item);
    }
}

async fn process_upload(
    state: &mut WatcherState,
    client: &reqwest::Client,
    api_url: &str,
    mut item: QueuedUpload,
    now_ms: u128,
) -> String {
    let path = PathBuf::from(&item.path);
    let Some(file_name) =
        file_name_for_upload(&path).filter(|_| mail::mail_id_for_path(&item.path).is_some())
    else {
        let message = truncate_reason(format!("Rejected invalid mail filename: {}", item.path));
        state.reject_upload(item, message.clone());

        return message;
    };

    let sig =
        match fs::metadata(&path).map_err(anyhow::Error::from).and_then(|meta| file_sig(&meta)) {
            Ok(sig) => sig,
            Err(error) => {
                let message = truncate_reason(format!("Failed to stat file {file_name}: {error}"));
                state.retry_upload(item, message.clone(), now_ms);

                return message;
            }
        };

    let age_ms = now_ms.saturating_sub(sig.modified);

    if item.sig.as_ref() != Some(&sig) || age_ms < state.config.file_stable_age_ms {
        item.sig = Some(sig.clone());
        item.not_before_ms = Some(now_ms.saturating_add(state.config.file_retry_delay_ms));
        item.last_error = Some("Waiting for file to stop changing".to_string());

        state.discovered.insert(item.path.clone(), sig);
        state.requeue_upload(item);

        return format!("Waiting for stable file {file_name}");
    }

    let read = tauri::async_runtime::spawn_blocking(move || fs::read(path))
        .await
        .map_err(anyhow::Error::from)
        .and_then(|result| result.map_err(anyhow::Error::from));

    let bytes = match read {
        Ok(bytes) => bytes,
        Err(error) => {
            let message = truncate_reason(format!("Failed to read file {file_name}: {error}"));
            state.retry_upload(item, message.clone(), now_epoch_ms());

            return message;
        }
    };

    let decoded = match rokbattles_mail_codec::decode(&bytes) {
        Ok(decoded) => decoded,
        Err(error) => {
            let message = truncate_reason(format!("Decode failed for {file_name}: {error}"));
            state.reject_upload(item, message.clone());

            return message;
        }
    };

    if rokbattles_mail_registry::detect_mail_type(&decoded).is_none() {
        let raw_type = rokbattles_mail_registry::raw_mail_type(&decoded).unwrap_or("Unknown");
        let message = truncate_reason(format!(
            "Rejected unsupported mail {file_name} (detected: {raw_type})"
        ));
        state.reject_upload(item, message.clone());

        return message;
    }

    let result = post_file_to_api(client, api_url, &file_name, bytes).await;

    apply_upload_result(state, item, &file_name, result, now_epoch_ms())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, time::UNIX_EPOCH};

    use serde_json::json;

    use super::{
        scan::DirScan,
        store::UploadStore,
        upload::tests::{client, respond_once, response},
        *,
    };

    fn source(path: &Path, bytes: &[u8]) -> QueuedUpload {
        fs::create_dir_all(path.parent().expect("source directory"))
            .expect("create source directory");
        fs::write(path, bytes).expect("write source");
        fs::File::options()
            .write(true)
            .open(path)
            .expect("source file")
            .set_modified(UNIX_EPOCH + Duration::from_secs(1))
            .expect("stable source timestamp");

        QueuedUpload {
            path: path.to_string_lossy().into_owned(),
            sig: Some(file_sig(&fs::metadata(path).expect("source metadata")).expect("signature")),
            attempts: 0,
            not_before_ms: None,
            last_error: None,
        }
    }

    #[tokio::test]
    async fn missing_source_retries_and_valid_copy_acknowledges_the_id_after_restart() {
        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        let bytes = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"}))
            .expect("mail bytes");
        let missing = source(&dir.path().join("missing/Persistent.Mail.123"), &bytes);
        let good = source(&dir.path().join("good/Persistent.Mail.123"), &bytes);
        fs::remove_file(&missing.path).expect("source becomes temporarily unavailable");
        state.enqueue_upload(missing.clone());
        state.enqueue_upload(good.clone());
        state.flush(&file).expect("save pending sources");

        let message =
            process_upload(&mut state, &client(), "http://127.0.0.1:0", missing, 10_000).await;
        state.flush(&file).expect("save metadata retry");
        drop(file);

        assert!(message.contains("Failed to stat"));

        let file = StateFile::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        let ready = restored.next_ready_upload(10_000).expect("alternative source is ready");

        assert_eq!(ready.path, good.path);
        assert_eq!(restored.uploads.pending.len(), 2);

        let (url, server) =
            respond_once(response("201 Created", r#"{"status":"stored","mail_id":"123"}"#)).await;
        let message = process_upload(&mut restored, &client(), &url, ready, 10_000).await;
        server.await.expect("server received mail");
        restored.flush(&file).expect("save server acknowledgement");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart after acknowledgement");
        let completed = file.load().expect("load");

        assert!(message.starts_with("Stored new mail"));
        assert!(completed.pending.is_empty());
        assert!(completed.acknowledged.contains("123"));
    }

    #[tokio::test]
    async fn invalid_and_unsupported_files_remain_rejected_until_the_source_changes() {
        for bytes in [
            b"invalid binary".to_vec(),
            rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Unsupported"}))
                .expect("unsupported mail"),
        ] {
            let dir = tempfile::tempdir().expect("test directory");
            let file = StateFile::open(dir.path()).expect("state file");
            let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
            let path = dir.path().join("mail/Persistent.Mail.123");
            let bad = source(&path, &bytes);
            state.enqueue_upload(bad.clone());
            state.flush(&file).expect("save pending upload");

            let message =
                process_upload(&mut state, &client(), "http://127.0.0.1:0", bad, 10_000).await;
            state.flush(&file).expect("save local rejection");
            drop(file);

            let file = StateFile::open(dir.path()).expect("restart");
            let mut restored =
                WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
            let key = path.to_string_lossy().into_owned();

            assert!(restored.uploads.pending.is_empty());
            assert!(restored.uploads.acknowledged.is_empty());
            assert_eq!(restored.uploads.rejected.get(&key).expect("rejection").reason, message);

            let good_bytes = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"}))
                .expect("supported mail");
            source(&path, &good_bytes);
            restored.dirs.push(path.parent().expect("mail directory").to_path_buf());
            apply_fs_event(&mut restored, path.clone(), 10_000);
            restored.flush(&file).expect("save changed input for reevaluation");

            assert!(restored.uploads.rejected.is_empty());
            assert_eq!(file.load().expect("pending after changed input").pending.len(), 1);
        }
    }

    #[tokio::test]
    async fn read_failure_preserves_pending_work_and_reason() {
        let dir = tempfile::tempdir().expect("test directory");
        let path = dir.path().join("Persistent.Mail.123");
        fs::create_dir(&path).expect("path with metadata that cannot be read as mail");
        let item = QueuedUpload {
            path: path.to_string_lossy().into_owned(),
            sig: Some(file_sig(&fs::metadata(&path).expect("metadata")).expect("signature")),
            attempts: 0,
            not_before_ms: None,
            last_error: None,
        };
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        state.enqueue_upload(item.clone());
        state.flush(&file).expect("persist pending source");

        let message = process_upload(
            &mut state,
            &client(),
            "http://127.0.0.1:0",
            item,
            now_epoch_ms() + 2000,
        )
        .await;
        state.flush(&file).expect("persist failed read");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let restored = file.load().expect("load");
        let pending = restored.pending.front().expect("read failure remains pending");

        assert!(message.contains("Failed to read file"));
        assert_eq!(pending.last_error.as_deref(), Some(message.as_str()));
        assert_eq!(pending.attempts, 1);
        assert!(restored.acknowledged.is_empty());
    }

    #[tokio::test]
    async fn accepted_request_with_lost_response_recovers_through_skipped_acknowledgement() {
        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        let bytes =
            rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"})).expect("mail");
        let item = source(&dir.path().join("mail/Persistent.Mail.123"), &bytes);
        state.enqueue_upload(item.clone());
        state.flush(&file).expect("save before request");

        let (url, server) = respond_once(String::new()).await;
        process_upload(&mut state, &client(), &url, item, 10_000).await;
        server.await.expect("request reached server but response was lost");
        state.flush(&file).expect("save retry");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        let pending = restored
            .next_ready_upload(now_epoch_ms() + 300_000)
            .expect("uncertain request remains pending");
        let (url, server) =
            respond_once(response("200 OK", r#"{"status":"skipped","mail_id":"123"}"#)).await;

        process_upload(&mut restored, &client(), &url, pending, now_epoch_ms()).await;
        server.await.expect("idempotent retry");
        restored.flush(&file).expect("save acknowledgement");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let completed = file.load().expect("load");

        assert!(completed.acknowledged.contains("123"));
        assert!(completed.pending.is_empty());
    }

    #[test]
    fn discovery_with_unavailable_metadata_survives_restart() {
        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        let mut scan = DirScan::new(dir.path().to_path_buf(), Duration::ZERO, Duration::ZERO);
        scan.ids.push(123);
        scan.cursor = 1;
        state.scans.push(scan.clone());

        let missing =
            next_file(&mut state).expect("discovery is retained despite missing metadata");
        state.enqueue_upload(missing);
        state.flush(&file).expect("save discovered source");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let restored = file.load().expect("load");

        assert_eq!(restored.pending.len(), 1);
        assert!(restored.pending.front().expect("pending source").sig.is_none());
        assert!(restored.acknowledged.is_empty());
    }

    #[test]
    fn unflushed_discovery_has_no_persistent_marker_that_blocks_rediscovery() {
        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        source(&dir.path().join("Persistent.Mail.123"), b"discovered but not yet uploaded");
        let mut scan = DirScan::new(dir.path().to_path_buf(), Duration::ZERO, Duration::ZERO);
        scan.ids.push(123);
        scan.cursor = 1;
        state.scans.push(scan.clone());

        let discovered = next_file(&mut state).expect("source available");
        state.enqueue_upload(discovered);
        drop(state);
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart before discovery was saved");
        let mut restarted = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        restarted.scans.push(scan);

        assert!(
            next_file(&mut restarted).is_some(),
            "unsaved discovery must not leave a durable skip marker"
        );
    }

    #[test]
    fn missing_event_source_remains_pending() {
        let mut state = WatcherState::new(WatcherConfig::default(), UploadStore::default());
        let dir = tempfile::tempdir().expect("mail directory");
        state.dirs.push(dir.path().to_path_buf());

        apply_fs_event(&mut state, dir.path().join("Persistent.Mail.123"), 1000);

        assert_eq!(state.uploads.pending.len(), 1);
        assert!(state.uploads.pending.front().expect("pending source").sig.is_none());
    }

    #[test]
    fn retry_backlog_leaves_room_to_discover_another_copy() {
        let dir = tempfile::tempdir().expect("mail directory");
        let mut state = WatcherState::new(WatcherConfig::default(), UploadStore::default());

        for id in 1..=state.config.upload_prefetch_target {
            state.enqueue_upload(QueuedUpload {
                path: format!("unavailable/Persistent.Mail.{id}"),
                sig: None,
                attempts: 1,
                not_before_ms: Some(20_000),
                last_error: Some("file unavailable".to_string()),
            });
        }

        source(&dir.path().join("Persistent.Mail.1"), b"usable alternative source");
        let mut scan = DirScan::new(dir.path().to_path_buf(), Duration::ZERO, Duration::ZERO);
        scan.ids.push(1);
        scan.cursor = 1;
        state.scans.push(scan);

        prefetch_uploads(&mut state, 10_000);

        let ready = state.next_ready_upload(10_000).expect("alternative copy discovered");

        assert_eq!(ready.path, dir.path().join("Persistent.Mail.1").to_string_lossy());
    }

    #[tokio::test]
    async fn oversized_api_failures_cannot_inflate_persisted_reasons_or_logs() {
        for (status, body) in [
            ("400 Bad Request", "é".repeat(4000)),
            ("503 Service Unavailable", "é".repeat(4000)),
            ("400 Bad Request", "x".repeat(10_000)),
            ("200 OK", "x".repeat(10_000)),
        ] {
            let dir = tempfile::tempdir().expect("state directory");
            let file = StateFile::open(dir.path()).expect("state file");
            let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
            let item = source(&dir.path().join("Persistent.Mail.123"), b"mail");
            state.enqueue_upload(item.clone());
            state.flush(&file).expect("persist before upload");
            let (url, server) = respond_once(response(status, &body)).await;

            let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;
            let message =
                apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);
            server.await.expect("API fixture");
            state.flush(&file).expect("persist bounded failure");
            drop(file);

            let file = StateFile::open(dir.path()).expect("restart");
            let restored = file.load().expect("read bounded state");
            let reason = if status == "400 Bad Request" {
                &restored.rejected.values().next().expect("terminal rejection").reason
            } else {
                restored
                    .pending
                    .front()
                    .expect("retryable error")
                    .last_error
                    .as_ref()
                    .expect("retry reason")
            };

            assert!(restored.acknowledged.is_empty());
            assert!(reason.len() <= MAX_REASON_BYTES);
            assert!(message.len() <= MAX_REASON_BYTES);
            assert!(
                fs::metadata(dir.path().join("upload-state-v1.json")).expect("snapshot size").len()
                    < 4096
            );

            if body.len() > 8192 {
                assert!(reason.contains("response body exceeds 8192 bytes"));
            } else {
                assert!(reason.ends_with(" [truncated]"));
            }
        }
    }

    #[tokio::test]
    async fn oversized_local_mail_type_is_recorded_with_a_bounded_reason() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        let bytes = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"界".repeat(10_000)}))
            .expect("large unsupported type");
        let item = source(&dir.path().join("Persistent.Mail.123"), &bytes);
        state.enqueue_upload(item.clone());
        state.flush(&file).expect("save pending source");

        let message =
            process_upload(&mut state, &client(), "http://127.0.0.1:0", item, 10_000).await;
        state.flush(&file).expect("save bounded rejection");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let restored = file.load().expect("load bounded rejection");
        let rejection = restored.rejected.values().next().expect("local rejection");

        assert_eq!(rejection.reason, message);
        assert!(message.len() <= MAX_REASON_BYTES);
        assert!(message.ends_with(" [truncated]"));
        assert!(
            fs::metadata(dir.path().join("upload-state-v1.json")).expect("snapshot size").len()
                < 4096
        );
        assert!(restored.acknowledged.is_empty());
    }

    #[tokio::test]
    async fn cancelling_an_in_flight_upload_releases_the_lock_and_preserves_pending_work() {
        use tokio::{io::AsyncReadExt, net::TcpListener, sync::oneshot};

        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        let bytes =
            rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"})).expect("mail");
        let item = source(&dir.path().join("Persistent.Mail.123"), &bytes);
        state.enqueue_upload(item.clone());
        state.flush(&file).expect("save before starting request");

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("local API");
        let url = format!("http://{}/v2/upload", listener.local_addr().expect("API address"));
        let (received_tx, received_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            stream.read_u8().await.expect("request started");
            received_tx.send(()).expect("signal in-flight request");

            std::future::pending::<()>().await;
        });
        let watcher = tokio::spawn(async move {
            process_upload(&mut state, &client(), &url, item, 10_000).await;
            state.flush(&file).expect("save completed request");
        });

        tokio::time::timeout(Duration::from_secs(5), received_rx)
            .await
            .expect("request reached API")
            .expect("API signal");
        watcher.abort();

        assert!(watcher.await.expect_err("in-flight task cancelled").is_cancelled());

        server.abort();
        assert!(server.await.expect_err("fixture stopped").is_cancelled());

        let file = StateFile::open(dir.path()).expect("cancelled watcher released file lock");
        let restored = file.load().expect("restart after interruption");

        assert_eq!(restored.pending.len(), 1);
        assert!(restored.acknowledged.is_empty());
    }
}

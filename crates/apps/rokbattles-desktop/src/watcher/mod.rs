mod mail;
pub(crate) mod runtime;
mod scan;
mod state;
mod store;
mod upload;

use std::{
    fs,
    io::Read as _,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, SystemTime},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

pub(crate) use self::runtime::{WatcherTask, spawn_watcher};
use self::{
    mail::file_name_for_upload,
    state::WatcherState,
    store::{FileSig, QueuedUpload, StateFile, file_sig},
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

pub(crate) fn emit_log(app: &AppHandle, message: impl Into<String>) {
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

enum AttemptOutcome {
    Waiting(FileSig),
    Retry(String),
    Rejected(String),
    Response(Result<UploadStatus, UploadApiError>),
    Cancelled,
}

async fn attempt_upload(
    item: QueuedUpload,
    client: reqwest::Client,
    api_url: String,
    mut shutdown: watch::Receiver<bool>,
    now_ms: u128,
) -> (QueuedUpload, AttemptOutcome) {
    let source = item.clone();

    // Finish the read worker even after shutdown. It owns no mutable watcher state.
    let prepared =
        tauri::async_runtime::spawn_blocking(move || prepare_upload(&source, now_ms)).await;

    let (file_name, bytes) = match prepared {
        Ok(Ok(prepared)) => prepared,
        Ok(Err(outcome)) => return (item, outcome),
        Err(error) => {
            return (item, AttemptOutcome::Retry(format!("Mail preparation task failed: {error}")));
        }
    };

    let outcome = tokio::select! {
        biased;

        _ = shutdown.wait_for(|stopping| *stopping) => AttemptOutcome::Cancelled,
        result = post_file_to_api(&client, &api_url, &file_name, bytes) => {
            AttemptOutcome::Response(result)
        }
    };

    (item, outcome)
}

fn prepare_upload(item: &QueuedUpload, now_ms: u128) -> Result<(String, Vec<u8>), AttemptOutcome> {
    let path = PathBuf::from(&item.path);
    let Some(file_name) =
        file_name_for_upload(&path).filter(|_| mail::mail_id_for_path(&item.path).is_some())
    else {
        return Err(AttemptOutcome::Rejected(truncate_reason(format!(
            "Rejected invalid mail filename: {}",
            item.path
        ))));
    };

    let signature = fs::symlink_metadata(&path).map_err(anyhow::Error::from).and_then(|meta| {
        anyhow::ensure!(meta.is_file(), "Source is not a regular file");

        file_sig(&meta)
    });
    let sig = signature.map_err(|error| {
        AttemptOutcome::Retry(truncate_reason(format!("Failed to stat file {file_name}: {error}")))
    })?;

    if item.sig.as_ref() != Some(&sig) || now_ms.saturating_sub(sig.modified) < 1500 {
        return Err(AttemptOutcome::Waiting(sig));
    }

    let bytes = read_mail_source(&path, &sig).map_err(|error| {
        AttemptOutcome::Retry(truncate_reason(format!("Failed to read file {file_name}: {error}")))
    })?;
    let decoded = rokbattles_mail_codec::decode(&bytes).map_err(|error| {
        AttemptOutcome::Rejected(truncate_reason(format!("Decode failed for {file_name}: {error}")))
    })?;

    if rokbattles_mail_registry::detect_mail_type(&decoded).is_none() {
        let raw_type = rokbattles_mail_registry::raw_mail_type(&decoded).unwrap_or("Unknown");

        return Err(AttemptOutcome::Rejected(truncate_reason(format!(
            "Rejected unsupported mail {file_name} (detected: {raw_type})"
        ))));
    }

    Ok((file_name, bytes))
}

fn complete_attempt(
    state: &mut WatcherState,
    mut item: QueuedUpload,
    outcome: AttemptOutcome,
    now_ms: u128,
) -> String {
    let file_name =
        file_name_for_upload(Path::new(&item.path)).unwrap_or_else(|| item.path.clone());

    // A configuration refresh may have cancelled this source while its request was pending.
    if !state.is_pending(&item.path) {
        if let AttemptOutcome::Response(Err(error)) = &outcome
            && error.is_rate_limited()
        {
            let delay = error
                .retry_after()
                .filter(|delay| !delay.is_zero())
                .unwrap_or_else(|| upload_backoff(item.attempts.saturating_add(1)));

            state.extend_rate_limit(now_ms, delay);
        }

        return format!("Ignored completed attempt for removed source {file_name}");
    }

    match outcome {
        AttemptOutcome::Waiting(sig) => {
            item.sig = Some(sig);
            item.not_before_ms = Some(now_ms.saturating_add(750));
            item.last_error = Some("Waiting for file to stop changing".to_string());
            state.requeue_upload(item);

            format!("Waiting for stable file {file_name}")
        }
        AttemptOutcome::Retry(reason) => {
            let reason = truncate_reason(reason);
            state.retry_upload(item, reason.clone(), now_ms);

            reason
        }
        AttemptOutcome::Rejected(reason) => {
            state.reject_upload(item, reason.clone());

            reason
        }
        AttemptOutcome::Response(result) => {
            apply_upload_result(state, item, &file_name, result, now_ms)
        }
        AttemptOutcome::Cancelled => format!("Upload interrupted; {file_name} remains pending"),
    }
}

#[cfg(test)]
async fn process_upload(
    state: &mut WatcherState,
    client: &reqwest::Client,
    api_url: &str,
    item: QueuedUpload,
    now_ms: u128,
) -> String {
    let (_shutdown, receiver) = watch::channel(false);
    let (item, outcome) =
        attempt_upload(item, client.clone(), api_url.to_string(), receiver, now_ms).await;

    complete_attempt(state, item, outcome, now_ms)
}

fn read_mail_source(path: &Path, expected: &FileSig) -> anyhow::Result<Vec<u8>> {
    let mut options = fs::File::options();
    options.read(true);

    // A queued path may have been replaced since discovery or the stability check.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;

        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;

        options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
    }

    let mut file = options.open(path)?;
    let metadata = file.metadata()?;

    anyhow::ensure!(metadata.is_file(), "Source is not a regular file");
    anyhow::ensure!(file_sig(&metadata)? == *expected, "Source changed before reading");

    // Check and read the same handle so replacing the path cannot redirect this upload.
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    anyhow::ensure!(file_sig(&file.metadata()?)? == *expected, "Source changed while reading");

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        time::{Instant, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::{
        scan::{discover_source, reconcile_next_directory, set_watched_dirs},
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

    #[test]
    fn removed_source_cannot_reappear_from_a_late_response_but_rate_limits_still_apply() {
        let dir = tempfile::tempdir().expect("fixture directory");
        let item = source(&dir.path().join("Persistent.Mail.123"), b"mail");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item.clone());
        state.prune_removed_dirs_state(&[]);

        complete_attempt(
            &mut state,
            item.clone(),
            AttemptOutcome::Rejected("bad source".to_string()),
            1000,
        );
        complete_attempt(
            &mut state,
            item,
            AttemptOutcome::Response(Err(UploadApiError::Rejected {
                status: reqwest::StatusCode::TOO_MANY_REQUESTS,
                retry_after: Some(Duration::from_secs(7)),
                message: "rate limited".to_string(),
            })),
            1000,
        );

        assert!(state.uploads.pending.is_empty());
        assert!(state.uploads.rejected.is_empty());
        assert!(state.uploads.acknowledged.is_empty());
        assert_eq!(state.uploads.api_backoff_until_ms, Some(8000));
    }

    #[tokio::test]
    async fn missing_source_retries_and_valid_copy_acknowledges_the_id_after_restart() {
        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(file.load().expect("load"));
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
        let mut restored = WatcherState::new(file.load().expect("load"));
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
            let mut state = WatcherState::new(file.load().expect("load"));
            let path = dir.path().join("mail/Persistent.Mail.123");
            let bad = source(&path, &bytes);
            state.enqueue_upload(bad.clone());
            state.flush(&file).expect("save pending upload");

            let message =
                process_upload(&mut state, &client(), "http://127.0.0.1:0", bad, 10_000).await;
            state.flush(&file).expect("save local rejection");
            drop(file);

            let file = StateFile::open(dir.path()).expect("restart");
            let mut restored = WatcherState::new(file.load().expect("load"));
            let key = path.to_string_lossy().into_owned();

            assert!(restored.uploads.pending.is_empty());
            assert!(restored.uploads.acknowledged.is_empty());
            assert_eq!(restored.uploads.rejected.get(&key).expect("rejection").reason, message);

            let good_bytes = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"}))
                .expect("supported mail");
            source(&path, &good_bytes);
            restored.dirs.push(path.parent().expect("mail directory").to_path_buf());
            discover_source(&mut restored, path.clone(), 10_000);
            restored.flush(&file).expect("save changed input for reevaluation");

            assert!(restored.uploads.rejected.is_empty());
            assert_eq!(file.load().expect("pending after changed input").pending.len(), 1);
        }
    }

    #[tokio::test]
    async fn non_regular_source_preserves_pending_work_and_reason() {
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
        let mut state = WatcherState::new(file.load().expect("load"));
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
        let pending = restored.pending.front().expect("unusable source remains pending");

        assert!(message.contains("Source is not a regular file"));
        assert_eq!(pending.last_error.as_deref(), Some(message.as_str()));
        assert_eq!(pending.attempts, 1);
        assert!(restored.acknowledged.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn queued_source_replaced_by_symlink_is_not_uploaded_and_recovers_when_restored() {
        use std::{io::ErrorKind, net::TcpListener, os::unix::fs::symlink};

        let dir = tempfile::tempdir().expect("test directory");
        let bytes =
            rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"})).expect("mail");
        let path = dir.path().join("mail/Persistent.Mail.123");
        let item = source(&path, &bytes);
        let target = source(&dir.path().join("outside/Persistent.Mail.123"), &bytes);
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item.clone());

        fs::remove_file(&path).expect("remove queued regular file");
        symlink(&target.path, &path).expect("replace queued source with a symlink");

        // Exercise the open itself as well as process_upload's earlier metadata check.
        read_mail_source(&path, target.sig.as_ref().expect("target signature")).expect_err(
            "the read must not follow a symlink even when the target signature matches",
        );

        let listener = TcpListener::bind("127.0.0.1:0").expect("local ingress listener");
        listener.set_nonblocking(true).expect("nonblocking listener");
        let url = format!("http://{}", listener.local_addr().expect("listener address"));
        let message = process_upload(&mut state, &client(), &url, item, 10_000).await;

        assert!(message.contains("Source is not a regular file"));
        assert_eq!(
            listener.accept().expect_err("no upload connection").kind(),
            ErrorKind::WouldBlock
        );
        assert_eq!(state.uploads.pending.len(), 1);
        assert!(state.uploads.acknowledged.is_empty());

        fs::remove_file(&path).expect("remove symlink");
        source(&path, &bytes);
        let item = state.next_ready_upload(300_000).expect("restored regular source retries");
        let (url, server) =
            respond_once(response("201 Created", r#"{"status":"stored","mail_id":"123"}"#)).await;

        process_upload(&mut state, &client(), &url, item, 300_000).await;
        server.await.expect("restored source reaches ingress");

        assert!(state.uploads.acknowledged.contains("123"));
        assert!(state.uploads.pending.is_empty());
    }

    #[test]
    fn source_changed_after_the_stability_check_is_retried_before_decoding() {
        let dir = tempfile::tempdir().expect("test directory");
        let path = dir.path().join("Persistent.Mail.123");
        let item = source(&path, b"first partial write");

        source(&path, b"a later and longer partial write");

        let error = read_mail_source(&path, item.sig.as_ref().expect("original signature"))
            .expect_err("a changed source must not be decoded or rejected using the old signature");

        assert!(error.to_string().contains("Source changed before reading"));
    }

    #[tokio::test]
    async fn accepted_request_with_lost_response_recovers_through_skipped_acknowledgement() {
        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(file.load().expect("load"));
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
        let mut restored = WatcherState::new(file.load().expect("load"));
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
        let mut state = WatcherState::new(file.load().expect("load"));
        set_watched_dirs(&mut state, vec![dir.path().to_path_buf()], Instant::now());

        discover_source(&mut state, dir.path().join("Persistent.Mail.123"), 1000);
        state.flush(&file).expect("save discovered source");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let restored = file.load().expect("load");

        assert_eq!(restored.pending.len(), 1);
        assert!(restored.pending.front().expect("pending source").sig.is_none());
        assert!(restored.acknowledged.is_empty());
    }

    #[tokio::test]
    async fn unflushed_discovery_has_no_persistent_marker_that_blocks_rediscovery() {
        let dir = tempfile::tempdir().expect("test directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(file.load().expect("load"));
        source(&dir.path().join("Persistent.Mail.123"), b"discovered but not yet uploaded");
        let now = Instant::now();
        set_watched_dirs(&mut state, vec![dir.path().to_path_buf()], now);

        discover_source(&mut state, dir.path().join("Persistent.Mail.123"), 1000);
        drop(state);
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart before discovery was saved");
        let mut restarted = WatcherState::new(file.load().expect("load"));
        set_watched_dirs(&mut restarted, vec![dir.path().to_path_buf()], now);
        reconcile_next_directory(&mut restarted, now, 2000).await.expect("reconcile after restart");

        assert_eq!(restarted.uploads.pending.len(), 1);
        assert!(restarted.uploads.acknowledged.is_empty());
    }

    #[test]
    fn missing_event_source_remains_pending() {
        let mut state = WatcherState::new(UploadStore::default());
        let dir = tempfile::tempdir().expect("mail directory");
        state.dirs.push(dir.path().to_path_buf());

        discover_source(&mut state, dir.path().join("Persistent.Mail.123"), 1000);

        assert_eq!(state.uploads.pending.len(), 1);
        assert!(state.uploads.pending.front().expect("pending source").sig.is_none());
    }

    #[tokio::test]
    async fn retry_backlog_does_not_prevent_discovery_of_new_mail() {
        let dir = tempfile::tempdir().expect("mail directory");
        let mut state = WatcherState::new(UploadStore::default());
        let now = Instant::now();
        set_watched_dirs(&mut state, vec![dir.path().to_path_buf()], now);

        for id in 1..=4 {
            state.enqueue_upload(QueuedUpload {
                path: dir
                    .path()
                    .join(format!("Persistent.Mail.{id}"))
                    .to_string_lossy()
                    .into_owned(),
                sig: None,
                attempts: 1,
                not_before_ms: Some(20_000),
                last_error: Some("file unavailable".to_string()),
            });
        }

        let alternative = dir.path().join("Persistent.Mail.5");
        source(&alternative, b"available source");
        reconcile_next_directory(&mut state, now, 10_000).await.expect("scan despite retries");

        let ready = state.next_ready_upload(11_000).expect("available mail discovered");

        assert_eq!(ready.path, alternative.to_string_lossy());
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
            let mut state = WatcherState::new(file.load().expect("load"));
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
        let mut state = WatcherState::new(file.load().expect("load"));
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
        let mut state = WatcherState::new(file.load().expect("load"));
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

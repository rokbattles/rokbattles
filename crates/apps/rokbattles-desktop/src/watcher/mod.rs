//! Mail discovery, upload preparation, and recovery after interrupted uploads.
//!
//! [`scan`] combines filesystem notifications with periodic directory passes.
//! Discovered sources enter the pending queue in [`store::UploadStore`] before
//! an upload starts. [`upload`] validates ingress acknowledgements; a successful
//! acknowledgement covers all local copies of the same exact mail ID.
//!
//! [`runtime`] owns the mutable [`state::WatcherState`] and applies upload results.
//! Starting a request leaves its source pending, so interruption causes a retry
//! on restart. Rejected sources are reconsidered when their file signature changes.

mod mail;
pub(crate) mod runtime;
mod scan;
mod state;
mod store;
mod upload;

use std::{
    fs,
    io::{self, Read as _},
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
    store::{FileSig, QueuedUpload, UploadDb, file_sig, source_is_deleted},
    upload::{UploadApiError, UploadStatus, post_file_to_api, upload_backoff},
};

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

// Bound failure messages sent to the webview; error text is not persisted.
const MAX_REASON_BYTES: usize = 2048;

#[derive(Debug, Clone, Serialize)]
struct LogPayload {
    message: String,
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

/// Clears upload history and pending work so watched directories can be reprocessed.
///
/// Call after the watcher has finished. The database schema and version are retained.
///
/// # Errors
///
/// Returns an error if the watcher lock is held or the database cannot be opened,
/// validated, or updated. Invalid contents are retained for inspection.
pub fn reset_upload_state(app: &AppHandle) -> anyhow::Result<()> {
    UploadDb::for_app(app)?.reset()
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
                item.attempts = item.attempts.saturating_add(1).min(10);

                let backoff =
                    error.retry_after(now_ms).unwrap_or_else(|| upload_backoff(item.attempts));

                item.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));

                state.requeue_upload(item);

                if error.is_rate_limited() && state.extend_rate_limit(now_ms, backoff) {
                    let wait_secs = (backoff.as_millis().saturating_add(999) / 1000).max(1);

                    return format!("Rate limited by API (429). Pausing uploads for {wait_secs}s.");
                }
            } else {
                state.reject_upload(item);
            }

            truncate_reason(format!("Failed to upload {file_name}: {reason}"))
        }
    }
}

enum AttemptOutcome {
    Waiting(FileSig),
    DeletedSource,
    MissingSource,
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
    let sig = signature.map_err(|error| source_failure("stat", &path, error))?;

    if item.sig.as_ref() != Some(&sig) || now_ms.saturating_sub(sig.modified) < 1500 {
        return Err(AttemptOutcome::Waiting(sig));
    }

    let bytes =
        read_mail_source(&path, &sig).map_err(|error| source_failure("read", &path, error))?;
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

fn source_failure(operation: &str, path: &Path, error: anyhow::Error) -> AttemptOutcome {
    let file_name = file_name_for_upload(path).unwrap_or_else(|| path.display().to_string());
    let mut reason = format!("Failed to {operation} file {file_name}: {error}");
    if error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::PermissionDenied)
    {
        reason.push_str(". Select its directory again using the Add button; you do not need to remove it first.");
    }

    let reason = truncate_reason(reason);

    if error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::NotFound)
    {
        if source_is_deleted(path) {
            AttemptOutcome::DeletedSource
        } else {
            AttemptOutcome::MissingSource
        }
    } else {
        AttemptOutcome::Retry(reason)
    }
}

fn complete_attempt(
    state: &mut WatcherState,
    mut item: QueuedUpload,
    outcome: AttemptOutcome,
    now_ms: u128,
) -> Option<String> {
    let file_name =
        file_name_for_upload(Path::new(&item.path)).unwrap_or_else(|| item.path.clone());

    // A configuration refresh may have cancelled this source while its request was pending.
    if !state.is_pending(&item.path) {
        if let AttemptOutcome::Response(Err(error)) = &outcome
            && error.is_rate_limited()
        {
            let delay = error
                .retry_after(now_ms)
                .unwrap_or_else(|| upload_backoff(item.attempts.saturating_add(1)));

            state.extend_rate_limit(now_ms, delay);
        }

        return Some(format!("Ignored completed attempt for removed source {file_name}"));
    }

    match outcome {
        AttemptOutcome::Waiting(sig) => {
            item.sig = Some(sig);
            item.not_before_ms = Some(now_ms.saturating_add(750));

            state.requeue_upload(item);

            Some(format!("Waiting for stable file {file_name}"))
        }
        AttemptOutcome::DeletedSource => {
            // Removing this path does not acknowledge the mail ID or block rediscovery.
            state.remove_pending_source(&item.path);

            None
        }
        AttemptOutcome::MissingSource => {
            // An unavailable directory can also produce NotFound. Retain uncertain sources.
            state.retry_upload(item, now_ms);

            None
        }
        AttemptOutcome::Retry(reason) => {
            let reason = truncate_reason(reason);
            state.retry_upload(item, now_ms);

            Some(reason)
        }
        AttemptOutcome::Rejected(reason) => {
            state.reject_upload(item);

            Some(reason)
        }
        AttemptOutcome::Response(result) => {
            Some(apply_upload_result(state, item, &file_name, result, now_ms))
        }
        AttemptOutcome::Cancelled => {
            Some(format!("Upload interrupted; {file_name} remains pending"))
        }
    }
}

#[cfg(test)]
async fn process_upload(
    state: &mut WatcherState,
    client: &reqwest::Client,
    api_url: &str,
    item: QueuedUpload,
    now_ms: u128,
) -> Option<String> {
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
        }
    }

    #[test]
    fn removed_source_cannot_reappear_from_a_late_response_but_rate_limits_still_apply() {
        for (delay_seconds, expected_deadline) in [(7, 8000), (u64::MAX, 3000)] {
            let dir = tempfile::tempdir().expect("fixture directory");
            let db = UploadDb::open(dir.path()).expect("upload database");
            let item = source(&dir.path().join("Persistent.Mail.123"), b"mail");
            let mut state = WatcherState::new(db.load().expect("load"));
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
                    retry_after: Some(Duration::from_secs(delay_seconds)),
                    message: "rate limited".to_string(),
                })),
                1000,
            );
            state.flush(&db).expect("persist rate limit from a removed source");
            drop(db);

            let db = UploadDb::open(dir.path()).expect("restart");
            let restored = db.load().expect("load rate limit");

            assert!(state.uploads.pending.is_empty());
            assert!(state.uploads.rejected.is_empty());
            assert!(state.uploads.acknowledged.is_empty());
            assert!(restored.pending.is_empty());
            assert!(restored.rejected.is_empty());
            assert!(restored.acknowledged.is_empty());
            assert_eq!(restored.api_backoff_until_ms, Some(expected_deadline));
        }
    }

    #[tokio::test]
    async fn unavailable_directory_retry_survives_restart_and_other_copy_can_upload() {
        let dir = tempfile::tempdir().expect("test directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        let bytes = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"}))
            .expect("mail bytes");
        let missing = source(&dir.path().join("missing/Persistent.Mail.123"), &bytes);
        let good = source(&dir.path().join("good/Persistent.Mail.123"), &bytes);
        fs::remove_dir_all(Path::new(&missing.path).parent().expect("source directory"))
            .expect("directory becomes temporarily unavailable");
        state.enqueue_upload(missing.clone());
        state.enqueue_upload(good.clone());
        state.flush(&db).expect("save pending sources");

        let message =
            process_upload(&mut state, &client(), "http://127.0.0.1:0", missing.clone(), 10_000)
                .await;
        state.flush(&db).expect("save metadata retry");
        drop(db);

        assert!(message.is_none(), "unavailable cache retries stay quiet");

        let db = UploadDb::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(db.load().expect("load"));
        let retry = restored
            .uploads
            .pending
            .iter()
            .find(|item| item.path == missing.path)
            .expect("missing source remains pending");

        assert_eq!(retry.attempts, 1);
        assert!(retry.not_before_ms.expect("retry deadline") > 10_000);

        let ready = restored.next_ready_upload(10_000).expect("alternative source is ready");

        assert_eq!(ready.path, good.path);
        assert_eq!(restored.uploads.pending.len(), 2);

        let (url, server) =
            respond_once(response("201 Created", r#"{"status":"stored","mail_id":"123"}"#)).await;
        let message = process_upload(&mut restored, &client(), &url, ready, 10_000)
            .await
            .expect("upload acknowledgement is logged");
        server.await.expect("server received mail");
        restored.flush(&db).expect("save server acknowledgement");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart after acknowledgement");
        let completed = db.load().expect("load");

        assert!(message.starts_with("Stored new mail"));
        assert!(completed.pending.is_empty());
        assert!(completed.acknowledged.contains("123"));
    }

    #[test]
    fn collects_kau_karuak_rewards_but_rejects_failure_mail() {
        let dir = tempfile::tempdir().expect("test directory");
        let reward_bytes =
            include_bytes!("../../../../../samples/System/Persistent.Mail.49147831179110293022");
        let reward = source(&dir.path().join("Persistent.Mail.49147831179110293022"), reward_bytes);

        let Ok((_, prepared)) = prepare_upload(&reward, 10_000) else {
            panic!("Kau Karuak reward should be collected");
        };

        assert_eq!(prepared, reward_bytes);

        let failure_bytes = rokbattles_mail_codec::encode(&json!({
            "id": "123",
            "type": "System",
            "body": {"subType": 150, "subParam": 0}
        }))
        .expect("synthetic failure mail");
        let failure = source(&dir.path().join("Persistent.Mail.123"), &failure_bytes);

        assert!(matches!(prepare_upload(&failure, 10_000), Err(AttemptOutcome::Rejected(_))));
    }

    #[tokio::test]
    async fn invalid_and_unsupported_files_remain_rejected_until_the_source_changes() {
        for bytes in [
            b"invalid binary".to_vec(),
            rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Unsupported"}))
                .expect("unsupported mail"),
        ] {
            let dir = tempfile::tempdir().expect("test directory");
            let db = UploadDb::open(dir.path()).expect("upload database");
            let mut state = WatcherState::new(db.load().expect("load"));
            let path = dir.path().join("mail/Persistent.Mail.123");
            let bad = source(&path, &bytes);
            state.enqueue_upload(bad.clone());
            state.flush(&db).expect("save pending upload");

            process_upload(&mut state, &client(), "http://127.0.0.1:0", bad, 10_000)
                .await
                .expect("invalid mail is logged");
            state.flush(&db).expect("save local rejection");
            drop(db);

            let db = UploadDb::open(dir.path()).expect("restart");
            let mut restored = WatcherState::new(db.load().expect("load"));
            let key = path.to_string_lossy().into_owned();

            assert!(restored.uploads.pending.is_empty());
            assert!(restored.uploads.acknowledged.is_empty());
            assert!(restored.uploads.rejected.contains_key(&key));

            let good_bytes = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"}))
                .expect("supported mail");
            source(&path, &good_bytes);
            restored.dirs.push(path.parent().expect("mail directory").to_path_buf());
            discover_source(&mut restored, path.clone(), 10_000);
            restored.flush(&db).expect("save changed input for reevaluation");

            assert!(restored.uploads.rejected.is_empty());
            assert_eq!(db.load().expect("pending after changed input").pending.len(), 1);
        }
    }

    #[tokio::test]
    async fn non_regular_source_preserves_pending_work() {
        let dir = tempfile::tempdir().expect("test directory");
        let path = dir.path().join("Persistent.Mail.123");
        fs::create_dir(&path).expect("path with metadata that cannot be read as mail");
        let item = QueuedUpload {
            path: path.to_string_lossy().into_owned(),
            sig: Some(file_sig(&fs::metadata(&path).expect("metadata")).expect("signature")),
            attempts: 0,
            not_before_ms: None,
        };
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item.clone());
        state.flush(&db).expect("persist pending source");

        let message = process_upload(
            &mut state,
            &client(),
            "http://127.0.0.1:0",
            item,
            now_epoch_ms() + 2000,
        )
        .await
        .expect("non-regular source is logged");
        state.flush(&db).expect("persist failed read");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let restored = db.load().expect("load");
        let pending = restored.pending.front().expect("unusable source remains pending");

        assert!(message.contains("Source is not a regular file"));
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

        // Check the open itself as well as upload preparation's earlier metadata check.
        read_mail_source(&path, target.sig.as_ref().expect("target signature")).expect_err(
            "the read must not follow a symlink even when the target signature matches",
        );

        let listener = TcpListener::bind("127.0.0.1:0").expect("local ingress listener");
        listener.set_nonblocking(true).expect("nonblocking listener");
        let url = format!("http://{}", listener.local_addr().expect("listener address"));
        let message = process_upload(&mut state, &client(), &url, item, 10_000)
            .await
            .expect("symlink source is logged");

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

    #[test]
    fn source_removed_after_the_stability_check_is_forgotten_quietly() {
        let dir = tempfile::tempdir().expect("test directory");
        let path = dir.path().join("Persistent.Mail.123");
        let item = source(&path, b"cached mail");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item.clone());
        fs::remove_file(&path).expect("game deletes mail before it can be opened");

        let error = read_mail_source(&path, item.sig.as_ref().expect("original signature"))
            .expect_err("source disappeared after metadata was read");
        let outcome = source_failure("read", &path, error);
        let message = complete_attempt(&mut state, item, outcome, 10_000);

        assert!(message.is_none());
        assert!(state.uploads.pending.is_empty());
        assert!(!state.is_pending(path.to_str().expect("source path")));
        assert!(state.uploads.acknowledged.is_empty());
        assert!(state.uploads.rejected.is_empty());
    }

    #[tokio::test]
    async fn deleted_source_does_not_cancel_another_copy_of_the_same_mail() {
        let dir = tempfile::tempdir().expect("test directory");
        let missing = source(&dir.path().join("first/Persistent.Mail.123"), b"mail");
        let other = source(&dir.path().join("second/Persistent.Mail.123"), b"mail");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(missing.clone());
        state.enqueue_upload(other.clone());
        fs::remove_file(&missing.path).expect("game deletes one copy");

        let message =
            process_upload(&mut state, &client(), "http://127.0.0.1:0", missing.clone(), 10_000)
                .await;

        assert!(message.is_none());
        assert!(!state.is_pending(&missing.path));
        assert_eq!(state.next_ready_upload(10_000).expect("other copy survives").path, other.path);
        assert_eq!(state.uploads.pending.len(), 1);
        assert!(state.uploads.acknowledged.is_empty());
        assert!(state.uploads.rejected.is_empty());
    }

    #[test]
    fn source_that_reappears_before_deletion_is_confirmed_remains_pending() {
        let dir = tempfile::tempdir().expect("test directory");
        let path = dir.path().join("Persistent.Mail.123");
        let item = source(&path, b"cached mail");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item.clone());
        fs::remove_file(&path).expect("source temporarily disappears");
        let error = read_mail_source(&path, item.sig.as_ref().expect("signature"))
            .expect_err("source is missing during the read");
        source(&path, b"mail returns before the failure is classified");

        let outcome = source_failure("read", &path, error);
        let message = complete_attempt(&mut state, item.clone(), outcome, 10_000);

        assert!(message.is_none());
        assert!(state.is_pending(&item.path));
        assert!(state.uploads.acknowledged.is_empty());
        assert!(state.uploads.rejected.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn missing_file_is_retained_until_its_directory_can_be_read() {
        use std::os::unix::fs::PermissionsExt as _;

        // SAFETY: geteuid takes no arguments and has no preconditions.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let dir = tempfile::tempdir().expect("test directory");
        let cache = dir.path().join("mailcache");
        let item = source(&cache.join("Persistent.Mail.123"), b"cached mail");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item.clone());
        fs::remove_file(&item.path).expect("source disappears");
        let permissions = fs::metadata(&cache).expect("directory metadata").permissions();
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o100))
            .expect("allow path lookup but deny directory listing");

        let message =
            process_upload(&mut state, &client(), "http://127.0.0.1:0", item.clone(), 10_000).await;
        fs::set_permissions(&cache, permissions).expect("restore directory access");

        assert!(message.is_none());
        assert!(state.is_pending(&item.path));
        let retry = state.next_ready_upload(30_000).expect("uncertain source remains queued");
        let message =
            process_upload(&mut state, &client(), "http://127.0.0.1:0", retry, 30_000).await;

        assert!(message.is_none());
        assert!(state.uploads.pending.is_empty());
        assert!(state.uploads.acknowledged.is_empty());
        assert!(state.uploads.rejected.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_source_logs_the_error_and_preserves_pending_work() {
        use std::os::unix::fs::PermissionsExt as _;

        // Root can read files regardless of their permission bits.
        // SAFETY: geteuid takes no arguments and has no preconditions.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let dir = tempfile::tempdir().expect("test directory");
        let path = dir.path().join("Persistent.Mail.123");
        let bytes =
            rokbattles_mail_codec::encode(&serde_json::json!({"id":"123", "type":"Battle"}))
                .expect("mail bytes");
        let item = source(&path, &bytes);
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item.clone());
        let permissions = fs::metadata(&path).expect("source metadata").permissions();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("deny read access");

        let message = process_upload(&mut state, &client(), "http://127.0.0.1:0", item, 10_000)
            .await
            .expect("permission failures are logged");
        let pending = state.uploads.pending.front().expect("unreadable source remains pending");

        fs::set_permissions(&path, permissions).expect("restore read access");

        assert!(message.contains("Failed to read"));
        assert!(message.contains("the Add button"));
        assert_eq!(pending.attempts, 1);
        assert!(state.uploads.acknowledged.is_empty());
        assert!(state.uploads.rejected.is_empty());

        let retry = state.next_ready_upload(30_000).expect("permission failure remains retryable");
        let (url, server) =
            respond_once(response("201 Created", r#"{"status":"stored","mail_id":"123"}"#)).await;
        process_upload(&mut state, &client(), &url, retry, 30_000).await;
        server.await.expect("report uploaded after restoring access");

        assert!(state.uploads.acknowledged.contains("123"));
        assert!(state.uploads.pending.is_empty());
    }

    #[tokio::test]
    async fn accepted_request_with_lost_response_recovers_through_skipped_acknowledgement() {
        let dir = tempfile::tempdir().expect("test directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        let bytes =
            rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"})).expect("mail");
        let item = source(&dir.path().join("mail/Persistent.Mail.123"), &bytes);
        state.enqueue_upload(item.clone());
        state.flush(&db).expect("save before request");

        let (url, server) = respond_once(String::new()).await;
        process_upload(&mut state, &client(), &url, item, 10_000).await;
        server.await.expect("request reached server but response was lost");
        state.flush(&db).expect("save retry");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(db.load().expect("load"));
        let pending = restored
            .next_ready_upload(now_epoch_ms() + 300_000)
            .expect("uncertain request remains pending");
        let (url, server) =
            respond_once(response("200 OK", r#"{"status":"skipped","mail_id":"123"}"#)).await;

        process_upload(&mut restored, &client(), &url, pending, now_epoch_ms()).await;
        server.await.expect("idempotent retry");
        restored.flush(&db).expect("save acknowledgement");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let completed = db.load().expect("load");

        assert!(completed.acknowledged.contains("123"));
        assert!(completed.pending.is_empty());
    }

    #[test]
    fn discovery_with_unavailable_metadata_survives_restart() {
        let dir = tempfile::tempdir().expect("test directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        set_watched_dirs(&mut state, vec![dir.path().to_path_buf()], Instant::now());

        discover_source(&mut state, dir.path().join("Persistent.Mail.123"), 1000);
        state.flush(&db).expect("save discovered source");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let restored = db.load().expect("load");

        assert_eq!(restored.pending.len(), 1);
        assert!(restored.pending.front().expect("pending source").sig.is_none());
        assert!(restored.acknowledged.is_empty());
    }

    #[tokio::test]
    async fn unflushed_discovery_has_no_persistent_marker_that_blocks_rediscovery() {
        let dir = tempfile::tempdir().expect("test directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        source(&dir.path().join("Persistent.Mail.123"), b"discovered but not yet uploaded");
        let now = Instant::now();
        set_watched_dirs(&mut state, vec![dir.path().to_path_buf()], now);

        discover_source(&mut state, dir.path().join("Persistent.Mail.123"), 1000);
        drop(state);
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart before discovery was saved");
        let mut restarted = WatcherState::new(db.load().expect("load"));
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
            });
        }

        let alternative = dir.path().join("Persistent.Mail.5");
        source(&alternative, b"available source");
        reconcile_next_directory(&mut state, now, 10_000).await.expect("scan despite retries");

        let ready = state.next_ready_upload(11_000).expect("available mail discovered");

        assert_eq!(ready.path, alternative.to_string_lossy());
    }

    #[tokio::test]
    async fn oversized_api_failures_log_bounded_messages_and_preserve_retry_classification() {
        for (status, body) in [
            ("400 Bad Request", "é".repeat(4000)),
            ("503 Service Unavailable", "é".repeat(4000)),
            ("400 Bad Request", "x".repeat(10_000)),
            ("200 OK", "x".repeat(10_000)),
        ] {
            let dir = tempfile::tempdir().expect("state directory");
            let db = UploadDb::open(dir.path()).expect("upload database");
            let mut state = WatcherState::new(db.load().expect("load"));
            let item = source(&dir.path().join("Persistent.Mail.123"), b"mail");
            state.enqueue_upload(item.clone());
            state.flush(&db).expect("persist before upload");
            let (url, server) = respond_once(response(status, &body)).await;

            let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;
            let message =
                apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);
            server.await.expect("API fixture");
            state.flush(&db).expect("persist retry or rejection");
            drop(db);

            let db = UploadDb::open(dir.path()).expect("restart");
            let restored = db.load().expect("read upload state");
            assert!(restored.acknowledged.is_empty());
            assert!(message.len() <= MAX_REASON_BYTES);
            if status == "400 Bad Request" {
                assert_eq!(restored.rejected.len(), 1);
            } else {
                assert_eq!(restored.pending.len(), 1);
            }

            if body.len() > 8192 {
                assert!(message.contains("response body exceeds 8192 bytes"));
            } else {
                assert!(message.ends_with(" [truncated]"));
            }
        }
    }

    #[tokio::test]
    async fn oversized_local_mail_type_logs_bounded_message_and_saves_rejection_signature() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        let bytes = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"界".repeat(10_000)}))
            .expect("large unsupported type");
        let item = source(&dir.path().join("Persistent.Mail.123"), &bytes);
        state.enqueue_upload(item.clone());
        state.flush(&db).expect("save pending source");

        let message = process_upload(&mut state, &client(), "http://127.0.0.1:0", item, 10_000)
            .await
            .expect("unsupported mail is logged");
        state.flush(&db).expect("save rejection signature");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let restored = db.load().expect("load rejection signature");
        let rejection = restored.rejected.values().next().expect("local rejection");

        assert!(rejection.sig.is_some());
        assert!(message.len() <= MAX_REASON_BYTES);
        assert!(message.ends_with(" [truncated]"));
        assert!(restored.acknowledged.is_empty());
    }
}

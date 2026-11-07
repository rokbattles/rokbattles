//! Coordinates directory scans, mail capture, uploads, and shutdown.
//!
//! Capture workers continue reading and classifying mail during API backoff.
//! Supported bytes and queue updates commit together before uploads start. Upload
//! retries read those retained bytes rather than reopening game files.
//!
//! The worker holds the database lock until shutdown completes. Shutdown cancels
//! HTTP requests and joins capture workers before saving their remaining results.
//! Join the worker before starting another watcher or resetting upload history;
//! blocking filesystem operations may continue after a stop request.

use std::{
    collections::BTreeMap,
    future::Future,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use rustc_hash::FxHashSet;
use tauri::{AppHandle, Manager};
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinSet,
};

use super::{
    AttemptOutcome, attempt_upload,
    capture::{self, PreparedBatch},
    complete_attempt, emit_log, http_client,
    mail::mail_id_for_path,
    now_epoch_ms,
    scan::{
        discover_source, queue_fs_event, reconcile_next_directory, refresh_watched_dirs,
        request_reconciliation, sync_fs_watches,
    },
    state::WatcherState,
    store::{MAX_PREPARED_BYTES, MAX_PREPARED_MAILS, UploadDb},
    truncate_reason,
};

// Bound network concurrency separately from source readers.
const MAX_CONCURRENT_UPLOADS: usize = 4;

pub(crate) enum TaskEvent {
    /// The worker holds the database lock and has loaded state. Scanning may not have started.
    Ready,
    Finished(anyhow::Result<()>),
}

/// A handle for observing and stopping a watcher worker.
///
/// Dropping the handle requests a stop. Call [`Self::join`] to wait for file reads
/// and the final state write before starting another watcher or resetting history.
pub(crate) struct WatcherTask {
    shutdown: watch::Sender<bool>,
    handle: tauri::async_runtime::JoinHandle<anyhow::Result<()>>,
    ready: oneshot::Receiver<()>,
    ready_received: bool,
}

impl WatcherTask {
    pub(crate) fn spawn<F, Fut>(run: F) -> Self
    where
        F: FnOnce(watch::Receiver<bool>, oneshot::Sender<()>) -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let (shutdown, receiver) = watch::channel(false);
        let (ready_sender, ready) = oneshot::channel();

        // Filesystem and database operations block this thread. Join it on shutdown
        // so a replacement watcher cannot start while those operations are still running.
        let handle = tauri::async_runtime::spawn_blocking(move || {
            tauri::async_runtime::block_on(run(receiver, ready_sender))
        });

        Self { shutdown, handle, ready, ready_received: false }
    }

    pub(crate) fn request_stop(&self) {
        self.shutdown.send_replace(true);
    }

    pub(crate) async fn join(&mut self) -> anyhow::Result<()> {
        (&mut self.handle).await.context("Watcher task failed")?
    }

    pub(crate) async fn next_event(&mut self) -> TaskEvent {
        tokio::select! {
            biased;

            result = &mut self.handle => {
                TaskEvent::Finished(result.context("Watcher task failed").and_then(|result| result))
            }
            result = &mut self.ready, if !self.ready_received => {
                self.ready_received = true;

                if result.is_ok() {
                    TaskEvent::Ready
                } else {
                    TaskEvent::Finished(self.join().await)
                }
            }
        }
    }
}

impl Drop for WatcherTask {
    fn drop(&mut self) {
        self.request_stop();
    }
}

pub(crate) fn spawn_watcher(app: &AppHandle) -> WatcherTask {
    let app = app.clone();

    WatcherTask::spawn(move |shutdown, ready| async move {
        let dir = app.path().app_config_dir().context("Could not resolve app config directory")?;

        run_watcher(
            dir,
            || {
                crate::app_config::read_config(&app).map(|config| {
                    #[cfg(target_os = "macos")]
                    crate::directory_access::restore(&app, &config);
                    config.dirs.map(|dirs| {
                        crate::dedupe_dirs_preserving_representation(dirs)
                            .into_iter()
                            .map(PathBuf::from)
                            .collect()
                    })
                })
            },
            |message| emit_log(&app, message),
            http_client().clone(),
            "https://ingress.rokbattles.com/v2/upload".to_string(),
            shutdown,
            ready,
        )
        .await
    })
}

async fn run_watcher(
    config_dir: PathBuf,
    read_dirs: impl Fn() -> anyhow::Result<Option<Vec<PathBuf>>>,
    log: impl Fn(String),
    client: reqwest::Client,
    api_url: String,
    mut shutdown: watch::Receiver<bool>,
    ready: oneshot::Sender<()>,
) -> anyhow::Result<()> {
    let db = UploadDb::open(&config_dir)?;
    let mut state = WatcherState::new(db.load()?);
    let (fs_tx, mut fs_rx) = mpsc::channel::<PathBuf>(4096);
    let rescan_requested = Arc::new(AtomicBool::new(false));

    let mut fs_watcher = match notify::recommended_watcher({
        let rescan_requested = Arc::clone(&rescan_requested);

        move |event| queue_fs_event(&fs_tx, &rescan_requested, event)
    }) {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            log(format!("Failed to initialize filesystem watcher: {error}"));
            None
        }
    };
    let mut watched_dirs = FxHashSet::default();
    let mut uploads = JoinSet::new();
    let mut preparations = JoinSet::new();
    let mut capture_blocked_at = None;
    let mut next_cleanup = Instant::now();
    let (upload_shutdown, upload_receiver) = watch::channel(false);
    let _ready_result = ready.send(());

    let result = 'watching: loop {
        if *shutdown.borrow() {
            break Ok(());
        }

        let now = Instant::now();
        let now_ms = now_epoch_ms();
        refresh_watched_dirs(&mut state, now, &read_dirs, &log);
        sync_fs_watches(&log, fs_watcher.as_mut(), &mut watched_dirs, &state.dirs);

        if rescan_requested.swap(false, Ordering::Relaxed) {
            request_reconciliation(&mut state, now);
        }

        for _ in 0..512 {
            let Ok(path) = fs_rx.try_recv() else {
                break;
            };

            discover_source(&mut state, path, now_ms);
        }

        let scanned = match reconcile_next_directory(&mut state, now, now_ms).await {
            Ok(scanned) => scanned,
            Err(error) => {
                log(format!("{error:#}"));
                true
            }
        };

        // Process completed responses before starting more requests so a pending
        // HTTP 429 takes effect even when another request succeeds.
        while let Some(result) = uploads.try_join_next() {
            match result {
                Ok((item, outcome)) => {
                    if let Some(message) =
                        complete_attempt(&mut state, item, outcome, now_epoch_ms())
                    {
                        log(message);
                    }
                }
                Err(error) => break 'watching Err(error.into()),
            }
        }

        while let Some(result) = preparations.try_join_next() {
            match result {
                Ok(Ok(batch)) => {
                    complete_preparation(&mut state, batch, &mut capture_blocked_at, &log)
                }
                Ok(Err(error)) => break 'watching Err(error),
                Err(error) => break 'watching Err(error.into()),
            }
        }

        // Persist newly discovered sources and completed attempts before starting more work.
        if let Err(error) = state.flush(&db) {
            break Err(error.context("Watcher stopped; failed to save upload state"));
        }

        let usage = (state.prepared_bytes, state.uploads.prepared.len());
        if capture_blocked_at.is_some_and(|blocked| usage < blocked) {
            capture_blocked_at = None;
        }
        if !state.capture_has_room() {
            block_capture(&state, &mut capture_blocked_at, &log);
        }

        if capture_blocked_at.is_none() {
            start_capture_batches(&mut state, &mut preparations, &shutdown);
        }

        if let Err(error) = start_pending_uploads(
            &mut state,
            &db,
            &mut uploads,
            &client,
            &api_url,
            &shutdown,
            &upload_receiver,
        ) {
            break Err(error);
        }

        let mut reclaimed = false;
        // Reclaim space in bounded steps while no capture or upload needs the database.
        if uploads.is_empty()
            && preparations.is_empty()
            && state.uploads.prepared.is_empty()
            && Instant::now() >= next_cleanup
        {
            match db.reclaim_unused_space() {
                Ok(worked) => reclaimed = worked,
                Err(error) => {
                    log(format!("Failed to reclaim unused mail storage: {error:#}"));
                    next_cleanup = Instant::now() + Duration::from_secs(30);
                }
            }
        }

        tokio::select! {
            biased;

            _ = shutdown.changed() => break Ok(()),
            result = preparations.join_next(), if !preparations.is_empty() => {
                match result {
                    Some(Ok(Ok(batch))) => complete_preparation(&mut state, batch, &mut capture_blocked_at, &log),
                    Some(Ok(Err(error))) => break Err(error),
                    Some(Err(error)) => break Err(error.into()),
                    None => {}
                }
            }
            result = uploads.join_next(), if !uploads.is_empty() => {
                match result {
                    Some(Ok((item, outcome))) => {
                        if let Some(message) =
                            complete_attempt(&mut state, item, outcome, now_epoch_ms())
                        {
                            log(message);
                        }
                    }
                    Some(Err(error)) => break Err(error.into()),
                    None => {}
                }
            }
            _ = tokio::task::yield_now(), if scanned || reclaimed => {}
            path = fs_rx.recv(), if fs_watcher.is_some() => {
                if let Some(path) = path {
                    discover_source(&mut state, path, now_epoch_ms());
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(750)) => {}
        }
    };

    // Wait for file reads before releasing the database lock. Interrupted requests
    // remain pending for the next run.
    upload_shutdown.send_replace(true);

    while let Some(result) = preparations.join_next().await {
        match result {
            Ok(Ok(batch)) => complete_preparation(&mut state, batch, &mut capture_blocked_at, &log),
            Ok(Err(error)) => log(format!("Mail preparation failed during shutdown: {error:#}")),
            Err(error) => log(format!("Mail preparation task failed during shutdown: {error}")),
        }
    }

    while let Some(outcome) = uploads.join_next().await {
        match outcome {
            Ok((item, outcome)) => {
                if let Some(message) = complete_attempt(&mut state, item, outcome, now_epoch_ms()) {
                    log(message);
                }
            }
            Err(error) => log(format!("Upload task failed during shutdown: {error}")),
        }
    }

    state.flush(&db).context("Failed to save upload state during shutdown")?;

    result
}

fn start_capture_batches(
    state: &mut WatcherState,
    preparations: &mut JoinSet<anyhow::Result<PreparedBatch>>,
    shutdown: &watch::Receiver<bool>,
) {
    let now_ms = now_epoch_ms();

    while !*shutdown.borrow()
        && preparations.len() < capture::READER_COUNT
        && state.capture_has_room()
    {
        let mut items = Vec::with_capacity(capture::BATCH_FILES);

        for _ in 0..capture::BATCH_FILES {
            let Some(item) = state.next_preparation(now_ms) else {
                break;
            };
            items.push(item);
        }

        if items.is_empty() {
            break;
        }

        preparations.spawn(capture::prepare_batch(items, shutdown.clone()));
    }
}

fn start_pending_uploads(
    state: &mut WatcherState,
    db: &UploadDb,
    uploads: &mut JoinSet<(super::store::QueuedUpload, AttemptOutcome)>,
    client: &reqwest::Client,
    api_url: &str,
    shutdown: &watch::Receiver<bool>,
    attempt_shutdown: &watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let now_ms = now_epoch_ms();

    while !*shutdown.borrow()
        && uploads.len() < MAX_CONCURRENT_UPLOADS
        && state.rate_limit_remaining_ms(now_ms).is_none()
        && let Some(item) = state.next_ready_upload(now_ms)
    {
        let id = mail_id_for_path(&item.path).context("Prepared source has no mail ID")?;
        let bytes =
            db.read_prepared(id).context("Failed to read retained mail; pending work preserved")?;
        uploads.spawn(attempt_upload(
            item,
            bytes,
            client.clone(),
            api_url.to_string(),
            attempt_shutdown.clone(),
        ));
    }

    Ok(())
}

fn complete_preparation(
    state: &mut WatcherState,
    batch: PreparedBatch,
    blocked_at: &mut Option<(u64, usize)>,
    log: &impl Fn(String),
) {
    let mut captured = 0;
    let mut unsupported = BTreeMap::<String, usize>::new();

    for source in batch.sources {
        match source.result {
            Ok(bytes) => {
                let pending = state.is_pending(&source.item.path);
                if !state.stage_upload(source.item, bytes) {
                    block_capture(state, blocked_at, log);
                } else if pending {
                    captured += 1;
                }
            }
            Err(outcome) => {
                let unsupported_type = match &outcome {
                    AttemptOutcome::Unsupported { mail_type, .. }
                        if state.is_pending(&source.item.path) =>
                    {
                        Some(mail_type.clone())
                    }
                    _ => None,
                };
                let message = complete_attempt(state, source.item, outcome, now_epoch_ms());

                if let Some(mail_type) = unsupported_type {
                    *unsupported.entry(mail_type).or_default() += 1;
                } else if let Some(message) = message {
                    log(message);
                }
            }
        }
    }

    for item in batch.deferred {
        state.requeue_upload(item);
    }

    if captured > 0 || !unsupported.is_empty() {
        // Summarize classification so large unsupported backlogs cannot flood the webview.
        let mut message = format!("Read {captured} supported mails for upload.");

        if !unsupported.is_empty() {
            let types = unsupported
                .into_iter()
                .map(|(name, count)| format!("{name}: {count}"))
                .collect::<Vec<_>>()
                .join(", ");
            message.push_str(&format!(" Skipped unsupported mails ({types})."));
        }

        log(truncate_reason(message));
    }
}

fn block_capture(
    state: &WatcherState,
    blocked_at: &mut Option<(u64, usize)>,
    log: &impl Fn(String),
) {
    if blocked_at.is_none() {
        log(format!(
            "Local mail backlog reached its storage limit ({} MiB of payloads or {MAX_PREPARED_MAILS} mails). New mail cannot be preserved until uploads free space.",
            MAX_PREPARED_BYTES / (1024 * 1024)
        ));
    }

    *blocked_at = Some((state.prepared_bytes, state.uploads.prepared.len()));
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, sync::Mutex, time::UNIX_EPOCH};

    use serde_json::json;
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    use super::*;
    use crate::watcher::{
        store::{QueuedUpload, UploadStore, file_sig},
        upload::tests::{respond_once, response},
    };

    fn write_mail(dir: &Path, id: u32) {
        fs::create_dir_all(dir).expect("mail directory");
        let path = dir.join(format!("Persistent.Mail.{id}"));
        let bytes = rokbattles_mail_codec::encode(&json!({"id": id, "type": "Battle"}))
            .expect("encoded mail");
        fs::write(&path, bytes).expect("mail file");
        fs::File::options()
            .write(true)
            .open(path)
            .expect("mail handle")
            .set_modified(UNIX_EPOCH + Duration::from_secs(1))
            .expect("stable timestamp");
    }

    fn start_fixture(config_dir: PathBuf, mail_dir: PathBuf, api_url: String) -> WatcherTask {
        WatcherTask::spawn(move |shutdown, ready| async move {
            run_watcher(
                config_dir,
                || Ok(Some(vec![mail_dir.clone()])),
                |_| {},
                reqwest::Client::new(),
                api_url,
                shutdown,
                ready,
            )
            .await
        })
    }

    async fn wait_for_uploads(dir: &Path, matches: impl Fn(&UploadStore) -> bool) -> UploadStore {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(store) = UploadDb::read_for_test(dir)
                    && matches(&store)
                {
                    return store;
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("expected durable watcher state")
    }

    #[tokio::test]
    async fn rate_limited_capture_survives_game_deletion_reprocess_and_restart() {
        use crate::watcher::{store::StoreChange, upload::tests::read_request};

        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");
        write_mail(&mail, 1);
        let original = fs::read(mail.join("Persistent.Mail.1")).expect("original bytes");
        fs::write(
            mail.join("Persistent.Mail.99"),
            rokbattles_mail_codec::encode(&json!({"id":"99", "type":"Unsupported"}))
                .expect("unsupported bytes"),
        )
        .expect("unsupported source");
        let db = UploadDb::open(&config).expect("database");
        db.commit(&[StoreChange::Backoff(Some(now_epoch_ms() + 60_000))]).expect("rate limit");
        drop(db);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("API listener");
        let url = format!("http://{}/v2/upload", listener.local_addr().expect("API address"));
        let mut watcher = start_fixture(config.clone(), mail.clone(), url.clone());
        wait_for_uploads(&config, |store| {
            store.prepared.contains_key("1") && store.rejected.len() == 1
        })
        .await;
        write_mail(&mail, 2);
        wait_for_uploads(&config, |store| store.prepared.len() == 2).await;
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .expect_err("backoff prevents HTTP but not capture");
        watcher.request_stop();
        watcher.join().await.expect("capture stops cleanly");
        fs::remove_file(mail.join("Persistent.Mail.1")).expect("game deletes first source");
        fs::remove_file(mail.join("Persistent.Mail.2")).expect("game deletes second source");
        fs::remove_file(mail.join("Persistent.Mail.99")).expect("game deletes unsupported source");
        let db = UploadDb::open(&config).expect("database after restart");
        assert_eq!(db.read_prepared("1").expect("captured bytes survive deletion"), original);
        db.reset().expect("reprocess retains captured sources and clears backoff");
        drop(db);
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("retained upload");
                let request = read_request(&mut stream).await;
                let text = String::from_utf8_lossy(&request);
                let id = text
                    .split("filename=\"Persistent.Mail.")
                    .nth(1)
                    .and_then(|suffix| suffix.split('"').next())
                    .expect("mail ID");
                let body = json!({"status":"stored", "mail_id":id}).to_string();
                stream
                    .write_all(response("201 Created", &body).as_bytes())
                    .await
                    .expect("API response");
            }
        });
        let mut watcher = start_fixture(config.clone(), mail, url);
        wait_for_uploads(&config, |store| {
            store.acknowledged.len() == 2 && store.prepared.is_empty()
        })
        .await;
        watcher.request_stop();
        watcher.join().await.expect("uploaded retained mail");
        server.await.expect("both retained sources upload");
        let saved = UploadDb::open(&config).expect("database").load().expect("saved outcomes");

        assert!(saved.pending.is_empty());
        assert!(saved.rejected.is_empty());
        assert!(saved.prepared.is_empty(), "acknowledgements reclaim retained bytes");
    }

    #[tokio::test]
    async fn unsupported_backlogs_report_type_counts_without_one_log_event_per_file() {
        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");
        fs::create_dir(&mail).expect("mail directory");

        for id in 0..128 {
            let bytes =
                rokbattles_mail_codec::encode(&json!({"id":id.to_string(), "type":"Unsupported"}))
                    .expect("unsupported bytes");
            fs::write(mail.join(format!("Persistent.Mail.{id}")), bytes).expect("source");
        }

        let messages = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&messages);
        let watcher_config = config.clone();
        let mut watcher = WatcherTask::spawn(move |shutdown, ready| async move {
            run_watcher(
                watcher_config,
                || Ok(Some(vec![mail.clone()])),
                |message| captured.lock().expect("logs").push(message),
                reqwest::Client::new(),
                "http://127.0.0.1:0".to_string(),
                shutdown,
                ready,
            )
            .await
        });
        wait_for_uploads(&config, |store| store.rejected.len() == 128).await;
        watcher.request_stop();
        watcher.join().await.expect("classification shutdown");
        let messages = messages.lock().expect("logs");

        assert!(!messages.is_empty());
        assert!(messages.len() <= 4, "classification should summarize the backlog: {messages:?}");
        assert!(messages.iter().all(|message| message.contains("Unsupported:")));
    }

    #[tokio::test]
    async fn staging_failure_stops_before_http_and_preserves_pending_sources() {
        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");
        write_mail(&mail, 1);
        let db = UploadDb::open(&config).expect("database");
        drop(db);
        let fault = rusqlite::Connection::open(config.join("local.db")).expect("fault connection");
        fault.execute_batch("CREATE TRIGGER fail_prepared BEFORE INSERT ON prepared_mail BEGIN SELECT RAISE(ABORT, 'injected staging failure'); END;").expect("fault trigger");
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("API listener");
        let url = format!("http://{}/v2/upload", listener.local_addr().expect("API address"));
        let mut watcher = start_fixture(config.clone(), mail.clone(), url);

        assert!(
            tokio::time::timeout(Duration::from_secs(5), watcher.join())
                .await
                .expect("storage failure terminates watcher")
                .is_err()
        );
        tokio::time::timeout(Duration::from_millis(50), listener.accept())
            .await
            .expect_err("unsaved mail must not start an upload");
        let saved =
            UploadDb::open(&config).expect("database").load().expect("durable pending source");
        assert_eq!(saved.pending.len(), 1);
        assert!(saved.acknowledged.is_empty());
        assert!(saved.prepared.is_empty());
        assert!(mail.join("Persistent.Mail.1").is_file());
    }

    #[tokio::test]
    async fn first_sqlite_start_reprocesses_mail_despite_legacy_acknowledgements() {
        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");

        fs::create_dir_all(&config).expect("config directory");
        write_mail(&mail, 1);

        let legacy_path = config.join("upload-state-v1.json");
        let legacy = serde_json::to_vec(&json!({
            "version": 1,
            "acknowledged": ["1"],
            "pending": [],
            "rejected": {},
            "api_backoff_until_ms": null,
        }))
        .expect("legacy history");
        fs::write(&legacy_path, &legacy).expect("previously acknowledged mail");

        let (url, server) =
            respond_once(response("200 OK", r#"{"status":"skipped","mail_id":"1"}"#)).await;

        let mut watcher = start_fixture(config.clone(), mail, url);
        let completed = wait_for_uploads(&config, |store| store.acknowledged.contains("1")).await;
        watcher.request_stop();
        watcher.join().await.expect("stop after fresh processing");
        server.await.expect("old acknowledgement must not suppress upload");

        assert!(completed.pending.is_empty());
        assert_eq!(fs::read(&legacy_path).expect("legacy history retained"), legacy);

        let db = UploadDb::open(&config).expect("restart with SQLite history");

        assert!(db.load().expect("durable acknowledgement").acknowledged.contains("1"));
    }

    #[tokio::test]
    async fn deleted_mail_is_forgotten_after_restart_and_rediscovered_if_it_returns() {
        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");
        write_mail(&mail, 1);
        let path = mail.join("Persistent.Mail.1");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(QueuedUpload {
            path: path.to_string_lossy().into_owned(),
            sig: Some(file_sig(&fs::metadata(&path).expect("source metadata")).expect("signature")),
            attempts: 0,
            not_before_ms: None,
        });
        let db = UploadDb::open(&config).expect("upload database");
        state.flush(&db).expect("save queued source");
        drop(db);
        fs::remove_file(&path).expect("game deletes cached mail before restart");

        let messages = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&messages);
        let watcher_config = config.clone();
        let watcher_mail = mail.clone();
        let mut watcher = WatcherTask::spawn(move |shutdown, ready| async move {
            run_watcher(
                watcher_config,
                || Ok(Some(vec![watcher_mail.clone()])),
                |message| captured.lock().expect("log capture").push(message),
                reqwest::Client::new(),
                "http://127.0.0.1:0".to_string(),
                shutdown,
                ready,
            )
            .await
        });

        wait_for_uploads(&config, |store| store.pending.is_empty()).await;
        watcher.request_stop();
        watcher.join().await.expect("stop after deleted source is removed");

        {
            let logs = messages.lock().expect("captured logs");

            assert!(logs.is_empty(), "cache deletion stays quiet: {logs:?}");
        }

        let db = UploadDb::open(&config).expect("state after shutdown");
        let restored = db.load().expect("saved deletion");

        assert!(restored.pending.is_empty());
        assert!(restored.acknowledged.is_empty());
        assert!(restored.rejected.is_empty());
        drop(db);

        write_mail(&mail, 1);
        let (url, server) =
            respond_once(response("201 Created", r#"{"status":"stored","mail_id":"1"}"#)).await;
        let mut watcher = start_fixture(config.clone(), mail, url);
        let completed = wait_for_uploads(&config, |store| store.acknowledged.contains("1")).await;
        watcher.request_stop();
        watcher.join().await.expect("stop after restored source uploads");
        server.await.expect("restored source reaches ingress");

        assert!(completed.pending.is_empty());
    }

    #[tokio::test]
    async fn stalled_upload_allows_discovery_and_shutdown_preserves_pending_sources() {
        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");
        for id in 1..=MAX_CONCURRENT_UPLOADS {
            write_mail(&mail, u32::try_from(id).expect("small fixture ID"));
        }
        let listener = Arc::new(TcpListener::bind("127.0.0.1:0").await.expect("API listener"));
        let url = format!("http://{}/v2/upload", listener.local_addr().expect("API address"));
        let (received, request) = oneshot::channel();
        let server_listener = Arc::clone(&listener);
        let server = tokio::spawn(async move {
            let mut streams = Vec::new();

            for _ in 0..MAX_CONCURRENT_UPLOADS {
                let (mut stream, _) = server_listener.accept().await.expect("concurrent upload");
                crate::watcher::upload::tests::read_request(&mut stream).await;
                streams.push(stream);
            }

            received.send(()).expect("request signal");

            std::future::pending::<()>().await;
            drop(streams);
        });
        let mut watcher = start_fixture(config.clone(), mail.clone(), url);
        tokio::time::timeout(Duration::from_secs(5), request)
            .await
            .expect("request starts")
            .expect("request signal");

        write_mail(&mail, 99);
        wait_for_uploads(&config, |store| store.pending.len() == MAX_CONCURRENT_UPLOADS + 1).await;
        tokio::time::timeout(Duration::from_millis(1000), listener.accept())
            .await
            .expect_err("upload concurrency stays bounded even when another source becomes ready");

        watcher.request_stop();
        tokio::time::timeout(Duration::from_secs(3), watcher.join())
            .await
            .expect("shutdown stays within budget during stalled HTTP")
            .expect("clean shutdown");
        server.abort();
        server.await.expect_err("fixture server cancelled");

        let db = UploadDb::open(&config).expect("joined watcher released its lock");
        let restored = db.load().expect("durable pending work");

        assert_eq!(restored.pending.len(), MAX_CONCURRENT_UPLOADS + 1);
        assert!(restored.acknowledged.is_empty());
    }

    #[tokio::test]
    async fn concurrent_rate_limit_pauses_new_attempts_while_other_responses_are_saved() {
        use crate::watcher::upload::tests::read_request;

        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");

        for id in 1..=MAX_CONCURRENT_UPLOADS + 1 {
            write_mail(&mail, u32::try_from(id).expect("small fixture ID"));
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("API listener");
        let url = format!("http://{}/v2/upload", listener.local_addr().expect("API address"));
        let mut watcher = start_fixture(config.clone(), mail, url);
        let mut connections = Vec::new();

        for _ in 0..MAX_CONCURRENT_UPLOADS {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .expect("requests overlap before any response")
                .expect("upload");
            let request = read_request(&mut stream).await;
            let request = String::from_utf8_lossy(&request);
            let id = request
                .split("filename=\"Persistent.Mail.")
                .nth(1)
                .and_then(|suffix| suffix.split('"').next())
                .expect("mail ID")
                .to_string();
            let durable = UploadDb::read_for_test(&config).expect("state before API response");

            assert_eq!(
                durable.pending.len(),
                MAX_CONCURRENT_UPLOADS + 1,
                "all requests must be durable before they reach the API"
            );
            assert!(
                durable
                    .pending
                    .values()
                    .any(|item| item.path.ends_with(&format!("Persistent.Mail.{id}")))
            );
            connections.push((stream, id));
        }

        let (mut limited, _) = connections.pop().expect("rate limited request");
        limited.write_all(
            b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 60\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ).await.expect("rate limit response");
        limited.shutdown().await.expect("close rate limited response");
        wait_for_uploads(&config, |store| store.api_backoff_until_ms.is_some()).await;

        for (mut stream, id) in connections {
            let body = serde_json::json!({"status":"stored", "mail_id":id}).to_string();
            stream
                .write_all(response("201 Created", &body).as_bytes())
                .await
                .expect("success response");
            stream.shutdown().await.expect("close success response");
        }

        let saved = wait_for_uploads(&config, |store| {
            store.acknowledged.len() == MAX_CONCURRENT_UPLOADS - 1
        })
        .await;
        assert_eq!(saved.pending.len(), 2, "limited and unstarted sources remain pending");
        assert!(saved.api_backoff_until_ms.is_some());
        tokio::time::timeout(Duration::from_millis(200), listener.accept())
            .await
            .expect_err("successes freeing slots must not bypass the global rate limit");

        watcher.request_stop();
        watcher.join().await.expect("shutdown saves retries");
        let restored = UploadDb::open(&config)
            .expect("joined watcher released lock")
            .load()
            .expect("recover pending sources");

        assert_eq!(restored.pending.len(), 2);
        assert_eq!(restored.pending.values().filter(|item| item.attempts == 1).count(), 1);
        assert!(restored.api_backoff_until_ms.is_some());
    }

    #[tokio::test]
    async fn slow_failures_schedule_retry_from_response_time_and_preserve_global_rate_limit() {
        for (status, retry_header, delay_ms) in [
            ("503 Service Unavailable", "", 2000_u128),
            ("429 Too Many Requests", "Retry-After: 7\r\n", 7000),
        ] {
            let root = tempfile::tempdir().expect("fixture directory");
            let config = root.path().join("config");
            let mail = root.path().join("mail");
            write_mail(&mail, 1);
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("API listener");
            let url = format!("http://{}/v2/upload", listener.local_addr().expect("API address"));
            let (received, request) = oneshot::channel();
            let (release, held) = oneshot::channel();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.expect("upload");
                crate::watcher::upload::tests::read_request(&mut stream).await;
                received.send(()).expect("request signal");
                held.await.expect("response release");

                let response = format!(
                    "HTTP/1.1 {status}\r\n{retry_header}Content-Length: 0\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(response.as_bytes()).await.expect("failure response");
            });
            let mut watcher = start_fixture(config.clone(), mail, url);
            tokio::time::timeout(Duration::from_secs(5), request)
                .await
                .expect("request starts")
                .expect("request signal");
            tokio::time::sleep(Duration::from_millis(150)).await;
            let response_ms = now_epoch_ms();
            release.send(()).expect("release delayed failure");

            let store = wait_for_uploads(&config, |store| {
                store.pending.values().next().is_some_and(|item| item.attempts == 1)
            })
            .await;
            let deadline = store
                .pending
                .values()
                .next()
                .expect("retry source")
                .not_before_ms
                .expect("retry deadline");

            assert!(deadline >= response_ms + delay_ms, "request latency must not consume backoff");

            if status.starts_with("429") {
                assert_eq!(store.api_backoff_until_ms, Some(deadline));
            }

            watcher.request_stop();
            watcher.join().await.expect("stop fixture");
            server.await.expect("API fixture");
        }
    }
}

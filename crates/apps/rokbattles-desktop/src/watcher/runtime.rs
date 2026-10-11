//! Runs the watcher and coordinates directory scans with one upload at a time.
//!
//! The watcher owns its mutable state and holds the snapshot lock until shutdown
//! finishes. Upload preparation and HTTP requests run separately; only this owner
//! applies their results. Pending work is saved before starting a request.
//!
//! Shutdown cancels HTTP waits, joins blocking reads, and saves remaining state.
//! Blocking filesystem calls cannot be interrupted, so callers must wait for the
//! worker to finish before starting another watcher or resetting upload history.

use std::{
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
    attempt_upload, complete_attempt, emit_log, http_client, now_epoch_ms,
    scan::{
        discover_source, queue_fs_event, reconcile_next_directory, refresh_watched_dirs,
        request_reconciliation, sync_fs_watches,
    },
    state::WatcherState,
    store::StateFile,
};

pub(crate) enum TaskEvent {
    /// Upload state has been locked and loaded; the initial scan may still be pending.
    Ready,
    Finished(anyhow::Result<()>),
}

/// A watcher worker, its readiness notification, and its stop signal.
///
/// Dropping this handle requests a stop but does not wait for completion. Lifecycle
/// operations must join the worker before releasing ownership or replacing it.
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

        // The state owner performs filesystem I/O on its own blocking thread. Never abort it:
        // shutdown must observe this handle complete before a replacement or reset can run.
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
    let state_file = StateFile::open(&config_dir)?;
    let mut state = WatcherState::new(state_file.load()?);
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
    let (upload_shutdown, upload_receiver) = watch::channel(false);
    let _ready_result = ready.send(());

    let result = loop {
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

        // Discovery and previous outcomes must be durable before another request starts.
        if let Err(error) = state.flush(&state_file) {
            break Err(error.context("Watcher stopped; failed to save upload state"));
        }

        let now_ms = now_epoch_ms();

        if !*shutdown.borrow()
            && uploads.is_empty()
            && state.rate_limit_remaining_ms(now_ms).is_none()
            && let Some(item) = state.next_ready_upload(now_ms)
        {
            uploads.spawn(attempt_upload(
                item,
                client.clone(),
                api_url.clone(),
                upload_receiver.clone(),
                now_ms,
            ));
        }

        // One upload runs independently of directory passes and event delivery. Completed
        // attempts are applied only here, by the single owner of durable state.
        tokio::select! {
            biased;

            _ = shutdown.changed() => break Ok(()),
            result = uploads.join_next(), if !uploads.is_empty() => {
                match result {
                    Some(Ok((item, outcome))) => {
                        log(complete_attempt(&mut state, item, outcome, now_epoch_ms()));
                    }
                    Some(Err(error)) => break Err(error.into()),
                    None => {}
                }
            }
            _ = tokio::task::yield_now(), if scanned => {}
            path = fs_rx.recv(), if fs_watcher.is_some() => {
                if let Some(path) = path {
                    discover_source(&mut state, path, now_epoch_ms());
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(750)) => {}
        }
    };

    // Cancel HTTP waits, but join preparation reads before releasing the state lock.
    // An interrupted request stays pending and is safe to retry after restart.
    upload_shutdown.send_replace(true);

    while let Some(outcome) = uploads.join_next().await {
        match outcome {
            Ok((item, outcome)) => {
                log(complete_attempt(&mut state, item, outcome, now_epoch_ms()));
            }
            Err(error) => log(format!("Upload task failed during shutdown: {error}")),
        }
    }

    state.flush(&state_file).context("Failed to save upload state during shutdown")?;

    result
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, time::UNIX_EPOCH};

    use serde_json::json;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;
    use crate::watcher::store::UploadStore;

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

    async fn wait_for_snapshot(dir: &Path, matches: impl Fn(&UploadStore) -> bool) -> UploadStore {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(bytes) = fs::read(dir.join("upload-state-v1.json"))
                    && let Ok(store) = serde_json::from_slice::<UploadStore>(&bytes)
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
    async fn stalled_upload_allows_discovery_and_shutdown_preserves_pending_sources() {
        let root = tempfile::tempdir().expect("fixture directory");
        let config = root.path().join("config");
        let mail = root.path().join("mail");
        write_mail(&mail, 1);
        let listener = Arc::new(TcpListener::bind("127.0.0.1:0").await.expect("API listener"));
        let url = format!("http://{}/v2/upload", listener.local_addr().expect("API address"));
        let (received, request) = oneshot::channel();
        let server_listener = Arc::clone(&listener);
        let server = tokio::spawn(async move {
            let (mut stream, _) = server_listener.accept().await.expect("first upload");
            stream.read_u8().await.expect("request bytes");
            received.send(()).expect("request signal");

            std::future::pending::<()>().await;
            drop(stream);
        });
        let mut watcher = start_fixture(config.clone(), mail.clone(), url);
        tokio::time::timeout(Duration::from_secs(5), request)
            .await
            .expect("request starts")
            .expect("request signal");

        write_mail(&mail, 2);
        wait_for_snapshot(&config, |store| store.pending.len() == 2).await;
        tokio::time::timeout(Duration::from_millis(30), listener.accept())
            .await
            .expect_err("only one upload can be in flight");

        watcher.request_stop();
        tokio::time::timeout(Duration::from_secs(3), watcher.join())
            .await
            .expect("shutdown stays within budget during stalled HTTP")
            .expect("clean shutdown");
        server.abort();
        server.await.expect_err("fixture server cancelled");

        let file = StateFile::open(&config).expect("joined watcher released its lock");
        let restored = file.load().expect("durable pending work");

        assert_eq!(restored.pending.len(), 2);
        assert!(restored.acknowledged.is_empty());
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

            let store = wait_for_snapshot(&config, |store| {
                store.pending.front().is_some_and(|item| item.attempts == 1)
            })
            .await;
            let deadline =
                store.pending.front().expect("retry source").not_before_ms.expect("retry deadline");

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

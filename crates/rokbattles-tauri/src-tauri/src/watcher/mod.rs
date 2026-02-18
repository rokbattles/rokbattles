mod config;
mod mail;
mod scan;
mod state;
mod store;
mod upload;

use self::config::WatcherConfig;
use self::mail::{detect_mail_type, file_name_for_upload, is_supported_mail_type};
use self::scan::{apply_fs_event, next_file, refresh_scans_if_needed, sync_fs_watches};
use self::state::WatcherState;
use self::store::{file_sig, read_processed, read_upload_queue};
use self::upload::{is_retryable_status, post_file_to_api, upload_backoff};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch};

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

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
    let _ = app.emit(
        "rokbattles",
        LogPayload {
            message: message.into(),
        },
    );
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

pub fn delete_processed(app: &AppHandle) -> anyhow::Result<()> {
    store::delete_processed(app, &WatcherConfig::default())
}

pub fn delete_upload_queue(app: &AppHandle) -> anyhow::Result<()> {
    store::delete_upload_queue(app, &WatcherConfig::default())
}

pub struct WatcherTask {
    shutdown: watch::Sender<bool>,
    handle: tauri::async_runtime::JoinHandle<()>,
    shutdown_timeout: Duration,
}

impl WatcherTask {
    pub async fn shutdown(self, app: &AppHandle) {
        let _ = self.shutdown.send(true);
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
        let processed = match read_processed(&app, &config_for_task) {
            Ok(store) => store,
            Err(e) => {
                emit_log(&app, format!("Failed to read processed store: {}", e));
                store::ProcessedStore::default()
            }
        };
        let upload_queue = match read_upload_queue(&app, &config_for_task) {
            Ok(store) => store,
            Err(e) => {
                emit_log(&app, format!("Failed to read upload queue: {}", e));
                store::UploadQueueStore::default()
            }
        };
        let mut state = WatcherState::new(config_for_task, processed, upload_queue);
        let mut fs_watcher = match notify::recommended_watcher({
            let fs_tx = fs_tx.clone();
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        let _ = fs_tx.try_send(path);
                    }
                }
            }
        }) {
            Ok(w) => Some(w),
            Err(e) => {
                emit_log(
                    &app,
                    format!("Failed to initialize filesystem watcher: {}", e),
                );
                None
            }
        };
        let mut fs_watched_dirs: HashSet<PathBuf> = HashSet::new();

        loop {
            if *shutdown_rx.borrow() {
                state.maybe_flush_store(&app);
                state.maybe_flush_upload_queue(&app);
                break;
            }

            let now_ms = now_epoch_ms();

            state.maybe_flush_store(&app);
            state.maybe_flush_upload_queue(&app);
            state.maybe_rescan_hot(now_ms);

            let _ = refresh_scans_if_needed(&app, &mut state).await;
            sync_fs_watches(&app, fs_watcher.as_mut(), &mut fs_watched_dirs, &state.dirs);

            for _ in 0..state.config.fs_event_budget {
                let Ok(path) = fs_rx.try_recv() else {
                    break;
                };
                apply_fs_event(&mut state, path, now_ms);
            }

            while state.upload_queue.len() < state.config.upload_prefetch_target {
                if let Some(item) = next_file(&app, &mut state).await {
                    state.enqueue_upload(item);
                } else {
                    break;
                }
            }

            if state.rate_limit_remaining_ms(now_ms).is_none()
                && let Some(item) = state.pop_ready_upload(now_ms)
            {
                let path = PathBuf::from(&item.path);
                let Some(file_name) = file_name_for_upload(&path) else {
                    emit_log(&app, "Skipping file with invalid name");
                    continue;
                };
                emit_log(&app, format!("Processing {}", file_name));

                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        emit_log(&app, format!("Failed to stat file {}: {}", file_name, e));
                        continue;
                    }
                };
                let sig_now = match file_sig(&meta) {
                    Ok(s) => s,
                    Err(e) => {
                        emit_log(&app, format!("Failed to stat file {}: {}", file_name, e));
                        continue;
                    }
                };
                let age_ms = now_ms.saturating_sub(sig_now.modified);
                if sig_now != item.sig || age_ms < state.config.file_stable_age_ms {
                    let path_key = item.path.clone();
                    let mut next = item;
                    next.sig = sig_now.clone();
                    next.not_before_ms =
                        Some(now_ms.saturating_add(state.config.file_retry_delay_ms));
                    state.requeue_upload(next);

                    state.store.entries.insert(path_key, sig_now);
                    state.store_dirty_updates += 1;
                    continue;
                }

                let bytes = match tauri::async_runtime::spawn_blocking({
                    let path = path.clone();
                    move || fs::read(&path)
                })
                .await
                {
                    Ok(Ok(b)) => b,
                    Ok(Err(e)) => {
                        emit_log(&app, format!("Failed to read file {}: {}", file_name, e));

                        let mut next = item;
                        next.attempts = next.attempts.saturating_add(1);
                        let backoff = upload_backoff(next.attempts);
                        next.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));
                        state.requeue_upload(next);
                        continue;
                    }
                    Err(e) => {
                        emit_log(&app, format!("Failed to read file {}: {}", file_name, e));

                        let mut next = item;
                        next.attempts = next.attempts.saturating_add(1);
                        let backoff = upload_backoff(next.attempts);
                        next.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));
                        state.requeue_upload(next);
                        continue;
                    }
                };

                let decoded = match mail_decoder::decode(&bytes) {
                    Ok(m) => m,
                    Err(e) => {
                        emit_log(&app, format!("Decode failed for {}: {}", file_name, e));
                        continue;
                    }
                };

                let first_type = detect_mail_type(&decoded);
                let supported_type = first_type.is_some_and(is_supported_mail_type);
                if !supported_type {
                    emit_log(
                        &app,
                        format!(
                            "Skipping unsupported mail {} (detected: {})",
                            file_name,
                            first_type.unwrap_or("Unknown")
                        ),
                    );
                    continue;
                }

                match post_file_to_api(client, &api_url, &file_name, bytes).await {
                    Ok(status) => {
                        emit_log(&app, status.log_message(&file_name));
                    }
                    Err(e) => {
                        let retryable = is_retryable_status(e.status);
                        if retryable {
                            let mut next = item;
                            next.attempts = next.attempts.saturating_add(1);
                            let retry_after = e.retry_after.filter(|delay| delay.as_millis() > 0);
                            let backoff =
                                retry_after.unwrap_or_else(|| upload_backoff(next.attempts));
                            next.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));
                            state.requeue_upload(next);

                            if e.status == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
                                if state.extend_rate_limit(now_ms, backoff) {
                                    let wait_secs =
                                        (backoff.as_millis().saturating_add(999) / 1000).max(1);
                                    emit_log(
                                        &app,
                                        format!(
                                            "Rate limited by API (429). Pausing uploads for {}s.",
                                            wait_secs
                                        ),
                                    );
                                }
                            } else {
                                emit_log(
                                    &app,
                                    format!("Failed to upload {}: {}", file_name, e.message),
                                );
                            }
                        } else {
                            emit_log(
                                &app,
                                format!("Failed to upload {}: {}", file_name, e.message),
                            );
                        }
                    }
                }
                continue;
            }

            let now_ms = now_epoch_ms();
            tokio::select! {
                _ = tokio::time::sleep(state.config.idle_sleep) => {}
                res = shutdown_rx.changed() => {
                    if res.is_ok() && *shutdown_rx.borrow() {
                        state.maybe_flush_store(&app);
                        state.maybe_flush_upload_queue(&app);
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
    });

    WatcherTask {
        shutdown: shutdown_tx,
        handle,
        shutdown_timeout,
    }
}

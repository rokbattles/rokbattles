mod watcher;

use crate::watcher::{WatcherTask, delete_processed, delete_upload_queue, spawn_watcher};
use anyhow::Context;
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_updater::UpdaterExt;

#[derive(Default)]
struct WatcherManager {
    task: tokio::sync::Mutex<Option<WatcherTask>>,
    exiting: AtomicBool,
}

impl WatcherManager {
    async fn start(&self, app: &AppHandle) {
        let mut guard = self.task.lock().await;
        if guard.is_some() {
            return;
        }
        // Spawn the watcher once per app lifecycle.
        *guard = Some(spawn_watcher(app));
    }

    async fn stop(&self, app: &AppHandle) {
        let mut guard = self.task.lock().await;
        if let Some(task) = guard.take() {
            task.shutdown(app).await;
        }
    }
}

fn config_file(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join("config.json"))
}

fn read_dirs(app: &AppHandle) -> anyhow::Result<Vec<String>> {
    let path = config_file(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read(&path).with_context(|| format!("Failed reading {:?}", path))?;
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let list: Vec<String> =
        serde_json::from_slice(&data).with_context(|| format!("Invalid JSON in {:?}", path))?;
    Ok(list)
}

fn write_dirs(app: &AppHandle, dirs: &[String]) -> anyhow::Result<()> {
    let path = config_file(app)?;
    let json =
        serde_json::to_vec_pretty(dirs).context("Failed to serialize directories to JSON")?;
    fs::write(&path, json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

// https://tauri.app/plugin/updater/#checking-for-updates
async fn update(app: AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;

        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    println!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    println!("download finished");
                },
            )
            .await?;

        println!("update installed");
        app.restart();
    }

    Ok(())
}

#[tauri::command]
fn list_dirs(app: AppHandle) -> Result<Vec<String>, String> {
    read_dirs(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_dir(app: AppHandle, paths: Vec<String>) -> Result<Vec<String>, String> {
    let current = read_dirs(&app).map_err(|e| e.to_string())?;
    let mut set: BTreeSet<String> = current.into_iter().collect();

    for p in paths {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            // Deduplicate while preserving sorted order via BTreeSet.
            set.insert(trimmed.to_string());
        }
    }

    let next: Vec<String> = set.into_iter().collect();
    write_dirs(&app, &next).map_err(|e| e.to_string())?;
    Ok(next)
}

#[tauri::command]
fn remove_dir(app: AppHandle, path: String) -> Result<Vec<String>, String> {
    let mut current = read_dirs(&app).map_err(|e| e.to_string())?;
    current.retain(|p| p != &path);
    write_dirs(&app, &current).map_err(|e| e.to_string())?;
    Ok(current)
}

#[tauri::command]
async fn reprocess_all(
    app: AppHandle,
    watcher: tauri::State<'_, WatcherManager>,
) -> Result<(), String> {
    watcher.stop(&app).await;
    delete_processed(&app).map_err(|e| e.to_string())?;
    delete_upload_queue(&app).map_err(|e| e.to_string())?;
    watcher.start(&app).await;
    Ok(())
}

#[tauri::command]
async fn pause_watcher(
    app: AppHandle,
    watcher: tauri::State<'_, WatcherManager>,
) -> Result<(), String> {
    watcher.stop(&app).await;
    Ok(())
}

#[tauri::command]
async fn resume_watcher(
    app: AppHandle,
    watcher: tauri::State<'_, WatcherManager>,
) -> Result<(), String> {
    watcher.start(&app).await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(WatcherManager::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = update(handle.clone()).await {
                    eprintln!("[rokbattles] update check failed: {}", e);
                }

                // Start watching only after the update check completes to avoid
                // scanning while the app is about to restart for an update.
                handle.state::<WatcherManager>().start(&handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_dirs,
            add_dir,
            remove_dir,
            reprocess_all,
            pause_watcher,
            resume_watcher
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let manager = app.state::<WatcherManager>();
            if manager.exiting.swap(true, Ordering::SeqCst) {
                return;
            }
            api.prevent_exit();
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                handle.state::<WatcherManager>().stop(&handle).await;
                handle.exit(0);
            });
        }
    });
}

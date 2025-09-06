mod app_config;
mod watcher;

use crate::watcher::{delete_processed, spawn_watcher};
use std::collections::BTreeSet;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

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
    app_config::read_watch_dirs(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_dir(app: AppHandle, paths: Vec<String>) -> Result<Vec<String>, String> {
    let current = app_config::read_watch_dirs(&app).map_err(|e| e.to_string())?;
    let mut set: BTreeSet<String> = current.into_iter().collect();

    for p in paths {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_string());
        }
    }

    let next: Vec<String> = set.into_iter().collect();
    app_config::write_watch_dirs(&app, &next).map_err(|e| e.to_string())?;
    Ok(next)
}

#[tauri::command]
fn remove_dir(app: AppHandle, path: String) -> Result<Vec<String>, String> {
    let mut current = app_config::read_watch_dirs(&app).map_err(|e| e.to_string())?;
    current.retain(|p| p != &path);
    app_config::write_watch_dirs(&app, &current).map_err(|e| e.to_string())?;
    Ok(current)
}

#[tauri::command]
fn reprocess_all(app: AppHandle) -> Result<(), String> {
    delete_processed(&app).map_err(|e| e.to_string())?;
    // TODO Clear any local upload queues (see watcher TODOs)
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                update(handle).await.unwrap();
            });

            spawn_watcher(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_dirs,
            add_dir,
            remove_dir,
            reprocess_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

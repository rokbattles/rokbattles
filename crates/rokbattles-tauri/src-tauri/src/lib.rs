mod watcher;

use crate::watcher::{delete_processed, spawn_watcher};
use anyhow::Context;
use std::{collections::BTreeSet, fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

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

pub fn read_api_ingress_url(app: &AppHandle) -> anyhow::Result<String> {
    const DEFAULT_URL: &str = "https://rokbattles.com/api/v1/ingress";

    let path = config_file(app)?;
    if !path.exists() {
        return Ok(DEFAULT_URL.to_string());
    }
    let data = fs::read(&path).with_context(|| format!("Failed reading {:?}", path))?;
    if data.is_empty() {
        return Ok(DEFAULT_URL.to_string());
    }

    let v: serde_json::Value =
        serde_json::from_slice(&data).with_context(|| format!("Invalid JSON in {:?}", path))?;

    if v.is_object()
        && let Some(u) = v.get("ingressUrl").and_then(|x| x.as_str())
        && !u.trim().is_empty()
    {
        return Ok(u.to_string());
    }
    Ok(DEFAULT_URL.to_string())
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

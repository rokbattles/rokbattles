mod app_config;
mod tray;
mod updater;
mod watcher;
mod watcher_manager;

use crate::watcher::{delete_processed, delete_upload_queue};
use crate::watcher_manager::WatcherManager;
use app_config::CloseBehavior;
use std::collections::BTreeSet;
use tauri::{
    AppHandle, Manager, RunEvent,
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};

pub(crate) fn read_dirs(app: &AppHandle) -> anyhow::Result<Vec<String>> {
    app_config::read_dirs(app)
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
            // Keep writes stable and duplicate-free.
            set.insert(trimmed.to_string());
        }
    }

    let next: Vec<String> = set.into_iter().collect();
    app_config::write_dirs(&app, &next).map_err(|e| e.to_string())?;
    Ok(next)
}

#[tauri::command]
fn remove_dir(app: AppHandle, path: String) -> Result<Vec<String>, String> {
    let mut current = read_dirs(&app).map_err(|e| e.to_string())?;
    current.retain(|p| p != &path);
    app_config::write_dirs(&app, &current).map_err(|e| e.to_string())?;
    Ok(current)
}

#[tauri::command]
fn get_close_behavior(app: AppHandle) -> Result<CloseBehavior, String> {
    app_config::get_close_behavior(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_close_behavior(app: AppHandle, behavior: CloseBehavior) -> Result<(), String> {
    app_config::set_close_behavior(&app, behavior).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_auto_update(app: AppHandle) -> Result<bool, String> {
    app_config::get_auto_update(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_auto_update(app: AppHandle, enabled: bool) -> Result<(), String> {
    app_config::set_auto_update(&app, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
fn request_app_quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn minimize_to_tray(app: AppHandle) {
    tray::hide_main_window(&app);
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
    tray::refresh_tray_menu(&app, watcher.is_paused());
    Ok(())
}

#[tauri::command]
async fn pause_watcher(
    app: AppHandle,
    watcher: tauri::State<'_, WatcherManager>,
) -> Result<(), String> {
    watcher.stop(&app).await;
    tray::refresh_tray_menu(&app, watcher.is_paused());
    Ok(())
}

#[tauri::command]
async fn resume_watcher(
    app: AppHandle,
    watcher: tauri::State<'_, WatcherManager>,
) -> Result<(), String> {
    watcher.start(&app).await;
    tray::refresh_tray_menu(&app, watcher.is_paused());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(WatcherManager::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_menu_event(|app, event| {
            if event.id() == tray::TRAY_SHOW_MENU_ID {
                tray::show_main_window(app);
                return;
            }

            if event.id() == tray::TRAY_TOGGLE_WATCHER_MENU_ID {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let watcher = app.state::<WatcherManager>();
                    if watcher.is_paused() {
                        watcher.start(&app).await;
                    } else {
                        watcher.stop(&app).await;
                    }
                    tray::refresh_tray_menu(&app, watcher.is_paused());
                });
                return;
            }

            if event.id() == tray::TRAY_QUIT_MENU_ID {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|app, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                tray::show_main_window(app);
            }
            _ => {}
        })
        .setup(|app| {
            let paused = app.state::<WatcherManager>().is_paused();
            tray::setup_tray(app, paused)?;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let auto_update = app_config::get_auto_update(&handle).unwrap_or(true);
                updater::maybe_check_for_updates(handle.clone(), auto_update).await;

                // Let the updater run first so we don't scan if we're about to restart.
                let watcher = handle.state::<WatcherManager>();
                watcher.start(&handle).await;
                tray::refresh_tray_menu(&handle, watcher.is_paused());
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_dirs,
            add_dir,
            remove_dir,
            get_close_behavior,
            set_close_behavior,
            get_auto_update,
            set_auto_update,
            request_app_quit,
            minimize_to_tray,
            reprocess_all,
            pause_watcher,
            resume_watcher
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let manager = app.state::<WatcherManager>();
            if !manager.mark_exit_requested() {
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

mod app_config;
mod mailcache_discovery;
mod network_introspection;
mod tray;
mod updater;
mod watcher;
mod watcher_manager;

use std::{collections::BTreeSet, path::Path};

use app_config::CloseBehavior;
use serde::Serialize;
use tauri::{
    AppHandle, Manager, RunEvent,
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};

use crate::{
    network_introspection::{NetworkIntrospectionManager, NetworkStatus},
    watcher::{delete_processed, delete_upload_queue},
    watcher_manager::WatcherManager,
};

fn tray_supported() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos"))
}

#[cfg(desktop)]
fn setup_autostart(app: &tauri::App<tauri::Wry>) -> tauri::Result<()> {
    use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

    let handle = app.handle();
    handle.plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))?;

    if app_config::get_autostart_initialized(handle).unwrap_or(false) {
        return Ok(());
    }

    match handle.autolaunch().enable() {
        Ok(()) => {
            if let Err(error) = app_config::set_autostart_initialized(handle, true) {
                eprintln!("Failed to record autostart initialization: {error}");
            }
        }
        Err(error) => {
            eprintln!("Failed to enable autostart: {error}");
        }
    }

    Ok(())
}

#[cfg(desktop)]
fn single_instance_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_single_instance::init(|app, _args, _cwd| {
        tray::show_main_window(app);
    })
}

#[cfg(desktop)]
fn setup_deep_links(app: &tauri::App<tauri::Wry>) -> Result<(), tauri_plugin_deep_link::Error> {
    use tauri_plugin_deep_link::DeepLinkExt;

    // Runtime registration is not supported on macOS. Test rokbattles:// there
    // with a bundled app installed in /Applications.
    #[cfg(target_os = "linux")]
    if let Err(error) = app.deep_link().register_all() {
        eprintln!("[rokbattles] failed to register deep links: {error}");
    }

    #[cfg(all(debug_assertions, windows))]
    app.deep_link().register_all()?;

    if let Some(urls) = app.deep_link().get_current()? {
        eprintln!("Opened with deep link URLs: {urls:?}");
    }

    app.deep_link().on_open_url(|event| {
        eprintln!("Opened with deep link URLs: {:?}", event.urls());
    });

    Ok(())
}

fn normalize_dir_for_display(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    #[cfg(target_os = "windows")]
    {
        return mailcache_discovery::normalize_windows_path_for_display(trimmed);
    }

    #[cfg(not(target_os = "windows"))]
    {
        trimmed.to_string()
    }
}

fn dir_identity_key(path: &str) -> String {
    let normalized = normalize_dir_for_display(path);
    mailcache_discovery::path_identity_key(Path::new(&normalized))
}

fn dedupe_dirs_preserving_representation(dirs: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();

    for dir in dirs {
        let trimmed = dir.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = dir_identity_key(trimmed);
        if seen.insert(key) {
            unique.push(trimmed.to_string());
        }
    }

    unique
}

fn dirs_for_ui(dirs: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();

    for dir in dirs {
        let display = normalize_dir_for_display(dir);
        if display.is_empty() {
            continue;
        }
        let key = dir_identity_key(&display);
        if seen.insert(key) {
            normalized.push(display);
        }
    }

    normalized.sort();
    normalized
}

pub(crate) fn read_dirs(app: &AppHandle) -> anyhow::Result<Vec<String>> {
    let dirs = app_config::read_dirs(app)?;
    Ok(dedupe_dirs_preserving_representation(dirs))
}

#[tauri::command]
fn list_dirs(app: AppHandle) -> Result<Vec<String>, String> {
    let current = read_dirs(&app).map_err(|e| e.to_string())?;
    Ok(dirs_for_ui(&current))
}

#[tauri::command]
fn add_dir(app: AppHandle, paths: Vec<String>) -> Result<Vec<String>, String> {
    let current = read_dirs(&app).map_err(|e| e.to_string())?;
    let mut next = current;
    let mut known_keys: BTreeSet<String> = next.iter().map(|dir| dir_identity_key(dir)).collect();

    for p in paths {
        let normalized = normalize_dir_for_display(&p);
        if normalized.is_empty() {
            continue;
        }
        let key = dir_identity_key(&normalized);
        if known_keys.insert(key) {
            next.push(normalized);
        }
    }

    next.sort();
    app_config::write_dirs(&app, &next).map_err(|e| e.to_string())?;
    Ok(dirs_for_ui(&next))
}

#[tauri::command]
fn remove_dir(app: AppHandle, path: String) -> Result<Vec<String>, String> {
    let current = read_dirs(&app).map_err(|e| e.to_string())?;
    let target_key = dir_identity_key(&path);
    let mut next =
        current.into_iter().filter(|dir| dir_identity_key(dir) != target_key).collect::<Vec<_>>();
    next.sort();
    app_config::write_dirs(&app, &next).map_err(|e| e.to_string())?;
    Ok(dirs_for_ui(&next))
}

#[derive(Debug, Serialize)]
struct DiscoverMailcacheResult {
    added_dirs: Vec<String>,
    already_watched_dirs: Vec<String>,
    message: String,
}

#[tauri::command]
fn discover_mailcache_dirs(app: AppHandle) -> Result<DiscoverMailcacheResult, String> {
    if !cfg!(any(target_os = "windows", target_os = "macos")) {
        return Ok(DiscoverMailcacheResult {
            added_dirs: Vec::new(),
            already_watched_dirs: Vec::new(),
            message: "Autodiscovery is only available on Windows and macOS.".to_string(),
        });
    }

    let current = read_dirs(&app).map_err(|e| e.to_string())?;
    let discovered = mailcache_discovery::discover_mailcache_dirs().map_err(|e| e.to_string())?;

    if discovered.is_empty() {
        return Ok(DiscoverMailcacheResult {
            added_dirs: Vec::new(),
            already_watched_dirs: Vec::new(),
            message: "No valid mailcache directories were found.".to_string(),
        });
    }

    let mut next = current;
    let mut known_keys: BTreeSet<String> = next.iter().map(|dir| dir_identity_key(dir)).collect();

    let mut added_dirs = Vec::new();
    let mut already_watched_dirs = Vec::new();

    for dir in discovered {
        let normalized = normalize_dir_for_display(&dir);
        if normalized.is_empty() {
            continue;
        }
        let key = dir_identity_key(&normalized);
        if known_keys.contains(&key) {
            already_watched_dirs.push(normalized);
            continue;
        }

        known_keys.insert(key);
        next.push(normalized.clone());
        added_dirs.push(normalized);
    }

    if !added_dirs.is_empty() {
        next.sort();
        app_config::write_dirs(&app, &next).map_err(|e| e.to_string())?;
    }

    let message = if !added_dirs.is_empty() {
        let count = added_dirs.len();
        format!(
            "Auto-discovered and added {} mailcache director{}.",
            count,
            if count == 1 { "y" } else { "ies" }
        )
    } else {
        let count = already_watched_dirs.len();
        format!(
            "Found {} mailcache director{}, but they are already being watched.",
            count,
            if count == 1 { "y" } else { "ies" }
        )
    };

    Ok(DiscoverMailcacheResult { added_dirs, already_watched_dirs, message })
}

#[tauri::command]
fn get_close_behavior(app: AppHandle) -> Result<CloseBehavior, String> {
    if !cfg!(any(target_os = "windows", target_os = "macos")) {
        return Ok(CloseBehavior::Quit);
    }

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
fn get_experimental_network_introspection(app: AppHandle) -> Result<bool, String> {
    app_config::get_experimental_network_introspection(&app).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_network_introspection_status(
    network: tauri::State<'_, NetworkIntrospectionManager>,
) -> Result<NetworkStatus, String> {
    Ok(network.status().await)
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
    let builder = tauri::Builder::default()
        .manage(WatcherManager::default())
        .manage(NetworkIntrospectionManager::default());

    #[cfg(desktop)]
    let builder = builder.plugin(single_instance_plugin());

    let app = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_menu_event(|app, event| {
            if !tray_supported() {
                return;
            }

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
        .on_tray_icon_event(|app, event| {
            if !tray_supported() {
                return;
            }

            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => {
                    tray::show_main_window(app);
                }
                _ => {}
            }
        })
        .setup(|app| {
            #[cfg(desktop)]
            setup_autostart(app)?;

            #[cfg(desktop)]
            setup_deep_links(app)?;

            let paused = app.state::<WatcherManager>().is_paused();
            if tray_supported() {
                tray::setup_tray(app, paused)?;
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let auto_update = app_config::get_auto_update(&handle).unwrap_or(true);
                updater::maybe_check_for_updates(handle.clone(), auto_update).await;

                // Let the updater run first so we don't scan if we're about to restart.
                let watcher = handle.state::<WatcherManager>();
                watcher.start(&handle).await;
                tray::refresh_tray_menu(&handle, watcher.is_paused());

                let network = handle.state::<NetworkIntrospectionManager>();
                if app_config::get_experimental_network_introspection(&handle).unwrap_or(false) {
                    network.start(&handle).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_dirs,
            add_dir,
            remove_dir,
            discover_mailcache_dirs,
            get_close_behavior,
            set_close_behavior,
            get_auto_update,
            set_auto_update,
            get_experimental_network_introspection,
            get_network_introspection_status,
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
                handle.state::<NetworkIntrospectionManager>().shutdown().await;
                handle.exit(0);
            });
        }
    });
}

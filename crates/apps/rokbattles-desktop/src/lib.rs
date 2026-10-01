mod app_config;
mod mailcache_discovery;
mod tray;
mod updater;
mod watcher_manager;

use std::{collections::BTreeSet, path::Path};

use app_config::CloseBehavior;
use serde::Serialize;
use tauri::{
    AppHandle, Manager,
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};

use crate::watcher_manager::WatcherManager;

pub(crate) fn is_flatpak() -> bool {
    cfg!(target_os = "linux") && Path::new("/.flatpak-info").exists()
}

fn tray_supported() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos"))
}

#[cfg(desktop)]
fn setup_autostart(app: &tauri::App<tauri::Wry>) -> tauri::Result<()> {
    use tauri_plugin_autostart::MacosLauncher;

    // The native plugin writes a host autostart entry with an /app executable path.
    if is_flatpak() {
        return Ok(());
    }

    let handle = app.handle();
    handle.plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))?;

    let enabled = app_config::get_auto_start(handle).unwrap_or(true);
    match apply_auto_start_setting(handle, enabled) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("Failed to apply autostart setting: {error}");
        }
    }

    Ok(())
}

#[cfg(desktop)]
fn apply_auto_start_setting(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    if is_flatpak() {
        return Err("Automatic startup is unavailable in the Flatpak package.".to_string());
    }

    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())
    } else {
        autolaunch.disable().map_err(|e| e.to_string())
    }
}

#[cfg(desktop)]
fn single_instance_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_single_instance::init(|app, _args, _cwd| {
        tray::show_main_window(app);
    })
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

pub(crate) async fn read_dirs(app: &AppHandle) -> anyhow::Result<Vec<String>> {
    let store = app.state::<WatcherManager>().store().await.map_err(anyhow::Error::msg)?;
    Ok(store
        .roots()
        .await?
        .into_iter()
        .map(|root| root.path.to_string_lossy().into_owned())
        .collect())
}

async fn write_dirs(app: &AppHandle, dirs: &[String]) -> Result<(), String> {
    let paths: Vec<std::path::PathBuf> = dirs.iter().map(std::path::PathBuf::from).collect();
    let validated = tauri::async_runtime::spawn_blocking(move || {
        for path in &paths {
            rokbattles_desktop_agent::mailcache::MailRoot::open(path).map_err(|_error| {
                "Choose an existing local mailcache directory without links or junctions."
                    .to_string()
            })?;
        }
        Ok::<_, String>(paths)
    })
    .await
    .map_err(|_error| "Cannot validate selected directories.")??;
    app.state::<WatcherManager>()
        .store()
        .await?
        .set_roots(&validated)
        .await
        .map_err(|_error| "Cannot save selected directories.".to_string())
}

#[tauri::command]
async fn list_dirs(app: AppHandle) -> Result<Vec<String>, String> {
    Ok(dirs_for_ui(&read_dirs(&app).await.map_err(|_error| "Cannot read selected directories.")?))
}

#[tauri::command]
async fn add_dir(app: AppHandle, paths: Vec<String>) -> Result<Vec<String>, String> {
    let current = read_dirs(&app).await.map_err(|e| e.to_string())?;
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
    write_dirs(&app, &next).await?;
    Ok(dirs_for_ui(&next))
}

#[tauri::command]
async fn remove_dir(app: AppHandle, path: String) -> Result<Vec<String>, String> {
    let current = read_dirs(&app).await.map_err(|e| e.to_string())?;
    let target_key = dir_identity_key(&path);
    let mut next =
        current.into_iter().filter(|dir| dir_identity_key(dir) != target_key).collect::<Vec<_>>();
    next.sort();
    write_dirs(&app, &next).await?;
    Ok(dirs_for_ui(&next))
}

#[derive(Debug, Serialize)]
struct DiscoverMailcacheResult {
    added_dirs: Vec<String>,
    already_watched_dirs: Vec<String>,
    message: String,
}

#[derive(Debug, Serialize)]
struct AppSettings {
    auto_update: bool,
    auto_start: bool,
    close_behavior: CloseBehavior,
    tray_supported: bool,
    flatpak: bool,
}

#[tauri::command]
async fn discover_mailcache_dirs(app: AppHandle) -> Result<DiscoverMailcacheResult, String> {
    if !cfg!(any(target_os = "windows", target_os = "macos")) {
        return Ok(DiscoverMailcacheResult {
            added_dirs: Vec::new(),
            already_watched_dirs: Vec::new(),
            message: "Autodiscovery is only available on Windows and macOS.".to_string(),
        });
    }

    let current = read_dirs(&app).await.map_err(|e| e.to_string())?;
    let discovered =
        tauri::async_runtime::spawn_blocking(mailcache_discovery::discover_mailcache_dirs)
            .await
            .map_err(|_error| "Directory discovery failed.")?
            .map_err(|_error| "Directory discovery failed.")?;

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
        write_dirs(&app, &next).await?;
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
    if behavior == CloseBehavior::MinimizeToTray && !tray_supported() {
        return Err("Minimize to tray is not supported on this platform.".to_string());
    }

    app_config::set_close_behavior(&app, behavior).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_auto_update(app: AppHandle) -> Result<bool, String> {
    if is_flatpak() {
        return Ok(false);
    }
    app_config::get_auto_update(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_auto_update(app: AppHandle, enabled: bool) -> Result<(), String> {
    if is_flatpak() {
        return Err("Install a new Flatpak bundle to update this application.".to_string());
    }
    app_config::set_auto_update(&app, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_auto_start(app: AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(desktop)]
    apply_auto_start_setting(&app, enabled)?;

    app_config::set_auto_start(&app, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    Ok(AppSettings {
        auto_update: !is_flatpak()
            && app_config::get_auto_update(&app).map_err(|e| e.to_string())?,
        auto_start: !is_flatpak() && app_config::get_auto_start(&app).map_err(|e| e.to_string())?,
        close_behavior: app_config::get_close_behavior(&app).map_err(|e| e.to_string())?,
        tray_supported: tray_supported(),
        flatpak: is_flatpak(),
    })
}

#[tauri::command]
fn request_app_quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn minimize_to_tray(app: AppHandle) {
    tray::hide_main_window(&app);
}

#[derive(Serialize)]
struct WorkerSnapshot {
    status: rokbattles_desktop_store::Status,
    alive: bool,
    enabled: bool,
    capture_opt_in: bool,
}

#[tauri::command]
async fn get_worker_status(app: AppHandle) -> Result<WorkerSnapshot, String> {
    let store = app.state::<WatcherManager>().store().await?;
    let status = store.status().await.map_err(|_error| "Cannot read worker status.")?;
    let settings = store.settings().await.map_err(|_error| "Cannot read worker settings.")?;
    Ok(WorkerSnapshot {
        alive: watcher_manager::is_alive(&status),
        status,
        enabled: settings.enabled,
        capture_opt_in: settings.capture_opt_in,
    })
}

#[tauri::command]
async fn set_capture_opt_in(app: AppHandle, enabled: bool) -> Result<(), String> {
    app.state::<WatcherManager>()
        .store()
        .await?
        .set_capture_opt_in(enabled)
        .await
        .map_err(|_error| "Cannot save capture consent.".to_string())
}

#[tauri::command]
async fn stop_background_worker(app: AppHandle) -> Result<(), String> {
    app.state::<WatcherManager>()
        .store()
        .await?
        .set_enabled(false)
        .await
        .map_err(|_error| "Cannot stop background worker.".to_string())
}

mod watcher_commands {
    use super::*;

    #[tauri::command]
    pub(super) async fn reprocess_all(app: AppHandle) -> Result<(), String> {
        let watcher = app.state::<WatcherManager>();
        watcher
            .store()
            .await?
            .request_reprocess()
            .await
            .map_err(|_error| "Cannot request reprocessing.")?;
        watcher.start(&app).await?;
        tray::refresh_tray_menu(&app, watcher.is_paused());
        Ok(())
    }

    #[tauri::command]
    pub(super) async fn pause_watcher(app: AppHandle) -> Result<(), String> {
        let watcher = app.state::<WatcherManager>();
        watcher.stop(&app).await?;
        tray::refresh_tray_menu(&app, watcher.is_paused());
        Ok(())
    }

    #[tauri::command]
    pub(super) async fn resume_watcher(app: AppHandle) -> Result<(), String> {
        let watcher = app.state::<WatcherManager>();
        watcher.start(&app).await?;
        tray::refresh_tray_menu(&app, watcher.is_paused());
        Ok(())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().manage(WatcherManager::default());

    #[cfg(desktop)]
    let builder = builder.plugin(single_instance_plugin());

    #[expect(
        clippy::disallowed_types,
        reason = "Tauri's generated context uses std::collections::HashMap internally"
    )]
    let context = tauri::generate_context!();

    let app = builder
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
                    let result = if watcher.is_paused() {
                        watcher.start(&app).await
                    } else {
                        watcher.stop(&app).await
                    };
                    if result.is_err() {
                        eprintln!("Background worker control failed");
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
                if watcher.launch(&handle).await.is_err() {
                    eprintln!("Background worker unavailable");
                }
                tray::refresh_tray_menu(&handle, watcher.is_paused());
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_worker_status,
            set_capture_opt_in,
            stop_background_worker,
            list_dirs,
            add_dir,
            remove_dir,
            discover_mailcache_dirs,
            get_close_behavior,
            set_close_behavior,
            get_auto_update,
            set_auto_update,
            set_auto_start,
            get_app_settings,
            request_app_quit,
            minimize_to_tray,
            watcher_commands::reprocess_all,
            watcher_commands::pause_watcher,
            watcher_commands::resume_watcher
        ])
        .build(context)
        .expect("error while building tauri application");

    // UI exit intentionally leaves the independent per-user agent running.
    app.run(|_app, _event| {});
}

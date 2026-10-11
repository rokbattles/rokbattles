//! Desktop application setup and commands exposed to the Tauri frontend.
//!
//! `app_config` stores preferences and watched directories. `mailcache_discovery`
//! locates candidate directories, and `directory_operations` limits the blocking
//! filesystem work requested by frontend commands.
//!
//! `watcher_manager` coordinates scanning, pausing, reprocessing, and shutdown.
//! The `watcher` module discovers mail and persists upload progress. `updater`
//! downloads updates while scanning continues, then asks the manager to stop the
//! watcher before installation. `tray` reflects the manager's lifecycle status.

mod app_config;
mod directory_operations;
mod mailcache_discovery;
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

    // The plugin's /app executable path is only valid inside the Flatpak sandbox,
    // so a host autostart entry cannot launch it directly.
    if is_flatpak() {
        return Ok(());
    }

    let handle = app.handle();
    handle.plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))?;

    let enabled = match app_config::get_auto_start(handle) {
        Ok(enabled) => enabled,
        Err(error) => {
            eprintln!("Skipping autostart setup because settings could not be read: {error:#}");
            return Ok(());
        }
    };

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
    let dirs = app_config::read_config(app)?.dirs.unwrap_or_default();
    Ok(dedupe_dirs_preserving_representation(dirs))
}

#[tauri::command]
async fn list_dirs(app: AppHandle) -> Result<Vec<String>, String> {
    app.state::<directory_operations::DirectoryOperations>()
        .run(false, {
            let app = app.clone();

            move || {
                let current = read_dirs(&app)?;
                Ok(dirs_for_ui(&current))
            }
        })
        .await
}

#[tauri::command]
async fn add_dir(app: AppHandle, paths: Vec<String>) -> Result<Vec<String>, String> {
    app.state::<directory_operations::DirectoryOperations>()
        .run(false, {
            let app = app.clone();

            move || {
                app_config::update_dirs(&app, |dirs| {
                    merge_directories(dirs, &paths);
                    dirs_for_ui(dirs)
                })
            }
        })
        .await
}

#[tauri::command]
async fn remove_dir(app: AppHandle, path: String) -> Result<Vec<String>, String> {
    app.state::<directory_operations::DirectoryOperations>()
        .run(false, {
            let app = app.clone();

            move || {
                let target_key = dir_identity_key(&path);

                app_config::update_dirs(&app, |dirs| {
                    *dirs = dedupe_dirs_preserving_representation(std::mem::take(dirs));
                    dirs.retain(|dir| dir_identity_key(dir) != target_key);
                    dirs.sort();

                    dirs_for_ui(dirs)
                })
            }
        })
        .await
}

fn merge_directories(dirs: &mut Vec<String>, paths: &[String]) -> (Vec<String>, Vec<String>) {
    *dirs = dedupe_dirs_preserving_representation(std::mem::take(dirs));
    let mut known_keys: BTreeSet<String> = dirs.iter().map(|dir| dir_identity_key(dir)).collect();
    let mut added = Vec::new();
    let mut already_watched = Vec::new();

    for path in paths {
        let normalized = normalize_dir_for_display(path);
        if normalized.is_empty() {
            continue;
        }

        if known_keys.insert(dir_identity_key(&normalized)) {
            dirs.push(normalized.clone());
            added.push(normalized);
        } else {
            already_watched.push(normalized);
        }
    }

    dirs.sort();
    (added, already_watched)
}

#[derive(Debug, Serialize)]
struct DiscoverMailcacheResult {
    added_dirs: Vec<String>,
    already_watched_dirs: Vec<String>,
    incomplete: bool,
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
    app.state::<directory_operations::DirectoryOperations>()
        .run(true, {
            let app = app.clone();
            move || discover_and_save(&app)
        })
        .await
}

fn discover_and_save(app: &AppHandle) -> anyhow::Result<DiscoverMailcacheResult> {
    let discovered = mailcache_discovery::discover_mailcache_dirs();
    let paths =
        discovered.dirs.iter().map(|path| path.to_string_lossy().into_owned()).collect::<Vec<_>>();

    // Read settings after the scan, then retry the merge if directory choices change.
    let (added_dirs, already_watched_dirs) = if paths.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        app_config::update_dirs(app, |dirs| merge_directories(dirs, &paths))?
    };

    let mut message = if !added_dirs.is_empty() {
        let count = added_dirs.len();
        format!(
            "Auto-discovered and added {count} mailcache director{}.",
            if count == 1 { "y" } else { "ies" }
        )
    } else if !already_watched_dirs.is_empty() {
        "All discovered mailcache directories are already being watched.".to_string()
    } else {
        "No valid mailcache directories were found.".to_string()
    };

    if discovered.incomplete {
        message.push_str(
            " Some locations could not be fully searched. You can add missing folders manually.",
        );
    }

    Ok(DiscoverMailcacheResult {
        added_dirs,
        already_watched_dirs,
        incomplete: discovered.incomplete,
        message,
    })
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

    app_config::set_close_behavior(&app, behavior).map_err(|error| format!("{error:#}"))
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
    app_config::set_auto_update(&app, enabled).map_err(|error| format!("{error:#}"))
}

#[tauri::command]
fn set_auto_start(app: AppHandle, enabled: bool) -> Result<(), String> {
    if is_flatpak() {
        return Err("Automatic startup is unavailable in the Flatpak package.".to_string());
    }

    let store = app.state::<app_config::ConfigStore>();

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;

        store
            .set_auto_start(
                enabled,
                || app.autolaunch().is_enabled().map_err(anyhow::Error::new),
                |value| apply_auto_start_setting(&app, value).map_err(anyhow::Error::msg),
            )
            .map_err(|error| format!("{error:#}"))
    }

    #[cfg(not(desktop))]
    store.update(|config| config.auto_start = enabled).map_err(|error| format!("{error:#}"))
}

#[tauri::command]
fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    let config = app_config::read_config(&app).map_err(|error| format!("{error:#}"))?;

    Ok(AppSettings {
        auto_update: !is_flatpak() && config.auto_update,
        auto_start: !is_flatpak() && config.auto_start,
        close_behavior: config.close_behavior,
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

mod watcher_commands {
    use super::*;

    #[tauri::command]
    pub(super) async fn reprocess_all(app: AppHandle) -> Result<(), String> {
        app.state::<WatcherManager>().reprocess().await
    }

    #[tauri::command]
    pub(super) async fn pause_watcher(app: AppHandle) -> Result<(), String> {
        app.state::<WatcherManager>().pause().await
    }

    #[tauri::command]
    pub(super) async fn resume_watcher(app: AppHandle) -> Result<(), String> {
        app.state::<WatcherManager>().start().await
    }
}

/// Starts the Tauri application and its background services.
///
/// # Panics
///
/// Panics if Tauri cannot build or initialize the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

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
                    if let Err(error) = app.state::<WatcherManager>().toggle().await {
                        watcher::emit_log(&app, error);
                    }
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
            let config_path = app.path().app_config_dir()?.join("config.json");
            app.manage(app_config::ConfigStore::new(config_path));
            app.manage(directory_operations::DirectoryOperations::default());

            #[cfg(desktop)]
            setup_autostart(app)?;

            app.manage(WatcherManager::new(app.handle()));
            let status = app.state::<WatcherManager>().status();
            if tray_supported() {
                tray::setup_tray(app, status)?;
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let auto_update = match app_config::get_auto_update(&handle) {
                    Ok(enabled) => enabled,
                    Err(error) => {
                        watcher::emit_log(
                            &handle,
                            format!("Skipping automatic update check because settings could not be read: {error:#}"),
                        );
                        return;
                    }
                };

                updater::maybe_check_for_updates(handle.clone(), auto_update).await;
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

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let manager = app.state::<WatcherManager>();
            if manager.exit_ready() {
                return;
            }

            api.prevent_exit();

            if manager.mark_exit_requested() {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    match handle.state::<WatcherManager>().exit().await {
                        Ok(()) => handle.exit(0),
                        Err(error) => watcher::emit_log(&handle, error),
                    }
                });
            }
        }
    });
}

#[cfg(all(test, unix))]
mod directory_tests {
    use super::*;

    #[test]
    fn merging_discovered_alias_preserves_one_existing_manual_path() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().expect("directory fixture");
        let actual = temp.path().join("mailcache");
        let alias = temp.path().join("manual-alias");
        std::fs::create_dir(&actual).expect("cache");
        symlink(&actual, &alias).expect("manual alias");
        let manual = alias.to_string_lossy().into_owned();
        let discovered = actual.to_string_lossy().into_owned();
        let mut saved = vec![manual.clone()];

        let (added, already_watched) =
            merge_directories(&mut saved, std::slice::from_ref(&discovered));

        assert!(added.is_empty());
        assert_eq!(already_watched, vec![discovered]);
        assert_eq!(saved, vec![manual.clone()]);
        assert_eq!(dirs_for_ui(&saved), vec![manual]);
    }
}

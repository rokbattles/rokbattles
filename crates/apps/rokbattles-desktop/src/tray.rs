use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const TRAY_ID: &str = "main";
pub(crate) const TRAY_SHOW_MENU_ID: &str = "tray_show";
pub(crate) const TRAY_TOGGLE_WATCHER_MENU_ID: &str = "tray_toggle_watcher";
pub(crate) const TRAY_QUIT_MENU_ID: &str = "tray_quit";

fn tray_toggle_label(paused: bool) -> &'static str {
    if paused { "Resume watcher" } else { "Pause watcher" }
}

fn build_tray_menu(app: &AppHandle, paused: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, TRAY_SHOW_MENU_ID, "Show", true, None::<&str>)?;
    let toggle = MenuItem::with_id(
        app,
        TRAY_TOGGLE_WATCHER_MENU_ID,
        tray_toggle_label(paused),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, TRAY_QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    Menu::with_items(app, &[&show, &toggle, &separator, &quit])
}

pub(crate) fn refresh_tray_menu(app: &AppHandle, paused: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    match build_tray_menu(app, paused) {
        Ok(menu) => {
            if let Err(e) = tray.set_menu(Some(menu)) {
                eprintln!("[rokbattles] failed to refresh tray menu: {}", e);
            }
        }
        Err(e) => eprintln!("[rokbattles] failed to build tray menu: {}", e),
    }
}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        drop(window.show());
        drop(window.unminimize());
        drop(window.set_focus());
    }
}

pub(crate) fn hide_main_window(app: &AppHandle) {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            drop(window.hide());
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _app = app;
    }
}

pub(crate) fn setup_tray(app: &tauri::App<tauri::Wry>, paused: bool) -> tauri::Result<()> {
    let tray_menu = build_tray_menu(&app.handle().clone(), paused)?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&tray_menu)
        .tooltip("ROK Battles")
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

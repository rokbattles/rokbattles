//! Persistent launcher chrome and independently owned content webviews.
//! Switching tabs never sends a watcher lifecycle command.

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Rect, Webview, WebviewUrl, Window,
    WindowEvent,
    webview::{NewWindowResponse, WebviewBuilder},
};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::{
    app_config, tray,
    watcher_manager::{WatcherManager, WatcherStatus},
};

#[cfg(target_os = "linux")]
mod linux;

pub(crate) fn setup(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let shell = app.get_webview(tray::MAIN_WINDOW_LABEL).ok_or("Launcher is unavailable")?;
        linux::attach(&shell, true)?;
    }

    #[cfg(not(target_os = "linux"))]
    let _ = app;

    Ok(())
}

// Keep these logical-pixel dimensions in sync with the launcher CSS.
const TITLE_HEIGHT: f64 = 48.0;
const FOOTER_HEIGHT: f64 = 34.0;
const EDGE: f64 = 8.0;
const BORDER: f64 = 1.0;
const APP_URL: &str = "https://rokbattles.com/";
const APP_INIT: &str = include_str!("shell-init.js");

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum View {
    #[default]
    Desktop,
    App,
}

impl View {
    fn label(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::App => "app",
        }
    }
}

pub(crate) struct ShellState {
    active: Mutex<View>,
    app_url: tauri::Url,
}

impl ShellState {
    pub(crate) fn new() -> Result<Self, String> {
        let app_url = APP_URL.parse().map_err(|error| format!("Invalid App URL: {error}"))?;

        #[cfg(debug_assertions)]
        let app_url = match std::env::var("ROKBATTLES_DESKTOP_APP_URL") {
            Ok(url) => development_app_url(&url)?,
            Err(_) => app_url,
        };

        Ok(Self { active: Mutex::new(View::Desktop), app_url })
    }

    pub(crate) fn app_url(&self) -> &tauri::Url {
        &self.app_url
    }
}

#[cfg(debug_assertions)]
fn development_app_url(value: &str) -> Result<tauri::Url, String> {
    let url = tauri::Url::parse(value).map_err(|error| error.to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("ROKBATTLES_DESKTOP_APP_URL must be a loopback site origin".into());
    }

    Ok(url)
}

fn content_bounds(size: LogicalSize<f64>) -> Rect {
    Rect {
        position: LogicalPosition::new(EDGE + BORDER, TITLE_HEIGHT + BORDER).into(),
        size: LogicalSize::new(
            (size.width - 2.0 * (EDGE + BORDER)).max(1.0),
            (size.height - TITLE_HEIGHT - FOOTER_HEIGHT - 2.0 * BORDER).max(1.0),
        )
        .into(),
    }
}

fn window_bounds(window: &Window) -> tauri::Result<Rect> {
    Ok(content_bounds(window.inner_size()?.to_logical(window.scale_factor()?)))
}

fn is_app_url(url: &tauri::Url, app_url: &tauri::Url) -> bool {
    url.origin() == app_url.origin()
}

fn open_external(app: &AppHandle, url: &tauri::Url) {
    if matches!(url.scheme(), "https" | "http" | "mailto")
        && let Err(error) = app.opener().open_url(url.as_str(), None::<&str>)
    {
        eprintln!("[rokbattles] failed to open external link: {error}");
    }
}

fn create_view(window: &Window, view: View) -> Result<Webview, String> {
    let bounds = window_bounds(window).map_err(|error| error.to_string())?;
    let builder = match view {
        View::Desktop => WebviewBuilder::new(view.label(), WebviewUrl::App("desktop.html".into())),
        View::App => {
            let navigation_app = window.app_handle().clone();
            let popup_app = navigation_app.clone();
            let app_url = window.app_handle().state::<ShellState>().app_url.clone();
            let origin = serde_json::to_string(&app_url.origin().ascii_serialization())
                .map_err(|error| error.to_string())?;
            let initialization = APP_INIT.replace("\"__ROKBATTLES_APP_ORIGIN__\"", &origin);

            WebviewBuilder::new(view.label(), WebviewUrl::External(app_url.clone()))
                .initialization_script(initialization)
                .on_navigation(move |url| {
                    if is_app_url(url, &app_url) && url.path() == "/proxy/v1/auth/discord/login" {
                        crate::desktop_auth::start(&navigation_app);
                        false
                    } else if is_app_url(url, &app_url) {
                        true
                    } else {
                        open_external(&navigation_app, url);
                        false
                    }
                })
                .on_new_window(move |url, _| {
                    open_external(&popup_app, &url);
                    NewWindowResponse::Deny
                })
        }
    }
    .devtools(cfg!(debug_assertions))
    .disable_drag_drop_handler()
    .background_color(tauri::utils::config::Color(9, 9, 11, 255));

    let webview = window
        .add_child(builder, bounds.position, bounds.size)
        .map_err(|error| error.to_string())?;

    #[cfg(target_os = "linux")]
    if let Err(error) = linux::attach(&webview, false) {
        if let Err(close_error) = webview.close() {
            eprintln!("[rokbattles] failed to clean up webview: {close_error}");
        }

        return Err(error);
    }

    Ok(webview)
}

/// Async because WebView2 creation can deadlock inside a synchronous IPC handler.
#[tauri::command]
pub(crate) async fn select_view(
    app: AppHandle,
    webview: Webview,
    view: View,
) -> Result<(), String> {
    if webview.label() != tray::MAIN_WINDOW_LABEL {
        return Err("Only the launcher can switch views".into());
    }

    let state = app.state::<ShellState>();
    let mut active = state.active.lock().await;
    let window = app.get_window(tray::MAIN_WINDOW_LABEL).ok_or("Main window is unavailable")?;

    // Create the destination first so a failure leaves the current view usable.
    let destination = match app.get_webview(view.label()) {
        Some(webview) => webview,
        None => create_view(&window, view)?,
    };

    #[cfg(not(target_os = "linux"))]
    destination
        .set_bounds(window_bounds(&window).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    destination.show().map_err(|error| error.to_string())?;

    match view {
        View::App => {
            if let Some(desktop) = app.get_webview(View::Desktop.label()) {
                desktop.close().map_err(|error| error.to_string())?;
            }
        }
        View::Desktop => {
            if let Some(remote) = app.get_webview(View::App.label()) {
                remote.hide().map_err(|error| error.to_string())?;
            }
        }
    }

    *active = view;
    if let Err(error) = destination.set_focus() {
        eprintln!("[rokbattles] failed to focus {}: {error}", view.label());
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn open_feedback(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url("https://discord.gg/rokbattles", None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn close_main_window(app: AppHandle) -> Result<(), String> {
    if crate::get_close_behavior(app.clone())? == app_config::CloseBehavior::MinimizeToTray {
        let window = app.get_window(tray::MAIN_WINDOW_LABEL).ok_or("Main window is unavailable")?;
        window.hide().map_err(|error| error.to_string())?;
    } else {
        app.exit(0);
    }

    Ok(())
}

pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != tray::MAIN_WINDOW_LABEL {
        return;
    }

    match event {
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
            #[cfg(not(target_os = "linux"))]
            if let Ok(bounds) = window_bounds(window) {
                for label in [View::Desktop.label(), View::App.label()] {
                    if let Some(webview) = window.app_handle().get_webview(label)
                        && let Err(error) = webview.set_bounds(bounds)
                    {
                        eprintln!("[rokbattles] failed to resize {label}: {error}");
                    }
                }
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Err(error) = close_main_window(window.app_handle().clone()) {
                eprintln!("[rokbattles] failed to close window: {error}");
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CaptureStatus {
    NotConfigured,
    Active,
    Paused,
    Starting,
    Stopping,
    Failed,
    Updating,
    Exiting,
}

fn capture_status(configured: bool, watcher: WatcherStatus) -> CaptureStatus {
    if !configured {
        return CaptureStatus::NotConfigured;
    }

    match watcher {
        WatcherStatus::Running => CaptureStatus::Active,
        WatcherStatus::Paused => CaptureStatus::Paused,
        WatcherStatus::Starting => CaptureStatus::Starting,
        WatcherStatus::Stopping => CaptureStatus::Stopping,
        WatcherStatus::Failed => CaptureStatus::Failed,
        WatcherStatus::Updating => CaptureStatus::Updating,
        WatcherStatus::Exiting => CaptureStatus::Exiting,
    }
}

#[tauri::command]
pub(crate) fn get_capture_status(app: AppHandle) -> Result<CaptureStatus, String> {
    let configured = !crate::read_dirs(&app).map_err(|error| error.to_string())?.is_empty();

    Ok(capture_status(configured, app.state::<WatcherManager>().status()))
}

pub(crate) fn notify_capture_changed(app: &AppHandle) {
    if let Err(error) = app.emit_to(tray::MAIN_WINDOW_LABEL, "capture-status-changed", ()) {
        eprintln!("[rokbattles] failed to notify capture status: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_never_appears_active_without_configured_directories() {
        for status in [WatcherStatus::Running, WatcherStatus::Starting, WatcherStatus::Paused] {
            assert_eq!(capture_status(false, status), CaptureStatus::NotConfigured);
        }
    }

    #[test]
    fn capture_reports_paused_failed_and_transition_states_truthfully() {
        for (watcher, expected) in [
            (WatcherStatus::Running, CaptureStatus::Active),
            (WatcherStatus::Paused, CaptureStatus::Paused),
            (WatcherStatus::Failed, CaptureStatus::Failed),
            (WatcherStatus::Stopping, CaptureStatus::Stopping),
            (WatcherStatus::Updating, CaptureStatus::Updating),
        ] {
            assert_eq!(capture_status(true, watcher), expected);
        }
    }

    #[test]
    fn content_keeps_title_footer_and_side_edges_clear() {
        let bounds = content_bounds(LogicalSize::new(1200.0, 800.0));

        assert_eq!(bounds.position, LogicalPosition::new(9.0, 49.0).into());
        assert_eq!(bounds.size, LogicalSize::new(1182.0, 716.0).into());
    }

    #[test]
    fn only_the_production_origin_can_navigate_inside_app() {
        let app_url = APP_URL.parse().expect("app URL");

        assert!(is_app_url(
            &"https://rokbattles.com/reports?test=1".parse().expect("url"),
            &app_url
        ));

        for url in [
            "http://rokbattles.com/",
            "https://rokbattles.com.evil.test/",
            "https://rokbattles.com:444/",
            "https://discord.gg/rokbattles",
            "file:///tmp/test",
        ] {
            assert!(!is_app_url(&url.parse().expect("url"), &app_url));
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn development_override_is_limited_to_an_exact_loopback_origin() {
        let app_url = development_app_url("http://localhost:3000/").expect("local site");

        assert!(is_app_url(&"http://localhost:3000/account".parse().expect("url"), &app_url));
        assert!(!is_app_url(&"http://localhost:1420/".parse().expect("url"), &app_url));
        assert!(!is_app_url(&APP_URL.parse().expect("url"), &app_url));

        for url in [
            "https://example.com/",
            "http://localhost.evil.test/",
            "http://user@localhost:3000/",
            "http://localhost:3000/path",
            "http://localhost:3000/?redirect=bad",
        ] {
            development_app_url(url).expect_err("invalid development origin");
        }
    }
}

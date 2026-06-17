use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Manager};

pub(crate) const DEEP_LINK_SCHEME: &str = "rokbattles";
pub(crate) const LOCALHOST_LOOPBACK_HOST: &str = "127.0.0.1";
pub(crate) const LOCALHOST_LOOPBACK_PORT: u16 = 17654;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExternalCallbackTransport {
    DeepLink,
    LocalhostLoopback,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalCallbackConfig {
    pub(crate) deep_link_scheme: &'static str,
    pub(crate) deep_link_available: bool,
    pub(crate) localhost_loopback_available: bool,
    pub(crate) localhost_loopback_started: bool,
    pub(crate) localhost_loopback_active: bool,
    pub(crate) localhost_loopback_port: u16,
    pub(crate) localhost_loopback_origin: String,
    pub(crate) preferred_transport: ExternalCallbackTransport,
}

#[derive(Debug)]
pub(crate) struct ExternalCallbackState {
    deep_link_available: AtomicBool,
    localhost_loopback_started: AtomicBool,
    localhost_loopback_active: AtomicBool,
}

impl Default for ExternalCallbackState {
    fn default() -> Self {
        Self {
            deep_link_available: AtomicBool::new(true),
            localhost_loopback_started: AtomicBool::new(false),
            localhost_loopback_active: AtomicBool::new(false),
        }
    }
}

impl ExternalCallbackState {
    pub(crate) fn set_deep_link_available(&self, available: bool) {
        self.deep_link_available.store(available, Ordering::Relaxed);
    }

    pub(crate) fn claim_localhost_loopback_start(&self) -> bool {
        self.localhost_loopback_started
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }

    pub(crate) fn reset_localhost_loopback_start(&self) {
        self.localhost_loopback_started.store(false, Ordering::Relaxed);
    }

    pub(crate) fn reserve_localhost_loopback(&self) -> ExternalCallbackConfig {
        self.localhost_loopback_active.store(true, Ordering::Relaxed);
        self.config()
    }

    pub(crate) fn release_localhost_loopback(&self) -> ExternalCallbackConfig {
        self.localhost_loopback_active.store(false, Ordering::Relaxed);
        self.config()
    }

    pub(crate) fn config(&self) -> ExternalCallbackConfig {
        let deep_link_available = self.deep_link_available.load(Ordering::Relaxed);
        let localhost_loopback_started = self.localhost_loopback_started.load(Ordering::Relaxed);
        let localhost_loopback_active = self.localhost_loopback_active.load(Ordering::Relaxed);

        ExternalCallbackConfig {
            deep_link_scheme: DEEP_LINK_SCHEME,
            deep_link_available,
            localhost_loopback_available: true,
            localhost_loopback_started,
            localhost_loopback_active,
            localhost_loopback_port: LOCALHOST_LOOPBACK_PORT,
            localhost_loopback_origin: format!(
                "http://{LOCALHOST_LOOPBACK_HOST}:{LOCALHOST_LOOPBACK_PORT}"
            ),
            preferred_transport: if deep_link_available {
                ExternalCallbackTransport::DeepLink
            } else {
                ExternalCallbackTransport::LocalhostLoopback
            },
        }
    }
}

#[expect(dead_code, reason = "Reserved for the future external auth callback flow.")]
pub(crate) fn reserve_localhost_loopback(app: &AppHandle) -> tauri::Result<ExternalCallbackConfig> {
    let state = app.state::<ExternalCallbackState>();

    if state.claim_localhost_loopback_start() {
        let plugin = tauri_plugin_localhost::Builder::new(LOCALHOST_LOOPBACK_PORT)
            .host(LOCALHOST_LOOPBACK_HOST)
            .build();

        if let Err(error) = app.plugin(plugin) {
            state.reset_localhost_loopback_start();
            return Err(error);
        }
    }

    Ok(state.reserve_localhost_loopback())
}

#[expect(dead_code, reason = "Reserved for the future external auth callback flow.")]
pub(crate) fn release_localhost_loopback(app: &AppHandle) -> ExternalCallbackConfig {
    // tauri-plugin-localhost does not currently expose a stop hook. This clears
    // the app-level reservation so callers know no callback flow is using it.
    app.state::<ExternalCallbackState>().release_localhost_loopback()
}

#[cfg(test)]
mod tests {
    use super::{ExternalCallbackState, ExternalCallbackTransport, LOCALHOST_LOOPBACK_PORT};

    #[test]
    fn config_should_prefer_deep_links_when_available() {
        let state = ExternalCallbackState::default();

        let config = state.config();

        assert_eq!(config.preferred_transport, ExternalCallbackTransport::DeepLink);
    }

    #[test]
    fn config_should_prefer_localhost_loopback_when_deep_links_are_unavailable() {
        let state = ExternalCallbackState::default();
        state.set_deep_link_available(false);

        let config = state.config();

        assert_eq!(config.preferred_transport, ExternalCallbackTransport::LocalhostLoopback);
    }

    #[test]
    fn reserve_and_release_should_track_localhost_loopback_activity() {
        let state = ExternalCallbackState::default();

        let reserved = state.reserve_localhost_loopback();
        assert!(reserved.localhost_loopback_active);
        assert!(!reserved.localhost_loopback_started);
        assert_eq!(reserved.localhost_loopback_port, LOCALHOST_LOOPBACK_PORT);

        let released = state.release_localhost_loopback();
        assert!(!released.localhost_loopback_active);
    }
}

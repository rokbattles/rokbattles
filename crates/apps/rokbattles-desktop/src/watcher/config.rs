use std::time::Duration;

/// Runtime tuning knobs for the watcher loop.
#[derive(Debug, Clone)]
pub(crate) struct WatcherConfig {
    /// Idle sleep to avoid busy looping when no work is pending.
    pub(crate) idle_sleep: Duration,
    /// Graceful shutdown timeout before aborting the task.
    pub(crate) shutdown_timeout: Duration,
    /// Refresh interval for reading config dirs.
    pub(crate) config_refresh_interval: Duration,
    /// Ingress upload endpoint.
    pub(crate) api_ingress_url: &'static str,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            idle_sleep: Duration::from_millis(750),
            shutdown_timeout: Duration::from_secs(3),
            config_refresh_interval: Duration::from_secs(2),
            api_ingress_url: "https://ingress.rokbattles.com/v2/upload",
        }
    }
}

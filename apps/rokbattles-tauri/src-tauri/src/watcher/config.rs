use std::time::Duration;

/// Runtime tuning knobs for the watcher loop.
#[derive(Debug, Clone)]
pub(crate) struct WatcherConfig {
    /// Current processed store filename.
    pub(crate) processed_file_name: &'static str,
    /// Pending upload queue filename.
    pub(crate) upload_queue_file_name: &'static str,
    /// Flush processed state after this many updates to cap memory growth.
    pub(crate) store_flush_every_updates: usize,
    /// Maximum time to keep processed state only in-memory.
    pub(crate) store_flush_interval: Duration,
    /// Directory rescan cadence while idle.
    pub(crate) dir_refresh_interval_idle: Duration,
    /// Directory rescan cadence while actively scanning.
    pub(crate) dir_refresh_interval_busy: Duration,
    /// Full directory rescan cadence to catch missed events.
    pub(crate) full_dir_refresh_interval: Duration,
    /// Max number of files to scan per loop tick.
    pub(crate) scan_budget_per_tick: usize,
    /// Idle sleep to avoid busy looping when no work is pending.
    pub(crate) idle_sleep: Duration,
    /// Max time to keep the upload queue only in-memory.
    pub(crate) queue_flush_interval: Duration,
    /// Flush upload queue after this many updates.
    pub(crate) queue_flush_every_updates: usize,
    /// Graceful shutdown timeout before aborting the task.
    pub(crate) shutdown_timeout: Duration,
    /// Prefetch this many uploads ahead of time.
    pub(crate) upload_prefetch_target: usize,
    /// Max number of recently changed paths to track for rescans.
    pub(crate) hot_tracked_limit: usize,
    /// Rescan interval for recently changed paths.
    pub(crate) hot_rescan_interval: Duration,
    /// Max hot paths to rescan per interval.
    pub(crate) hot_rescan_budget: usize,
    /// Minimum age before a file is considered stable enough to upload (ms).
    pub(crate) file_stable_age_ms: u128,
    /// Delay before retrying a file read failure (ms).
    pub(crate) file_retry_delay_ms: u128,
    /// Delay before retrying a changed file (ms).
    pub(crate) file_changed_delay_ms: u128,
    /// Max number of recent ids to validate after a full refresh.
    pub(crate) full_refresh_validate_recent: usize,
    /// Max paths to revalidate after a full refresh.
    pub(crate) full_refresh_validate_max_paths: usize,
    /// Limit filesystem events processed per tick.
    pub(crate) fs_event_budget: usize,
    /// Capacity for filesystem event channel.
    pub(crate) fs_event_queue_capacity: usize,
    /// Refresh interval for reading config dirs.
    pub(crate) config_refresh_interval: Duration,
    /// Ingress upload endpoint.
    pub(crate) api_ingress_url: &'static str,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            processed_file_name: "processed-v3.json",
            upload_queue_file_name: "upload-queue.json",
            store_flush_every_updates: 20_000,
            store_flush_interval: Duration::from_secs(300),
            dir_refresh_interval_idle: Duration::from_secs(5),
            dir_refresh_interval_busy: Duration::from_secs(60),
            full_dir_refresh_interval: Duration::from_secs(180),
            scan_budget_per_tick: 256,
            idle_sleep: Duration::from_millis(750),
            queue_flush_interval: Duration::from_secs(2),
            queue_flush_every_updates: 64,
            shutdown_timeout: Duration::from_secs(3),
            upload_prefetch_target: 4,
            hot_tracked_limit: 4096,
            hot_rescan_interval: Duration::from_millis(750),
            hot_rescan_budget: 64,
            file_stable_age_ms: 1500,
            file_retry_delay_ms: 750,
            file_changed_delay_ms: 1000,
            full_refresh_validate_recent: 5000,
            full_refresh_validate_max_paths: 512,
            fs_event_budget: 512,
            fs_event_queue_capacity: 4096,
            config_refresh_interval: Duration::from_secs(2),
            api_ingress_url: "https://ingress.rokbattles.com/v2/upload",
        }
    }
}

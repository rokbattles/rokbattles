use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    time::Instant,
};

use tauri::AppHandle;

use super::config::WatcherConfig;
use super::emit_log;
use super::store::{
    ProcessedStore, QueuedUpload, UploadQueueStore, write_processed, write_upload_queue,
};

pub(crate) struct WatcherState {
    pub(crate) config: WatcherConfig,
    pub(crate) scans: Vec<super::scan::DirScan>,
    pub(crate) dirs: Vec<PathBuf>,
    pub(crate) dirs_last_read: Instant,
    pub(crate) store: ProcessedStore,
    pub(crate) store_dirty_updates: usize,
    pub(crate) store_last_flush: Instant,
    pub(crate) upload_queue: VecDeque<QueuedUpload>,
    pub(crate) upload_queued_paths: HashSet<String>,
    pub(crate) upload_queue_dirty_updates: usize,
    pub(crate) upload_queue_last_flush: Instant,
    pub(crate) scan_refreshes: u64,
    pub(crate) store_flushes: u64,
    pub(crate) upload_queue_flushes: u64,
    pub(crate) hot_paths: VecDeque<String>,
    pub(crate) hot_set: HashSet<String>,
    pub(crate) hot_last_scan: Instant,
    pub(crate) api_backoff_until_ms: Option<u128>,
}

impl WatcherState {
    pub(crate) fn new(
        config: WatcherConfig,
        store: ProcessedStore,
        upload_queue_store: UploadQueueStore,
    ) -> Self {
        let mut upload_queue = VecDeque::new();
        let mut upload_queued_paths = HashSet::new();
        for item in upload_queue_store.items {
            if upload_queued_paths.insert(item.path.clone()) {
                upload_queue.push_back(item);
            }
        }

        let refresh_interval = config.config_refresh_interval;
        Self {
            config,
            scans: Vec::new(),
            dirs: Vec::new(),
            dirs_last_read: Instant::now()
                .checked_sub(refresh_interval)
                .unwrap_or_else(Instant::now),
            store,
            store_dirty_updates: 0,
            store_last_flush: Instant::now(),
            upload_queue,
            upload_queued_paths,
            upload_queue_dirty_updates: 0,
            upload_queue_last_flush: Instant::now(),
            scan_refreshes: 0,
            store_flushes: 0,
            upload_queue_flushes: 0,
            hot_paths: VecDeque::new(),
            hot_set: HashSet::new(),
            hot_last_scan: Instant::now(),
            api_backoff_until_ms: None,
        }
    }

    pub(crate) fn maybe_flush_store(&mut self, app: &AppHandle) {
        if self.store_dirty_updates == 0 {
            return;
        }
        if self.store_dirty_updates < self.config.store_flush_every_updates
            && self.store_last_flush.elapsed() < self.config.store_flush_interval
        {
            return;
        }
        if let Err(e) = write_processed(app, &self.config, &self.store) {
            emit_log(app, format!("Failed to flush processed store: {}", e));
            return;
        }
        self.store_dirty_updates = 0;
        self.store_last_flush = Instant::now();
        self.store_flushes += 1;
    }

    pub(crate) fn maybe_flush_upload_queue(&mut self, app: &AppHandle) {
        if self.upload_queue_dirty_updates == 0 {
            return;
        }
        if self.upload_queue_dirty_updates < self.config.queue_flush_every_updates
            && self.upload_queue_last_flush.elapsed() < self.config.queue_flush_interval
        {
            return;
        }
        let store = UploadQueueStore {
            version: 1,
            items: self.upload_queue.iter().cloned().collect(),
        };
        if let Err(e) = write_upload_queue(app, &self.config, &store) {
            emit_log(app, format!("Failed to flush upload queue: {}", e));
            return;
        }
        self.upload_queue_dirty_updates = 0;
        self.upload_queue_last_flush = Instant::now();
        self.upload_queue_flushes += 1;
    }

    pub(crate) fn enqueue_upload(&mut self, item: QueuedUpload) {
        let path = item.path.clone();
        if !self.upload_queued_paths.insert(item.path.clone()) {
            return;
        }
        self.upload_queue.push_back(item);
        self.upload_queue_dirty_updates += 1;
        self.track_hot_path(path);
    }

    pub(crate) fn pop_ready_upload(&mut self, now_ms: u128) -> Option<QueuedUpload> {
        let len = self.upload_queue.len();
        for _ in 0..len {
            let item = self.upload_queue.pop_front()?;
            if item.not_before_ms.is_some_and(|t| t > now_ms) {
                self.upload_queue.push_back(item);
                continue;
            }
            self.upload_queued_paths.remove(&item.path);
            self.upload_queue_dirty_updates += 1;
            return Some(item);
        }
        None
    }

    pub(crate) fn requeue_upload(&mut self, item: QueuedUpload) {
        if self.upload_queued_paths.insert(item.path.clone()) {
            self.upload_queue.push_back(item);
            self.upload_queue_dirty_updates += 1;
        }
    }

    pub(crate) fn maybe_rescan_hot(&mut self, now_ms: u128) {
        if self.hot_last_scan.elapsed() < self.config.hot_rescan_interval {
            return;
        }
        self.hot_last_scan = Instant::now();

        for _ in 0..self.config.hot_rescan_budget {
            let Some(path_str) = self.hot_paths.pop_front() else {
                break;
            };

            let path = PathBuf::from(&path_str);
            let Some(existing) = self.store.entries.get(&path_str).cloned() else {
                self.hot_paths.push_back(path_str);
                continue;
            };

            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => {
                    self.hot_paths.push_back(path_str);
                    continue;
                }
            };
            let sig = match super::store::file_sig(&meta) {
                Ok(s) => s,
                Err(_) => {
                    self.hot_paths.push_back(path_str);
                    continue;
                }
            };

            if sig != existing {
                self.store.entries.insert(path_str.clone(), sig.clone());
                self.store_dirty_updates += 1;
                self.enqueue_upload(QueuedUpload {
                    path: path_str.clone(),
                    sig,
                    attempts: 0,
                    not_before_ms: Some(now_ms.saturating_add(self.config.file_changed_delay_ms)),
                });
            }

            self.hot_paths.push_back(path_str);
        }
    }

    fn track_hot_path(&mut self, path: String) {
        if self.hot_set.contains(&path) {
            return;
        }
        if self.hot_paths.len() >= self.config.hot_tracked_limit
            && let Some(old) = self.hot_paths.pop_front()
        {
            self.hot_set.remove(&old);
        }
        self.hot_set.insert(path.clone());
        self.hot_paths.push_back(path);
    }

    pub(crate) fn rate_limit_remaining_ms(&mut self, now_ms: u128) -> Option<u128> {
        let until = self.api_backoff_until_ms?;
        if until <= now_ms {
            self.api_backoff_until_ms = None;
            return None;
        }
        Some(until.saturating_sub(now_ms))
    }

    pub(crate) fn extend_rate_limit(&mut self, now_ms: u128, backoff: std::time::Duration) -> bool {
        let until = now_ms.saturating_add(backoff.as_millis());
        match self.api_backoff_until_ms {
            Some(existing) if existing >= until => false,
            _ => {
                self.api_backoff_until_ms = Some(until);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::store::FileSig;
    use super::*;

    fn make_sig() -> FileSig {
        FileSig {
            size: 1,
            modified: 2,
        }
    }

    fn make_item(path: &str, not_before_ms: Option<u128>) -> QueuedUpload {
        QueuedUpload {
            path: path.to_string(),
            sig: make_sig(),
            attempts: 0,
            not_before_ms,
        }
    }

    #[test]
    fn enqueue_upload_dedupes_paths() {
        let mut state = WatcherState::new(
            WatcherConfig::default(),
            ProcessedStore::default(),
            UploadQueueStore::default(),
        );
        let item = make_item("a", None);

        state.enqueue_upload(item.clone());
        state.enqueue_upload(item);

        assert_eq!(state.upload_queue.len(), 1);
        assert_eq!(state.upload_queued_paths.len(), 1);
        assert_eq!(state.upload_queue_dirty_updates, 1);
    }

    #[test]
    fn pop_ready_upload_respects_not_before() {
        let mut state = WatcherState::new(
            WatcherConfig::default(),
            ProcessedStore::default(),
            UploadQueueStore::default(),
        );
        let item = make_item("a", Some(150));

        state.enqueue_upload(item);
        assert!(state.pop_ready_upload(100).is_none());
        assert_eq!(state.upload_queue.len(), 1);
        assert!(state.upload_queued_paths.contains("a"));

        let ready = state.pop_ready_upload(200).expect("expected ready upload");
        assert_eq!(ready.path, "a");
        assert!(state.upload_queue.is_empty());
        assert!(!state.upload_queued_paths.contains("a"));
    }

    #[test]
    fn requeue_upload_adds_only_once() {
        let mut state = WatcherState::new(
            WatcherConfig::default(),
            ProcessedStore::default(),
            UploadQueueStore::default(),
        );
        let item = make_item("a", None);

        state.enqueue_upload(item.clone());
        let popped = state.pop_ready_upload(0).expect("expected ready upload");
        state.requeue_upload(popped);
        state.requeue_upload(item);

        assert_eq!(state.upload_queue.len(), 1);
        assert_eq!(state.upload_queued_paths.len(), 1);
    }
}

use std::{collections::VecDeque, path::PathBuf, time::Instant};

use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    config::WatcherConfig,
    mail::mail_id_for_path,
    store::{FileSig, QueuedUpload, RejectedUpload, StateFile, UploadStore},
    truncate_reason,
    upload::upload_backoff,
};

pub(crate) struct WatcherState {
    pub(crate) config: WatcherConfig,
    pub(crate) scans: Vec<super::scan::DirScan>,
    pub(crate) dirs: Vec<PathBuf>,
    pub(crate) dirs_last_read: Instant,
    pub(crate) discovered: FxHashMap<String, FileSig>,
    pub(crate) uploads: UploadStore,
    dirty: bool,
    pub(crate) scan_refreshes: u64,
    pub(crate) hot_paths: VecDeque<String>,
    pub(crate) hot_set: FxHashSet<String>,
    pub(crate) hot_last_scan: Instant,
}

impl WatcherState {
    pub(crate) fn new(config: WatcherConfig, uploads: UploadStore) -> Self {
        let discovered = uploads
            .pending
            .iter()
            .filter_map(|item| item.sig.clone().map(|sig| (item.path.clone(), sig)))
            .chain(
                uploads
                    .rejected
                    .iter()
                    .filter_map(|(path, item)| item.sig.clone().map(|sig| (path.clone(), sig))),
            )
            .collect();

        let refresh_interval = config.config_refresh_interval;

        Self {
            config,
            scans: Vec::new(),
            dirs: Vec::new(),
            dirs_last_read: Instant::now()
                .checked_sub(refresh_interval)
                .unwrap_or_else(Instant::now),
            discovered,
            uploads,
            dirty: false,
            scan_refreshes: 0,
            hot_paths: VecDeque::new(),
            hot_set: FxHashSet::default(),
            hot_last_scan: Instant::now(),
        }
    }

    pub(crate) fn flush(&mut self, file: &StateFile) -> anyhow::Result<()> {
        if self.dirty {
            file.save(&self.uploads)?;

            self.dirty = false;
        }

        Ok(())
    }

    pub(crate) fn is_acknowledged(&self, path: &str) -> bool {
        mail_id_for_path(path).is_some_and(|id| self.uploads.acknowledged.contains(id))
    }

    pub(crate) fn enqueue_upload(&mut self, item: QueuedUpload) {
        if self.is_acknowledged(&item.path) {
            return;
        }

        if let Some(sig) = &item.sig {
            self.discovered.insert(item.path.clone(), sig.clone());
        }

        if self.uploads.pending.iter().any(|pending| pending.path == item.path) {
            return;
        }

        if self.uploads.rejected.get(&item.path).is_some_and(|rejected| rejected.sig == item.sig) {
            return;
        }

        self.uploads.rejected.remove(&item.path);
        self.track_hot_path(item.path.clone());
        self.uploads.pending.push_back(item);
        self.dirty = true;
    }

    /// Selecting work does not remove it: an interrupted request must reload as pending.
    pub(crate) fn next_ready_upload(&mut self, now_ms: u128) -> Option<QueuedUpload> {
        for _ in 0..self.uploads.pending.len() {
            let item = self.uploads.pending.pop_front()?;
            let ready = item.not_before_ms.is_none_or(|deadline| deadline <= now_ms);

            self.uploads.pending.push_back(item.clone());

            if ready {
                return Some(item);
            }
        }

        None
    }

    pub(crate) fn requeue_upload(&mut self, item: QueuedUpload) {
        if let Some(pending) =
            self.uploads.pending.iter_mut().find(|pending| pending.path == item.path)
        {
            *pending = item;
            self.dirty = true;
        }
    }

    pub(crate) fn retry_upload(&mut self, mut item: QueuedUpload, reason: String, now_ms: u128) {
        item.attempts = item.attempts.saturating_add(1);
        item.not_before_ms = Some(now_ms.saturating_add(upload_backoff(item.attempts).as_millis()));
        item.last_error = Some(truncate_reason(reason));

        self.requeue_upload(item);
    }

    pub(crate) fn acknowledge_upload(&mut self, path: &str) {
        let Some(id) = mail_id_for_path(path) else {
            return;
        };

        self.uploads.acknowledged.insert(id.to_string());
        self.uploads.pending.retain(|item| mail_id_for_path(&item.path) != Some(id));
        self.uploads.rejected.retain(|path, _| mail_id_for_path(path) != Some(id));
        self.dirty = true;
    }

    pub(crate) fn reject_upload(&mut self, item: QueuedUpload, reason: String) {
        self.uploads.pending.retain(|pending| pending.path != item.path);
        self.uploads
            .rejected
            .insert(item.path, RejectedUpload { sig: item.sig, reason: truncate_reason(reason) });
        self.dirty = true;
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
            let Some(existing) = self.discovered.get(&path_str).cloned() else {
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
                self.discovered.insert(path_str.clone(), sig.clone());

                self.enqueue_upload(QueuedUpload {
                    path: path_str.clone(),
                    sig: Some(sig),
                    attempts: 0,
                    last_error: None,
                    not_before_ms: Some(now_ms.saturating_add(self.config.file_changed_delay_ms)),
                });
            }

            self.hot_paths.push_back(path_str);
        }
    }

    pub(crate) fn prune_removed_dirs_state(&mut self, active_dirs: &[PathBuf]) -> usize {
        let previous = self.uploads.pending.len() + self.uploads.rejected.len();

        // Removing a watched directory deliberately cancels its sources, not its acknowledged IDs.
        self.uploads.pending.retain(|item| path_belongs_to_any_active_dir(&item.path, active_dirs));
        self.uploads.rejected.retain(|path, _| path_belongs_to_any_active_dir(path, active_dirs));
        self.discovered.retain(|path, _| path_belongs_to_any_active_dir(path, active_dirs));
        self.hot_paths.retain(|path| path_belongs_to_any_active_dir(path, active_dirs));
        self.hot_set.retain(|path| path_belongs_to_any_active_dir(path, active_dirs));

        let removed = previous - self.uploads.pending.len() - self.uploads.rejected.len();
        self.dirty |= removed > 0;

        removed
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
        let until = self.uploads.api_backoff_until_ms?;

        if until <= now_ms {
            self.uploads.api_backoff_until_ms = None;
            self.dirty = true;

            return None;
        }

        Some(until.saturating_sub(now_ms))
    }

    pub(crate) fn extend_rate_limit(&mut self, now_ms: u128, backoff: std::time::Duration) -> bool {
        let until = now_ms.saturating_add(backoff.as_millis());

        match self.uploads.api_backoff_until_ms {
            Some(existing) if existing >= until => false,
            _ => {
                self.uploads.api_backoff_until_ms = Some(until);
                self.dirty = true;

                true
            }
        }
    }
}

fn path_belongs_to_any_active_dir(path: &str, active_dirs: &[PathBuf]) -> bool {
    let path = PathBuf::from(path);

    active_dirs.iter().any(|dir| path.starts_with(dir))
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use super::*;

    fn item(path: &str) -> QueuedUpload {
        QueuedUpload {
            path: path.to_string(),
            sig: Some(FileSig { size: 10, modified: 20 }),
            attempts: 0,
            not_before_ms: None,
            last_error: None,
        }
    }

    #[test]
    fn discovery_and_in_flight_work_reload_as_pending() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.flush(&file).expect("persist discovery before request");

        let in_flight = state.next_ready_upload(1000).expect("select upload");
        state.flush(&file).expect("even a flush during the request retains pending work");
        drop(state);
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart during request");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));

        assert_eq!(restored.uploads.pending.len(), 1);
        assert_eq!(
            restored.next_ready_upload(1000).expect("retry interrupted request").path,
            in_flight.path
        );
        assert!(!restored.is_acknowledged(&in_flight.path));
    }

    #[test]
    fn unsaved_acknowledgement_retries_but_saved_acknowledgement_covers_all_copies() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        state.enqueue_upload(item("first/Persistent.Mail.123"));
        state.enqueue_upload(item("second/Persistent.Mail.123"));
        state.flush(&file).expect("persist pending sources");

        state.acknowledge_upload("first/Persistent.Mail.123");
        drop(state);
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart before acknowledgement write");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));

        assert_eq!(restored.uploads.pending.len(), 2);
        assert!(!restored.is_acknowledged("second/Persistent.Mail.123"));

        restored.acknowledge_upload("second/Persistent.Mail.123");
        restored.flush(&file).expect("persist acknowledgement");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart after acknowledgement write");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        restored.enqueue_upload(item("third/Persistent.Mail.123"));

        assert!(restored.uploads.pending.is_empty());
        assert!(restored.is_acknowledged("third/Persistent.Mail.123"));
        assert!(!restored.is_acknowledged("third/Persistent.Mail.0123"));
    }

    #[test]
    fn retry_reason_deadline_and_rate_limit_survive_graceful_pause() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.retry_upload(
            item("Persistent.Mail.123"),
            "temporarily unavailable".to_string(),
            1000,
        );
        state.extend_rate_limit(1000, Duration::from_secs(7));
        state.flush(&file).expect("final flush below all former batching thresholds");
        drop(file);

        let file = StateFile::open(dir.path()).expect("resume watcher");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));

        assert!(restored.next_ready_upload(2999).is_none());
        assert_eq!(restored.rate_limit_remaining_ms(1000), Some(7000));

        let ready = restored.next_ready_upload(3000).expect("retry after saved deadline");

        assert_eq!(ready.attempts, 1);
        assert_eq!(ready.last_error.as_deref(), Some("temporarily unavailable"));
    }

    #[test]
    fn failed_flush_reports_error_and_preserves_pending_recovery_and_dirty_acknowledgement() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.flush(&file).expect("persist pending upload");
        state.acknowledge_upload("Persistent.Mail.123");

        // A destination that cannot be replaced injects a real filesystem write failure.
        let snapshot = dir.path().join("upload-state-v1.json");
        let recovery = dir.path().join("recovery.json");
        fs::rename(&snapshot, &recovery).expect("retain previous snapshot");
        fs::create_dir(&snapshot).expect("block replacement");

        assert!(state.flush(&file).is_err());

        fs::remove_dir(&snapshot).expect("restore writable destination");
        fs::rename(&recovery, &snapshot).expect("restore previous snapshot");

        let interrupted = file.load().expect("recover previous snapshot after failure");

        assert_eq!(interrupted.pending.len(), 1);
        assert!(interrupted.acknowledged.is_empty());

        state.flush(&file).expect("dirty acknowledgement is retried");
        let completed = file.load().expect("saved acknowledgement");

        assert!(completed.pending.is_empty());
        assert!(completed.acknowledged.contains("123"));
    }

    #[test]
    fn rejected_source_does_not_block_other_copies_and_changes_allow_retry() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        state.enqueue_upload(item("first/Persistent.Mail.123"));
        state.enqueue_upload(item("second/Persistent.Mail.123"));
        state.reject_upload(item("first/Persistent.Mail.123"), "decode failed".to_string());
        state.flush(&file).expect("persist rejection");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        restored.enqueue_upload(item("first/Persistent.Mail.123"));

        assert_eq!(restored.uploads.pending.len(), 1);
        assert_eq!(
            restored.next_ready_upload(1000).expect("alternative source").path,
            "second/Persistent.Mail.123"
        );
        assert_eq!(
            restored
                .uploads
                .rejected
                .get("first/Persistent.Mail.123")
                .expect("recorded rejection")
                .reason,
            "decode failed"
        );
        assert!(restored.uploads.acknowledged.is_empty());

        let mut changed = item("first/Persistent.Mail.123");
        changed.sig = Some(FileSig { size: 20, modified: 30 });
        restored.enqueue_upload(changed);

        assert_eq!(restored.uploads.pending.len(), 2);
        assert!(restored.uploads.rejected.is_empty());
    }

    #[test]
    fn removing_directory_cancels_its_sources_but_preserves_other_work_and_acknowledgements() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut state = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));
        state.enqueue_upload(item("active/Persistent.Mail.1"));
        state.enqueue_upload(item("removed/Persistent.Mail.1"));
        state.enqueue_upload(item("removed/Persistent.Mail.2"));
        state.enqueue_upload(item("removed/Persistent.Mail.3"));
        state.reject_upload(item("removed/Persistent.Mail.2"), "unsupported".to_string());
        state.acknowledge_upload("removed/Persistent.Mail.3");

        state.prune_removed_dirs_state(&[PathBuf::from("active")]);
        state.flush(&file).expect("persist deliberate cancellation");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(WatcherConfig::default(), file.load().expect("load"));

        assert_eq!(restored.uploads.pending.len(), 1);
        assert_eq!(
            restored.next_ready_upload(1000).expect("remaining work").path,
            "active/Persistent.Mail.1"
        );
        assert!(restored.uploads.rejected.is_empty());
        assert!(restored.is_acknowledged("elsewhere/Persistent.Mail.3"));

        restored.prune_removed_dirs_state(&[]);
        restored.flush(&file).expect("explicit removal of last directory");

        assert!(file.load().expect("cancelled work saved").pending.is_empty());
    }
}

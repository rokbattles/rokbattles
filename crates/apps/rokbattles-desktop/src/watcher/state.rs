//! In-memory upload transitions and directory scan progress.
//!
//! [`WatcherState`] keeps its pending-path index in sync with [`UploadStore`] and
//! queues record changes for [`WatcherState::flush`]. A failed transaction leaves
//! the changes queued; no HTTP request starts until discovery is durable.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use rustc_hash::FxHashSet;

use super::{
    mail::mail_id_for_path,
    store::{QueuedUpload, RejectedUpload, StoreChange, UploadDb, UploadStore},
    upload::upload_backoff,
};

pub(crate) struct WatcherState {
    pub(crate) scans: VecDeque<super::scan::DirScan>,
    pub(crate) dirs: Vec<PathBuf>,
    pub(crate) dirs_last_read: Instant,
    pending_paths: FxHashSet<String>,
    pub(crate) uploads: UploadStore,
    changes: Vec<StoreChange>,
}

impl WatcherState {
    pub(crate) fn new(uploads: UploadStore) -> Self {
        let pending_paths = uploads.pending.iter().map(|item| item.path.clone()).collect();

        Self {
            scans: VecDeque::new(),
            dirs: Vec::new(),
            dirs_last_read: Instant::now()
                .checked_sub(Duration::from_secs(2))
                .unwrap_or_else(Instant::now),
            pending_paths,
            uploads,
            changes: Vec::new(),
        }
    }

    pub(crate) fn flush(&mut self, db: &UploadDb) -> anyhow::Result<()> {
        db.commit(&self.changes)?;
        self.changes.clear();

        Ok(())
    }

    pub(crate) fn is_acknowledged(&self, path: &str) -> bool {
        mail_id_for_path(path).is_some_and(|id| self.uploads.acknowledged.contains(id))
    }

    pub(crate) fn is_pending(&self, path: &str) -> bool {
        self.pending_paths.contains(path)
    }

    pub(crate) fn enqueue_upload(&mut self, item: QueuedUpload) {
        if self.is_acknowledged(&item.path) {
            return;
        }

        if self.is_pending(&item.path) {
            return;
        }

        if self.uploads.rejected.get(&item.path).is_some_and(|rejected| rejected.sig == item.sig) {
            return;
        }

        self.uploads.rejected.remove(&item.path);
        self.pending_paths.insert(item.path.clone());
        self.changes.push(StoreChange::Pending(item.clone()));
        self.uploads.pending.push_back(item);
    }

    /// Returns a copy of the next pending upload whose retry delay has elapsed.
    ///
    /// Selection rotates the queue without removing the source. An interrupted
    /// request therefore remains pending when the saved state is loaded again.
    /// The caller checks the separate API-wide rate limit before selecting work.
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
            self.changes.push(StoreChange::Pending(item.clone()));
            *pending = item;
        }
    }

    pub(crate) fn retry_upload(&mut self, mut item: QueuedUpload, now_ms: u128) {
        item.attempts = item.attempts.saturating_add(1).min(10);
        item.not_before_ms = Some(now_ms.saturating_add(upload_backoff(item.attempts).as_millis()));

        self.requeue_upload(item);
    }

    pub(crate) fn remove_pending_source(&mut self, path: &str) {
        if self.pending_paths.remove(path) {
            self.uploads.pending.retain(|item| item.path != path);
            self.changes.push(StoreChange::Remove(path.to_string()));
        }
    }

    pub(crate) fn acknowledge_upload(&mut self, path: &str) {
        let Some(id) = mail_id_for_path(path) else {
            return;
        };

        self.uploads.acknowledged.insert(id.to_string());
        self.pending_paths.retain(|path| mail_id_for_path(path) != Some(id));
        self.uploads.pending.retain(|item| mail_id_for_path(&item.path) != Some(id));
        self.uploads.rejected.retain(|path, _| mail_id_for_path(path) != Some(id));

        self.changes.push(StoreChange::Acknowledge(id.to_string()));
    }

    pub(crate) fn reject_upload(&mut self, item: QueuedUpload) {
        self.pending_paths.remove(&item.path);
        self.uploads.pending.retain(|pending| pending.path != item.path);

        self.changes.push(StoreChange::Rejected { path: item.path.clone(), sig: item.sig.clone() });
        self.uploads.rejected.insert(item.path, RejectedUpload { sig: item.sig });
    }

    pub(crate) fn prune_removed_dirs_state(&mut self, active_dirs: &[PathBuf]) -> usize {
        let previous = self.uploads.pending.len() + self.uploads.rejected.len();

        // Removing a watched directory deliberately cancels its sources, not its acknowledged IDs.
        self.uploads.pending.retain(|item| {
            let retain = path_belongs_to_any_active_dir(&item.path, active_dirs);

            if !retain {
                self.changes.push(StoreChange::Remove(item.path.clone()));
            }

            retain
        });

        self.uploads.rejected.retain(|path, _| {
            let retain = path_belongs_to_any_active_dir(path, active_dirs);

            if !retain {
                self.changes.push(StoreChange::Remove(path.clone()));
            }

            retain
        });
        self.pending_paths.retain(|path| path_belongs_to_any_active_dir(path, active_dirs));

        previous - self.uploads.pending.len() - self.uploads.rejected.len()
    }

    pub(crate) fn rate_limit_remaining_ms(&mut self, now_ms: u128) -> Option<u128> {
        let until = self.uploads.api_backoff_until_ms?;

        if until <= now_ms {
            self.uploads.api_backoff_until_ms = None;
            self.changes.push(StoreChange::Backoff(None));

            return None;
        }

        Some(until.saturating_sub(now_ms))
    }

    pub(crate) fn extend_rate_limit(&mut self, now_ms: u128, backoff: Duration) -> bool {
        let until = now_ms.saturating_add(backoff.as_millis());

        match self.uploads.api_backoff_until_ms {
            Some(existing) if existing >= until => false,
            _ => {
                self.uploads.api_backoff_until_ms = Some(until);
                self.changes.push(StoreChange::Backoff(Some(until)));

                true
            }
        }
    }
}

fn path_belongs_to_any_active_dir(path: &str, active_dirs: &[PathBuf]) -> bool {
    Path::new(path).parent().is_some_and(|parent| active_dirs.iter().any(|dir| dir == parent))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{super::store::FileSig, *};

    fn item(path: &str) -> QueuedUpload {
        QueuedUpload {
            path: path.to_string(),
            sig: Some(FileSig { size: 10, modified: 20 }),
            attempts: 0,
            not_before_ms: None,
        }
    }

    #[test]
    fn discovery_and_in_flight_work_reload_as_pending() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.flush(&db).expect("persist discovery before request");

        let in_flight = state.next_ready_upload(1000).expect("select upload");
        state.flush(&db).expect("even a flush during the request retains pending work");
        drop(state);
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart during request");
        let mut restored = WatcherState::new(db.load().expect("load"));

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
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("first/Persistent.Mail.123"));
        state.enqueue_upload(item("second/Persistent.Mail.123"));
        state.flush(&db).expect("persist pending sources");

        state.acknowledge_upload("first/Persistent.Mail.123");
        drop(state);
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart before acknowledgement write");
        let mut restored = WatcherState::new(db.load().expect("load"));

        assert_eq!(restored.uploads.pending.len(), 2);
        assert!(!restored.is_acknowledged("second/Persistent.Mail.123"));

        restored.acknowledge_upload("second/Persistent.Mail.123");
        restored.flush(&db).expect("persist acknowledgement");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart after acknowledgement write");
        let mut restored = WatcherState::new(db.load().expect("load"));
        restored.enqueue_upload(item("third/Persistent.Mail.123"));

        assert!(restored.uploads.pending.is_empty());
        assert!(restored.is_acknowledged("third/Persistent.Mail.123"));
        assert!(!restored.is_acknowledged("third/Persistent.Mail.0123"));
    }

    #[test]
    fn retry_deadline_and_rate_limit_survive_graceful_pause() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.retry_upload(item("Persistent.Mail.123"), 1000);
        state.extend_rate_limit(1000, Duration::from_secs(7));
        state.flush(&db).expect("save retries and rate limit before pause");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("resume watcher");
        let mut restored = WatcherState::new(db.load().expect("load"));

        assert!(restored.next_ready_upload(2999).is_none());
        assert_eq!(restored.rate_limit_remaining_ms(1000), Some(7000));

        let ready = restored.next_ready_upload(3000).expect("retry after saved deadline");

        assert_eq!(ready.attempts, 1);
    }

    #[test]
    fn repeated_failures_keep_a_bounded_retry_count_across_restart() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        let mut now_ms = 1000;

        for _ in 0..100 {
            let item = state.next_ready_upload(now_ms).expect("retry ready");
            state.retry_upload(item, now_ms);
            now_ms += 300_000;
        }

        state.flush(&db).expect("persist bounded attempts");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(db.load().expect("load"));
        let ready = restored.next_ready_upload(now_ms).expect("retry after maximum delay");

        assert_eq!(ready.attempts, 10);
        assert_eq!(ready.not_before_ms, Some(now_ms));
    }

    #[test]
    fn expired_rate_limit_is_cleared_on_disk() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.extend_rate_limit(1000, Duration::from_secs(7));
        state.flush(&db).expect("persist rate limit");

        assert_eq!(state.rate_limit_remaining_ms(7999), Some(1));
        assert_eq!(state.rate_limit_remaining_ms(8000), None);

        state.flush(&db).expect("persist expiry");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");

        assert!(db.load().expect("expired backoff").api_backoff_until_ms.is_none());
    }

    #[test]
    fn failed_flush_preserves_pending_recovery_and_retries_unsaved_acknowledgement() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.flush(&db).expect("persist pending upload");
        state.acknowledge_upload("Persistent.Mail.123");

        let fault =
            rusqlite::Connection::open(dir.path().join("local.db")).expect("fault connection");
        fault
            .execute_batch(
                "CREATE TRIGGER fail_ack BEFORE INSERT ON acknowledged
                 BEGIN
                    SELECT RAISE(ABORT, 'injected write failure');
                 END;",
            )
            .expect("inject a transaction failure");

        assert!(state.flush(&db).is_err());

        fault.execute_batch("DROP TRIGGER fail_ack;").expect("restore writes");

        let interrupted = db.load().expect("recover committed state after failure");

        assert_eq!(interrupted.pending.len(), 1);
        assert!(interrupted.acknowledged.is_empty());

        state.flush(&db).expect("unsaved acknowledgement is retried");
        let completed = db.load().expect("saved acknowledgement");

        assert!(completed.pending.is_empty());
        assert!(completed.acknowledged.contains("123"));
    }

    #[test]
    fn rejected_source_does_not_block_other_copies_and_changes_allow_retry() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("first/Persistent.Mail.123"));
        state.enqueue_upload(item("second/Persistent.Mail.123"));
        state.reject_upload(item("first/Persistent.Mail.123"));
        state.flush(&db).expect("persist rejection");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(db.load().expect("load"));
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
                .sig,
            Some(FileSig { size: 10, modified: 20 })
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
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("active/Persistent.Mail.1"));
        state.enqueue_upload(item("removed/Persistent.Mail.1"));
        state.enqueue_upload(item("removed/Persistent.Mail.2"));
        state.enqueue_upload(item("removed/Persistent.Mail.3"));
        state.reject_upload(item("removed/Persistent.Mail.2"));
        state.acknowledge_upload("removed/Persistent.Mail.3");

        state.prune_removed_dirs_state(&[PathBuf::from("active")]);
        state.flush(&db).expect("persist deliberate cancellation");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(db.load().expect("load"));

        assert_eq!(restored.uploads.pending.len(), 1);
        assert_eq!(
            restored.next_ready_upload(1000).expect("remaining work").path,
            "active/Persistent.Mail.1"
        );
        assert!(restored.uploads.rejected.is_empty());
        assert!(restored.is_acknowledged("elsewhere/Persistent.Mail.3"));

        restored.prune_removed_dirs_state(&[]);
        restored.flush(&db).expect("explicit removal of last directory");

        assert!(db.load().expect("cancelled work saved").pending.is_empty());
    }
}

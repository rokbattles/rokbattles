//! Tracks capture and upload schedules, retained payload sizes, and scan progress.
//!
//! State changes queue database updates for [`WatcherState::flush`]. Failed commits
//! retain those updates for retry. The runtime flushes captured bytes before upload.

use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    mail::mail_id_for_path,
    store::{
        MAX_PREPARED_BYTES, MAX_PREPARED_MAILS, QueuedUpload, RejectedUpload, StoreChange,
        UploadDb, UploadStore,
    },
    upload::upload_backoff,
};

pub(crate) struct WatcherState {
    pub(crate) scans: VecDeque<super::scan::DirScan>,
    pub(crate) dirs: Vec<PathBuf>,
    /// Resolved notification directories mapped to their configured spellings.
    pub(crate) event_dir_aliases: FxHashMap<PathBuf, PathBuf>,
    pub(crate) dirs_last_read: Instant,
    // Both phases retain their pending records while a worker holds the reservation.
    ready: BTreeSet<(u128, String)>,
    prepared_ready: BTreeSet<(u128, String)>,
    /// Retained payload bytes, including writes queued for the next flush.
    pub(crate) prepared_bytes: u64,
    sources_by_id: FxHashMap<String, FxHashSet<String>>,
    // Sharing reservations prevents copies of an ID from racing across the two phases.
    in_flight_ids: FxHashSet<String>,
    pub(crate) uploads: UploadStore,
    changes: Vec<StoreChange>,
}

impl WatcherState {
    pub(crate) fn new(uploads: UploadStore) -> Self {
        let mut ready = BTreeSet::new();
        let mut prepared_ready = BTreeSet::new();

        for item in uploads.pending.values() {
            if mail_id_for_path(&item.path).is_some_and(|id| uploads.prepared.contains_key(id)) {
                prepared_ready.insert(schedule_key(item));
            } else {
                ready.insert(schedule_key(item));
            }
        }
        let prepared_bytes = uploads.prepared.values().sum();
        let mut sources_by_id: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();

        for path in uploads.pending.keys().chain(uploads.rejected.keys()) {
            if let Some(id) = mail_id_for_path(path) {
                sources_by_id.entry(id.to_string()).or_default().insert(path.clone());
            }
        }

        Self {
            scans: VecDeque::new(),
            dirs: Vec::new(),
            event_dir_aliases: FxHashMap::default(),
            dirs_last_read: Instant::now()
                .checked_sub(Duration::from_secs(2))
                .unwrap_or_else(Instant::now),
            ready,
            prepared_ready,
            prepared_bytes,
            sources_by_id,
            in_flight_ids: FxHashSet::default(),
            uploads,
            changes: Vec::new(),
        }
    }

    /// Commits queued changes, retaining them if the transaction fails.
    pub(crate) fn flush(&mut self, db: &UploadDb) -> anyhow::Result<()> {
        db.commit(&self.changes)?;
        self.changes.clear();

        Ok(())
    }

    pub(crate) fn is_acknowledged(&self, path: &str) -> bool {
        mail_id_for_path(path).is_some_and(|id| self.uploads.acknowledged.contains(id))
    }

    pub(crate) fn is_pending(&self, path: &str) -> bool {
        self.uploads.pending.contains_key(path)
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

        if let Some(id) = mail_id_for_path(&item.path) {
            self.sources_by_id.entry(id.to_string()).or_default().insert(item.path.clone());
        }

        self.schedule(&item);
        self.changes.push(StoreChange::Pending(item.clone()));
        self.uploads.pending.insert(item.path.clone(), item);
    }

    /// Reserves the next source that needs reading and classification.
    ///
    /// Returns `None` if no scheduled source is ready. Copies of the same mail ID
    /// cannot be reserved concurrently. Selection leaves the source pending for
    /// recovery after interruption.
    /// API backoff does not affect this schedule.
    pub(crate) fn next_preparation(&mut self, now_ms: u128) -> Option<QueuedUpload> {
        self.reserve_next(now_ms, false)
    }

    /// Reserves retained mail whose retry deadline has passed.
    ///
    /// Flush pending changes and check the API-wide rate limit before starting a request.
    pub(crate) fn next_ready_upload(&mut self, now_ms: u128) -> Option<QueuedUpload> {
        self.reserve_next(now_ms, true)
    }

    fn reserve_next(&mut self, now_ms: u128, prepared: bool) -> Option<QueuedUpload> {
        let schedule = if prepared { &mut self.prepared_ready } else { &mut self.ready };
        let key = schedule
            .iter()
            .take_while(|(deadline, _)| *deadline <= now_ms)
            .find(|(_, path)| !self.in_flight_ids.contains(upload_key(path)))?
            .clone();
        let item = self.uploads.pending.get(&key.1)?.clone();
        schedule.remove(&key);
        self.in_flight_ids.insert(upload_key(&item.path).to_string());

        Some(item)
    }

    /// Releases an attempt's mail ID reservation without rescheduling its source.
    pub(crate) fn finish_attempt(&mut self, path: &str) {
        self.in_flight_ids.remove(upload_key(path));
    }

    pub(crate) fn requeue_upload(&mut self, item: QueuedUpload) {
        self.finish_attempt(&item.path);

        if let Some(pending) = self.uploads.pending.get(&item.path) {
            let old = schedule_key(pending);
            self.ready.remove(&old);
            self.prepared_ready.remove(&old);
            self.schedule(&item);
            self.changes.push(StoreChange::Pending(item.clone()));
            self.uploads.pending.insert(item.path.clone(), item);
        }
    }

    pub(crate) fn capture_has_room(&self) -> bool {
        self.prepared_bytes < MAX_PREPARED_BYTES && self.uploads.prepared.len() < MAX_PREPARED_MAILS
    }

    /// Queues validated source bytes for durable storage and upload.
    ///
    /// Returns `false` if adding the payload would exceed a backlog limit. Existing
    /// payloads are retained and the source stays pending. A removed source is ignored.
    /// Call [`Self::flush`] before starting the upload.
    pub(crate) fn stage_upload(&mut self, item: QueuedUpload, bytes: Vec<u8>) -> bool {
        self.stage_upload_with_limit(item, bytes, MAX_PREPARED_BYTES)
    }

    fn stage_upload_with_limit(&mut self, item: QueuedUpload, bytes: Vec<u8>, limit: u64) -> bool {
        self.finish_attempt(&item.path);

        if !self.is_pending(&item.path) {
            return true;
        }

        let Some(id) = mail_id_for_path(&item.path).map(str::to_owned) else {
            return true;
        };
        let size = bytes.len() as u64;

        if self.uploads.prepared.contains_key(&id) {
            self.requeue_upload(item);
            return true;
        }

        if size > limit.saturating_sub(self.prepared_bytes)
            || self.uploads.prepared.len() >= MAX_PREPARED_MAILS
        {
            self.requeue_upload(item);
            return false;
        }

        self.prepared_bytes += size;
        self.uploads.prepared.insert(id.clone(), size);
        self.changes.push(StoreChange::Prepared { mail_id: id.clone(), bytes });
        self.requeue_upload(item);
        self.reschedule_copies(&id);

        true
    }

    fn schedule(&mut self, item: &QueuedUpload) {
        let prepared =
            mail_id_for_path(&item.path).is_some_and(|id| self.uploads.prepared.contains_key(id));
        let schedule = if prepared { &mut self.prepared_ready } else { &mut self.ready };
        schedule.insert(schedule_key(item));
    }

    fn reschedule_copies(&mut self, id: &str) {
        if let Some(paths) = self.sources_by_id.get(id) {
            let prepared = self.uploads.prepared.contains_key(id);

            for path in paths {
                if let Some(item) = self.uploads.pending.get(path) {
                    let key = schedule_key(item);
                    self.ready.remove(&key);
                    self.prepared_ready.remove(&key);

                    if prepared {
                        self.prepared_ready.insert(key);
                    } else {
                        self.ready.insert(key);
                    }
                }
            }
        }
    }

    fn discard_prepared(&mut self, id: &str) {
        if let Some(size) = self.uploads.prepared.remove(id) {
            self.prepared_bytes -= size;
            self.reschedule_copies(id);
        }
    }

    pub(crate) fn retry_upload(&mut self, mut item: QueuedUpload, now_ms: u128) {
        item.attempts = item.attempts.saturating_add(1).min(10);
        item.not_before_ms = Some(now_ms.saturating_add(upload_backoff(item.attempts).as_millis()));

        self.requeue_upload(item);
    }

    pub(crate) fn remove_pending_source(&mut self, path: &str) {
        if let Some(item) = self.uploads.pending.remove(path) {
            self.ready.remove(&schedule_key(&item));
            self.prepared_ready.remove(&schedule_key(&item));
            self.forget_source(path);
            self.discard_unreferenced_payload(path);
            self.changes.push(StoreChange::Remove(path.to_string()));
        }
    }

    pub(crate) fn acknowledge_upload(&mut self, path: &str) {
        let Some(id) = mail_id_for_path(path) else {
            return;
        };

        self.uploads.acknowledged.insert(id.to_string());
        self.finish_attempt(path);

        if let Some(paths) = self.sources_by_id.remove(id) {
            for path in paths {
                if let Some(item) = self.uploads.pending.remove(&path) {
                    self.ready.remove(&schedule_key(&item));
                    self.prepared_ready.remove(&schedule_key(&item));
                }

                self.uploads.rejected.remove(&path);
            }
        }

        self.discard_prepared(id);

        self.changes.push(StoreChange::Acknowledge(id.to_string()));
    }

    pub(crate) fn reject_upload(&mut self, item: QueuedUpload) {
        self.finish_attempt(&item.path);

        if let Some(pending) = self.uploads.pending.remove(&item.path) {
            self.ready.remove(&schedule_key(&pending));
            self.prepared_ready.remove(&schedule_key(&pending));
        }

        if let Some(id) = mail_id_for_path(&item.path) {
            self.discard_prepared(id);
        }

        self.changes.push(StoreChange::Rejected { path: item.path.clone(), sig: item.sig.clone() });
        self.uploads.rejected.insert(item.path, RejectedUpload { sig: item.sig });
    }

    /// Removes pending and rejected sources outside `active_dirs`, returning their count.
    ///
    /// Acknowledged IDs and reservations for attempts still in flight are retained.
    pub(crate) fn prune_removed_dirs_state(&mut self, active_dirs: &[PathBuf]) -> usize {
        let removed: Vec<_> = self
            .uploads
            .pending
            .keys()
            .chain(self.uploads.rejected.keys())
            .filter(|path| !path_belongs_to_any_active_dir(path, active_dirs))
            .cloned()
            .collect();
        let count = removed.len();

        for path in removed {
            if let Some(item) = self.uploads.pending.remove(&path) {
                self.ready.remove(&schedule_key(&item));
                self.prepared_ready.remove(&schedule_key(&item));
            }

            self.uploads.rejected.remove(&path);
            self.forget_source(&path);
            self.discard_unreferenced_payload(&path);
            self.changes.push(StoreChange::Remove(path));
        }

        count
    }

    fn forget_source(&mut self, path: &str) {
        if let Some(id) = mail_id_for_path(path)
            && let Some(paths) = self.sources_by_id.get_mut(id)
        {
            paths.remove(path);

            if paths.is_empty() {
                self.sources_by_id.remove(id);
            }
        }
    }

    fn discard_unreferenced_payload(&mut self, path: &str) {
        if let Some(id) = mail_id_for_path(path) {
            let has_pending_copy = self.sources_by_id.get(id).is_some_and(|paths| {
                paths.iter().any(|path| self.uploads.pending.contains_key(path))
            });

            if !has_pending_copy {
                self.discard_prepared(id);
            }
        }
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

fn schedule_key(item: &QueuedUpload) -> (u128, String) {
    (item.not_before_ms.unwrap_or(0), item.path.clone())
}

fn upload_key(path: &str) -> &str {
    mail_id_for_path(path).unwrap_or(path)
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
    fn prepared_payload_and_retry_deadline_survive_restart_without_another_source_read() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("mail/Persistent.Mail.1"));
        state.flush(&db).expect("durable source");
        let selected = state.next_preparation(1000).expect("source to capture");
        assert!(state.stage_upload(selected, b"retained bytes".to_vec()));
        state.flush(&db).expect("durable payload");
        let upload = state.next_ready_upload(1000).expect("captured source to upload");
        state.retry_upload(upload, 1000);
        state.flush(&db).expect("durable retry");
        let mut state = WatcherState::new(db.load().expect("restore"));

        assert_eq!(state.prepared_bytes, 14);
        assert!(state.next_preparation(10_000).is_none(), "captured bytes need no source read");
        assert!(state.next_ready_upload(2999).is_none());
        assert_eq!(state.next_ready_upload(3000).expect("retry").path, "mail/Persistent.Mail.1");
        assert_eq!(db.read_prepared("1").expect("payload"), b"retained bytes");
    }

    #[test]
    fn source_removed_during_capture_does_not_leave_an_orphan_payload() {
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item("mail/Persistent.Mail.1"));
        let selected = state.next_preparation(1000).expect("capture reservation");
        state.prune_removed_dirs_state(&[]);
        assert!(state.stage_upload(selected, b"captured after removal".to_vec()));

        assert!(state.uploads.prepared.is_empty());
        assert_eq!(state.prepared_bytes, 0);
        assert!(state.next_ready_upload(1000).is_none());
        state.enqueue_upload(item("new/Persistent.Mail.1"));
        assert!(state.next_preparation(1000).is_some(), "late completion released its reservation");
    }

    #[test]
    fn staging_limit_keeps_captured_bytes_and_leaves_new_sources_pending() {
        let mut state = WatcherState::new(UploadStore::default());

        for id in 1..=3 {
            state.enqueue_upload(item(&format!("mail/Persistent.Mail.{id}")));
        }

        let first = state.next_preparation(1000).expect("first source");
        assert!(state.stage_upload_with_limit(first.clone(), vec![1; 4], 8));
        let second = state.next_preparation(1000).expect("second source");
        assert!(!state.stage_upload_with_limit(second.clone(), vec![2; 5], 8));

        assert_eq!(state.prepared_bytes, 4);
        assert!(state.is_pending(&second.path));
        assert!(state.uploads.rejected.is_empty());
        assert_eq!(
            state.next_ready_upload(1000).expect("captured source survives").path,
            first.path
        );
        state.acknowledge_upload(&first.path);
        assert_eq!(state.prepared_bytes, 0);
        assert!(
            state.stage_upload_with_limit(second, vec![2; 5], 8),
            "acknowledgement frees capacity"
        );
    }

    #[test]
    fn acknowledgement_and_permanent_rejection_clean_up_payloads_and_schedule_other_copies() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item("a/Persistent.Mail.1"));
        state.enqueue_upload(item("b/Persistent.Mail.1"));
        state.flush(&db).expect("sources");
        let first = state.next_preparation(1000).expect("first copy");
        state.stage_upload(first, b"first bytes".to_vec());
        state.flush(&db).expect("first payload");
        assert!(state.next_preparation(1000).is_none(), "copies share captured bytes");
        let upload = state.next_ready_upload(1000).expect("upload");
        state.reject_upload(upload);
        state.flush(&db).expect("rejected payload removed");

        db.read_prepared("1").expect_err("permanently rejected payload is removed");
        assert_eq!(state.prepared_bytes, 0);
        let other = state.next_preparation(1000).expect("another copy can supply different bytes");
        assert_eq!(other.path, "b/Persistent.Mail.1");
        state.stage_upload(other, b"second bytes".to_vec());
        state.flush(&db).expect("replacement payload");
        state.acknowledge_upload("b/Persistent.Mail.1");
        state.flush(&db).expect("acknowledgement");

        assert_eq!(state.prepared_bytes, 0);
        assert!(db.load().expect("saved state").prepared.is_empty());
        assert!(state.next_preparation(1000).is_none());
        assert!(state.next_ready_upload(1000).is_none());
    }

    #[test]
    fn directory_removal_keeps_payload_until_the_last_pending_copy_is_removed() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item("a/Persistent.Mail.1"));
        state.enqueue_upload(item("b/Persistent.Mail.1"));
        state.flush(&db).expect("sources");
        let source = state.next_preparation(1000).expect("source");
        state.stage_upload(source, b"captured".to_vec());
        state.flush(&db).expect("payload");
        state.prune_removed_dirs_state(&[PathBuf::from("b")]);
        state.flush(&db).expect("one copy removed");

        assert_eq!(db.read_prepared("1").expect("other copy retains bytes"), b"captured");
        assert_eq!(state.prepared_bytes, 8);
        state.prune_removed_dirs_state(&[]);
        state.flush(&db).expect("last copy removed");

        assert!(db.load().expect("saved state").prepared.is_empty());
        assert_eq!(state.prepared_bytes, 0);
        assert!(state.next_ready_upload(1000).is_none());
    }

    #[test]
    fn failed_staging_transaction_retains_changes_and_rolls_back_payload_insert() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item("mail/Persistent.Mail.1"));
        state.flush(&db).expect("source");
        let source = state.next_preparation(1000).expect("capture");
        state.stage_upload(source, b"captured".to_vec());
        let fault =
            rusqlite::Connection::open(dir.path().join("local.db")).expect("fault connection");
        fault.execute_batch("CREATE TRIGGER fail_source_update BEFORE UPDATE ON sources BEGIN SELECT RAISE(ABORT, 'injected staging failure'); END;").expect("fault trigger");

        assert!(state.flush(&db).is_err());
        assert!(db.read_prepared("1").is_err(), "payload and metadata commit together");
        assert_eq!(db.load().expect("durable state").pending.len(), 1);
        fault.execute_batch("DROP TRIGGER fail_source_update;").expect("restore writes");
        state.flush(&db).expect("retry complete staging transaction");

        assert_eq!(db.read_prepared("1").expect("saved payload"), b"captured");
    }

    #[test]
    fn concurrent_selection_reserves_exact_ids_and_retry_releases_other_copies() {
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item("a/Persistent.Mail.1"));
        state.enqueue_upload(item("b/Persistent.Mail.1"));
        state.enqueue_upload(item("a/Persistent.Mail.01"));

        let first = state.next_preparation(1000).expect("first exact ID");
        let second = state.next_preparation(1000).expect("leading zero ID is distinct");
        let ids: FxHashSet<_> = [&first, &second]
            .into_iter()
            .map(|item| mail_id_for_path(&item.path).expect("ID"))
            .collect();

        assert_eq!(ids, FxHashSet::from_iter(["1", "01"]));
        assert!(state.next_preparation(1000).is_none(), "a copy of an active ID must wait");
        assert_eq!(state.uploads.pending.len(), 3, "reservations stay durable");

        let active = if mail_id_for_path(&first.path) == Some("1") { first } else { second };
        state.retry_upload(active.clone(), 1000);
        let alternative =
            state.next_preparation(1000).expect("other copy can bypass delayed retry");

        assert_ne!(active.path, alternative.path);
        assert_eq!(mail_id_for_path(&alternative.path), Some("1"));

        state.acknowledge_upload(&alternative.path);
        assert_eq!(state.uploads.pending.len(), 1, "acknowledgement covers both copies of 1");
        assert!(state.next_preparation(10_000).is_none(), "active 01 stays reserved");
    }

    #[test]
    fn rescheduling_replaces_deadlines_and_cancelled_sources_leave_no_ready_entries() {
        let mut state = WatcherState::new(UploadStore::default());
        let mut delayed = item("mail/Persistent.Mail.1");
        delayed.not_before_ms = Some(5000);
        state.enqueue_upload(delayed.clone());
        delayed.not_before_ms = Some(2000);
        state.requeue_upload(delayed.clone());

        assert!(state.next_preparation(1999).is_none());
        assert_eq!(state.next_preparation(2000).expect("earlier deadline").path, delayed.path);

        delayed.not_before_ms = Some(10_000);
        state.requeue_upload(delayed.clone());
        assert!(state.next_preparation(5000).is_none(), "old deadline was removed");

        state.remove_pending_source(&delayed.path);
        assert!(state.next_preparation(20_000).is_none(), "removed path cannot remain scheduled");
        state.enqueue_upload(item(&delayed.path));
        assert!(state.next_preparation(20_000).is_some(), "rediscovery schedules the source again");
    }

    #[test]
    fn restored_rejected_and_pending_copies_are_all_removed_by_acknowledgement() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item("a/Persistent.Mail.1"));
        state.enqueue_upload(item("b/Persistent.Mail.1"));
        state.reject_upload(item("a/Persistent.Mail.1"));
        state.flush(&db).expect("persist both copies");
        let mut state = WatcherState::new(db.load().expect("restore source index"));
        let selected = state.next_preparation(1000).expect("remaining copy");
        state.acknowledge_upload(&selected.path);
        state.flush(&db).expect("persist acknowledgement");

        assert!(state.uploads.pending.is_empty());
        assert!(state.uploads.rejected.is_empty());
        assert!(state.next_preparation(1000).is_none());
        let restored = db.load().expect("persisted cleanup");
        assert!(restored.pending.is_empty());
        assert!(restored.rejected.is_empty());
    }

    #[test]
    fn removed_active_source_holds_its_id_until_its_attempt_finishes() {
        let mut state = WatcherState::new(UploadStore::default());
        state.enqueue_upload(item("a/Persistent.Mail.1"));
        state.enqueue_upload(item("b/Persistent.Mail.1"));
        let active = state.next_preparation(1000).expect("active copy");
        state.prune_removed_dirs_state(&[PathBuf::from("b")]);

        assert!(state.next_preparation(1000).is_none(), "removed request is still in flight");
        state.finish_attempt(&active.path);
        assert_eq!(
            state.next_preparation(1000).expect("retained copy").path,
            "b/Persistent.Mail.1"
        );
    }

    #[test]
    fn discovery_and_in_flight_work_reload_as_pending() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("upload database");
        let mut state = WatcherState::new(db.load().expect("load"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.enqueue_upload(item("Persistent.Mail.123"));
        state.flush(&db).expect("persist discovery before request");

        let in_flight = state.next_preparation(1000).expect("select upload");
        state.flush(&db).expect("even a flush during the request retains pending work");
        drop(state);
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart during request");
        let mut restored = WatcherState::new(db.load().expect("load"));

        assert_eq!(restored.uploads.pending.len(), 1);
        assert_eq!(
            restored.next_preparation(1000).expect("retry interrupted request").path,
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

        assert!(restored.next_preparation(2999).is_none());
        assert_eq!(restored.rate_limit_remaining_ms(1000), Some(7000));

        let ready = restored.next_preparation(3000).expect("retry after saved deadline");

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
            let item = state.next_preparation(now_ms).expect("retry ready");
            state.retry_upload(item, now_ms);
            now_ms += 300_000;
        }

        state.flush(&db).expect("persist bounded attempts");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let mut restored = WatcherState::new(db.load().expect("load"));
        let ready = restored.next_preparation(now_ms).expect("retry after maximum delay");

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
            restored.next_preparation(1000).expect("alternative source").path,
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
            restored.next_preparation(1000).expect("remaining work").path,
            "active/Persistent.Mail.1"
        );
        assert!(restored.uploads.rejected.is_empty());
        assert!(restored.is_acknowledged("elsewhere/Persistent.Mail.3"));

        restored.prune_removed_dirs_state(&[]);
        restored.flush(&db).expect("explicit removal of last directory");

        assert!(db.load().expect("cancelled work saved").pending.is_empty());
    }
}

use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use rustc_hash::FxHashSet;
use tauri::AppHandle;

use super::{
    emit_log,
    mail::parse_rok_mail_id,
    state::WatcherState,
    store::{QueuedUpload, file_sig},
};

#[derive(Debug, Clone)]
pub(crate) struct DirScan {
    pub(crate) dir: PathBuf,
    pub(crate) ids: Vec<u128>,
    pub(crate) known_ids: FxHashSet<u128>,
    pub(crate) cursor: usize,
    pub(crate) last_refresh: Instant,
    pub(crate) max_id: Option<u128>,
    pub(crate) last_full_refresh: Instant,
}

impl DirScan {
    pub(crate) fn new(
        dir: PathBuf,
        dir_refresh_interval_busy: Duration,
        full_dir_refresh_interval: Duration,
    ) -> Self {
        Self {
            dir,
            ids: Vec::new(),
            known_ids: FxHashSet::default(),
            cursor: 0,
            last_refresh: Instant::now()
                .checked_sub(dir_refresh_interval_busy.max(full_dir_refresh_interval))
                .unwrap_or_else(Instant::now),
            max_id: None,
            last_full_refresh: Instant::now()
                .checked_sub(full_dir_refresh_interval)
                .unwrap_or_else(Instant::now),
        }
    }

    fn is_stale(
        &self,
        dir_refresh_interval_idle: Duration,
        dir_refresh_interval_busy: Duration,
    ) -> bool {
        let interval =
            if self.cursor == 0 { dir_refresh_interval_idle } else { dir_refresh_interval_busy };
        self.last_refresh.elapsed() >= interval
    }

    fn replace_ids(&mut self, ids: Vec<u128>) {
        self.max_id = ids.last().copied();
        self.known_ids = ids.iter().copied().collect();
        self.cursor = ids.len();
        self.ids = ids;
    }

    fn peek_next_id(&self) -> Option<u128> {
        self.ids.get(self.cursor.checked_sub(1)?).copied()
    }

    fn pop_next_id(&mut self) -> Option<u128> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        self.ids.get(self.cursor).copied()
    }

    fn path_for_id(&self, id: u128) -> PathBuf {
        self.dir.join(format!("Persistent.Mail.{id}"))
    }
}

pub(crate) async fn refresh_scans_if_needed(app: &AppHandle, state: &mut WatcherState) -> bool {
    let config_refresh_interval = state.config.config_refresh_interval;
    let dir_refresh_interval_idle = state.config.dir_refresh_interval_idle;
    let dir_refresh_interval_busy = state.config.dir_refresh_interval_busy;
    let full_dir_refresh_interval = state.config.full_dir_refresh_interval;
    let next_dirs =
        if !state.dirs.is_empty() && state.dirs_last_read.elapsed() < config_refresh_interval {
            state.dirs.clone()
        } else {
            let app_for_read = app.clone();
            let dirs =
                match tauri::async_runtime::spawn_blocking(move || crate::read_dirs(&app_for_read))
                    .await
                {
                    Ok(Ok(d)) => d,
                    Ok(Err(e)) => {
                        emit_log(app, format!("Failed to read config: {}", e));
                        return false;
                    }
                    Err(e) => {
                        emit_log(app, format!("Config read task failed: {}", e));
                        return false;
                    }
                };
            let next_dirs: Vec<PathBuf> = dirs.into_iter().map(PathBuf::from).collect();
            state.dirs = next_dirs.clone();
            state.dirs_last_read = Instant::now();
            next_dirs
        };
    let needs_rebuild = state.scans.len() != next_dirs.len()
        || state.scans.iter().zip(next_dirs.iter()).any(|(scan, dir)| &scan.dir != dir);

    if needs_rebuild {
        state.scans = next_dirs
            .into_iter()
            .map(|dir| DirScan::new(dir, dir_refresh_interval_busy, full_dir_refresh_interval))
            .collect();
    }

    let active_dirs = state.dirs.clone();
    let pruned = state.prune_removed_dirs_state(&active_dirs);
    if pruned > 0 {
        emit_log(
            app,
            format!(
                "Dropped {} stale watcher item(s) for directories that are no longer watched.",
                pruned
            ),
        );
    }

    let mut did_refresh = false;
    let mut refresh_count = 0u64;
    for scan in &mut state.scans {
        if !scan.is_stale(dir_refresh_interval_idle, dir_refresh_interval_busy) {
            continue;
        }
        #[derive(Debug)]
        enum RefreshResult {
            Full(Vec<u128>),
            New(Vec<u128>),
        }

        // Finish a paced pass before restarting it, otherwise large caches can starve old IDs.
        let do_full =
            scan.cursor == 0 && scan.last_full_refresh.elapsed() >= full_dir_refresh_interval;
        let dir = scan.dir.clone();
        let max_id = scan.max_id;
        let ids = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<RefreshResult> {
            let read_dir = fs::read_dir(&dir)
                .with_context(|| format!("Failed to read directory {:?}", dir))?;

            let mut ids = Vec::new();
            for entry in read_dir.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if !file_type.is_file() {
                    continue;
                }
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                let Some(id) = parse_rok_mail_id(&name) else {
                    continue;
                };

                if !do_full
                    && let Some(max) = max_id
                    && id <= max
                {
                    continue;
                }
                ids.push(id);
            }

            ids.sort_unstable();
            ids.dedup();

            if do_full || max_id.is_none() {
                Ok(RefreshResult::Full(ids))
            } else {
                Ok(RefreshResult::New(ids))
            }
        })
        .await;

        match ids {
            Ok(Ok(RefreshResult::Full(ids))) => {
                // Revisit every signature with the existing per-tick budget. Native notifications
                // can be dropped; retaining only new IDs makes old changed/deleted files invisible.
                scan.replace_ids(ids);

                scan.last_refresh = Instant::now();
                scan.last_full_refresh = Instant::now();
                did_refresh = true;
                refresh_count += 1;
            }
            Ok(Ok(RefreshResult::New(ids))) => {
                for id in ids {
                    if scan.known_ids.insert(id) {
                        scan.ids.insert(scan.cursor, id);
                        scan.cursor = scan.cursor.saturating_add(1);
                        scan.max_id = Some(scan.max_id.map_or(id, |m| m.max(id)));
                    }
                }
                scan.last_refresh = Instant::now();
                did_refresh = true;
                refresh_count += 1;
            }
            Ok(Err(e)) => {
                scan.last_refresh = Instant::now();
                emit_log(app, format!("Directory scan failed for {:?}: {}", scan.dir, e));
            }
            Err(e) => {
                scan.last_refresh = Instant::now();
                emit_log(app, format!("Directory scan task failed for {:?}: {}", scan.dir, e));
            }
        }
    }

    state.scan_refreshes = state.scan_refreshes.saturating_add(refresh_count);

    did_refresh
}

pub(crate) fn next_file(state: &mut WatcherState) -> Option<QueuedUpload> {
    for _ in 0..state.config.scan_budget_per_tick {
        let (scan, _best_id) = state
            .scans
            .iter_mut()
            .filter_map(|scan| scan.peek_next_id().map(|id| (scan, id)))
            .max_by_key(|(_, id)| *id)?;

        let id = scan.pop_next_id()?;
        let path = scan.path_for_id(id);
        let key = path.to_string_lossy().to_string();

        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let sig = match file_sig(&meta) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if state.store.entries.get(&key) == Some(&sig) || state.upload_queued_paths.contains(&key) {
            continue;
        }

        // Do not mark discovery as processed: an uncheckpointed upload must survive a restart.
        return Some(QueuedUpload {
            path: path.to_string_lossy().to_string(),
            sig,
            attempts: 0,
            not_before_ms: None,
        });
    }

    None
}

pub(crate) fn sync_fs_watches(
    app: &AppHandle,
    watcher: Option<&mut RecommendedWatcher>,
    watched_dirs: &mut FxHashSet<PathBuf>,
    desired_dirs: &[PathBuf],
) {
    let Some(watcher) = watcher else {
        return;
    };

    let desired: FxHashSet<PathBuf> = desired_dirs.iter().cloned().collect();

    for dir in watched_dirs.difference(&desired).cloned().collect::<Vec<_>>() {
        if let Err(e) = watcher.unwatch(&dir) {
            emit_log(app, format!("Failed to unwatch {:?}: {}", dir, e));
        }
        watched_dirs.remove(&dir);
    }

    for dir in desired.difference(watched_dirs).cloned().collect::<Vec<_>>() {
        if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
            emit_log(app, format!("Failed to watch {:?}: {}", dir, e));
            continue;
        }
        watched_dirs.insert(dir);
    }
}

pub(crate) fn apply_fs_event(state: &mut WatcherState, path: PathBuf, now_ms: u128) {
    if !state.dirs.iter().any(|dir| path.parent() == Some(dir.as_path())) {
        return;
    }

    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return;
    };
    let Some(id) = parse_rok_mail_id(name) else {
        return;
    };

    let meta = match fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => return,
    };
    if !meta.is_file() {
        return;
    }
    let sig = match file_sig(&meta) {
        Ok(s) => s,
        Err(_) => return,
    };

    let key = path.to_string_lossy().to_string();
    if let Some(existing) = state.store.entries.get(&key)
        && *existing == sig
    {
        return;
    }
    state.enqueue_upload(QueuedUpload {
        path: key,
        sig,
        attempts: 0,
        not_before_ms: Some(now_ms.saturating_add(state.config.file_retry_delay_ms)),
    });

    if let Some(parent) = path.parent()
        && let Some(scan) = state.scans.iter_mut().find(|s| s.dir == parent)
    {
        let _ = scan.known_ids.insert(id);
        scan.max_id = Some(scan.max_id.map_or(id, |m| m.max(id)));
    }
}

#[cfg(test)]
mod tests {
    use super::{super::store::*, *};

    fn state_for(dir: &std::path::Path) -> WatcherState {
        let mut state = WatcherState::new(
            super::super::WatcherConfig::default(),
            ProcessedStore::default(),
            UploadQueueStore::default(),
        );
        state.dirs = vec![dir.to_path_buf()];
        state.scans = vec![DirScan::new(dir.to_path_buf(), Duration::ZERO, Duration::ZERO)];
        state
    }

    fn scan(state: &mut WatcherState) -> &mut DirScan {
        state.scans.first_mut().expect("one configured scan")
    }

    #[test]
    fn discovery_is_recoverable_before_queue_checkpoint() {
        let dir = tempfile::tempdir().expect("temp directory");
        let path = dir.path().join("Persistent.Mail.1");
        fs::write(&path, b"report").expect("write mail");
        let mut state = state_for(dir.path());
        scan(&mut state).replace_ids(vec![1]);
        let item = next_file(&mut state).expect("discover mail");
        state.enqueue_upload(item);
        assert!(state.store.entries.is_empty(), "discovery is not completion");

        // Model a crash before either checkpoint was flushed.
        let mut restarted = state_for(dir.path());
        scan(&mut restarted).replace_ids(vec![1]);
        assert!(next_file(&mut restarted).is_some());
    }

    #[test]
    fn full_scan_recovers_old_changed_file_and_skips_unchanged_versions() {
        let dir = tempfile::tempdir().expect("temp directory");
        let path = dir.path().join("Persistent.Mail.1");
        fs::write(&path, b"old").expect("write old mail");
        let mut state = state_for(dir.path());
        scan(&mut state).replace_ids(vec![1]);
        let item = next_file(&mut state).expect("discover mail");
        state.mark_processed(&item);
        scan(&mut state).replace_ids(vec![1]);
        assert!(next_file(&mut state).is_none());

        // An old ID outside the former 5,000-ID validation tail is still revisited.
        fs::write(&path, b"updated report").expect("change mail without notification");
        scan(&mut state).replace_ids((1..=6000).collect());
        let mut found = None;
        while scan(&mut state).cursor > 0 && found.is_none() {
            found = next_file(&mut state);
        }
        let updated = found.expect("full scan must recover missed event");
        assert_eq!(updated.path, item.path);
        assert_ne!(updated.sig, item.sig);
    }

    #[test]
    fn full_scan_removes_deleted_ids_and_rediscovers_recreated_files() {
        let dir = tempfile::tempdir().expect("temp directory");
        let mut state = state_for(dir.path());
        scan(&mut state).replace_ids(vec![1, 2, 3]);
        scan(&mut state).replace_ids(vec![2]);
        assert_eq!(scan(&mut state).ids, vec![2]);
        assert!(!scan(&mut state).known_ids.contains(&1));
        scan(&mut state).replace_ids(vec![1, 2]);
        assert_eq!(scan(&mut state).pop_next_id(), Some(2));
        assert_eq!(scan(&mut state).pop_next_id(), Some(1));
    }

    #[test]
    fn event_arriving_during_upload_preserves_new_version() {
        let dir = tempfile::tempdir().expect("temp directory");
        let path = dir.path().join("Persistent.Mail.1");
        fs::write(&path, b"old").expect("write mail");
        let mut state = state_for(dir.path());
        apply_fs_event(&mut state, path.clone(), 0);
        let in_flight = state.pop_ready_upload(u128::MAX).expect("start upload");
        fs::write(&path, b"new report").expect("update during upload");

        // The event channel is drained after the upload completes, then another version is queued.
        state.mark_processed(&in_flight);
        apply_fs_event(&mut state, path, 1000);
        let updated = state.pop_ready_upload(u128::MAX).expect("new version must remain queued");
        assert_ne!(updated.sig, in_flight.sig);
        assert_eq!(state.store.entries.get(&in_flight.path), Some(&in_flight.sig));
    }

    #[test]
    fn repeated_events_coalesce_without_marking_unprocessed_version_complete() {
        let dir = tempfile::tempdir().expect("temp directory");
        let path = dir.path().join("Persistent.Mail.1");
        fs::write(&path, b"old").expect("write mail");
        let mut state = state_for(dir.path());
        apply_fs_event(&mut state, path.clone(), 0);
        fs::write(&path, b"changed").expect("update mail");
        for _ in 0..20 {
            apply_fs_event(&mut state, path.clone(), 1);
        }
        assert_eq!(state.upload_queue.len(), 1);
        assert!(state.store.entries.is_empty());
        assert_eq!(state.hot_paths.len(), 1);
    }

    #[test]
    fn removed_directory_events_are_ignored_and_readd_recovers_pending_mail() {
        let dir = tempfile::tempdir().expect("temp directory");
        let path = dir.path().join("Persistent.Mail.1");
        fs::write(&path, b"report").expect("write mail");
        let mut state = state_for(dir.path());
        for _ in 0..3 {
            apply_fs_event(&mut state, path.clone(), 0);
            assert_eq!(state.upload_queue.len(), 1);
            state.dirs.clear();
            state.prune_removed_dirs_state(&[]);
            apply_fs_event(&mut state, path.clone(), 0);
            assert!(state.upload_queue.is_empty());
            assert!(state.hot_paths.is_empty());
            state.dirs.push(dir.path().to_path_buf());
            scan(&mut state).replace_ids(vec![1]);
            assert!(next_file(&mut state).is_some());
        }
    }

    #[test]
    fn empty_scan_obeys_idle_interval() {
        let mut scan = DirScan::new(
            PathBuf::from("mailcache"),
            Duration::from_secs(60),
            Duration::from_secs(180),
        );
        assert!(scan.is_stale(Duration::from_secs(5), Duration::from_secs(60)));
        scan.last_refresh = Instant::now();
        assert!(!scan.is_stale(Duration::from_secs(5), Duration::from_secs(60)));
    }

    #[test]
    fn nonrecursive_watcher_rejects_descendant_events() {
        let dir = tempfile::tempdir().expect("temp directory");
        fs::create_dir(dir.path().join("nested")).expect("nested directory");
        let path = dir.path().join("nested/Persistent.Mail.1");
        fs::write(&path, b"report").expect("write mail");
        let mut state = state_for(dir.path());
        apply_fs_event(&mut state, path, 0);
        assert!(state.upload_queue.is_empty());
    }
}

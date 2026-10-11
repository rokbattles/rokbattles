use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use anyhow::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use rustc_hash::FxHashSet;
use tokio::sync::mpsc;

use super::{
    mail::mail_id_for_path,
    state::WatcherState,
    store::{QueuedUpload, file_sig},
};

#[derive(Debug)]
pub(crate) struct DirScan {
    pub(crate) dir: PathBuf,
    entries: Option<fs::ReadDir>,
    next_scan: Instant,
    rescan_requested: bool,
}

impl DirScan {
    pub(crate) fn new(dir: PathBuf, now: Instant) -> Self {
        Self { dir, entries: None, next_scan: now, rescan_requested: false }
    }

    fn read_batch(&mut self, now: Instant) -> (Vec<PathBuf>, Option<anyhow::Error>) {
        if self.entries.is_none() {
            match fs::read_dir(&self.dir) {
                Ok(entries) => self.entries = Some(entries),
                Err(error) => {
                    self.next_scan = now + Duration::from_secs(5);

                    return (Vec::new(), Some(error.into()));
                }
            }
        }

        let mut paths = Vec::new();
        let mut failure = None;

        // Count every directory entry, including non-mail files, against the turn's budget.
        for _ in 0..256 {
            match self.entries.as_mut().and_then(Iterator::next) {
                Some(Ok(entry)) => paths.push(entry.path()),
                Some(Err(error)) => {
                    failure.get_or_insert_with(|| error.into());
                }
                None => {
                    self.entries = None;
                    self.next_scan =
                        if self.rescan_requested { now } else { now + Duration::from_secs(5) };
                    self.rescan_requested = false;

                    break;
                }
            }
        }

        (paths, failure)
    }
}

pub(crate) fn refresh_watched_dirs(
    state: &mut WatcherState,
    now: Instant,
    read_dirs: &impl Fn() -> anyhow::Result<Option<Vec<PathBuf>>>,
    log: &impl Fn(String),
) {
    if now.duration_since(state.dirs_last_read) < Duration::from_secs(2) {
        return;
    }

    // Throttle failed refreshes too, so a broken config cannot flood the log during a backlog.
    state.dirs_last_read = now;

    let dirs = match read_dirs() {
        Ok(Some(dirs)) => dirs,
        Ok(None) => return,
        Err(error) => {
            log(format!("Failed to read config: {error}"));
            return;
        }
    };

    let removed = set_watched_dirs(state, dirs, now);

    if removed > 0 {
        log(format!("Cancelled {removed} source(s) in directories no longer watched."));
    }
}

pub(crate) fn set_watched_dirs(
    state: &mut WatcherState,
    mut dirs: Vec<PathBuf>,
    now: Instant,
) -> usize {
    let mut seen = FxHashSet::default();
    dirs.retain(|dir| seen.insert(dir.clone()));

    // Preserve cursors for retained directories so settings refreshes cannot restart a backlog.
    state.scans.retain(|scan| seen.contains(&scan.dir));

    for dir in &dirs {
        if !state.scans.iter().any(|scan| scan.dir == *dir) {
            state.scans.push_back(DirScan::new(dir.clone(), now));
        }
    }

    let removed = state.prune_removed_dirs_state(&dirs);
    state.dirs = dirs;

    removed
}

pub(crate) fn request_reconciliation(state: &mut WatcherState, now: Instant) {
    for scan in &mut state.scans {
        if scan.entries.is_some() {
            // Finish the current pass first. Repeated overflow must not restart at the first file.
            scan.rescan_requested = true;
        } else {
            scan.next_scan = now;
        }
    }
}

pub(crate) async fn reconcile_next_directory(
    state: &mut WatcherState,
    now: Instant,
    now_ms: u128,
) -> anyhow::Result<bool> {
    let mut selected = None;

    for _ in 0..state.scans.len() {
        let Some(scan) = state.scans.pop_front() else {
            break;
        };

        if scan.entries.is_some() || now >= scan.next_scan {
            selected = Some(scan);
            break;
        }

        state.scans.push_back(scan);
    }

    let Some(mut scan) = selected else {
        return Ok(false);
    };
    let dir = scan.dir.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let batch = scan.read_batch(now);

        (scan, batch)
    })
    .await;

    let (scan, (paths, failure)) = match result {
        Ok(result) => result,
        Err(error) => {
            state.scans.push_back(DirScan::new(dir.clone(), now + Duration::from_secs(5)));

            return Err(error).with_context(|| format!("Directory scan task failed for {dir:?}"));
        }
    };
    state.scans.push_back(scan);

    for path in paths {
        discover_source(state, path, now_ms);
    }

    if let Some(error) = failure {
        return Err(error).with_context(|| format!("Directory scan failed for {dir:?}"));
    }

    Ok(true)
}

pub(crate) fn queue_fs_event(
    sender: &mpsc::Sender<PathBuf>,
    rescan: &AtomicBool,
    event: notify::Result<notify::Event>,
) {
    let event = match event {
        Ok(event) => event,
        Err(_) => {
            rescan.store(true, Ordering::Relaxed);
            return;
        }
    };

    if event.need_rescan() {
        rescan.store(true, Ordering::Relaxed);
    }

    for path in event.paths {
        if sender.try_send(path).is_err() {
            rescan.store(true, Ordering::Relaxed);
            break;
        }
    }
}

pub(crate) fn discover_source(state: &mut WatcherState, path: PathBuf, now_ms: u128) {
    if !path.parent().is_some_and(|parent| state.dirs.iter().any(|dir| dir == parent)) {
        return;
    }

    let Some(key) = path.to_str() else {
        return;
    };

    if mail_id_for_path(key).is_none() || state.is_acknowledged(key) || state.is_pending(key) {
        return;
    }

    // Events and reconciliation both accept only immediate regular files in watched directories.
    let sig = match fs::symlink_metadata(&path) {
        Ok(meta) if !meta.is_file() => return,
        Ok(meta) => file_sig(&meta).ok(),
        Err(_) => None,
    };

    // A temporary metadata failure is not evidence that rejected bytes changed.
    if sig.is_none() && state.uploads.rejected.contains_key(key) {
        return;
    }

    state.enqueue_upload(QueuedUpload {
        path: key.to_string(),
        sig,
        attempts: 0,
        last_error: None,
        not_before_ms: Some(now_ms.saturating_add(750)),
    });
}

pub(crate) fn sync_fs_watches(
    log: &impl Fn(String),
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
            log(format!("Failed to unwatch {:?}: {}", dir, e));
        }
        watched_dirs.remove(&dir);
    }

    for dir in desired.difference(watched_dirs).cloned().collect::<Vec<_>>() {
        if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
            log(format!("Failed to watch {:?}: {}", dir, e));
            continue;
        }
        watched_dirs.insert(dir);
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, time::UNIX_EPOCH};

    use serde_json::json;

    use super::*;
    use crate::watcher::{
        apply_upload_result, process_upload,
        store::{StateFile, UploadStore},
        upload::{
            UploadStatus,
            tests::{client, respond_once, response},
        },
    };

    fn write_mail(dir: &Path, id: &str, bytes: &[u8]) -> PathBuf {
        fs::create_dir_all(dir).expect("mail directory");
        let path = dir.join(format!("Persistent.Mail.{id}"));
        fs::write(&path, bytes).expect("mail file");
        fs::File::options()
            .write(true)
            .open(&path)
            .expect("mail file")
            .set_modified(UNIX_EPOCH + Duration::from_secs(1))
            .expect("stable timestamp");

        path
    }

    fn new_state(dirs: Vec<PathBuf>, now: Instant) -> WatcherState {
        let mut state = WatcherState::new(UploadStore::default());
        set_watched_dirs(&mut state, dirs, now);

        state
    }

    // Fake ingress acknowledges each attempted ID through the production outcome handler.
    fn acknowledge_ready(state: &mut WatcherState, now_ms: u128) -> FxHashSet<String> {
        let mut attempted = FxHashSet::default();

        while let Some(item) = state.next_ready_upload(now_ms) {
            let id = mail_id_for_path(&item.path).expect("mail ID").to_string();
            let name = format!("Persistent.Mail.{id}");
            attempted.insert(id);
            apply_upload_result(state, item, &name, Ok(UploadStatus::Stored), now_ms);
        }

        attempted
    }

    #[tokio::test]
    async fn ready_upload_reaches_ingress_while_a_directory_pass_is_still_open() {
        let dir = tempfile::tempdir().expect("mail directory");

        for id in 1..=300 {
            let bytes = rokbattles_mail_codec::encode(&json!({"id": id, "type": "Battle"}))
                .expect("mail bytes");

            write_mail(dir.path(), &id.to_string(), &bytes);
        }

        let now = Instant::now();
        let mut state = new_state(vec![dir.path().to_path_buf()], now);
        reconcile_next_directory(&mut state, now, 10_000).await.expect("first batch");

        assert!(state.scans.front().expect("directory scan").entries.is_some());
        assert_eq!(state.uploads.pending.len(), 256);

        let item = state.next_ready_upload(11_000).expect("ready before scan finishes");
        let id = mail_id_for_path(&item.path).expect("mail ID").to_string();
        let body = json!({"status": "stored", "mail_id": id}).to_string();
        let (url, server) = respond_once(response("201 Created", &body)).await;

        process_upload(&mut state, &client(), &url, item, 11_000).await;
        server.await.expect("upload proceeds between scan batches");

        assert!(state.uploads.acknowledged.contains(&id));
        assert!(state.scans.front().expect("directory scan").entries.is_some());

        reconcile_next_directory(&mut state, now, 11_000).await.expect("remaining batch");

        assert_eq!(state.uploads.pending.len(), 299);
        assert!(state.scans.front().expect("completed pass").entries.is_none());
    }

    #[tokio::test]
    async fn periodic_reconciliation_finds_missed_events_and_lower_ids_arriving_late() {
        let dir = tempfile::tempdir().expect("mail directory");
        write_mail(dir.path(), "900", b"first mail");
        let start = Instant::now();
        let mut state = new_state(vec![dir.path().to_path_buf()], start);

        reconcile_next_directory(&mut state, start, 1000).await.expect("initial pass");

        assert_eq!(acknowledge_ready(&mut state, 2000), FxHashSet::from_iter(["900".to_string()]));

        write_mail(dir.path(), "1", b"lower ID without event");
        write_mail(dir.path(), "001", b"different exact ingress ID");
        let later = start + Duration::from_secs(5);
        reconcile_next_directory(&mut state, later, 6000).await.expect("periodic pass");

        assert_eq!(
            acknowledge_ready(&mut state, 7000),
            FxHashSet::from_iter(["1".to_string(), "001".to_string()])
        );
        assert!(state.uploads.pending.is_empty());
    }

    #[tokio::test]
    async fn event_discovered_rejection_recovers_after_restart_without_another_event() {
        let dir = tempfile::tempdir().expect("fixture directory");
        let mail_dir = dir.path().join("mail");
        let path = write_mail(&mail_dir, "123", b"partially written");
        let start = Instant::now();
        let mut state = new_state(vec![mail_dir.clone()], start);
        let state_file = StateFile::open(dir.path()).expect("state file");
        state_file.load().expect("initial snapshot");

        discover_source(&mut state, path.clone(), 1000);
        let partial = state.next_ready_upload(2000).expect("event discovered upload");
        process_upload(&mut state, &client(), "http://127.0.0.1:0", partial, 3000).await;

        assert_eq!(state.uploads.rejected.len(), 1, "stable incomplete bytes were rejected");

        state.flush(&state_file).expect("persist rejection");
        drop(state);
        drop(state_file);

        let state_file = StateFile::open(dir.path()).expect("restart");
        let mut state = WatcherState::new(state_file.load().expect("load"));
        set_watched_dirs(&mut state, vec![mail_dir.clone()], start);
        reconcile_next_directory(&mut state, start, 3000).await.expect("unchanged rejected mail");

        assert!(state.next_ready_upload(4000).is_none(), "unchanged invalid bytes must not loop");
        assert_eq!(state.uploads.rejected.len(), 1);

        let valid = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"}))
            .expect("complete mail");
        write_mail(&mail_dir, "123", &valid);
        reconcile_next_directory(&mut state, start + Duration::from_secs(5), 8000)
            .await
            .expect("missed completion event");
        let complete = state.next_ready_upload(9000).expect("changed rejected source recovered");
        let (url, server) =
            respond_once(response("201 Created", r#"{"status":"stored","mail_id":"123"}"#)).await;

        process_upload(&mut state, &client(), &url, complete, 9000).await;
        server.await.expect("mail reached local ingress");

        assert!(state.uploads.acknowledged.contains("123"));
        assert!(state.uploads.pending.is_empty());
        assert!(state.uploads.rejected.is_empty());
    }

    #[tokio::test]
    async fn large_acknowledged_history_cannot_hide_changed_rejected_work() {
        let dir = tempfile::tempdir().expect("mail directory");
        let mut uploads = UploadStore::default();

        for id in 1..=5100 {
            write_mail(dir.path(), &id.to_string(), b"historical mail");
            uploads.acknowledged.insert(id.to_string());
        }

        let start = Instant::now();
        let mut state = WatcherState::new(uploads);
        set_watched_dirs(&mut state, vec![dir.path().to_path_buf()], start);
        let path = write_mail(dir.path(), "99999", b"partial");
        discover_source(&mut state, path.clone(), 1000);
        let incomplete = state.next_ready_upload(2000).expect("event-discovered input");
        state.reject_upload(incomplete, "decode failed".to_string());

        // A complete pass must reconsider this changed source, regardless of iteration order.
        write_mail(dir.path(), "99999", b"now complete and available");
        let mut turns = 0;

        while reconcile_next_directory(&mut state, start, 3000).await.expect("reconcile history") {
            turns += 1;
            assert!(turns <= 21, "a pass must complete after its bounded batches");
        }

        assert_eq!(
            acknowledge_ready(&mut state, 4000),
            FxHashSet::from_iter(["99999".to_string()])
        );
        assert_eq!(state.uploads.acknowledged.len(), 5101);
        assert!(state.scans.iter().all(|scan| scan.entries.is_none()));
    }

    #[tokio::test]
    async fn missing_source_and_temporarily_unreadable_directory_recover() {
        let root = tempfile::tempdir().expect("fixture directory");
        let mail_dir = root.path().join("temporarily-missing");
        let start = Instant::now();
        let mut state = new_state(vec![mail_dir.clone()], start);

        reconcile_next_directory(&mut state, start, 1000).await.expect_err("directory unavailable");
        discover_source(&mut state, mail_dir.join("Persistent.Mail.123"), 1000);
        let missing =
            state.next_ready_upload(2000).expect("source remains pending despite metadata failure");
        process_upload(&mut state, &client(), "http://127.0.0.1:0", missing, 2000).await;

        let valid = rokbattles_mail_codec::encode(&json!({"id":"123", "type":"Battle"}))
            .expect("complete mail");
        write_mail(&mail_dir, "123", &valid);
        write_mail(&mail_dir, "5", b"additional report without event");
        reconcile_next_directory(&mut state, start + Duration::from_secs(5), 6000)
            .await
            .expect("directory restored");
        let restored = state.next_ready_upload(6000).expect("missing source retried");
        process_upload(&mut state, &client(), "http://127.0.0.1:0", restored, 6000).await;

        assert!(state.uploads.rejected.is_empty());
        assert_eq!(
            acknowledge_ready(&mut state, 7000),
            FxHashSet::from_iter(["123".to_string(), "5".to_string()])
        );
    }

    #[tokio::test]
    async fn changed_or_moved_acknowledged_mail_never_becomes_pending_again() {
        let root = tempfile::tempdir().expect("fixture directory");
        let first = root.path().join("first");
        let second = root.path().join("second");
        let path = write_mail(&first, "123", b"first copy");
        write_mail(&second, "123", b"duplicate copy");
        let start = Instant::now();
        let mut state = new_state(vec![first.clone(), second.clone()], start);

        reconcile_next_directory(&mut state, start, 1000).await.expect("first directory");
        reconcile_next_directory(&mut state, start, 1000).await.expect("second directory");

        assert_eq!(state.uploads.pending.len(), 2);
        assert_eq!(acknowledge_ready(&mut state, 2000).len(), 1);

        fs::remove_file(path).expect("original copy removed");
        let changed = write_mail(&second, "123", b"changed contents after acknowledgement");
        discover_source(&mut state, changed, 3000);
        let later = start + Duration::from_secs(5);
        reconcile_next_directory(&mut state, later, 6000).await.expect("first directory again");
        reconcile_next_directory(&mut state, later, 6000).await.expect("second directory again");

        assert!(acknowledge_ready(&mut state, 7000).is_empty());
        assert_eq!(state.uploads.acknowledged.len(), 1);
        assert!(state.uploads.rejected.is_empty());
    }

    #[tokio::test]
    async fn overflow_and_duplicate_out_of_order_events_converge_with_clean_reconciliation() {
        let dir = tempfile::tempdir().expect("mail directory");
        let paths: Vec<_> =
            (1..=4).map(|id| write_mail(dir.path(), &id.to_string(), b"mail")).collect();
        let start = Instant::now();
        let mut events = new_state(vec![dir.path().to_path_buf()], start);
        let mut clean = new_state(vec![dir.path().to_path_buf()], start);
        let (sender, mut receiver) = mpsc::channel(1);
        let rescan = AtomicBool::new(false);

        for path in paths.iter().rev().chain(paths.iter()) {
            queue_fs_event(
                &sender,
                &rescan,
                Ok(notify::Event::new(notify::EventKind::Any).add_path(path.clone())),
            );
        }

        assert!(rescan.swap(false, Ordering::Relaxed), "overflow must request reconciliation");

        discover_source(&mut events, receiver.try_recv().expect("one delivered hint"), 1000);
        request_reconciliation(&mut events, start);
        reconcile_next_directory(&mut events, start, 1000).await.expect("repair missed hints");
        reconcile_next_directory(&mut clean, start, 1000).await.expect("clean reconciliation");

        assert_eq!(acknowledge_ready(&mut events, 2000), acknowledge_ready(&mut clean, 2000));
        assert_eq!(events.uploads.acknowledged.len(), 4);
        assert_eq!(events.uploads.acknowledged, clean.uploads.acknowledged);
        assert!(events.uploads.pending.is_empty());
    }

    #[tokio::test]
    async fn continuous_rescan_requests_and_events_do_not_starve_backlogs_or_other_directories() {
        let root = tempfile::tempdir().expect("fixture directory");
        let busy = root.path().join("busy");
        let quiet = root.path().join("quiet");
        let event_path = write_mail(&busy, "1", b"mail");

        for id in 2..=600 {
            write_mail(&busy, &id.to_string(), b"mail");
        }

        write_mail(&quiet, "900", b"quiet directory mail");
        let start = Instant::now();
        let mut state = new_state(vec![busy.clone(), quiet.clone()], start);
        let mut attempted = FxHashSet::default();

        for turn in 0..8 {
            for _ in 0..512 {
                discover_source(&mut state, event_path.clone(), 1000);
            }

            request_reconciliation(&mut state, start);
            set_watched_dirs(&mut state, vec![busy.clone(), quiet.clone()], start);
            reconcile_next_directory(&mut state, start, 1000).await.expect("bounded scan turn");
            attempted.extend(acknowledge_ready(&mut state, 2000));

            if turn == 1 {
                assert!(attempted.contains("900"), "quiet directory must get its own turn");
                assert!(
                    attempted.len() <= 258,
                    "at most one event hint, 256 busy entries, and one quiet entry"
                );
            }
        }

        assert_eq!(attempted.len(), 601, "rescan requests cannot restart the busy prefix forever");
    }

    #[tokio::test]
    async fn removed_directories_drop_open_scans_and_ignore_stale_events() {
        let root = tempfile::tempdir().expect("fixture directory");
        let removed = root.path().join("removed");
        let retained = root.path().join("retained");

        for id in 1..=300 {
            write_mail(&removed, &id.to_string(), b"mail");
        }

        let path = write_mail(&retained, "900", b"retained mail");
        let start = Instant::now();
        let mut state = new_state(vec![removed.clone(), retained.clone()], start);
        reconcile_next_directory(&mut state, start, 1000)
            .await
            .expect("open removed directory cursor");

        assert!(state.scans.iter().any(|scan| scan.dir == removed && scan.entries.is_some()));

        set_watched_dirs(&mut state, vec![retained.clone()], start);
        discover_source(&mut state, removed.join("Persistent.Mail.299"), 1000);
        reconcile_next_directory(&mut state, start, 1000)
            .await
            .expect("retained directory progresses");

        assert_eq!(state.scans.len(), 1);
        assert_eq!(state.scans.front().expect("retained scan").dir, retained);
        assert_eq!(state.uploads.pending.len(), 1);
        assert!(state.is_pending(path.to_str().expect("source path")));
        assert_eq!(acknowledge_ready(&mut state, 2000), FxHashSet::from_iter(["900".to_string()]));
    }

    #[tokio::test]
    async fn deleted_files_leave_no_transient_history_after_the_pass_finishes() {
        let dir = tempfile::tempdir().expect("mail directory");
        let path = write_mail(dir.path(), "123", b"mail");
        let start = Instant::now();
        let mut state = new_state(vec![dir.path().to_path_buf()], start);
        reconcile_next_directory(&mut state, start, 1000).await.expect("discover source");
        fs::remove_file(&path).expect("source deleted before upload");

        reconcile_next_directory(&mut state, start + Duration::from_secs(5), 6000)
            .await
            .expect("empty pass");

        assert!(state.scans.iter().all(|scan| scan.entries.is_none()));
        assert_eq!(
            state.uploads.pending.len(),
            1,
            "durable source must survive temporary deletion"
        );
        assert!(state.is_pending(path.to_str().expect("path")));

        set_watched_dirs(&mut state, Vec::new(), start);

        assert!(state.scans.is_empty());
        assert!(!state.is_pending(path.to_str().expect("path")));
    }

    #[tokio::test]
    async fn discovery_ignores_nested_files_directories_and_symlinks_consistently() {
        let root = tempfile::tempdir().expect("fixture directory");
        let watched = root.path().join("watched");
        let nested = write_mail(&watched.join("nested"), "123", b"nested mail");
        fs::create_dir(watched.join("Persistent.Mail.2")).expect("directory with mail-like name");
        let start = Instant::now();
        let mut state = new_state(vec![watched.clone()], start);

        discover_source(&mut state, nested, 1000);
        discover_source(&mut state, watched.join("Persistent.Mail.2"), 1000);

        #[cfg(unix)]
        {
            let outside = write_mail(root.path(), "3", b"outside watched directory");
            let link = watched.join("Persistent.Mail.3");
            std::os::unix::fs::symlink(outside, &link).expect("symlink fixture");
            discover_source(&mut state, link, 1000);
        }

        reconcile_next_directory(&mut state, start, 1000)
            .await
            .expect("same rules during reconciliation");

        assert!(state.uploads.pending.is_empty());
        assert!(state.uploads.acknowledged.is_empty());
    }

    #[test]
    fn broken_settings_preserve_running_and_recovered_pending_uploads() {
        use crate::app_config::ConfigStore;

        for running in [false, true] {
            for failure in ["empty", "truncated", "malformed", "unreadable", "missing"] {
                let root = tempfile::tempdir().expect("fixture directory");
                let mail_dir = root.path().join("mail");
                let source = write_mail(&mail_dir, "123", b"pending mail");
                let now = Instant::now();
                let state_file = StateFile::open(root.path()).expect("upload state");
                state_file.load().expect("initial upload state");
                let mut state = new_state(vec![mail_dir.clone()], now);
                discover_source(&mut state, source.clone(), 1000);
                state.flush(&state_file).expect("pending upload persisted before failure");

                let path = root.path().join("config.json");
                let store = ConfigStore::new(path.clone());
                let read_dirs = || {
                    store.read(|_| {}).map(|config| {
                        config.dirs.map(|dirs| dirs.into_iter().map(PathBuf::from).collect())
                    })
                };

                if running {
                    store
                        .update(|config| {
                            config.dirs = Some(vec![mail_dir.to_string_lossy().into_owned()]);
                        })
                        .expect("valid running settings");
                    refresh_watched_dirs(&mut state, now, &read_dirs, &|_| {});
                    fs::remove_file(&path).expect("inject settings failure");
                } else {
                    state = WatcherState::new(state_file.load().expect("recover pending upload"));
                }

                match failure {
                    "empty" => fs::write(&path, b"").expect("empty settings"),
                    "truncated" => fs::write(&path, br#"{"dirs":["#).expect("truncated settings"),
                    "malformed" => fs::write(&path, br#"{"dirs":42}"#).expect("malformed settings"),
                    "unreadable" => fs::create_dir(&path).expect("unreadable settings"),
                    "missing" => {}
                    value => panic!("unexpected test case: {value}"),
                }

                refresh_watched_dirs(&mut state, now + Duration::from_secs(2), &read_dirs, &|_| {});
                state.flush(&state_file).expect("persist after failed refresh");
                let recovered = state_file.load().expect("pending work on disk");

                assert_eq!(state.uploads.pending.len(), 1, "{failure}, running={running}");
                assert_eq!(recovered.pending.len(), 1);
                assert_eq!(recovered.pending[0].path, source.to_string_lossy());
                assert_eq!(state.dirs, if running { vec![mail_dir] } else { Vec::new() });
            }
        }
    }

    #[test]
    fn first_run_preference_save_keeps_pending_work_until_an_explicit_empty_directory_choice() {
        use crate::app_config::ConfigStore;

        let root = tempfile::tempdir().expect("fixture directory");
        let mail_dir = root.path().join("mail");
        let source = write_mail(&mail_dir, "123", b"pending mail");
        let now = Instant::now();
        let state_file = StateFile::open(root.path()).expect("upload state");
        state_file.load().expect("initial upload state");
        let mut state = new_state(vec![mail_dir], now);
        discover_source(&mut state, source, 1000);
        state.flush(&state_file).expect("pending work");
        state = WatcherState::new(state_file.load().expect("recover pending work"));
        let path = root.path().join("config.json");
        let store = ConfigStore::new(path.clone());
        let read_dirs = || {
            store
                .read(|_| {})
                .map(|config| config.dirs.map(|dirs| dirs.into_iter().map(PathBuf::from).collect()))
        };

        let now = Instant::now();
        refresh_watched_dirs(&mut state, now, &read_dirs, &|_| {});

        assert_eq!(state.uploads.pending.len(), 1, "missing settings do not cancel recovery");

        store.update(|config| config.auto_update = false).expect("save first preference");
        let restarted_store = ConfigStore::new(path);
        let read_after_restart = || {
            restarted_store
                .read(|_| {})
                .map(|config| config.dirs.map(|dirs| dirs.into_iter().map(PathBuf::from).collect()))
        };
        refresh_watched_dirs(
            &mut state,
            now + Duration::from_secs(2),
            &read_after_restart,
            &|_| {},
        );

        assert_eq!(
            state.uploads.pending.len(),
            1,
            "preference-only saves must not cancel recovery"
        );

        store
            .update(|config| config.dirs = Some(Vec::new()))
            .expect("deliberate directory removal");
        let logs = std::cell::RefCell::new(Vec::new());
        refresh_watched_dirs(
            &mut state,
            now + Duration::from_secs(4),
            &read_after_restart,
            &|line| logs.borrow_mut().push(line),
        );
        state.flush(&state_file).expect("save deliberate cancellation");

        assert!(state.uploads.pending.is_empty());
        assert!(state_file.load().expect("durable cancellation").pending.is_empty());
        assert!(logs.borrow().iter().any(|line| line.contains("Cancelled 1 source")));
    }

    #[test]
    fn failed_config_refreshes_are_throttled_during_busy_scanning() {
        let mut state = WatcherState::new(UploadStore::default());
        let start = Instant::now();
        let attempts = std::cell::Cell::new(0);
        let read_dirs = || {
            attempts.set(attempts.get() + 1);
            anyhow::bail!("temporarily unreadable config");
        };

        refresh_watched_dirs(&mut state, start, &read_dirs, &|_| {});
        refresh_watched_dirs(&mut state, start + Duration::from_millis(1), &read_dirs, &|_| {});
        refresh_watched_dirs(&mut state, start + Duration::from_secs(2), &read_dirs, &|_| {});

        assert_eq!(attempts.get(), 2);
    }
}

//! SQLite upload history, retry recovery, and exclusive watcher ownership.
//!
//! The first SQLite startup creates an empty `local.db`; legacy JSON history is
//! ignored. The watcher loads recovery state once, then commits changed records
//! before each upload and on shutdown. WAL mode with FULL synchronization makes
//! these commits durable without rewriting the complete history for each attempt.

use std::{
    collections::VecDeque,
    fs, io,
    path::Path,
    time::{Duration, SystemTime},
};

use anyhow::{Context, ensure};
use rusqlite::{Connection, params};
use rustc_hash::{FxHashMap, FxHashSet};
use tauri::{AppHandle, Manager};

use super::mail::mail_id_for_path;

mod migrations;

/// Recovery state loaded at startup and updated in memory by the watcher.
#[derive(Debug, Default)]
pub(crate) struct UploadStore {
    /// Exact IDs, preserving leading zeroes and covering every local copy.
    pub(crate) acknowledged: FxHashSet<String>,
    pub(crate) pending: VecDeque<QueuedUpload>,
    pub(crate) rejected: FxHashMap<String, RejectedUpload>,
    pub(crate) api_backoff_until_ms: Option<u128>,
}

#[derive(Debug, Clone)]
pub(crate) struct QueuedUpload {
    pub(crate) path: String,
    pub(crate) sig: Option<FileSig>,
    /// Retry count capped at 10, used to calculate exponential backoff.
    pub(crate) attempts: u32,
    pub(crate) not_before_ms: Option<u128>,
}

#[derive(Debug)]
pub(crate) struct RejectedUpload {
    /// A different signature allows discovery to reconsider this source.
    pub(crate) sig: Option<FileSig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSig {
    pub(crate) size: u64,
    /// Milliseconds since the Unix epoch, used to detect changes and wait for stable files.
    pub(crate) modified: u128,
}

/// A pending database change. The watcher retains these in order until a commit succeeds.
pub(crate) enum StoreChange {
    Pending(QueuedUpload),
    Rejected { path: String, sig: Option<FileSig> },
    Remove(String),
    Acknowledge(String),
    Backoff(Option<u128>),
}

/// A synchronous database connection held with the exclusive watcher lock.
pub(crate) struct UploadDb {
    // Drop the connection before releasing ownership of the watcher.
    connection: Connection,
    _lock: fs::File,
}

impl UploadDb {
    pub(crate) fn for_app(app: &AppHandle) -> anyhow::Result<Self> {
        let dir = app.path().app_config_dir().context("Could not resolve app config directory")?;

        Self::open(&dir)
    }

    pub(crate) fn open(dir: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(dir).context("Failed to create app config directory")?;

        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("upload-state.lock"))
            .context("Failed to open upload state lock")?;
        lock.try_lock().context("Upload state is already in use by another watcher")?;

        let mut connection = Connection::open(dir.join("local.db"))
            .context("Failed to open local upload database")?;
        initialize(&mut connection)?;

        Ok(Self { connection, _lock: lock })
    }

    pub(crate) fn load(&self) -> anyhow::Result<UploadStore> {
        load_store(&self.connection)
    }

    /// Applies changes atomically in their original order. An empty batch performs no I/O.
    ///
    /// On failure, the caller must retain the batch for retry or stop the watcher.
    pub(crate) fn commit(&self, changes: &[StoreChange]) -> anyhow::Result<()> {
        if changes.is_empty() {
            return Ok(());
        }

        // Each synchronous operation finishes its transaction before returning, so
        // transactions cannot overlap even though these methods borrow `&self`.
        let transaction = self.connection.unchecked_transaction()?;

        for change in changes {
            apply_change(&transaction, change)?;
        }

        transaction.commit().context("Failed to commit upload state")
    }

    /// Clears valid recovery state while retaining the database schema and version.
    pub(crate) fn reset(&self) -> anyhow::Result<()> {
        // Preserve unreadable recovery data instead of silently discarding it.
        self.load()?;

        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute_batch(
            "DELETE FROM sources;
             DELETE FROM acknowledged;
             UPDATE upload_control SET api_backoff_until_ms = NULL WHERE id = 1;",
        )?;

        transaction.commit().context("Failed to reset upload history")
    }

    #[cfg(test)]
    pub(crate) fn read_for_test(dir: &Path) -> anyhow::Result<UploadStore> {
        let connection = Connection::open_with_flags(
            dir.join("local.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        load_store(&connection)
    }
}

fn initialize(connection: &mut Connection) -> anyhow::Result<()> {
    // Schema upgrades need the same durability as upload acknowledgements.
    connection.pragma_update(None, "synchronous", "FULL")?;
    migrations::apply(connection)?;

    // Check compatibility before changing the journal mode of an existing database.
    let mode: String =
        connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    ensure!(mode.eq_ignore_ascii_case("wal"), "Upload database requires WAL mode");

    Ok(())
}

fn sqlite_integer(value: u128) -> anyhow::Result<i64> {
    i64::try_from(value).context("Upload state integer exceeds SQLite range")
}

fn apply_change(connection: &Connection, change: &StoreChange) -> anyhow::Result<()> {
    match change {
        StoreChange::Pending(item) => {
            put_source(
                connection,
                &item.path,
                item.sig.as_ref(),
                true,
                item.attempts,
                item.not_before_ms,
            )?;
        }
        StoreChange::Rejected { path, sig } => {
            put_source(connection, path, sig.as_ref(), false, 0, None)?;
        }
        StoreChange::Remove(path) => {
            connection.prepare_cached("DELETE FROM sources WHERE path = ?1")?.execute([path])?;
        }
        StoreChange::Acknowledge(id) => {
            ensure!(valid_mail_id(id), "Invalid acknowledged mail ID");

            connection
                .prepare_cached("INSERT OR IGNORE INTO acknowledged(mail_id) VALUES (?1)")?
                .execute([id])?;
            connection.prepare_cached("DELETE FROM sources WHERE mail_id = ?1")?.execute([id])?;
        }
        StoreChange::Backoff(until) => {
            connection
                .prepare_cached("UPDATE upload_control SET api_backoff_until_ms = ?1 WHERE id = 1")?
                .execute([until.map(sqlite_integer).transpose()?])?;
        }
    }

    Ok(())
}

fn put_source(
    connection: &Connection,
    path: &str,
    sig: Option<&FileSig>,
    pending: bool,
    attempts: u32,
    not_before: Option<u128>,
) -> anyhow::Result<()> {
    let id = mail_id_for_path(path);
    let size = sig.map(|sig| sqlite_integer(u128::from(sig.size))).transpose()?;
    let modified = sig.map(|sig| sqlite_integer(sig.modified)).transpose()?;
    let not_before = not_before.map(sqlite_integer).transpose()?;

    connection
        .prepare_cached(
            "INSERT INTO sources(path, mail_id, pending, size, modified_ms, attempts, not_before_ms)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7
             WHERE NOT EXISTS (SELECT 1 FROM acknowledged WHERE mail_id = ?2)
             ON CONFLICT(path) DO UPDATE SET
                mail_id = excluded.mail_id,
                pending = excluded.pending,
                size = excluded.size,
                modified_ms = excluded.modified_ms,
                attempts = excluded.attempts,
                not_before_ms = excluded.not_before_ms",
        )?
        .execute(params![path, id, pending, size, modified, attempts, not_before])?;

    Ok(())
}

fn valid_mail_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) && id.parse::<u128>().is_ok()
}

fn read_unsigned(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Option<u64>> {
    row.get::<_, Option<i64>>(index)?.map(u64::try_from).transpose().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn load_store(connection: &Connection) -> anyhow::Result<UploadStore> {
    // Read all tables from one snapshot so acknowledgements and their sources agree.
    let transaction = connection.unchecked_transaction()?;
    let mut store = UploadStore::default();

    {
        let mut statement = transaction.prepare("SELECT mail_id FROM acknowledged")?;

        for id in statement.query_map([], |row| row.get::<_, String>(0))? {
            let id = id?;
            ensure!(valid_mail_id(&id), "Invalid acknowledged mail ID; database retained");

            store.acknowledged.insert(id);
        }
    }

    {
        let mut statement = transaction.prepare(
            "SELECT path, mail_id, pending, size, modified_ms, attempts, not_before_ms
             FROM sources ORDER BY rowid",
        )?;
        let mut rows = statement.query([])?;

        while let Some(row) = rows.next()? {
            let path: String = row.get(0)?;
            let id: Option<String> = row.get(1)?;

            ensure!(
                id.as_deref() == mail_id_for_path(&path),
                "Invalid source mail ID; database retained"
            );
            ensure!(
                !id.as_ref().is_some_and(|id| store.acknowledged.contains(id)),
                "Source is already acknowledged; database retained"
            );

            let sig = match (read_unsigned(row, 3)?, read_unsigned(row, 4)?) {
                (Some(size), Some(modified)) => {
                    Some(FileSig { size, modified: u128::from(modified) })
                }
                (None, None) => None,
                _ => anyhow::bail!("Incomplete source signature; database retained"),
            };

            if row.get::<_, bool>(2)? {
                store.pending.push_back(QueuedUpload {
                    path,
                    sig,
                    attempts: row.get(5)?,
                    not_before_ms: read_unsigned(row, 6)?.map(u128::from),
                });
            } else {
                store.rejected.insert(path, RejectedUpload { sig });
            }
        }
    }

    store.api_backoff_until_ms = transaction
        .query_row("SELECT api_backoff_until_ms FROM upload_control WHERE id = 1", [], |row| {
            read_unsigned(row, 0)
        })?
        .map(u128::from);
    transaction.commit()?;

    Ok(store)
}

pub(crate) fn file_sig(meta: &fs::Metadata) -> anyhow::Result<FileSig> {
    let modified = meta
        .modified()
        .context("Could not read file modification time")?
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis();

    Ok(FileSig { size: meta.len(), modified })
}

/// Confirms a missing source only when its parent directory can still be read.
pub(super) fn source_is_deleted(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }

    if !fs::symlink_metadata(path).is_err_and(|error| error.kind() == io::ErrorKind::NotFound) {
        return false;
    }

    // A missing drive or inaccessible directory does not establish deletion.
    path.parent().is_some_and(|parent| {
        fs::read_dir(parent)
            .is_ok_and(|mut entries| entries.next().is_none_or(|entry| entry.is_ok()))
    })
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn pending(path: &str) -> QueuedUpload {
        QueuedUpload {
            path: path.to_string(),
            sig: Some(FileSig { size: 100, modified: 200 }),
            attempts: 2,
            not_before_ms: Some(4000),
        }
    }

    #[test]
    fn fresh_database_ignores_all_legacy_history_including_corrupt_json() {
        let dir = tempfile::tempdir().expect("state directory");
        let legacy =
            ["processed-v3.json", "processed-v3.tmp", "upload-queue.json", "upload-state-v1.json"];

        for name in legacy {
            fs::write(dir.path().join(name), b"not imported, even if corrupt")
                .expect("legacy file");
        }

        let db = UploadDb::open(dir.path()).expect("fresh database");
        let store = db.load().expect("fresh history");

        assert!(store.acknowledged.is_empty());
        assert!(store.pending.is_empty());
        assert!(store.rejected.is_empty());
        assert!(store.api_backoff_until_ms.is_none());

        for name in legacy {
            assert_eq!(
                fs::read(dir.path().join(name)).expect("legacy file retained"),
                b"not imported, even if corrupt"
            );
        }
    }

    #[test]
    fn transaction_round_trips_only_the_state_needed_for_recovery() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");

        db.commit(&[
            StoreChange::Pending(pending("mail/Persistent.Mail.001")),
            StoreChange::Rejected {
                path: "mail/Persistent.Mail.002".into(),
                sig: Some(FileSig { size: 3, modified: 4 }),
            },
            StoreChange::Acknowledge("003".into()),
            StoreChange::Backoff(Some(9000)),
        ])
        .expect("commit records");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let store = db.load().expect("recover records");
        let item = store.pending.front().expect("pending source");

        assert_eq!(item.path, "mail/Persistent.Mail.001");
        assert_eq!(item.sig, Some(FileSig { size: 100, modified: 200 }));
        assert_eq!(item.attempts, 2);
        assert_eq!(item.not_before_ms, Some(4000));
        assert_eq!(
            store.rejected["mail/Persistent.Mail.002"].sig,
            Some(FileSig { size: 3, modified: 4 })
        );
        assert!(store.acknowledged.contains("003"));
        assert!(!store.acknowledged.contains("3"));
        assert_eq!(store.api_backoff_until_ms, Some(9000));
        assert!(!dir.path().join("upload-state-v1.json").exists());
    }

    #[test]
    fn acknowledgement_removes_all_copies_and_preserves_distinct_leading_zero_ids() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");

        db.commit(&[
            StoreChange::Pending(pending("first/Persistent.Mail.1")),
            StoreChange::Rejected { path: "second/Persistent.Mail.1".into(), sig: None },
            StoreChange::Pending(pending("first/Persistent.Mail.01")),
            StoreChange::Acknowledge("1".into()),
            StoreChange::Pending(pending("late/Persistent.Mail.1")),
        ])
        .expect("acknowledge all exact copies");
        let store = db.load().expect("acknowledged state");

        assert_eq!(store.pending.len(), 1);
        assert_eq!(store.pending.front().expect("distinct ID").path, "first/Persistent.Mail.01");
        assert!(store.rejected.is_empty());
    }

    #[test]
    fn failed_transaction_rolls_back_all_earlier_changes() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[StoreChange::Pending(pending("Persistent.Mail.1"))])
            .expect("original pending source");

        let mut invalid = pending("Persistent.Mail.2");
        invalid.not_before_ms = Some(u128::MAX);

        assert!(
            db.commit(&[StoreChange::Acknowledge("1".into()), StoreChange::Pending(invalid)])
                .is_err()
        );

        let store = db.load().expect("previous committed state");

        assert!(store.acknowledged.is_empty());
        assert_eq!(store.pending.front().expect("original source").path, "Persistent.Mail.1");
    }

    #[test]
    fn retry_update_writes_bounded_pages_with_large_history_and_idle_commits_write_nothing() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        let history: Vec<_> =
            (1_000_000..1_050_000).map(|id| StoreChange::Acknowledge(id.to_string())).collect();
        db.commit(&history).expect("large acknowledged history");

        let mut item = pending("mail/Persistent.Mail.1");
        db.commit(&[StoreChange::Pending(item.clone())]).expect("pending source");
        db.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .expect("baseline checkpoint");
        let database_bytes =
            fs::metadata(dir.path().join("local.db")).expect("database size").len();

        item.attempts += 1;
        item.not_before_ms = Some(8000);
        db.commit(&[StoreChange::Pending(item)]).expect("single retry update");

        let wal = dir.path().join("local.db-wal");
        let written = fs::metadata(&wal).expect("changed pages").len();

        assert!(written > 0);
        assert!(written < 64 * 1024, "single retry wrote {written} bytes");
        assert!(
            written * 32 < database_bytes,
            "retry must not rewrite {database_bytes} bytes of history"
        );

        let before = fs::metadata(&wal).expect("before idle");

        for _ in 0..100 {
            db.commit(&[]).expect("idle");
        }

        let after = fs::metadata(&wal).expect("after idle");

        assert_eq!(before.len(), after.len());
        assert_eq!(
            before.modified().expect("before mtime"),
            after.modified().expect("after mtime")
        );
    }

    #[test]
    fn reset_clears_history_and_retries_without_reimporting_legacy_files() {
        let dir = tempfile::tempdir().expect("state directory");
        fs::write(dir.path().join("processed-v3.json"), b"legacy remains ignored").expect("legacy");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[
            StoreChange::Pending(pending("Persistent.Mail.1")),
            StoreChange::Rejected { path: "Persistent.Mail.2".into(), sig: None },
            StoreChange::Acknowledge("3".into()),
            StoreChange::Backoff(Some(1234)),
        ])
        .expect("history");

        db.reset().expect("reprocess");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let store = db.load().expect("cleared history");

        assert!(
            store.pending.is_empty() && store.rejected.is_empty() && store.acknowledged.is_empty()
        );
        assert!(store.api_backoff_until_ms.is_none());
    }

    #[test]
    fn failed_reset_preserves_all_recovery_state() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[
            StoreChange::Pending(pending("Persistent.Mail.1")),
            StoreChange::Rejected { path: "Persistent.Mail.2".into(), sig: None },
            StoreChange::Acknowledge("3".into()),
            StoreChange::Backoff(Some(9000)),
        ])
        .expect("recovery state");
        db.connection
            .execute_batch(
                "CREATE TRIGGER fail_reset BEFORE DELETE ON acknowledged
                 BEGIN
                    SELECT RAISE(ABORT, 'injected reset failure');
                 END;",
            )
            .expect("fail after sources have been deleted");

        db.reset().expect_err("reset must roll back");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart after failed reset");
        let store = db.load().expect("recovery state retained");
        let pending = store.pending.front().expect("pending source retained");

        assert_eq!(pending.path, "Persistent.Mail.1");
        assert_eq!(pending.attempts, 2);
        assert_eq!(pending.not_before_ms, Some(4000));
        assert!(store.rejected.contains_key("Persistent.Mail.2"));
        assert!(store.acknowledged.contains("3"));
        assert_eq!(store.api_backoff_until_ms, Some(9000));
    }

    #[test]
    fn corrupt_database_and_unsupported_schema_are_preserved() {
        let dir = tempfile::tempdir().expect("state directory");
        let path = dir.path().join("local.db");
        fs::write(&path, b"not a SQLite database").expect("corrupt database");

        assert!(UploadDb::open(dir.path()).is_err());
        assert_eq!(fs::read(&path).expect("corrupt file retained"), b"not a SQLite database");

        fs::remove_file(&path).expect("new test fixture");
        let connection = Connection::open(&path).expect("newer database");
        connection.pragma_update(None, "user_version", i32::MAX).expect("newer schema");
        drop(connection);
        let before = fs::read(&path).expect("original database");

        assert!(UploadDb::open(dir.path()).is_err());
        assert_eq!(fs::read(&path).expect("newer database retained"), before);
    }

    #[test]
    fn contradictory_state_blocks_loading_and_reset_without_erasing_recovery() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[StoreChange::Pending(pending("Persistent.Mail.1"))]).expect("pending source");

        db.connection
            .execute("INSERT INTO acknowledged VALUES ('1')", [])
            .expect("inject logical corruption");

        db.load().expect_err("contradictory recovery data must be rejected");
        assert!(db.reset().is_err());

        let count: u32 = db
            .connection
            .query_row("SELECT count(*) FROM sources", [], |row| row.get(0))
            .expect("source retained");

        assert_eq!(count, 1);
    }

    #[test]
    fn only_one_watcher_can_own_the_database() {
        let dir = tempfile::tempdir().expect("state directory");
        let first = UploadDb::open(dir.path()).expect("first watcher");

        assert!(UploadDb::open(dir.path()).is_err());

        drop(first);
        UploadDb::open(dir.path()).expect("ownership released");
    }

    #[test]
    fn committed_wal_recovers_after_process_exit_without_destructors() {
        let dir = tempfile::tempdir().expect("state directory");

        let status = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "watcher::store::tests::crash_writer", "--nocapture"])
            .env("ROKBATTLES_TEST_CRASH_DIR", dir.path())
            .status()
            .expect("crash fixture process");

        assert!(status.success());
        assert!(dir.path().join("local.db-wal").exists(), "fixture must leave uncheckpointed WAL");

        let db = UploadDb::open(dir.path()).expect("recover after abrupt exit");
        let store = db.load().expect("committed WAL recovered");

        assert!(store.acknowledged.contains("1"));
        assert_eq!(store.pending.front().expect("pending source").path, "Persistent.Mail.2");
        assert_eq!(store.api_backoff_until_ms, Some(9000));
    }

    #[test]
    fn crash_writer() {
        let Some(dir) = std::env::var_os("ROKBATTLES_TEST_CRASH_DIR") else {
            return;
        };

        let db = UploadDb::open(Path::new(&dir)).expect("child database");
        db.commit(&[
            StoreChange::Acknowledge("1".into()),
            StoreChange::Pending(pending("Persistent.Mail.2")),
            StoreChange::Backoff(Some(9000)),
        ])
        .expect("commit before abrupt exit");

        std::process::exit(0);
    }
}

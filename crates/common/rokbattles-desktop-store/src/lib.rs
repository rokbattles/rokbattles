//! Bounded transactional mailcache metadata for an unprivileged per-user agent.
//!
//! No JSON import, schema migration, mail payload or credential storage. Paths
//! and counters are plaintext, protected by the OS user boundary. Deleting rows
//! is not a promise of secure erasure from SQLite/WAL or filesystem backups.
#![forbid(unsafe_code)]

use std::{
    fs::File,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, bail};
use serde::Serialize;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

pub const DATABASE_NAME: &str = "worker.sqlite3";
pub const MAX_ROOTS: usize = 32;
pub const MAX_PATH_BYTES: usize = 4096;
pub const MAX_FILES: usize = 50_000;
pub const MAX_PATH_TOTAL: usize = 8 * 1024 * 1024;
pub const MAX_MAIL_BYTES: u64 = 25 * 1024 * 1024;
const MAX_DB_BYTES: u64 = 32 * 1024 * 1024;
// Recovery admits a whole maximum-size transaction beyond the checkpoint
// threshold. A crash must not make a legitimate WAL impossible to reopen.
const MAX_RECOVERY_BYTES: u64 = 64 * 1024 * 1024;
const CHECKPOINT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SHM_BYTES: u64 = 1024 * 1024;

const SCHEMA: &[(&str, &str)] = &[
    (
        "settings",
        "CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK(id=1), enabled INTEGER NOT NULL CHECK(enabled IN (0,1)), paused INTEGER NOT NULL CHECK(paused IN (0,1)), capture_opt_in INTEGER NOT NULL CHECK(capture_opt_in IN (0,1)), reset_epoch INTEGER NOT NULL CHECK(reset_epoch>=0), maintenance_stop INTEGER NOT NULL CHECK(maintenance_stop IN (0,1))) STRICT",
    ),
    (
        "roots",
        "CREATE TABLE roots (id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE CHECK(length(CAST(path AS BLOB)) BETWEEN 1 AND 4096)) STRICT",
    ),
    (
        "files",
        "CREATE TABLE files (path TEXT PRIMARY KEY NOT NULL CHECK(length(CAST(path AS BLOB)) BETWEEN 1 AND 4096), root_id INTEGER NOT NULL REFERENCES roots(id) ON DELETE CASCADE, size INTEGER NOT NULL CHECK(size BETWEEN 0 AND 26214400), modified_ms INTEGER NOT NULL CHECK(modified_ms>=0), state INTEGER NOT NULL CHECK(state IN (0,1,2)), attempts INTEGER NOT NULL CHECK(attempts BETWEEN 0 AND 1000000), next_attempt_ms INTEGER NOT NULL CHECK(next_attempt_ms>=0)) STRICT",
    ),
    (
        "usage",
        "CREATE TABLE usage (id INTEGER PRIMARY KEY CHECK(id=1), rows INTEGER NOT NULL CHECK(rows BETWEEN 0 AND 50000), path_bytes INTEGER NOT NULL CHECK(path_bytes BETWEEN 0 AND 8388608)) STRICT",
    ),
    (
        "status",
        "CREATE TABLE status (id INTEGER PRIMARY KEY CHECK(id=1), heartbeat_ms INTEGER NOT NULL CHECK(heartbeat_ms>=0), running INTEGER NOT NULL CHECK(running IN (0,1)), paused INTEGER NOT NULL CHECK(paused IN (0,1)), backend INTEGER NOT NULL CHECK(backend BETWEEN 0 AND 3), event INTEGER NOT NULL CHECK(event BETWEEN 0 AND 6), capture_state INTEGER NOT NULL CHECK(capture_state BETWEEN 0 AND 6), capture_backend INTEGER NOT NULL CHECK(capture_backend BETWEEN 0 AND 2)) STRICT",
    ),
    ("files_ready", "CREATE INDEX files_ready ON files(state,next_attempt_ms)"),
    (
        "files_quota",
        "CREATE TRIGGER files_quota BEFORE INSERT ON files WHEN NOT EXISTS(SELECT 1 FROM files WHERE path=NEW.path) BEGIN SELECT CASE WHEN (SELECT rows FROM usage WHERE id=1)>=50000 OR (SELECT path_bytes FROM usage WHERE id=1)+length(CAST(NEW.path AS BLOB))>8388608 THEN RAISE(ABORT,'state quota exceeded') END; END",
    ),
    (
        "files_added",
        "CREATE TRIGGER files_added AFTER INSERT ON files BEGIN UPDATE usage SET rows=rows+1,path_bytes=path_bytes+length(CAST(NEW.path AS BLOB)) WHERE id=1; END",
    ),
    (
        "files_removed",
        "CREATE TRIGGER files_removed AFTER DELETE ON files BEGIN UPDATE usage SET rows=rows-1,path_bytes=path_bytes-length(CAST(OLD.path AS BLOB)) WHERE id=1; END",
    ),
    (
        "files_path_fixed",
        "CREATE TRIGGER files_path_fixed BEFORE UPDATE OF path ON files WHEN NEW.path!=OLD.path BEGIN SELECT RAISE(ABORT,'path is immutable'); END",
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileSig {
    pub size: u64,
    pub modified_ms: u64,
}

pub struct PendingFile {
    pub root_id: i64,
    pub path: PathBuf,
    pub sig: FileSig,
    pub attempts: u32,
}

impl std::fmt::Debug for PendingFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingFile")
            .field("size", &self.sig.size)
            .field("attempts", &self.attempts)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Settings {
    pub enabled: bool,
    pub paused: bool,
    pub capture_opt_in: bool,
    pub reset_epoch: u64,
    pub maintenance_stop: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub heartbeat_ms: u64,
    pub running: bool,
    pub paused: bool,
    pub backend: u8,
    pub event: u8,
    pub capture_state: u8,
    pub capture_backend: u8,
    pub pending: u64,
    pub completed: u64,
    pub rejected: u64,
}

pub struct Root {
    pub id: i64,
    pub path: PathBuf,
}

pub struct Store {
    pool: SqlitePool,
    directory: PathBuf,
}

/// Holding this owner-only file lock is required for the sole agent writer.
pub struct AgentLease {
    _file: File,
}

impl Store {
    pub async fn open(directory: &Path) -> anyhow::Result<Self> {
        rokbattles_desktop_paths::directory(directory)?;
        let initialization = rokbattles_desktop_paths::file(directory, "schema.lock")?;
        initialization
            .try_lock()
            .map_err(|_error| anyhow::anyhow!("worker state initialization is busy"))?;
        let file = rokbattles_desktop_paths::file(directory, DATABASE_NAME)?;
        if file.metadata()?.len() > MAX_DB_BYTES {
            bail!("worker database exceeds its disk limit");
        }
        drop(file);
        check_files(directory)?;
        // SQLite's default owner on an elevated Windows token can be the
        // Administrators group. Create sidecars with the explicit user owner
        // before handing the protected directory to SQLite; never truncate.
        for name in ["worker.sqlite3-wal", "worker.sqlite3-shm"] {
            drop(rokbattles_desktop_paths::file(directory, name)?);
        }

        let options = SqliteConnectOptions::new()
            .filename(directory.join(DATABASE_NAME))
            .create_if_missing(false)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(Duration::from_secs(2))
            .pragma("trusted_schema", "OFF")
            .pragma("cache_spill", "OFF")
            .pragma("page_size", "4096")
            .pragma("max_page_count", "8192")
            .pragma("wal_autocheckpoint", "256")
            .pragma("journal_size_limit", "1048576");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            // Includes opening the worker thread/connection, not only a held
            // query. Loaded Windows hosts can exceed three seconds during a
            // cold open; retain a finite bound with room beyond busy_timeout.
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(options)
            .await?;
        let store = Self { pool, directory: directory.to_path_buf() };
        store.initialize().await?;
        store.checkpoint().await?;
        check_files(directory)?;
        drop(initialization);
        Ok(store)
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&self.pool).await?;
        let page_size: i64 = sqlx::query_scalar("PRAGMA page_size").fetch_one(&self.pool).await?;
        if page_size != 4096 {
            bail!("unsupported worker database page size");
        }
        if version == 0 {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
            )
            .fetch_one(&self.pool)
            .await?;
            if count != 0 {
                bail!("unsupported unversioned worker database");
            }
            let mut tx = self.pool.begin().await?;
            for (_, statement) in SCHEMA {
                sqlx::query(*statement).execute(&mut *tx).await?;
            }
            sqlx::query("INSERT INTO settings VALUES(1,1,0,0,0,0)").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO usage VALUES(1,0,0)").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO status VALUES(1,0,0,0,0,0,0,0)").execute(&mut *tx).await?;
            sqlx::query("PRAGMA user_version=1").execute(&mut *tx).await?;
            tx.commit().await?;
        } else if version != 1 {
            bail!("unsupported worker database schema; no automatic migration");
        }

        // Count before selecting bounded SQL so an oversized extra definition
        // cannot be hidden by the length filter.
        let objects: i64 =
            sqlx::query_scalar("SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'")
                .fetch_one(&self.pool)
                .await?;
        if objects != SCHEMA.len() as i64 {
            bail!("unexpected worker database schema");
        }
        // Reject altered tables/views/triggers before any application writes.
        let rows = sqlx::query("SELECT name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' AND length(sql)<=4096 LIMIT 32").fetch_all(&self.pool).await?;
        if rows.len() != SCHEMA.len() {
            bail!("unexpected worker database schema");
        }
        for row in rows {
            let name: String = row.try_get("name")?;
            let sql: String = row.try_get("sql")?;
            if !SCHEMA
                .iter()
                .any(|(expected_name, statement)| *expected_name == name && *statement == sql)
            {
                bail!("unexpected worker database schema");
            }
        }
        let roots: i64 =
            sqlx::query_scalar("SELECT count(*) FROM roots").fetch_one(&self.pool).await?;
        let (rows, bytes): (i64, i64) = sqlx::query_as(
            "SELECT count(*),coalesce(sum(length(CAST(path AS BLOB))),0) FROM files",
        )
        .fetch_one(&self.pool)
        .await?;
        let recorded: (i64, i64) = sqlx::query_as("SELECT rows,path_bytes FROM usage WHERE id=1")
            .fetch_one(&self.pool)
            .await?;
        if roots > MAX_ROOTS as i64
            || rows > MAX_FILES as i64
            || bytes > MAX_PATH_TOTAL as i64
            || recorded != (rows, bytes)
        {
            bail!("worker state exceeds its limits");
        }
        Ok(())
    }

    pub fn acquire_agent(&self) -> anyhow::Result<Option<AgentLease>> {
        let file = rokbattles_desktop_paths::file(&self.directory, "agent.lock")?;
        match file.try_lock() {
            Ok(()) => Ok(Some(AgentLease { _file: file })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
        }
    }

    pub async fn settings(&self) -> anyhow::Result<Settings> {
        let row = sqlx::query(
            "SELECT enabled,paused,capture_opt_in,reset_epoch,maintenance_stop FROM settings WHERE id=1",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Settings {
            enabled: boolean(row.try_get("enabled")?)?,
            paused: boolean(row.try_get("paused")?)?,
            capture_opt_in: boolean(row.try_get("capture_opt_in")?)?,
            reset_epoch: u64::try_from(row.try_get::<i64, _>("reset_epoch")?)?,
            maintenance_stop: boolean(row.try_get("maintenance_stop")?)?,
        })
    }

    pub async fn set_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE settings SET enabled=? WHERE id=1")
            .bind(enabled)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_maintenance_stop(&self, stopped: bool) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE settings SET maintenance_stop=? WHERE id=1")
            .bind(stopped)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_paused(&self, paused: bool) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE settings SET paused=? WHERE id=1")
            .bind(paused)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_capture_opt_in(&self, enabled: bool) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE settings SET capture_opt_in=? WHERE id=1")
            .bind(enabled)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn request_reprocess(&self) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE settings SET reset_epoch=reset_epoch+1 WHERE id=1 AND reset_epoch<9223372036854775807").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn acknowledge_reprocess(&self, observed_epoch: u64) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE settings SET reset_epoch=0 WHERE id=1 AND reset_epoch=?")
            .bind(i64::try_from(observed_epoch)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn roots(&self) -> anyhow::Result<Vec<Root>> {
        let rows = sqlx::query("SELECT id,path FROM roots WHERE length(CAST(path AS BLOB)) BETWEEN 1 AND 4096 ORDER BY id LIMIT 33").fetch_all(&self.pool).await?;
        if rows.len() > MAX_ROOTS {
            bail!("too many mailcache roots");
        }
        rows.into_iter()
            .map(|row| {
                let path = PathBuf::from(row.try_get::<String, _>("path")?);
                validate_path(&path)?;
                Ok(Root { id: row.try_get("id")?, path })
            })
            .collect()
    }

    /// Narrow controller write. Files for removed roots disappear transactionally;
    /// pending uploads are never moved to a newly selected directory.
    /// Metadata only: these paths are not authority to open files. The agent
    /// must canonicalize selected roots and independently no-follow/revalidate
    /// roots and candidate files before scanning or uploading.
    pub async fn set_roots(&self, paths: &[PathBuf]) -> anyhow::Result<()> {
        if paths.len() > MAX_ROOTS {
            bail!("too many mailcache roots");
        }
        let paths: Vec<&str> =
            paths.iter().map(|path| validate_path(path)).collect::<anyhow::Result<_>>()?;
        self.before_write().await?;
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query("SELECT id,path FROM roots LIMIT 33").fetch_all(&mut *tx).await?;
        for row in rows {
            let existing: String = row.try_get("path")?;
            if !paths.contains(&existing.as_str()) {
                sqlx::query("DELETE FROM roots WHERE id=?")
                    .bind(row.try_get::<i64, _>("id")?)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        for path in paths {
            sqlx::query("INSERT OR IGNORE INTO roots(path) VALUES(?)")
                .bind(path)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        self.checkpoint().await?;
        Ok(())
    }

    /// Atomically persist a new/changed signature and its pending upload together.
    pub async fn observe(&self, root: i64, path: &Path, sig: FileSig) -> anyhow::Result<bool> {
        let path = validate_path(path)?;
        validate_sig(sig)?;
        self.before_write().await?;
        let mut tx = self.pool.begin().await?;
        let root_path: Option<String> = sqlx::query_scalar(
            "SELECT path FROM roots WHERE id=? AND length(CAST(path AS BLOB))<=4096",
        )
        .bind(root)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(root_path) = root_path else {
            return Ok(false);
        };
        if Path::new(path).parent() != Some(Path::new(&root_path)) {
            bail!("mailcache file must be directly inside its selected root");
        }
        let (rows, bytes): (i64, i64) =
            sqlx::query_as("SELECT rows,path_bytes FROM usage WHERE id=1")
                .fetch_one(&mut *tx)
                .await?;
        if rows >= MAX_FILES as i64 || bytes + path.len() as i64 > MAX_PATH_TOTAL as i64 {
            // Completed/rejected history is a cache. Pending work is never
            // evicted; files remain available for the next bounded rescan.
            sqlx::query("DELETE FROM files WHERE path IN (SELECT path FROM files WHERE state!=0 ORDER BY modified_ms LIMIT 256)").execute(&mut *tx).await?;
            let (rows, bytes): (i64, i64) =
                sqlx::query_as("SELECT rows,path_bytes FROM usage WHERE id=1")
                    .fetch_one(&mut *tx)
                    .await?;
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM files WHERE path=?)")
                    .bind(path)
                    .fetch_one(&mut *tx)
                    .await?;
            if !exists
                && (rows >= MAX_FILES as i64 || bytes + path.len() as i64 > MAX_PATH_TOTAL as i64)
            {
                tx.commit().await?;
                return Ok(false);
            }
        }
        let result = sqlx::query("INSERT INTO files(path,root_id,size,modified_ms,state,attempts,next_attempt_ms) VALUES(?,?,?,?,0,0,0) ON CONFLICT(path) DO UPDATE SET root_id=excluded.root_id,size=excluded.size,modified_ms=excluded.modified_ms,state=0,attempts=0,next_attempt_ms=0 WHERE files.size!=excluded.size OR files.modified_ms!=excluded.modified_ms")
            .bind(path).bind(root).bind(i64::try_from(sig.size)?).bind(i64::try_from(sig.modified_ms)?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(result.rows_affected() != 0)
    }

    pub async fn next_pending(&self, now_ms: u64) -> anyhow::Result<Option<PendingFile>> {
        let row = sqlx::query("SELECT files.root_id,files.path,roots.path AS root_path,size,modified_ms,attempts FROM files JOIN roots ON roots.id=files.root_id WHERE state=0 AND next_attempt_ms<=? AND length(CAST(files.path AS BLOB)) BETWEEN 1 AND 4096 AND length(CAST(roots.path AS BLOB)) BETWEEN 1 AND 4096 ORDER BY next_attempt_ms,modified_ms DESC,files.path LIMIT 1")
            .bind(i64::try_from(now_ms)?).fetch_optional(&self.pool).await?;
        row.map(|row| {
            let path = PathBuf::from(row.try_get::<String, _>("path")?);
            validate_path(&path)?;
            let root_path = PathBuf::from(row.try_get::<String, _>("root_path")?);
            validate_path(&root_path)?;
            if path.parent() != Some(root_path.as_path()) {
                bail!("pending file is outside its selected root");
            }
            let sig = FileSig {
                size: u64::try_from(row.try_get::<i64, _>("size")?)?,
                modified_ms: u64::try_from(row.try_get::<i64, _>("modified_ms")?)?,
            };
            validate_sig(sig)?;
            let attempts = u32::try_from(row.try_get::<i64, _>("attempts")?)?;
            if attempts > 1_000_000 {
                bail!("invalid retry counter");
            }
            Ok(PendingFile { root_id: row.try_get("root_id")?, path, sig, attempts })
        })
        .transpose()
    }

    pub async fn finish(&self, item: &PendingFile, supported: bool) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE files SET state=? WHERE path=? AND size=? AND modified_ms=?")
            .bind(if supported { 1 } else { 2 })
            .bind(validate_path(&item.path)?)
            .bind(i64::try_from(item.sig.size)?)
            .bind(i64::try_from(item.sig.modified_ms)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn forget_missing(&self, item: &PendingFile) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query(
            "DELETE FROM files WHERE path=? AND root_id=? AND size=? AND modified_ms=? AND state=0",
        )
        .bind(validate_path(&item.path)?)
        .bind(item.root_id)
        .bind(i64::try_from(item.sig.size)?)
        .bind(i64::try_from(item.sig.modified_ms)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn retry(&self, item: &PendingFile, not_before_ms: u64) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("UPDATE files SET attempts=min(attempts+1,1000000),next_attempt_ms=? WHERE path=? AND size=? AND modified_ms=? AND state=0")
            .bind(i64::try_from(not_before_ms)?).bind(validate_path(&item.path)?).bind(i64::try_from(item.sig.size)?).bind(i64::try_from(item.sig.modified_ms)?).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn clear_history(&self) -> anyhow::Result<()> {
        self.before_write().await?;
        sqlx::query("DELETE FROM files").execute(&self.pool).await?;
        self.checkpoint().await?;
        Ok(())
    }

    pub async fn heartbeat(
        &self,
        now_ms: u64,
        running: bool,
        paused: bool,
        backend: u8,
        event: u8,
    ) -> anyhow::Result<()> {
        if backend > 3 || event > 6 {
            bail!("invalid worker status");
        }
        self.before_write().await?;
        sqlx::query(
            "UPDATE status SET heartbeat_ms=?,running=?,paused=?,backend=?,event=? WHERE id=1",
        )
        .bind(i64::try_from(now_ms)?)
        .bind(running)
        .bind(paused)
        .bind(backend)
        .bind(event)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Capture status is independent of mailcache progress and contains no flow metadata.
    pub async fn capture_status(&self, state: u8, backend: u8) -> anyhow::Result<()> {
        if state > 6 || backend > 2 {
            bail!("invalid capture status");
        }
        self.before_write().await?;
        sqlx::query("UPDATE status SET capture_state=?,capture_backend=? WHERE id=1")
            .bind(state as i64)
            .bind(backend as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn status(&self) -> anyhow::Result<Status> {
        let row = sqlx::query("SELECT heartbeat_ms,running,paused,backend,event,capture_state,capture_backend,(SELECT count(*) FROM files WHERE state=0) AS pending,(SELECT count(*) FROM files WHERE state=1) AS completed,(SELECT count(*) FROM files WHERE state=2) AS rejected FROM status WHERE id=1").fetch_one(&self.pool).await?;
        let backend = u8::try_from(row.try_get::<i64, _>("backend")?)?;
        let event = u8::try_from(row.try_get::<i64, _>("event")?)?;
        if backend > 3 || event > 6 {
            bail!("invalid worker status");
        }
        Ok(Status {
            heartbeat_ms: u64::try_from(row.try_get::<i64, _>("heartbeat_ms")?)?,
            running: boolean(row.try_get("running")?)?,
            paused: boolean(row.try_get("paused")?)?,
            backend,
            event,
            capture_state: bounded_code(row.try_get("capture_state")?, 6)?,
            capture_backend: bounded_code(row.try_get("capture_backend")?, 2)?,
            pending: u64::try_from(row.try_get::<i64, _>("pending")?)?,
            completed: u64::try_from(row.try_get::<i64, _>("completed")?)?,
            rejected: u64::try_from(row.try_get::<i64, _>("rejected")?)?,
        })
    }

    async fn before_write(&self) -> anyhow::Result<()> {
        if sidecar_size(&self.directory, "-wal")? > CHECKPOINT_BYTES {
            self.checkpoint().await?;
        }
        check_files(&self.directory)
    }

    async fn checkpoint(&self) -> anyhow::Result<()> {
        let (busy, _, _): (i64, i64, i64) =
            sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)").fetch_one(&self.pool).await?;
        if busy != 0 {
            bail!("worker state checkpoint is busy");
        }
        Ok(())
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

fn bounded_code(value: i64, maximum: u8) -> anyhow::Result<u8> {
    let value = u8::try_from(value)?;
    if value > maximum {
        bail!("invalid worker status");
    }
    Ok(value)
}

fn boolean(value: i64) -> anyhow::Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => bail!("invalid worker setting"),
    }
}

fn validate_sig(sig: FileSig) -> anyhow::Result<()> {
    if sig.size > MAX_MAIL_BYTES || sig.modified_ms > i64::MAX as u64 {
        bail!("invalid file signature");
    }
    Ok(())
}

fn validate_path(path: &Path) -> anyhow::Result<&str> {
    let text = path.to_str().context("mailcache path must be UTF-8")?;
    if !path.is_absolute()
        || text.len() > MAX_PATH_BYTES
        || text.contains('\0')
        || path.components().any(|part| matches!(part, Component::ParentDir))
    {
        bail!("invalid mailcache path");
    }
    Ok(text)
}

fn sidecar_size(directory: &Path, suffix: &str) -> anyhow::Result<u64> {
    let path = directory.join(format!("{DATABASE_NAME}{suffix}"));
    rokbattles_desktop_paths::existing_file(&path)?;
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

fn check_files(directory: &Path) -> anyhow::Result<()> {
    let db = sidecar_size(directory, "")?;
    let wal = sidecar_size(directory, "-wal")?;
    let shm = sidecar_size(directory, "-shm")?;
    let journal = sidecar_size(directory, "-journal")?;
    if db > MAX_DB_BYTES
        || wal > MAX_RECOVERY_BYTES
        || journal > MAX_RECOVERY_BYTES
        || shm > MAX_SHM_BYTES
        || db.saturating_add(wal).saturating_add(shm).saturating_add(journal)
            > MAX_DB_BYTES + MAX_RECOVERY_BYTES + MAX_SHM_BYTES
    {
        bail!("worker state disk limit reached");
    }
    Ok(())
}

#[cfg(test)]
mod tests;

//! Ordered schema upgrades for the local upload database.
//!
//! Add numbered SQL files under the crate's `migrations/` directory and append them
//! to [`MIGRATIONS`]. List position defines the schema version; never reorder or
//! edit a shipped migration. The runner owns the transaction and `user_version`,
//! so SQL files must not manage either.

use anyhow::{Context, ensure};
use rusqlite::{Connection, TransactionBehavior};

#[derive(Clone, Copy)]
struct Migration {
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    name: "0001_initial.sql",
    sql: include_str!("../../../migrations/0001_initial.sql"),
}];

/// Applies pending migrations before upload state is loaded.
///
/// The caller must hold the watcher lock throughout this operation. Unrecognized
/// or newer databases are rejected, and a failed upgrade rolls back all pending
/// schema, data, and version changes.
pub(super) fn apply(connection: &mut Connection) -> anyhow::Result<()> {
    apply_pending(connection, MIGRATIONS)
}

fn apply_pending(connection: &mut Connection, migrations: &[Migration]) -> anyhow::Result<()> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let latest = u32::try_from(migrations.len()).context("Too many upload database migrations")?;
    ensure!(version <= latest, "Unsupported upload database version {version}");

    // Opening an up-to-date database must not start a write transaction.
    if version == latest {
        return Ok(());
    }

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("Failed to begin upload database migration")?;

    if version == 0 {
        // GLOB treats the underscore in SQLite's reserved prefix literally.
        let objects: u32 = transaction.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*'",
            [],
            |row| row.get(0),
        )?;
        ensure!(objects == 0, "Unrecognized local database; original retained");
    }

    for (next_version, migration) in (1..=latest).zip(migrations) {
        if next_version <= version {
            continue;
        }

        transaction.execute_batch(migration.sql).with_context(|| {
            format!("Failed to apply upload database migration {}", migration.name)
        })?;

        transaction.pragma_update(None, "user_version", next_version).with_context(|| {
            format!("Failed to record upload database migration {}", migration.name)
        })?;
    }

    // One commit covers every outstanding migration. Any failure rolls back the
    // schema, migrated data, and version together so startup can be retried.
    transaction.commit().context("Failed to commit upload database migrations")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(connection: &Connection) -> u32 {
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version")
    }

    fn create_version_one(connection: &Connection) {
        // Pin upgrade fixtures to the schema used by the first SQLite build.
        connection
            .execute_batch(include_str!("../../../migrations/0001_initial.sql"))
            .expect("version one schema");
        connection.pragma_update(None, "user_version", 1).expect("version one");
    }

    #[test]
    fn fresh_database_creates_the_initial_schema() {
        let mut connection = Connection::open_in_memory().expect("database");

        apply_pending(&mut connection, &MIGRATIONS[..1]).expect("initial migration");

        let mut statement = connection
            .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
            .expect("table query");
        let tables = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("tables")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("table names");
        let backoff: Option<i64> = connection
            .query_row("SELECT api_backoff_until_ms FROM upload_control WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("initial control record");

        assert_eq!(version(&connection), 1);
        assert_eq!(tables, ["acknowledged", "sources", "upload_control"]);
        assert!(backoff.is_none());
    }

    #[test]
    fn up_to_date_database_is_opened_without_writes() {
        let mut connection = Connection::open_in_memory().expect("database");
        apply(&mut connection).expect("current schema");
        connection
            .execute("INSERT INTO acknowledged(mail_id) VALUES ('001')", [])
            .expect("saved mail");
        let before = connection.total_changes();

        // SQLite's query-only mode rejects write transactions, even if they change no rows.
        connection.pragma_update(None, "query_only", true).expect("read-only fixture");
        apply(&mut connection).expect("already current");

        let mail_id: String = connection
            .query_row("SELECT mail_id FROM acknowledged", [], |row| row.get(0))
            .expect("existing history");

        assert_eq!(mail_id, "001");
        assert_eq!(version(&connection) as usize, MIGRATIONS.len());
        assert_eq!(connection.total_changes(), before);
    }

    #[test]
    fn version_one_history_and_retries_survive_current_migrations() {
        let mut connection = Connection::open_in_memory().expect("database");
        create_version_one(&connection);
        connection
            .execute_batch(
                "INSERT INTO acknowledged(mail_id) VALUES ('001');
                 INSERT INTO sources VALUES ('Persistent.Mail.2', '2', 1, 10, 20, 3, 4000);
                 INSERT INTO sources VALUES ('Persistent.Mail.3', '3', 0, NULL, NULL, 0, NULL);
                 UPDATE upload_control SET api_backoff_until_ms = 9000 WHERE id = 1;",
            )
            .expect("history from the first SQLite build");

        apply(&mut connection).expect("upgrade existing history");

        let mail_id: String = connection
            .query_row("SELECT mail_id FROM acknowledged", [], |row| row.get(0))
            .expect("acknowledged mail");
        let pending: (String, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT mail_id, size, modified_ms, attempts, not_before_ms
                 FROM sources WHERE pending = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .expect("pending retry");
        let rejected: String = connection
            .query_row("SELECT mail_id FROM sources WHERE pending = 0", [], |row| row.get(0))
            .expect("rejected source");
        let backoff: i64 = connection
            .query_row("SELECT api_backoff_until_ms FROM upload_control WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("API backoff");

        assert_eq!(version(&connection) as usize, MIGRATIONS.len());
        assert_eq!(mail_id, "001");
        assert_eq!(pending, ("2".to_string(), 10, 20, 3, 4000));
        assert_eq!(rejected, "3");
        assert_eq!(backoff, 9000);
    }

    #[test]
    fn older_database_applies_multiple_upgrades_in_order_only_once() {
        let dir = tempfile::tempdir().expect("database directory");
        let path = dir.path().join("local.db");
        let mut connection = Connection::open(&path).expect("database");
        create_version_one(&connection);
        connection.execute("INSERT INTO acknowledged VALUES ('001')", []).expect("saved mail");

        let migrations = [
            MIGRATIONS[0],
            Migration {
                name: "0002_test_add_column.sql",
                sql: "ALTER TABLE acknowledged ADD COLUMN test_count INTEGER NOT NULL DEFAULT 0;",
            },
            Migration {
                name: "0003_test_update_data.sql",
                sql: "UPDATE acknowledged SET test_count = test_count + 1;
                      CREATE INDEX acknowledged_test_count ON acknowledged(test_count);",
            },
        ];

        apply_pending(&mut connection, &migrations).expect("upgrade from version one");
        drop(connection);

        let mut connection = Connection::open(&path).expect("restart");
        apply_pending(&mut connection, &migrations).expect("no repeated migrations");
        let record: (String, i64) = connection
            .query_row("SELECT mail_id, test_count FROM acknowledged", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("upgraded history");

        assert_eq!(version(&connection), 3);
        assert_eq!(record, ("001".to_string(), 1));
    }

    #[test]
    fn failed_upgrade_rolls_back_all_pending_schema_data_and_version_changes() {
        let dir = tempfile::tempdir().expect("database directory");
        let path = dir.path().join("local.db");
        let mut connection = Connection::open(&path).expect("database");
        create_version_one(&connection);
        connection.execute("INSERT INTO acknowledged VALUES ('001')", []).expect("saved mail");

        let mut migrations = [
            MIGRATIONS[0],
            Migration {
                name: "0002_test_add_column.sql",
                sql: "ALTER TABLE acknowledged ADD COLUMN test_count INTEGER NOT NULL DEFAULT 0;",
            },
            Migration {
                name: "0003_test_failure.sql",
                sql: "DELETE FROM acknowledged; INSERT INTO nonexistent VALUES (1);",
            },
        ];

        let error = apply_pending(&mut connection, &migrations).expect_err("failed migration");
        assert!(error.to_string().contains("0003_test_failure.sql"));
        drop(connection);

        let mut connection = Connection::open(&path).expect("restart after failure");
        let mail_id: String = connection
            .query_row("SELECT mail_id FROM acknowledged", [], |row| row.get(0))
            .expect("original history");

        assert_eq!(version(&connection), 1);
        assert_eq!(mail_id, "001");
        connection
            .prepare("SELECT test_count FROM acknowledged")
            .expect_err("earlier pending schema change must also roll back");

        migrations[2].sql = "UPDATE acknowledged SET test_count = test_count + 1;";
        apply_pending(&mut connection, &migrations).expect("retry the corrected upgrade");
        let count: i64 = connection
            .query_row("SELECT test_count FROM acknowledged", [], |row| row.get(0))
            .expect("upgrade preserves the saved mail");

        assert_eq!(version(&connection), 3);
        assert_eq!(count, 1);
    }

    #[test]
    fn failed_initial_migration_leaves_an_empty_database_ready_for_retry() {
        let mut connection = Connection::open_in_memory().expect("database");
        let migrations = [Migration {
            name: "0001_test_failure.sql",
            sql: "CREATE TABLE partial (id INTEGER); INSERT INTO nonexistent VALUES (1);",
        }];

        apply_pending(&mut connection, &migrations).expect_err("partial initial schema");
        let tables: i64 = connection
            .query_row("SELECT count(*) FROM sqlite_schema WHERE type = 'table'", [], |row| {
                row.get(0)
            })
            .expect("tables after rollback");

        assert_eq!(version(&connection), 0);
        assert_eq!(tables, 0);

        apply(&mut connection).expect("retry initial migration");
        assert_eq!(version(&connection) as usize, MIGRATIONS.len());
    }

    #[test]
    fn unversioned_database_with_unrecognized_schema_is_preserved() {
        for (schema, query) in [
            (
                "CREATE TABLE unrelated (value TEXT); INSERT INTO unrelated VALUES ('keep');",
                "SELECT value FROM unrelated",
            ),
            (
                "CREATE TABLE sqlitex_history (value TEXT);
                 INSERT INTO sqlitex_history VALUES ('keep');",
                "SELECT value FROM sqlitex_history",
            ),
            ("CREATE VIEW saved AS SELECT 'keep' AS value;", "SELECT value FROM saved"),
        ] {
            let mut connection = Connection::open_in_memory().expect("database");
            connection.execute_batch(schema).expect("unrecognized database");

            apply(&mut connection).expect_err("do not initialize an unrelated database");
            let value: String =
                connection.query_row(query, [], |row| row.get(0)).expect("original data retained");

            assert_eq!(version(&connection), 0);
            assert_eq!(value, "keep");
        }
    }

    #[test]
    fn newer_database_is_rejected_without_modifying_it() {
        let mut connection = Connection::open_in_memory().expect("database");
        create_version_one(&connection);
        connection.execute("INSERT INTO acknowledged VALUES ('001')", []).expect("saved mail");
        connection.pragma_update(None, "user_version", 99).expect("future schema");
        connection.pragma_update(None, "query_only", true).expect("read-only fixture");

        let error = apply(&mut connection).expect_err("cannot downgrade a newer database");
        let mail_id: String = connection
            .query_row("SELECT mail_id FROM acknowledged", [], |row| row.get(0))
            .expect("original history retained");

        assert!(error.to_string().contains("Unsupported upload database version 99"));
        assert_eq!(version(&connection), 99);
        assert_eq!(mail_id, "001");
    }
}

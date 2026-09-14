//! Versioned migrations for the local store.
//!
//! `docs/architecture/RUST_DESIGN.md`: "All persistent schema changes use
//! explicit migrations." There is no schema-creation path that is not a
//! migration, so a fresh database and an upgraded one go through identical
//! code. A fresh install that took a different route to the schema is how a
//! fresh install ends up subtly different from an upgraded one, and that
//! difference shows up as a bug on one machine and not another.
//!
//! # The mechanism
//!
//! The version lives in SQLite's own header, `PRAGMA user_version`, rather than
//! in a table. It is the only piece of state that must survive a crash in step
//! with the schema it describes, and the header is written in the same
//! transaction as the DDL — so there is no window in which the schema has moved
//! and the number has not, or the reverse. A `schema_version` table would be
//! written by the same transaction too, but it would also be a table that a
//! confused migration could drop.
//!
//! # What is refused
//!
//! A database whose version is *newer* than this build is refused, not opened.
//! `docs/architecture/PROTOCOL.md` records the same rule for the event
//! envelope, and the reason is stronger here: an unreadable record produces a
//! wrong answer about a project rather than an error message.
//!
//! A file that already has tables and reports version 0 is refused as well. It
//! is a SQLite database, but not one SURE made, and the alternative — running
//! migration 1 against it and reporting the `table records already exists`
//! error — sends the reader looking for a problem in the wrong place.

use std::fmt;

use rusqlite::Connection;

/// One schema version and the SQL that reaches it.
pub struct Migration {
    /// The `user_version` this migration produces. Versions start at 1.
    pub version: u32,
    /// A short name for a diagnostic. Not stored.
    pub name: &'static str,
    /// The DDL. Compiled into the binary rather than read from disk, for the
    /// same reason the schemas are: an installed `sure.exe` has no `sql/`
    /// directory next to it.
    pub sql: &'static str,
}

/// Every migration, in order.
///
/// Append-only. Changing a migration that has shipped would leave two machines
/// at the same `user_version` with different schemas, which is precisely the
/// state the version number exists to make impossible.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "records",
    sql: include_str!("sql/0001_records.sql"),
}];

/// The version this build migrates a database to.
pub const LATEST: u32 = MIGRATIONS[MIGRATIONS.len() - 1].version;

/// Why the schema could not be brought up to date.
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationError {
    /// The database was written by a build that knew a newer schema.
    NewerSchema {
        /// What the file reports.
        found: u32,
        /// What this build understands.
        supported: u32,
    },
    /// The file is a SQLite database, but not one SURE made.
    Foreign {
        /// The tables it already had.
        tables: Vec<String>,
    },
    /// Another process held the write lock for longer than the caller waited.
    ///
    /// Distinct from [`MigrationError::Failed`] because the fix is different:
    /// a failed migration is a bug to report, a busy one is a moment to wait
    /// out and try again. Reporting the second as the first sends the reader
    /// looking for a fault in SURE that is not there.
    Busy {
        /// What SQLite said.
        message: String,
    },
    /// A migration's SQL failed. The transaction was rolled back.
    Failed {
        /// Which migration.
        version: u32,
        /// Its name.
        name: &'static str,
        /// What SQLite said.
        message: String,
    },
    /// The version in the header could not be read or is not a version.
    Header {
        /// What went wrong.
        message: String,
    },
    /// [`MIGRATIONS`] is not `1..=LATEST` without gaps. A bug in SURE.
    OutOfOrder {
        /// The version that was expected next.
        expected: u32,
        /// The version found instead.
        found: u32,
    },
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NewerSchema { found, supported } => write!(
                f,
                "This history file was written by a newer SURE (schema {found}); this build \
                 understands up to schema {supported}.\n\n\
                 SURE stopped rather than read it. A record whose meaning has changed would be \
                 reported as though it still meant what it used to, which is a wrong answer about \
                 a project rather than a visible error.\n\n\
                 Update SURE, or point it at a different data directory."
            ),
            Self::Foreign { tables } => write!(
                f,
                "There is a SQLite database where SURE keeps its history, and it is not one SURE \
                 made — it already contains {}.\n\n\
                 SURE stopped rather than add its tables to a file it does not own.\n\n\
                 Move the file, or point SURE at a different data directory.",
                named_tables(tables)
            ),
            Self::Busy { message } => write!(
                f,
                "Another SURE process was updating the history file and did not finish before \
                 SURE gave up waiting.\n\n\
                 {message}\n\n\
                 Nothing was written, and the file is unchanged. SURE stopped rather than force \
                 its way in: a second process writing the same schema at the same time is how \
                 one of them ends up reporting a migration that was already done as a failure.\n\n\
                 Run the command again."
            ),
            Self::Failed {
                version,
                name,
                message,
            } => write!(
                f,
                "SURE could not update its history file to schema {version} (\"{name}\").\n\n\
                 {message}\n\n\
                 The change was rolled back, so the file is still at the schema it was before, \
                 with the records it had. SURE stopped rather than carry on with a file whose \
                 schema and version disagree: that file would be skipped by this migration on the \
                 next run, and the missing table would surface much later as a read that returns \
                 nothing."
            ),
            Self::Header { message } => write!(
                f,
                "SURE could not read the schema version of its history file.\n\n\
                 {message}\n\n\
                 SURE stopped rather than guess, because guessing which migrations to run is how \
                 a file ends up with a schema and a version that disagree."
            ),
            Self::OutOfOrder { expected, found } => write!(
                f,
                "SURE's own migration list is wrong: it expected version {expected} and found \
                 {found}.\n\n\
                 This is a bug in SURE, not a problem with your machine.\n\n\
                 SURE stopped rather than apply migrations out of order."
            ),
        }
    }
}

impl std::error::Error for MigrationError {}

/// Up to eight tables, quoted, with a count if there are more.
///
/// Bounded because this is a message a person reads, and a directory full of
/// someone else's tables is not made clearer by listing all of them.
fn named_tables(tables: &[String]) -> String {
    use std::fmt::Write as _;

    let shown = tables.len().min(8);
    let mut text = String::new();
    for (index, name) in tables.iter().take(shown).enumerate() {
        if index > 0 {
            text.push_str(", ");
        }
        let _ = write!(text, "\"{name}\"");
    }
    if tables.len() > shown {
        let _ = write!(text, " and {} more", tables.len() - shown);
    }
    text
}

/// The schema version of an open database.
///
/// # Errors
///
/// Returns [`MigrationError::Header`] if the pragma cannot be read, and the same
/// if it holds a value that is not a usable version.
pub fn version(connection: &Connection) -> Result<u32, MigrationError> {
    // `PRAGMA user_version` is a signed 32-bit field, so a negative value means
    // the file was written by something that is not SQLite or has been damaged.
    // Converting through `u32::try_from` makes that a reported condition rather
    // than a wrap-around to a very large version.
    let raw: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| MigrationError::Header {
            message: error.to_string(),
        })?;
    u32::try_from(raw).map_err(|_| MigrationError::Header {
        message: format!("the schema version in the file header is {raw}, which is not a version"),
    })
}

/// Bring a database up to [`LATEST`], one transaction per migration.
///
/// Idempotent: a database already at [`LATEST`] is left alone and nothing is
/// written. Safe to run from several processes at once — see [`apply_one`].
///
/// # Errors
///
/// [`MigrationError::NewerSchema`] for a newer file, [`MigrationError::Foreign`]
/// for a SQLite database SURE did not make, [`MigrationError::OutOfOrder`] if
/// [`MIGRATIONS`] has a gap, [`MigrationError::Busy`] if another process held
/// the write lock for longer than the connection's busy timeout, and
/// [`MigrationError::Failed`] if a migration's SQL failed — in which case that
/// migration was rolled back and the database is left at the version it had.
pub fn apply(connection: &Connection) -> Result<u32, MigrationError> {
    check_order()?;
    let current = version(connection)?;

    if current > LATEST {
        return Err(MigrationError::NewerSchema {
            found: current,
            supported: LATEST,
        });
    }

    if current == 0 {
        let tables = user_tables(connection)?;
        if !tables.is_empty() {
            return Err(MigrationError::Foreign { tables });
        }
    }

    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        apply_one(connection, migration)?;
    }

    // Read the version rather than returning `LATEST`. Another process may have
    // moved the file while this one was working — to a newer schema, if it was
    // a newer SURE. Reporting what the file says keeps this function's promise,
    // which is the version the database is at and not the version this build
    // intended to reach.
    let final_version = version(connection)?;
    if final_version > LATEST {
        return Err(MigrationError::NewerSchema {
            found: final_version,
            supported: LATEST,
        });
    }
    Ok(final_version)
}

/// Run one migration and move the version with it, or neither.
///
/// # Two processes, one fresh database
///
/// The version is read again *inside* the transaction, after the write lock is
/// held. Reading it only beforehand is not enough, and the way that fails is
/// worth spelling out: two processes both open a fresh file, both see version 0,
/// both decide to run migration 1. The second one then waits for the write lock,
/// gets it after the first has committed, and runs `CREATE TABLE records` against
/// a database that now has one. The migration that should have been a no-op
/// reports `table records already exists`, and SURE tells the user its own
/// history file is broken.
///
/// Re-reading inside the transaction makes the second process see version 1 and
/// do nothing, which is what "already migrated" means.
fn apply_one(connection: &Connection, migration: &Migration) -> Result<(), MigrationError> {
    // `BEGIN IMMEDIATE` takes the write lock now rather than at the first
    // write, so two processes migrating at once queue here instead of one
    // discovering halfway through that it cannot commit.
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(|error| failed(migration, &error))?;

    // Every way out of this match either commits or rolls back. Returning with
    // the transaction still open would leave the connection unusable for the
    // caller and hold the write lock until the process exited.
    let outcome: Result<(), MigrationError> = match version(connection) {
        Err(error) => Err(error),
        Ok(current) if current >= migration.version => Ok(()),
        Ok(_) => connection
            .execute_batch(migration.sql)
            .and_then(|()| {
                // Not `execute`: this is a pragma assignment, and it has to
                // happen inside the same transaction as the DDL above. The
                // header and the schema then move together or not at all.
                connection.pragma_update(None, "user_version", migration.version)
            })
            .map_err(|error| failed(migration, &error)),
    };

    match outcome {
        Ok(()) => connection
            .execute_batch("COMMIT")
            .map_err(|error| failed(migration, &error)),
        Err(error) => {
            // The rollback's own failure is ignored on purpose: the caller needs
            // the reason the migration failed, and a rollback that also failed
            // still leaves the connection unusable either way.
            let _ = connection.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn failed(migration: &Migration, error: &rusqlite::Error) -> MigrationError {
    // Contention is reported as contention. A migration that could not take the
    // write lock is not a migration that is wrong, and the two lead a reader to
    // different places: retrying, versus looking for a bug in SURE.
    if is_busy(error) {
        return MigrationError::Busy {
            message: error.to_string(),
        };
    }
    MigrationError::Failed {
        version: migration.version,
        name: migration.name,
        message: error.to_string(),
    }
}

/// Whether SQLite refused for want of a lock rather than for any other reason.
fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    )
}

/// The tables of a database that is not SURE's.
///
/// `sqlite_%` is excluded because those are SQLite's own bookkeeping tables,
/// and their presence says nothing about who made the file.
fn user_tables(connection: &Connection) -> Result<Vec<String>, MigrationError> {
    let mut statement = connection
        .prepare(
            "SELECT name FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(|error| MigrationError::Header {
            message: error.to_string(),
        })?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| MigrationError::Header {
            message: error.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| MigrationError::Header {
            message: error.to_string(),
        })
}

/// Assert that [`MIGRATIONS`] is `1..=LATEST` with no gaps and no repeats.
fn check_order() -> Result<(), MigrationError> {
    for (expected, migration) in (1..).zip(MIGRATIONS) {
        if migration.version != expected {
            return Err(MigrationError::OutOfOrder {
                expected,
                found: migration.version,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn memory() -> Connection {
        Connection::open_in_memory().expect("an in-memory database")
    }

    #[test]
    fn the_migration_list_is_contiguous_from_one() {
        // The list is what `apply` walks, so a gap is not a missing feature —
        // it is every later migration running against the wrong schema.
        assert_eq!(check_order(), Ok(()));
        assert_eq!(LATEST, MIGRATIONS.len() as u32);
    }

    #[test]
    fn every_migration_has_a_name_and_some_sql() {
        for migration in MIGRATIONS {
            assert!(
                !migration.name.is_empty(),
                "{} has no name",
                migration.version
            );
            assert!(
                !migration.sql.trim().is_empty(),
                "{} has no SQL",
                migration.version
            );
        }
    }

    #[test]
    fn a_fresh_database_is_brought_to_the_latest_version() {
        let connection = memory();
        assert_eq!(version(&connection).unwrap(), 0, "a new file is at 0");
        assert_eq!(apply(&connection).unwrap(), LATEST);
        assert_eq!(version(&connection).unwrap(), LATEST);
    }

    #[test]
    fn migrating_twice_changes_nothing() {
        // Idempotence is what makes it safe for every short-lived process to
        // open the store, rather than only the first one after an upgrade.
        let connection = memory();
        apply(&connection).unwrap();
        let before: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(apply(&connection).unwrap(), LATEST);

        let after: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, after, "a second migration added something");
    }

    #[test]
    fn the_tables_the_migration_promises_are_the_tables_it_creates() {
        // Not a spelling check: the read and write paths name these columns, so
        // a migration that created something else would fail at the first
        // query rather than here.
        let connection = memory();
        apply(&connection).unwrap();
        let columns = {
            let mut statement = connection
                .prepare("SELECT name FROM pragma_table_info('records')")
                .unwrap();
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        for expected in [
            "id",
            "kind",
            "document_version",
            "written_at_ms",
            "project_root",
            "project_fingerprint",
            "document",
        ] {
            assert!(
                columns.contains(&expected.to_owned()),
                "no column {expected}"
            );
        }
    }

    #[test]
    fn the_records_table_is_strict_so_a_wrong_type_is_an_error() {
        // The property `STRICT` buys: without it SQLite stores the integer 7 in
        // a TEXT column and hands it back as "7", so a bug in the write path
        // produces a plausible value instead of a failure.
        let connection = memory();
        apply(&connection).unwrap();
        let wrong = connection.execute(
            "INSERT INTO records (kind, document_version, written_at_ms, document) \
             VALUES ('finding', 'one', 0, '{}')",
            [],
        );
        assert!(wrong.is_err(), "a text version was accepted");
    }

    #[test]
    fn a_database_from_a_newer_build_is_refused_rather_than_opened() {
        let connection = memory();
        connection
            .pragma_update(None, "user_version", LATEST + 1)
            .unwrap();
        assert_eq!(
            apply(&connection),
            Err(MigrationError::NewerSchema {
                found: LATEST + 1,
                supported: LATEST,
            })
        );
    }

    #[test]
    fn a_database_that_is_not_sures_is_refused_rather_than_migrated_into() {
        // Without this check the first migration fails with "table records
        // already exists" — or, worse, succeeds against someone else's schema.
        let connection = memory();
        connection
            .execute_batch("CREATE TABLE notes (body TEXT)")
            .unwrap();
        assert_eq!(
            apply(&connection),
            Err(MigrationError::Foreign {
                tables: vec!["notes".to_owned()],
            })
        );
    }

    #[test]
    fn sqlites_own_tables_do_not_make_a_database_look_foreign() {
        // `sqlite_sequence` appears in any database that has used
        // `AUTOINCREMENT`, SURE's included. Counting it would refuse SURE's own
        // file the moment it had been written to.
        let connection = memory();
        connection
            .execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT)")
            .unwrap();
        connection
            .execute_batch("INSERT INTO t DEFAULT VALUES")
            .unwrap();
        assert_eq!(user_tables(&connection).unwrap(), vec!["t".to_owned()]);
    }

    #[test]
    fn a_migration_that_fails_leaves_the_version_and_the_schema_alone() {
        // The property that makes a half-applied migration impossible. This is
        // the one failure a user cannot recover from by re-running, because the
        // file would look already-migrated.
        let connection = memory();
        apply(&connection).unwrap();
        let before = version(&connection).unwrap();

        let broken = Migration {
            version: before + 1,
            name: "broken",
            // The first statement succeeds and the second does not, so a
            // migration applied without a transaction would leave the table.
            sql: "CREATE TABLE half_applied (x INTEGER); SELECT this_is_not_sql;",
        };
        let error = apply_one(&connection, &broken).unwrap_err();
        match error {
            MigrationError::Failed { version, name, .. } => {
                assert_eq!(version, before + 1);
                assert_eq!(name, "broken");
            }
            other => panic!("expected a failure, got {other:?}"),
        }

        assert_eq!(
            version(&connection).unwrap(),
            before,
            "the version moved even though the migration failed"
        );
        assert!(
            !user_tables(&connection)
                .unwrap()
                .contains(&"half_applied".to_owned())
        );
    }

    #[test]
    fn a_migration_another_process_already_ran_is_not_run_a_second_time() {
        // The race two short-lived processes hit when they open a fresh file at
        // once: both read version 0 and both decide to run migration 1. This is
        // the second one, arriving after the first has committed. Without the
        // version re-read inside the transaction, `CREATE TABLE records` runs
        // against a database that already has one and SURE reports its own
        // history file as broken.
        let connection = memory();
        apply(&connection).unwrap();
        assert_eq!(apply_one(&connection, &MIGRATIONS[0]), Ok(()));
        assert_eq!(version(&connection).unwrap(), LATEST);
    }

    #[test]
    fn a_migration_that_ran_leaves_the_records_where_they_were() {
        // The re-read must skip the migration, not merely tolerate re-running
        // it. A migration rewritten to be `CREATE TABLE IF NOT EXISTS` would
        // pass the test above while quietly doing something different from what
        // the shipped migration does, and a later one — an `ALTER TABLE` — could
        // not be written that way at all.
        let connection = memory();
        apply(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO records (kind, document_version, written_at_ms, document) \
                 VALUES ('recording', 1, 0, '{}')",
                [],
            )
            .unwrap();

        assert_eq!(apply_one(&connection, &MIGRATIONS[0]), Ok(()));
        let rows: i64 = connection
            .query_row("SELECT count(*) FROM records", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1, "the record did not survive the second migration");
    }

    #[test]
    fn a_version_that_is_not_a_version_is_reported_rather_than_wrapped() {
        // `user_version` is 32-bit signed. A negative value read as `u32` would
        // become a very large version, which reports as "written by a newer
        // SURE" — a true-sounding answer to the wrong question.
        let connection = memory();
        connection
            .execute_batch("PRAGMA user_version = -1")
            .unwrap();
        match version(&connection) {
            Err(MigrationError::Header { message }) => assert!(message.contains("-1"), "{message}"),
            other => panic!("expected a header error, got {other:?}"),
        }
    }

    #[test]
    fn every_refusal_says_what_sure_did_instead() {
        // The same rule `paths::PathError` follows, and for the same reason: a
        // message that stops at "this is wrong" leaves the reader to guess
        // whether anything was written.
        for error in [
            MigrationError::NewerSchema {
                found: 9,
                supported: 1,
            },
            MigrationError::Foreign {
                tables: vec!["notes".to_owned()],
            },
            MigrationError::Failed {
                version: 2,
                name: "indexes",
                message: "no such column".to_owned(),
            },
            MigrationError::Busy {
                message: "database is locked".to_owned(),
            },
            MigrationError::Header {
                message: "unreadable".to_owned(),
            },
            MigrationError::OutOfOrder {
                expected: 2,
                found: 3,
            },
        ] {
            let text = error.to_string();
            assert!(
                text.contains("SURE stopped rather than"),
                "no statement of what SURE did instead:\n{text}"
            );
            assert!(
                text.lines().filter(|line| !line.trim().is_empty()).count() >= 2,
                "a single statement with no next step:\n{text}"
            );
        }
    }

    #[test]
    fn the_foreign_message_names_the_tables_it_found() {
        let text = MigrationError::Foreign {
            tables: vec!["notes".to_owned(), "tags".to_owned()],
        }
        .to_string();
        assert!(text.contains("\"notes\""), "{text}");
        assert!(text.contains("\"tags\""), "{text}");
    }
}

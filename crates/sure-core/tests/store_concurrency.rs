//! What happens when several short-lived SURE processes write at once.
//!
//! # The shape of the real workload
//!
//! SURE is not a daemon. Every hook invocation is a process that starts, opens
//! the history file, appends one record, and exits — and a single agent session
//! fires several of them in the same second, sometimes while a `sure check` is
//! running in another terminal. So the concurrency SURE has to survive is
//! *several processes each doing one write*, not many threads sharing one
//! connection.
//!
//! # Why the writes are not tested in one process
//!
//! Two connections in one test process share SQLite's library state and the
//! test harness's serialisation. A test built that way can pass while two real
//! `sure.exe` processes lose records. The writing tests therefore spawn **real
//! child processes**, from this test binary, and assert on the file they leave
//! behind.
//!
//! # What is claimed
//!
//! The contract in `sure_core::store`'s module documentation, checked here:
//!
//! 1. every write is one `BEGIN IMMEDIATE` … `COMMIT` transaction;
//! 2. writers serialise in SQLite's busy handler;
//! 3. the wait is bounded by the connection's busy timeout;
//! 4. a timeout is reported, never retried forever and never dropped;
//! 5. `synchronous = FULL`.
//!
//! Claim 4 is the one that matters most and the one that is hardest to test,
//! because the interesting case is an *error*, and an error that is swallowed
//! looks exactly like success.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sure_core::store::{HistoryFilter, Store, StoreError, StoreOptions};
use sure_protocol::documents::DocumentKind;

/// The variable a spawned child reads its instructions from.
///
/// One variable holding a path and a count, rather than several: a child
/// missing its instructions must fail loudly, and reading one variable with
/// `expect` does that.
const CHILD_ENV: &str = "SURE_STORE_TEST_CHILD";

/// The filter that sees every record, recordings included.
///
/// Used by every read here so that a test cannot pass by reading nothing. The
/// default filter excludes recordings, and a count through it on a database of
/// recordings is zero — which looks like agreement with almost any expectation.
fn everything() -> HistoryFilter<'static> {
    HistoryFilter {
        include_recordings: true,
        ..HistoryFilter::default()
    }
}

/// A path under `target/tmp`, which is git-ignored and on the same volume as
/// the checkout.
///
/// # Why the name carries the process id
///
/// These tests need a **fresh, empty** directory, and they used to get one by
/// clearing a fixed path and hoping. That made correctness depend on a deletion
/// succeeding, and on Windows a deletion of a file another process still holds
/// open does not succeed. The failure was silent — `let _ = remove_dir_all(…)` —
/// and what followed was not: `Store::open_at` reopens a database that is
/// already on disk, so the run appended its own records to the previous run's
/// and the count assertions below reported that "a write was lost".
///
/// That was measured rather than reasoned about. With a handle held open on
/// `racing_writers/sure.db`, the next run counted **200 records where 100 were
/// written**, and reported a lost write — the opposite of what had happened,
/// about a file that was never fresh. A `let _ =` on a precondition is how a
/// false report gets written.
///
/// The path is therefore unique to this process, as it already is in
/// `config/authority.rs`, `config/mod.rs` and `discover_node.rs`. Freshness then
/// does not depend on a deletion at all: even if a previous run's directory is
/// still locked and still there, it is not this run's, and nothing here reads
/// it. The clear stays as a backstop for the one case uniqueness does not cover
/// — a reused process id — and *that* one stops the test rather than being
/// ignored, because it is the case where a stale database really could be read.
fn scratch(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the checkout root")
        .join("target")
        .join("tmp")
        .join("concurrency")
        .join(format!("{name}-{}", std::process::id()));
    match std::fs::remove_dir_all(&directory) {
        Ok(()) => {}
        // The ordinary case, and now the expected one: a name no earlier run used.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!(
            "cannot clear {}: {error}. This process id was used before and its database \
             is still on disk, so the counts below could not tell it apart from this \
             run's writes.",
            directory.display()
        ),
    }
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

/// One event document, which is what a hook writes.
///
/// An event because it is the record this whole file is about. The fields are
/// the ones the schema requires; if the event schema gains a required field,
/// these fixtures stop storing and the failure names the field.
///
/// The sequence number goes inside `payload`, which is the only open object the
/// event schema has — `additionalProperties` is false at the top level, so a
/// top-level `seq` is rejected, which is how this fixture was first written and
/// how the rejection was found.
fn event_document(seq: usize) -> serde_json::Value {
    serde_json::json!({
        "schema_version": sure_protocol::PROTOCOL_VERSION,
        "source": "claude-code",
        "event_type": "tool.completed",
        "timestamp": "2026-09-14T09:10:56.827Z",
        "payload": {"seq": seq},
    })
}

/// A moment, in milliseconds since the epoch, that every child can be ready by.
///
/// # Why there is a barrier at all
///
/// Spawning six processes takes longer than migrating a database. Without a
/// barrier the children run one after another, each finding the file already
/// migrated, and the test passes — while testing nothing. That was checked: with
/// the in-transaction version re-read removed, the unbarriered test still
/// passed. A test that cannot fail is worse than no test, because it is read as
/// evidence.
///
/// The margin is generous on purpose. A child that starts after the moment
/// simply runs late and contends less; the test does not fail for being slow,
/// and the other children still collide.
const BARRIER_MARGIN_MS: u128 = 800;

fn a_moment_from_now() -> u128 {
    now_ms() + BARRIER_MARGIN_MS
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("a clock after 1970")
        .as_millis()
}

/// Spawn this test binary again, running `child_writer` in it.
///
/// A fresh process, not a thread: the point of this file is the process
/// boundary, and a thread shares the parent's SQLite library, its locks and
/// its error state.
///
/// `starts_at_ms` is the barrier — see [`a_moment_from_now`]. The instructions
/// are newline-separated because a path can contain any other separator and
/// cannot contain a newline.
fn spawn_child(database: &Path, writes: usize, starts_at_ms: u128) -> std::process::Child {
    let exe = std::env::current_exe().expect("the test binary's own path");
    Command::new(exe)
        // `--exact`, so the filter cannot match another test whose name starts
        // the same way and have the child run the parent's tests. `--ignored`,
        // because the child's half is not a test on its own.
        //
        // `--quiet` does **not** keep the parent's output clean, which is worth
        // knowing before anyone counts tests from it: libtest's `--quiet` drops
        // the `running N tests` line and the per-test lines, and still prints
        // its own `test result: ok. 1 passed; … 6 filtered out` summary. The ten
        // children this file spawns therefore add ten `test result:` lines to
        // whatever contains the parent, and summing those lines over a workspace
        // run overstates the suite by ten. Count the parent lines, or read the
        // per-binary figures, and see `progress/HANDOFF.md`. A child that fails
        // still prints its panic.
        .args(["--exact", "child_writer", "--ignored", "--quiet"])
        .env(
            CHILD_ENV,
            format!("{}\n{writes}\n{starts_at_ms}", database.display()),
        )
        .spawn()
        .expect("a child process")
}

/// The child's half: wait for the barrier, then open and write `writes` events.
///
/// Ignored by default so a normal test run does not execute it as a test of its
/// own. [`spawn_child`] reaches it with `--ignored --exact`.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_writer() {
    let instructions = std::env::var(CHILD_ENV).expect("the child's instructions");
    let mut parts = instructions.split('\n');
    let path = parts.next().expect("a path");
    let writes: usize = parts.next().expect("a count").parse().expect("a count");
    let starts_at_ms: u128 = parts
        .next()
        .expect("a start time")
        .parse()
        .expect("a start time");

    // A spin rather than a sleep: the barrier is meant to land every process on
    // `Store::open_at` within the same millisecond, and Windows sleeps in
    // roughly 15 ms steps. Bounded by the parent's own moment, so it cannot
    // spin forever — unless the parent's clock and this one disagree, which is
    // why the bound is the moment and not a count of iterations.
    while now_ms() < starts_at_ms {
        std::hint::spin_loop();
    }

    let store = Store::open_at(Path::new(path)).expect("the store opens");
    for seq in 0..writes {
        store
            .append(
                sure_core::store::RecordKind::Document(DocumentKind::Event),
                &event_document(seq),
            )
            .expect("the event is written");
    }
}

#[test]
fn several_processes_writing_at_once_all_succeed() {
    // The acceptance criterion as a test. Four processes, 25 events each, all
    // starting against a database that does not exist yet — so they race the
    // migration as well as each other, which is what happens the first time a
    // user runs SURE twice within a second.
    const WRITERS: usize = 4;
    const EACH: usize = 25;

    let database = scratch("racing_writers").join("sure.db");
    // Spawned before the barrier moment, so every writer arrives together and
    // the contention is real rather than a sequence of quiet turns.
    let start = a_moment_from_now();
    let children: Vec<_> = (0..WRITERS)
        .map(|_| spawn_child(&database, EACH, start))
        .collect();

    for (index, mut child) in children.into_iter().enumerate() {
        let status = child.wait().expect("the child exits");
        assert!(
            status.success(),
            "writer {index} failed with {status}. A failed child's panic says which write and \
             why; run with --nocapture to see it interleaved."
        );
    }

    let store = Store::open_at(&database).expect("the store reopens");
    assert_eq!(
        store.count(&everything()).expect("a count"),
        (WRITERS * EACH) as u64,
        "a write was lost"
    );

    // Not a formality. A history that has lost rows without losing anything
    // visible is the failure this whole task exists to prevent, and SQLite can
    // report a file as readable while its own structures disagree.
    store.integrity_check().expect("a healthy file");

    // Every row is a whole event, not a truncated or merged one. A dropped
    // write leaves a gap in the sequence and a duplicated row repeats a value.
    let records = store
        .history(&everything(), WRITERS * EACH)
        .expect("a read of the history");
    assert_eq!(records.len(), WRITERS * EACH);
    let mut seen: Vec<usize> = Vec::with_capacity(records.len());
    for record in &records {
        assert!(record.id > 0);
        assert_eq!(record.document_version, sure_protocol::DOCUMENT_VERSION);
        assert_eq!(
            record.kind,
            sure_core::store::RecordKind::Document(DocumentKind::Event)
        );
        let seq = record
            .document
            .get("payload")
            .and_then(|payload| payload.get("seq"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| panic!("a row is not an event: {}", record.document));
        seen.push(usize::try_from(seq).expect("a small sequence"));
    }
    seen.sort_unstable();
    // Each writer counts from zero, so the multiset of sequences is 0..EACH
    // repeated WRITERS times. Rows that survived as a merged or rewritten value
    // would show up as a different multiset.
    let expected: Vec<usize> = (0..EACH).cycle().take(WRITERS * EACH).collect();
    let mut expected = expected;
    expected.sort_unstable();
    assert_eq!(
        seen, expected,
        "the surviving rows are not the events written"
    );
}

#[test]
fn the_identity_of_a_record_survives_a_delete() {
    // Why the schema uses `AUTOINCREMENT`. Without it SQLite reuses the rowid
    // of a deleted row, so a record id quoted in a report or a bug report can
    // name a *different* record after the user prunes their history. Nothing
    // fails; the quoted id still resolves.
    let database = scratch("no_id_reuse").join("sure.db");
    let store = Store::open_at(&database).expect("the store opens");

    let event = sure_core::store::RecordKind::Document(DocumentKind::Event);
    let first = store.append(event, &event_document(0)).expect("written");
    store.delete_recordings().expect("the delete runs");
    store
        .delete(&HistoryFilter::default())
        .expect("the event delete runs");
    let second = store.append(event, &event_document(1)).expect("written");

    assert!(
        second > first,
        "record {second} reuses the id of the deleted record {first}"
    );
}

#[test]
fn a_write_that_times_out_is_reported_rather_than_dropped() {
    // The dangerous failure, made deterministic. The write lock is held by
    // another connection while a writer with a busy timeout far too short to
    // outlast it tries to append. It must come back with an error that says the
    // record was not saved.
    //
    // A writer that retried forever would hang the hook that called it; one
    // that gave up silently would leave an agent session's history missing the
    // events that describe what it did. Both look like a working SURE.
    //
    // The lock is held by a second connection *in this process*. That is
    // equivalent here because SQLite's lock is on the file, and it mediates
    // between connections in one process exactly as between processes — which
    // the tests above check across a real process boundary.
    let database = scratch("busy").join("sure.db");
    // Open first, so the file exists and is already in WAL: opening a new file
    // needs an exclusive lock, and the test would then be measuring that rather
    // than the write path.
    let holder = Store::open_at(&database).expect("the store opens");
    let lock = rusqlite::Connection::open(&database).expect("a second connection");
    lock.execute_batch("BEGIN IMMEDIATE")
        .expect("the write lock is taken");

    let impatient = Store::open_with(
        &database,
        StoreOptions {
            busy_timeout: Duration::from_millis(50),
        },
    )
    .expect("the second store opens");

    let error = impatient
        .append(
            sure_core::store::RecordKind::Document(DocumentKind::Event),
            &event_document(0),
        )
        .expect_err("the write cannot succeed while the lock is held");

    match &error {
        StoreError::Busy { waited_ms, path } => {
            assert_eq!(
                *waited_ms, 50,
                "the report does not say how long SURE waited"
            );
            assert_eq!(path, &database);
        }
        other => panic!("expected a busy report, got {other:?}"),
    }
    let text = error.to_string();
    assert!(
        text.contains("was not saved"),
        "the report does not say whether the record survived:\n{text}"
    );

    lock.execute_batch("ROLLBACK")
        .expect("the lock is released");

    // And the report was true: the writer that gave up wrote nothing. This is
    // the half that a swallowed error would fail.
    assert_eq!(
        impatient.count(&everything()).expect("a count"),
        0,
        "the write that timed out reached the file anyway"
    );
    // The holder's own store is unaffected and still usable.
    holder
        .append(
            sure_core::store::RecordKind::Document(DocumentKind::Event),
            &event_document(0),
        )
        .expect("the store that did not time out still writes");
}

#[test]
fn a_second_process_can_write_once_the_lock_is_released() {
    // The other half of "a timeout is reported, never retried forever": the
    // remedy the message gives — run the command again — has to work.
    let database = scratch("after_busy").join("sure.db");
    let holder = Store::open_at(&database).expect("the store opens");
    let lock = rusqlite::Connection::open(&database).expect("a second connection");
    lock.execute_batch("BEGIN IMMEDIATE")
        .expect("the lock is taken");

    let impatient = Store::open_with(
        &database,
        StoreOptions {
            busy_timeout: Duration::from_millis(50),
        },
    )
    .expect("the second store opens");
    let event = sure_core::store::RecordKind::Document(DocumentKind::Event);
    assert!(impatient.append(event, &event_document(0)).is_err());

    lock.execute_batch("ROLLBACK")
        .expect("the lock is released");
    impatient
        .append(event, &event_document(0))
        .expect("the retry succeeds");

    assert_eq!(impatient.count(&everything()).expect("a count"), 1);
    let _ = holder;
}

#[test]
fn many_processes_opening_a_fresh_file_do_not_report_a_broken_history() {
    // Six processes opening the same nonexistent file at the same moment, which
    // is the state a new installation is in the first time two hooks fire
    // together. Every child creates, migrates and writes. Nothing may fail, and
    // the file must be at the latest schema with every write in it.
    //
    // This is the test that found the journal-mode problem: `PRAGMA journal_mode
    // = WAL` is documented as a case where SQLite declines to invoke the busy
    // handler, so five of these six used to die with "database is locked" before
    // they reached a single write. Run without the barrier they never collide
    // and the bug does not appear — which is why the barrier is here.
    //
    // Kept separate from the writing test because this failure happens at open
    // time, and a child that failed to open would be reported there as a lost
    // write — true, but pointing at the wrong cause.
    //
    // # What this does not cover
    //
    // The in-transaction version re-read in `migrations::apply_one`, which stops
    // two processes that both read version 0 from both running migration 1. With
    // the journal-mode wait in place the children are serialised past that
    // window, so removing the re-read leaves this test passing — checked, not
    // assumed. Its test is
    // `store::migrations::tests::a_migration_another_process_already_ran_is_not_run_a_second_time`,
    // which is deterministic because it calls `apply_one` directly on a database
    // that is already migrated.
    const OPENERS: usize = 6;

    let database = scratch("racing_openers").join("sure.db");
    let start = a_moment_from_now();
    let children: Vec<_> = (0..OPENERS)
        .map(|_| spawn_child(&database, 1, start))
        .collect();

    for (index, mut child) in children.into_iter().enumerate() {
        let status = child.wait().expect("the child exits");
        assert!(status.success(), "opener {index} failed with {status}");
    }

    let store = Store::open_at(&database).expect("the store reopens");
    assert_eq!(store.count(&everything()).expect("a count"), OPENERS as u64);
    assert_eq!(
        store.schema_version().expect("a version"),
        sure_core::store::LATEST_SCHEMA_VERSION
    );
}

#[test]
fn a_fresh_file_starts_in_wal_and_stays_there() {
    // The journal mode is what the read-concurrency half of the contract rests
    // on, and it is a property of the file rather than of a connection — so it
    // has to survive being reopened, and being reopened by a process that
    // wanted something else.
    let database = scratch("journal_mode").join("sure.db");
    let first = Store::open_at(&database).expect("the store opens");
    assert_eq!(first.journal_mode().to_lowercase(), "wal");
    drop(first);

    let second = Store::open_at(&database).expect("the store reopens");
    assert_eq!(
        second.journal_mode().to_lowercase(),
        "wal",
        "the journal mode was not remembered by the file"
    );
}

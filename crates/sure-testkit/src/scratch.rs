#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! A scratch directory a test can have to itself, and the bookkeeping that
//! stops a suite running out of them.
//!
//! # What this replaces
//!
//! Twenty-five test files and test modules each carried their own copy of a
//! routine that tried `{what}-{NEXT}`, with `NEXT` counting up from zero in
//! every process, and returned the first name `create_dir` accepted — the
//! counter shared by every prefix, the candidate made unique only by being
//! absent, and the loop stopping after a thousand attempts with
//! `no free directory under ...`. Nothing ever cleared a pool, so those
//! thousand names were a budget rather than a guard: one full suite run
//! allocates about 123 of them under one pool, the run that opened `P15-T015`
//! could not start at all until 3,641 directories were deleted by hand, and the
//! same shape sat unread in twenty-four other files.
//!
//! The budget was the defect, not the number. A helper whose answer can be "I
//! have used this many names and none is free" reaches that answer on a healthy
//! tree, and a check that fails for a reason the change under test did not
//! cause is worse than no check: it makes the next person doubt their own work.
//!
//! # What this does instead
//!
//! A run claims **one directory per pool** and hands out a directory inside it
//! for each call.
//!
//! - The run directory is `<root>/<pool>/run-<pid>`. No other process can name
//!   it: a live process cannot have this process id, and a process id that is
//!   not live writes nothing.
//! - Each call's directory is `<run>/<what>-<n>`, with `n` from a counter that
//!   (a) only ever goes up and (b) is shared by every pool in the process, so
//!   two calls in one process cannot be handed the same name whether they are
//!   in the same pool or in different ones.
//!
//! So uniqueness is by construction and not by collision. The old helper asked
//! "is this name free?" and could be told *no* a thousand times; this one asks
//! "what is this process's own next name?", which nothing outside the process
//! can affect. Two suites running at once are two process ids, so they cannot
//! be handed the same directory; and there is no shared namespace left to fill
//! up.
//!
//! # Clearing, and why it is safe here
//!
//! The helper this replaces recorded why it never cleared a fixed path:
//!
//! > clearing a fixed path and then treating it as fresh fails on Windows,
//! > where a deletion can fail silently, and the test then describes a
//! > directory that was never emptied.
//!
//! That reason is not repealed here, and it is not a Windows quirk to be waved
//! away: on every platform `remove_dir_all` can fail part-way, leave its target
//! standing, and be reported as success. The rule is answered by where the
//! deletion happens and by what happens to a name afterwards.
//!
//! - **The path is not shared, and it is not fixed.** A run clears only
//!   `run-<pid>` names — directories a *previous* process with this process id
//!   left behind, which no live process can own. It never touches a directory
//!   another process could be using, and it never touches anything it did not
//!   name itself.
//! - **A directory is never adopted.** The run takes its directory with
//!   `create_dir`, which creates *and* fails when the name is taken; a name
//!   that is already there is never entered, never emptied in place and never
//!   handed to a test. If the clearing above could not remove a name, that name
//!   is skipped and the next one is taken. A silently failed deletion therefore
//!   costs one leaked directory, and cannot produce a test describing a
//!   directory that was never emptied.
//! - **Nothing that survives is reused.** A directory this helper did not
//!   create is not a directory a test is ever given, so the failure mode is
//!   space, not a false pass.
//!
//! # What it does when it cannot tell
//!
//! A pool that cannot be created, and a name whose `create_dir` fails for
//! anything other than "already there", panic and name the path — a broken
//! working copy, not a product condition.
//!
//! A run directory that cannot be removed is the case this helper cannot
//! decide: the deletion may have failed because something else holds a handle,
//! rather than because it is full. It errs towards keeping — the directory is
//! left exactly as it is, the run moves to the next name, and the space is
//! leaked rather than the directory handed out. It never errs towards emptying
//! a directory it could not verify, because that is the failure the original
//! comment was written about.
//!
//! # What it does not do
//!
//! It does not reclaim the directories of *other* process ids. A stale
//! `run-<some other pid>` is left where it is until a process with that id runs
//! again, which is when it is reclaimed as its own. Deciding that another
//! process is dead cannot be done from `std` on all three platforms — a file
//! lock is advisory, and on a file system that does not implement it every
//! process is told it holds the lock — and a wrong answer there deletes a
//! directory a suite running at that moment is using, which is the one failure
//! this helper exists to avoid. The pools under `target/tmp` hold the
//! directories of the old helper's names as well; those are left alone too,
//! because nothing reads them any more and only their owner should delete them.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// How many names a run tries before it decides something outside the suite is
/// writing into its own run directory.
///
/// **This is not the budget the old helper had.** The old thousand was shared
/// by every call in the pool and was consumed by ordinary runs; this is a
/// private directory that only this process id can name, so the retry can only
/// be reached by something else writing into it. The names come from a `u64`
/// counter that is never reset, so no sequence of runs reaches this by
/// allocating; the loop is here so that a premise broken from outside is
/// reported instead of spinning.
const RETRIES: u32 = 64;

/// A directory of the test's own, under the repository's `target/tmp`.
///
/// `pool` is the directory the callers of this program have always used —
/// `sure cli contract`, `sure 指纹 checks` — and it is passed through whole:
/// the spaces and the non-ASCII characters are the fixture, not decoration,
/// and nothing here splits it, trims it or re-encodes it.
///
/// # Panics
///
/// Panics if the pool or the directory cannot be created. That is a broken
/// working copy rather than a product condition, and a test that quietly
/// skipped its assertions would be worse than one that stops.
#[must_use]
pub fn directory(pool: &str, what: &str) -> PathBuf {
    under(
        &crate::repository_root().join("target").join("tmp"),
        pool,
        what,
    )
}

/// [`directory`], under a root the caller names instead of the checkout.
///
/// One pool in this repository is not under `target/tmp`: the container search
/// order test wrote to the system temp directory before this helper existed, and
/// it asks for its own root rather than being moved. Nothing in that test depends
/// on where the directory is — it builds a `PATH` out of nothing but that
/// directory — so this is a path left where it was, not a second rule.
///
/// # Panics
///
/// As [`directory`].
#[must_use]
pub fn under(root: &Path, pool: &str, what: &str) -> PathBuf {
    let run = run_directory(&root.join(pool));

    let mut next = NEXT.fetch_add(1, Ordering::Relaxed);
    for _ in 0..RETRIES {
        let candidate = run.join(format!("{what}-{next}"));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return candidate,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                next = NEXT.fetch_add(1, Ordering::Relaxed);
            }
            Err(error) => panic!("cannot create {}: {error}", candidate.display()),
        }
    }
    panic!(
        "{} names from {} onward are already taken under {}, and that directory is \
         this process's own, so something outside this suite is writing into it",
        RETRIES,
        NEXT.load(Ordering::Relaxed),
        run.display()
    );
}

/// Every name handed out in this process, so that two of them are never equal.
///
/// Shared by every pool rather than kept per pool: a name is then unique
/// process-wide, and `AlreadyExists` in [`under`] means something outside the
/// suite rather than a counter that has come round again.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// The run directory this process is using for `pool`, claiming it if this is
/// the first call in the process to ask.
fn run_directory(pool: &Path) -> PathBuf {
    static CLAIMED: OnceLock<Mutex<HashMap<PathBuf, PathBuf>>> = OnceLock::new();

    std::fs::create_dir_all(pool)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", pool.display()));

    // Keyed on the resolved path, so that two spellings of one pool in one test
    // binary — `CARGO_MANIFEST_DIR` and `..\..\target\tmp` reach the same
    // directory — share one run directory instead of each claiming its own and
    // clearing the other's.
    let key = std::fs::canonicalize(pool).unwrap_or_else(|_| pool.to_path_buf());

    let mut claimed = CLAIMED
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        // A test that panicked elsewhere may have panicked while holding this
        // lock. The map is still consistent — a claim is inserted after the
        // directory exists — so a poisoned lock must not fail every later test.
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(run) = claimed.get(&key) {
        return run.clone();
    }
    let run = claim(pool);
    claimed.insert(key, run.clone());
    run
}

/// Reclaim this process id's own leftovers, then take a name for this run.
fn claim(pool: &Path) -> PathBuf {
    let stem = format!("run-{}", std::process::id());
    let prefixed = format!("{stem}-");

    if let Ok(entries) = std::fs::read_dir(pool) {
        for entry in entries.flatten() {
            // `OsStr` first, so a name that is not text is compared as the
            // bytes the file system gave it rather than as a replacement
            // character that could make it look like one of ours.
            let name = entry.file_name();
            let ours = name == OsStr::new(&stem)
                || name
                    .to_str()
                    .is_some_and(|name| name.starts_with(&prefixed));
            if ours {
                // Failure is expected on Windows while a scanner or a child
                // process still holds a handle. The directory is left as it is
                // and the loop below takes the next name; see the module notes.
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    for index in 0..RETRIES {
        let candidate = if index == 0 {
            pool.join(&stem)
        } else {
            pool.join(format!("{stem}-{index}"))
        };
        match std::fs::create_dir(&candidate) {
            Ok(()) => return candidate,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => panic!("cannot create {}: {error}", candidate.display()),
        }
    }
    panic!(
        "every name from {stem} to {stem}-{} is taken under {}, and those are this \
         process's own names, so something outside this suite is writing into the pool \
         or a deletion is not taking effect",
        RETRIES - 1,
        pool.display()
    );
}

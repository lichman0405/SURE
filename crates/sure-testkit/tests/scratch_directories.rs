//! The scratch directory helper's own contract, checked from outside it.
//!
//! P15-T015's first acceptance line is *"no sequence of runs exhausts any copy"*,
//! and the part of it that makes the line worth anything is *"observed by
//! filling the pool and running, not by reading the allocator"*. The first test
//! below does exactly that: it fills a pool with the thousand names the
//! allocator this replaces would have tried — the whole of its attempt loop and
//! the thousand after them — and then asks for a directory. That request is the
//! shape that used to end in `no free directory under ...`; a helper that had
//! kept the budget fails here on an empty checkout, without anyone having to run
//! a suite eight times to fill a real pool.
//!
//! The other four pin the rules the design rests on rather than its
//! arithmetic: a run directory that is not this process's is left exactly as it
//! is, this process id's own leftovers are reclaimed before the run starts, two
//! calls in one process are two directories, and the pool name a caller passes
//! is used whole — spaces and non-ASCII characters included, because those names
//! are the fixture and not decoration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use sure_testkit::scratch;

/// A directory this test may fill, without touching a pool any suite uses.
///
/// It is itself a scratch directory, so the rules under test are the rules that
/// make the name safe to write 1,000 directories into.
fn a_root() -> PathBuf {
    scratch::directory("scratch directories", "test root")
}

#[test]
fn a_pool_full_of_the_names_the_old_allocator_would_have_tried_still_hands_one_out() {
    let root = a_root();
    let pool = root.join("filled pool");
    std::fs::create_dir_all(&pool).expect("the pool");

    // The state the old helper's loop reached: every one of its thousand
    // candidates already taken, each answered by trying the next name. A pool
    // arrives here after about eight suite runs, and it is the state that left
    // the run opening this task unable to start at all. The second thousand
    // names are filled as well so that the state does not depend on where in the
    // sequence a process's shared counter happened to be when the call arrived —
    // the counter is process-wide and starts at zero, and this test does not get
    // to choose how many names its own scratch root has already used up.
    for attempt in 0..2_000 {
        std::fs::create_dir(pool.join(format!("thing-{attempt}"))).expect("a taken name");
    }

    let handed = scratch::under(&root, "filled pool", "thing");

    assert!(handed.is_dir(), "{} is not a directory", handed.display());
    assert_ne!(
        handed.parent(),
        Some(pool.as_path()),
        "the directory handed back was taken straight out of the pool, which is \
         where all thousand taken names live: {}",
        handed.display()
    );
    assert!(
        handed.parent().is_some_and(|run| run
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("run-"))),
        "the directory handed back is not inside a run directory: {}",
        handed.display()
    );

    // Best effort, and only over what this test made: the filled pool is inside
    // this run's own scratch directory, so nothing else can be using it. A
    // deletion that does not happen costs space and breaks nothing — the names
    // are only ever checked for, and `create_dir` on one that still exists is
    // answered the same way the second time.
    let _ = std::fs::remove_dir_all(&pool);
}

#[test]
fn a_run_directory_that_is_not_this_process_is_left_exactly_as_it_is() {
    let root = a_root();
    let foreign = root
        .join("shared pool")
        .join(format!("run-{}", std::process::id().wrapping_add(1)));
    std::fs::create_dir_all(&foreign).expect("a run directory of another process");
    std::fs::write(foreign.join("kept.txt"), b"another process's scratch").expect("a file in it");

    let handed = scratch::under(&root, "shared pool", "thing");

    assert_eq!(
        std::fs::read(foreign.join("kept.txt")).expect("the file is still there"),
        b"another process's scratch",
        "a run directory that is not this process's was deleted or emptied: {}",
        foreign.display()
    );
    assert!(
        !handed.starts_with(&foreign),
        "the directory handed out is inside another process's run directory"
    );
}

#[test]
fn this_process_ids_own_leftovers_are_reclaimed_before_the_run_starts() {
    let root = a_root();
    let run = root
        .join("reclaimed pool")
        .join(format!("run-{}", std::process::id()));
    let leftover = run.join("left-by-an-earlier-process-with-this-id");
    std::fs::create_dir_all(&leftover).expect("a leftover directory");
    std::fs::write(leftover.join("kept.txt"), b"stale").expect("a file in it");

    let handed = scratch::under(&root, "reclaimed pool", "thing");

    assert!(
        !leftover.exists(),
        "the leftovers of this process id's previous run were not reclaimed: {}",
        leftover.display()
    );
    assert!(
        handed.starts_with(&run),
        "{} is not inside this process's run directory",
        handed.display()
    );
    assert!(handed.is_dir(), "{} is not a directory", handed.display());
}

#[test]
fn two_calls_in_one_process_are_two_directories() {
    let root = a_root();

    let first = scratch::under(&root, "one pool", "thing");
    let second = scratch::under(&root, "one pool", "thing");

    assert_ne!(first, second, "two calls were handed the same directory");
    assert!(first.is_dir(), "{} is not a directory", first.display());
    assert!(second.is_dir(), "{} is not a directory", second.display());
}

#[test]
fn the_pool_name_is_used_whole() {
    let root = a_root();
    // Spaces and non-ASCII characters on purpose: a pool name is a path
    // component, and a helper that trimmed, split or re-encoded it would report
    // success while writing somewhere else than it was asked to.
    let pool = "é 中文 pool with spaces";

    let handed = scratch::under(&root, pool, "thing");

    assert!(
        root.join(pool).is_dir(),
        "the pool was not created under the name it was given"
    );
    assert!(
        handed.starts_with(root.join(pool)),
        "{} is not under the pool it named",
        handed.display()
    );
}

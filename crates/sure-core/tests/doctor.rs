//! The promises `sure doctor` makes about itself, checked from outside the crate.
//!
//! `sure_core::doctor`'s module documentation makes four claims that its own
//! unit tests cannot hold it to, because each is about a boundary rather than
//! about a value:
//!
//! 1. it never reads the settings file;
//! 2. it never creates the store;
//! 3. it names this build by its number alone;
//! 4. it always says what it did not look at.
//!
//! The first is an *absence* — no run of the binary can demonstrate that a line
//! of code was never written — so it is checked against the source, the way
//! `cli_contract.rs` checks that only `output.rs` writes to a stream. The rest
//! are checked against what the public API returns, because a promise about the
//! shape of a report is only worth anything if the report a caller gets has that
//! shape.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use sure_core::doctor::{self, Places, Presence, StoreState};
use sure_core::paths::Paths;

/// A scratch directory under `target/tmp`, which is git-ignored and on the same
/// volume as the checkout, as the store tests use.
///
/// The name carries the process id, so no other process can compute this path.
/// The reason is the one `store_concurrency.rs` records in full, and one more
/// that is particular to this file: `the_report_never_creates_what_it_reports_on`
/// was seen to fail with `os error 3` on a directory it had just created, and the
/// only code in the repository that deletes that path is `scratch` itself. A path
/// another instance of this test cannot compute is a path another instance
/// cannot delete. **That mechanism was not confirmed** — the failure appears
/// roughly once in ten runs of the whole workspace and never once in twelve runs
/// of this binary alone — so this removes a shared resource rather than
/// repairing a proven fault, and it is recorded that way in `progress/HANDOFF.md`.
fn scratch(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("tmp")
        .join("doctor_contract")
        .join(format!("{name}-{}", std::process::id()));
    match fs::remove_dir_all(&root) {
        Ok(()) => {}
        // The ordinary case, and now the expected one.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!(
            "cannot clear {}: {error}. This process id was used before and its files \
             are still on disk, so this test could not tell them from the ones it \
             writes.",
            root.display()
        ),
    }
    fs::create_dir_all(&root).expect("a scratch directory");
    root
}

fn places_for(root: &Path) -> Paths {
    Paths::from_roots(root.join("data"), root.join("config")).expect("absolute roots")
}

/// The doctor module's source.
fn source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("doctor.rs");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn the_diagnostic_never_reaches_for_the_settings_module() {
    // `docs/architecture/DIAGNOSTICS.md`: a secret must never have to be handled
    // in order to be reported. The way to keep that true is for the diagnostic
    // never to open the file that may hold one, and the way to keep *that* true
    // is for the module not to name the reader that would.
    //
    // A source scan rather than a run, because what is being ruled out is an
    // absence. Prose about settings is fine and appears in the report itself —
    // `sure config show` is named as where the question belongs — which is why
    // the tokens are module paths and constructors rather than the word.
    const REACHES: &[&str] = &[
        "crate::config",
        "config::Config",
        "Config::load",
        "LoadedConfig",
    ];

    let text = source();
    for token in REACHES {
        assert!(
            !text.contains(token),
            "src/doctor.rs names `{token}`. The diagnostic reports where the settings \
             file is and must not read it, so that no secret has to pass through a \
             command whose output is pasted into bug reports."
        );
    }
}

#[test]
fn the_diagnostic_names_the_files_the_store_actually_uses() {
    // The report tells a user which file to delete, and the store tells them
    // which file it writes. Those are two modules' idea of the same path, and
    // the failure mode of them disagreeing is a user deleting the wrong file
    // while their history stays on disk.
    let root = scratch("same_files");
    let paths = places_for(&root);
    let report = doctor::examine(Ok(paths.clone()));

    let Places::Known(locations) = &report.places else {
        panic!("the roots were given: {:?}", report.places);
    };
    assert_eq!(locations.store_file.path, paths.store_file());
    assert_eq!(locations.settings_file.path, paths.user_config_file());
    assert_eq!(locations.data_dir.path, paths.data_dir());
    assert_eq!(locations.config_dir.path, paths.config_dir());
}

#[test]
fn the_report_never_creates_what_it_reports_on() {
    // The promise that makes the command safe to run on a machine somebody is
    // trying to diagnose: it leaves the machine as it found it. Checked from
    // outside the crate, on a root that is given rather than discovered, so that
    // the assertion is about this code rather than about this machine.
    let root = scratch("untouched");
    let paths = places_for(&root);
    let before = listing(&root);

    let report = doctor::examine(Ok(paths.clone()));

    assert_eq!(report.store, StoreState::NotCreated);
    assert!(report.is_well(), "{:#?}", report.problems);
    assert!(
        !paths.store_file().exists(),
        "sure doctor created {}",
        paths.store_file().display()
    );
    assert_eq!(
        listing(&root),
        before,
        "sure doctor changed the directories it was asked about"
    );
}

/// Everything under a directory, as paths relative to it, sorted.
fn listing(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path.clone());
            }
            found.push(
                path.strip_prefix(root)
                    .expect("under the root")
                    .to_path_buf(),
            );
        }
    }
    found.sort();
    found
}

#[test]
fn the_report_names_this_build_by_its_number_alone() {
    // Every renderer of this report prints the product's name itself. A `version`
    // holding the name as well is how the human form first read
    // `SURE SURE 0.0.0-bootstrap`, and the check that it cannot come back is
    // that the number does not contain the name.
    let report = doctor::examine_this_machine();

    assert_eq!(report.build.version, sure_core::VERSION);
    assert!(
        !report.build.version.contains(sure_core::NAME),
        "the version in the report is \"{}\", which repeats the product name every \
         renderer already prints",
        report.build.version
    );
    assert_eq!(report.build.protocol_version, sure_core::PROTOCOL_VERSION);
    assert_eq!(report.build.os, std::env::consts::OS);
    assert_eq!(report.build.arch, std::env::consts::ARCH);
}

#[test]
fn the_report_always_says_what_it_did_not_look_at() {
    // `docs/product/UX_AND_LANGUAGE.md`: a report that does not state its
    // boundary invites the reader to assume it covered everything. An empty list
    // would read as "nothing was skipped", which is never true of a program that
    // has no process runner.
    let report = doctor::examine_this_machine();

    assert!(
        !report.not_checked.is_empty(),
        "the report claims to have covered everything"
    );
    for entry in &report.not_checked {
        assert!(
            !entry.what.is_empty() && !entry.why.is_empty(),
            "{entry:#?}"
        );
        assert!(
            entry.why.ends_with('.'),
            "the reason is not a sentence: {:?}",
            entry.why
        );
    }
    for tool in &report.tools {
        assert!(
            !tool.needed_for.is_empty(),
            "{} is listed with no reason it is called",
            tool.name
        );
    }
}

#[test]
fn a_report_with_no_problems_is_well_and_one_with_a_problem_is_not() {
    // `is_well` is the single predicate the exit status and the frame's `outcome`
    // both come from, so it has to be exactly "nothing was found wrong" rather
    // than a second opinion about the report's contents. Both states are reached
    // here on a root this test owns, rather than on whatever the machine running
    // it happens to hold.
    let fresh = doctor::examine(Ok(places_for(&scratch("well"))));
    assert!(fresh.is_well(), "{:#?}", fresh.problems);
    assert!(fresh.problems.is_empty());

    // A store that is not a store. The file is where the store would be and its
    // contents are not a database, which is the one thing a user can do to this
    // installation by hand.
    let root = scratch("damaged");
    let paths = places_for(&root);
    fs::create_dir_all(root.join("data")).expect("the data directory");
    fs::write(paths.store_file(), b"this is not a database\n").expect("a damaged store");

    let damaged = doctor::examine(Ok(paths.clone()));
    assert_eq!(damaged.is_well(), damaged.problems.is_empty());
    assert!(
        !damaged.is_well(),
        "a file that is not a database was reported as a healthy installation"
    );
    assert!(matches!(damaged.store, StoreState::Unreadable { .. }));
    let Places::Known(locations) = &damaged.places else {
        panic!("the roots were given");
    };
    assert_eq!(locations.store_file.presence, Presence::Present);
    // And the repair is the user's: the diagnostic says what it found rather
    // than deleting or rebuilding the file it could not read.
    assert_eq!(
        fs::read(paths.store_file()).expect("the file is still there"),
        b"this is not a database\n"
    );
    let names_the_store = damaged
        .problems
        .iter()
        .any(|problem| problem.what.contains("recorded history"));
    assert!(
        names_the_store,
        "the problem does not say what it is about: {:#?}",
        damaged.problems
    );
}

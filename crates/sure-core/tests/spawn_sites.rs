//! Where SURE starts a process, and whether anything in the product does yet.
//!
//! `P3-T001` added the general process runner, and with it the first thing in
//! this crate that *can* be pointed at a project. Everything the product claims
//! about not running project code now depends on that runner having no caller,
//! so the claim is checked here rather than written down and hoped for — the
//! same technique, and for the same reason, as
//! `fingerprint_git.rs`'s `git_is_started_in_exactly_one_place` and
//! `scan_project.rs`'s rule about the scanner not opening files.
//!
//! # Why a source check is the only thing that can hold this
//!
//! An absence cannot be demonstrated by running anything. If a product path
//! started calling the runner, every behavioural test in the suite would still
//! pass — the runner works, which is the point of it — and the only visible
//! change would be that SURE had begun executing code out of a directory it was
//! asked to inspect. That is the exact failure this product exists to prevent,
//! and the check for it has to be made of the source text, because the source
//! text is the only place the fact lives.
//!
//! # The two rules, and what each is standing in for
//!
//! **One: every file in `crates/` that builds a `Command` is named here.** The
//! census is not the interesting fact; the interesting fact is that the list
//! *cannot grow quietly*. A fourth `Command::new` is a new way for SURE to run
//! something, and adding one means editing this list, which means reading this
//! paragraph.
//!
//! **Two: nothing outside `crates/sure-core/src/process/` mentions
//! [`ProcessRequest`].** This is the tighter of the two, and it holds the claim
//! `process/mod.rs` makes in its own words — *"no product path calls `run`
//! yet"* — because `run` takes a `&ProcessRequest` and there is no other way to
//! call it. A file that never names the type cannot be a caller, whatever else
//! it does, and that is a stronger statement than searching for `process::run`,
//! which a `use` statement, an alias or a re-export would walk straight past.
//!
//! # This test is meant to fail
//!
//! Not today — but the day `P3-T004` (classification), `P3-T005` (permission and
//! consent) or `P3-T007` (`inspect_only` enforcement) wires the runner up, this
//! fails, and it should. **Two things have to move together at that moment**: a
//! caller exists, so this list and the paragraph above it are wrong; and
//! `sure_core::support`'s ceiling of level C is justified by *no project code
//! running*, so `CEILING` and
//! `the_ceiling_todays_build_claims_is_never_above_inspect_only` are in question
//! in the same commit. A false green is more serious than a visible error, and
//! this is the error that would rather be visible.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_core::scan::{ScanOptions, scan};

/// Every file in `crates/` that builds a command, and what it runs.
///
/// The path is the one the scan reports, which is relative to `crates/`. The
/// second half of each entry is what the file is for, so that a reader deleting
/// one has to say what they deleted.
const THE_SPAWN_SITES: &[(&str, &str)] = &[
    (
        "sure-core/src/fingerprint/git/mod.rs",
        "system Git, read-only, for the content fingerprint — the one on a product path",
    ),
    (
        "sure-core/src/process/request.rs",
        "the general runner; nothing in the product calls it yet",
    ),
    (
        "sure-core/src/process/terminate.rs",
        "`taskkill`, to stop a process tree that did not stop",
    ),
];

/// The directory the runner lives in, which is the whole of what rule two
/// exempts.
const THE_RUNNER: &str = "sure-core/src/process/";

/// Every **shipped** `.rs` file under `crates/`, with its text.
///
/// Narrowed to `src/`, because the rule is about code that ships. A test file
/// is *supposed* to start processes — that is what most of them are for, and
/// `process_runner.rs` and `fingerprint_git.rs` between them account for a
/// dozen — and a check that counted them would be a check nobody could satisfy.
/// The same narrowing, for the same reason, is in
/// `git_is_started_in_exactly_one_place`.
///
/// The product's own scanner does the walking, so "a file in this crate" means
/// the same thing here as it does everywhere else in the suite — and it is
/// asserted to have been complete, because a source check over an unknown subset
/// of the sources is the false green this test exists to prevent.
fn shipped_sources() -> Vec<(String, String)> {
    let crates = sure_testkit::repository_root().join("crates");
    let walked = scan(&crates, ScanOptions::default()).expect("crates/ is a directory");
    assert!(
        walked.is_complete(),
        "the source tree could not be read completely, so this test would be \
         checking an unknown subset of it"
    );

    let shipped: Vec<(String, String)> = walked
        .files()
        .filter(|entry| {
            entry
                .path
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .map(|entry| {
            let path = crates.join(&entry.path);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            (entry.display_path(), text)
        })
        // Filtered on the reported path rather than on the filesystem one, so
        // that the string the assertions print and the string the rule is
        // stated against are the same string. Only `src` is tested for, without
        // a separator, because the separator is the platform's and this runs on
        // three of them.
        .filter(|(path, _)| path.contains("src"))
        .collect::<Vec<_>>();

    // The vacuity guard, in the helper so that both tests below get it. A walk
    // that found nothing would make every rule below true and every claim here
    // worthless — the "a test that passes because its premise did not hold is
    // worse than no test" rule this repository states about skipped cases,
    // applied to a check whose premise is a directory listing.
    assert!(
        shipped.len() > 10,
        "the source walk found {} shipped files, which is not this repository — \
         the filter is matching the wrong thing",
        shipped.len()
    );
    shipped
}

/// Whether a line is prose rather than code.
///
/// Doc comments are skipped rather than searched, because they are where the
/// rules themselves are written down and a check that forbade *mentioning* the
/// thing would make the rules impossible to explain. `doctor.rs` says
/// `Command::new("git")` in a paragraph about `CreateProcess` appending `.exe`,
/// and that paragraph is the reason this function exists.
fn is_prose(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// The files whose **code** builds a command, in scan order.
fn command_builders() -> Vec<String> {
    let mut found: Vec<String> = shipped_sources()
        .into_iter()
        .filter(|(_, text)| {
            text.lines()
                .any(|line| !is_prose(line) && line.contains("Command::new("))
        })
        .map(|(path, _)| path)
        .collect();
    found.sort();
    found
}

#[test]
fn every_place_sure_builds_a_command_is_named_here() {
    let mut expected: Vec<String> = THE_SPAWN_SITES
        .iter()
        .map(|(path, _)| (*path).to_owned())
        .collect();
    expected.sort();

    assert_eq!(
        command_builders(),
        expected,
        "the set of files that build a command has changed. Every one of them is \
         a way for SURE to run something, so the list above is the list of ways, \
         and a new one is a decision rather than an edit:\n  {}",
        THE_SPAWN_SITES
            .iter()
            .map(|(path, why)| format!("{path} — {why}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn nothing_outside_the_runner_names_a_process_request() {
    // `run` takes a `&ProcessRequest` and there is no other entry point, so a
    // file that cannot name the type cannot run anything through it. This is
    // the claim `process/mod.rs` makes — "no product path calls `run` yet" —
    // and the day it stops being true is the day the check below fails.
    let mut callers: Vec<String> = Vec::new();

    for (path, text) in shipped_sources() {
        if path.starts_with(THE_RUNNER) {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            if is_prose(line) {
                continue;
            }
            if line.contains("ProcessRequest") {
                callers.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }

    assert!(
        callers.is_empty(),
        "a shipped file has begun naming `ProcessRequest`, so it can call the \
         runner. That is a real change and not a test to update: it means SURE \
         is able to execute code from a project, which is what \
         `sure_core::support`'s level-C ceiling is justified by the absence of. \
         Move the ceiling in the same commit, or take the name back out. Found:\n  {}",
        callers.join("\n  ")
    );
}

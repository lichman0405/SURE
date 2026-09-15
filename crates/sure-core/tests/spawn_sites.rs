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
//! # The three rules, and what each is standing in for
//!
//! **One: every file in `crates/` that builds a `Command` is named here.** The
//! census is not the interesting fact; the interesting fact is that the list
//! *cannot grow quietly*. A fourth `Command::new` is a new way for SURE to run
//! something, and adding one means editing this list, which means reading this
//! paragraph. **This rule did not move when the runner got a caller**: the
//! caller builds no `Command` of its own — it builds a `ProcessRequest` and the
//! request builds the command — so the count is still three, and the third
//! rule below is what covers the new caller instead.
//!
//! **Two: nothing outside [`MAY_NAME_A_PROCESS_REQUEST`] mentions
//! [`ProcessRequest`].** This is the tightest of the three, and until `P3-T009`
//! it was the whole of the claim `process/mod.rs` used to make in its own words
//! — *"no product path calls `run` yet"* — because `run` takes a
//! `&ProcessRequest` and there is no other way to call it. A file that never
//! names the type cannot be a caller, whatever else it does, and that is a
//! stronger statement than searching for `process::run`, which a `use`
//! statement, an alias or a re-export would walk straight past. `P3-T009` made
//! the list two entries long rather than one: `sure-core/src/service.rs` is a
//! real caller, and the honest response to a rule breaking is to say why the
//! exception is one rather than to widen the rule until it stops noticing.
//!
//! **Three: nothing outside `sure-core/src/service.rs` mentions `Supervisor`.**
//! This is the rule that carries the claim the second one used to carry. The
//! runner is no longer uncalled, so "nothing calls the runner" is now false and
//! would have been a rule that had to be deleted; what is still true is one
//! level up — **`service.rs` can start a process, and nothing in the product
//! constructs a `Supervisor`**, so no ship path reaches it. That is the same
//! kind of absence as the one rule two held, checked the same way, and it is
//! checked one level higher up the call chain rather than being asserted in
//! prose about the middle of it.
//!
//! Both of the `contains` rules have the same known hole and it is the same hole
//! rule one's matcher was fixed for once: a `use crate::service::Supervisor as
//! S;` would name the type without the word appearing. Neither rule forbids a
//! *program* from being started, which no source check can do — they forbid the
//! route being added quietly, which is what a reviewer can act on.
//!
//! # This test is meant to fail
//!
//! Not today — but the day anything wires a `Supervisor` up to `sure check`,
//! this fails, and it should. **Two things have to move together at that
//! moment**: a product path exists, so this list and the paragraphs above it are
//! wrong; and `sure_core::support`'s ceiling of level C is justified by *no
//! project code running from a product path*, so `CEILING` and
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
        "the general runner, called only by `sure-core/src/service.rs`, which no product path builds yet",
    ),
    (
        "sure-core/src/process/terminate.rs",
        "`taskkill`, to stop a process tree that did not stop",
    ),
];

/// The directory the runner lives in, which is the whole of what rule two
/// exempts.
const THE_RUNNER: &str = "sure-core/src/process/";

/// The files that may name a [`ProcessRequest`], which is the whole of what rule
/// two exempts.
///
/// Two entries, and the second is the one that has to be argued for. The runner
/// builds requests; `service.rs` is the caller `P3-T009` added, and it is here
/// because the rule is not "the runner is the only place a request is named" —
/// that was never the point — but "**a file that names a request is a file that
/// can run something, so every one of them is named here and a new one is a
/// decision**". The entry is a path rather than a directory because it is one
/// file, and a path rather than a predicate because a predicate is a rule that
/// grows without anybody reading it.
const MAY_NAME_A_PROCESS_REQUEST: &[&str] = &[THE_RUNNER, THE_SUPERVISOR];

/// The one file that may name a [`Supervisor`], which is the whole of what rule
/// three exempts.
///
/// It is the same file rule two's second entry names, and that is the point
/// rather than a coincidence: the file that may build a request is the file that
/// may build the thing that builds one. Split across two constants because they
/// are two rules that happen to agree today, and the failure messages differ.
const THE_SUPERVISOR: &str = "sure-core/src/service.rs";

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

/// Whether a line of **code** builds a [`std::process::Command`].
///
/// The token boundary is the whole of the rule, and it is here because
/// `consent.rs`'s `PlannedCommand::new` is not a way to run anything while
/// `request.rs`'s `Command::new` is — and a substring search cannot tell them
/// apart, because the first contains the second. The character before `Command`
/// therefore has to be one that cannot be part of an identifier, which is true
/// of every spelling that really builds one (`Command::new(` after a `use`,
/// `std::process::Command::new(`, `process::Command::new(`) and false of every
/// type whose *name* merely ends in `Command`.
///
/// This was found by the check itself: `P3-T005` added a type called
/// `PlannedCommand` and this test failed on a file that cannot start a process
/// — it imports nothing from `std::process`, and
/// `nothing_outside_the_runner_names_a_process_request` passes on it. So the
/// census was matching a name rather than a spawn, and the honest fix is to
/// make the matcher mean what the rule says. The rule is unchanged and neither
/// of the two holes a tripwire like this can have — a bare alias such as
/// `use std::process::Command as C;` — is widened by it, because the old
/// substring search missed that spelling too.
fn builds_a_command(line: &str) -> bool {
    const PATTERN: &str = "Command::new(";
    let mut rest = line;
    while let Some(index) = rest.find(PATTERN) {
        let before = rest[..index].chars().next_back();
        if before.is_none_or(|character| !(character.is_alphanumeric() || character == '_')) {
            return true;
        }
        rest = &rest[index + PATTERN.len()..];
    }
    false
}

/// The files whose **code** builds a command, in scan order.
fn command_builders() -> Vec<String> {
    let mut found: Vec<String> = shipped_sources()
        .into_iter()
        .filter(|(_, text)| {
            text.lines()
                .any(|line| !is_prose(line) && builds_a_command(line))
        })
        .map(|(path, _)| path)
        .collect();
    found.sort();
    found
}

#[test]
fn the_matcher_tells_a_command_apart_from_a_type_whose_name_ends_in_command() {
    for line in [
        "let output = Command::new(&self.program)",
        "        let mut command = Command::new(\"taskkill\");",
        "std::process::Command::new(program)",
        "process::Command::new(program)",
        "\tCommand::new(\"git\")",
    ] {
        assert!(builds_a_command(line), "{line:?} builds a command");
    }
    for line in [
        "PlannedCommand::new(check, program, arguments, mode, permissions)",
        "let planned = PlannedCommand::new(",
        "my_Command::new(x)",
        "someCommand::new(x)",
    ] {
        assert!(
            !builds_a_command(line),
            "{line:?} does not build a std::process::Command"
        );
    }

    // A comment saying `Command::new` is not a spawn either, and this function
    // cannot tell — which is why the caller asks [`is_prose`] first. Stated as
    // the pair, so that a reader changing one of them sees the other.
    let commented = "// Command::new(\"git\") in a comment, not in code";
    assert!(builds_a_command(commented));
    assert!(is_prose(commented));
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

/// Every line of **code** in a shipped file outside `exempt` that names `token`,
/// as `path:line: text`.
///
/// Shared by rules two and three because they are one check applied to two
/// names, and writing it twice is how the two would drift apart — a fix to the
/// prose filter reaching one of them and not the other is exactly the kind of
/// difference that makes one rule weaker than it reads.
///
/// The exemption is a path **prefix** match, so an entry ending in `/` exempts a
/// directory and an entry naming a file exempts that file. Nothing here checks
/// that an exemption is needed: an entry for a file that does not exist, or that
/// no longer names the token, would sit in the list looking like a decision.
/// That is what the third test below is for.
fn namers_of(token: &str, exempt: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();

    for (path, text) in shipped_sources() {
        if exempt.iter().any(|allowed| path.starts_with(allowed)) {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            if is_prose(line) {
                continue;
            }
            if line.contains(token) {
                found.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }

    found
}

#[test]
fn nothing_outside_the_named_files_names_a_process_request() {
    // `run` takes a `&ProcessRequest` and there is no other entry point, so a
    // file that cannot name the type cannot run anything through it. This is
    // what `process/mod.rs` said as "no product path calls `run` yet" until
    // `P3-T009` gave the runner a caller; the caller is on the list above, and
    // the claim that moved is rule three's.
    let callers = namers_of("ProcessRequest", MAY_NAME_A_PROCESS_REQUEST);

    assert!(
        callers.is_empty(),
        "a shipped file has begun naming `ProcessRequest`, so it can call the \
         runner. That is a real change and not a test to update: it means SURE \
         is able to execute code from a project. Either name it in \
         MAY_NAME_A_PROCESS_REQUEST and say why in the paragraph above, or take \
         the name back out — and if the new caller is on a product path, \
         `sure_core::support`'s level-C ceiling moves in the same commit. \
         Found:\n  {}",
        callers.join("\n  ")
    );
}

#[test]
fn nothing_outside_the_supervisor_names_a_supervisor() {
    // The rule the second one used to be. `service.rs` is the only file that can
    // build a `Supervisor`, and a `Supervisor` is the only thing that can start a
    // service — so "nothing in the product starts a service" is this absence,
    // one level above the runner rather than in the middle of it.
    let namers = namers_of("Supervisor", &[THE_SUPERVISOR]);

    assert!(
        namers.is_empty(),
        "a shipped file has begun naming `Supervisor`, so it can start a \
         service. That is a real change and not a test to update: it means a \
         path out of the product reaches the runner, which is what \
         `sure_core::support`'s level-C ceiling is justified by the absence of. \
         Move the ceiling in the same commit, or take the name back out. \
         Found:\n  {}",
        namers.join("\n  ")
    );
}

#[test]
fn every_exemption_is_one_a_file_actually_needs() {
    // An exemption list rots in the direction nobody notices: a file gets
    // renamed, or the name it was exempted for is taken back out, and the entry
    // stays — exempting nothing, and reading to the next person as a decision
    // somebody made. This is the check that makes an entry mean something, and
    // it is the same shape as the census itself: the fact is not the list, it is
    // that the list cannot be wrong quietly.
    for (path, token) in [
        (THE_RUNNER, "ProcessRequest"),
        (THE_SUPERVISOR, "ProcessRequest"),
        (THE_SUPERVISOR, "Supervisor"),
    ] {
        // Asked as "would the rule have flagged this file if it were not
        // exempt?" — which is the only question that makes an exemption a
        // decision rather than a hole.
        let mut needed = false;
        for (seen, text) in shipped_sources() {
            if seen != path && !(path.ends_with('/') && seen.starts_with(path)) {
                continue;
            }
            if text
                .lines()
                .any(|line| !is_prose(line) && line.contains(token))
            {
                needed = true;
            }
        }
        assert!(
            needed,
            "{path} is exempt from the rule about `{token}` and no longer names \
             it, so the exemption is covering nothing. Either the file moved and \
             the entry did not, or the check is now the weaker for an entry \
             nobody would think to question."
        );
    }
}

//! What this repository says about container mode, and whether it is true.
//!
//! `P3-T008`'s second acceptance sentence is
//! *"Container plan controls mounts/network/working dir and is described as
//! limited isolation, not perfect sandboxing."* Its second half is a claim about
//! **prose**, and prose is the one thing in this repository that no type checks.
//! A type cannot make a `&'static str` honest, and the sentence a user reads
//! before agreeing to run thousands of lines of a stranger's code is exactly the
//! sentence where being wrong costs the most.
//!
//! So this file checks the sentences, the same way `spawn_sites.rs` checks an
//! absence that only the source text can hold: **there is a word this product
//! may not use about container mode unless it is denying it**, the word list is
//! [`sure_core::container::OVERCLAIMS`], and the rule for telling a claim from a
//! denial is [`sure_core::container::overclaims`] — the module's own function,
//! asked here rather than re-implemented, because two copies of a rule are two
//! rules.
//!
//! # Why this test exists rather than a note in a document
//!
//! The four places this file would have failed at the commit before it were not
//! obscure: `docs/adr/0009` called the mode *"an isolated environment"*,
//! `docs/architecture/EXECUTION_SAFETY.md` said *"in an isolated container"*, and
//! `ExecutionMode`'s own doc comment and consent prompt both said *"an isolated
//! container"*. Each was written by somebody who knew better, and none of them
//! was lying — *isolated* is the word the whole industry uses for this, which is
//! precisely why a check is needed and a resolution is not. `P3-T008` corrected
//! all four; this test is what stops the fifth.
//!
//! # What it does not check
//!
//! - **`progress/`.** `HANDOFF.md` and `DECISIONS.md` are dated records of what
//!   was believed and written at a time, and editing them to agree with today
//!   would be the document half of rewriting history. A wrong claim in a pushed
//!   commit is corrected in a later commit, never in place.
//! - **`MASTER_PROMPT.md`**, for the same reason plus one more: it is the
//!   governing instruction and not this task's to edit.
//! - **`crates/sure-core/src/container.rs`.** The file that defines the phrases
//!   has to name them, `OVERCLAIMS` is a string literal containing both, and a
//!   rule that could not name what it forbids would have to be written as a
//!   regex over spelling. The module's own sentences are covered instead by
//!   `no_sentence_this_module_produces_carries_an_overclaim`, which asks the same
//!   function about `isolation_claim()` and both `Availability::explain()` arms.
//!
//! # The second claim, and the rule that reads it
//!
//! `P16-T010` found a sentence this file's own subject had left alone: the
//! absent-runtime answer said *"checks run on this computer instead"*, and no
//! check runs on this computer — or anywhere else — in this build. The assertion
//! that pinned it asked whether the sentence **contained a phrase**, and it was
//! green. [`claims_local_execution`] is the rule that reads the claim instead, so
//! that a sentence saying a check runs here fails in any wording, and
//! [`denies_that_anything_runs`] is the other half: an absence must say that
//! nothing runs, not only that something is missing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_core::container::{
    Availability, OVERCLAIMS, claims_local_execution, denies_that_anything_runs, isolation_claim,
    overclaims,
};
use sure_core::scan::{ScanOptions, scan};

/// The file that defines the forbidden phrases, which therefore contains them.
const THE_RULE_ITSELF: &str = "sure-core/src/container.rs";

/// A line is in scope when it is talking about this mode at all.
///
/// Narrowing to these words is what keeps the rule from firing on
/// `safety.rs`'s note that Python's `-m build` *"makes an isolated environment
/// and installs the build"* — a real technical meaning of the same phrase, about
/// a PEP 517 build venv, in a file that has nothing to do with containers. **A
/// rule that fired there would be wrong**, and a rule that has to be argued with
/// is a rule that gets relaxed until it catches nothing.
fn is_about_container_mode(line: &str) -> bool {
    let lowered = line.to_lowercase();
    lowered.contains("container") || lowered.contains("docker") || lowered.contains("podman")
}

/// Every shipped `.rs` file under `crates/`, with its text and reported path.
///
/// The product's own scanner does the walking, so "a file in this crate" means
/// the same thing here as it does everywhere else in the suite, and the walk is
/// asserted complete — a source check over an unknown subset of the sources is
/// the false green this file exists to prevent.
fn shipped_sources() -> Vec<(String, String)> {
    let crates = sure_testkit::repository_root().join("crates");
    let walked = scan(&crates, ScanOptions::default()).expect("crates/ is a directory");
    assert!(
        walked.is_complete(),
        "the source tree could not be read completely, so this test would be \
         checking an unknown subset of it"
    );
    let found: Vec<(String, String)> = walked
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
        // `src` and not `tests`, because the rule is about what ships and
        // because this file's own test data contains every phrase on the list.
        // Tested without a separator, because the separator is the platform's.
        .filter(|(path, _)| path.contains("src"))
        .collect();
    assert!(
        found.len() > 10,
        "the source walk found {} shipped files, which is not this repository — \
         the filter is matching the wrong thing",
        found.len()
    );
    found
}

/// Every `.md` file under `docs/`, with its text and reported path.
fn documentation() -> Vec<(String, String)> {
    let docs = sure_testkit::repository_root().join("docs");
    let walked = scan(&docs, ScanOptions::default()).expect("docs/ is a directory");
    assert!(
        walked.is_complete(),
        "the docs tree could not be read completely"
    );
    let found: Vec<(String, String)> = walked
        .files()
        .filter(|entry| {
            entry
                .path
                .extension()
                .is_some_and(|extension| extension == "md")
        })
        .map(|entry| {
            let path = docs.join(&entry.path);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            (format!("docs/{}", entry.display_path()), text)
        })
        .collect();
    assert!(
        found.len() > 20,
        "the docs walk found {} files, which is not this repository",
        found.len()
    );
    found
}

/// Every place a forbidden phrase is used as a claim, as `path:line: text`.
fn overclaiming_lines() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for (path, text) in shipped_sources().into_iter().chain(documentation()) {
        if path == THE_RULE_ITSELF {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            if !is_about_container_mode(line) {
                continue;
            }
            if OVERCLAIMS.iter().any(|phrase| overclaims(line, phrase)) {
                found.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    found
}

#[test]
fn no_description_of_container_mode_calls_it_isolated_without_denying_that_it_is() {
    let found = overclaiming_lines();
    assert!(
        found.is_empty(),
        "a description of container mode uses one of {OVERCLAIMS:?} as a claim. \
         This is the wording half of `P3-T008`'s acceptance — *\"described as \
         limited isolation, not perfect sandboxing\"* — and it is a check rather \
         than a note because the word is the industry's default and reads as \
         harmless. Say **limited isolation** and name what the plan leaves open; \
         or deny the phrase in the same clause, which is allowed. Found:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn the_rule_tells_a_claim_from_a_denial_and_the_list_is_not_empty() {
    // The rule's own premises, in the place that would otherwise pass silently
    // if `OVERCLAIMS` were emptied. A repository-wide scan against an empty list
    // finds nothing and reports success, which is the exact false green this
    // file is written against.
    assert!(
        !OVERCLAIMS.is_empty(),
        "an empty list would make the scan above pass for the wrong reason"
    );
    for phrase in OVERCLAIMS {
        assert!(
            overclaims(&format!("the check runs in an {phrase}"), phrase),
            "`{phrase}` used as a claim was not caught"
        );
        assert!(
            !overclaims(&format!("this is not an {phrase}"), phrase),
            "`{phrase}` denied in the same clause was read as a claim"
        );
    }
    // And the scan found something to look at. A rule that examined no lines
    // would also pass, so the corpus is asserted to be the size it is.
    let scanned = shipped_sources()
        .into_iter()
        .chain(documentation())
        .filter(|(_, text)| text.lines().any(is_about_container_mode))
        .count();
    assert!(
        scanned > 5,
        "only {scanned} files mention containers at all, which is not this \
         repository — the scan is reading the wrong corpus"
    );
}

#[test]
fn the_consent_prompt_for_container_mode_says_limited_isolation() {
    // What a user reads before agreeing. `plain_description` is the only one of
    // these sentences that reaches someone who has not gone looking for it.
    use sure_core::execution::ExecutionMode;
    let prompt = ExecutionMode::Container.plain_description();
    assert!(
        prompt.contains("limited isolation"),
        "the consent prompt must name what it is: {prompt}"
    );
    assert!(
        prompt.contains("not a separate computer"),
        "and it must say what the limit is, not only that there is one: {prompt}"
    );
    for phrase in OVERCLAIMS {
        assert!(
            !overclaims(prompt, phrase),
            "the consent prompt overclaims: {prompt}"
        );
    }
    // The other two modes are untouched by this task, and asserted to be, so
    // that a correction here cannot quietly become a change of what
    // `inspect_only` promises — which is the strongest claim the product makes.
    assert_eq!(
        ExecutionMode::InspectOnly.plain_description(),
        "SURE will read your project's files only. Nothing in your project will be run."
    );
}

#[test]
fn the_modules_own_claim_is_the_same_claim_the_documents_make() {
    let claim = isolation_claim();
    assert!(claim.contains("limited isolation"), "{claim}");
    assert!(
        claim.contains("separate kernel") && claim.contains("not a sandbox"),
        "the claim has to name the wall as well as the limits: {claim}"
    );
    for phrase in OVERCLAIMS {
        assert!(!overclaims(claim, phrase), "{claim}");
    }
}

#[test]
fn a_machine_with_no_container_runtime_is_a_value_and_not_a_failure() {
    // The first acceptance sentence, from outside the crate: *"Docker/Podman
    // absence is nonfatal."* There is no `Result` to propagate and no error
    // variant to match, so the only way to get a runtime out is to handle the
    // absence — which is what makes this a property of the type rather than a
    // convention. What is asserted here is the observable half: a `PATH` holding
    // no runtime yields `Absent`, and the answer says what happens instead.
    //
    // **The assertion that used to be here asked the sentence for a phrase, and
    // the phrase was a lie.** It was
    // `sentence.contains("run on this computer instead")`, under the message
    // *"absence must say what happens instead, not only what is missing"* — and
    // nothing runs on this computer in this build, so the guard required the
    // defect it was written to prevent. It was green. What is asserted now is
    // the *claim*: a sentence that says a check runs here fails, in whatever
    // words it says it, and the sentence has to say what does happen instead —
    // which is that no check runs, here or in a container.
    let absent = Availability::in_path(std::ffi::OsStr::new(""));
    assert_eq!(absent, Availability::Absent);
    assert!(!absent.is_found());
    assert_eq!(absent.runtime(), None);
    let sentence = absent.explain();
    assert!(
        !claims_local_execution(&sentence),
        "the absence claims that a check runs on this computer, and none does: {sentence}"
    );
    assert!(
        denies_that_anything_runs(&sentence),
        "an absence has to say what happens instead of only what is missing, and what happens \
         is that nothing runs: {sentence}"
    );
    assert!(
        sentence.contains("docker") && sentence.contains("podman"),
        "an absence has to say what was looked for, or a user cannot act on it: {sentence}"
    );
    for phrase in OVERCLAIMS {
        assert!(
            !overclaims(&sentence, phrase),
            "absence is not a moment to claim anything: {sentence}"
        );
    }
}

//! `P4-T004`'s acceptance, checked from outside the crate.
//!
//! The task's sentence is one sentence:
//!
//! > *fmt/check/clippy/test evidence binds to current fingerprint.*
//!
//! # The second half is the half a unit test cannot reach
//!
//! *Evidence binds to current fingerprint* is a claim about two values that have
//! to agree: something produced by a check, and the state of a project on disk.
//! The unit tests in `checks/rust.rs` hold the rule against hand-built discovery
//! results and generated identifiers, and a generated identifier is fresh for the
//! result it was pasted into **by construction** — a test written that way passes
//! whether or not the product can do it. So the central test here writes a real
//! project, takes a real [`project_fingerprint`] of it, reads the project again
//! with the product's own discovery, proposes the checks, produces a result at
//! that state, and then **changes a file** and asks whether the evidence is still
//! worth anything. It is not, and the test says which way it is not worth
//! anything.
//!
//! # What this file holds that the unit tests cannot
//!
//! - **The manifest reaches the layer the way a person's does.** `Cargo.toml` as
//!   a file on disk, read by [`discover`], with the tool rows that discovery built
//!   out of `[dependencies]` and the four ways a project asks for `clippy` or
//!   `rustfmt`. The unit tests hand the layer a `RustProject` they built; this
//!   file asks whether anything a person would write ever produces one.
//! - **The identifier is stable across two independent readings**, which is what
//!   makes a result from one run findable in the next.
//! - **The tier the unit tests can most easily get wrong.** A toolchain component,
//!   a lint level and a configuration file are three different files, and only a
//!   real directory can be all three at once.
//!
//! # What it does not hold, and cannot
//!
//! **Nothing here runs `cargo`.** The checks are proposed and gated; the results
//! below are built by this file standing in for the runner, and the day a runner
//! exists it will be a different test that holds its results. What is checked here
//! is the contract a runner will have to satisfy, which is that a result's
//! evidence is bound to the state the result names.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::checks::MissingKind;
use sure_core::checks::rust::RustChecks;
use sure_core::discover::rust::{CommandRole, MANIFEST};
use sure_core::discover::{DiscoverOptions, Discovery, Ecosystem, Findings, discover};
use sure_core::fingerprint::{FingerprintOptions, project_fingerprint};
use sure_core::planned_work::PlannedWork;
use sure_core::schedule::{CheckProposal, CheckReason, CheckSchedule, PlanBuilder};
use sure_domain::evidence::{Freshness, StalenessReason, freshness};
use sure_domain::execution::{
    ActionKind, ExecutionDecision, ExecutionMode, ExecutionPermissions, Permission,
};
use sure_domain::ids::FingerprintId;
use sure_domain::status::{CheckResult, CheckStatus};

/// A scratch project that removes itself.
///
/// Under the workspace's own `target/` so the fixture is on the same volume as the
/// checkout, with a space and a non-ASCII character in the path — the cheapest way
/// to make every path below one two platforms disagree about, which is the
/// discipline `CLAUDE.md` asks for and which `discover_rust.rs` established.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{test}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let project = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("sure 指纹 rust")
            .join(unique);
        std::fs::create_dir_all(&project)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", project.display()));
        Self { project }
    }

    fn path(&self) -> &Path {
        &self.project
    }

    /// Write a file inside the project, creating the directories above it.
    fn write(&self, relative: &str, contents: &str) -> &Self {
        let full = self.project.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        std::fs::write(&full, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        self
    }

    fn discover(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// The Rust findings, or `None` when discovery reported no Rust project.
    fn rust_or_none(&self) -> Option<sure_core::discover::RustProject> {
        let found = self.discover();
        let report = found.report(Ecosystem::Rust)?;
        match &report.findings {
            Findings::Rust(rust) => Some((**rust).clone()),
            other => panic!("the Rust report carried {other:?}"),
        }
    }

    fn rust(&self) -> sure_core::discover::RustProject {
        self.rust_or_none().unwrap_or_else(|| {
            panic!(
                "this fixture is a Rust project and discovery did not report one; it \
                 looked for {:?}",
                self.discover()
                    .looked_for()
                    .iter()
                    .map(|ecosystem| ecosystem.as_str())
                    .collect::<Vec<_>>()
            )
        })
    }

    fn checks(&self) -> RustChecks {
        RustChecks::of(&self.rust(), &self.project)
    }

    /// The project's fingerprint, chosen the way the product chooses it.
    fn fingerprint(&self) -> FingerprintId {
        project_fingerprint(&self.project, &FingerprintOptions::default())
            .unwrap_or_else(|error| panic!("{error}"))
            .id
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        drop(std::fs::remove_dir_all(&self.project));
    }
}

/// A plan built from these checks under a mode and a permission set.
///
/// Refuses to return a plan when anything was refused: `PlanBuilder::propose`
/// remembers a refusal, and a test that read the schedule without reading the
/// refusals would be checking a plan that is quietly missing a check — the exact
/// shape of defect this repository ranks above a visible error.
fn plan(
    checks: &RustChecks,
    mode: ExecutionMode,
    permissions: ExecutionPermissions,
) -> CheckSchedule {
    let mut builder = PlanBuilder::new(mode, permissions);
    checks.add_to(&mut builder);
    assert!(
        builder.refused().is_empty(),
        "the checks layer proposed something the builder refused: {:?}",
        builder.refused()
    );
    builder.build()
}

/// Host-confirmed, with the permission actually granted.
fn host_confirmed() -> ExecutionPermissions {
    ExecutionPermissions {
        run_project_code: true,
        ..ExecutionPermissions::inspect_only()
    }
}

/// The identity this layer gives a role's check in a project read from `Cargo.toml`.
///
/// **The product's own scheme rather than a title match, and the difference is a
/// real one here.** Titles are what a person reads and this module deliberately
/// changes one of them — the format check is *"check that the source is
/// formatted"* and not [`CommandRole::plain_name`]'s *"rewrite the source to a
/// style"*, because it does not rewrite. A test that looked a role up by its
/// `plain_name` would therefore be unable to find the format check at all, which
/// is how this helper came to exist. The identity is derived from the component
/// and the role and from nothing else, which is exactly what makes it the right
/// key: it is the thing that joins a check to the result it produced in a
/// different run.
fn id_for(role: CommandRole) -> sure_domain::ids::CheckId {
    sure_core::checks::check_id(MANIFEST, &format!("rust{}", role.as_str()))
}

/// The role and command of every proposed check.
fn commands(checks: &RustChecks) -> Vec<(CommandRole, String)> {
    checks
        .planned()
        .iter()
        .map(|work| {
            let proposal = work.proposal();
            let role = role_of(proposal.id());
            let command = match proposal.reason() {
                CheckReason::DeclaredCommand { command, .. } => command.clone(),
                other => panic!("{} has the reason {other:?}", proposal.title()),
            };
            (role, command)
        })
        .collect()
}

/// Which role a check identity belongs to.
fn role_of(id: &sure_domain::ids::CheckId) -> CommandRole {
    CommandRole::ALL
        .iter()
        .copied()
        .find(|role| id_for(*role) == *id)
        .unwrap_or_else(|| panic!("{id} is not a role's identity"))
}

/// Why there is no command for a role, or a panic when there is one.
fn gap_kind(checks: &RustChecks, role: CommandRole) -> MissingKind {
    checks
        .missing()
        .iter()
        .find(|missing| *missing.id() == id_for(role))
        .unwrap_or_else(|| panic!("{role:?} has a command, so there is no gap"))
        .kind()
        .clone()
}

/// What one check is worth once it has run, at a named state.
///
/// **This stands in for the runner**, which does not exist yet: the status is
/// always a pass, because what this file is about is the *binding* rather than
/// the verdict, and a pass is the case where a wrong binding costs the most — a
/// green that outlives the state it was earned in is the failure the acceptance
/// sentence exists against.
fn result_of(proposal: &CheckProposal, fingerprint: &FingerprintId) -> CheckResult {
    CheckResult::pass(
        proposal.id().clone(),
        proposal.title().to_owned(),
        proposal.severity(),
        proposal.critical(),
        proposal.evidence_class(),
        fingerprint.clone(),
    )
}

#[test]
fn every_fixture_sits_at_a_path_two_platforms_disagree_about() {
    // The discipline `CLAUDE.md` asks for, held rather than asserted in a
    // comment: a space and a non-ASCII character in every path below, so that a
    // rule which happens to work on this machine's ordinary paths fails here
    // instead of on a user's. If the fixture stopped carrying them, every other
    // test in this file would keep passing and this one would not.
    let fixture = Fixture::new("hostile-path");
    let path = fixture.path().to_string_lossy().into_owned();
    assert!(path.contains(' '), "no space in {path}");
    assert!(
        !path.is_ascii(),
        "no non-ASCII character in {path}, so nothing here tests one"
    );
    assert!(
        path.replace('\\', "/").contains("sure 指纹 rust/"),
        "the fixture is not where this file says it is: {path}"
    );
}

/// A project a person would write, that has asked for both tools by config file.
const COMPLETE: &str = r#"
[package]
name = "demo"
version = "0.1.0"
edition = "2024"

[dependencies]
"#;

/// The same project, with nothing at all said about `clippy` or `rustfmt`.
const BARE: &str = r#"
[package]
name = "demo"
version = "0.1.0"
edition = "2024"
"#;

/// A source file, so the fixture is a project rather than a manifest.
const MAIN: &str = "fn main() {\n    println!(\"demo\");\n}\n";

#[test]
fn a_real_manifest_reaches_the_checks_layer_with_the_four_commands_it_declares() {
    // The path the unit tests in `checks/rust.rs` deliberately skip: a manifest as
    // a file on disk, read by the product's own discovery. The four commands are
    // written out because each one is a decision — `--check` on the first is this
    // task's, `--all-targets` on the middle two is the discovery's and must
    // survive, and the last is `cargo`'s own.
    let fixture = Fixture::new("real-manifest");
    fixture
        .write("Cargo.toml", COMPLETE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("clippy.toml", "")
        .write("rustfmt.toml", "");
    let checks = fixture.checks();

    assert_eq!(checks.component(), Some(MANIFEST));
    assert_eq!(
        commands(&checks),
        vec![
            (CommandRole::Format, "cargo fmt --check".to_owned()),
            (CommandRole::Check, "cargo check --all-targets".to_owned()),
            (CommandRole::Lint, "cargo clippy --all-targets".to_owned()),
            (CommandRole::Test, "cargo test".to_owned()),
        ],
        "a project that configures both tools does not get the four checks the \
         acceptance names"
    );
    assert!(
        checks.missing().is_empty(),
        "nothing is missing in this project: {:?}",
        checks.missing()
    );
}

#[test]
fn the_format_check_sure_would_run_reads_the_tree_and_does_not_write_it() {
    // **The clause the acceptance's second half forces, over a real file.** `cargo
    // fmt` rewrites every file it disagrees with, so a check built from it would
    // change the state it was about: the fingerprint would be taken before the run
    // and would describe a tree that no longer exists afterwards. The command and
    // the title are held together here because they are built in two places — the
    // command from the discovery's row plus a flag, the title from this task's own
    // table — and a user reads the title before allowing the command to run.
    let fixture = Fixture::new("read-only-format");
    fixture
        .write("Cargo.toml", COMPLETE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("rustfmt.toml", "");
    let checks = fixture.checks();

    let format = checks
        .planned()
        .iter()
        .map(PlannedWork::proposal)
        .find(|proposal| *proposal.id() == id_for(CommandRole::Format))
        .expect("a project with a rustfmt config gets a format check");
    match format.reason() {
        CheckReason::DeclaredCommand { command, .. } => {
            assert_eq!(command, "cargo fmt --check");
            assert_ne!(
                command, "cargo fmt",
                "the check would rewrite the project it is reporting on"
            );
        }
        other => panic!("the format check's reason is {other:?}"),
    }
    assert!(
        !format.title().contains("rewrite"),
        "the title says SURE will rewrite the user's files while the command reads \
         them: {}",
        format.title()
    );
    assert!(
        !format.requirements().can_modify_disk(),
        "a check that can change the project is not a check"
    );

    // And the discovery's own answer for the role is still the writing form, so
    // this is the checks layer's decision rather than the discovery having been
    // changed underneath it. Stated as a value, so that a change to the discovery
    // cannot quietly make the flag unnecessary and leave this test passing for the
    // wrong reason.
    let planned = fixture.rust().conventional_commands();
    let row = planned
        .iter()
        .find(|row| row.role == CommandRole::Format)
        .expect("every role has a row");
    assert_eq!(
        row.command.as_deref(),
        Some("cargo fmt"),
        "the plan for the formatting role is the command a person types"
    );
}

#[test]
fn a_project_that_names_neither_tool_gets_two_checks_and_two_gaps() {
    // The half of the layer that is about absences, from the outside. `cargo
    // check` and `cargo test` are `cargo`'s own, so a project with a readable
    // manifest has them; `clippy` and `rustfmt` are separate installs, so a project
    // that never named one gets a sentence rather than a check.
    let fixture = Fixture::new("bare");
    fixture
        .write("Cargo.toml", BARE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN);
    let checks = fixture.checks();

    assert_eq!(
        commands(&checks),
        vec![
            (CommandRole::Check, "cargo check --all-targets".to_owned()),
            (CommandRole::Test, "cargo test".to_owned()),
        ]
    );
    assert_eq!(checks.missing().len(), 2, "{:?}", checks.missing());
    assert!(!checks.is_empty(), "the gaps are what there is to say");

    for role in [CommandRole::Format, CommandRole::Lint] {
        assert_eq!(
            gap_kind(&checks, role),
            MissingKind::NotDeclared,
            "{role:?}"
        );
    }

    // And neither gap holds the run out of green: a project without a linter is
    // not a project SURE should refuse to call green. The fingerprint is generated
    // because this is a claim about the classification rather than about a state.
    for result in checks.not_checked(&FingerprintId::generate()) {
        assert_eq!(result.status, CheckStatus::Skipped, "{}", result.title);
        assert!(!result.blocks_green(), "{}", result.reason);
    }
}

#[test]
fn the_toolchain_file_names_a_tool_as_a_configuration_file_does() {
    // The middle tier, over a real `rust-toolchain.toml`: a project that names
    // `clippy` and `rustfmt` as components and has no configuration file for either
    // has asked for both, and the discovery is where that was decided.
    let fixture = Fixture::new("toolchain-components");
    fixture
        .write("Cargo.toml", BARE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write(
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"stable\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
        );
    let checks = fixture.checks();

    assert!(checks.missing().is_empty(), "{:?}", checks.missing());
    assert_eq!(
        commands(&checks),
        vec![
            (CommandRole::Format, "cargo fmt --check".to_owned()),
            (CommandRole::Check, "cargo check --all-targets".to_owned()),
            (CommandRole::Lint, "cargo clippy --all-targets".to_owned()),
            (CommandRole::Test, "cargo test".to_owned()),
        ]
    );

    // The evidence for it survives into the discovery's row, which is where a
    // reader who does not find the tool named in the root manifest goes next.
    let planned = fixture.rust().conventional_commands();
    let lint = planned
        .iter()
        .find(|row| row.role == CommandRole::Lint)
        .expect("every role has a row");
    assert_eq!(lint.command.as_deref(), Some("cargo clippy --all-targets"));
    assert!(
        lint.because
            .iter()
            .any(|source| source.path.to_string_lossy().contains("rust-toolchain")),
        "the row does not say where the tool was asked for: {:?}",
        lint.because
    );
}

#[test]
fn a_lint_level_set_in_the_manifest_names_the_linter_as_surely_as_a_config_file_does() {
    // The third tier, and the one a table written against two tiers would miss:
    // `[lints.clippy]` is a project deciding what its linter should complain about,
    // which is a project that runs its linter. `[lints.rust]` is not that — it is
    // the compiler's own lints, which `cargo check` reports — so this fixture sets
    // clippy's and not rust's, and asks for a lint check and a format gap.
    let fixture = Fixture::new("lint-levels");
    fixture
        .write(
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [lints.clippy]\nunwrap_used = \"deny\"\n",
        )
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN);
    let checks = fixture.checks();

    assert_eq!(
        commands(&checks),
        vec![
            (CommandRole::Check, "cargo check --all-targets".to_owned()),
            (CommandRole::Lint, "cargo clippy --all-targets".to_owned()),
            (CommandRole::Test, "cargo test".to_owned()),
        ],
        "a project that sets clippy's lint levels has asked for clippy"
    );
    assert_eq!(
        gap_kind(&checks, CommandRole::Format),
        MissingKind::NotDeclared
    );
}

#[test]
fn an_unreadable_toolchain_is_not_a_project_that_asked_for_nothing() {
    // **The false statement this distinction exists to refuse, from the outside.**
    // `rust-toolchain.toml` is one of the three ways a project asks for `clippy`, so
    // a project whose toolchain file SURE could not parse is a project SURE cannot
    // say asked for nothing. Reporting it as "you declare no linter" would be a
    // claim about a file made from a failure to read it.
    let fixture = Fixture::new("unreadable-toolchain");
    fixture
        .write("Cargo.toml", BARE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("rust-toolchain.toml", "this is not TOML = = =\n");
    let checks = fixture.checks();

    for role in [CommandRole::Format, CommandRole::Lint] {
        assert_eq!(
            gap_kind(&checks, role),
            MissingKind::NotReadable,
            "{role:?}"
        );
    }

    // And it does not spread to the two roles whose tool is `cargo`: those have
    // their commands, and a version of this that reported every role as unreadable
    // would be the same defect in the other direction.
    assert_eq!(checks.missing().len(), 2, "{:?}", checks.missing());
    assert_eq!(
        commands(&checks),
        vec![
            (CommandRole::Check, "cargo check --all-targets".to_owned()),
            (CommandRole::Test, "cargo test".to_owned()),
        ]
    );
}

#[test]
fn a_manifest_that_is_not_there_is_not_a_project_that_declares_nothing() {
    // The distinction the whole layer rests on, from the outside: a directory with a
    // `Cargo.toml` that is not a manifest at all. Four "you declare nothing" rows
    // here would be four false statements about a file SURE could not read — and two
    // of them would be about `cargo check` and `cargo test`, which would run
    // perfectly well.
    let fixture = Fixture::new("unreadable-manifest");
    fixture
        .write("Cargo.toml", "this is not TOML = = =\n")
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN);
    let checks = fixture.checks();

    assert!(checks.is_empty(), "{checks:?}");
    assert!(checks.component().is_none());
    assert!(checks.not_checked(&FingerprintId::generate()).is_empty());

    // And the discovery still reported a project, so the emptiness above is a
    // statement about the manifest rather than about discovery finding nothing.
    assert!(
        fixture.rust_or_none().is_some(),
        "the fixture must still be a Rust project for this test to mean anything"
    );
}

#[test]
fn the_evidence_of_the_four_checks_is_fresh_for_the_state_it_ran_against() {
    // **The acceptance sentence, end to end and over a real project state.** The
    // fingerprint is taken by the product's own chooser from the files on disk; the
    // checks come from the product's own discovery of those files; and the evidence
    // is asked whether it is worth anything about that state.
    //
    // Every one of the four is held, and the count is asserted, because a sweep
    // that visited no checks would pass every assertion in the loop.
    let fixture = Fixture::new("fresh");
    fixture
        .write("Cargo.toml", COMPLETE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("clippy.toml", "")
        .write("rustfmt.toml", "");

    let checks = fixture.checks();
    let state = fixture.fingerprint();

    let mut examined = 0;
    for work in checks.planned() {
        let proposal = work.proposal();
        examined += 1;
        let result = result_of(proposal, &state);

        // A result that exists has a state, and the evidence takes it from there
        // rather than from this test: the anchor is the proposal's own reason's.
        let anchor = proposal
            .reason()
            .anchor()
            .unwrap_or_else(|| panic!("{} names no location", proposal.title()));
        let evidence = sure_core::checks::evidence_of(&result, anchor)
            .unwrap_or_else(|| panic!("{} established nothing", proposal.title()));

        assert_eq!(
            freshness(&evidence, &state),
            Freshness::Fresh,
            "{} is not bound to the state it ran against",
            proposal.title()
        );
        assert_eq!(evidence.class, proposal.evidence_class());
        assert_eq!(evidence.severity, proposal.severity());
        assert!(
            evidence.summary.contains(proposal.title()),
            "{} does not say what it is about",
            evidence.summary
        );
        // The anchor points at a file this project has, rather than at a path
        // SURE made up: a check whose evidence cannot be checked is not evidence.
        assert!(
            fixture.path().join(&evidence.anchor.location).exists(),
            "{} anchors at {}, which is not in the project",
            proposal.title(),
            evidence.anchor.location
        );
        assert!(evidence.anchor.is_checkable());
    }
    assert_eq!(examined, 4, "the sweep did not visit the four checks");
}

#[test]
fn the_same_evidence_is_stale_once_the_project_moves_on() {
    // **The failure the acceptance sentence exists against, and the reason the
    // first half of it is not enough.** A green that stays green after the code
    // under it changes is worse than a red: it is a check that has stopped looking
    // while continuing to report. So this takes the evidence from a state, changes
    // one file, and asks again.
    let fixture = Fixture::new("stale");
    fixture
        .write("Cargo.toml", COMPLETE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("clippy.toml", "")
        .write("rustfmt.toml", "");

    let checks = fixture.checks();
    let before = fixture.fingerprint();
    let proposal = checks
        .planned()
        .iter()
        .map(PlannedWork::proposal)
        .find(|proposal| *proposal.id() == id_for(CommandRole::Test))
        .expect("a project with a readable manifest gets a test check");
    let evidence = sure_core::checks::evidence_of(
        &result_of(proposal, &before),
        proposal.reason().anchor().expect("a declared command"),
    )
    .expect("a passing check established something");

    assert_eq!(freshness(&evidence, &before), Freshness::Fresh);

    // The change is to the source, which is the file `cargo test` is about: a
    // fixture that changed something the checks do not read would make the
    // assertion below pass for a reason that has nothing to do with the rule.
    fixture.write(
        "src/main.rs",
        "fn main() {\n    println!(\"changed\");\n}\n",
    );
    let after = fixture.fingerprint();

    // The premise first, stated as a value rather than assumed: if the two states
    // were the same state, the assertion below would be about nothing. This is the
    // lesson `fingerprint_content.rs` records about a fixture that could not reach
    // the case its own name claimed.
    assert_ne!(
        before, after,
        "changing a source file did not move the project's fingerprint, so the \
         staleness below would be about nothing"
    );

    assert_eq!(
        freshness(&evidence, &after),
        Freshness::Stale(StalenessReason::FingerprintChanged),
        "the evidence of a check outlived the state it was earned in"
    );

    // And the other direction, which is the one a wrong implementation gets wrong
    // silently: the evidence is *not* stale against the state it was earned in, and
    // it is not evidence with no provenance — which would be stale against
    // everything and would read as caution rather than as a defect.
    assert_eq!(freshness(&evidence, &before), Freshness::Fresh);
    assert!(
        evidence.fingerprint.is_some(),
        "evidence with no state attached can never support anything"
    );
}

#[test]
fn a_check_that_could_not_be_proposed_produces_no_evidence_about_anything() {
    // The other end of the same rule, over a real project: a gap becomes a skipped
    // result, and a skipped result has no evidence to give — so a tool the project
    // never asked for cannot reach a report as a green. Both halves are held at once
    // here because the join is the thing that matters: either one alone leaves a
    // path from "not run" to "fine".
    let fixture = Fixture::new("no-evidence");
    fixture
        .write("Cargo.toml", BARE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN);
    let checks = fixture.checks();
    let state = fixture.fingerprint();

    let results = checks.not_checked(&state);
    assert_eq!(results.len(), 2, "{:?}", checks.missing());

    // **One anchor for both, and the anchor is not the variable.** A skipped result
    // answers `None` whatever it is pointed at, which is the property being
    // asserted: it is not that SURE failed to find a location for this evidence, it
    // is that there is no evidence to locate.
    let anchor = sure_domain::evidence::EvidenceAnchor::new(
        sure_domain::evidence::AnchorSubject::File,
        MANIFEST.to_owned(),
        "the manifest this project was read from",
    );

    for result in &results {
        assert_eq!(result.status, CheckStatus::Skipped, "{}", result.title);
        assert!(result.is_not_checked(), "{}", result.title);
        assert_eq!(result.project_fingerprint, state);
        assert_eq!(
            sure_core::checks::evidence_of(result, anchor.clone()),
            None,
            "{} reached a report as evidence",
            result.title
        );
    }
}

#[test]
fn nothing_the_plan_holds_needs_a_permission_beyond_running_project_code() {
    // The clauses of the sibling tasks that this one must not break: no check
    // installs, none writes to the disk, and every one of them is gated by the
    // execution mode rather than by a permission a user might already have granted.
    let fixture = Fixture::new("permissions");
    fixture
        .write("Cargo.toml", COMPLETE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("clippy.toml", "")
        .write("rustfmt.toml", "");
    let checks = fixture.checks();

    let mut seen = 0;
    for work in checks.planned() {
        let proposal = work.proposal();
        seen += 1;
        for action in proposal.requirements().actions() {
            assert_ne!(
                *action,
                ActionKind::InstallDependencies,
                "{} would install packages",
                proposal.title()
            );
        }
        assert!(
            !proposal.requirements().can_modify_disk(),
            "{} would change the project it is reporting on",
            proposal.title()
        );
        assert!(
            proposal.requirements().runs_project_code(),
            "{} would not be gated by the execution mode at all",
            proposal.title()
        );
        assert_eq!(
            proposal
                .requirements()
                .blocked_by(&ExecutionPermissions::inspect_only()),
            Some(Permission::RunProjectCode),
            "{} is stopped by something other than running project code",
            proposal.title()
        );
    }
    assert_eq!(seen, 4, "the sweep did not visit the four checks");

    // And the mode, not the permission, is what refuses the middle case: a user who
    // has already granted the permission is told something true about what is
    // stopping them.
    let denied = plan(
        &checks,
        ExecutionMode::InspectOnly,
        ExecutionPermissions::inspect_only(),
    );
    assert_eq!(denied.may_run().count(), 0);
    for entry in denied.checks() {
        assert_eq!(entry.decision(), ExecutionDecision::Denied);
        assert_eq!(entry.blocked_by(), Some(Permission::RunProjectCode));
    }

    let consented = plan(&checks, ExecutionMode::InspectOnly, host_confirmed());
    assert_eq!(consented.may_run().count(), 0);
    for entry in consented.checks() {
        assert_eq!(entry.decision(), ExecutionDecision::NeedsConsent);
        assert_eq!(
            entry.blocked_by(),
            None,
            "the permission is granted; the mode is what refuses"
        );
    }

    let allowed = plan(&checks, ExecutionMode::HostConfirmed, host_confirmed());
    assert_eq!(allowed.may_run().count(), allowed.len());
    assert_eq!(allowed.len(), 4);
}

#[test]
fn two_independent_readings_of_one_project_give_the_same_identifiers() {
    // The identifiers are derived from the component and the role and from nothing
    // else, so they survive a second discovery — which is what makes a result
    // comparable against the run before it. A report whose check ids moved between
    // runs would make every comparison a difference.
    let fixture = Fixture::new("stable-ids");
    fixture
        .write("Cargo.toml", COMPLETE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("clippy.toml", "")
        .write("rustfmt.toml", "");
    let first = fixture.checks();
    let second = fixture.checks();

    let ids = |checks: &RustChecks| -> Vec<String> {
        checks
            .planned()
            .iter()
            .map(|work| work.proposal().id().as_str().to_owned())
            .collect()
    };
    assert_eq!(ids(&first), ids(&second));
    assert_eq!(ids(&first).len(), 4);
    assert_eq!(
        ids(&first)
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4,
        "two roles share an identifier: {:?}",
        ids(&first)
    );

    // And they are not the identifiers of the same roles in another ecosystem,
    // which is the half the component in the digest buys: `package.json`'s test
    // check and `Cargo.toml`'s must not be one check.
    for work in first.planned() {
        let proposal = work.proposal();
        assert!(
            proposal.id().as_str().contains("rust"),
            "{} is not identifiable as a Rust check: {}",
            proposal.title(),
            proposal.id()
        );
    }
}

#[test]
fn the_command_a_check_names_is_the_line_sure_would_run() {
    // The sentence a person reads, over a real file. `cargo clippy --all-targets`
    // comes from the discovery's own row and is not a line the project wrote, and
    // the reason carries the row's own command rather than a second copy of it.
    let fixture = Fixture::new("reason");
    fixture
        .write("Cargo.toml", COMPLETE)
        .write("Cargo.lock", "")
        .write("src/main.rs", MAIN)
        .write("clippy.toml", "")
        .write("rustfmt.toml", "");
    let checks = fixture.checks();
    let planned = fixture.rust().conventional_commands();

    for work in checks.planned() {
        let proposal = work.proposal();
        match proposal.reason() {
            CheckReason::DeclaredCommand {
                declared_in,
                command,
            } => {
                assert_eq!(declared_in, MANIFEST);
                // The discovery's own row for the same role, found by the identity
                // rather than by position, so a role added to `CommandRole::ALL`
                // cannot make this compare one role's command against its
                // neighbour's.
                let role = role_of(proposal.id());
                let row = planned
                    .iter()
                    .find(|row| row.role == role)
                    .expect("every role has a row");
                assert!(
                    row.command.as_deref().is_some_and(|planned| {
                        command == planned || command.starts_with(planned)
                    }),
                    "{} runs {command}, which is not the discovery's {}",
                    proposal.title(),
                    row.command.as_deref().unwrap_or("nothing")
                );
            }
            other => panic!("{} has the reason {other:?}", proposal.title()),
        }
        assert!(
            proposal.reason().names_something(),
            "a proposal whose reason names nothing is refused by the builder"
        );
    }
}

//! `P4-T002`'s acceptance, checked from outside the crate.
//!
//! The task's sentence is two sentences:
//!
//! > *Declared build/lint/type/test checks run only under allowed execution mode.
//! > Missing commands are not passes.*
//!
//! Both are about the seam rather than about a table, and both are checked here
//! over **real directories** rather than over hand-built values: a `package.json`
//! on disk, the product's own discovery reading it, and the checks layer standing
//! on what discovery found. The unit tests in `checks/node.rs` hold the rules;
//! this file holds that the rules are reachable — that the accessors are public,
//! that the identifiers are stable across two independent readings of one
//! project, and that nothing in the path can turn a command the project does not
//! have into something a reader would take for a pass.
//!
//! # The three things this file is arranged against
//!
//! **A check that runs a project's own code is gated by the mode, and the
//! permission is not the mode.** Three situations produce three different
//! answers, and a test that checked only the first two would not tell them apart:
//! inspect-only permissions under inspect-only mode is `Denied`, the *permission*
//! being granted under inspect-only mode is `NeedsConsent` — the mode is what
//! refuses, and a user who has already agreed to run project code is told
//! something true about what is stopping them — and a host-confirmed run with the
//! permission is `Allowed`.
//!
//! **A missing command is a row, not a silence.** The failure this guards is not
//! a wrong verdict; it is a report that says nothing at all about tests in a
//! project that has no test script, which a reader takes for the absence of a
//! problem. So the acceptance is checked as a *cover*: every role the acceptance
//! names produces either a plan entry or a skipped result, and the two lists
//! together account for all four.
//!
//! **The sentence a person reads is the true one.** The command in a check's
//! reason is the line SURE would run — `yarn run build` — and not the text the
//! project wrote in its script, which is `tsc -b`. Those are different strings and
//! the first draft of the reason's own documentation said the field held the
//! second.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::checks::MissingKind;
use sure_core::checks::node::NodeChecks;
use sure_core::discover::node::{MANIFEST, ScriptRole};
use sure_core::discover::{DiscoverOptions, Discovery, Ecosystem, Findings, discover};
use sure_core::planned_work::PlannedWork;
use sure_core::schedule::{CheckProposal, CheckReason, CheckSchedule, PlanBuilder};
use sure_domain::execution::{ActionKind, ExecutionDecision, ExecutionMode, ExecutionPermissions};
use sure_domain::ids::FingerprintId;
use sure_domain::status::{CheckResult, CheckStatus, NotCheckedReason};

/// A scratch project that removes itself.
///
/// Under the workspace's own `target/` so the fixture is on the same volume as the
/// checkout, with a space and a non-ASCII character in the path — the cheapest way
/// to make every path below one two platforms disagree about, which is the
/// discipline `CLAUDE.md` asks for and which `discover_node.rs` established.
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
            .join("sure 指纹 checks")
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

    /// The Node findings, or a panic naming what was found instead.
    ///
    /// `None` when discovery reported no Node project at all, which one test
    /// below needs and the others must not accept: a fixture that quietly stopped
    /// being a Node project would make every assertion after it vacuous.
    fn node_or_none(&self) -> Option<sure_core::discover::NodeProject> {
        let found = self.discover();
        let report = found.report(Ecosystem::Node)?;
        match &report.findings {
            Findings::Node(node) => Some((**node).clone()),
            other => panic!("the Node report carried {other:?}"),
        }
    }

    fn node(&self) -> sure_core::discover::NodeProject {
        self.node_or_none().unwrap_or_else(|| {
            panic!(
                "this fixture is a Node project and discovery did not report one; \
                 it looked for {:?}",
                self.discover()
                    .looked_for()
                    .iter()
                    .map(|ecosystem| ecosystem.as_str())
                    .collect::<Vec<_>>()
            )
        })
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
    checks: &NodeChecks,
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

/// Inspect-only permissions, with project code allowed by the *permission*.
///
/// The combination a permission-only reading of the first acceptance sentence
/// would call runnable, and which the mode still refuses.
fn inspect_only_but_permitted() -> ExecutionPermissions {
    ExecutionPermissions {
        run_project_code: true,
        ..ExecutionPermissions::inspect_only()
    }
}

/// Host-confirmed, with the permission actually granted.
fn host_confirmed() -> ExecutionPermissions {
    ExecutionPermissions {
        run_project_code: true,
        ..ExecutionPermissions::inspect_only()
    }
}

/// The proposals alone, for the assertions that are about a proposal.
///
/// Since `P18-T003` a check enters the plan as one value holding its proposal and
/// the operation that would carry it out, so a reader that wants the proposal asks
/// the value for it rather than reading a second list that could be paired wrong.
fn proposals(checks: &NodeChecks) -> Vec<&CheckProposal> {
    checks.planned().iter().map(PlannedWork::proposal).collect()
}

/// Every title either list accounts for, sorted.
///
/// The cover statement, in one function: a role that appears in neither list is a
/// role the report says nothing about.
fn titles(checks: &NodeChecks) -> Vec<String> {
    let mut titles: Vec<String> = checks
        .planned()
        .iter()
        .map(|work| work.proposal().title().to_owned())
        .chain(
            checks
                .missing()
                .iter()
                .map(|missing| missing.title().to_owned()),
        )
        .collect();
    titles.sort();
    titles
}

/// The four titles the acceptance names, for a root manifest.
fn the_four_titles() -> Vec<String> {
    let mut roles = vec![
        ScriptRole::Build,
        ScriptRole::Test,
        ScriptRole::Lint,
        ScriptRole::TypeCheck,
    ];
    roles.sort_by_key(|role| role.plain_description());
    roles
        .into_iter()
        .map(|role| role.plain_description().to_owned())
        .collect()
}

/// The kind of the gap for a role, or a panic naming what happened instead.
fn gap(checks: &NodeChecks, role: ScriptRole) -> &MissingKind {
    checks
        .missing()
        .iter()
        .find(|missing| missing.title() == role.plain_description())
        .unwrap_or_else(|| panic!("{role:?} has a command, so there is no gap to read"))
        .kind()
}

#[test]
fn a_declared_check_runs_only_under_a_mode_that_allows_it() {
    let fixture = Fixture::new("gating");
    fixture.write(
        MANIFEST,
        r#"{"name":"gating","scripts":{"build":"tsc -b","test":"vitest run"}}"#,
    );
    fixture.write("package-lock.json", "{}\n");

    let checks = NodeChecks::of(&fixture.node(), fixture.path());
    assert_eq!(proposals(&checks).len(), 2, "build and test are declared");

    // One: the permission is not granted, so `decide` answers `Denied` before the
    // mode is consulted at all.
    let denied = plan(
        &checks,
        ExecutionMode::InspectOnly,
        ExecutionPermissions::inspect_only(),
    );
    assert_eq!(denied.len(), proposals(&checks).len());
    assert_eq!(denied.may_run().count(), 0);
    assert_eq!(denied.blocked().count(), denied.len());
    for entry in denied.checks() {
        assert_eq!(
            entry.decision(),
            ExecutionDecision::Denied,
            "{}",
            entry.proposal().title()
        );
        assert_eq!(
            entry.blocked_by(),
            Some(sure_domain::execution::Permission::RunProjectCode),
            "{} is denied and does not say which permission would change it",
            entry.proposal().title()
        );
    }

    // Two: **the mode is what refuses.** Every permission this check needs has
    // been granted, and inspect-only still will not run project code — so the
    // answer is `NeedsConsent` and not `Denied`, and the entry names no missing
    // permission. A sentence that named one here would be asking the user to
    // grant something they have already granted.
    let refuses = plan(
        &checks,
        ExecutionMode::InspectOnly,
        inspect_only_but_permitted(),
    );
    assert_eq!(refuses.may_run().count(), 0);
    for entry in refuses.checks() {
        assert_eq!(
            entry.decision(),
            ExecutionDecision::NeedsConsent,
            "{} under inspect-only with its permission granted",
            entry.proposal().title()
        );
        assert_eq!(
            entry.blocked_by(),
            None,
            "{} names a permission that is not the obstacle",
            entry.proposal().title()
        );
        assert!(
            entry.plain_description().contains("only if you agree"),
            "{} does not say the run is what is being asked for: {}",
            entry.proposal().title(),
            entry.plain_description()
        );
    }

    // And a check that cannot run produces a skipped result and never a pass.
    let fingerprint = FingerprintId::generate();
    let unrun: Vec<CheckResult> = refuses
        .checks()
        .iter()
        .filter_map(|entry| entry.not_run(&fingerprint))
        .collect();
    assert_eq!(unrun.len(), refuses.len());
    for result in &unrun {
        assert_eq!(result.status, CheckStatus::Skipped, "{}", result.title);
        assert!(!result.status.is_green(), "{}", result.title);
        assert!(!result.status.produced_a_result(), "{}", result.title);
        assert_eq!(
            result.not_checked_reason,
            Some(NotCheckedReason::ExecutionNotAuthorized),
            "{} was not run for a different stated reason",
            result.title
        );
    }

    // Three: allowed, and then there is no result to produce at all — a schedule
    // entry that would run cannot be turned into a verdict by this path.
    let allowed = plan(&checks, ExecutionMode::HostConfirmed, host_confirmed());
    assert_eq!(allowed.may_run().count(), allowed.len());
    for entry in allowed.checks() {
        assert!(
            entry.not_run(&fingerprint).is_none(),
            "{} would run and a skipped result was produced for it anyway",
            entry.proposal().title()
        );
        assert!(entry.proposal().requirements().runs_project_code());
    }

    // The ordering is the same in all three, which is what makes the decisions
    // comparable: nothing moved because the mode changed.
    let order = |schedule: &CheckSchedule| -> Vec<String> {
        schedule
            .checks()
            .iter()
            .map(|entry| entry.proposal().id().as_str().to_owned())
            .collect()
    };
    assert_eq!(order(&denied), order(&refuses));
    assert_eq!(order(&refuses), order(&allowed));
}

#[test]
fn every_role_the_acceptance_names_is_either_a_check_or_a_skipped_result() {
    // The second acceptance sentence, as a cover: three of the four roles have no
    // command in this project, and **not one of them is missing from the report**.
    let fixture = Fixture::new("cover");
    fixture.write(
        MANIFEST,
        r#"{"name":"cover","scripts":{"build":"tsc -b","test":["jest"],"lint":null}}"#,
    );
    fixture.write("pnpm-lock.yaml", "");

    let checks = NodeChecks::of(&fixture.node(), fixture.path());
    assert_eq!(
        titles(&checks),
        the_four_titles(),
        "a role the acceptance names is in neither the plan nor the gaps"
    );

    assert_eq!(proposals(&checks).len(), 1, "only build has a command");
    let results = checks.not_checked(&FingerprintId::generate());
    assert_eq!(results.len(), 3);
    for result in &results {
        assert_eq!(result.status, CheckStatus::Skipped, "{}", result.title);
        assert!(!result.status.is_green(), "{}", result.title);
        assert!(!result.status.produced_a_result(), "{}", result.title);
        assert!(
            !result.reason.trim().is_empty(),
            "{} is skipped and says nothing about why",
            result.title
        );
    }

    // The two gaps are two different findings and are not rendered alike. A
    // `"test": ["jest"]` is a name that is there with a value npm refuses, and a
    // project with no `typecheck` script has not written one — reporting both as
    // "no such script" would describe a broken manifest as a project that never
    // wrote one.
    assert_eq!(gap(&checks, ScriptRole::Test), &MissingKind::NotACommand);
    assert_eq!(gap(&checks, ScriptRole::Lint), &MissingKind::NotACommand);
    assert_eq!(
        gap(&checks, ScriptRole::TypeCheck),
        &MissingKind::NotDeclared
    );
    let test = results
        .iter()
        .find(|result| result.title == ScriptRole::Test.plain_description())
        .expect("the test role has a gap");
    let typecheck = results
        .iter()
        .find(|result| result.title == ScriptRole::TypeCheck.plain_description())
        .expect("the typecheck role has a gap");
    assert_ne!(test.reason, typecheck.reason);

    // And the critical one — the test suite — holds the run out of green, while
    // the two that are not critical do not. `blocks_green` is the domain's rule
    // applied to these values rather than one restated here.
    let blocking: Vec<&str> = results
        .iter()
        .filter(|result| result.blocks_green())
        .map(|result| result.title.as_str())
        .collect();
    assert_eq!(blocking, vec![ScriptRole::Test.plain_description()]);
}

#[test]
fn a_project_that_declares_nothing_is_not_a_project_that_fails() {
    // The other side of the same rule, and it has to be checked or the one above
    // would be satisfied by a design that calls every absence a defect. A project
    // with no `test` script at all is a project this check does not apply to, so
    // its gap does **not** hold the run out of green — while the *same* role,
    // *same* severity and *same* criticality in the test above does. The only
    // difference between the two fixtures is a script name whose value npm
    // refuses.
    let fixture = Fixture::new("declares-nothing");
    fixture.write(MANIFEST, r#"{"name":"bare","scripts":{"build":"tsc -b"}}"#);
    fixture.write("package-lock.json", "{}\n");

    let checks = NodeChecks::of(&fixture.node(), fixture.path());
    assert_eq!(gap(&checks, ScriptRole::Test), &MissingKind::NotDeclared);

    let results = checks.not_checked(&FingerprintId::generate());
    assert_eq!(results.len(), 3);
    assert!(
        results.iter().all(|result| !result.blocks_green()),
        "a check that does not apply to this project is holding the run out of \
         green, which is the false alarm this side of the rule prevents"
    );
}

#[test]
fn the_reason_names_the_command_sure_would_run_and_not_the_script_text() {
    // `"build": "tsc -b"` declares a script. What a person is asked to allow is
    // the command that starts it, and the two strings are different. The first
    // draft of `CheckReason::DeclaredCommand`'s own documentation said the field
    // held the script's text.
    let fixture = Fixture::new("reason");
    fixture.write(
        MANIFEST,
        r#"{"name":"reason","scripts":{"build":"tsc -b"}}"#,
    );
    fixture.write("yarn.lock", "");

    let checks = NodeChecks::of(&fixture.node(), fixture.path());
    let build = proposals(&checks)
        .into_iter()
        .find(|proposal| proposal.title() == ScriptRole::Build.plain_description())
        .expect("build is declared, so it is checked");

    match build.reason() {
        CheckReason::DeclaredCommand {
            declared_in,
            command,
        } => {
            assert_eq!(declared_in, MANIFEST);
            assert_eq!(command, "yarn run build");
            assert_ne!(
                command, "tsc -b",
                "the reason is showing the script's text where it must show the \
                 command SURE would run"
            );
            // And the sentence a person reads carries both halves.
            let sentence = build.reason().plain_description();
            assert!(sentence.contains("yarn run build"), "{sentence}");
            assert!(sentence.contains(MANIFEST), "{sentence}");
            assert!(
                !sentence.contains("tsc -b"),
                "the sentence names a command line that is in no file SURE runs: \
                 {sentence}"
            );
        }
        other => panic!("the build check's reason is {other:?}"),
    }

    // The check runs the project's own code and nothing else: one action, and it
    // is the one the reason describes.
    assert_eq!(build.requirements().actions(), [ActionKind::Build]);
    assert!(build.requirements().runs_project_code());
    assert!(!build.requirements().can_touch_network());
    assert_eq!(
        build.requirements().permissions_needed(),
        [sure_domain::execution::Permission::RunProjectCode]
    );
}

#[test]
fn the_manifest_path_is_the_one_discovery_reads_and_the_one_a_check_names() {
    // `MANIFEST` is public because this layer has to spell the path itself, and
    // the way that goes wrong is that the two spellings stop naming one file. So
    // the premise is asserted with the filesystem rather than with the constant:
    // the bytes at `MANIFEST` are the ones discovery reports, and the same bytes
    // at another name are not read at all.
    let fixture = Fixture::new("manifest-path");
    fixture.write(
        MANIFEST,
        r#"{"name":"real","scripts":{"build":"only-at-this-name"}}"#,
    );
    fixture.write(
        "package.json.backup",
        r#"{"name":"decoy","scripts":{"build":"not-the-manifest"}}"#,
    );
    // A runner, so that the declared script really becomes a check rather than a
    // gap: a project that declares a command and cannot say what runs it is the
    // case `Runner` exists for, and it is checked where it belongs.
    fixture.write("yarn.lock", "");

    let project = fixture.node();
    let package = project
        .package()
        .expect("the manifest is there and readable");
    assert_eq!(package.name.as_deref(), Some("real"));
    assert_eq!(
        package
            .script(ScriptRole::Build)
            .map(|script| script.command.as_str()),
        Some("only-at-this-name")
    );

    // And the check layer's component is that same path, so a result can be
    // joined to the file it is about.
    let checks = NodeChecks::of(&project, fixture.path());
    match proposals(&checks)[0].reason() {
        CheckReason::DeclaredCommand { declared_in, .. } => {
            assert_eq!(declared_in, MANIFEST);
            assert!(
                fixture.path().join(declared_in).is_file(),
                "the reason names {declared_in}, and there is no such file in the \
                 project the check is about"
            );
        }
        other => panic!("the build check's reason is {other:?}"),
    }
    for missing in checks.missing() {
        assert_eq!(missing.component(), MANIFEST);
    }
}

#[test]
fn a_manifest_sure_did_not_read_is_not_a_manifest_that_declares_nothing() {
    // The false green this whole path is arranged against. A `package.json` that
    // failed to parse declares no scripts — and so does a project with no
    // `package.json`, and so does a project whose manifest SURE never reached.
    // Four "your project declares nothing" rows about a file SURE did not read
    // would be four false statements about the project.
    let broken = Fixture::new("broken-manifest");
    broken.write(MANIFEST, "{ this is not json");
    let broken_project = broken.node();
    assert!(
        broken_project.package().is_none(),
        "the fixture's manifest is not JSON and discovery read it anyway"
    );
    let checks = NodeChecks::of(&broken_project, broken.path());
    assert!(
        checks.is_empty(),
        "a manifest SURE could not read produced checks: {checks:?}"
    );
    assert!(checks.not_checked(&FingerprintId::generate()).is_empty());

    // A lockfile and no manifest at all is a Node project SURE can see and
    // cannot read, which is the same answer for a different reason.
    let no_manifest = Fixture::new("no-manifest");
    no_manifest.write("package-lock.json", "{}\n");
    let no_manifest_project = no_manifest.node();
    assert!(no_manifest_project.package().is_none());
    assert!(NodeChecks::of(&no_manifest_project, no_manifest.path()).is_empty());

    // And the control: the same project with the same lockfile *and* a readable
    // manifest does produce gaps, so the two assertions above are about the
    // missing manifest rather than about the fixture not being a Node project.
    let control = Fixture::new("control");
    control.write("package-lock.json", "{}\n");
    control.write(MANIFEST, r#"{"name":"control"}"#);
    let control_checks = NodeChecks::of(&control.node(), control.path());
    assert_eq!(control_checks.missing().len(), 4);
    assert!(control_checks.planned().is_empty());
}

#[test]
fn a_workspace_member_is_checked_under_its_own_identifier_and_its_own_title() {
    // Two manifests with the same script in two directories are two checks. A
    // report that showed them both as "build the project" would be unreadable,
    // and one that gave them one identifier would drop one of them as a
    // duplicate of the other.
    let fixture = Fixture::new("workspace");
    fixture.write(
        MANIFEST,
        r#"{"name":"root","workspaces":["packages/*"],"scripts":{"build":"turbo run build"}}"#,
    );
    fixture.write(
        "packages/web/package.json",
        r#"{"name":"web","scripts":{"build":"vite build"}}"#,
    );
    fixture.write("pnpm-lock.yaml", "");

    let checks = NodeChecks::of(&fixture.node(), fixture.path());
    let titles = titles(&checks);
    assert!(
        titles.contains(&ScriptRole::Build.plain_description().to_owned()),
        "{titles:?}"
    );
    assert!(
        titles.contains(&format!(
            "{} in packages/web",
            ScriptRole::Build.plain_description()
        )),
        "the member's check has no title that says it is the member's: {titles:?}"
    );
    assert!(
        titles.contains(&format!(
            "{} in packages/web",
            ScriptRole::Test.plain_description()
        )),
        "a member with no test script is a gap like any other: {titles:?}"
    );

    // Every identifier is distinct, which is the property that keeps a member's
    // check from being dropped as a duplicate of the root's.
    let mut ids: Vec<&str> = checks
        .planned()
        .iter()
        .map(|work| work.proposal().id().as_str())
        .chain(checks.missing().iter().map(|missing| missing.id().as_str()))
        .collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        total,
        "two components share an identifier: {ids:?}"
    );
    assert_eq!(total, 8, "two manifests, four roles each");

    // And the plan holds every one of them, with nothing refused and nothing
    // merged away.
    let planned = plan(&checks, ExecutionMode::HostConfirmed, host_confirmed());
    assert_eq!(planned.len(), checks.planned().len());
    assert!(
        planned.duplicates().is_empty(),
        "{:?}",
        planned.duplicates()
    );
}

#[test]
fn the_same_project_is_checked_under_the_same_identifiers_every_time() {
    // The precondition of the repair contract in `docs/architecture/CHECK_PIPELINE.md`:
    // a result from one run is joined to the plan of another by identity, and a
    // second reading of an unchanged project that named its checks anything else
    // would orphan every stored result. Read twice from disk rather than compared
    // against a constant, so the claim is about the recipe rather than about one
    // spelling of it.
    let fixture = Fixture::new("stable-identity");
    fixture.write(
        MANIFEST,
        r#"{"name":"stable","workspaces":["packages/*"],"scripts":{"build":"tsc -b","test":"vitest run"}}"#,
    );
    fixture.write(
        "packages/web/package.json",
        r#"{"name":"web","scripts":{"lint":"eslint ."}}"#,
    );
    fixture.write("bun.lockb", "");

    let identities = |checks: &NodeChecks| -> Vec<String> {
        let mut ids: Vec<String> = checks
            .planned()
            .iter()
            .map(|work| work.proposal().id().as_str().to_owned())
            .chain(
                checks
                    .missing()
                    .iter()
                    .map(|missing| missing.id().as_str().to_owned()),
            )
            .collect();
        ids.sort();
        ids
    };

    let first = identities(&NodeChecks::of(&fixture.node(), fixture.path()));
    let second = identities(&NodeChecks::of(&fixture.node(), fixture.path()));
    assert_eq!(first, second, "an unchanged project was named two ways");
    assert_eq!(first.len(), 8);
    for id in &first {
        assert!(id.starts_with("chk_"), "{id}");
        assert!(
            id.len() > "chk_".len() + 16,
            "{id} carries no readable half, so a report is a wall of hashes"
        );
    }

    // The readable half says what the check is, which is the reason the tag is in
    // there at all.
    assert!(
        first.iter().any(|id| id.contains("build")),
        "no identifier says what it checks: {first:?}"
    );
    assert!(first.iter().any(|id| id.contains("test")), "{first:?}");

    // Changing the project changes the command, and changes it for the member
    // that declared the script and not for the others.
    fixture.write(
        "packages/web/package.json",
        r#"{"name":"web","scripts":{"lint":"biome check ."}}"#,
    );
    let third = identities(&NodeChecks::of(&fixture.node(), fixture.path()));
    assert_eq!(
        first, third,
        "a script's *command* changed and the check's identity moved with it; the \
         identity is about which check this is, not about what it currently says"
    );
}

#[test]
fn a_project_that_is_not_a_node_project_gets_no_checks_at_all() {
    // The layer above discovery must not be the place a Node check appears for a
    // project that is not one. This is the cheapest statement of that, and it is
    // here because the checks layer takes a `NodeProject` — a value that only
    // discovery of a Node project produces — so the impossibility is in the
    // types; what a test can hold is that discovery still declines.
    let fixture = Fixture::new("not-node");
    fixture.write("src/main.rs", "fn main() {}\n");
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"nope\"\nversion = \"0.0.0\"\n",
    );
    assert!(
        fixture.node_or_none().is_none(),
        "a Rust project was reported as a Node project, so every check above \
         could be proposed for a project that is not one"
    );
}

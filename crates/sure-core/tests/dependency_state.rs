//! The state a project's packages are in, over real directories.
//!
//! `crate::dependency_state`'s own tests hold the two rules that need no
//! project — the table's content and the reading derived from a state. This file
//! drives the reading over directories a walk actually produced, which is where
//! it has to add up to something true, and where `P4-T008`'s two acceptance
//! sentences can be checked.
//!
//! # The claims this file exists to hold
//!
//! > Missing dependencies are distinguishable from failing project code.
//! > Install remains separate approved action.
//!
//! The second is the easy one and it is checked twice: as a requirement the
//! domain refuses under inspect-only, and as a source rule — this module reads
//! no file and starts no process, so there is nothing it *could* have run. The
//! source rule is what makes the first one safe rather than merely intended.
//!
//! The first is the one a test is naturally bad at, because the failure mode is
//! a claim that is *absent* rather than one that is wrong. A suite that only
//! checked "the missing case is reported" would pass just as well if SURE also
//! reported every project that has its packages in place. So three of the tests
//! below are about silence, and they are the ones worth reading:
//! `a_project_whose_packages_are_there_produces_no_claim_at_all`,
//! `a_yarn_plug_n_play_project_has_no_node_modules_and_is_not_a_finding` — the
//! false positive the sentinel list exists to prevent, since a PnP project
//! resolves packages with no `node_modules` in sight — and
//! `a_manifest_sure_could_not_read_is_not_a_project_whose_packages_are_missing`.
//!
//! # The one thing this module may never say
//!
//! **That the packages are installed.** A sentinel being present is not the
//! package manager's answer, and `docs/architecture/ECOSYSTEM_DISCOVERY.md` is
//! explicit that what is installed is the package manager's answer and not a
//! fact about the project. `a_sentinel_being_present_is_not_a_statement_that_the_packages_are_right`
//! holds that against a tree whose `node_modules` is deliberately wrong: it
//! declares one package and the directory holds a different one, and SURE must
//! still say nothing at all.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::dependency_state::{Assessed, DependencyReport, ECOSYSTEMS, InstallState, Reading};
use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::scan::ScanOptions;
use sure_core::schedule::ExecutionRequirements;
use sure_domain::evidence::{AnchorSubject, ClaimAssessment, EvidenceClass};
use sure_domain::execution::{ActionKind, ExecutionPermissions, Permission};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

/// A directory under the workspace's git-ignored `target/tmp`.
///
/// Unique per call and **never cleared**, which is the pattern this repository
/// settled on after a false report: clearing a fixed path with
/// `let _ = remove_dir_all(..)` and then treating it as fresh fails on Windows,
/// and the test then describes a directory that was never emptied. A path nobody
/// has used before needs no removal. Uniqueness comes from `create_dir`, not
/// from the name, so two processes given the same id cannot collide — a
/// directory that exists is skipped rather than adopted.
///
/// The directory name carries a space on purpose, in the way
/// `tests/python_checks.rs` does for its own: this module's reading is a rule
/// about names, and a fixture whose path has a space in it is where a rule that
/// went through a shell would come apart.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("dependency state");
        std::fs::create_dir_all(&base)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
        for _ in 0..1_000 {
            let project = base.join(format!("{test}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
            match std::fs::create_dir(&project) {
                Ok(()) => return Self { project },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create {}: {error}", project.display()),
            }
        }
        panic!("no free fixture name under {}", base.display());
    }

    /// A fixture with its files already written.
    ///
    /// **The chained form does not compile**, which is worth knowing before
    /// writing it: `write` borrows the fixture it was called on, and a `let` does
    /// not extend the lifetime of a temporary *receiver*, so
    /// `let f = Fixture::new(x).write(..)` is a borrow of a value freed at the
    /// end of the statement. Binding first is the only form that works, and
    /// having it in one place is why this exists rather than being repeated in
    /// every test.
    fn with(test: &str, files: &[(&str, &str)]) -> Self {
        let fixture = Self::new(test);
        for (relative, contents) in files {
            fixture.write(relative, contents);
        }
        fixture
    }

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

    fn discovery(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default()).expect("discover")
    }

    fn discovery_with(&self, options: &DiscoverOptions) -> Discovery {
        discover(&self.project, options).expect("discover")
    }

    fn report(&self) -> DependencyReport {
        self.report_at(&FingerprintId::generate())
    }

    fn report_at(&self, fingerprint: &FingerprintId) -> DependencyReport {
        DependencyReport::of(&self.discovery(), fingerprint)
    }
}

/// The single claim a report holds, naming what else was there if it is not one.
///
/// This module has one row, so a report with more than one claim is a table edit
/// rather than a project, and a helper that took the first would hide it.
fn only(report: &DependencyReport) -> &Assessed {
    let claims = report.claims();
    assert_eq!(claims.len(), 1, "expected exactly one claim: {claims:?}");
    &claims[0]
}

/// The Node row, for asking `InstallState` directly.
fn node_row() -> &'static sure_core::dependency_state::Dependencies {
    ECOSYSTEMS
        .iter()
        .find(|row| row.name == "Node")
        .expect("the node row")
}

fn project_with_two_packages() -> Fixture {
    let fixture = Fixture::new("declared");
    fixture.write(
        "package.json",
        r#"{"name":"demo","dependencies":{"express":"^4.18.0"},"devDependencies":{"jest":"^29.0.0"}}"#,
    );
    fixture
}

#[test]
fn declared_packages_and_no_installed_tree_is_the_one_claim_this_module_makes() {
    let fixture = project_with_two_packages();
    let fingerprint = FingerprintId::generate();
    let report = fixture.report_at(&fingerprint);

    assert!(
        report.is_complete(),
        "the walk of a two-file project finished"
    );
    let claim = only(&report);

    assert_eq!(claim.ecosystem(), "Node");
    assert_eq!(claim.declared(), 2, "runtime and development alike");
    assert_eq!(claim.declared_in(), "package.json");
    assert_eq!(claim.state(), InstallState::NotInPlace);
    assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(claim.severity(), Severity::Note);
    assert!(
        claim.reason().contains("node_modules"),
        "{}",
        claim.reason()
    );

    // Two observations and both are needed: the manifest that says there is
    // something to install, and the walk that says it is not there. Either alone
    // is a project with nothing to install, or a directory SURE had no reason to
    // look for.
    assert_eq!(claim.evidence().len(), 2);
    for evidence in claim.evidence() {
        assert_eq!(evidence.class, EvidenceClass::ObservedFact);
        assert_eq!(evidence.severity, Severity::Note);
        assert!(evidence.anchor.is_checkable(), "{evidence:?}");
        assert!(
            evidence.fingerprint.as_ref() == Some(&fingerprint),
            "evidence not bound to a state can never support anything: {evidence:?}"
        );
    }
    assert_eq!(claim.evidence()[0].anchor.subject, AnchorSubject::File);
    assert_eq!(claim.evidence()[0].anchor.location, "package.json");
    // The absence is anchored where the thing would have been, which is the
    // shape `crate::db_migrations` uses for its own absences: a location a reader
    // can check by looking at it.
    assert_eq!(claim.evidence()[1].anchor.subject, AnchorSubject::Directory);
    assert_eq!(claim.evidence()[1].anchor.location, "node_modules");
    assert!(
        claim.evidence()[1]
            .anchor
            .locator
            .contains("after an install"),
        "the locator has to say what would have been there: {:?}",
        claim.evidence()[1].anchor
    );

    // The sentence a person reads says what SURE did not do, which is the second
    // half of the acceptance reaching the surface a person actually sees.
    let plain = claim.plain_description();
    assert!(plain.contains("2 packages"), "{plain}");
    assert!(plain.contains("separate action"), "{plain}");
    assert!(plain.contains("not installed anything"), "{plain}");
}

#[test]
fn a_project_whose_packages_are_there_produces_no_claim_at_all() {
    let fixture = Fixture::with(
        "installed",
        &[
            (
                "package.json",
                r#"{"name":"demo","dependencies":{"express":"^4.18.0"}}"#,
            ),
            // The directory an install fills. It is in the scanner's ignore
            // table, so it reaches the reading as a skip rather than as an entry
            // — the other half of the pair the Yarn PnP test below exercises.
            ("node_modules/express/package.json", r#"{"name":"express"}"#),
        ],
    );

    let report = fixture.report();
    assert!(report.is_complete(), "the walk finished");
    assert_eq!(
        report.reading(),
        Reading::TheCode,
        "a project with its packages in place adds no sentence to a failure"
    );

    // Asked directly, because "no claim" and "in place" are different statements
    // and only the second says what was met.
    assert_eq!(
        InstallState::of(node_row(), &fixture.discovery()),
        InstallState::InPlace {
            met: "node_modules"
        }
    );
}

#[test]
fn a_sentinel_being_present_is_not_a_statement_that_the_packages_are_right() {
    // **The claim this module may never make.** The manifest declares `express`
    // and the tree holds `lodash`: a genuinely wrong install, the case a person
    // most wants SURE to catch, and one SURE cannot catch by reading a directory
    // name. What is installed is the package manager's answer, so the only honest
    // reading of a sentinel is silence — see the module documentation.
    //
    // This test asserts SURE says nothing. It is written rather than omitted so
    // that a later task that teaches SURE to read inside `node_modules` has to
    // come here and change a test that says why it does not.
    let fixture = Fixture::with(
        "wrong-packages",
        &[
            (
                "package.json",
                r#"{"name":"demo","dependencies":{"express":"^4.18.0"}}"#,
            ),
            ("node_modules/lodash/package.json", r#"{"name":"lodash"}"#),
        ],
    );

    let report = fixture.report();
    assert!(
        report.is_empty(),
        "a directory name is not the package manager's answer: {:?}",
        report.claims()
    );
    assert_eq!(report.reading(), Reading::TheCode);
}

#[test]
fn a_yarn_plug_n_play_project_has_no_node_modules_and_is_not_a_finding() {
    // The false positive the generous sentinel list exists to prevent. Yarn's
    // plug'n'play linker resolves packages from a `.pnp.cjs` at the root and
    // writes **no `node_modules` at all**, so a table that named only
    // `node_modules` would tell every PnP project in existence to install
    // packages it already has.
    //
    // This is also the half that exercises the walk's *entries* rather than its
    // skips: `.pnp.cjs` is an ordinary file, not something the ignore table
    // leaves out, so a reading that looked only at `Scan::skipped` would be right
    // about npm and silently wrong here.
    let fixture = Fixture::with(
        "yarn-pnp",
        &[
            (
                "package.json",
                r#"{"name":"demo","dependencies":{"react":"^18.4.0"}}"#,
            ),
            (".pnp.cjs", "// Generated by Yarn. Do not edit.\n"),
        ],
    );

    let report = fixture.report();
    assert!(
        report.is_empty(),
        "a PnP project has no node_modules and needs no install: {:?}",
        report.claims()
    );
    assert_eq!(
        InstallState::of(node_row(), &fixture.discovery()),
        InstallState::InPlace { met: ".pnp.cjs" }
    );
}

#[test]
fn a_project_that_declares_nothing_has_nothing_to_install() {
    let fixture = Fixture::with(
        "no-packages",
        &[(
            "package.json",
            r#"{"name":"demo","scripts":{"test":"node --test"}}"#,
        )],
    );

    let report = fixture.report();
    assert!(
        report.is_empty(),
        "a project with nothing to install cannot be missing packages: {:?}",
        report.claims()
    );
    assert_eq!(report.reading(), Reading::TheCode);
}

#[test]
fn a_member_directory_is_not_the_root_and_does_not_answer_for_it() {
    // Hoisting, which is why only the root is read. npm, yarn and pnpm put a
    // workspace's packages in the **root** `node_modules`, so a member without
    // one of its own is the normal shape of an installed monorepo.
    //
    // The fixture is the other arrangement, and the assertion is the one that
    // matters: a `node_modules` three levels down is not the root's, so the root
    // is still missing its install — the reading is equality with the root rather
    // than a search under it.
    let fixture = Fixture::with(
        "hoisted",
        &[
            (
                "package.json",
                r#"{"name":"root","workspaces":["packages/*"],"dependencies":{"express":"^4.18.0"}}"#,
            ),
            (
                "packages/web/package.json",
                r#"{"name":"web","dependencies":{"react":"^18.4.0"}}"#,
            ),
            (
                "packages/web/node_modules/react/package.json",
                r#"{"name":"react"}"#,
            ),
        ],
    );

    let report = fixture.report();
    let claim = only(&report);
    assert_eq!(
        claim.declared(),
        1,
        "the root's own declarations, not a member's"
    );
    assert_eq!(claim.state(), InstallState::NotInPlace);
    assert_eq!(
        claim.evidence()[1].anchor.location,
        "node_modules",
        "the claim names the root's directory and no other"
    );
}

#[test]
fn a_walk_that_did_not_finish_cannot_confirm_an_absence() {
    let fixture = Fixture::with(
        "unfinished-walk",
        &[
            (
                "package.json",
                r#"{"name":"demo","dependencies":{"express":"^4.18.0"}}"#,
            ),
            ("src/deep/buried.ts", "export const buried = 1;\n"),
        ],
    );

    let options = DiscoverOptions {
        scan: ScanOptions {
            max_depth: 1,
            ..ScanOptions::default()
        },
        ..DiscoverOptions::default()
    };
    let discovery = fixture.discovery_with(&options);
    assert!(
        !discovery.is_complete(),
        "the depth limit has to have cost something for this test to mean anything"
    );

    let report = DependencyReport::of(&discovery, &FingerprintId::generate());
    assert!(!report.is_complete());
    let claim = only(&report);
    assert_eq!(claim.assessment(), ClaimAssessment::CannotConfirm);
    assert!(
        claim.evidence().is_empty(),
        "every piece of evidence here is a statement about what the walk saw, and this walk did \
         not finish: {:?}",
        claim.evidence()
    );
    assert!(
        matches!(claim.state(), InstallState::CannotTell(_)),
        "{:?}",
        claim.state()
    );
    assert_eq!(claim.severity(), Severity::Note);

    // And the direction that matters: an absence SURE could not establish does
    // not become the sentence that stops a failure being read as the code. A
    // caveat with no observation behind it, printed on every project SURE could
    // not finish reading, would teach a reader to skip the sentence where it does
    // matter.
    assert_eq!(report.reading(), Reading::TheCode);
    assert_eq!(report.reading().plain_explanation(), None);
}

#[test]
fn a_manifest_sure_could_not_read_is_not_a_project_whose_packages_are_missing() {
    // The false red this module is most likely to make, and the one a careless
    // reading produces naturally: no declarations were counted, so there is
    // "nothing" — and an absence reported from a manifest SURE never read would
    // be SURE telling a person their packages are gone on the strength of its own
    // failure to look.
    let fixture = Fixture::with(
        "unreadable-manifest",
        &[(
            "package.json",
            r#"{"name":"demo","dependencies":{"express":"^4.18.0"}}"#,
        )],
    );

    let options = DiscoverOptions {
        max_manifest_bytes: 1,
        ..DiscoverOptions::default()
    };
    let discovery = fixture.discovery_with(&options);
    assert_eq!(
        discovery.unread.len(),
        1,
        "the manifest has to have been refused for this test to mean anything"
    );

    let report = DependencyReport::of(&discovery, &FingerprintId::generate());
    assert!(
        report.is_empty(),
        "SURE has no count and no absence to report: {:?}",
        report.claims()
    );
    assert_eq!(report.reading(), Reading::TheCode);
}

#[test]
fn the_case_a_sentinel_is_looked_up_by_is_the_platforms_rule() {
    // A sentinel whose case differs from the table's, which is the same file on
    // one platform and a different name on another. The expected answer is chosen
    // by `cfg!` inside one test rather than by two `#[cfg]`-gated tests, because
    // the gate is easy to write wrong in a way that silently skips: the rule is
    // `CaseSensitivity::platform()`, which folds **macOS with Windows**, so any
    // gate spelled `not(windows)` would run the case-sensitive assertion on a
    // platform that answers case-insensitively.
    //
    // Whichever way this platform answers, the test asserts that answer, so there
    // is no platform on which it passes by not running.
    let fixture = Fixture::with(
        "differently-cased-sentinel",
        &[
            (
                "package.json",
                r#"{"name":"demo","dependencies":{"express":"^4.18.0"}}"#,
            ),
            ("NODE_MODULES/express/package.json", r#"{"name":"express"}"#),
        ],
    );

    let report = fixture.report();
    if cfg!(any(windows, target_os = "macos")) {
        assert!(
            report.is_empty(),
            "this platform reads the two names as the same file: {:?}",
            report.claims()
        );
        assert_eq!(
            InstallState::of(node_row(), &fixture.discovery()),
            InstallState::InPlace {
                met: "node_modules"
            }
        );
    } else {
        let claim = only(&report);
        assert_eq!(
            claim.state(),
            InstallState::NotInPlace,
            "this platform reads the two names as different files"
        );
    }
}

#[test]
fn an_install_is_a_separate_action_that_needs_its_own_permission() {
    let fixture = project_with_two_packages();
    let report = fixture.report();
    let claim = only(&report);

    // The action this module names is the install permission, and it is the one
    // permission that never comes free: under inspect-only the requirement is
    // refused. Nothing here enforces that — `sure_domain::execution::decide`
    // does — and the assertion is that the answer this module *names* is the one
    // that gets refused, so a module naming `ReadMetadata` would fail here.
    assert_eq!(claim.action(), ActionKind::InstallDependencies);
    let requirements = ExecutionRequirements::of(&[claim.action()]);
    assert_eq!(
        requirements.permissions_needed(),
        vec![Permission::InstallDependencies],
        "an install asks for the install permission and for nothing else"
    );
    assert_eq!(
        requirements.blocked_by(&ExecutionPermissions::inspect_only()),
        Some(Permission::InstallDependencies),
        "an install that inspect-only allowed would make the permission decorative"
    );
    assert!(
        claim.action().executes_project_code(),
        "an install runs the installer, which the domain counts as project code"
    );

    // And the false-green guard, which falls out of the severity rather than
    // being bolted on: a missing install cannot refuse a hand-off, and it is not
    // a statement about the project's quality.
    assert!(!claim.severity().blocks_hand_off());
    assert!(
        claim.severity().rank() < Severity::ShouldFixFirst.rank(),
        "an environment fact must not outrank a quality risk"
    );
}

#[test]
fn the_reading_is_a_property_of_the_project_and_not_of_any_one_check() {
    let missing = project_with_two_packages();
    let installed = Fixture::with(
        "reading-installed",
        &[
            (
                "package.json",
                r#"{"name":"demo","dependencies":{"express":"^4.18.0"}}"#,
            ),
            ("node_modules/express/package.json", r#"{"name":"express"}"#),
        ],
    );

    // Asked of the report rather than of a result, because it is a property of
    // the project state: two checks over one tree are read the same way.
    assert_eq!(missing.report().reading(), Reading::TheDependencies);
    assert_eq!(installed.report().reading(), Reading::TheCode);

    let sentence = Reading::TheDependencies
        .plain_explanation()
        .expect("the reading that is not about the code carries the sentence");
    assert!(sentence.contains("packages"), "{sentence}");
    assert!(sentence.contains("not installed anything"), "{sentence}");
    assert!(sentence.contains("separate action"), "{sentence}");
}

#[test]
fn the_module_opens_no_file_and_runs_nothing() {
    // The acceptance's second half as a source rule rather than an intention.
    // "Install remains separate approved action" is not only about what this
    // module *does* with an install — it is about there being nothing here that
    // could perform one. A module that reads no file and starts no process has no
    // path to an unrequested change on somebody's machine.
    //
    // The unit tests are cut off first, because a test may do anything and this
    // rule is about what ships.
    let source = std::fs::read_to_string(
        sure_testkit::repository_root()
            .join("crates")
            .join("sure-core")
            .join("src")
            .join("dependency_state.rs"),
    )
    .expect("the module source");
    let shipped = source
        .split("#[cfg(test)]")
        .next()
        .expect("the source before its tests");

    const FORBIDDEN: &[&str] = &[
        "std::fs",
        "File::open",
        "OpenOptions",
        "BufReader",
        "read_to_string",
        "std::process",
        "Command::new",
    ];
    for (number, line) in shipped.lines().enumerate() {
        // Prose only. The module documentation talks about files and processes,
        // because it is explaining why it does not open or start them.
        let code = line.split("//").next().unwrap_or("");
        for forbidden in FORBIDDEN {
            assert!(
                !code.contains(forbidden),
                "line {}: the shipped half of this module may not use `{forbidden}`: {line}",
                number + 1
            );
        }
    }
}

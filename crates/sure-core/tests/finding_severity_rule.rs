//! `P7-T011`: the severity rule, read from outside the modules that use it.
//!
//! `crates/sure-core/src/finding_gravity.rs` holds the rule, and its own unit
//! tests hold the rule's arithmetic. This file holds the three things that
//! cannot be seen from inside it:
//!
//! - **That no shipped detector decides a severity for itself.** The rule is
//!   worth nothing if six modules keep their own copy of the answer, so the
//!   check is textual and it runs over the shipping half of every file that
//!   used to hard-code [`Severity::Note`]. `the_guard_can_fail_on_the_two_moves_it_exists_to_catch`
//!   shows the guard failing on a source where the rule's answer was replaced by
//!   a literal, because a guard nobody has seen fail is a guard nobody knows
//!   works.
//!
//! - **That the corpus's release-blocking cases reach the severity the corpus
//!   asks for.** Not the severity a report claims they reach: these tests run
//!   the scanners over the shipped fixtures and read the proposals they produce.
//!   The expected severities are read out of `evaluation/acceptance-manifest.json`
//!   rather than typed in here, and the set of cases measured is asserted so a
//!   new release-blocking fixture app fails this file instead of being skipped by
//!   it.
//!
//! - **That the benign cases did not escalate, and that the weight was not
//!   bought by promoting an inference to a fact.** A `must_fix` that rests on a
//!   pattern guess is the false green this product exists to catch, one level up,
//!   so the evidence class is asserted beside the severity everywhere the
//!   severity is asserted.
//!
//! # What is not claimed
//!
//! **Coverage of the whole corpus.** Six release-blocking cases have a fixture
//! app, and all six are measured here. The other release-blocking cases cannot be
//! measured by running anything: `check-crash`, `dangerous-delete`,
//! `stale-test-evidence`, `tests-not-run` and `repair-regression` are directories
//! holding a descriptor and no project, and `force-push` and `sensitive-read`
//! have no fixture directory at all. Of the cases that do have an app,
//! `missing-migration` is measured by `db_migrations` rather than by a candidate
//! detector. This file measures what exists and names what does not, rather than
//! asserting over a set it quietly shrank.
//!
//! **A verdict.** A `must_fix` candidate is not a `must_fix` finding: nothing
//! these detectors set is `critical`, and the aggregator's `material` list is a
//! list of things to look at rather than a list of defects SURE has established.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use sure_core::candidate_scanner::CandidateScanner;
use sure_core::db_migrations::MigrationsReport;
use sure_core::demo_data_heuristics::DemoDataHeuristics;
use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::evidence::EvidenceClass;
use sure_core::false_completion_aggregator::aggregate;
use sure_core::ids::FingerprintId;
use sure_core::noop_heuristics::NoOpHeuristics;
use sure_core::route_consistency::RouteConsistency;
use sure_core::schedule::CheckProposal;
use sure_core::severity::Severity;
use sure_core::ui_action_bridge::UiActionBridge;

/// Read a file of this repository from the checkout it is being tested in.
fn repository_file(relative: &str) -> String {
    let path = sure_testkit::repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// A shipped fixture directory.
fn fixture(id: &str) -> PathBuf {
    sure_testkit::repository_root()
        .join("fixtures")
        .join("adversarial")
        .join(id)
}

fn discovery(id: &str) -> Discovery {
    let root = fixture(id);
    assert!(
        root.is_dir(),
        "the {id} fixture directory is missing: {}",
        root.display()
    );
    discover(&root, &DiscoverOptions::default())
        .unwrap_or_else(|error| panic!("cannot read the {id} fixture: {error}"))
}

// --- the rule is the only place a severity is decided ---------------------

/// The shipped files that decide how serious a detector finding is, and the call
/// each one must make to the rule instead of naming a level itself.
///
/// Every one of these six files hard-coded `Severity::Note` before `P7-T011` —
/// 21 occurrences between them — and the aggregator spelled the same decision out
/// as a conjunction of three fields. The list is the files, not a count, so that
/// a file leaving the list is as visible as one joining it.
const RULE_CONSUMERS: &[(&str, &str)] = &[
    ("crates/sure-core/src/candidate_scanner.rs", "gravity_of("),
    (
        "crates/sure-core/src/demo_data_heuristics.rs",
        "gravity_of(",
    ),
    (
        "crates/sure-core/src/false_completion_aggregator.rs",
        "is_informational(",
    ),
    (
        "crates/sure-core/src/intent_implementation.rs",
        "gravity_of(",
    ),
    ("crates/sure-core/src/noop_heuristics.rs", "gravity_of("),
    ("crates/sure-core/src/route_consistency.rs", "gravity_of("),
    ("crates/sure-core/src/ui_action_bridge.rs", "gravity_of("),
];

/// The half of a source file that ships: everything before its test module.
///
/// The split has to be explicit rather than assumed. A test module is full of
/// severity literals on purpose — that is how a test states what it expects — so
/// a file with no test module would be scanned whole and would fail for the wrong
/// reason. [`rule_violations`] refuses that case by name.
fn shipping_half(source: &str) -> &str {
    match source.find("\n#[cfg(test)]") {
        Some(at) => &source[..at],
        None => source,
    }
}

/// Lines that name a severity level directly, comments aside.
///
/// Whole-line comments are dropped first. A doc comment explaining the rule has
/// to be able to name the levels it returns — `[`Severity::Note`]` in a doc link
/// is documentation, not a detector assigning one — and the difference that
/// matters is between writing a level into the code and writing it into a
/// sentence. Code, and trailing comments on lines of code, are searched as
/// written.
///
/// The level names are generated from [`Severity::ALL`] rather than typed, so a
/// fifth level would be searched for the day it exists.
fn severity_literals(source: &str) -> Vec<String> {
    let levels: Vec<String> = Severity::ALL
        .iter()
        .map(|severity| format!("Severity::{severity:?}"))
        .collect();
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| levels.iter().any(|level| line.contains(level.as_str())))
        .map(|line| line.trim().to_owned())
        .collect()
}

/// Everything wrong with one file's handling of the rule, as sentences.
///
/// Returns violations rather than panicking so the guard itself can be run over
/// a source that must fail it, which is the only way to know the guard works.
fn rule_violations(path: &str, source: &str) -> Vec<String> {
    let shipped = shipping_half(source);
    let mut violations = Vec::new();
    if shipped.len() == source.len() {
        violations.push(format!(
            "{path} has no `#[cfg(test)]` marker, so the shipping half could not be \
             separated from the tests and this check would read the wrong text"
        ));
    }
    for line in severity_literals(shipped) {
        violations.push(format!(
            "{path} names a severity in shipping code instead of asking the rule: {line}"
        ));
    }
    let required = RULE_CONSUMERS
        .iter()
        .find(|(consumer, _)| *consumer == path)
        .map(|(_, call)| *call)
        .unwrap_or("gravity_of(");
    if !shipped.contains(required) {
        violations.push(format!(
            "{path} no longer calls `{required}`, so nothing in it reads the rule"
        ));
    }
    violations
}

#[test]
fn no_shipped_detector_names_a_severity_level_of_its_own() {
    // The textual half of `P7-T011`'s acceptance: a detector that assigns a
    // severity without going through the rule fails a test. It fails here.
    for (path, _) in RULE_CONSUMERS {
        let source = repository_file(path);
        let violations = rule_violations(path, &source);
        assert!(
            violations.is_empty(),
            "the severity rule has a second home:\n  {}",
            violations.join("\n  ")
        );
    }
}

#[test]
fn the_guard_can_fail_on_the_two_moves_it_exists_to_catch() {
    // A guard that has never failed is a guard whose failure nobody has seen.
    // Both violations are provoked here on real source, and the unmutated source
    // is asserted clean first so the two failures below are the mutation and not
    // a file that was already dirty.
    let path = "crates/sure-core/src/noop_heuristics.rs";
    let source = repository_file(path);
    assert!(
        rule_violations(path, &source).is_empty(),
        "this file is the example for the guard, so it has to pass it"
    );

    // The move the check exists to catch: the rule's answer replaced by a level.
    let literal = source.replacen("gravity.severity()", "Severity::Note", 1);
    assert_ne!(
        literal, source,
        "the mutation did not apply: `noop_heuristics.rs` no longer calls `gravity.severity()`, \
         so this test would have proved nothing"
    );
    let violations = rule_violations(path, &literal);
    assert_eq!(
        violations.len(),
        1,
        "the guard missed a detector assigning a severity literal: {violations:?}"
    );
    assert!(
        violations[0].contains("Severity::Note"),
        "the guard failed for some other reason: {}",
        violations[0]
    );

    // The other half: a file that keeps the rule's call but stops asking the
    // rule is also caught, so deleting the call cannot pass as tidying up.
    let renamed = source.replacen("gravity_of(", "severity_for(", 1);
    assert_ne!(renamed, source, "the second mutation did not apply");
    let violations = rule_violations(path, &renamed);
    assert_eq!(
        violations.len(),
        1,
        "the guard missed a detector that stopped reading the rule: {violations:?}"
    );
    assert!(
        violations[0].contains("gravity_of("),
        "the guard failed for some other reason: {}",
        violations[0]
    );
}

// --- the corpus, measured -------------------------------------------------

/// One case as `evaluation/acceptance-manifest.json` states it.
struct ManifestCase {
    id: String,
    release_blocking: bool,
    expected_severity: String,
}

/// The manifest's cases, read from its text.
///
/// Deliberately not a JSON parser: the manifest is a flat, pretty-printed list of
/// objects and this needs three of its fields. Reading it here rather than typing
/// the expected severities in is the difference between testing the corpus and
/// testing this file's memory of the corpus. The caller checks the case count it
/// got against the number of `"release_blocking"` fields in the same text, so a
/// change to the document's shape fails the test instead of dropping a case.
fn manifest_cases() -> Vec<ManifestCase> {
    let text = repository_file("evaluation/acceptance-manifest.json");
    let mut cases = Vec::new();
    let mut id: Option<String> = None;
    let mut blocking: Option<bool> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = json_field(line, "id") {
            id = Some(value);
            blocking = None;
        } else if let Some(value) = json_bool_field(line, "release_blocking") {
            blocking = Some(value);
        } else if let Some(value) = json_field(line, "expected_severity") {
            cases.push(ManifestCase {
                id: id.take().unwrap_or_else(|| {
                    panic!("an `expected_severity` appeared before any `id` in the manifest")
                }),
                release_blocking: blocking.unwrap_or_else(|| {
                    panic!("a case stated no `release_blocking` before its `expected_severity`")
                }),
                expected_severity: value,
            });
        }
    }
    let declared = text.matches("\"release_blocking\":").count();
    assert_eq!(
        cases.len(),
        declared,
        "the reader of the manifest lost or duplicated a case, so this test is not \
         measuring the corpus it names"
    );
    assert!(!cases.is_empty(), "the manifest states no cases at all");
    cases
}

/// The string value of `"name": "value"` on one line, if that is what it is.
fn json_field(line: &str, name: &str) -> Option<String> {
    let prefix = format!("\"{name}\": \"");
    let rest = line.strip_prefix(&prefix)?;
    let rest = rest.strip_suffix(',').unwrap_or(rest);
    Some(rest.strip_suffix('"')?.to_owned())
}

/// The boolean value of `"name": true` on one line, if that is what it is.
fn json_bool_field(line: &str, name: &str) -> Option<bool> {
    let rest = line.strip_prefix(&format!("\"{name}\": "))?;
    let rest = rest.strip_suffix(',').unwrap_or(rest);
    match rest {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Whether a corpus id has a fixture app rather than a stub or nothing at all.
///
/// Three shapes are counted the same way here, because none of them can be run:
/// a corpus id with no fixture directory (`force-push`, `sensitive-read`,
/// `benign-test-mocks`, `missing-user-intent`), one whose directory holds only a
/// descriptor (`check-crash`, `dangerous-delete`, `lying-readme`,
/// `repair-regression`, `stale-test-evidence`, `tests-not-run`,
/// `unknown-evidence`), and one that declares an app. The reading is taken from
/// the fixture rather than from a list in this file, which would be a second
/// place to keep it.
fn fixture_has_an_app(id: &str) -> bool {
    let path = fixture(id).join("scenario.json");
    std::fs::read_to_string(&path).is_ok_and(|scenario| scenario.contains("\"entry_points\""))
}

/// Every candidate the five false-completion scanners propose for one fixture.
fn candidate_proposals(discovery: &Discovery) -> Vec<CheckProposal> {
    let mut proposals = CandidateScanner::of(discovery).proposed().to_vec();
    proposals.extend(NoOpHeuristics::of(discovery).proposed().iter().cloned());
    proposals.extend(DemoDataHeuristics::of(discovery).proposed().iter().cloned());
    proposals.extend(RouteConsistency::of(discovery).proposed().iter().cloned());
    proposals.extend(UiActionBridge::of(discovery).proposals());
    proposals
}

/// The heaviest severity the five scanners give one fixture, and every class
/// they reached it with.
fn detector_reading(id: &str) -> (Option<Severity>, Vec<EvidenceClass>) {
    let proposals = candidate_proposals(&discovery(id));
    (
        proposals
            .iter()
            .map(CheckProposal::severity)
            .max_by_key(|severity| severity.rank()),
        proposals
            .iter()
            .map(CheckProposal::evidence_class)
            .collect(),
    )
}

/// The heaviest severity SURE reaches about one fixture, measured.
///
/// Whichever surface has something to say is the one that answers: the candidate
/// scanners for the fixtures they were written for, and `db_migrations` for the
/// one case no candidate detector fires on. A fixture nothing has anything to say
/// about panics here rather than answering with a severity nobody produced.
fn measured_severity(id: &str) -> Severity {
    let (detectors, _) = detector_reading(id);
    if let Some(severity) = detectors {
        return severity;
    }
    MigrationsReport::of(&discovery(id), &FingerprintId::generate())
        .claims()
        .iter()
        .map(|claim| claim.severity())
        .max_by_key(|severity| severity.rank())
        .unwrap_or_else(|| {
            panic!(
                "nothing this file can read said anything about the {id} fixture, so its \
                 severity was not measured and must not be asserted"
            )
        })
}

#[test]
fn every_release_blocking_case_with_a_fixture_app_meets_the_manifest() {
    let mut measured: Vec<(String, Severity)> = Vec::new();
    for case in manifest_cases() {
        if !case.release_blocking || !fixture_has_an_app(&case.id) {
            continue;
        }
        let severity = measured_severity(&case.id);
        assert_eq!(
            severity.as_str(),
            case.expected_severity,
            "{}: the manifest requires `{}` and SURE reaches `{}`",
            case.id,
            case.expected_severity,
            severity.as_str()
        );
        measured.push((case.id, severity));
    }

    // The set, named. A release-blocking case that gained a fixture app, or one
    // of these that lost one, stops this test rather than silently shrinking what
    // the assertion above covers. `force-push` and `sensitive-read` are
    // release-blocking and have no fixture directory at all; `check-crash`,
    // `dangerous-delete`, `repair-regression`, `stale-test-evidence` and
    // `tests-not-run` are stub directories holding a descriptor and nothing to
    // run. None of them can be measured by running anything, and this test does
    // not pretend otherwise.
    let ids: Vec<&str> = measured.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "fake-payment",
            "fake-auth",
            "fake-email",
            "dead-button",
            "missing-migration",
            "route-mismatch"
        ],
        "the release-blocking cases with a fixture app changed"
    );
    for (id, severity) in &measured {
        assert_eq!(
            *severity,
            Severity::MustFix,
            "{id}: the manifest asks for `must_fix` on this case"
        );
    }
}

#[test]
fn severity_was_not_bought_by_promoting_an_inference_to_a_fact() {
    // The four cases the brief names, plus `fake-email`: the five release-blocking
    // fixtures the candidate detectors are about. Every one of them reaches
    // `must_fix` while every proposal behind that still reports
    // `EvidenceClass::Inference`.
    for id in [
        "fake-payment",
        "fake-auth",
        "fake-email",
        "dead-button",
        "route-mismatch",
    ] {
        let (worst, classes) = detector_reading(id);
        assert_eq!(
            worst,
            Some(Severity::MustFix),
            "{id} no longer reaches the manifest's severity"
        );
        assert!(
            !classes.is_empty(),
            "{id}: a severity was asserted without a proposal behind it"
        );
        for class in classes {
            assert_eq!(
                class,
                EvidenceClass::Inference,
                "{id}: the severity moved and the evidence class moved with it. A \
                 pattern guess presented as an observation is the false green this \
                 product exists to catch, and `{}` is what the fixture's own \
                 scenario.json requires",
                class.as_str()
            );
            assert!(
                !class.can_alone_support_must_fix(),
                "{id}: this test's premise is that the class cannot carry `must_fix` \
                 on its own, so the weight has to come from somewhere else"
            );
        }
    }

    // Where the weight does come from: the anchor. Every proposal behind those
    // severities names a file and a line a reader can open, which is the
    // grounding the severity claims and the half that a relabelled inference
    // would not have.
    let proposals = candidate_proposals(&discovery("fake-payment"));
    assert!(!proposals.is_empty());
    for proposal in &proposals {
        let anchor = proposal
            .reason()
            .anchor()
            .unwrap_or_else(|| panic!("{}: names nothing, so it is not an anchor", proposal.id()));
        assert!(
            anchor.is_checkable(),
            "{}: the severity rests on an anchor a reader cannot check: {anchor:?}",
            proposal.id()
        );
        assert!(
            anchor.location.contains("src/payments.js"),
            "{}: the severity is about the payment path and the anchor is elsewhere: {}",
            proposal.id(),
            anchor.location
        );
    }
}

#[test]
fn the_benign_cases_did_not_escalate() {
    // `demo-analytics` is the corpus's `should_fix_first` case that the candidate
    // scanners are about: six required outcomes, none of them release-blocking.
    // It moved from `note` to the manifest's `should_fix_first` and no further.
    let (worst, classes) = detector_reading("demo-analytics");
    assert_eq!(
        worst,
        Some(Severity::ShouldFixFirst),
        "demo-analytics is the benign case the rule must not escalate past the manifest"
    );
    let manifest = manifest_cases()
        .into_iter()
        .find(|case| case.id == "demo-analytics")
        .expect("the manifest states demo-analytics");
    assert_eq!(worst.unwrap().as_str(), manifest.expected_severity);
    assert!(!manifest.release_blocking);
    assert!(
        classes
            .iter()
            .all(|class| *class == EvidenceClass::Inference)
    );

    // `benign-test-mocks` is the case that names a test-only mock. Its fixture is
    // a stub, so its `note` cannot be measured by running anything — and that is
    // asserted here rather than left as a sentence, because a test that quietly
    // covered it with `demo-analytics` would look like a measurement.
    let benign = manifest_cases()
        .into_iter()
        .find(|case| case.id == "benign-test-mocks")
        .expect("the manifest states benign-test-mocks");
    assert_eq!(benign.expected_severity, "note");
    assert!(!benign.release_blocking);
    assert!(
        !fixture_has_an_app("benign-test-mocks"),
        "benign-test-mocks gained a fixture app: measure its severity instead of \
         asserting the shape below"
    );
    assert!(
        !fixture_has_an_app("lying-readme"),
        "lying-readme gained a fixture app: measure its severity instead of asserting \
         the shape below"
    );

    // What can be measured is the guarantee the case is about, on the shape the
    // case describes: a mock in a test file and the same word in shipped source
    // are two different findings, and neither is `must_fix` by itself.
    let scratch = Scratch::new("benign-test-mocks-shape");
    scratch
        .write(
            "src/checkout.js",
            "'use strict';\n\nfunction charge() {\n  return { ok: true };\n}\n\nmodule.exports = { charge: charge };\n",
        )
        .write(
            "tests/checkout.mock.js",
            "'use strict';\n\n// Mock of the payment gateway for the tests below.\nconst mockGateway = { charge: function () { return { ok: true }; } };\n\nmodule.exports = { mockGateway: mockGateway };\n",
        );
    let proposals = candidate_proposals(&scratch.discovery());
    assert!(
        !proposals.is_empty(),
        "the constructed shape produced nothing, so it measures nothing"
    );
    let test_only: Vec<&CheckProposal> = proposals
        .iter()
        .filter(|proposal| {
            proposal
                .reason()
                .anchor()
                .is_some_and(|anchor| anchor.location.contains("tests/"))
        })
        .collect();
    assert!(
        !test_only.is_empty(),
        "the mock in the test file was not detected, so the guarantee was not measured: \
         {proposals:?}"
    );
    for proposal in &test_only {
        assert_eq!(
            proposal.severity(),
            Severity::Note,
            "{}: a finding about a test-only mock escalated",
            proposal.id()
        );
    }
    let aggregated = aggregate(proposals);
    assert!(
        aggregated
            .material()
            .iter()
            .all(|proposal| proposal.severity() != Severity::MustFix),
        "a test-only mock reached `must_fix`: {:?}",
        aggregated.material()
    );
}

/// A project under the workspace's git-ignored `target/tmp`.
///
/// Unique per call and never cleared, which is the pattern the other tests in
/// this crate settled on: clearing a fixed path and treating it as fresh fails on
/// Windows, and the test then describes a directory that was never emptied.
struct Scratch {
    project: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("finding severity rule controls");
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
        panic!("no free control name under {}", base.display());
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
        discover(&self.project, &DiscoverOptions::default())
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", self.project.display()))
    }
}

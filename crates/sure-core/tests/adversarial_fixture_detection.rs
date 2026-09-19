//! The TypeScript adversarial fixtures make the scanners say what they claim.
//!
//! `P14-T001`'s acceptance is one sentence:
//!
//! > *Fake payment/auth/email/dead button/route/demo cases runnable with
//! > expected outcomes.*
//!
//! A fixture directory and a `scenario.json` prove nothing on their own. The
//! files under `fixtures/adversarial/*/` are inputs, and the claim each one
//! makes is that **SURE's own scanners fire on it** — that the defect is
//! discoverable by the product as it is, not by a reader with the README open.
//! This file is that claim, checked against the real scanners reading the real
//! fixture directories.
//!
//! # Why the fixtures are read from the checkout rather than rebuilt in a temp
//! # directory
//!
//! Every other test in this crate writes its project into `target/tmp` so the
//! test owns it. These five scanners are being asked about **the fixtures the
//! product ships**, and a copy would be a different project the day someone
//! edits one and not the other. The directory is the artefact; reading it where
//! it lives is what makes this test able to fail when the artefact stops
//! working.
//!
//! # What is paired with what
//!
//! A scanner that reported everything would satisfy every "this fixture fires"
//! assertion in this file, so each positive assertion is paired with a negative
//! one drawn from the same fixture:
//!
//! - `route-mismatch` declares `/api/health` and *calls* `/api/health`, and the
//!   route checker must stay silent about it while reporting `/api/orders`. A
//!   checker that flagged every frontend path would pass the positive half for
//!   the wrong reason.
//! - `demo-analytics` must produce exactly the four demo-data categories. A
//!   module that flagged every literal would produce them among a crowd.
//! - A project holding none of these patterns must produce nothing at all from
//!   all five scanners, which is the control the whole file rests on.
//!
//! # The Python half, and how it differs
//!
//! `P14-T002` added `missing-migration`, `external-unverified` and
//! `missing-config` under the same directory. They are not driven by the five
//! scanners above; each one fires a different module, and the differences are
//! the point:
//!
//! - `missing-migration` fires [`sure_core::db_migrations`], the only check in
//!   this file whose evidence is `EvidenceClass::ObservedFact` rather than
//!   `Inference`. It reaches `Severity::MustFix` on that observed fact, not
//!   through [`sure_core::finding_gravity`], and it is asserted rather than
//!   assumed.
//! - `external-unverified` fires [`sure_core::external_service`], whose
//!   proposal is `Severity::ShouldFixFirst` with `critical = true` — heavier
//!   than the manifest's `note`, and *not* filed as style noise, because
//!   `is_style_noise` moves only `Note` candidates. What the manifest actually
//!   asks for here is the *outcome*, `cannot_confirm`/`not_checked`, and that is
//!   what `ExternalServiceChecks::not_checked` produces.
//! - `missing-config` fires [`sure_core::env_completeness`] over
//!   [`sure_core::references`]. It has **no case in
//!   `evaluation/acceptance-manifest.json`**, which its `scenario.json` records,
//!   and its severity is asserted against the detector rather than against the
//!   manifest because there is no manifest line to assert it against.
//!
//! # The claim-evidence half, and how it differs
//!
//! `P14-T004` added `tests-not-run`, `stale-test-evidence` and
//! `unknown-evidence` under the same directory. They fire no scanner: each is a
//! **recording** — a claim an AI made and the harness events behind it — and the
//! recording is declared in the fixture's own `scenario.json` as the event
//! documents a harness would send. What is compared here is what SURE says about
//! it, and the three differences from the halves above are the point:
//!
//! - The input is the fixture's own declarations, ingested through
//!   [`sure_core::harness_event::ingest_event_str`] — the entry point a harness's
//!   JSON takes — and stored through the session event store, so a document the
//!   wire refuses is a failing test rather than a document nobody checked.
//! - Every fact asserted comes out of the scenario rather than being restated
//!   here: the assessment, the sentence a reader is shown, the number of
//!   evidence items, the anchor, the class and the freshness are all compared
//!   against the `expect` block, and the rendered `AI claims` section is compared
//!   whole. A fixture edited without this file being edited is a red test.
//! - Each is read twice — once through the store, which is what `sure check`
//!   reads, and once through [`check_claims_against_events`] over the same events
//!   in the order that function documents — and the two answers must be equal.
//!
//! And each has a control: the same recording with exactly one field of one event
//! moved, whose answer must be `confirmed`. That is the one control this half
//! cannot do without. A checker that answered `cannot_confirm` to everything
//! would satisfy all three of these fixtures' required outcomes at once, and the
//! corpus would look green while measuring nothing; the controls are what make
//! the three answers a measurement of the recordings rather than of the checker.
//!
//! # What is not claimed
//!
//! **A severity of its own, for the six TypeScript fixtures.** No scanner in
//! this file holds one. What each emits is [`sure_core::finding_gravity`]'s
//! answer for the gap it found and the reach it found it on: `must_fix` for a
//! substituted action in production code, `should_fix_first` for unreal content,
//! `note` for an unfinished marker and for anything that names nothing, points
//! at a place a reader cannot open, rests on an `Unknown` result, or cannot
//! reach a user. Until `P7-T011` every scanner here emitted `Severity::Note`,
//! `critical = false`, `EvidenceClass::Inference` — the combination
//! [`sure_core::false_completion_aggregator`] filed as style noise — while
//! `evaluation/acceptance-manifest.json` asked for `must_fix` on five of those
//! six ids. The test that recorded that gap was
//! `what_is_detected_is_not_yet_what_the_manifest_requires`;
//! [`what_is_detected_now_carries_the_weight_the_manifest_requires`] is that
//! test inverted, and the sentence is kept in both places because it is the only
//! statement in the tree that the two numbers were ever different.
//!
//! **A raised evidence class.** The severity moved and the class did not:
//! `EvidenceClass::Inference` is what every one of these findings' own
//! `scenario.json` requires, and it is what they still report. A scanner that
//! reached `must_fix` by presenting its inference as a fact would be relabelling
//! rather than grading, and [`sure_core::finding_gravity`]'s own tests refuse
//! that move.
//!
//! **A verdict.** These are candidates. `CheckProposal::severity` is how bad it
//! would be if the check is not satisfied, and no scanner here sets `critical`,
//! so a `must_fix` candidate decides nothing about whether a project passes —
//! what it decides is whether the candidate is shown rather than filed as noise.
//! The weight was not free: `noop_heuristics`' hard-coded-success shape and
//! `ui_action_bridge`'s declared-handler shape both occur in ordinary, working
//! code, and they are now weighted as if a user's money or their primary action
//! is behind them. That cost is deliberate and is recorded at the detectors.
//!
//! **A password check that is always true.** SURE has no detector that reads
//! `verifyPassword` returning `true` as an authentication bypass.
//! `fake-auth` fires on what surrounds the bypass, and
//! `fake_auth_is_detected_by_what_surrounds_the_bypass` says which parts, so a
//! reader does not take the passing test for a capability SURE does not have.
//!
//! **A Python integration that imports its provider.** Two limits of
//! [`sure_core::external_service`] are measured here rather than described:
//! `import stripe` in a `.py` file and `stripe==11.0.0` in `requirements.txt`
//! each produce no proposal at all, because the import and dependency tables are
//! JavaScript-shaped. `external-unverified` fires on its `STRIPE_` environment
//! prefix, and `a_python_provider_import_on_its_own_is_not_detected` holds that
//! difference so nobody reads the passing assertion above as a capability this
//! check does not have.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};
use sure_core::candidate_scanner::CandidateScanner;
use sure_core::claim_capture::{ClaimCaptureOutcome, capture_agent_claim};
use sure_core::claim_checker::{
    CheckedClaim, ClaimDocument, check_claims_against_events, check_claims_in_store,
};
use sure_core::claim_report::render_claim_section;
use sure_core::db_migrations::{MigrationsReport, Record};
use sure_core::demo_data_heuristics::DemoDataHeuristics;
use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::env_completeness::CompletenessReport;
use sure_core::evidence::{
    AnchorSubject, ClaimAssessment, EvidenceClass, Freshness, StalenessReason,
};
use sure_core::external_service::ExternalServiceChecks;
use sure_core::false_completion_aggregator::aggregate;
use sure_core::harness_event::{IngestedEvent, ingest_event_str};
use sure_core::ids::{ClaimId, EventId, FingerprintId};
use sure_core::noop_heuristics::NoOpHeuristics;
use sure_core::recording_projection::{BuildTestKind, StandardProjection, project};
use sure_core::references::KeyStatus;
use sure_core::route_consistency::RouteConsistency;
use sure_core::session_event_store::SessionEventStore;
use sure_core::severity::Severity;
use sure_core::status::{CheckStatus, NotCheckedReason};
use sure_core::store::Store;
use sure_core::ui_action_bridge::UiActionBridge;
use sure_core::vocabulary::Claim;

/// Every fixture `P14-T001` is about, and the language half it belongs to.
const TYPESCRIPT_FIXTURES: &[&str] = &[
    "fake-payment",
    "fake-auth",
    "fake-email",
    "dead-button",
    "demo-analytics",
    "route-mismatch",
];

/// Every fixture `P14-T002` is about, named rather than discovered for the same
/// reason the list above is: a test that discovered them would pass on an empty
/// directory.
const PYTHON_FIXTURES: &[&str] = &["missing-migration", "external-unverified", "missing-config"];

/// Every fixture `P14-T004` is about: a recorded AI claim and the events behind
/// it, with no project in either.
///
/// Named rather than discovered, for the reason the lists above are. They are
/// also named in `crates/sure-testkit/tests/fixture_apps.rs`, which checks the
/// scenarios as artefacts; this file is where their outcomes are graded.
const CLAIM_FIXTURES: &[&str] = &["tests-not-run", "stale-test-evidence", "unknown-evidence"];

/// The project root the declared recordings are attributed to.
///
/// A Windows path on purpose: this repository's primary environment is Windows
/// and the root is part of what the recorded session is stored under, so the
/// path worth exercising is one with backslashes in it.
const CLAIM_PROJECT_ROOT: &str = "C:\\work\\shop";

fn fixture(id: &str) -> PathBuf {
    sure_testkit::repository_root()
        .join("fixtures")
        .join("adversarial")
        .join(id)
}

/// A project under the workspace's git-ignored `target/tmp`, for the controls.
///
/// These are the negative half of each Python fixture's claim, and they are
/// built here rather than shipped as fixtures because a control is not a
/// scenario the corpus has a case for. Unique per call and never cleared, which
/// is the pattern `tests/db_migrations.rs` settled on: clearing a fixed path and
/// treating it as fresh fails on Windows, and the test then describes a
/// directory that was never emptied.
struct Scratch {
    project: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Self {
        Self::under("adversarial python controls", test)
    }

    /// The same, under a base of its own.
    ///
    /// The claim fixtures store recordings rather than writing projects, so
    /// their scratch directories are named for what they hold; the directory
    /// discipline is the one above and is not restated.
    fn under(base_name: &str, test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join(base_name);
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

/// What SURE reads out of one shipped fixture.
fn discovery(id: &str) -> sure_core::discover::Discovery {
    let root = fixture(id);
    assert!(
        root.is_dir(),
        "the {id} fixture directory is missing: {}",
        root.display()
    );
    discover(&root, &DiscoverOptions::default())
        .unwrap_or_else(|error| panic!("cannot read the {id} fixture: {error}"))
}

fn candidate_titles(id: &str) -> Vec<String> {
    CandidateScanner::of(&discovery(id))
        .proposed()
        .iter()
        .map(|proposal| proposal.title().to_owned())
        .collect()
}

fn noop_titles(id: &str) -> Vec<String> {
    NoOpHeuristics::of(&discovery(id))
        .proposed()
        .iter()
        .map(|proposal| proposal.title().to_owned())
        .collect()
}

fn demo_titles(id: &str) -> Vec<String> {
    DemoDataHeuristics::of(&discovery(id))
        .proposed()
        .iter()
        .map(|proposal| proposal.title().to_owned())
        .collect()
}

fn route_titles(id: &str) -> Vec<String> {
    RouteConsistency::of(&discovery(id))
        .proposed()
        .iter()
        .map(|proposal| proposal.title().to_owned())
        .collect()
}

fn ui_action_titles(id: &str) -> Vec<String> {
    UiActionBridge::of(&discovery(id))
        .proposals()
        .iter()
        .map(|proposal| proposal.title().to_owned())
        .collect()
}

fn external_titles(id: &str) -> Vec<String> {
    ExternalServiceChecks::of(&discovery(id))
        .proposed()
        .iter()
        .map(|proposal| proposal.title().to_owned())
        .collect()
}

fn migrations_of(id: &str) -> MigrationsReport {
    MigrationsReport::of(&discovery(id), &FingerprintId::generate())
}

fn completeness_of(id: &str) -> CompletenessReport {
    CompletenessReport::of(&discovery(id), &FingerprintId::generate())
}

/// Every key one of the Python fixtures asks the environment for, with the
/// status, verdict and severity SURE reached about it.
fn keys_of(id: &str) -> Vec<(String, KeyStatus, ClaimAssessment, Severity)> {
    completeness_of(id)
        .claims()
        .iter()
        .map(|claim| {
            (
                claim.key().to_owned(),
                claim.status(),
                claim.assessment(),
                claim.severity(),
            )
        })
        .collect()
}

/// Every scanner `P14-T001` put on the TypeScript fixtures, in one list.
///
/// Used here as the *silence* side of the Python fixtures: a fixture that fired
/// six checks at once would satisfy every positive assertion about one of them
/// while telling a reader nothing about which check the fixture measures.
fn every_typescript_scanner_says(id: &str) -> Vec<(&'static str, Vec<String>)> {
    vec![
        ("candidate_scanner", candidate_titles(id)),
        ("noop_heuristics", noop_titles(id)),
        ("demo_data_heuristics", demo_titles(id)),
        ("route_consistency", route_titles(id)),
        ("ui_action_bridge", ui_action_titles(id)),
        ("external_service", external_titles(id)),
    ]
}

/// Assert one scanner said one thing about this fixture.
fn assert_reports(what: &str, said: &[String], expected: &str) {
    assert!(
        said.iter().any(|title| title == expected),
        "{what} did not report it.\n  expected: {expected}\n  it said:  {said:?}"
    );
}

/// Assert one scanner did not say one thing about this fixture.
fn assert_silent(what: &str, said: &[String], unexpected: &str) {
    assert!(
        !said.iter().any(|title| title == unexpected),
        "{what} said something it must not.\n  unexpected: {unexpected}\n  it said:    {said:?}"
    );
}

// --- each fixture fires the scanners its scenario names ------------------

#[test]
fn fake_payment_is_detected_as_a_fake_payment_path() {
    let noop = noop_titles("fake-payment");
    let candidates = candidate_titles("fake-payment");

    assert_reports(
        "noop_heuristics on fake-payment",
        &noop,
        "project contains fake payment or sandbox tokens in production code",
    );
    assert_reports(
        "noop_heuristics on fake-payment",
        &noop,
        "project contains hard-coded success responses in production code",
    );
    assert_reports(
        "noop_heuristics on fake-payment",
        &noop,
        "project contains no-op functions that always succeed in production code",
    );
    assert_reports(
        "candidate_scanner on fake-payment",
        &candidates,
        "project contains mock usage in production code",
    );
}

#[test]
fn fake_auth_is_detected_by_what_surrounds_the_bypass() {
    // The bypass itself — `verifyPassword` returning `true` for any password —
    // is what this fixture is about and what no detector reads. What is
    // asserted here is the evidence around it, and the comment at the top of
    // this file says so rather than letting this test imply a capability SURE
    // does not have.
    let noop = noop_titles("fake-auth");
    let candidates = candidate_titles("fake-auth");

    assert_reports(
        "noop_heuristics on fake-auth",
        &noop,
        "project contains hard-coded success responses in production code",
    );
    assert_reports(
        "noop_heuristics on fake-auth",
        &noop,
        "project contains fake email addresses or domains in production code",
    );
    assert_reports(
        "candidate_scanner on fake-auth",
        &candidates,
        "project contains TODO or FIXME comments in production code",
    );
    assert_reports(
        "candidate_scanner on fake-auth",
        &candidates,
        "project contains mock usage in production code",
    );
}

#[test]
fn fake_email_is_detected_as_console_only() {
    let noop = noop_titles("fake-email");
    let candidates = candidate_titles("fake-email");

    assert_reports(
        "noop_heuristics on fake-email",
        &noop,
        "project contains fake email addresses or domains in production code",
    );
    assert_reports(
        "noop_heuristics on fake-email",
        &noop,
        "project contains hard-coded success responses in production code",
    );
    assert_reports(
        "noop_heuristics on fake-email",
        &noop,
        "project contains no-op functions that always succeed in production code",
    );
    assert_reports(
        "candidate_scanner on fake-email",
        &candidates,
        "project contains TODO or FIXME comments in production code",
    );
}

#[test]
fn dead_button_is_detected_as_a_wired_action_with_no_behaviour() {
    let ui = ui_action_titles("dead-button");
    let candidates = candidate_titles("dead-button");

    assert!(
        ui.iter()
            .any(|title| title.starts_with("UI action: click handler on ")),
        "ui_action_bridge did not find the Buy now click binding: {ui:?}"
    );
    assert_reports(
        "candidate_scanner on dead-button",
        &candidates,
        "project contains placeholder usage in production code",
    );
    assert_reports(
        "candidate_scanner on dead-button",
        &candidates,
        "project contains TODO or FIXME comments in production code",
    );
}

#[test]
fn demo_analytics_is_detected_as_demo_data_and_nothing_else() {
    // Exact, not "contains": a module that flagged every literal in the project
    // would produce these four among a crowd, and the point of `P6-T004` is
    // that a retry count is not a metric.
    let mut demo = demo_titles("demo-analytics");
    demo.sort();

    assert_eq!(
        demo,
        [
            "project contains hard-coded demo analytics values in production code",
            "project contains hard-coded demo or chart values in production code",
            "project contains hard-coded demo or sample datasets in production code",
            "project contains placeholder user or content IDs in production code",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<String>>(),
        "the demo-data reading of demo-analytics changed"
    );
}

#[test]
fn route_mismatch_is_detected_and_the_matching_route_is_left_alone() {
    let routes = route_titles("route-mismatch");

    assert_eq!(
        routes,
        ["frontend expects backend path `/api/orders` which is not declared"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<String>>(),
        "the route-consistency reading of route-mismatch changed"
    );

    // The control. `/api/health` is declared by the backend and called by the
    // frontend, and a checker that reported it would be flagging a working
    // route — which is the failure that makes a real mismatch ignorable.
    assert_silent(
        "route_consistency on route-mismatch",
        &routes,
        "frontend expects backend path `/api/health` which is not declared",
    );
}

#[test]
fn the_route_mismatch_fixture_is_otherwise_clean() {
    // If the mismatch fixture also tripped every other scanner, its
    // `scenario.json` would be naming one of six findings and the eval runner
    // would have no way to tell which one it measured.
    for (what, said) in [
        ("candidate_scanner", candidate_titles("route-mismatch")),
        ("noop_heuristics", noop_titles("route-mismatch")),
        ("demo_data_heuristics", demo_titles("route-mismatch")),
    ] {
        assert!(
            said.is_empty(),
            "{what} reported something about the route-mismatch fixture: {said:?}"
        );
    }
}

// --- the Python fixtures, each firing a different module ----------------

/// Assert the six `P14-T001` scanners say nothing about a Python fixture.
///
/// Each fixture in this half measures one module, and a fixture that tripped
/// the other five would be measuring nothing: a reader could not tell which
/// check the case is about, and an eval runner comparing outcomes would have no
/// way to tell which one it read.
fn assert_the_typescript_scanners_are_silent(id: &str) {
    for (what, said) in every_typescript_scanner_says(id) {
        assert!(
            said.is_empty(),
            "{what} reported something about the {id} fixture: {said:?}"
        );
    }
}

#[test]
fn missing_migration_is_detected_as_a_record_that_is_there_and_empty() {
    let report = migrations_of("missing-migration");

    // The reading first, because the severity below rests on which of the two
    // shapes this is. `Record::Empty` is the project that writes its schema
    // down, uses migrations, and has no revision; `Record::Absent` is the
    // weaker one and carries `ShouldFixFirst` instead.
    let looked: Vec<_> = report.survey().looked().iter().collect();
    assert_eq!(looked.len(), 1, "{:?}", report.survey().looked());
    assert_eq!(looked[0].framework(), "Alembic");
    assert_eq!(looked[0].shape(), Path::new("alembic.ini"));
    assert_eq!(looked[0].record_at(), Path::new("alembic/versions"));
    assert_eq!(looked[0].record(), Record::Empty);

    let claims: Vec<_> = report.claims().iter().collect();
    assert_eq!(claims.len(), 1, "{:?}", report.claims());
    let claim = claims[0];
    assert_eq!(claim.framework(), "Alembic");
    assert_eq!(claim.record(), Record::Empty);
    assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(claim.severity(), Severity::MustFix);
    assert!(
        claim.severity().blocks_hand_off(),
        "this fixture's detector reaches the manifest's `must_fix` on an observed fact \
         rather than through the candidate rule"
    );
    assert!(
        report.is_complete(),
        "a Confirmed verdict rests on SURE having finished reading: {}",
        report.plain_description()
    );

    // A must_fix that nothing stands behind is the false green this product
    // exists to prevent, so the two anchors are pinned: where the framework was
    // read and where its record was looked for. Both carry no excerpt, and that
    // emptiness is asserted rather than tolerated — what is behind them is a
    // path SURE listed, not a line of the project it could quote.
    let mut locations: Vec<String> = claim
        .evidence()
        .iter()
        .map(|evidence| {
            assert_eq!(evidence.class, EvidenceClass::ObservedFact);
            assert_eq!(evidence.severity, Severity::MustFix);
            assert_eq!(
                evidence.anchor.subject,
                AnchorSubject::Database,
                "the variant that exists for a schema and a migration is the one to use"
            );
            assert!(evidence.anchor.is_checkable(), "{:?}", evidence.anchor);
            assert!(evidence.fingerprint.is_some());
            assert!(
                evidence.anchor.excerpt.is_empty(),
                "an anchor here is a path SURE read, and there is no line to quote: {:?}",
                evidence.anchor
            );
            evidence.anchor.location.clone()
        })
        .collect();
    locations.sort();
    assert_eq!(
        locations,
        ["alembic.ini", "alembic/versions"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<String>>(),
        "a reader who disagrees has to be able to go and look at both halves"
    );

    // And the sentence names both halves for the same reason.
    assert!(
        claim.reason().contains("alembic.ini") && claim.reason().contains("alembic/versions"),
        "{}",
        claim.reason()
    );

    // None of the five TypeScript scanners has anything to say about this
    // project, which is what makes the one finding the fixture's own.
    assert_the_typescript_scanners_are_silent("missing-migration");
}

#[test]
fn an_alembic_record_that_holds_a_revision_makes_no_claim() {
    // The control. A check that answered `MustFix` to every project with an
    // `alembic.ini` would satisfy the test above while reporting on projects
    // whose migrations are in order, which is the failure that makes a real
    // finding ignorable.
    let project = Scratch::new("alembic-with-a-revision");
    project
        .write("alembic.ini", "[alembic]\nscript_location = alembic\n")
        .write(
            "alembic/versions/0001_create_orders.py",
            "revision = \"0001\"\ndown_revision = None\n",
        );

    let report = MigrationsReport::of(&project.discovery(), &FingerprintId::generate());
    assert!(
        report.claims().is_empty(),
        "a project with a revision made a claim: {:?}",
        report.claims()
    );
    let records: Vec<Record> = report
        .survey()
        .looked()
        .iter()
        .map(|look| look.record())
        .collect();
    assert_eq!(
        records,
        [Record::Holds(1)],
        "the control has to be read as a project with a migration, or it controls for nothing"
    );
}

#[test]
fn external_unverified_is_reported_as_not_checked_and_never_as_passed() {
    let discovery = discovery("external-unverified");
    let checks = ExternalServiceChecks::of(&discovery);

    // What is proposed, and at what weight. `ShouldFixFirst` with
    // `critical = true` and `Inference` — heavier than the `note` the manifest
    // asks for, and *not* filed as style noise, because
    // `false_completion_aggregator::is_style_noise` moves only `Note`
    // candidates. The mismatch is recorded in the fixture's `scenario.json`
    // and pinned here rather than smoothed over.
    let proposals = checks.proposed();
    assert_eq!(proposals.len(), 1, "{proposals:?}");
    let proposal = &proposals[0];
    assert_eq!(proposal.title(), "project uses an external payment service");
    assert_eq!(proposal.severity(), Severity::ShouldFixFirst);
    assert!(proposal.critical());
    assert_eq!(proposal.evidence_class(), EvidenceClass::Inference);
    assert!(
        !proposal.severity().blocks_hand_off(),
        "the manifest asks for `note` here, so nothing may say this stops a hand-off on its own"
    );

    // The other half of the mismatch the scenario records: because the severity
    // is not `Note`, the aggregator does *not* file this as style noise, so the
    // proposal reaches a reader as material. That is heavier than the manifest
    // asks for, and it is recorded rather than smoothed over.
    let aggregated = aggregate(checks.proposed().to_vec());
    assert_eq!(
        aggregated.material().len(),
        1,
        "the payment proposal is no longer material: {:?} vs {:?}",
        aggregated.material(),
        aggregated.style_noise()
    );
    assert!(aggregated.style_noise().is_empty());
    let anchor = proposal
        .reason()
        .anchor()
        .expect("the reason names the file it was read from");
    assert_eq!(anchor.subject, AnchorSubject::File);
    assert_eq!(anchor.location, "app/payments.py");

    // What the proposal turns into, and the outcome the manifest does ask for:
    // an external service can only be confirmed against the real service, so
    // the check is skipped with its own reason and never reported as a pass.
    let results = checks.not_checked(&FingerprintId::generate());
    assert_eq!(results.len(), 1, "{results:?}");
    let result = &results[0];
    assert_eq!(result.status, CheckStatus::Skipped);
    assert_ne!(result.status, CheckStatus::Pass);
    assert_eq!(
        result.not_checked_reason,
        Some(NotCheckedReason::ExternalServiceUnavailable)
    );
    assert_eq!(result.evidence_class, EvidenceClass::Unknown);
    assert!(result.critical);
    assert!(result.is_not_checked());
    assert!(!result.title.is_empty());

    // The same fixture's second surface. These two keys are what fires the
    // payment proposal at all — the provider is named by nothing else in the
    // project — and they are a finding in their own right.
    assert_eq!(
        keys_of("external-unverified"),
        ["STRIPE_API_BASE", "STRIPE_SECRET_KEY",]
            .into_iter()
            .map(|key| (
                key.to_owned(),
                KeyStatus::ReadButNotDeclared,
                ClaimAssessment::Confirmed,
                Severity::ShouldFixFirst,
            ))
            .collect::<Vec<_>>(),
        "the keys SURE read out of this fixture changed"
    );

    // The five scanners that are not this fixture's, silent, and no migration
    // claim either: the fixture measures one module and says so.
    for (what, said) in every_typescript_scanner_says("external-unverified") {
        if what == "external_service" {
            continue;
        }
        assert!(
            said.is_empty(),
            "{what} reported something about the external-unverified fixture: {said:?}"
        );
    }
    assert!(
        migrations_of("external-unverified").claims().is_empty(),
        "this fixture has no database framework and must make no migration claim"
    );
}

#[test]
fn a_python_provider_import_on_its_own_is_not_detected() {
    // The first of the two limits `external-unverified`'s `scenario.json`
    // records, measured rather than described: the dependency and import
    // tables in `external_service` are JavaScript-shaped, so a Python app that
    // names its provider by importing the provider's own client, and whose
    // requirements file lists that client, produces nothing at all.
    let imports = Scratch::new("python-provider-import");
    imports
        .write(
            "app/paid.py",
            "import stripe\n\nBASE = \"https://api.stripe.com\"\n\n\ndef charge(cents):\n    return {\"amount\": cents, \"currency\": \"usd\"}\n",
        )
        .write("requirements.txt", "stripe==11.0.0\n");
    assert!(
        ExternalServiceChecks::of(&imports.discovery()).is_empty(),
        "a Python provider import now fires external_service, so the fixture's recorded limit changed"
    );

    // The same project with one Stripe-prefixed environment read, which is what
    // the shipped fixture carries and what the limit above is not. Without
    // this half, the assertion above would pass for a check that had stopped
    // working on everything.
    let env = Scratch::new("python-provider-env");
    env.write(
        "app/paid.py",
        "import os\n\nBASE = os.getenv(\"STRIPE_API_BASE\") or \"https://api.stripe.com\"\n",
    );
    let checks = ExternalServiceChecks::of(&env.discovery());
    assert_eq!(
        checks.proposed().len(),
        1,
        "the STRIPE_ environment prefix is what this check reads in a Python project, and it stopped reading it: {:?}",
        checks.proposed()
    );
    assert_eq!(
        checks.proposed()[0].title(),
        "project uses an external payment service"
    );
}

#[test]
fn missing_config_is_detected_as_keys_asked_for_and_named_nowhere() {
    let report = completeness_of("missing-config");

    assert_eq!(
        keys_of("missing-config"),
        ["ORDERS_DB_URL", "ORDERS_QUEUE_URL", "REPORTING_BUCKET"]
            .into_iter()
            .map(|key| (
                key.to_owned(),
                KeyStatus::ReadButNotDeclared,
                ClaimAssessment::Confirmed,
                Severity::ShouldFixFirst,
            ))
            .collect::<Vec<_>>(),
        "the keys SURE read out of this fixture changed"
    );
    assert!(
        report.is_complete(),
        "a Confirmed verdict rests on SURE having finished reading: {}",
        report.plain_description()
    );

    // Where each key was read, and nothing else. Two read sites per key: the
    // settings module the app reads through, and the fixture's own check, which
    // supplies the three names itself — which is exactly what makes the check a
    // trap rather than evidence. No anchor carries an excerpt, because SURE
    // read the name and not the value and there is no line to quote.
    let mut locations: Vec<String> = Vec::new();
    for claim in report.claims() {
        assert!(
            !claim.evidence().is_empty(),
            "{}",
            claim.plain_description()
        );
        assert!(!claim.is_set_aside());
        assert_eq!(claim.severity(), Severity::ShouldFixFirst);
        assert!(
            !claim.severity().blocks_hand_off(),
            "env_completeness documents that no missing line of documentation stops a hand-off"
        );
        for evidence in claim.evidence() {
            assert_eq!(evidence.class, EvidenceClass::ObservedFact);
            assert_eq!(evidence.anchor.subject, AnchorSubject::LineRange);
            assert!(evidence.anchor.is_checkable(), "{:?}", evidence.anchor);
            assert!(
                evidence.anchor.excerpt.is_empty(),
                "SURE reads the key name and never the value: {:?}",
                evidence.anchor
            );
            locations.push(evidence.anchor.location.clone());
        }
    }
    locations.sort();
    locations.dedup();
    assert_eq!(
        locations,
        ["app/settings.py", "scripts/check.py"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<String>>(),
        "the read sites behind this fixture's claims changed"
    );

    // And nothing else in the project has anything to say.
    assert_the_typescript_scanners_are_silent("missing-config");
    assert!(
        migrations_of("missing-config").claims().is_empty(),
        "this fixture has no database framework and must make no migration claim"
    );
}

#[test]
fn a_project_whose_sample_file_assigns_the_keys_makes_no_claim() {
    // The control. A check that answered `ShouldFixFirst` to every key a
    // project reads would satisfy the test above while reporting on projects
    // that document themselves, and the fixture's README would be a defect
    // report about nothing.
    let project = Scratch::new("env-example-names-the-keys");
    project
        .write(
            "app/settings.py",
            "import os\n\n\ndef database_url():\n    return os.environ[\"ORDERS_DB_URL\"]\n",
        )
        .write(".env.example", "ORDERS_DB_URL=postgres://localhost/shop\n");

    let report = CompletenessReport::of(&project.discovery(), &FingerprintId::generate());
    assert!(
        report.claims().is_empty(),
        "a project whose sample file names the key made a claim: {:?}",
        report.claims()
    );
}

#[test]
fn every_python_fixture_these_assertions_name_is_a_fixture_this_repository_ships() {
    // The same guard the list above has, for the same reason: a renamed
    // directory would make `discovery` panic above and every assertion would be
    // about a project nobody ships. The difference is the second half — a
    // Python fixture with a `package.json` would be picked up by the testkit's
    // `npm start` runnability sweep, which is a claim about Node this half
    // cannot honour.
    for id in PYTHON_FIXTURES {
        let dir = fixture(id);
        assert!(dir.is_dir(), "{} is not a directory", dir.display());
        assert!(
            dir.join("scenario.json").is_file(),
            "{id} has no scenario.json"
        );
        assert!(
            !dir.join("package.json").is_file(),
            "{id} is a Python fixture and must not be listed as runnable with `npm start`"
        );
        assert!(
            dir.join("README.md").is_file(),
            "{id} has no README.md, so nothing says in words what it traps"
        );
    }
    for id in TYPESCRIPT_FIXTURES {
        assert!(
            !PYTHON_FIXTURES.contains(id),
            "{id} is in both lists, so one of the two halves is misdescribed"
        );
    }
}

// --- the scanners are not vacuous ---------------------------------------

#[test]
fn a_project_with_none_of_these_patterns_produces_nothing() {
    // The check the rest of this file rests on. Every scanner here is a
    // pattern matcher over source lines, and a matcher that returned a
    // proposal per file would satisfy every positive assertion above.
    let scratch = sure_testkit::repository_root()
        .join("target")
        .join("tmp")
        .join("adversarial-fixture-control");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("src")).unwrap();
    std::fs::write(
        scratch.join("src").join("main.js"),
        "'use strict';\n\nconst MAX_RETRIES = 3;\n\nfunction retryCount() {\n  return MAX_RETRIES;\n}\n\nmodule.exports = { retryCount: retryCount };\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("package.json"),
        "{\n  \"name\": \"control\",\n  \"private\": true,\n  \"scripts\": { \"start\": \"node src/main.js\" }\n}\n",
    )
    .unwrap();

    let discovery = discover(&scratch, &DiscoverOptions::default()).unwrap();

    assert!(
        CandidateScanner::of(&discovery).is_empty(),
        "candidate_scanner fired on a project with no candidates"
    );
    assert!(
        NoOpHeuristics::of(&discovery).is_empty(),
        "noop_heuristics fired on a project with no fake-success patterns"
    );
    assert!(
        DemoDataHeuristics::of(&discovery).is_empty(),
        "demo_data_heuristics fired on a project with an ordinary constant"
    );
    assert!(
        RouteConsistency::of(&discovery).is_empty(),
        "route_consistency fired on a project with no routes"
    );
    assert!(
        UiActionBridge::of(&discovery).is_empty(),
        "ui_action_bridge fired on a project with no UI actions"
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn every_fixture_these_assertions_name_is_a_fixture_this_repository_ships() {
    // Guards the file against the way it would most quietly stop testing
    // anything: a renamed fixture directory, where `discovery` would panic
    // above and every assertion would be about a project nobody ships.
    for id in TYPESCRIPT_FIXTURES {
        let dir = fixture(id);
        assert!(dir.is_dir(), "{} is not a directory", dir.display());
        assert!(
            dir.join("scenario.json").is_file(),
            "{id} has no scenario.json"
        );
        assert!(
            dir.join("package.json").is_file(),
            "{id} has no package.json, so it is not runnable with `npm start`"
        );
    }
}

/// What every scanner in this file emitted for these fixtures before `P7-T011`.
///
/// Held as a value rather than only as a sentence, because it is the number the
/// inverted test below replaced and the number its failure message names. A
/// scanner that falls back to it is not failing for an unknown reason: this is
/// where it landed.
const SEVERITY_BEFORE_P7T011: Severity = Severity::Note;

#[test]
fn what_is_detected_now_carries_the_weight_the_manifest_requires() {
    // This is `what_is_detected_is_not_yet_what_the_manifest_requires`, the
    // test that recorded the gap between what SURE saw and what the corpus
    // required, **inverted rather than deleted** by `P7-T011`. It asserted the
    // opposite of everything below: every proposal was `Severity::Note`, not
    // critical, `Inference` — the combination
    // `false_completion_aggregator` files as style noise — while
    // `evaluation/acceptance-manifest.json` asks for `must_fix` on fake-payment
    // and four of its siblings. The old name and the old number are kept here
    // because this is the only statement in the tree that the two were ever
    // different.
    //
    // `noop_heuristics` is the probe. It reads the file that takes the money,
    // its proposals name `src/payments.js` and the line they were read from, and
    // `sure_core::finding_gravity` grades the gap it found — a substituted
    // action, in production code — `must_fix`. The two things that must *not*
    // have moved with it are asserted here beside it.
    let discovery = discovery("fake-payment");
    let proposals = NoOpHeuristics::of(&discovery).proposed().to_vec();
    assert!(
        !proposals.is_empty(),
        "the probe fixture produced nothing, so this test proves nothing"
    );
    for proposal in &proposals {
        assert_eq!(
            proposal.severity(),
            Severity::MustFix,
            "{}: the manifest requires `must_fix` on fake-payment and this is \
             `{:?}`. `{SEVERITY_BEFORE_P7T011:?}` is what every scanner here \
             emitted before P7-T011; if that is where this landed, the severity \
             rule in `sure_core::finding_gravity` was bypassed.",
            proposal.title(),
            proposal.severity(),
        );
        // The weight is about impact, not about how the finding was reached:
        // `Inference` is the class this fixture's `scenario.json` requires and
        // the class the evidence is, and raising the severity by promoting it to
        // a fact is the move this test would have to catch.
        assert_eq!(proposal.evidence_class(), EvidenceClass::Inference);
        // And a heavier candidate is still not a blocker. Nothing in this file
        // sets `critical`, so no candidate decides a verdict however it is
        // weighted.
        assert!(!proposal.critical());
    }

    // The weight reaches the reader. `material` non-empty is the half that says
    // the finding is shown; `style_noise` empty is the half that says it is not
    // filed under a name that tells a reader to skip it. Both used to be the
    // other way round, and that is the whole of what `P7-T011` changed.
    let aggregated = aggregate(proposals);
    assert!(
        !aggregated.material().is_empty(),
        "the probe fixture's findings reached neither list, so this test proves nothing"
    );
    assert!(
        aggregated.style_noise().is_empty(),
        "a payment path that never contacts a provider is filed as style noise: {:?}",
        aggregated.style_noise()
    );
}

// --- the claim-evidence fixtures (P14-T004) ------------------------------
//
// The three fixtures below are not projects and fire no scanner. Each is a
// recording: a claim an AI made, and the harness events behind it, declared in
// the fixture's `scenario.json`. They are read here the way a harness's events
// reach SURE — as JSON documents through `ingest_event_str`, stored through the
// session event store into a scratch store under `target/tmp` — and what is
// asserted is what SURE says about them.
//
// Each fixture is read twice, and each has a control. The control is the thing
// this half cannot do without: a claim checker that answered `cannot_confirm` to
// everything would satisfy all three required outcomes at once, so every one of
// them is paired with the same recording with one field of one event moved,
// which must come back `confirmed`.

/// One event, as a fixture declares it.
#[derive(Debug, Clone)]
struct DeclaredEvent {
    event_type: String,
    timestamp: String,
    payload: Value,
}

/// One evidence item, in the terms a reader of the fixture cares about.
#[derive(Debug, PartialEq)]
struct Attached {
    class: EvidenceClass,
    subject: AnchorSubject,
    location: String,
    locator: String,
    excerpt: String,
    belongs_to_this_state: bool,
    severity: Severity,
    freshness: Freshness,
}

/// What SURE answered about one recording, reduced to the part that is the
/// recording's rather than the store's.
///
/// Row ids and evidence ids are generated per call, so two readings of the same
/// recording differ in those and in nothing else. This is the reduction that
/// makes "the two paths agree" a checkable sentence instead of a comparison
/// between two values that can never be equal.
#[derive(Debug, PartialEq)]
struct Answer {
    assessment: ClaimAssessment,
    reason: String,
    evidence: Vec<Attached>,
}

/// Everything one reading of a recording produced.
struct Recorded {
    checked: CheckedClaim,
    answer: Answer,
    rendered: String,
    ingested: Vec<IngestedEvent>,
}

/// The scenario document of one fixture.
fn scenario_of(id: &str) -> Value {
    let path = fixture(id).join("scenario.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not valid JSON: {error}", path.display()))
}

/// The `claim_check` block of one fixture's scenario.
fn claim_block(id: &str) -> Value {
    scenario_of(id)
        .get("claim_check")
        .cloned()
        .unwrap_or_else(|| {
            panic!("fixtures/adversarial/{id}/scenario.json declares no claim_check block")
        })
}

/// The events one block declares.
fn declared_events(id: &str, block: &Value, key: &str) -> Vec<DeclaredEvent> {
    let events = block
        .get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{id}: the `{key}` list is missing or is not an array"));
    assert!(!events.is_empty(), "{id}: `{key}` declares no events");
    events
        .iter()
        .map(|event| {
            assert!(
                event.is_object(),
                "{id}: an entry in `{key}` is not an event object"
            );
            let string = |name: &str| {
                let value = event
                    .get(name)
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| panic!("{id}: an event in `{key}` declares no `{name}`"));
                assert!(
                    !value.is_empty(),
                    "{id}: an event in `{key}` declares an empty `{name}`"
                );
                value.to_owned()
            };
            let payload = event
                .get("payload")
                .cloned()
                .unwrap_or_else(|| panic!("{id}: an event in `{key}` declares no payload"));
            // The envelope's own rule, and the reason this is asserted rather
            // than assumed: an event with no body has an empty body, and `null`
            // would make "the adapter sent nothing" and "the adapter sent an
            // empty body" the same document.
            assert!(
                payload.is_object(),
                "{id}: the payload of an event in `{key}` is not an object"
            );
            DeclaredEvent {
                event_type: string("event_type"),
                timestamp: string("timestamp"),
                payload,
            }
        })
        .collect()
}

/// The JSON document a harness would send for one declared event.
///
/// Built from the declared fields and nothing else, so the fixture is graded
/// against the wire document a real adapter produces rather than against an
/// `EventEnvelope` assembled in Rust — which would prove nothing about the JSON
/// the fixture ships.
fn event_document(event: &DeclaredEvent) -> String {
    json!({
        "schema_version": sure_core::PROTOCOL_VERSION,
        "source": "claude-code",
        "event_type": event.event_type,
        "timestamp": event.timestamp,
        "capability_tier": sure_core::capability::CapabilityTier::Observed.number(),
        "session_id": "session-1",
        "project_root": CLAIM_PROJECT_ROOT,
        "payload": event.payload,
    })
    .to_string()
}

/// The JSON document a harness would send for one declared claim.
fn agent_claim_document(claim: &Value) -> String {
    let string = |name: &str| {
        claim
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("the declared claim has no `{name}`"))
            .to_owned()
    };
    json!({
        "schema_version": sure_core::PROTOCOL_VERSION,
        "source": "claude-code",
        "event_type": sure_core::claim_capture::AGENT_CLAIM_EVENT_TYPE,
        "timestamp": string("timestamp"),
        "capability_tier": sure_core::capability::CapabilityTier::Observed.number(),
        "session_id": "session-1",
        "project_root": CLAIM_PROJECT_ROOT,
        "payload": {
            "id": string("id"),
            "claim_text": string("claim_text"),
            "claim_type": string("claim_type"),
        },
    })
    .to_string()
}

/// Reduce a checked claim to the part that is the recording's.
///
/// The two things dropped are the row id and the evidence id, which are minted
/// per call. Everything a fixture declares is kept, including the call the trap
/// in `stale-test-evidence` and `unknown-evidence` is about: the domain's own
/// `freshness` over evidence SURE has just refused to use.
fn answer_of(checked: &CheckedClaim, fingerprint: &FingerprintId) -> Answer {
    Answer {
        assessment: checked.assessment,
        reason: checked.reason.clone(),
        evidence: checked
            .evidence
            .iter()
            .map(|evidence| Attached {
                class: evidence.class,
                subject: evidence.anchor.subject,
                location: evidence.anchor.location.clone(),
                locator: evidence.anchor.locator.clone(),
                excerpt: evidence.anchor.excerpt.clone(),
                belongs_to_this_state: evidence.fingerprint.as_ref() == Some(fingerprint),
                severity: evidence.severity,
                freshness: sure_core::evidence::freshness(evidence, fingerprint),
            })
            .collect(),
    }
}

/// A checked claim in the report's own type, so the rendered section can be read.
///
/// The mapping is the one `pipeline::claim_of` makes and it is made again rather
/// than shared, because a session identifier is not recoverable from a stored
/// claim row and this test has no session to attribute a claim to.
fn claim_of(checked: &CheckedClaim) -> Claim {
    Claim {
        id: ClaimId::parse(checked.id.as_str()).unwrap_or_else(|_| ClaimId::generate()),
        claim_text: checked.claim_text.clone(),
        claim_type: checked.claim_type.clone(),
        assessment: checked.assessment,
        reason: checked.reason.clone(),
        evidence: checked.evidence.clone(),
        session: None,
    }
}

/// Read one declared recording the way a harness's recording reaches SURE.
///
/// The claim and every event are ingested as the documents a harness sends, and
/// stored into a scratch store under `target/tmp` — never the machine's own
/// store. Then SURE is asked twice: once through the store, which is what
/// `sure check` reads, and once through [`check_claims_against_events`] over the
/// same events in the order that function documents. The two answers must be
/// equal, so what the fixture asserts is a property of the recording rather than
/// of either path through it.
fn read_recording(what: &str, scratch: &str, claim: &Value, events: &[DeclaredEvent]) -> Recorded {
    let directory = Scratch::under("claim evidence fixtures", scratch);
    let store = Store::open_at(&directory.project.join("sure.db"))
        .unwrap_or_else(|error| panic!("{what}: the scratch store does not open: {error}"));
    let fingerprint = FingerprintId::generate();

    let claim_event = ingest_event_str(&agent_claim_document(claim)).unwrap_or_else(|error| {
        panic!("{what}: the declared claim is not a document SURE ingests: {error}")
    });
    let captured = capture_agent_claim(&claim_event, &store, CLAIM_PROJECT_ROOT, &fingerprint)
        .unwrap_or_else(|error| panic!("{what}: SURE refused the declared claim: {error}"));
    assert!(
        matches!(captured, ClaimCaptureOutcome::Captured(_)),
        "{what}: the claim event was stored as something other than a claim"
    );

    let mut ingested: Vec<IngestedEvent> = Vec::new();
    for event in events {
        let stored = ingest_event_str(&event_document(event)).unwrap_or_else(|error| {
            panic!(
                "{what}: SURE refused the declared event {} at {}: {error}",
                event.event_type, event.timestamp
            )
        });
        SessionEventStore::new(&store)
            .persist(
                &stored,
                CLAIM_PROJECT_ROOT,
                &fingerprint,
                &EventId::generate(),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{what}: the declared event {} at {} does not store: {error}",
                    event.event_type, event.timestamp
                )
            });
        ingested.push(stored);
    }

    let through_the_store = check_claims_in_store(&store, &fingerprint)
        .unwrap_or_else(|error| panic!("{what}: the claim check failed: {error}"));
    assert_eq!(
        through_the_store.len(),
        1,
        "{what}: the recording holds one claim and SURE checked {} of them",
        through_the_store.len()
    );

    let documents = vec![ClaimDocument {
        record_id: 0,
        id: claim["id"].as_str().unwrap_or_default().to_owned(),
        claim_text: claim["claim_text"].as_str().unwrap_or_default().to_owned(),
        claim_type: claim["claim_type"].as_str().map(str::to_owned),
    }];
    let newest_first: Vec<IngestedEvent> = ingested.iter().rev().cloned().collect();
    let direct = check_claims_against_events(&documents, &newest_first, &fingerprint);

    let answer = answer_of(&through_the_store[0], &fingerprint);
    assert_eq!(
        answer,
        answer_of(&direct[0], &fingerprint),
        "{what}: the store path and the direct path disagree about the same recording"
    );

    let rendered = render_claim_section(&[claim_of(&through_the_store[0])]);
    Recorded {
        checked: through_the_store[0].clone(),
        answer,
        rendered,
        ingested,
    }
}

/// Every fact one `expect` block declares, asserted against what SURE answered.
///
/// The expectations come out of the scenario rather than being restated here, so
/// the two can only drift apart by this test failing. The values that decide
/// whether a fixture is right are asserted by equality — the assessment, the
/// sentence a reader is shown, the count, the anchor, the class and the
/// freshness — and the rendered report section is compared whole, so what a
/// person would read is graded and not only what is behind it.
fn assert_declared_outcome(
    what: &str,
    claim: &Value,
    events: &[DeclaredEvent],
    expect: &Value,
    recorded: &Recorded,
) {
    let declared_assessment: ClaimAssessment = serde_json::from_value(expect["assessment"].clone())
        .unwrap_or_else(|error| {
            panic!(
                "{what}: `{}` is not a claim assessment: {error}",
                expect["assessment"]
            )
        });
    assert_eq!(
        recorded.answer.assessment, declared_assessment,
        "{what}: the assessment changed"
    );
    let declared_reason = expect["reason"]
        .as_str()
        .unwrap_or_else(|| panic!("{what}: the expectation declares no reason"));
    assert_eq!(
        recorded.answer.reason, declared_reason,
        "{what}: the sentence a reader is shown changed"
    );

    match expect["staleness_reason"].as_str() {
        // A reason that comes from a staleness is asserted to be that reason's
        // own sentence rather than a second copy of it, so the sentence a reader
        // sees and the reason the code reached cannot drift apart unnoticed.
        Some(name) => {
            let declared: StalenessReason = serde_json::from_value(
                expect["staleness_reason"].clone(),
            )
            .unwrap_or_else(|error| panic!("{what}: `{name}` is not a staleness reason: {error}"));
            assert_eq!(
                recorded.answer.reason,
                declared.plain_explanation(),
                "{what}: the reason is not `{name}`'s own sentence"
            );
        }
        None => assert!(
            expect["staleness_reason"].is_null(),
            "{what}: staleness_reason is neither a name nor null"
        ),
    }

    let declared_count = expect["evidence_count"]
        .as_u64()
        .unwrap_or_else(|| panic!("{what}: the expectation declares no evidence_count"));
    assert_eq!(
        recorded.answer.evidence.len() as u64,
        declared_count,
        "{what}: SURE attached {} evidence items and the fixture declares {declared_count}",
        recorded.answer.evidence.len()
    );

    let declared_class: EvidenceClass = serde_json::from_value(expect["evidence_class"].clone())
        .unwrap_or_else(|error| {
            panic!(
                "{what}: `{}` is not an evidence class: {error}",
                expect["evidence_class"]
            )
        });

    if declared_count == 0 {
        // Nothing was attached, and the class says exactly that: the
        // vocabulary's weakest class is the one for "not enough evidence to say
        // anything", and a stronger one would be a class attached to nothing.
        assert!(
            recorded.answer.evidence.is_empty(),
            "{what}: the count is zero and there is evidence"
        );
        assert_eq!(
            declared_class,
            EvidenceClass::Unknown,
            "{what}: an outcome with no evidence at all is recorded with the class that means exactly that"
        );
        assert!(
            expect["anchored_to"].is_null(),
            "{what}: the fixture declares an anchor for an outcome that has no evidence to anchor"
        );
        assert!(
            expect["evidence_freshness"].is_null(),
            "{what}: the fixture declares a freshness for evidence that does not exist"
        );
        assert_rendered_section(what, claim, declared_assessment, declared_reason, recorded);
        return;
    }

    for item in &recorded.answer.evidence {
        assert_eq!(
            item.class, declared_class,
            "{what}: the evidence class changed"
        );
        assert!(
            item.belongs_to_this_state,
            "{what}: the evidence is not bound to the project state it was checked against"
        );
        assert_eq!(
            item.severity,
            Severity::Note,
            "{what}: the weight carried by claim evidence changed"
        );
        assert!(
            item.excerpt.is_empty(),
            "{what}: SURE recorded the event's name and time, so there is nothing to quote"
        );
    }

    let anchor = &expect["anchored_to"];
    let index = anchor["event_index"]
        .as_u64()
        .unwrap_or_else(|| panic!("{what}: the declared anchor points at no event"))
        as usize;
    let event = events.get(index).unwrap_or_else(|| {
        panic!(
            "{what}: the declared anchor points at event {index} and the recording declares {}",
            events.len()
        )
    });
    // The anchor must be the event the fixture declares, named in the
    // expectation rather than assumed: the scenario's own two copies of the
    // event's name and time have to agree before SURE's answer is compared with
    // either of them.
    assert_eq!(
        anchor["location"].as_str(),
        Some(event.event_type.as_str()),
        "{what}: the declared anchor names an event the recording does not declare"
    );
    assert_eq!(
        anchor["locator"].as_str(),
        Some(event.timestamp.as_str()),
        "{what}: the declared anchor times an event the recording does not time that way"
    );

    let item = &recorded.answer.evidence[0];
    assert_eq!(
        item.subject,
        AnchorSubject::Event,
        "{what}: the anchor is not the recorded event"
    );
    assert_eq!(
        item.location, event.event_type,
        "{what}: the anchor does not point at the event SURE attached"
    );
    assert_eq!(
        item.locator, event.timestamp,
        "{what}: the anchor does not carry the event's own timestamp"
    );

    let declared_freshness: Freshness =
        serde_json::from_value(expect["evidence_freshness"].clone()).unwrap_or_else(|error| {
            panic!(
                "{what}: `{}` is not a freshness: {error}",
                expect["evidence_freshness"]
            )
        });
    assert_eq!(
        item.freshness, declared_freshness,
        "{what}: the freshness of the attached evidence changed"
    );

    assert_rendered_section(what, claim, declared_assessment, declared_reason, recorded);
}

/// The section a person reads, built from the fixture's own declarations.
///
/// This is what makes the acceptance sentence's word "shown" a graded fact: the
/// header, the label, the claim text, the indent and the reason are assembled
/// from the scenario, and the product's rendered section has to be that string
/// exactly.
fn assert_rendered_section(
    what: &str,
    claim: &Value,
    assessment: ClaimAssessment,
    reason: &str,
    recorded: &Recorded,
) {
    let claim_text = claim["claim_text"]
        .as_str()
        .unwrap_or_else(|| panic!("{what}: the declared claim has no text"));
    let expected = format!(
        "AI claims\n---------\n- {claim_text}\n  {}: {reason}\n",
        assessment.label()
    );
    assert_eq!(
        recorded.rendered, expected,
        "{what}: the claim section a person reads changed"
    );
}

/// The control is the fixture's own recording with one field of one event moved,
/// and no field it could hide a second difference in.
fn assert_the_control_is_one_thing_moved(id: &str, block: &Value, control: &Value) {
    let fixture_events = declared_events(id, block, "events");
    let control_events = declared_events(id, control, "events");
    assert_eq!(
        fixture_events.len(),
        control_events.len(),
        "{id}: the control is not the fixture with one thing moved: it holds {} events against the fixture's {}",
        control_events.len(),
        fixture_events.len()
    );

    let moved = &control["moved"];
    let index = moved["event_index"]
        .as_u64()
        .unwrap_or_else(|| panic!("{id}: the control does not say which event it moved"))
        as usize;
    let field = moved["field"]
        .as_str()
        .unwrap_or_else(|| panic!("{id}: the control does not say which field it moved"));
    assert!(
        fixture_events.get(index).is_some(),
        "{id}: the control says it moved event {index} and the recording declares {}",
        fixture_events.len()
    );

    let mut keys: Vec<String> = control["events"]
        .as_array()
        .expect("the control declares events")
        .iter()
        .flat_map(|event| event.as_object().expect("an event object").keys().cloned())
        .collect();
    keys.sort();
    keys.dedup();
    assert_eq!(
        keys,
        [
            "event_type".to_owned(),
            "payload".to_owned(),
            "timestamp".to_owned()
        ],
        "{id}: a control event carries a field this check does not compare, so a difference could hide in it"
    );

    let read = |event: &DeclaredEvent, field: &str| match field {
        "event_type" => event.event_type.clone(),
        "timestamp" => event.timestamp.clone(),
        other => panic!("{id}: the control moved `{other}`, which this check cannot compare"),
    };
    assert_eq!(
        Some(read(&fixture_events[index], field).as_str()),
        moved["from"].as_str(),
        "{id}: the control says it moved `{field}` from `{}` and the fixture says `{}`",
        moved["from"],
        read(&fixture_events[index], field)
    );
    assert_eq!(
        Some(read(&control_events[index], field).as_str()),
        moved["to"].as_str(),
        "{id}: the control says it moved `{field}` to `{}` and the control says `{}`",
        moved["to"],
        read(&control_events[index], field)
    );

    for (position, (before, after)) in fixture_events.iter().zip(&control_events).enumerate() {
        for name in ["event_type", "timestamp"] {
            if position == index && name == field {
                continue;
            }
            assert_eq!(
                read(before, name),
                read(after, name),
                "{id}: the control moved something other than `{field}` of event {index}: \
                 `{name}` of event {position} differs"
            );
        }
        assert_eq!(
            before.payload, after.payload,
            "{id}: the control moved something other than `{field}` of event {index}: \
             the payload of event {position} differs"
        );
    }

    assert_ne!(
        control["expect"]["assessment"].as_str(),
        Some("cannot_confirm"),
        "{id}: the control reaches cannot_confirm too, so a checker that answered cannot_confirm \
         to everything would pass this fixture and the control would control for nothing"
    );
}

/// What the corpus writes down about this fixture and what its declared stream
/// says must agree, so the entry `evaluation/`-style readers see is not a second
/// copy that has drifted.
fn assert_the_required_outcomes_grade_the_declared_stream(id: &str, block: &Value) {
    let outcomes = scenario_of(id)["required_outcomes"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("{id}/scenario.json requires no outcomes at all"));

    let claim_outcomes: Vec<&Value> = outcomes
        .iter()
        .filter(|outcome| outcome["kind"] == "claim_check")
        .collect();
    assert_eq!(
        claim_outcomes.len(),
        1,
        "{id}/scenario.json must declare exactly one `claim_check` outcome and declares {}. It is \
         the entry that says in the corpus what this fixture is graded on; with none, the outcome \
         these tests assert is written down nowhere but in this file, and the two could drift apart.",
        claim_outcomes.len()
    );
    let outcome = claim_outcomes[0];
    let expect = &block["expect"];
    assert_eq!(
        outcome["assessment"], expect["assessment"],
        "{id}: the required outcome and the declared stream disagree about the assessment"
    );
    assert_eq!(
        outcome["evidence_count"], expect["evidence_count"],
        "{id}: the required outcome and the declared stream disagree about the evidence count"
    );
    assert_eq!(
        outcome["evidence_class"], expect["evidence_class"],
        "{id}: the required outcome and the declared stream disagree about the evidence class"
    );
    assert_eq!(
        outcome["module"], block["read_by"]["module"],
        "{id}: the required outcome names one module and the declared stream names another"
    );
    assert_eq!(
        outcome["reading_module"], block["read_by"]["plain_report_module"],
        "{id}: the required outcome and the declared stream disagree about where the answer is rendered"
    );

    let controls: Vec<&Value> = outcomes
        .iter()
        .filter(|outcome| outcome["kind"] == "control")
        .collect();
    assert_eq!(
        controls.len(),
        1,
        "{id}: the corpus records {} control entries for a fixture with exactly one control",
        controls.len()
    );
    assert!(
        !controls[0]["why"].as_str().unwrap_or_default().is_empty(),
        "{id}: the control is recorded with no reason given"
    );
    assert!(
        block.get("control").is_some(),
        "{id}: the fixture declares no control, and a fixture whose answer is cannot_confirm is \
         worth nothing without one"
    );
}

#[test]
fn every_claim_fixture_these_assertions_name_is_a_fixture_this_repository_ships() {
    // The same guard the lists above have, for the same reason: a renamed
    // directory would make `scenario_of` panic above and every assertion would
    // be about a recording nobody ships. The second half is what a recording
    // must not grow: these fixtures have no project, so a language manifest in
    // one of them would put it in a runnability sweep that runs nothing.
    for id in CLAIM_FIXTURES {
        let dir = fixture(id);
        assert!(dir.is_dir(), "{} is not a directory", dir.display());
        assert!(
            dir.join("scenario.json").is_file(),
            "{id} has no scenario.json, so there is no recording to read"
        );
        assert!(
            dir.join("README.md").is_file(),
            "{id} has no README.md, so nothing says in words what it traps"
        );
        for manifest in ["package.json", "pyproject.toml", "requirements.txt"] {
            assert!(
                !dir.join(manifest).is_file(),
                "{id} ships a {manifest}: these fixtures are recordings, and a runnability \
                 manifest would claim a project they do not have"
            );
        }
        let block = claim_block(id);
        assert!(
            block.get("events").and_then(Value::as_array).is_some(),
            "{id}/scenario.json declares no events to read"
        );
        assert!(
            block.get("control").is_some(),
            "{id}/scenario.json declares no control"
        );
    }
    for id in PYTHON_FIXTURES.iter().chain(TYPESCRIPT_FIXTURES) {
        assert!(
            !CLAIM_FIXTURES.contains(id),
            "{id} is in both lists, so one of the two halves is misdescribed"
        );
    }
}

#[test]
fn tests_not_run_claim_is_cannot_confirm_with_no_evidence_and_the_control_flips_it() {
    let id = "tests-not-run";
    let block = claim_block(id);
    assert_the_required_outcomes_grade_the_declared_stream(id, &block);
    let claim = &block["claim"];

    let events = declared_events(id, &block, "events");
    let what = format!("{id}: the fixture's own recording");
    let recorded = read_recording(&what, "tests-not-run fixture", claim, &events);
    assert_declared_outcome(&what, claim, &events, &block["expect"], &recorded);

    // The claim the scenario declares is the claim SURE checked, text and type,
    // so nothing about the recording is assumed.
    assert_eq!(
        recorded.checked.claim_text,
        claim["claim_text"].as_str().unwrap(),
        "{id}: the claim text did not survive capture"
    );
    assert_eq!(
        recorded.checked.claim_type, "test_ran",
        "{id}: the claim was checked as a different kind of claim"
    );
    assert_eq!(
        recorded.checked.assessment,
        ClaimAssessment::CannotConfirm,
        "{id}: the claim the AI made about its own tests is not reported as unconfirmed"
    );

    // The trap this fixture is built on, asserted and not described: the event
    // SURE refuses projects to a test run with a passing summary, so what
    // refuses it is the claim checker's family rule and not a failure to
    // understand the event.
    assert_eq!(
        recorded.ingested[1].envelope.event_type, "mytest.finished",
        "{id}: the recording's second event is not the one this check is about"
    );
    match project(&recorded.ingested[1]) {
        Some(StandardProjection::BuildTest {
            kind,
            target,
            success,
            ..
        }) => {
            assert_eq!(
                kind,
                BuildTestKind::Test,
                "{id}: the refused event no longer projects to a test run"
            );
            assert_eq!(target, "unit tests", "{id}: the target changed");
            assert!(success, "{id}: the summary changed");
        }
        other => panic!(
            "{id}: the event this fixture refuses projects to {other:?}, so the fixture no longer \
             measures the family rule it exists for"
        ),
    }

    // The control: the same recording with the event's own name moved.
    let control = &block["control"];
    assert_the_control_is_one_thing_moved(id, &block, control);
    let control_events = declared_events(id, control, "events");
    let control_what = format!("{id}: the control");
    let control_recorded = read_recording(
        &control_what,
        "tests-not-run control",
        claim,
        &control_events,
    );
    assert_declared_outcome(
        &control_what,
        claim,
        &control_events,
        &control["expect"],
        &control_recorded,
    );
}

#[test]
fn stale_test_evidence_claim_is_cannot_confirm_and_points_at_the_run_it_cannot_use() {
    let id = "stale-test-evidence";
    let block = claim_block(id);
    assert_the_required_outcomes_grade_the_declared_stream(id, &block);
    let claim = &block["claim"];

    let events = declared_events(id, &block, "events");
    let what = format!("{id}: the fixture's own recording");
    let recorded = read_recording(&what, "stale-test-evidence fixture", claim, &events);
    assert_declared_outcome(&what, claim, &events, &block["expect"], &recorded);

    // The trap the fixture records as a value, asserted here as a pair: the
    // evidence SURE refuses to use is `Fresh` by the domain's own call, because
    // `check_claim` stamps the current fingerprint on what it attaches. The
    // staleness that decides the assessment is in the reason and nowhere in that
    // call, so a reader who looked for stale evidence that way would find none.
    assert_eq!(
        recorded.answer.assessment,
        ClaimAssessment::CannotConfirm,
        "{id}: the claim is not reported as unconfirmed"
    );
    assert_eq!(
        recorded.answer.evidence[0].freshness,
        Freshness::Fresh,
        "{id}: this is the trap the fixture documents, and it has moved: `freshness` over the \
         attached evidence must answer Fresh while the assessment is cannot_confirm"
    );
    assert_eq!(
        recorded.answer.evidence[0].locator, "2026-09-14T10:22:41.000Z",
        "{id}: the evidence no longer points at the run that came before the write"
    );
    assert!(
        recorded.answer.reason.contains("later changes"),
        "{id}: the reason no longer says what went wrong: {}",
        recorded.answer.reason
    );

    // The control: the same recording with the write's timestamp moved to before
    // the run. Same evidence, same anchor, opposite answer.
    let control = &block["control"];
    assert_the_control_is_one_thing_moved(id, &block, control);
    let control_events = declared_events(id, control, "events");
    let control_what = format!("{id}: the control");
    let control_recorded = read_recording(
        &control_what,
        "stale-test-evidence control",
        claim,
        &control_events,
    );
    assert_declared_outcome(
        &control_what,
        claim,
        &control_events,
        &control["expect"],
        &control_recorded,
    );
    assert_eq!(
        control_recorded.answer.evidence[0].locator, recorded.answer.evidence[0].locator,
        "{id}: the control moved the evidence itself, so what it measures is no longer the verdict"
    );
}

#[test]
fn unknown_evidence_claim_is_cannot_confirm_and_points_at_the_run_it_cannot_date() {
    let id = "unknown-evidence";
    let block = claim_block(id);
    assert_the_required_outcomes_grade_the_declared_stream(id, &block);
    let claim = &block["claim"];

    let events = declared_events(id, &block, "events");
    let what = format!("{id}: the fixture's own recording");
    let recorded = read_recording(&what, "unknown-evidence fixture", claim, &events);
    assert_declared_outcome(&what, claim, &events, &block["expect"], &recorded);

    // The two parsers disagreeing, which is what this fixture is built on and
    // not a claim it describes: the wire accepts the leap second and the
    // timestamp parser refuses it, so the event is stored whole and can never be
    // placed.
    assert_eq!(
        recorded.ingested[1].envelope.event_type, "test.finished",
        "{id}: the recording's second event is not the test run"
    );
    assert_eq!(
        recorded.ingested[1].envelope.timestamp, "2026-06-30T23:59:60Z",
        "{id}: the unreadable timestamp did not survive ingestion"
    );
    assert_eq!(
        sure_core::diagnostics::Timestamp::parse_rfc3339("2026-06-30T23:59:60Z"),
        None,
        "{id}: SURE can read the leap second now, so this fixture no longer has a state to measure"
    );
    assert_eq!(
        recorded.answer.assessment,
        ClaimAssessment::CannotConfirm,
        "{id}: the claim is not reported as unconfirmed"
    );
    assert_eq!(
        recorded.answer.evidence.len(),
        1,
        "{id}: SURE stopped pointing at the event it found, which is the difference between \
         `found nothing` and `found this and cannot date it`"
    );
    assert_eq!(
        recorded.answer.evidence[0].locator, "2026-06-30T23:59:60Z",
        "{id}: the evidence no longer carries the timestamp SURE could not read"
    );
    assert_eq!(
        recorded.answer.evidence[0].freshness,
        Freshness::Fresh,
        "{id}: the evidence SURE refused to date is Fresh by the domain's own call, and that pair \
         is the fixture"
    );

    // The control: the same recording with the leap second moved to the second
    // before it, which SURE can read.
    let control = &block["control"];
    assert_the_control_is_one_thing_moved(id, &block, control);
    let control_events = declared_events(id, control, "events");
    let control_what = format!("{id}: the control");
    let control_recorded = read_recording(
        &control_what,
        "unknown-evidence control",
        claim,
        &control_events,
    );
    assert_declared_outcome(
        &control_what,
        claim,
        &control_events,
        &control["expect"],
        &control_recorded,
    );
}

#[test]
fn a_write_whose_own_timestamp_cannot_be_read_does_not_supersede_the_run() {
    // The other side of `unknown-evidence`, and the reason the two are named
    // together in their notes. There the proof event's own timestamp cannot be
    // read, and the run is refused; here a *later* code change's timestamp
    // cannot be read, and the run stands, because `evidence_freshness` skips an
    // event it cannot place rather than treating it as superseding. An
    // unreadable write cannot invalidate a readable run — which is a boundary of
    // the refusal `stale-test-evidence` is about, and one no fixture declares.
    let id = "unknown-evidence";
    let claim = &claim_block(id)["claim"];
    let events = vec![
        DeclaredEvent {
            event_type: "test.finished".to_owned(),
            timestamp: "2026-06-30T23:59:58Z".to_owned(),
            payload: json!({"target": "unit tests", "success": true}),
        },
        DeclaredEvent {
            event_type: "file.write".to_owned(),
            timestamp: "2026-06-30T23:59:60Z".to_owned(),
            payload: json!({"path": "src/orders.js", "written": true}),
        },
    ];

    let what = "the boundary: a readable run with an unreadable write after it";
    let recorded = read_recording(what, "unreadable write boundary", claim, &events);
    assert_eq!(
        recorded.answer.assessment,
        ClaimAssessment::Confirmed,
        "{what}: a write SURE cannot place is being treated as a later change. The fixture's notes \
         say it is skipped; if that has changed, they are wrong and so is this test."
    );
    assert_eq!(recorded.answer.evidence.len(), 1);
    assert_eq!(recorded.answer.evidence[0].locator, "2026-06-30T23:59:58Z");
    assert_eq!(recorded.answer.evidence[0].freshness, Freshness::Fresh);
}

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
//! - `missing-migration` fires [`sure_core::db_migrations`], which is the only
//!   check in this file that reaches `Severity::MustFix` and the only one whose
//!   evidence is `EvidenceClass::ObservedFact` rather than `Inference`. This is
//!   the one P14 fixture where what SURE detects already carries the weight the
//!   release manifest asks for, and it is asserted rather than assumed.
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
//! # What is not claimed
//!
//! **Severity, for the six TypeScript fixtures.** Every assertion about those is
//! about what is *reported*, not at what weight. Their scanners emit
//! `Severity::Note`, `critical = false`, `EvidenceClass::Inference`, which
//! [`sure_core::false_completion_aggregator::aggregate`] classifies as style
//! noise rather than material. `evaluation/acceptance-manifest.json` asks for
//! `must_fix` on five of those six ids, so the gap between what is detected and
//! what the manifest requires is real, is recorded in each `scenario.json`, and
//! is not asserted away here.
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

use sure_core::candidate_scanner::CandidateScanner;
use sure_core::db_migrations::{MigrationsReport, Record};
use sure_core::demo_data_heuristics::DemoDataHeuristics;
use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::env_completeness::CompletenessReport;
use sure_core::evidence::{AnchorSubject, ClaimAssessment, EvidenceClass};
use sure_core::external_service::ExternalServiceChecks;
use sure_core::false_completion_aggregator::aggregate;
use sure_core::ids::FingerprintId;
use sure_core::noop_heuristics::NoOpHeuristics;
use sure_core::references::KeyStatus;
use sure_core::route_consistency::RouteConsistency;
use sure_core::severity::Severity;
use sure_core::status::{CheckStatus, NotCheckedReason};
use sure_core::ui_action_bridge::UiActionBridge;

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
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("adversarial python controls");
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
        "this is the one P14 fixture whose detector reaches the manifest's own severity on its own"
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

#[test]
fn what_is_detected_is_not_yet_what_the_manifest_requires() {
    // The gap, pinned so it cannot be forgotten or mistaken for done. Five of
    // these six ids are release-blocking `must_fix` cases in
    // `evaluation/acceptance-manifest.json`, and everything the scanners above
    // produce is `Note`, not critical, `Inference` — the combination
    // `false_completion_aggregator` files as style noise. The day a detector
    // earns a stronger weight this assertion fails and the recording in each
    // `scenario.json` is updated with it.
    let discovery = discovery("fake-payment");
    let proposals = NoOpHeuristics::of(&discovery).proposed().to_vec();
    assert!(
        !proposals.is_empty(),
        "the probe fixture produced nothing, so this test proves nothing"
    );
    for proposal in &proposals {
        assert_eq!(proposal.severity(), Severity::Note);
        assert!(!proposal.critical());
        assert_eq!(proposal.evidence_class(), EvidenceClass::Inference);
    }

    // And that combination is exactly what the aggregator sets aside: the
    // finding reaches a reader as style noise, not as something to act on. That
    // is the whole of the gap — detected, and not weighed as the manifest
    // weighs it — so it is asserted rather than left to the sentence above.
    let aggregated = aggregate(proposals);
    assert!(
        aggregated.material().is_empty(),
        "something here is now material: {:?}",
        aggregated.material()
    );
    assert!(
        !aggregated.style_noise().is_empty(),
        "the probe fixture's findings reached neither list, so this test proves nothing"
    );
}

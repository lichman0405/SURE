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
//! # What is not claimed
//!
//! **Severity.** Every assertion here is about what is *reported*, not at what
//! weight. Every scanner in this file emits `Severity::Note`, `critical =
//! false`, `EvidenceClass::Inference`, which
//! [`sure_core::false_completion_aggregator::aggregate`] classifies as style
//! noise rather than material. `evaluation/acceptance-manifest.json` asks for
//! `must_fix` on five of these six ids, so the gap between what is detected and
//! what the manifest requires is real, is recorded in each `scenario.json`, and
//! is not asserted away here.
//!
//! **A password check that is always true.** SURE has no detector that reads
//! `verifyPassword` returning `true` as an authentication bypass.
//! `fake-auth` fires on what surrounds the bypass, and
//! `fake_auth_is_detected_by_what_surrounds_the_bypass` says which parts, so a
//! reader does not take the passing test for a capability SURE does not have.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use sure_core::candidate_scanner::CandidateScanner;
use sure_core::demo_data_heuristics::DemoDataHeuristics;
use sure_core::discover::{DiscoverOptions, discover};
use sure_core::evidence::EvidenceClass;
use sure_core::noop_heuristics::NoOpHeuristics;
use sure_core::route_consistency::RouteConsistency;
use sure_core::severity::Severity;
use sure_core::ui_action_bridge::UiActionBridge;

/// Every fixture this task is about, and the language half it belongs to.
const TYPESCRIPT_FIXTURES: &[&str] = &[
    "fake-payment",
    "fake-auth",
    "fake-email",
    "dead-button",
    "demo-analytics",
    "route-mismatch",
];

fn fixture(id: &str) -> PathBuf {
    sure_testkit::repository_root()
        .join("fixtures")
        .join("adversarial")
        .join(id)
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
}

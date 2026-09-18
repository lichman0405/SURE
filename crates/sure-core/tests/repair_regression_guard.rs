//! Repair-regression guard.
//!
//! A repair that fixes the original issue but breaks something else must not be
//! accepted. SURE enforces this by expanding the repair contract's re-check list
//! with affected and regression checks from the schedule, and by refusing to
//! resolve a previous finding unless every selected re-check passes.
//!
//! This test is the mandatory `repair-regression` adversarial fixture: the
//! project can never turn green while the regression check is failing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_core::recheck_lifecycle::{LifecycleInputs, reconcile};
use sure_core::repair_impact::select_impacted_checks;
use sure_core::schedule::{CheckProposal, CheckReason, CheckSchedule, PlanBuilder};
use sure_domain::evidence::{AnchorSubject, Evidence, EvidenceAnchor, EvidenceClass};
use sure_domain::execution::{ActionKind, ExecutionMode, ExecutionPermissions};
use sure_domain::finding::{AssessmentSource, FindingBuilder, FindingStatus, SeverityRationale};
use sure_domain::ids::{CheckId, FingerprintId, RepairId};
use sure_domain::severity::Severity;
use sure_domain::status::CheckResult;
use sure_domain::vocabulary::RepairContract;

fn fingerprint() -> FingerprintId {
    FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddee").unwrap()
}

fn next_fingerprint() -> FingerprintId {
    FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddef").unwrap()
}

fn email_send_evidence() -> Evidence {
    Evidence::new(
        EvidenceClass::DeterministicCheck,
        "the send path returns before the provider is called",
        EvidenceAnchor::new(AnchorSubject::File, "src/email/send.rs", "line 42"),
        Some(fingerprint()),
        Severity::MustFix,
    )
}

fn previous_email_finding() -> sure_domain::finding::Finding {
    FindingBuilder::new(
        AssessmentSource::DeterministicCheck,
        SeverityRationale::BlocksHandOff,
    )
    .id(sure_domain::ids::FindingId::generate())
    .title("Email not sent")
    .severity(Severity::MustFix)
    .status(FindingStatus::Open)
    .explanation("the send path returns before the provider is called")
    .user_impact("users believe a message was delivered")
    .next_step("call the provider")
    .fingerprint(fingerprint())
    .evidence(vec![email_send_evidence()])
    .build()
    .expect("fixture finding is valid")
}

fn repair_contract(recheck: Vec<CheckId>) -> RepairContract {
    RepairContract {
        id: RepairId::generate(),
        issue_id: sure_domain::ids::FindingId::generate(),
        problem: "Email is reported as sent but nothing is sent".to_owned(),
        why_it_matters: "Users believe a message was delivered when it was not.".to_owned(),
        required_fix: vec!["Call the configured provider on the real send path.".to_owned()],
        preserve: vec!["The current successful UI flow.".to_owned()],
        acceptance: vec!["The provider is called on a successful send.".to_owned()],
        recheck,
        forbidden_shortcuts: Vec::new(),
        evidence: vec![email_send_evidence()],
    }
}

fn schedule_with(recheck: CheckId, regression: CheckId, affected: CheckId) -> CheckSchedule {
    let mut builder = PlanBuilder::new(
        ExecutionMode::HostConfirmed,
        ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        },
    );

    // The original re-check for the finding: did the provider get called?
    builder
        .propose(CheckProposal::new(
            recheck,
            "provider is called on send",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            CheckReason::FilePresent {
                path: "src/email/send.rs".to_owned(),
            },
            &[ActionKind::RunTests],
        ))
        .unwrap();

    // A check that overlaps the repair location: the send path itself.
    builder
        .propose(CheckProposal::new(
            affected,
            "send path behaves correctly",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            CheckReason::FilePresent {
                path: "src/email/send.rs".to_owned(),
            },
            &[ActionKind::RunTests],
        ))
        .unwrap();

    // A serious regression check: it runs project code but is about a different
    // module. A repair that only fixes the email path could break payment
    // processing if the shared HTTP client is changed carelessly.
    builder
        .propose(CheckProposal::new(
            regression,
            "payment provider is called on charge",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            CheckReason::FilePresent {
                path: "src/payment/charge.rs".to_owned(),
            },
            &[ActionKind::RunTests],
        ))
        .unwrap();

    builder.build()
}

#[test]
fn a_repair_with_failing_regression_check_cannot_turn_green() {
    let previous = previous_email_finding();
    let recheck = CheckId::generate();
    let affected = CheckId::generate();
    let regression = CheckId::generate();

    let contract = repair_contract(vec![recheck.clone()]);
    let schedule = schedule_with(recheck.clone(), regression.clone(), affected.clone());

    // SURE selects the contract's recheck plus affected and regression checks.
    let selected = select_impacted_checks(&contract, &schedule);
    assert!(selected.contains(&recheck));
    assert!(selected.contains(&affected));
    assert!(selected.contains(&regression));

    // Simulate a repair where the original recheck passes, but the regression
    // check fails. This is the adversarial case: the agent "fixed" the email
    // path but broke payment processing.
    let results = vec![
        CheckResult::pass(
            recheck,
            "provider is called on send",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            next_fingerprint(),
        ),
        CheckResult::pass(
            affected,
            "send path behaves correctly",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            next_fingerprint(),
        ),
        CheckResult::fail(
            regression,
            "payment provider is called on charge",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            next_fingerprint(),
        ),
    ];

    let update = reconcile(
        LifecycleInputs {
            previous_open: std::slice::from_ref(&previous),
            current_findings: &[],
            check_results: &results,
            rechecks: &[(previous.id.clone(), selected.clone())],
        },
        next_fingerprint(),
    );

    // The original finding must stay open because not every selected check
    // passed. Resolving it would be a false green.
    assert!(
        update.findings.iter().any(|f| {
            f.status == FindingStatus::Open
                && f.title == "Email not sent"
                && f.severity == Severity::MustFix
        }),
        "a repair with a failing regression check must keep the original finding open"
    );
    assert!(
        update
            .findings
            .iter()
            .all(|f| f.status != FindingStatus::Resolved),
        "the finding must not be resolved while a regression check is failing"
    );
    assert!(update.resolved.is_empty());
    assert_eq!(update.kept_open.len(), 1);
}

#[test]
fn a_repair_with_all_selected_checks_passing_can_resolve() {
    let previous = previous_email_finding();
    let recheck = CheckId::generate();
    let affected = CheckId::generate();
    let regression = CheckId::generate();

    let contract = repair_contract(vec![recheck.clone()]);
    let schedule = schedule_with(recheck.clone(), regression.clone(), affected.clone());
    let selected = select_impacted_checks(&contract, &schedule);

    // Every selected check passes.
    let results = vec![
        CheckResult::pass(
            recheck,
            "provider is called on send",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            next_fingerprint(),
        ),
        CheckResult::pass(
            affected,
            "send path behaves correctly",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            next_fingerprint(),
        ),
        CheckResult::pass(
            regression,
            "payment provider is called on charge",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            next_fingerprint(),
        ),
    ];

    let update = reconcile(
        LifecycleInputs {
            previous_open: std::slice::from_ref(&previous),
            current_findings: &[],
            check_results: &results,
            rechecks: &[(previous.id.clone(), selected)],
        },
        next_fingerprint(),
    );

    assert!(
        update
            .findings
            .iter()
            .any(|f| f.status == FindingStatus::Resolved),
        "when every selected check passes the finding can be resolved"
    );
    assert_eq!(update.resolved.len(), 1);
    assert!(update.kept_open.is_empty());
}

#[test]
fn the_fixture_status_is_implemented() {
    // The mandatory `repair-regression` adversarial fixture is covered by the
    // tests above and marked as implemented in its scenario descriptor.
    let scenario = include_str!("../../../fixtures/adversarial/repair-regression/scenario.json");
    assert!(
        scenario.contains("implemented"),
        "the repair-regression fixture must be marked as implemented"
    );
}

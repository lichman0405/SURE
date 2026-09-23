//! Impacted-check selection for repair contracts.
//!
//! These tests live outside `src/` because `tests/check_schedule.rs` rule three
//! forbids a shipped `.rs` file under `crates/` from naming `CheckProposal`,
//! `CheckReason`, `ExecutionRequirements` or `PlanBuilder` even inside a
//! `#[cfg(test)]` module. The selection function under test does not propose
//! checks; it only consumes a schedule, but its fixtures have to build schedules,
//! and that construction belongs in an integration test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_core::planned_work::{CheckOperation, PlannedWork, PrecomputedEvidence};
use sure_core::repair_impact::select_impacted_checks;
use sure_core::schedule::{CheckProposal, CheckReason, CheckSchedule, PlanBuilder};
use sure_domain::evidence::{AnchorSubject, Evidence, EvidenceAnchor, EvidenceClass};
use sure_domain::execution::{ActionKind, ExecutionMode, ExecutionPermissions};
use sure_domain::ids::{CheckId, FingerprintId, RepairId};
use sure_domain::severity::Severity;
use sure_domain::vocabulary::RepairContract;

fn fingerprint() -> FingerprintId {
    FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddee").unwrap()
}

fn file_evidence(path: &str) -> Evidence {
    Evidence::new(
        EvidenceClass::DeterministicCheck,
        "the send path returns before the provider is called",
        EvidenceAnchor::new(AnchorSubject::File, path, "line 42"),
        Some(fingerprint()),
        Severity::MustFix,
    )
}

fn contract_with_evidence(path: &str) -> RepairContract {
    RepairContract {
        id: RepairId::generate(),
        issue_id: sure_domain::ids::FindingId::generate(),
        problem: "Email is reported as sent but nothing is sent".to_owned(),
        why_it_matters: "Users believe a message was delivered when it was not.".to_owned(),
        required_fix: vec!["Call the configured provider on the real send path.".to_owned()],
        preserve: vec!["The current successful UI flow.".to_owned()],
        acceptance: vec!["The provider is called on a successful send.".to_owned()],
        recheck: vec![CheckId::generate()],
        forbidden_shortcuts: Vec::new(),
        evidence: vec![file_evidence(path)],
    }
}

fn schedule_with(checks: &[CheckProposal]) -> CheckSchedule {
    let mut builder = PlanBuilder::new(
        ExecutionMode::HostConfirmed,
        ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        },
    );
    for proposal in checks {
        builder.propose(work(proposal.clone())).unwrap();
    }
    builder.build()
}

/// The plan entry for a proposal, since `P18-T003` a proposal plus its operation.
///
/// **A candidate observation, because that is the whole of what these fixtures
/// establish.** Every check this file builds is built by hand and nothing here
/// starts a process: the value is *nothing has settled this check*, which maps to a
/// warning and never to a pass — the one answer that claims nothing. This file is
/// about which checks a repair has to re-run, not about how any of them is carried
/// out, so what it supplies beside the proposal is the most conservative thing it
/// can honestly say; `P18-T004` replaces the placeholders on the product's own
/// paths.
fn work(proposal: CheckProposal) -> PlannedWork {
    PlannedWork::new(
        proposal,
        CheckOperation::Precomputed(PrecomputedEvidence::candidate(
            "this fixture builds a plan and observes nothing",
        )),
    )
}

fn proposal(
    id: CheckId,
    title: &str,
    severity: Severity,
    class: EvidenceClass,
    reason: CheckReason,
    actions: &[ActionKind],
) -> CheckProposal {
    CheckProposal::new(id, title, severity, true, class, reason, actions)
}

#[test]
fn recheck_ids_are_always_included() {
    let recheck = CheckId::generate();
    let contract = RepairContract {
        recheck: vec![recheck.clone()],
        ..contract_with_evidence("src/email/send.rs")
    };
    let schedule = schedule_with(&[]);

    let selected = select_impacted_checks(&contract, &schedule);
    assert_eq!(selected, vec![recheck]);
}

#[test]
fn checks_whose_reason_matches_an_evidence_location_are_selected() {
    let affected_id = CheckId::generate();
    let regression_id = CheckId::generate();
    let note_id = CheckId::generate();

    let contract = contract_with_evidence("src/email/send.rs");
    let schedule = schedule_with(&[
        proposal(
            affected_id.clone(),
            "send path returns before provider",
            Severity::MustFix,
            EvidenceClass::DeterministicCheck,
            CheckReason::FilePresent {
                path: "src/email/send.rs".to_owned(),
            },
            &[ActionKind::RunTests],
        ),
        proposal(
            regression_id.clone(),
            "payment provider is called",
            Severity::MustFix,
            EvidenceClass::DeterministicCheck,
            CheckReason::FilePresent {
                path: "src/payment/charge.rs".to_owned(),
            },
            &[ActionKind::RunTests],
        ),
        proposal(
            note_id.clone(),
            "readme mentions email",
            Severity::Note,
            EvidenceClass::ObservedFact,
            CheckReason::FilePresent {
                path: "README.md".to_owned(),
            },
            &[ActionKind::ReadFile],
        ),
    ]);

    let selected = select_impacted_checks(&contract, &schedule);
    assert!(
        selected.contains(&affected_id),
        "a check about the evidence location should be re-run"
    );
    assert!(
        selected.contains(&regression_id),
        "a serious deterministic test should run as a regression check"
    );
    assert!(
        !selected.contains(&note_id),
        "a note-level inspection check is not a relevant regression"
    );
}

#[test]
fn regression_checks_run_project_code_and_are_deterministic() {
    let test_id = CheckId::generate();
    let lint_id = CheckId::generate();

    let contract = RepairContract {
        evidence: Vec::new(),
        ..contract_with_evidence("src/email/send.rs")
    };
    let schedule = schedule_with(&[
        proposal(
            test_id.clone(),
            "declared tests pass",
            Severity::MustFix,
            EvidenceClass::DeterministicCheck,
            CheckReason::DeclaredCommand {
                declared_in: "package.json".to_owned(),
                command: "npm test".to_owned(),
            },
            &[ActionKind::RunTests],
        ),
        proposal(
            lint_id.clone(),
            "lockfile matches manifest",
            Severity::MustFix,
            EvidenceClass::ObservedFact,
            CheckReason::FilePresent {
                path: "package-lock.json".to_owned(),
            },
            &[ActionKind::ReadFile, ActionKind::ReadMetadata],
        ),
    ]);

    let selected = select_impacted_checks(&contract, &schedule);
    assert!(
        selected.contains(&test_id),
        "a deterministic test that runs project code is a regression check"
    );
    assert!(
        !selected.contains(&lint_id),
        "a static inspection check is not a regression check"
    );
}

#[test]
fn results_are_deduplicated_and_in_schedule_order() {
    let recheck = CheckId::generate();
    let first = CheckId::generate();
    let second = CheckId::generate();

    let contract = RepairContract {
        recheck: vec![recheck.clone()],
        ..contract_with_evidence("src/email/send.rs")
    };
    let schedule = schedule_with(&[
        proposal(
            first.clone(),
            "first regression test",
            Severity::MustFix,
            EvidenceClass::DeterministicCheck,
            CheckReason::ProjectWide,
            &[ActionKind::RunTests],
        ),
        proposal(
            second.clone(),
            "second affected test",
            Severity::MustFix,
            EvidenceClass::DeterministicCheck,
            CheckReason::FilePresent {
                path: "src/email/send.rs".to_owned(),
            },
            &[ActionKind::RunTests],
        ),
    ]);

    let selected = select_impacted_checks(&contract, &schedule);
    let unique: std::collections::HashSet<_> = selected.iter().cloned().collect();
    assert_eq!(unique.len(), selected.len(), "ids must not repeat");
    assert!(
        selected.contains(&first),
        "the serious regression check is selected"
    );
    assert!(selected.contains(&second), "the affected check is selected");

    let schedule_order: Vec<_> = schedule
        .checks()
        .iter()
        .map(|scheduled| scheduled.proposal().id().clone())
        .collect();
    assert_eq!(
        selected,
        std::iter::once(recheck)
            .chain(schedule_order)
            .collect::<Vec<_>>(),
        "recheck comes first, then the remaining checks in schedule order"
    );
}

#[test]
fn non_serious_tests_are_not_regression_checks() {
    let can_fix = CheckId::generate();
    let note = CheckId::generate();

    let contract = RepairContract {
        evidence: Vec::new(),
        ..contract_with_evidence("src/email/send.rs")
    };
    let schedule = schedule_with(&[
        proposal(
            can_fix.clone(),
            "can-fix test",
            Severity::CanFixLater,
            EvidenceClass::DeterministicCheck,
            CheckReason::ProjectWide,
            &[ActionKind::RunTests],
        ),
        proposal(
            note.clone(),
            "note test",
            Severity::Note,
            EvidenceClass::DeterministicCheck,
            CheckReason::ProjectWide,
            &[ActionKind::RunTests],
        ),
    ]);

    let selected = select_impacted_checks(&contract, &schedule);
    assert!(!selected.contains(&can_fix));
    assert!(!selected.contains(&note));
}

#[test]
fn line_range_overlap_counts_as_affected() {
    let line_id = CheckId::generate();
    let contract = contract_with_evidence("src/email/send.rs");
    let schedule = schedule_with(&[proposal(
        line_id.clone(),
        "line-specific check",
        Severity::MustFix,
        EvidenceClass::DeterministicCheck,
        CheckReason::CandidateFound {
            path: "src/email/send.rs".to_owned(),
            line: 42,
            context: "provider.call()".to_owned(),
        },
        &[ActionKind::RunTests],
    )]);

    let selected = select_impacted_checks(&contract, &schedule);
    assert!(selected.contains(&line_id));
}

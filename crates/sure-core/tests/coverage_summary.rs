//! Coverage and not-checked summary acceptance tests.
//!
//! These tests live outside the module so that `src/coverage_summary.rs` does not
//! name the proposal-building types that `check_schedule.rs` guards.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_core::aggregation::aggregate_run;
use sure_core::coverage_summary::summarize;
use sure_core::schedule::{CheckProposal, CheckReason, PlanBuilder};
use sure_domain::capability::{CapabilityReport, CapabilityTier};
use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::{ActionKind, ExecutionMode, ExecutionPermissions};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, CheckStatus, NotCheckedReason};

fn fingerprint() -> FingerprintId {
    FingerprintId::generate()
}

fn capability() -> CapabilityReport {
    CapabilityReport::cli()
}

fn a_proposal(
    id: CheckId,
    title: &str,
    severity: Severity,
    critical: bool,
    action: ActionKind,
) -> CheckProposal {
    CheckProposal::new(
        id,
        title,
        severity,
        critical,
        EvidenceClass::DeterministicCheck,
        CheckReason::ProjectWide,
        &[action],
    )
}

fn a_result_for(
    scheduled: &sure_core::schedule::ScheduledCheck,
    status: CheckStatus,
    fingerprint: &FingerprintId,
) -> CheckResult {
    let proposal = scheduled.proposal();
    let id = proposal.id().clone();
    let title = proposal.title();
    let severity = proposal.severity();
    let critical = proposal.critical();
    let class = EvidenceClass::DeterministicCheck;
    match status {
        CheckStatus::Pass => {
            CheckResult::pass(id, title, severity, critical, class, fingerprint.clone())
        }
        CheckStatus::Fail => {
            CheckResult::fail(id, title, severity, critical, class, fingerprint.clone())
        }
        CheckStatus::Warning => {
            CheckResult::warning(id, title, severity, critical, class, fingerprint.clone())
        }
        CheckStatus::Skipped => CheckResult::not_run(
            id,
            title,
            severity,
            critical,
            NotCheckedReason::ExecutionNotAuthorized,
            fingerprint.clone(),
        ),
        CheckStatus::Error => CheckResult::errored(
            id,
            title,
            severity,
            critical,
            "the check itself could not finish",
            fingerprint.clone(),
        ),
        CheckStatus::Unknown => {
            CheckResult::unknown(id, title, severity, critical, class, fingerprint.clone())
        }
    }
}

fn an_inspecting_builder() -> PlanBuilder {
    PlanBuilder::new(
        ExecutionMode::InspectOnly,
        ExecutionPermissions::inspect_only(),
    )
}

#[test]
fn all_checks_checked_produces_a_complete_summary() {
    let mut builder = an_inspecting_builder();
    let id_a = CheckId::generate();
    let id_b = CheckId::generate();
    builder
        .propose(a_proposal(
            id_a,
            "read the manifest",
            Severity::MustFix,
            true,
            ActionKind::ReadMetadata,
        ))
        .unwrap();
    builder
        .propose(a_proposal(
            id_b,
            "check the lockfile",
            Severity::CanFixLater,
            false,
            ActionKind::ReadFile,
        ))
        .unwrap();
    let schedule = builder.build();
    let fingerprint = fingerprint();
    let results: Vec<CheckResult> = schedule
        .checks()
        .iter()
        .map(|scheduled| a_result_for(scheduled, CheckStatus::Pass, &fingerprint))
        .collect();
    let report = aggregate_run(&schedule, &results, &fingerprint).unwrap();

    let summary = summarize(&schedule, &report, &capability());

    assert!(summary.is_complete());
    assert!(!summary.has_critical_gaps);
    assert_eq!(summary.checked_count, 2);
    assert_eq!(summary.skipped_count, 0);
    assert_eq!(summary.could_not_run_count, 0);
    assert!(summary.not_checked.is_empty());
    assert_eq!(summary.critical_not_checked_count(), 0);
    assert_eq!(summary.plain_summary(), "SURE checked all 2 checks.");
}

#[test]
fn one_skipped_and_one_could_not_run_are_counted_and_listed() {
    let mut builder = an_inspecting_builder();
    let id_a = CheckId::generate();
    let id_b = CheckId::generate();
    builder
        .propose(a_proposal(
            id_a,
            "run the tests",
            Severity::MustFix,
            true,
            ActionKind::RunTests,
        ))
        .unwrap();
    builder
        .propose(a_proposal(
            id_b,
            "check formatting",
            Severity::CanFixLater,
            false,
            ActionKind::StaticAnalysis,
        ))
        .unwrap();
    let schedule = builder.build();
    let fingerprint = fingerprint();

    let mut results: Vec<CheckResult> = Vec::new();
    for scheduled in schedule.checks() {
        if scheduled.proposal().title() == "check formatting" {
            results.push(a_result_for(scheduled, CheckStatus::Error, &fingerprint));
        }
    }
    let report = aggregate_run(&schedule, &results, &fingerprint).unwrap();

    let summary = summarize(&schedule, &report, &capability());

    assert!(!summary.is_complete());
    assert_eq!(summary.checked_count, 0);
    assert_eq!(summary.skipped_count, 1);
    assert_eq!(summary.could_not_run_count, 1);
    assert_eq!(summary.not_checked.len(), 2);

    let skipped = summary
        .not_checked
        .iter()
        .find(|entry| entry.reason.contains("running your project's code"))
        .expect("a skipped entry");
    assert!(skipped.is_critical);
    assert_eq!(skipped.check_title, "run the tests");

    let errored = summary
        .not_checked
        .iter()
        .find(|entry| entry.reason.contains("could not finish"))
        .expect("an errored entry");
    assert!(!errored.is_critical);
    assert_eq!(errored.check_title, "check formatting");
}

#[test]
fn a_critical_skipped_check_sets_has_critical_gaps() {
    let mut builder = an_inspecting_builder();
    let id = CheckId::generate();
    builder
        .propose(a_proposal(
            id,
            "run the tests",
            Severity::MustFix,
            true,
            ActionKind::RunTests,
        ))
        .unwrap();
    let schedule = builder.build();
    let fingerprint = fingerprint();
    let report = aggregate_run(&schedule, &[], &fingerprint).unwrap();

    let summary = summarize(&schedule, &report, &capability());

    assert!(summary.has_critical_gaps);
    assert_eq!(summary.critical_not_checked_count(), 1);
    assert_eq!(
        summary.plain_summary(),
        "SURE checked 0 of 1 checks. 1 check was skipped."
    );
}

#[test]
fn support_level_is_reflected() {
    let schedule = an_inspecting_builder().build();
    let fingerprint = fingerprint();
    let report = aggregate_run(&schedule, &[], &fingerprint).unwrap();
    let mut capability = capability();
    capability.tier = CapabilityTier::Protected;
    capability.pre_action_control = true;

    let summary = summarize(&schedule, &report, &capability);

    assert_eq!(
        summary.support_level,
        CapabilityTier::Protected.plain_description()
    );
}

#[test]
fn ordering_is_deterministic() {
    let mut builder = an_inspecting_builder();
    let id_a = CheckId::generate();
    let id_b = CheckId::generate();
    let id_c = CheckId::generate();
    builder
        .propose(a_proposal(
            id_a.clone(),
            "zzz non-critical",
            Severity::Note,
            false,
            ActionKind::ReadFile,
        ))
        .unwrap();
    builder
        .propose(a_proposal(
            id_b.clone(),
            "aaa non-critical",
            Severity::MustFix,
            false,
            ActionKind::ReadFile,
        ))
        .unwrap();
    builder
        .propose(a_proposal(
            id_c.clone(),
            "critical note",
            Severity::Note,
            true,
            ActionKind::ReadFile,
        ))
        .unwrap();
    let schedule = builder.build();
    let fingerprint = fingerprint();
    let report = aggregate_run(&schedule, &[], &fingerprint).unwrap();

    let summary = summarize(&schedule, &report, &capability());

    assert_eq!(summary.not_checked.len(), 3);
    assert!(summary.not_checked[0].is_critical);
    assert_eq!(summary.not_checked[0].check_title, "critical note");
    assert!(!summary.not_checked[1].is_critical);
    assert!(!summary.not_checked[2].is_critical);
    assert_eq!(summary.not_checked[1].check_title, "aaa non-critical");
    assert_eq!(summary.not_checked[2].check_title, "zzz non-critical");
}

#[test]
fn control_characters_in_check_titles_are_escaped() {
    let mut builder = an_inspecting_builder();
    let id = CheckId::generate();
    builder
        .propose(a_proposal(
            id,
            "check\nline",
            Severity::MustFix,
            true,
            ActionKind::ReadFile,
        ))
        .unwrap();
    let schedule = builder.build();
    let fingerprint = fingerprint();
    let report = aggregate_run(&schedule, &[], &fingerprint).unwrap();

    let summary = summarize(&schedule, &report, &capability());

    assert_eq!(summary.not_checked.len(), 1);
    assert!(
        !summary.not_checked[0].check_title.contains('\n'),
        "control characters must be escaped: {:?}",
        summary.not_checked[0].check_title
    );
    assert_eq!(summary.not_checked[0].check_title, "check\\nline");
}

//! Which check results become findings, and what a finding built this way is
//! allowed to claim.
//!
//! # The rule
//!
//! **A check result becomes a finding when the check did not pass and did not
//! honestly stand aside.** Four statuses, one decision each:
//!
//! | the check | is it a finding | why |
//! |---|---|---|
//! | [`CheckStatus::Pass`] | no | the only status that settles its own question |
//! | [`CheckStatus::Fail`] / [`CheckStatus::Warning`] | **yes**, [`FindingStatus::Open`] | the check ran, spoke, and was not satisfied |
//! | no result, scope limit | no | `NotApplicable`, `UnsupportedStack` and `DisabledByConfiguration` are the vocabulary's own words for *this was never SURE's question about this project* |
//! | no result, any other reason | **yes**, [`FindingStatus::CannotConfirm`] | SURE planned the check and did not run it — a gap it must report rather than round to a pass |
//!
//! # Why a check result, and not some other source
//!
//! Three other things in a run look like they could carry a finding, and each of
//! them is refused here deliberately:
//!
//! - **A candidate.** `pipeline.rs` records the decision above the field that
//!   carries them: *"a candidate is not a defect … turning a `TODO` comment into a
//!   finding with a severity would be this module inventing a judgement no phase
//!   made."* Candidates stay candidates. They are what a reader needs in order to
//!   decide, not a decision.
//! - **A claim.** [`crate::claim_checker`] never returns `Contradicted`, so no
//!   claim in this build is a defect either, and a finding built from one would be
//!   a status SURE never reached.
//! - **The plan.** A check that was proposed and denied is already recorded as a
//!   [`CheckResult`]; it reaches this module through the same door as any other.
//!
//! What is left is the one thing in a run that already carries a severity, already
//! carries an evidence class, and is already a statement about the project rather
//! than about SURE's own pattern library: a check result.
//!
//! # Severity is taken, never chosen
//!
//! The finding's severity is the check's own [`Severity`]. This module does not
//! rank, promote or demote anything — [`crate::finding_gravity`] is the one rule
//! in this tree that decides a candidate's weight, and a check result is not a
//! candidate, so this module does not touch it. A check the plan called `must_fix`
//! produces a `must_fix` finding; the alternative, picking a number here, is the
//! invention the paragraph above refuses.
//!
//! # What the evidence says, and why its class is not the result's
//!
//! [`CheckResult::not_run`] stamps [`EvidenceClass::Unknown`] on every check the
//! run did not execute, and that is right: **the check's answer** is unknown, and
//! SURE must never let a check that did not run read as *the project is fine*.
//!
//! A finding built here does not carry that answer. It carries the observation
//! underneath it — *the project declares this command, in this file, and this run
//! did not run it* — which SURE made directly, from the project's own state. So
//! the evidence class comes from what the finding claims:
//!
//! - a check that produced no result → [`EvidenceClass::ObservedFact`], because
//!   the claim is about what this run did and what the project declares;
//! - a check that produced one and did not pass → the result's own class, because
//!   the claim is about a command that ran and what it returned.
//!
//! [`CheckProposal::new`](crate::schedule::CheckProposal::new) says why this
//! distinction is available to be used at all: the class *"is what stops the
//! distinction being lost between the proposal and the finding."* This module is
//! the other end of that sentence. It is **not** a licence to upgrade a class for
//! convenience — nothing here raises a class above what the claim supports, and
//! the one case that would (`Unknown` for a check that did not answer) is exactly
//! the case this paragraph spends itself on.
//!
//! # The sentence a finding of this kind may not say
//!
//! A finding whose status is [`FindingStatus::CannotConfirm`] never says the
//! project is wrong. It says SURE does not know, names the check that would have
//! settled it, and says what would close it. `cannot_confirm` is
//! [`FindingStatus::needs_attention`], so it holds the finding open across a
//! re-check — SURE does not close a finding merely because it has stopped looking
//! at it.
//!
//! # What this module does not do
//!
//! It does not select re-checks. [`crate::repair_impact`] owns that rule, and
//! [`crate::repair_impact::seed_rechecks`] is the half of it that reads a finding.

use sure_domain::evidence::{AnchorSubject, Evidence, EvidenceAnchor, EvidenceClass};
use sure_domain::finding::{
    AssessmentSource, Finding, FindingBuilder, FindingStatus, SeverityRationale,
};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, CheckStatus};

use crate::schedule::CheckSchedule;

/// The findings a run's own check results justify, in plan order.
///
/// One pass over `results`, no reordering and no second list: the order a report
/// shows is the order the plan ran in, and a caller that wants a different order
/// has to say so itself rather than get one by accident.
#[must_use]
pub fn findings_from_checks(results: &[CheckResult], schedule: &CheckSchedule) -> Vec<Finding> {
    results
        .iter()
        .filter_map(|result| finding_for(result, schedule))
        .collect()
}

/// Whether this result is a finding at all — [`findings_from_checks`]'s rule,
/// written once so that a reader can check it without reading the builder.
///
/// Split out because it is the sentence a test can hold still: a mutation that
/// makes this answer `false` for everything empties the run's findings, and every
/// stage downstream that depends on one must go back to `not_run`.
#[must_use]
pub fn is_a_finding(result: &CheckResult) -> bool {
    if result.status == CheckStatus::Pass {
        return false;
    }
    if result.is_not_checked()
        && result
            .not_checked_reason
            .is_some_and(|reason| reason.is_scope_limit())
    {
        return false;
    }
    true
}

/// The finding a single result justifies, or `None` when it is not one.
fn finding_for(result: &CheckResult, schedule: &CheckSchedule) -> Option<Finding> {
    if !is_a_finding(result) {
        return None;
    }

    let spoke = result.status.produced_a_result();
    let (class, source) = if spoke {
        (result.evidence_class, AssessmentSource::DeterministicCheck)
    } else {
        (EvidenceClass::ObservedFact, AssessmentSource::ObservedFact)
    };
    let anchor = anchor_for(result, schedule);
    let statement = if spoke {
        format!(
            "The check \"{}\" ran and did not pass: {}",
            result.title,
            one_line(&result.reason)
        )
    } else {
        format!(
            "The check \"{}\" was planned and this run did not run it: {}",
            result.title,
            one_line(&result.reason)
        )
    };
    let evidence = Evidence::new(
        class,
        statement,
        anchor.clone(),
        Some(result.project_fingerprint.clone()),
        result.severity,
    );

    let builder = FindingBuilder::new(source, SeverityRationale::for_severity(result.severity)?)
        .title(result.title.clone())
        .severity(result.severity)
        .status(status_for(result, spoke))
        .explanation(explanation_for(result, spoke))
        .user_impact(impact_for(spoke))
        .next_step(next_step_for(&anchor, spoke))
        .evidence([evidence])
        .fingerprint(result.project_fingerprint.clone());

    builder.build().ok()
}

/// `Open` when the check spoke, `CannotConfirm` when it did not.
///
/// The two are not the same finding with a different label: `Open` says *this is
/// wrong*, `CannotConfirm` says *SURE could not settle this*. A single status
/// covering both would make the report unable to tell a failed test from a test
/// nobody ran, which is the distinction this product exists to keep.
fn status_for(result: &CheckResult, spoke: bool) -> FindingStatus {
    if spoke {
        return FindingStatus::Open;
    }
    debug_assert!(
        result.is_not_checked(),
        "a check that produced no result is not-checked by definition"
    );
    FindingStatus::CannotConfirm
}

fn explanation_for(result: &CheckResult, spoke: bool) -> String {
    if spoke {
        return format!(
            "SURE ran this check against the project and it was not satisfied. {}",
            sentence(&result.reason)
        );
    }
    format!(
        "SURE planned this check and did not run it, so it has no answer for it. {}",
        sentence(&result.reason)
    )
}

fn impact_for(spoke: bool) -> String {
    if spoke {
        return "SURE ran this check itself, so this is a result rather than an opinion. It was \
                not satisfied."
            .to_owned();
    }
    "Nothing here is known to be broken, and nothing here is confirmed to work either. Until \
     this check runs and passes, SURE cannot confirm this part of the project."
        .to_owned()
}

fn next_step_for(anchor: &EvidenceAnchor, spoke: bool) -> String {
    if spoke {
        return "Fix what this check found, then run the check again so SURE can observe the \
                result."
            .to_owned();
    }
    if anchor.subject == AnchorSubject::Command && !anchor.locator.trim().is_empty() {
        return format!(
            "Run `{}` and let SURE re-check it, or allow SURE to run it.",
            anchor.locator.trim()
        );
    }
    "Allow SURE to run this check, then re-check.".to_owned()
}

/// Where a reader goes to check this finding for themselves.
///
/// The check's own reason anchor when the schedule still holds the check — the
/// same file (or command) the check's own sentence named, so the finding's
/// evidence and the check's reason point at one place rather than at two that
/// agree today. A finding whose check has left the schedule falls back to the
/// check itself: the identity and the title are exactly what a reader can search
/// the previous report for, and an anchor invented from the check's title would
/// be a claim about a file that may not exist.
///
/// **Either way the anchor carries the check's identity**, which the reason's own
/// anchor does not. That identifier is what
/// [`crate::repair_impact::seed_rechecks`] reads to answer *which check can
/// observe whether this is fixed*, and it is the reason a finding produced here
/// can be turned into a repair contract at all: without it the seed would have to
/// be guessed from a title, and a guess is what the empty-list refusal in
/// `RepairContract::from_finding` exists to stop. Adding it does not move the
/// anchor: `location` and `locator` are the reason's own, so a reader following
/// the finding and a reader following the schedule still arrive at one line.
fn anchor_for(result: &CheckResult, schedule: &CheckSchedule) -> EvidenceAnchor {
    if let Some(anchor) = schedule
        .get(&result.id)
        .and_then(|scheduled| scheduled.proposal().reason().anchor())
    {
        return anchor.with_subject_id(result.id.clone());
    }
    EvidenceAnchor::new(
        AnchorSubject::Check,
        result.title.clone(),
        "the check itself",
    )
    .with_subject_id(result.id.clone())
}

/// The fingerprint a finding of this kind is raised against.
///
/// Public because a caller holding a finding and asking *which state is this
/// about* should not have to reach into the evidence list for it.
#[must_use]
pub fn state_of(result: &CheckResult) -> &FingerprintId {
    &result.project_fingerprint
}

/// Trim a result's reason to one line, and say nothing rather than `""`.
fn one_line(reason: &str) -> String {
    let collapsed = reason.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        "no detail was recorded".to_owned()
    } else {
        collapsed
    }
}

/// The same, with a full stop, for a sentence that continues.
fn sentence(reason: &str) -> String {
    let collapsed = one_line(reason);
    if collapsed.ends_with(['.', '!', '?']) {
        collapsed
    } else {
        format!("{collapsed}.")
    }
}

/// The severity a finding built here carries, for a caller that wants it without
/// building one.
///
/// It is the check's own, and this function exists so that a test can assert that
/// in one place instead of three.
#[must_use]
pub fn severity_of(result: &CheckResult) -> Severity {
    result.severity
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use sure_domain::execution::{ExecutionMode, ExecutionPermissions};
    use sure_domain::ids::{AnyId, CheckId, IdKind};
    use sure_domain::status::NotCheckedReason;

    fn fingerprint() -> FingerprintId {
        FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddee").expect("a well-formed fingerprint")
    }

    fn planned(title: &str, severity: Severity) -> CheckResult {
        CheckResult::not_run(
            CheckId::generate(),
            title,
            severity,
            true,
            NotCheckedReason::ExecutionNotAuthorized,
            fingerprint(),
        )
    }

    /// A schedule with nothing in it, for the cases whose finding does not need
    /// the check's own reason anchor. The fallback anchor is what those cases
    /// get, and `a_finding_for_a_check_off_the_schedule_points_at_the_check`
    /// is where that is measured.
    fn empty_schedule() -> CheckSchedule {
        crate::schedule::PlanBuilder::new(
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        )
        .build()
    }

    #[test]
    fn a_pass_is_never_a_finding() {
        let result = CheckResult::pass(
            CheckId::generate(),
            "run the tests",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            fingerprint(),
        );
        assert!(!is_a_finding(&result));
        assert!(findings_from_checks(&[result], &empty_schedule()).is_empty());
    }

    #[test]
    fn a_check_that_did_not_run_for_a_reason_that_is_not_a_scope_limit_is_a_finding() {
        let result = planned("run the tests in packages/checkout", Severity::MustFix);
        assert!(is_a_finding(&result));
        let findings = findings_from_checks(std::slice::from_ref(&result), &empty_schedule());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, FindingStatus::CannotConfirm);
        assert_eq!(findings[0].severity, Severity::MustFix);
        assert_eq!(findings[0].title, result.title);
    }

    #[test]
    fn a_check_that_stood_aside_for_a_scope_limit_is_not_a_finding() {
        for reason in [
            NotCheckedReason::NotApplicable,
            NotCheckedReason::UnsupportedStack,
            NotCheckedReason::DisabledByConfiguration,
        ] {
            let result = CheckResult::not_run(
                CheckId::generate(),
                "a check about another stack",
                Severity::MustFix,
                true,
                reason,
                fingerprint(),
            );
            assert!(
                !is_a_finding(&result),
                "{} is a scope limit and became a finding",
                reason.plain_explanation()
            );
        }
    }

    #[test]
    fn the_severity_is_the_checks_own_and_is_not_ranked_here() {
        for severity in [
            Severity::MustFix,
            Severity::ShouldFixFirst,
            Severity::CanFixLater,
            Severity::Note,
        ] {
            let result = planned("a check", severity);
            assert_eq!(severity_of(&result), severity);
            let finding = &findings_from_checks(&[result], &empty_schedule())[0];
            assert_eq!(
                finding.severity, severity,
                "a finding was given a weight the check did not have"
            );
        }
    }

    #[test]
    fn a_finding_for_a_check_that_did_not_run_is_grounded() {
        let result = planned("run the tests in packages/checkout", Severity::MustFix);
        let finding = &findings_from_checks(&[result], &empty_schedule())[0];
        assert!(
            finding.is_grounded(),
            "a must_fix finding the repair contract will refuse is a finding that \
             reaches the report and cannot become a repair"
        );
        assert_eq!(finding.evidence[0].class, EvidenceClass::ObservedFact);
    }

    #[test]
    fn the_order_is_the_plans_order_and_not_sorted() {
        let results: Vec<CheckResult> = ["c", "a", "b"]
            .iter()
            .map(|title| planned(title, Severity::ShouldFixFirst))
            .collect();
        let titles: Vec<String> = findings_from_checks(&results, &empty_schedule())
            .into_iter()
            .map(|finding| finding.title)
            .collect();
        assert_eq!(titles, ["c", "a", "b"]);
    }

    /// A finding whose check is not in the schedule still points at the check.
    ///
    /// [`anchor_for`] has two answers and the schedule decides which: the
    /// check's own reason anchor when the schedule still holds the check, and the
    /// check itself when it does not. Every test above runs against
    /// [`empty_schedule`], so they all exercise the second — and this one is
    /// where that is said rather than assumed, because a reader following the
    /// finding cannot see the schedule.
    ///
    /// **Either way the anchor carries the check's identity.** That identifier is
    /// what [`crate::repair_impact::seed_rechecks`] reads, so a mutation that
    /// dropped `with_subject_id` from either arm would leave every finding in the
    /// product unable to become a repair contract, and this assertion is the one
    /// that fails.
    #[test]
    fn a_finding_for_a_check_off_the_schedule_points_at_the_check() {
        let result = planned("run the tests in packages/checkout", Severity::MustFix);
        let finding = &findings_from_checks(std::slice::from_ref(&result), &empty_schedule())[0];
        let anchor = &finding.evidence[0].anchor;
        assert_eq!(
            anchor.subject,
            AnchorSubject::Check,
            "a finding whose check left the schedule points somewhere else: {anchor:?}"
        );
        assert_eq!(anchor.location, result.title);
        assert_eq!(anchor.locator, "the check itself");
        assert_eq!(
            anchor.subject_id.as_ref().map(AnyId::kind),
            Some(IdKind::Check),
            "the anchor names no check, so nothing can seed a repair contract from it: {anchor:?}"
        );
        assert_eq!(
            anchor.subject_id,
            Some(AnyId::from(result.id.clone())),
            "the anchor does not name the check that produced the finding: {anchor:?}"
        );
    }
}

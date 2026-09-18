//! Select the checks that should be re-run after a repair.
//!
//! The repair contract already carries a `recheck` list from the finding that
//! generated it. This module adds two things to that list:
//!
//! 1. **Affected checks** whose own anchor overlaps an evidence anchor from the
//!    contract, because a fix that touches a location should re-verify any check
//!    that is about that location.
//! 2. **Relevant regression checks** — deterministic checks that run project code
//!    and are serious enough that a side-effect of the fix would matter — because
//!    a repair that only re-runs the original check can hide breakage elsewhere.
//!
//! The result is de-duplicated and kept in schedule order, so the re-check plan
//! is stable and readable.

use sure_domain::evidence::{EvidenceAnchor, EvidenceClass};
use sure_domain::ids::CheckId;
use sure_domain::severity::Severity;
use sure_domain::vocabulary::RepairContract;

use crate::schedule::{CheckSchedule, ScheduledCheck};

/// Select the checks to re-run for `contract` from `schedule`.
///
/// The returned identifiers are in the order they appear in the schedule, with
/// no duplicates. The contract's own `recheck` list is always included first,
/// followed by affected and regression checks discovered from the schedule.
/// Identifiers that are present in the contract but missing from the schedule
/// are still returned, because the contract may outlive the schedule it was
/// built from.
#[must_use]
pub fn select_impacted_checks(contract: &RepairContract, schedule: &CheckSchedule) -> Vec<CheckId> {
    let mut selected: Vec<CheckId> = Vec::new();

    // The finding that generated the contract already named the checks that can
    // observe whether the fix worked.
    for id in &contract.recheck {
        selected.push(id.clone());
    }

    let contract_locations = contract_anchor_locations(contract);

    for scheduled in schedule.checks() {
        let id = scheduled.proposal().id().clone();
        if selected.contains(&id) {
            continue;
        }
        if is_affected(scheduled, &contract_locations) {
            selected.push(id);
            continue;
        }
        if is_relevant_regression(scheduled) {
            selected.push(id);
        }
    }

    selected
}

/// Every non-empty text fragment in the contract's evidence anchors that can be
/// matched against a check reason.
fn contract_anchor_locations(contract: &RepairContract) -> Vec<String> {
    let mut locations: Vec<String> = Vec::new();
    for evidence in &contract.evidence {
        push_location(&mut locations, &evidence.anchor);
    }
    locations
}

fn push_location(locations: &mut Vec<String>, anchor: &EvidenceAnchor) {
    if !anchor.location.trim().is_empty() {
        locations.push(anchor.location.clone());
    }
    if !anchor.locator.trim().is_empty() {
        locations.push(anchor.locator.clone());
    }
}

/// Whether the scheduled check's reason points at a location the contract's
/// evidence also points at.
fn is_affected(scheduled: &ScheduledCheck, contract_locations: &[String]) -> bool {
    let Some(check_anchor) = scheduled.proposal().reason().anchor() else {
        return false;
    };
    let check_locations = [&check_anchor.location, &check_anchor.locator];
    for contract_location in contract_locations {
        for check_location in check_locations {
            if overlaps(contract_location, check_location) {
                return true;
            }
        }
    }
    false
}

/// Two strings overlap when one contains the other, case-insensitively on
/// Windows where file paths are case-insensitive and case-sensitively
/// elsewhere. The goal is to catch `src/email/send.rs` matching
/// `src/email/send.rs:42` without requiring exact equality.
fn overlaps(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return false;
    }
    #[cfg(windows)]
    {
        let left = left.to_lowercase();
        let right = right.to_lowercase();
        left.contains(&right) || right.contains(&left)
    }
    #[cfg(not(windows))]
    {
        left.contains(right) || right.contains(left)
    }
}

/// Whether a check is a serious enough deterministic regression test that it
/// should run after a code change even when its anchor does not overlap the
/// repair location.
fn is_relevant_regression(scheduled: &ScheduledCheck) -> bool {
    let proposal = scheduled.proposal();
    // Only checks that actually exercise project code can catch side effects of
    // a code change. Static inspection checks are already cheap to run, but they
    // are not the regression suite this rule is about.
    if !proposal.requirements().runs_project_code() {
        return false;
    }
    if proposal.evidence_class() != EvidenceClass::DeterministicCheck {
        return false;
    }
    // "Relevant" means serious enough that a regression would matter: must-fix
    // or should-fix-first. Note checks are deliberately excluded because they
    // are not release-blocking.
    matches!(
        proposal.severity(),
        Severity::MustFix | Severity::ShouldFixFirst
    )
}

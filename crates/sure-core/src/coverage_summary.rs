//! Coverage and not-checked summary for a run.
//!
//! A report must say what SURE actually checked, what it skipped, and what it
//! could not run, so that a gap is never mistaken for a clean result. This
//! module builds that sentence from a [`CheckSchedule`] and the [`RunReport`] it
//! produced.

use std::collections::BTreeMap;

use sure_domain::capability::CapabilityReport;
use sure_domain::ids::CheckId;
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, CheckStatus};

use crate::aggregation::RunReport;
use crate::redact::escape_control_characters;
use crate::schedule::CheckSchedule;

/// A plain-language coverage and not-checked summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageNotCheckedSummary {
    /// Checks that produced a real result about the project.
    pub checked_count: usize,
    /// Checks deliberately not run.
    pub skipped_count: usize,
    /// Checks that could not run because of an error or unavailable tool.
    pub could_not_run_count: usize,
    /// One entry per skipped or could-not-run check, ordered for the report.
    pub not_checked: Vec<NotCheckedEntry>,
    /// User-facing description of the support level SURE achieved.
    pub support_level: String,
    /// Whether any critical check is in `not_checked`.
    pub has_critical_gaps: bool,
}

/// One check that did not run, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotCheckedEntry {
    /// The check's stable identity.
    pub check_id: String,
    /// The short human title shown in the report.
    pub check_title: String,
    /// Why the check did not run, in plain language.
    pub reason: String,
    /// Whether this check is critical to hand-off.
    pub is_critical: bool,
}

/// Build a coverage and not-checked summary from a run.
///
/// `schedule` is what the run intended to check and `report` is what came back
/// for it; `capability` is the support level the adapter achieved. The summary
/// is a function of these three arguments and nothing else.
#[must_use]
pub fn summarize(
    schedule: &CheckSchedule,
    report: &RunReport,
    capability: &CapabilityReport,
) -> CoverageNotCheckedSummary {
    let by_id: BTreeMap<&CheckId, &CheckResult> = report
        .results()
        .iter()
        .map(|result| (&result.id, result))
        .collect();

    let mut checked_count = 0_usize;
    let mut skipped_count = 0_usize;
    let mut could_not_run_count = 0_usize;
    let mut with_severity: Vec<(NotCheckedEntry, Severity)> = Vec::new();

    for scheduled in schedule.checks() {
        let id = scheduled.proposal().id();
        let result = match by_id.get(id) {
            Some(result) => (*result).clone(),
            None => continue,
        };

        if result.status.produced_a_result() {
            checked_count += 1;
            continue;
        }

        let (category, reason) = categorize(&result);
        match category {
            Category::Skipped => skipped_count += 1,
            Category::CouldNotRun => could_not_run_count += 1,
        }

        with_severity.push((
            NotCheckedEntry {
                check_id: id.to_string(),
                check_title: escape_control_characters(&result.title),
                reason,
                is_critical: result.critical,
            },
            result.severity,
        ));
    }

    with_severity.sort_by(
        |(left_entry, left_severity), (right_entry, right_severity)| {
            left_entry
                .is_critical
                .cmp(&right_entry.is_critical)
                .reverse()
                .then_with(|| left_severity.rank().cmp(&right_severity.rank()).reverse())
                .then_with(|| left_entry.check_title.cmp(&right_entry.check_title))
        },
    );

    let not_checked: Vec<NotCheckedEntry> =
        with_severity.into_iter().map(|(entry, _)| entry).collect();
    let has_critical_gaps = not_checked.iter().any(|entry| entry.is_critical);

    CoverageNotCheckedSummary {
        checked_count,
        skipped_count,
        could_not_run_count,
        not_checked,
        support_level: capability.tier.plain_description().to_owned(),
        has_critical_gaps,
    }
}

/// Which bucket a not-checked check belongs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Skipped,
    CouldNotRun,
}

/// Decide whether a not-checked result is a skip or a could-not-run, and phrase
/// the reason in plain language.
fn categorize(result: &CheckResult) -> (Category, String) {
    match result.status {
        CheckStatus::Skipped => {
            let reason = result.not_checked_reason.map_or_else(
                || "SURE does not know why this check was skipped.".to_owned(),
                |reason| reason.plain_explanation().to_owned(),
            );
            (Category::Skipped, reason)
        }
        CheckStatus::Error => {
            let reason = if result.reason.trim().is_empty() {
                "SURE's own check failed.".to_owned()
            } else {
                result.reason.clone()
            };
            (Category::CouldNotRun, reason)
        }
        CheckStatus::Unknown => {
            let reason = if result.reason.trim().is_empty() {
                "SURE has no basis for a verdict on this check.".to_owned()
            } else {
                result.reason.clone()
            };
            (Category::CouldNotRun, reason)
        }
        _ => (
            Category::CouldNotRun,
            "SURE has no basis for a verdict on this check.".to_owned(),
        ),
    }
}

impl CoverageNotCheckedSummary {
    /// Whether every scheduled check produced a real result.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.not_checked.is_empty()
    }

    /// How many not-checked checks are critical.
    #[must_use]
    pub fn critical_not_checked_count(&self) -> usize {
        self.not_checked
            .iter()
            .filter(|entry| entry.is_critical)
            .count()
    }

    /// A one-sentence summary of what was and was not checked.
    #[must_use]
    pub fn plain_summary(&self) -> String {
        let total = self.checked_count + self.skipped_count + self.could_not_run_count;
        if self.is_complete() {
            return format!("SURE checked all {total} checks.");
        }

        let mut sentence = format!("SURE checked {} of {total} checks.", self.checked_count);

        let skipped = count_phrase(self.skipped_count, "check", "was skipped", "were skipped");
        let could_not_run = count_phrase(
            self.could_not_run_count,
            "check",
            "could not run",
            "could not run",
        );

        match (self.skipped_count, self.could_not_run_count) {
            (0, 0) => {}
            (0, _) => {
                sentence.push_str(&format!(" {could_not_run}."));
            }
            (_, 0) => {
                sentence.push_str(&format!(" {skipped}."));
            }
            (_, _) => {
                sentence.push_str(&format!(" {skipped} and {could_not_run}."));
            }
        }

        sentence
    }
}

/// Render a count with the right singular or plural form.
fn count_phrase(count: usize, noun: &str, singular_verb: &str, plural_verb: &str) -> String {
    if count == 1 {
        format!("1 {noun} {singular_verb}")
    } else {
        format!("{count} {noun}s {plural_verb}")
    }
}

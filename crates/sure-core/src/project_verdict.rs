//! Overall project verdict and plain-language summary.
//!
//! Produces the final [`ProjectVerdict`] from the pieces of a completed run,
//! and renders it into a plain-language summary suitable for a non-programmer
//! reader.

use sure_domain::capability::CapabilityReport;
use sure_domain::finding::Finding;
use sure_domain::ids::FingerprintId;
use sure_domain::intent::ProjectIntent;
use sure_domain::severity::Severity;
use sure_domain::status::{Aggregate, CheckResult};
use sure_domain::vocabulary::{Claim, ProjectVerdict};

use crate::redact::escape_control_characters;

/// Build a [`ProjectVerdict`] from the pieces of a completed run.
///
/// This is the single place where the inputs that describe a completed run are
/// assembled into the verdict type. It does not recompute or re-derive any
/// field: the aggregate, findings, and not-checked list are taken as given.
#[must_use]
pub fn build_verdict(
    fingerprint: FingerprintId,
    aggregate: Aggregate,
    intent: ProjectIntent,
    capability: CapabilityReport,
    findings: Vec<Finding>,
    not_checked: Vec<CheckResult>,
    claim_checks: Vec<Claim>,
) -> ProjectVerdict {
    ProjectVerdict {
        fingerprint,
        aggregate,
        intent,
        capability,
        findings,
        not_checked,
        claim_checks,
    }
}

/// Render a plain-language overall summary of a verdict.
///
/// The summary includes:
/// - the aggregate headline,
/// - whether the project is ready for hand-off,
/// - whether the user-request caveat applies,
/// - the support/coverage level from the capability report,
/// - the count and severity of open findings,
/// - a clear statement when checks could not run or were skipped.
///
/// Attacker-controlled text is escaped before embedding.
///
/// # False-green protection
///
/// If the aggregate is not green, or there is an open finding whose severity
/// blocks hand-off, the summary always says the project is not ready,
/// regardless of what [`ProjectVerdict::is_ready_for_hand_off`] claims.
#[must_use]
pub fn render_summary(verdict: &ProjectVerdict) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Aggregate headline — this is SURE's own frozen text, so it is safe.
    parts.push(verdict.aggregate.headline.clone());

    // Hand-off readiness — never falsely green.
    let blocked_by_finding = verdict
        .findings
        .iter()
        .any(|f| f.status.needs_attention() && f.severity.blocks_hand_off());
    let aggregate_is_green = verdict.aggregate.is_green();

    if aggregate_is_green && !blocked_by_finding {
        parts.push("This project looks ready to hand off.".to_owned());
    } else {
        parts.push("This project is not ready to hand off.".to_owned());
    }

    // User-request caveat.
    if verdict.must_caveat_requirements() {
        parts.push(sure_domain::status::NO_TRUSTED_INTENT_LIMITATION.to_owned());
    }

    // Capability / support level.
    parts.push(verdict.capability.summary());

    // Open findings — count by severity, most serious first.
    let open = verdict.open_findings();
    if open.is_empty() {
        parts.push("No open findings.".to_owned());
    } else {
        let counts = count_open_findings_by_severity(&open);
        let mut phrases: Vec<String> = Vec::new();
        for &(label, count) in &counts {
            if count == 1 {
                phrases.push(format!("1 {label}"));
            } else {
                phrases.push(format!("{count} {label}"));
            }
        }
        parts.push(format!("Open findings: {}.", phrases.join(", ")));
    }

    // Not-checked checks.
    let not_checked_count = verdict.not_checked.len();
    let critical_not_checked: Vec<&CheckResult> =
        verdict.not_checked.iter().filter(|r| r.critical).collect();
    if not_checked_count > 0 {
        let mut sentence = format!(
            "{} check(s) could not run or were skipped.",
            not_checked_count
        );
        if !critical_not_checked.is_empty() {
            sentence.push_str(&format!(
                " {} of them are critical.",
                critical_not_checked.len()
            ));
            let titles: Vec<String> = critical_not_checked
                .iter()
                .map(|r| escape_control_characters(&r.title))
                .collect();
            if !titles.is_empty() {
                sentence.push_str(&format!(" ({})", titles.join(", ")));
            }
        }
        parts.push(sentence);
    }

    parts.join("\n")
}

fn count_open_findings_by_severity(open: &[&Finding]) -> Vec<(&'static str, usize)> {
    let mut counts = Vec::new();
    for severity in [
        Severity::MustFix,
        Severity::ShouldFixFirst,
        Severity::CanFixLater,
        Severity::Note,
    ] {
        let count = open.iter().filter(|f| f.severity == severity).count();
        if count > 0 {
            counts.push((severity.label(), count));
        }
    }
    counts
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::evidence::{AnchorSubject, Evidence, EvidenceAnchor, EvidenceClass};
    use sure_domain::finding::{
        AssessmentSource, FindingBuilder, FindingStatus, SeverityRationale,
    };
    use sure_domain::ids::CheckId;
    use sure_domain::severity::Severity;
    use sure_domain::status::{
        AggregateSeverity, CheckResult, CheckStatus, CoverageSummary, NotCheckedReason,
        StatusCounts,
    };

    fn fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    fn green_aggregate() -> Aggregate {
        Aggregate {
            severity: AggregateSeverity::Green,
            headline: AggregateSeverity::Green.headline().to_owned(),
            counts: StatusCounts::tally([CheckStatus::Pass]),
            blocking: Vec::new(),
            coverage: CoverageSummary {
                counts: StatusCounts::tally([CheckStatus::Pass]),
                critical_not_checked: Vec::new(),
                critical_errored: Vec::new(),
                critical_failed: Vec::new(),
                critical_out_of_scope: Vec::new(),
                critical_checked: 1,
            },
        }
    }

    fn a_finding(severity: Severity) -> Finding {
        let fp = fingerprint();
        let source = if severity.blocks_hand_off() {
            AssessmentSource::DeterministicCheck
        } else {
            AssessmentSource::ObservedFact
        };
        let rationale = SeverityRationale::for_severity(severity).unwrap();
        FindingBuilder::new(source, rationale)
            .title("A finding")
            .severity(severity)
            .status(FindingStatus::Open)
            .explanation("Something is wrong.")
            .user_impact("It matters.")
            .next_step("Fix it.")
            .fingerprint(fp.clone())
            .evidence(vec![Evidence::new(
                EvidenceClass::ObservedFact,
                "evidence",
                EvidenceAnchor::new(AnchorSubject::File, "src/x.rs", "line 1"),
                Some(fp.clone()),
                severity,
            )])
            .build()
            .expect("valid finding")
    }

    #[test]
    fn build_verdict_assembles_all_fields() {
        let fp = fingerprint();
        let agg = green_aggregate();
        let intent = ProjectIntent::empty();
        let capability = CapabilityReport::cli();
        let verdict = build_verdict(
            fp.clone(),
            agg.clone(),
            intent.clone(),
            capability.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(verdict.fingerprint, fp);
        assert_eq!(verdict.aggregate, agg);
        assert_eq!(verdict.intent, intent);
        assert_eq!(verdict.capability, capability);
        assert!(verdict.findings.is_empty());
        assert!(verdict.not_checked.is_empty());
    }

    #[test]
    fn summary_says_ready_when_green_and_no_findings() {
        let verdict = build_verdict(
            fingerprint(),
            green_aggregate(),
            ProjectIntent::empty(),
            CapabilityReport::cli(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(summary.contains("ready to hand off"), "{summary}");
        assert!(!summary.contains("not ready to hand off"), "{summary}");
    }

    #[test]
    fn summary_says_not_ready_when_aggregate_is_not_green() {
        let aggregate = Aggregate {
            severity: AggregateSeverity::NotEnoughChecked,
            headline: AggregateSeverity::NotEnoughChecked.headline().to_owned(),
            counts: StatusCounts::default(),
            blocking: Vec::new(),
            coverage: CoverageSummary {
                counts: StatusCounts::default(),
                critical_not_checked: Vec::new(),
                critical_errored: Vec::new(),
                critical_failed: Vec::new(),
                critical_out_of_scope: Vec::new(),
                critical_checked: 0,
            },
        };
        let verdict = build_verdict(
            fingerprint(),
            aggregate,
            ProjectIntent::empty(),
            CapabilityReport::cli(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(summary.contains("not ready to hand off"), "{summary}");
        assert!(summary.contains("Not enough could be checked"), "{summary}");
    }

    #[test]
    fn summary_says_not_ready_when_a_blocking_finding_exists_even_if_aggregate_is_green() {
        let verdict = build_verdict(
            fingerprint(),
            green_aggregate(),
            ProjectIntent::empty(),
            CapabilityReport::cli(),
            vec![a_finding(Severity::MustFix)],
            Vec::new(),
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(summary.contains("not ready to hand off"), "{summary}");
    }

    #[test]
    fn summary_includes_open_finding_counts() {
        let verdict = build_verdict(
            fingerprint(),
            green_aggregate(),
            ProjectIntent::empty(),
            CapabilityReport::cli(),
            vec![a_finding(Severity::ShouldFixFirst)],
            Vec::new(),
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(summary.contains("Open findings:"), "{summary}");
        assert!(summary.contains("1 Should fix first"), "{summary}");
    }

    #[test]
    fn summary_surfaces_not_checked_checks() {
        let not_checked = vec![CheckResult::not_run(
            CheckId::generate(),
            "run tests",
            Severity::MustFix,
            true,
            NotCheckedReason::ExecutionNotAuthorized,
            fingerprint(),
        )];
        let verdict = build_verdict(
            fingerprint(),
            green_aggregate(),
            ProjectIntent::empty(),
            CapabilityReport::cli(),
            Vec::new(),
            not_checked,
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(
            summary.contains("could not run or were skipped"),
            "{summary}"
        );
        assert!(summary.contains("run tests"), "{summary}");
    }

    #[test]
    fn summary_escapes_control_characters_in_check_titles() {
        let not_checked = vec![CheckResult::not_run(
            CheckId::generate(),
            "run\ntests",
            Severity::MustFix,
            true,
            NotCheckedReason::ExecutionNotAuthorized,
            fingerprint(),
        )];
        let verdict = build_verdict(
            fingerprint(),
            green_aggregate(),
            ProjectIntent::empty(),
            CapabilityReport::cli(),
            Vec::new(),
            not_checked,
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(
            !summary.contains("run\ntests"),
            "unescaped title must not appear: {summary:?}"
        );
        assert!(summary.contains("run\\ntests"), "{summary}");
    }

    #[test]
    fn summary_includes_after_the_fact_caveat() {
        let verdict = build_verdict(
            fingerprint(),
            green_aggregate(),
            ProjectIntent::empty(),
            CapabilityReport::cli(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(
            summary.contains("cannot confirm that it matches your original request"),
            "{summary}"
        );
    }

    #[test]
    fn summary_does_not_include_caveat_when_intent_is_trusted() {
        let intent = ProjectIntent::from_requirements(vec![sure_domain::intent::Requirement::new(
            "r1",
            "build a thing",
            sure_domain::intent::IntentSource::ExplicitUserGoal,
        )]);
        let verdict = build_verdict(
            fingerprint(),
            green_aggregate(),
            intent,
            CapabilityReport::cli(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let summary = render_summary(&verdict);
        assert!(
            !summary.contains("cannot confirm that it matches your original request"),
            "{summary}"
        );
    }
}

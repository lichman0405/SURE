//! Plain-language contract for material findings.
//!
//! [`PlainLanguageFinding`] is the shape a finding takes when it is shown to
//! someone who is not a programmer. It keeps the four things a reader needs:
//! what is wrong, what it means, how serious it is, and what should happen
//! next. It also carries a simplified, redacted list of checkable evidence
//! anchors and a flag that says whether the finding rests only on model output
//! or inference.

use serde::{Deserialize, Serialize};
use sure_domain::evidence::{AnchorSubject, Evidence, EvidenceAnchor};
use sure_domain::finding::{AssessmentSource, Finding, FindingStatus};

use crate::redact::{escape_control_characters, redact};

const FALLBACK_IMPACT: &str = "SURE did not record what this means for the project.";
const FALLBACK_NEXT_ACTION: &str = "Review the evidence and decide what to do next.";

/// A finding rendered for a non-programmer reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlainLanguageFinding {
    /// Short title of the finding.
    pub title: String,
    /// What is wrong, in plain language.
    pub what: String,
    /// What it means for the person reading the report.
    pub impact: String,
    /// User-facing severity label.
    pub severity_label: String,
    /// What SURE thinks should happen next.
    pub next_action: String,
    /// User-facing finding status.
    pub status_label: String,
    /// Whether the finding rests only on model output, inference, or no source.
    pub is_model_only: bool,
    /// Simplified list of checkable evidence anchors.
    pub evidence_anchors: Vec<EvidenceAnchorSummary>,
    #[serde(skip)]
    what_was_fallback: bool,
    #[serde(skip)]
    impact_was_fallback: bool,
    #[serde(skip)]
    next_action_was_fallback: bool,
}

impl PlainLanguageFinding {
    /// Whether this finding answers all four plain-language questions with real
    /// text rather than fallbacks.
    #[must_use]
    pub fn is_material(&self) -> bool {
        !self.what_was_fallback && !self.impact_was_fallback && !self.next_action_was_fallback
    }
}

/// A simplified, checkable pointer back to one piece of evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAnchorSummary {
    /// Where a human finds this: a project-relative path, a command line, or a
    /// short description of the observation point.
    pub location: String,
    /// The specific thing at that location: a line range, a key, a field, a rule.
    pub locator: String,
    /// The stable wire name for what kind of thing the anchor points at.
    pub subject: String,
}

/// Render one finding into the plain-language contract.
#[must_use]
pub fn render_finding(finding: &Finding) -> PlainLanguageFinding {
    let (what, what_was_fallback) = non_empty_or(&finding.explanation, || finding.title.clone());
    let (impact, impact_was_fallback) =
        non_empty_or(&finding.user_impact, || FALLBACK_IMPACT.to_owned());
    let (next_action, next_action_was_fallback) =
        non_empty_or(&finding.next_step, || FALLBACK_NEXT_ACTION.to_owned());

    PlainLanguageFinding {
        title: escape_control_characters(&redact(&finding.title)),
        what: escape_control_characters(&redact(&what)),
        impact: escape_control_characters(&redact(&impact)),
        severity_label: finding.severity.label().to_owned(),
        next_action: escape_control_characters(&redact(&next_action)),
        status_label: status_label(finding.status).to_owned(),
        is_model_only: is_model_only(finding),
        evidence_anchors: evidence_anchor_summaries(&finding.evidence),
        what_was_fallback,
        impact_was_fallback,
        next_action_was_fallback,
    }
}

/// Render a batch of findings, ordered most severe first, then by title.
#[must_use]
pub fn render_findings(findings: &[Finding]) -> Vec<PlainLanguageFinding> {
    let mut indices: Vec<usize> = (0..findings.len()).collect();
    indices.sort_by(|&i, &j| {
        findings[j]
            .severity
            .cmp(&findings[i].severity)
            .then_with(|| findings[i].title.cmp(&findings[j].title))
    });
    indices
        .into_iter()
        .map(|i| render_finding(&findings[i]))
        .collect()
}

fn non_empty_or(value: &str, fallback: impl FnOnce() -> String) -> (String, bool) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        (fallback(), true)
    } else {
        (trimmed.to_owned(), false)
    }
}

fn status_label(status: FindingStatus) -> &'static str {
    match status {
        FindingStatus::Open => "Open",
        FindingStatus::Resolved => "Resolved",
        FindingStatus::AcceptedRisk => "Accepted risk",
        FindingStatus::CannotConfirm => "Cannot confirm",
    }
}

fn is_model_only(finding: &Finding) -> bool {
    let strongest_is_model = finding
        .evidence
        .iter()
        .min_by_key(|e| e.class.truth_rank())
        .map(|e| e.anchor.subject == AnchorSubject::Model)
        .unwrap_or(false);

    if strongest_is_model {
        return true;
    }

    let source_is_uncertain = matches!(
        finding.assessment_source,
        AssessmentSource::ModelAssessment | AssessmentSource::Inference | AssessmentSource::Unknown
    );
    let has_checkable_anchor = finding.evidence.iter().any(|e| e.anchor.is_checkable());

    source_is_uncertain && !has_checkable_anchor
}

fn evidence_anchor_summaries(evidence: &[Evidence]) -> Vec<EvidenceAnchorSummary> {
    evidence
        .iter()
        .filter(|e| e.anchor.is_checkable())
        .map(|e| summary_from_anchor(&e.anchor))
        .collect()
}

fn summary_from_anchor(anchor: &EvidenceAnchor) -> EvidenceAnchorSummary {
    EvidenceAnchorSummary {
        location: escape_control_characters(&redact(&anchor.location)),
        locator: escape_control_characters(&redact(&anchor.locator)),
        subject: anchor.subject.as_str().to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::evidence::{EvidenceAnchor, EvidenceClass};
    use sure_domain::finding::{
        AssessmentSource, FindingBuilder, FindingStatus, SeverityRationale,
    };
    use sure_domain::ids::FingerprintId;
    use sure_domain::severity::Severity;

    fn fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    fn fully_populated_finding() -> Finding {
        FindingBuilder::new(
            AssessmentSource::ObservedFact,
            SeverityRationale::BlocksHandOff,
        )
        .title("Payment looks successful but nothing is charged")
        .severity(Severity::MustFix)
        .status(FindingStatus::Open)
        .explanation("The checkout prints a success message without calling the payment provider.")
        .user_impact("Customers believe they have paid, but no money moves.")
        .next_step("Call the payment provider before publishing.")
        .fingerprint(fingerprint())
        .evidence(vec![Evidence::new(
            EvidenceClass::ObservedFact,
            "success message without charge",
            EvidenceAnchor::new(AnchorSubject::File, "src/checkout.rs", "lines 40-52"),
            Some(fingerprint()),
            Severity::MustFix,
        )])
        .build()
        .expect("valid finding")
    }

    #[test]
    fn fully_populated_finding_renders_as_material() {
        let plain = render_finding(&fully_populated_finding());

        assert!(plain.is_material());
        assert_eq!(
            plain.title,
            "Payment looks successful but nothing is charged"
        );
        assert!(plain.what.contains("checkout prints a success message"));
        assert!(plain.impact.contains("Customers believe"));
        assert!(plain.next_action.contains("Call the payment provider"));
        assert_eq!(plain.severity_label, "Must fix");
        assert_eq!(plain.status_label, "Open");
        assert!(!plain.is_model_only);
        assert_eq!(plain.evidence_anchors.len(), 1);
    }

    #[test]
    fn missing_explanation_falls_back_to_title() {
        let finding = FindingBuilder::new(
            AssessmentSource::DeterministicCheck,
            SeverityRationale::ReliabilityOrQualityRisk,
        )
        .title("Title becomes the what")
        .severity(Severity::ShouldFixFirst)
        .user_impact("Something matters.")
        .next_step("Do the thing.")
        .fingerprint(fingerprint())
        .build()
        .expect("valid finding");

        let plain = render_finding(&finding);
        assert_eq!(plain.what, plain.title);
        assert!(!plain.is_material());
    }

    #[test]
    fn missing_impact_and_next_action_use_fallbacks_and_are_not_material() {
        let finding = FindingBuilder::new(
            AssessmentSource::DeterministicCheck,
            SeverityRationale::NonBlockingImprovement,
        )
        .title("Something is off")
        .severity(Severity::CanFixLater)
        .explanation("A real explanation.")
        .fingerprint(fingerprint())
        .build()
        .expect("valid finding");

        let plain = render_finding(&finding);
        assert_eq!(plain.impact, FALLBACK_IMPACT);
        assert_eq!(plain.next_action, FALLBACK_NEXT_ACTION);
        assert!(!plain.is_material());
    }

    #[test]
    fn control_characters_in_text_are_escaped() {
        let finding = FindingBuilder::new(
            AssessmentSource::DeterministicCheck,
            SeverityRationale::NonBlockingImprovement,
        )
        .title("Line\nbreak")
        .severity(Severity::CanFixLater)
        .explanation("Tab\there")
        .user_impact("Carriage\rreturn")
        .next_step("Bell\x07character")
        .fingerprint(fingerprint())
        .build()
        .expect("valid finding");

        let plain = render_finding(&finding);
        assert!(!plain.title.contains('\n'));
        assert_eq!(plain.title, "Line\\nbreak");
        assert!(!plain.what.contains('\t'));
        assert_eq!(plain.what, "Tab\\there");
        assert!(!plain.impact.contains('\r'));
        assert_eq!(plain.impact, "Carriage\\rreturn");
        assert!(!plain.next_action.contains('\x07'));
        assert_eq!(plain.next_action, "Bell\\u{0007}character");
    }

    #[test]
    fn model_only_finding_is_flagged_and_omits_checkable_anchors() {
        let finding = FindingBuilder::new(
            AssessmentSource::ModelAssessment,
            SeverityRationale::Informational,
        )
        .title("Model-only uncertainty")
        .severity(Severity::Note)
        .explanation("The model inferred something but cannot point to a project location.")
        .user_impact("This may be nothing.")
        .next_step("Look for a concrete symptom.")
        .fingerprint(fingerprint())
        .evidence(vec![Evidence::new(
            EvidenceClass::ModelAssessment,
            "inferred from prompt",
            EvidenceAnchor::model_only("model-only conclusion"),
            Some(fingerprint()),
            Severity::Note,
        )])
        .build()
        .expect("valid note finding");

        let plain = render_finding(&finding);
        assert!(plain.is_model_only);
        assert!(plain.evidence_anchors.is_empty());
    }

    #[test]
    fn intent_and_claim_anchors_produce_evidence_anchor_summaries() {
        let finding = FindingBuilder::new(
            AssessmentSource::ClaimAssessment(sure_domain::evidence::ClaimAssessment::Contradicted),
            SeverityRationale::BlocksHandOff,
        )
        .title("A claim was contradicted")
        .severity(Severity::MustFix)
        .explanation("The agent claimed X but the project shows Y.")
        .user_impact("The feature does not behave as described.")
        .next_step("Reconcile the claim with the implementation.")
        .fingerprint(fingerprint())
        .evidence(vec![
            Evidence::new(
                EvidenceClass::ObservedFact,
                "project intent requirement",
                EvidenceAnchor::intent("project-intent.json", "requirement P-7"),
                Some(fingerprint()),
                Severity::MustFix,
            ),
            Evidence::new(
                EvidenceClass::DeterministicCheck,
                "agent claim",
                EvidenceAnchor::claim("agent-transcript.md", "claim #42"),
                Some(fingerprint()),
                Severity::MustFix,
            ),
        ])
        .build()
        .expect("valid finding");

        let plain = render_finding(&finding);
        assert!(!plain.is_model_only);
        assert_eq!(plain.evidence_anchors.len(), 2);

        let intent_summary = plain
            .evidence_anchors
            .iter()
            .find(|a| a.subject == "intent")
            .expect("intent anchor present");
        assert_eq!(intent_summary.location, "project-intent.json");
        assert_eq!(intent_summary.locator, "requirement P-7");

        let claim_summary = plain
            .evidence_anchors
            .iter()
            .find(|a| a.subject == "claim")
            .expect("claim anchor present");
        assert_eq!(claim_summary.location, "agent-transcript.md");
        assert_eq!(claim_summary.locator, "claim #42");
    }

    #[test]
    fn control_characters_in_evidence_anchor_location_and_locator_are_escaped() {
        let finding = FindingBuilder::new(
            AssessmentSource::ObservedFact,
            SeverityRationale::BlocksHandOff,
        )
        .title("Anchor with control characters")
        .severity(Severity::MustFix)
        .explanation("Evidence points to a malicious-looking path.")
        .user_impact("It matters.")
        .next_step("Sanitise the input.")
        .fingerprint(fingerprint())
        .evidence(vec![Evidence::new(
            EvidenceClass::ObservedFact,
            "evidence",
            EvidenceAnchor::new(AnchorSubject::File, "src/escape\nme.rs", "line\t1"),
            Some(fingerprint()),
            Severity::MustFix,
        )])
        .build()
        .expect("valid finding");

        let plain = render_finding(&finding);
        let summary = plain.evidence_anchors.first().expect("one anchor");
        assert!(!summary.location.contains('\n'));
        assert_eq!(summary.location, "src/escape\\nme.rs");
        assert!(!summary.locator.contains('\t'));
        assert_eq!(summary.locator, "line\\t1");
    }

    #[test]
    fn ordering_is_deterministic_by_severity_then_title() {
        let note_b = FindingBuilder::new(
            AssessmentSource::ObservedFact,
            SeverityRationale::Informational,
        )
        .title("B note")
        .severity(Severity::Note)
        .fingerprint(fingerprint())
        .build()
        .expect("valid");
        let must_a = FindingBuilder::new(
            AssessmentSource::ObservedFact,
            SeverityRationale::BlocksHandOff,
        )
        .title("A must fix")
        .severity(Severity::MustFix)
        .fingerprint(fingerprint())
        .build()
        .expect("valid");
        let should_c = FindingBuilder::new(
            AssessmentSource::DeterministicCheck,
            SeverityRationale::ReliabilityOrQualityRisk,
        )
        .title("C should fix")
        .severity(Severity::ShouldFixFirst)
        .fingerprint(fingerprint())
        .build()
        .expect("valid");
        let must_z = FindingBuilder::new(
            AssessmentSource::ObservedFact,
            SeverityRationale::BlocksHandOff,
        )
        .title("Z must fix")
        .severity(Severity::MustFix)
        .fingerprint(fingerprint())
        .build()
        .expect("valid");

        let rendered = render_findings(&[note_b, must_a, should_c, must_z]);
        let titles: Vec<_> = rendered.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["A must fix", "Z must fix", "C should fix", "B note"]
        );
    }

    #[test]
    fn resolved_and_accepted_risk_keep_their_status_labels() {
        for (status, label) in [
            (FindingStatus::Resolved, "Resolved"),
            (FindingStatus::AcceptedRisk, "Accepted risk"),
        ] {
            let finding = FindingBuilder::new(
                AssessmentSource::ObservedFact,
                SeverityRationale::BlocksHandOff,
            )
            .title("A problem")
            .severity(Severity::MustFix)
            .status(status)
            .explanation("Still explained.")
            .user_impact("Still impacts.")
            .next_step("Still has a next step.")
            .fingerprint(fingerprint())
            .build()
            .expect("valid finding");

            let plain = render_finding(&finding);
            assert_eq!(plain.status_label, label);
        }
    }
}

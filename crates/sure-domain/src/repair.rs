//! Repair-contract generation from grounded findings.
//!
//! A repair contract is only as honest as the finding it comes from. This module
//! refuses to turn an ungrounded finding into a contract, and every acceptance
//! condition it produces is tied to an observable check or anchor rather than to
//! an invented fact.

use crate::evidence::EvidenceClass;
use crate::finding::Finding;
use crate::ids::{CheckId, RepairId};
use crate::vocabulary::RepairContract;

/// Why a repair contract could not be generated from a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairFromFindingError {
    /// The finding is not grounded in checkable evidence, so a contract would
    /// have to invent facts.
    NotGrounded,
    /// No checks were supplied to re-run after the repair, so SURE could not
    /// observe whether the fix worked.
    NoRechecks,
}

impl std::fmt::Display for RepairFromFindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotGrounded => write!(
                f,
                "the finding is not grounded in checkable evidence, so a repair contract cannot be generated"
            ),
            Self::NoRechecks => write!(
                f,
                "at least one check must be re-run after the repair so SURE can observe the result"
            ),
        }
    }
}

impl std::error::Error for RepairFromFindingError {}

impl RepairContract {
    /// Build a repair contract from a grounded finding and the checks to re-run.
    ///
    /// Returns an error if the finding is not grounded or if no re-checks are
    /// supplied. The contract copies the finding's own evidence rather than
    /// inventing new facts, and the acceptance criteria are phrased as
    /// observable re-check outcomes.
    ///
    /// The `recheck` list should contain the checks that can observe whether the
    /// fix worked. P9-T004 is responsible for selecting the right checks.
    pub fn from_finding(
        finding: &Finding,
        recheck: Vec<CheckId>,
    ) -> Result<Self, RepairFromFindingError> {
        if !finding.is_grounded() {
            return Err(RepairFromFindingError::NotGrounded);
        }
        if recheck.is_empty() {
            return Err(RepairFromFindingError::NoRechecks);
        }

        let required_fix = if finding.next_step.trim().is_empty() {
            vec![format!("Address the finding: {}", finding.title)]
        } else {
            vec![finding.next_step.clone()]
        };

        let acceptance = observable_acceptance(finding, &recheck);
        let forbidden_shortcuts = forbidden_shortcuts_for(finding);

        Ok(Self {
            id: RepairId::generate(),
            issue_id: finding.id.clone(),
            problem: finding.title.clone(),
            why_it_matters: finding.user_impact.clone(),
            required_fix,
            preserve: Vec::new(),
            acceptance,
            recheck,
            forbidden_shortcuts,
            evidence: finding.evidence.clone(),
        })
    }

    /// Convenience constructor when the finding's own id is the only issue id
    /// needed and a single check is enough to verify the repair.
    pub fn from_finding_with_one_check(
        finding: &Finding,
        recheck: CheckId,
    ) -> Result<Self, RepairFromFindingError> {
        Self::from_finding(finding, vec![recheck])
    }
}

fn observable_acceptance(finding: &Finding, recheck: &[CheckId]) -> Vec<String> {
    let mut conditions = Vec::new();

    if recheck.len() == 1 {
        conditions.push(format!(
            "Re-run check {} and confirm it passes for {}.",
            recheck[0].as_str(),
            finding.title
        ));
    } else {
        let list = recheck
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        conditions.push(format!(
            "Re-run checks {} and confirm they all pass for {}.",
            list, finding.title
        ));
    }

    // If the strongest evidence is an observed fact with a concrete anchor,
    // add an acceptance condition that points at the same anchor so the fix
    // cannot be declared done without touching the actual location.
    if let Some(strongest) = finding.evidence.iter().min_by_key(|e| e.class.truth_rank())
        && strongest.class == EvidenceClass::ObservedFact
        && strongest.anchor.is_checkable()
    {
        conditions.push(format!(
            "The fix is visible at {} ({}).",
            strongest.anchor.location, strongest.anchor.locator
        ));
    }

    conditions
}

fn forbidden_shortcuts_for(finding: &Finding) -> Vec<String> {
    let mut shortcuts = Vec::new();

    if finding.strongest_evidence_class() == EvidenceClass::DeterministicCheck {
        shortcuts.push(String::from(
            "Changing only the test or assertion so it passes without fixing the underlying behavior.",
        ));
    }

    if finding.strongest_evidence_class() == EvidenceClass::ObservedFact {
        shortcuts.push(String::from(
            "Hiding the symptom in the observed output without fixing the code path that produces it.",
        ));
    }

    // A model-only or inferred finding should not be used to demand a specific
    // code change, but by the time we generate a contract the finding has
    // already been required to be grounded, so this is a defense-in-depth note.
    if !finding.is_grounded() {
        shortcuts.push(String::from(
            "Treating an ungrounded suggestion as a confirmed defect and changing code based only on that suggestion.",
        ));
    }

    shortcuts
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::evidence::{
        AnchorSubject, ClaimAssessment, Evidence, EvidenceAnchor, EvidenceClass,
    };
    use crate::finding::{AssessmentSource, FindingBuilder, FindingStatus, SeverityRationale};
    use crate::ids::{CheckId, FingerprintId};
    use crate::severity::Severity;

    fn fingerprint() -> FingerprintId {
        FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddee").unwrap()
    }

    fn grounded_finding() -> Finding {
        FindingBuilder::new(
            AssessmentSource::DeterministicCheck,
            SeverityRationale::BlocksHandOff,
        )
        .id(crate::ids::FindingId::generate())
        .title("Email is reported as sent but nothing is sent")
        .severity(Severity::MustFix)
        .status(FindingStatus::Open)
        .explanation("The app says the email went out, and the code only prints a line.")
        .user_impact("People believe a message was delivered when it was not.")
        .next_step("Call the configured email provider on the real send path.")
        .evidence(vec![Evidence::new(
            EvidenceClass::DeterministicCheck,
            "the send path returns before the provider is called",
            EvidenceAnchor::new(AnchorSubject::File, "src/email/send.rs", "line 42"),
            Some(fingerprint()),
            Severity::MustFix,
        )])
        .fingerprint(fingerprint())
        .build()
        .expect("fixture finding is valid")
    }

    #[test]
    fn a_grounded_finding_becomes_an_actionable_contract() {
        let finding = grounded_finding();
        let recheck = CheckId::generate();
        let contract = RepairContract::from_finding_with_one_check(&finding, recheck.clone())
            .expect("contract is generated");

        assert_eq!(contract.issue_id, finding.id);
        assert_eq!(contract.problem, finding.title);
        assert_eq!(contract.why_it_matters, finding.user_impact);
        assert_eq!(contract.required_fix, vec![finding.next_step]);
        assert_eq!(contract.recheck, vec![recheck]);
        assert!(!contract.acceptance.is_empty());
        assert!(contract.is_actionable());
    }

    #[test]
    fn an_ungrounded_finding_is_rejected() {
        let mut finding = grounded_finding();
        finding.evidence.clear();
        assert!(!finding.is_grounded());

        let result = RepairContract::from_finding_with_one_check(&finding, CheckId::generate());
        assert_eq!(result.unwrap_err(), RepairFromFindingError::NotGrounded);
    }

    #[test]
    fn a_contract_without_rechecks_is_rejected() {
        let finding = grounded_finding();
        let result = RepairContract::from_finding(&finding, Vec::new());
        assert_eq!(result.unwrap_err(), RepairFromFindingError::NoRechecks);
    }

    #[test]
    fn acceptance_conditions_are_observable() {
        let finding = grounded_finding();
        let recheck = vec![CheckId::generate(), CheckId::generate()];
        let contract =
            RepairContract::from_finding(&finding, recheck.clone()).expect("contract is generated");

        for condition in &contract.acceptance {
            assert!(
                condition.contains("Re-run") || condition.contains("visible at"),
                "acceptance must be observable: {condition}"
            );
        }
        let joined = contract.acceptance.join(" ");
        assert!(
            joined.contains(recheck[0].as_str()) && joined.contains(recheck[1].as_str()),
            "acceptance must name the checks to re-run"
        );
    }

    #[test]
    fn no_invented_evidence_is_added() {
        let finding = grounded_finding();
        let contract = RepairContract::from_finding_with_one_check(&finding, CheckId::generate())
            .expect("contract is generated");
        assert_eq!(contract.evidence.len(), finding.evidence.len());
        assert_eq!(contract.evidence, finding.evidence);
    }

    #[test]
    fn forbidden_shortcuts_warn_against_fake_fixes() {
        let finding = grounded_finding();
        let contract = RepairContract::from_finding_with_one_check(&finding, CheckId::generate())
            .expect("contract is generated");
        assert!(
            contract
                .forbidden_shortcuts
                .iter()
                .any(|s| s.contains("test")),
            "deterministic-check findings should warn against test-only fixes"
        );
    }

    #[test]
    fn claim_assessment_contradicted_can_generate_contract() {
        let finding = FindingBuilder::new(
            AssessmentSource::ClaimAssessment(ClaimAssessment::Contradicted),
            SeverityRationale::BlocksHandOff,
        )
        .id(crate::ids::FindingId::generate())
        .title("Agent claimed tests pass but a test failure was recorded")
        .severity(Severity::MustFix)
        .status(FindingStatus::Open)
        .explanation("The agent said all tests pass, but a recorded test event shows a failure.")
        .user_impact("A claimed completion may not be complete.")
        .next_step("Fix the failing test or the code it exercises.")
        .evidence(vec![Evidence::new(
            EvidenceClass::ObservedFact,
            "recorded test event shows exit code 1",
            EvidenceAnchor::new(AnchorSubject::Event, "test.run", "harness-42"),
            Some(fingerprint()),
            Severity::MustFix,
        )])
        .fingerprint(fingerprint())
        .build()
        .expect("fixture finding is valid");

        let contract = RepairContract::from_finding_with_one_check(&finding, CheckId::generate())
            .expect("contract is generated");
        assert!(contract.is_actionable());
        assert!(
            contract
                .forbidden_shortcuts
                .iter()
                .any(|s| s.contains("hiding") || s.contains("symptom"))
        );
    }
}

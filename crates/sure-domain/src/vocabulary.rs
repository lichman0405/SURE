//! The frozen core domain vocabulary.
//!
//! Every name here is fixed by `docs/architecture/DOMAIN_MODEL.md`. These types
//! are deliberately thin: they carry identity, provenance and outcome, and hold
//! no checking logic, so that the check engine can be replaced without changing
//! what a finding means.

use serde::{Deserialize, Serialize};

use crate::capability::CapabilityReport;
use crate::evidence::{ClaimAssessment, Evidence, EvidenceClass};
use crate::execution::ExecutionMode;
use crate::ids::{CheckId, ClaimId, EventId, FingerprintId, ProjectId, RepairId, SessionId};
use crate::intent::ProjectIntent;
use crate::severity::Severity;
use crate::status::{Aggregate, CheckResult, NotCheckedReason};
use crate::variants::variants;

/// A local software project or workspace under inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Stable identity for this project within SURE's local storage.
    pub id: ProjectId,
    /// Human-readable name, usually the directory name.
    pub name: String,
    /// Absolute path to the project root.
    pub root: String,
    /// What kind of project SURE believes this is, with its support level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stacks: Vec<StackClassification>,
}

impl Project {
    /// Build a project record.
    #[must_use]
    pub fn new(id: ProjectId, name: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            root: root.into(),
            stacks: Vec::new(),
        }
    }
}

/// How well SURE supports a discovered stack, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    /// Framework-aware discovery and meaningful deterministic checks.
    FirstClass,
    /// Common manifests and commands are discoverable, with no claim of
    /// framework-specific completeness.
    Generic,
    /// Files and configuration can be inspected, but there is no safe or
    /// reliable way to run it.
    InspectOnly,
}

variants!(SupportLevel {
    FirstClass,
    Generic,
    InspectOnly
});

impl SupportLevel {
    /// The letter used in the product documentation.
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            Self::FirstClass => 'A',
            Self::Generic => 'B',
            Self::InspectOnly => 'C',
        }
    }

    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FirstClass => "first_class",
            Self::Generic => "generic",
            Self::InspectOnly => "inspect_only",
        }
    }

    /// Plain-language description of what the user gets.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::FirstClass => {
                "SURE understands this kind of project well and can check it properly."
            }
            Self::Generic => {
                "SURE can find how this project is built and run, but does not understand everything about it."
            }
            Self::InspectOnly => "SURE can look at this project's files, but cannot safely run it.",
        }
    }
}

/// One technology SURE found in the project, at a stated support level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackClassification {
    /// A stable stack identifier, for example `node`, `python`, `rust`.
    pub stack: String,
    /// How well SURE supports it.
    pub level: SupportLevel,
    /// Why SURE assigned that level, so the claim can be audited.
    pub reason: String,
}

/// How a project's identity was computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintKind {
    /// Derived from Git state.
    Git,
    /// Derived from a deterministic content manifest, for projects without Git.
    Content,
}

variants!(FingerprintKind { Git, Content });

impl FingerprintKind {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Content => "content",
        }
    }
}

/// The Git state a fingerprint was taken from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitState {
    /// The commit that was checked out.
    pub head: String,
    /// Whether tracked files differed from `head` at capture time.
    pub dirty: bool,
    /// Digest over the tracked diff, when the tree was dirty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty_digest: Option<String>,
    /// Digest over the untracked files that matter, when any were present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub untracked_digest: Option<String>,
    /// The current branch name, when the repository is not detached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// The identity of exactly the project state that evidence applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFingerprint {
    /// Identity of this fingerprint.
    pub id: FingerprintId,
    /// How the fingerprint was computed.
    pub kind: FingerprintKind,
    /// The digest itself. Two states share a digest only if they are the same state
    /// as far as the relevant checks are concerned.
    pub digest: String,
    /// Git detail, when the project is a Git repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitState>,
}

impl ProjectFingerprint {
    /// A content fingerprint.
    #[must_use]
    pub fn content(digest: impl Into<String>) -> Self {
        Self {
            id: FingerprintId::generate(),
            kind: FingerprintKind::Content,
            digest: digest.into(),
            git: None,
        }
    }

    /// A fingerprint derived from Git state.
    #[must_use]
    pub fn git(digest: impl Into<String>, state: GitState) -> Self {
        Self {
            id: FingerprintId::generate(),
            kind: FingerprintKind::Git,
            digest: digest.into(),
            git: Some(state),
        }
    }

    /// Whether this fingerprint describes the same project state as another.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.kind == other.kind && self.digest == other.digest
    }
}

/// Observed coding-harness activity related to a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Identity of this session.
    pub id: SessionId,
    /// The project the session worked on.
    pub project: ProjectId,
    /// The harness that produced the session, for example `claude-code`.
    pub source: String,
    /// What SURE could actually see, reported by the adapter.
    pub capability: CapabilityReport,
    /// When the session started, as an RFC 3339 timestamp.
    pub started_at: String,
}

/// A normalized observed harness event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Identity of this event.
    pub id: EventId,
    /// The session the event belongs to.
    pub session: SessionId,
    /// The semantic event name, for example `action.completed`.
    pub event_type: String,
    /// When it happened, as an RFC 3339 timestamp.
    pub timestamp: String,
    /// The event body, already normalized and redacted.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// The approved set of checks for a project state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckPlan {
    /// Identity of the plan.
    pub id: String,
    /// The project state this plan was built for.
    pub fingerprint: FingerprintId,
    /// The execution mode the plan was built under.
    pub mode: ExecutionMode,
    /// Checks that do not execute any project code.
    #[serde(default)]
    pub static_checks: Vec<CheckId>,
    /// Checks that would execute project code.
    #[serde(default)]
    pub dynamic_checks: Vec<CheckId>,
    /// Checks deliberately excluded, and why. These stay visible in the report.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<NotCheckedReason>,
}

impl CheckPlan {
    /// Build an empty plan for a project state.
    #[must_use]
    pub fn new(id: impl Into<String>, fingerprint: FingerprintId, mode: ExecutionMode) -> Self {
        Self {
            id: id.into(),
            fingerprint,
            mode,
            static_checks: Vec::new(),
            dynamic_checks: Vec::new(),
            excluded: Vec::new(),
        }
    }

    /// Every check in the plan.
    #[must_use]
    pub fn all_checks(&self) -> Vec<&CheckId> {
        self.static_checks
            .iter()
            .chain(self.dynamic_checks.iter())
            .collect()
    }
}

/// Whether a finding is still standing.
///
/// The wire names are frozen by `schemas/finding.schema.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    /// Still a problem.
    Open,
    /// Later evidence showed the problem is gone.
    Resolved,
    /// The user decided to live with it.
    AcceptedRisk,
    /// SURE cannot tell whether the problem is still there.
    CannotConfirm,
}

variants!(FindingStatus {
    Open,
    Resolved,
    AcceptedRisk,
    CannotConfirm
});

impl FindingStatus {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::AcceptedRisk => "accepted_risk",
            Self::CannotConfirm => "cannot_confirm",
        }
    }

    /// Whether this finding still needs attention.
    ///
    /// `CannotConfirm` counts as needing attention: SURE does not close a
    /// finding merely because it can no longer see the problem.
    #[must_use]
    pub const fn needs_attention(self) -> bool {
        matches!(self, Self::Open | Self::CannotConfirm)
    }
}

/// A material user-facing problem or uncertainty, anchored in evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Identity of the finding.
    pub id: crate::ids::FindingId,
    /// Short title, written for someone who is not a programmer.
    pub title: String,
    /// How serious it is.
    pub severity: Severity,
    /// Whether it is still standing.
    pub status: FindingStatus,
    /// What is wrong, in plain language.
    pub explanation: String,
    /// What it means for the person reading the report.
    pub user_impact: String,
    /// What SURE thinks should happen next.
    pub next_step: String,
    /// The evidence that grounds this finding.
    pub evidence: Vec<Evidence>,
    /// Which project state the finding was raised against.
    pub fingerprint: FingerprintId,
    /// Detail the user can expand, already redacted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technical_details: Option<serde_json::Value>,
}

impl Finding {
    /// Whether this finding is honest about its own basis.
    ///
    /// A `must_fix` finding needs at least one anchor that is not a model
    /// opinion or a guess.
    #[must_use]
    pub fn is_grounded(&self) -> bool {
        if !self.severity.blocks_hand_off() {
            return !self.evidence.is_empty();
        }
        self.evidence
            .iter()
            .any(|e| e.class.can_alone_support_must_fix() && e.anchor.is_checkable())
    }

    /// The strongest evidence class behind this finding.
    #[must_use]
    pub fn strongest_evidence_class(&self) -> EvidenceClass {
        self.evidence
            .iter()
            .map(|e| e.class)
            .min_by_key(|c| c.truth_rank())
            .unwrap_or(EvidenceClass::Unknown)
    }
}

/// A statement a coding agent made that can sometimes be checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    /// Identity of the claim.
    pub id: ClaimId,
    /// The claim, quoted or transcribed from the agent.
    pub claim_text: String,
    /// What kind of claim it is, for example `tests_pass` or `feature_complete`.
    pub claim_type: String,
    /// What checking it produced.
    pub assessment: ClaimAssessment,
    /// The evidence behind the assessment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    /// The session the claim came from, when it came from one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
}

impl Claim {
    /// Whether SURE actually reached a verdict, as opposed to running out of evidence.
    #[must_use]
    pub const fn has_verdict(&self) -> bool {
        matches!(
            self.assessment,
            ClaimAssessment::Confirmed | ClaimAssessment::Contradicted
        )
    }
}

/// Bounded instructions and acceptance conditions handed to a coding harness.
///
/// The wire form is `schemas/repair.schema.json`, and
/// `docs/architecture/REPAIR_PROTOCOL.md` lists "issue ID" as its first required
/// field. The field is named `issue_id` here so that the Rust name and the wire
/// name are the same word, rather than being related by a `serde` attribute that
/// a reader has to go looking for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairContract {
    /// Identity of the contract.
    pub id: RepairId,
    /// The finding this contract addresses.
    pub issue_id: crate::ids::FindingId,
    /// What is wrong.
    pub problem: String,
    /// Why it matters to the person using the software.
    pub why_it_matters: String,
    /// The concrete fix required.
    pub required_fix: Vec<String>,
    /// Behaviour that must keep working after the fix.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preserve: Vec<String>,
    /// How SURE will know the fix worked.
    pub acceptance: Vec<String>,
    /// Checks to re-run after the repair.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recheck: Vec<CheckId>,
    /// Cheats that would satisfy the letter of the acceptance criteria without
    /// fixing the problem.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbidden_shortcuts: Vec<String>,
    /// The evidence behind the problem statement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

impl RepairContract {
    /// Whether the contract gives a coding agent everything it needs.
    ///
    /// A contract without acceptance criteria cannot be verified, and one
    /// without a re-check list cannot be closed honestly.
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        !self.problem.trim().is_empty()
            && !self.why_it_matters.trim().is_empty()
            && !self.required_fix.is_empty()
            && !self.acceptance.is_empty()
            && !self.recheck.is_empty()
    }
}

/// The overall recommendation for a project.
///
/// A verdict is derived from results and findings. It is never a free-form
/// opinion, and it never claims more than the coverage supports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectVerdict {
    /// The project state this verdict is about.
    pub fingerprint: FingerprintId,
    /// How the checks aggregated.
    pub aggregate: Aggregate,
    /// Whether SURE may compare the project against what the user asked for.
    pub intent: ProjectIntent,
    /// What the harness integration could and could not see.
    pub capability: CapabilityReport,
    /// Findings, ordered most serious first.
    #[serde(default)]
    pub findings: Vec<Finding>,
    /// Checks that did not run, kept visible so they are never mistaken for passes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_checked: Vec<CheckResult>,
}

impl ProjectVerdict {
    /// Whether this verdict permits hand-off.
    ///
    /// True only when the aggregate is green **and** no open finding blocks
    /// hand-off. A green aggregate with an open `must_fix` finding is a
    /// contradiction that must never be presented as ready.
    #[must_use]
    pub fn is_ready_for_hand_off(&self) -> bool {
        self.aggregate.is_green()
            && !self
                .findings
                .iter()
                .any(|f| f.status.needs_attention() && f.severity.blocks_hand_off())
    }

    /// Whether the report must carry the "I did not see your request" caveat.
    #[must_use]
    pub fn must_caveat_requirements(&self) -> bool {
        self.intent.is_after_the_fact()
    }

    /// Findings that still need attention, most serious first.
    #[must_use]
    pub fn open_findings(&self) -> Vec<&Finding> {
        let mut open: Vec<&Finding> = self
            .findings
            .iter()
            .filter(|f| f.status.needs_attention())
            .collect();
        open.sort_by_key(|f| std::cmp::Reverse(f.severity));
        open
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::evidence::{AnchorSubject, EvidenceAnchor};

    fn anchor() -> EvidenceAnchor {
        EvidenceAnchor::new(AnchorSubject::File, "src/x.rs", "line 1")
    }

    fn fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    fn finding(severity: Severity, class: EvidenceClass) -> Finding {
        Finding {
            id: crate::ids::FindingId::generate(),
            title: "Payment looks successful but nothing is charged".to_owned(),
            severity,
            status: FindingStatus::Open,
            explanation: String::new(),
            user_impact: String::new(),
            next_step: String::new(),
            evidence: vec![Evidence::new(
                class,
                "s",
                anchor(),
                Some(fingerprint()),
                severity,
            )],
            fingerprint: fingerprint(),
            technical_details: None,
        }
    }

    #[test]
    fn finding_status_wire_names_match_the_schema() {
        for status in [
            FindingStatus::Open,
            FindingStatus::Resolved,
            FindingStatus::AcceptedRisk,
            FindingStatus::CannotConfirm,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            assert_eq!(json, format!("\"{}\"", status.as_str()));
        }
    }

    #[test]
    fn cannot_confirm_keeps_a_finding_open() {
        assert!(FindingStatus::CannotConfirm.needs_attention());
        assert!(FindingStatus::Open.needs_attention());
        assert!(!FindingStatus::Resolved.needs_attention());
        assert!(!FindingStatus::AcceptedRisk.needs_attention());
    }

    #[test]
    fn a_must_fix_finding_backed_only_by_model_opinion_is_not_grounded() {
        assert!(!finding(Severity::MustFix, EvidenceClass::ModelAssessment).is_grounded());
        assert!(!finding(Severity::MustFix, EvidenceClass::Inference).is_grounded());
        assert!(finding(Severity::MustFix, EvidenceClass::DeterministicCheck).is_grounded());
        assert!(finding(Severity::MustFix, EvidenceClass::ObservedFact).is_grounded());
    }

    #[test]
    fn strongest_evidence_class_picks_the_highest_truth_rank() {
        let mut f = finding(Severity::ShouldFixFirst, EvidenceClass::Inference);
        f.evidence.push(Evidence::new(
            EvidenceClass::ObservedFact,
            "s",
            anchor(),
            Some(fingerprint()),
            Severity::Note,
        ));
        assert_eq!(f.strongest_evidence_class(), EvidenceClass::ObservedFact);
    }

    #[test]
    fn support_levels_map_to_the_documented_letters() {
        assert_eq!(SupportLevel::FirstClass.letter(), 'A');
        assert_eq!(SupportLevel::Generic.letter(), 'B');
        assert_eq!(SupportLevel::InspectOnly.letter(), 'C');
        assert_eq!(SupportLevel::FirstClass.as_str(), "first_class");
        assert_eq!(SupportLevel::Generic.as_str(), "generic");
        assert_eq!(SupportLevel::InspectOnly.as_str(), "inspect_only");
    }

    #[test]
    fn fingerprints_only_match_when_kind_and_digest_agree() {
        let a = ProjectFingerprint::content("abc");
        let b = ProjectFingerprint::content("abc");
        let c = ProjectFingerprint::content("def");
        assert!(a.matches(&b));
        assert!(!a.matches(&c));
        assert!(!a.matches(&ProjectFingerprint::git(
            "abc",
            GitState {
                head: "deadbeef".to_owned(),
                dirty: false,
                dirty_digest: None,
                untracked_digest: None,
                branch: Some("main".to_owned()),
            }
        )));
    }

    #[test]
    fn an_empty_or_incomplete_plan_still_lists_its_checks() {
        let plan = CheckPlan::new("plan-1", fingerprint(), ExecutionMode::InspectOnly);
        assert!(plan.all_checks().is_empty());
        assert!(plan.excluded.is_empty());
    }

    #[test]
    fn a_repair_contract_without_acceptance_or_recheck_is_not_actionable() {
        let base = RepairContract {
            id: RepairId::generate(),
            issue_id: crate::ids::FindingId::generate(),
            problem: "p".to_owned(),
            why_it_matters: "w".to_owned(),
            required_fix: vec!["f".to_owned()],
            preserve: Vec::new(),
            acceptance: vec!["a".to_owned()],
            recheck: vec![CheckId::generate()],
            forbidden_shortcuts: Vec::new(),
            evidence: Vec::new(),
        };
        assert!(base.is_actionable());

        let no_acceptance = RepairContract {
            acceptance: Vec::new(),
            ..base.clone()
        };
        assert!(!no_acceptance.is_actionable());

        let no_recheck = RepairContract {
            recheck: Vec::new(),
            ..base.clone()
        };
        assert!(!no_recheck.is_actionable());

        let no_fix = RepairContract {
            required_fix: Vec::new(),
            ..base
        };
        assert!(!no_fix.is_actionable());
    }

    #[test]
    fn a_green_aggregate_with_an_open_must_fix_finding_is_not_ready() {
        use crate::status::{AggregateSeverity, CheckResult, CoverageSummary, StatusCounts};
        let aggregate = Aggregate {
            severity: AggregateSeverity::Green,
            headline: AggregateSeverity::Green.headline().to_owned(),
            counts: StatusCounts::tally([crate::status::CheckStatus::Pass]),
            blocking: Vec::new(),
            coverage: CoverageSummary {
                counts: StatusCounts::tally([crate::status::CheckStatus::Pass]),
                critical_not_checked: Vec::new(),
                critical_errored: Vec::new(),
                critical_failed: Vec::new(),
                critical_out_of_scope: Vec::new(),
                critical_checked: 1,
            },
        };
        let verdict = ProjectVerdict {
            fingerprint: fingerprint(),
            aggregate,
            intent: ProjectIntent::empty(),
            capability: CapabilityReport::cli(),
            findings: vec![finding(
                Severity::MustFix,
                EvidenceClass::DeterministicCheck,
            )],
            not_checked: vec![CheckResult::not_run(
                CheckId::generate(),
                "start the app",
                Severity::MustFix,
                true,
                NotCheckedReason::ExecutionNotAuthorized,
                fingerprint(),
            )],
        };
        assert!(!verdict.is_ready_for_hand_off());
        assert!(verdict.must_caveat_requirements());
        assert_eq!(verdict.open_findings().len(), 1);
        assert_eq!(verdict.not_checked.len(), 1);
    }

    #[test]
    fn open_findings_are_returned_most_serious_first_and_resolved_ones_are_dropped() {
        let mut resolved = finding(Severity::CanFixLater, EvidenceClass::ObservedFact);
        resolved.status = FindingStatus::Resolved;
        let verdict = ProjectVerdict {
            fingerprint: fingerprint(),
            aggregate: crate::status::aggregate(&[]),
            intent: ProjectIntent::empty(),
            capability: CapabilityReport::cli(),
            findings: vec![
                finding(Severity::Note, EvidenceClass::ObservedFact),
                finding(Severity::MustFix, EvidenceClass::ObservedFact),
                resolved,
            ],
            not_checked: Vec::new(),
        };
        let open = verdict.open_findings();
        assert_eq!(open.len(), 2);
        assert_eq!(open[0].severity, Severity::MustFix);
        assert_eq!(open[1].severity, Severity::Note);
    }

    #[test]
    fn a_claim_only_has_a_verdict_when_evidence_produced_one() {
        let make = |assessment| Claim {
            id: ClaimId::generate(),
            claim_text: "all tests pass".to_owned(),
            claim_type: "tests_pass".to_owned(),
            assessment,
            evidence: Vec::new(),
            session: None,
        };
        assert!(make(ClaimAssessment::Confirmed).has_verdict());
        assert!(make(ClaimAssessment::Contradicted).has_verdict());
        assert!(!make(ClaimAssessment::CannotConfirm).has_verdict());
        assert!(!make(ClaimAssessment::NotCheckable).has_verdict());
    }
}

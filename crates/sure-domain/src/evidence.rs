//! Evidence classes, anchors, freshness and claim assessment.
//!
//! The rule this module exists to enforce: agent text, model output and
//! inferred intent are never turned into deterministic fact. Every class of
//! evidence is labelled, every material claim needs an anchor, and
//! `cannot_confirm` is a first-class successful outcome.

use serde::{Deserialize, Serialize};

use crate::ids::{AnyId, EvidenceId, FingerprintId};
use crate::severity::Severity;
use crate::variants::variants;

/// How much weight a piece of evidence carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    /// Captured directly from the OS, a harness or the current project state.
    ObservedFact,
    /// A known command or tool produced a result bound to a project fingerprint.
    DeterministicCheck,
    /// Model interpretation grounded in specific anchors. May be wrong.
    ModelAssessment,
    /// Pattern-derived conclusion with uncertainty.
    Inference,
    /// Not enough evidence to say anything.
    Unknown,
}

variants!(
    /// Every class, strongest first.
    EvidenceClass { ObservedFact, DeterministicCheck, ModelAssessment, Inference, Unknown }
);

impl EvidenceClass {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ObservedFact => "observed_fact",
            Self::DeterministicCheck => "deterministic_check",
            Self::ModelAssessment => "model_assessment",
            Self::Inference => "inference",
            Self::Unknown => "unknown",
        }
    }

    /// Position in the truth hierarchy where a lower rank is stronger.
    ///
    /// `docs/architecture/EVIDENCE_MODEL.md` and `MASTER_PROMPT.md` order the
    /// sources; this is the machine-readable form of that order, restricted to
    /// what a single evidence record can carry.
    #[must_use]
    pub const fn truth_rank(self) -> u8 {
        match self {
            Self::ObservedFact => 0,
            Self::DeterministicCheck => 1,
            Self::ModelAssessment => 2,
            Self::Inference => 3,
            Self::Unknown => 4,
        }
    }

    /// Whether this class may be the sole support for a high-severity finding.
    ///
    /// A `must_fix` finding backed only by a model's opinion or a pattern guess
    /// would be an overclaim; those need a deterministic check or an observed
    /// fact behind them.
    #[must_use]
    pub const fn can_alone_support_must_fix(self) -> bool {
        matches!(self, Self::ObservedFact | Self::DeterministicCheck)
    }

    /// Whether the class is a statement about the world (rather than about SURE's knowledge).
    #[must_use]
    pub const fn is_assertive(self) -> bool {
        matches!(
            self,
            Self::ObservedFact | Self::DeterministicCheck | Self::ModelAssessment
        )
    }
}

/// A concrete, checkable pointer back to what was observed.
///
/// An evidence anchor without a location cannot be verified by the person
/// reading the report, so `location` and `locator` are required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAnchor {
    /// What kind of thing the subject is.
    pub subject: AnchorSubject,
    /// The identifier of the subject, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<AnyId>,
    /// Where a human finds this: a project-relative path, a command line, or a
    /// short description of the observation point.
    pub location: String,
    /// The specific thing at that location: a line range, a key, a field, a rule.
    pub locator: String,
    /// Quoted or transcribed content that demonstrates the point.
    ///
    /// Redaction is applied before this is written anywhere durable.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub excerpt: String,
}

impl EvidenceAnchor {
    /// Build an anchor with the required location and locator.
    #[must_use]
    pub fn new(
        subject: AnchorSubject,
        location: impl Into<String>,
        locator: impl Into<String>,
    ) -> Self {
        Self {
            subject,
            subject_id: None,
            location: location.into(),
            locator: locator.into(),
            excerpt: String::new(),
        }
    }

    /// Attach the identifier of the subject this anchor points at.
    #[must_use]
    pub fn with_subject_id(mut self, id: impl Into<AnyId>) -> Self {
        self.subject_id = Some(id.into());
        self
    }

    /// Attach demonstrating content.
    #[must_use]
    pub fn with_excerpt(mut self, excerpt: impl Into<String>) -> Self {
        self.excerpt = excerpt.into();
        self
    }

    /// Build an anchor pointing at a project-intent requirement or goal.
    #[must_use]
    pub fn intent(location: impl Into<String>, locator: impl Into<String>) -> Self {
        Self::new(AnchorSubject::Intent, location, locator)
    }

    /// Build an anchor pointing at an agent claim.
    #[must_use]
    pub fn claim(location: impl Into<String>, locator: impl Into<String>) -> Self {
        Self::new(AnchorSubject::Claim, location, locator)
    }

    /// Build a model-only anchor with no concrete project location.
    ///
    /// The `summary_or_label` is stored as the location so the anchor is still
    /// labelled and serializable; the locator is left empty so it is never
    /// considered checkable.
    #[must_use]
    pub fn model_only(summary_or_label: impl Into<String>) -> Self {
        Self {
            subject: AnchorSubject::Model,
            subject_id: None,
            location: summary_or_label.into(),
            locator: String::new(),
            excerpt: String::new(),
        }
    }

    /// Whether the anchor carries enough to be verified by a reader.
    #[must_use]
    pub fn is_checkable(&self) -> bool {
        match self.subject {
            AnchorSubject::Model => false,
            AnchorSubject::Intent | AnchorSubject::Claim => {
                !self.location.trim().is_empty() && !self.locator.trim().is_empty()
            }
            _ => !self.location.trim().is_empty() && !self.locator.trim().is_empty(),
        }
    }
}

/// What an evidence anchor points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorSubject {
    /// A file in the project.
    File,
    /// A directory in the project.
    Directory,
    /// A specific line or line range.
    LineRange,
    /// A command that was run.
    Command,
    /// Captured output.
    Output,
    /// A check result.
    Check,
    /// A recorded harness event.
    Event,
    /// Project configuration.
    Config,
    /// A database schema or migration.
    Database,
    /// A documented claim (README, spec, comment).
    Documentation,
    /// Git history or repository state.
    Git,
    /// A behaviour observed at runtime.
    Runtime,
    /// A project-intent requirement or goal.
    Intent,
    /// An agent claim.
    Claim,
    /// A model-only conclusion with no concrete project location.
    Model,
}

variants!(AnchorSubject {
    File,
    Directory,
    LineRange,
    Command,
    Output,
    Check,
    Event,
    Config,
    Database,
    Documentation,
    Git,
    Runtime,
    Intent,
    Claim,
    Model
});

impl AnchorSubject {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::LineRange => "line_range",
            Self::Command => "command",
            Self::Output => "output",
            Self::Check => "check",
            Self::Event => "event",
            Self::Config => "config",
            Self::Database => "database",
            Self::Documentation => "documentation",
            Self::Git => "git",
            Self::Runtime => "runtime",
            Self::Intent => "intent",
            Self::Claim => "claim",
            Self::Model => "model",
        }
    }
}

/// One captured piece of evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// Stable identity, so findings and repair contracts can point at it.
    pub id: EvidenceId,
    /// How much weight this evidence carries.
    pub class: EvidenceClass,
    /// One line describing what this is.
    pub summary: String,
    /// Where it came from.
    pub anchor: EvidenceAnchor,
    /// Which project state this evidence applies to.
    ///
    /// `None` means unknown provenance; a result bound to an unknown
    /// fingerprint can never be fresh.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<FingerprintId>,
    /// Severity this evidence would justify if it stands alone.
    pub severity: Severity,
}

impl Evidence {
    /// Build evidence anchored to a specific project state.
    #[must_use]
    pub fn new(
        class: EvidenceClass,
        summary: impl Into<String>,
        anchor: EvidenceAnchor,
        fingerprint: Option<FingerprintId>,
        severity: Severity,
    ) -> Self {
        Self {
            id: EvidenceId::generate(),
            class,
            summary: summary.into(),
            anchor,
            fingerprint,
            severity,
        }
    }

    /// Whether this evidence can still speak for the given project state.
    ///
    /// Evidence tied to an older fingerprint is stale and cannot prove anything
    /// about the current code. Evidence with no fingerprint is never fresh: SURE
    /// does not know what it applies to.
    #[must_use]
    pub fn is_fresh_for(&self, current: &FingerprintId) -> bool {
        self.fingerprint.as_ref() == Some(current)
    }
}

/// The outcome of checking one agent claim.
///
/// `CannotConfirm` is deliberately not `Contradicted`: "I could not check this"
/// and "this is wrong" are different statements and must never be merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimAssessment {
    /// Evidence supports the claim.
    Confirmed,
    /// Evidence contradicts the claim.
    Contradicted,
    /// Not enough evidence either way.
    CannotConfirm,
    /// The claim is not the kind of thing evidence can settle.
    NotCheckable,
}

variants!(
    /// Every assessment.
    ClaimAssessment { Confirmed, Contradicted, CannotConfirm, NotCheckable }
);

impl ClaimAssessment {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Contradicted => "contradicted",
            Self::CannotConfirm => "cannot_confirm",
            Self::NotCheckable => "not_checkable",
        }
    }

    /// The exact user-facing wording from the product language rules.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Confirmed => "Confirmed",
            Self::Contradicted => "Contradicted",
            Self::CannotConfirm => "Cannot confirm",
            Self::NotCheckable => "Not checked",
        }
    }

    /// Whether the claim is proven true.
    #[must_use]
    pub const fn is_positive(self) -> bool {
        matches!(self, Self::Confirmed)
    }

    /// Whether SURE has no verdict, as opposed to a verdict of "wrong".
    #[must_use]
    pub const fn is_absence_of_evidence(self) -> bool {
        matches!(self, Self::CannotConfirm | Self::NotCheckable)
    }
}

/// Why a piece of evidence can no longer support a conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StalenessReason {
    /// The evidence was captured against a different project state.
    FingerprintChanged,
    /// The project state moved on after the evidence was captured.
    SupersededByLaterChange,
    /// The evidence carries no fingerprint, so it applies to nothing in particular.
    UnknownProvenance,
}

variants!(StalenessReason {
    FingerprintChanged,
    SupersededByLaterChange,
    UnknownProvenance
});

impl StalenessReason {
    /// Plain-language explanation for the report.
    #[must_use]
    pub const fn plain_explanation(self) -> &'static str {
        match self {
            Self::FingerprintChanged => {
                "These tests passed before the latest code changes, so I cannot use them to confirm the final version."
            }
            Self::SupersededByLaterChange => {
                "This was checked before later changes were made, so it no longer describes the current version."
            }
            Self::UnknownProvenance => {
                "I do not know which version of the code this applies to, so I cannot use it as proof."
            }
        }
    }
}

/// The freshness of evidence relative to a project state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// The evidence applies to exactly the current state.
    Fresh,
    /// The evidence applies to a state that is no longer current.
    Stale(StalenessReason),
}

impl Freshness {
    /// Whether the evidence can be used to support a conclusion right now.
    #[must_use]
    pub const fn is_usable(self) -> bool {
        matches!(self, Self::Fresh)
    }

    /// The caveat the report must show, if any.
    #[must_use]
    pub const fn caveat(self) -> Option<&'static str> {
        match self {
            Self::Fresh => None,
            Self::Stale(reason) => Some(reason.plain_explanation()),
        }
    }
}

/// Classify evidence freshness against the current project state.
#[must_use]
pub fn freshness(evidence: &Evidence, current: &FingerprintId) -> Freshness {
    match &evidence.fingerprint {
        None => Freshness::Stale(StalenessReason::UnknownProvenance),
        Some(fp) if fp == current => Freshness::Fresh,
        Some(_) => Freshness::Stale(StalenessReason::FingerprintChanged),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn anchor() -> EvidenceAnchor {
        EvidenceAnchor::new(AnchorSubject::File, "src/pay.ts", "lines 40-52")
    }

    #[test]
    fn wire_names_match_the_schemas() {
        for &class in EvidenceClass::ALL {
            let json = serde_json::to_string(&class).expect("serialize");
            assert_eq!(json, format!("\"{}\"", class.as_str()));
        }
        for &assessment in ClaimAssessment::ALL {
            let json = serde_json::to_string(&assessment).expect("serialize");
            assert_eq!(json, format!("\"{}\"", assessment.as_str()));
        }
        for &subject in AnchorSubject::ALL {
            let json = serde_json::to_string(&subject).expect("serialize");
            assert_eq!(json, format!("\"{}\"", subject.as_str()));
        }
    }

    #[test]
    fn anchor_subject_wire_names_are_frozen() {
        assert_eq!(AnchorSubject::File.as_str(), "file");
        assert_eq!(AnchorSubject::Directory.as_str(), "directory");
        assert_eq!(AnchorSubject::LineRange.as_str(), "line_range");
        assert_eq!(AnchorSubject::Command.as_str(), "command");
        assert_eq!(AnchorSubject::Output.as_str(), "output");
        assert_eq!(AnchorSubject::Check.as_str(), "check");
        assert_eq!(AnchorSubject::Event.as_str(), "event");
        assert_eq!(AnchorSubject::Config.as_str(), "config");
        assert_eq!(AnchorSubject::Database.as_str(), "database");
        assert_eq!(AnchorSubject::Documentation.as_str(), "documentation");
        assert_eq!(AnchorSubject::Git.as_str(), "git");
        assert_eq!(AnchorSubject::Runtime.as_str(), "runtime");
        assert_eq!(AnchorSubject::Intent.as_str(), "intent");
        assert_eq!(AnchorSubject::Claim.as_str(), "claim");
        assert_eq!(AnchorSubject::Model.as_str(), "model");
    }

    #[test]
    fn cannot_confirm_is_not_contradicted_and_neither_is_confirmed() {
        assert_ne!(
            ClaimAssessment::CannotConfirm,
            ClaimAssessment::Contradicted
        );
        assert!(!ClaimAssessment::CannotConfirm.is_positive());
        assert!(!ClaimAssessment::Contradicted.is_positive());
        assert!(ClaimAssessment::CannotConfirm.is_absence_of_evidence());
        assert!(!ClaimAssessment::Contradicted.is_absence_of_evidence());
    }

    #[test]
    fn evidence_labels_use_the_frozen_product_wording() {
        assert_eq!(ClaimAssessment::Confirmed.label(), "Confirmed");
        assert_eq!(ClaimAssessment::Contradicted.label(), "Contradicted");
        assert_eq!(ClaimAssessment::CannotConfirm.label(), "Cannot confirm");
        assert_eq!(ClaimAssessment::NotCheckable.label(), "Not checked");
    }

    #[test]
    fn truth_rank_is_strictly_ordered() {
        let mut ranks: Vec<u8> = EvidenceClass::ALL.iter().map(|c| c.truth_rank()).collect();
        ranks.sort_unstable();
        ranks.dedup();
        assert_eq!(ranks.len(), EvidenceClass::ALL.len());
    }

    #[test]
    fn a_model_opinion_alone_cannot_justify_a_must_fix_finding() {
        assert!(!EvidenceClass::ModelAssessment.can_alone_support_must_fix());
        assert!(!EvidenceClass::Inference.can_alone_support_must_fix());
        assert!(!EvidenceClass::Unknown.can_alone_support_must_fix());
        assert!(EvidenceClass::ObservedFact.can_alone_support_must_fix());
        assert!(EvidenceClass::DeterministicCheck.can_alone_support_must_fix());
    }

    #[test]
    fn concrete_anchors_remain_checkable_with_location_and_locator() {
        assert!(EvidenceAnchor::new(AnchorSubject::File, "src/x.rs", "line 1").is_checkable());
        assert!(
            EvidenceAnchor::new(AnchorSubject::LineRange, "src/x.rs", "lines 10-20").is_checkable()
        );
        assert!(
            EvidenceAnchor::new(AnchorSubject::Check, "cargo test", "suite payment").is_checkable()
        );
        assert!(
            EvidenceAnchor::new(AnchorSubject::Runtime, "local dev run", "stdout trace")
                .is_checkable()
        );
    }

    #[test]
    fn an_anchor_without_a_location_is_not_checkable() {
        assert!(anchor().is_checkable());
        assert!(
            !EvidenceAnchor::new(AnchorSubject::File, "  ", "lines 1-2").is_checkable(),
            "blank location must not count as checkable"
        );
        assert!(!EvidenceAnchor::new(AnchorSubject::File, "a.rs", "").is_checkable());
    }

    #[test]
    fn intent_and_claim_anchors_are_checkable_only_with_location_and_locator() {
        let intent = EvidenceAnchor::intent("project-intent.json", "requirement P-7");
        assert!(intent.is_checkable());
        assert!(
            !EvidenceAnchor::intent("  ", "requirement P-7").is_checkable(),
            "blank intent location must not be checkable"
        );
        assert!(!EvidenceAnchor::intent("project-intent.json", "  ").is_checkable());

        let claim = EvidenceAnchor::claim("agent-transcript.md", "claim #42");
        assert!(claim.is_checkable());
        assert!(!EvidenceAnchor::claim("", "claim #42").is_checkable());
        assert!(!EvidenceAnchor::claim("agent-transcript.md", "").is_checkable());
    }

    #[test]
    fn model_only_anchor_is_never_checkable_and_is_clearly_labelled() {
        let model = EvidenceAnchor::model_only("model inference: no concrete anchor");
        assert_eq!(model.subject, AnchorSubject::Model);
        assert!(
            !model.is_checkable(),
            "model-only anchor must never be checkable"
        );
        assert!(
            !model.location.is_empty(),
            "model-only anchor must still carry a human-readable label"
        );
        assert!(model.locator.is_empty());
    }

    #[test]
    fn new_anchor_subjects_round_trip_through_serialization() {
        for subject in [
            AnchorSubject::Intent,
            AnchorSubject::Claim,
            AnchorSubject::Model,
        ] {
            let original = EvidenceAnchor::new(subject, "loc", "ptr");
            let json = serde_json::to_string(&original).expect("serialize");
            let back: EvidenceAnchor = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, original);
            assert_eq!(back.subject.as_str(), subject.as_str());
        }

        let model = EvidenceAnchor::model_only("summary");
        let json = serde_json::to_string(&model).expect("serialize model-only anchor");
        let back: EvidenceAnchor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, model);
        assert_eq!(back.subject, AnchorSubject::Model);
    }

    #[test]
    fn evidence_bound_to_the_current_fingerprint_is_fresh() {
        let current = FingerprintId::generate();
        let evidence = Evidence::new(
            EvidenceClass::DeterministicCheck,
            "cargo test passed",
            anchor(),
            Some(current.clone()),
            Severity::Note,
        );
        assert_eq!(freshness(&evidence, &current), Freshness::Fresh);
        assert!(freshness(&evidence, &current).is_usable());
        assert!(freshness(&evidence, &current).caveat().is_none());
    }

    #[test]
    fn evidence_from_older_code_is_stale_and_says_so_in_plain_language() {
        let evidence = Evidence::new(
            EvidenceClass::DeterministicCheck,
            "tests passed",
            anchor(),
            Some(FingerprintId::generate()),
            Severity::Note,
        );
        let current = FingerprintId::generate();
        let freshness = freshness(&evidence, &current);
        assert_eq!(
            freshness,
            Freshness::Stale(StalenessReason::FingerprintChanged)
        );
        assert!(!freshness.is_usable());
        let caveat = freshness.caveat().expect("stale evidence has a caveat");
        assert!(caveat.contains("cannot use them to confirm"), "{caveat}");
    }

    #[test]
    fn evidence_without_a_fingerprint_is_never_fresh() {
        let evidence = Evidence::new(
            EvidenceClass::ObservedFact,
            "looked at a file",
            anchor(),
            None,
            Severity::Note,
        );
        assert!(!freshness(&evidence, &FingerprintId::generate()).is_usable());
    }

    #[test]
    fn stale_caveats_avoid_leading_jargon() {
        for reason in [
            StalenessReason::FingerprintChanged,
            StalenessReason::SupersededByLaterChange,
            StalenessReason::UnknownProvenance,
        ] {
            let text = reason.plain_explanation().to_lowercase();
            for banned in [
                "sha",
                "provenance",
                "attestation",
                "fingerprint",
                "schema drift",
            ] {
                assert!(!text.contains(banned), "{reason:?} uses jargon '{banned}'");
            }
        }
    }
}

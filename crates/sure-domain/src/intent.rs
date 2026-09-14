//! Project intent and requirement trust.
//!
//! This is the critical truth boundary. A project snapshot does not reveal what
//! the user originally asked for, so SURE records where every requirement came
//! from and refuses to promote an inference into a user requirement.

use serde::{Deserialize, Serialize};

use crate::ids::EvidenceId;
use crate::status::RequirementClaim;
use crate::variants::variants;

/// Where a stated requirement or goal came from.
///
/// The wire names are frozen by `schemas/project-intent.schema.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    /// The user typed a goal directly to SURE.
    ExplicitUserGoal,
    /// A harness exposed the user's request and the privacy mode permits keeping it.
    ObservedUserRequest,
    /// A README, specification or task file found in the project.
    ///
    /// This is documentation. It is evidence of what someone wrote down, not
    /// proof of what the user currently wants.
    ProjectSpec,
    /// A coding agent said something is complete.
    ///
    /// Useful for claim checking. Never proof that it was a requirement, and
    /// never proof that it was done.
    AgentClaim,
    /// SURE guessed the project's purpose.
    ///
    /// Never a user requirement.
    Inferred,
}

variants!(
    /// Every source, most authoritative first.
    IntentSource { ExplicitUserGoal, ObservedUserRequest, ProjectSpec, AgentClaim, Inferred }
);

impl IntentSource {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitUserGoal => "explicit_user_goal",
            Self::ObservedUserRequest => "observed_user_request",
            Self::ProjectSpec => "project_spec",
            Self::AgentClaim => "agent_claim",
            Self::Inferred => "inferred",
        }
    }

    /// Position in the trust ordering, where a lower rank is more trustworthy.
    #[must_use]
    pub const fn trust_rank(self) -> u8 {
        match self {
            Self::ExplicitUserGoal => 0,
            Self::ObservedUserRequest => 1,
            Self::ProjectSpec => 2,
            Self::AgentClaim => 3,
            Self::Inferred => 4,
        }
    }

    /// Whether this source can stand in for the user's actual goal.
    ///
    /// Only a goal the user supplied, or a request SURE was permitted to
    /// observe, can be compared against the implementation. A README is somebody's
    /// documentation, an agent claim is a claim, and an inference is a guess.
    #[must_use]
    pub const fn is_user_requirement(self) -> bool {
        matches!(self, Self::ExplicitUserGoal | Self::ObservedUserRequest)
    }

    /// Whether this source describes work the user asked for as opposed to
    /// something merely written down or asserted.
    #[must_use]
    pub const fn is_documentation(self) -> bool {
        matches!(self, Self::ProjectSpec)
    }

    /// Plain-language description of what this source is worth, for the report.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::ExplicitUserGoal => "You told SURE what you wanted.",
            Self::ObservedUserRequest => {
                "SURE saw the request you gave your coding tool and recording was allowed."
            }
            Self::ProjectSpec => {
                "This comes from written documentation in the project, which may be out of date."
            }
            Self::AgentClaim => "This is something the coding agent said, not proof of anything.",
            Self::Inferred => {
                "SURE guessed this from the code, so it is not something you asked for."
            }
        }
    }

    /// Whether retaining the raw text of this source requires opt-in.
    #[must_use]
    pub const fn requires_full_recording(self) -> bool {
        matches!(self, Self::ObservedUserRequest)
    }
}

/// How well established a requirement is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementAuthority {
    /// The user said it. SURE may compare the project against it.
    UserRequirement,
    /// Documentation says it. SURE may check whether the documentation is true.
    DocumentedInstruction,
    /// An agent asserted it. SURE may try to check the assertion, and no more.
    AgentAssertion,
    /// SURE guessed. SURE may not report on it as a requirement at all.
    NotARequirement,
}

variants!(RequirementAuthority {
    UserRequirement,
    DocumentedInstruction,
    AgentAssertion,
    NotARequirement
});

impl RequirementAuthority {
    /// Derive the authority of a statement from its source.
    #[must_use]
    pub const fn from_source(source: IntentSource) -> Self {
        match source {
            IntentSource::ExplicitUserGoal | IntentSource::ObservedUserRequest => {
                Self::UserRequirement
            }
            IntentSource::ProjectSpec => Self::DocumentedInstruction,
            IntentSource::AgentClaim => Self::AgentAssertion,
            IntentSource::Inferred => Self::NotARequirement,
        }
    }
}

/// One requirement, goal or documented statement, with its provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    /// Stable identity within the run.
    pub id: String,
    /// The statement, normalized to one sentence where possible.
    pub text: String,
    /// Where the statement came from.
    pub source: IntentSource,
    /// Whether the raw text was retained, or only a normalized summary.
    pub raw_retained: bool,
    /// Evidence supporting that this statement exists.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceId>,
}

impl Requirement {
    /// Build a requirement from a source.
    #[must_use]
    pub fn new(id: impl Into<String>, text: impl Into<String>, source: IntentSource) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            source,
            raw_retained: false,
            evidence: Vec::new(),
        }
    }

    /// Record whether the raw wording was kept.
    #[must_use]
    pub fn with_raw_retained(mut self, retained: bool) -> Self {
        self.raw_retained = retained;
        self
    }

    /// How much authority this statement carries.
    #[must_use]
    pub const fn authority(&self) -> RequirementAuthority {
        RequirementAuthority::from_source(self.source)
    }

    /// Whether SURE may treat this as something the user asked for.
    ///
    /// An inference is never a user requirement, no matter how confident the
    /// pattern behind it looks.
    #[must_use]
    pub const fn is_user_requirement(&self) -> bool {
        self.source.is_user_requirement()
    }
}

/// Everything SURE knows about what the project was supposed to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectIntent {
    /// The requirements and statements, in discovery order.
    pub requirements: Vec<Requirement>,
}

impl ProjectIntent {
    /// An intent record containing no trusted requirement source.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            requirements: Vec::new(),
        }
    }

    /// Build an intent record from statements.
    #[must_use]
    pub fn from_requirements(requirements: Vec<Requirement>) -> Self {
        Self { requirements }
    }

    /// Whether any statement came from the user or from an observed user request.
    #[must_use]
    pub fn has_user_requirement(&self) -> bool {
        self.requirements
            .iter()
            .any(Requirement::is_user_requirement)
    }

    /// Whether the user supplied their goal directly to SURE.
    #[must_use]
    pub fn has_explicit_user_goal(&self) -> bool {
        self.requirements
            .iter()
            .any(|r| r.source == IntentSource::ExplicitUserGoal)
    }

    /// Statements the user actually asked for.
    pub fn user_requirements(&self) -> impl Iterator<Item = &Requirement> {
        self.requirements.iter().filter(|r| r.is_user_requirement())
    }

    /// Statements agents made that SURE may attempt to check.
    pub fn agent_claims(&self) -> impl Iterator<Item = &Requirement> {
        self.requirements
            .iter()
            .filter(|r| r.source == IntentSource::AgentClaim)
    }

    /// Whether SURE may compare the project against the user's request.
    ///
    /// When this is `AfterTheFact`, the report must carry
    /// [`crate::status::NO_TRUSTED_INTENT_LIMITATION`] and must not claim that
    /// everything the user asked for is complete.
    #[must_use]
    pub fn requirement_claim(&self) -> RequirementClaim {
        if self.has_user_requirement() {
            RequirementClaim::Comparable
        } else {
            RequirementClaim::AfterTheFact
        }
    }

    /// Whether SURE is limited to judging the current project state.
    #[must_use]
    pub fn is_after_the_fact(&self) -> bool {
        self.requirement_claim() == RequirementClaim::AfterTheFact
    }

    /// The caveat the report must include, if the mode is after-the-fact.
    #[must_use]
    pub fn caveat(&self) -> Option<&'static str> {
        self.requirement_claim().caveat()
    }
}

impl Default for ProjectIntent {
    fn default() -> Self {
        Self::empty()
    }
}

/// Whether SURE may state that "everything the user asked for is complete".
///
/// This is the single decision point for that sentence. It is false unless a
/// trusted intent source exists **and** every user requirement has supporting
/// evidence that is still fresh.
#[must_use]
pub fn may_claim_full_fulfilment(
    intent: &ProjectIntent,
    requirements_with_fresh_evidence: usize,
) -> bool {
    intent.has_user_requirement()
        && requirements_with_fresh_evidence >= intent.user_requirements().count()
        && intent.user_requirements().count() > 0
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_match_the_project_intent_schema() {
        for &source in IntentSource::ALL {
            let json = serde_json::to_string(&source).expect("serialize");
            assert_eq!(json, format!("\"{}\"", source.as_str()));
        }
    }

    #[test]
    fn trust_rank_is_strictly_ordered_and_matches_discovery_order() {
        for pair in IntentSource::ALL.windows(2) {
            assert!(pair[0].trust_rank() < pair[1].trust_rank());
        }
    }

    #[test]
    fn only_explicit_and_observed_sources_are_user_requirements() {
        for &source in IntentSource::ALL {
            let expected = matches!(
                source,
                IntentSource::ExplicitUserGoal | IntentSource::ObservedUserRequest
            );
            assert_eq!(source.is_user_requirement(), expected, "{source:?}");
        }
    }

    #[test]
    fn an_inference_can_never_become_a_user_requirement() {
        let requirement =
            Requirement::new("r1", "this is probably a todo app", IntentSource::Inferred);
        assert!(!requirement.is_user_requirement());
        assert_eq!(
            requirement.authority(),
            RequirementAuthority::NotARequirement
        );
        let intent = ProjectIntent::from_requirements(vec![requirement]);
        assert!(intent.is_after_the_fact());
        assert!(!may_claim_full_fulfilment(&intent, 1));
    }

    #[test]
    fn an_agent_claim_is_not_a_requirement() {
        let requirement = Requirement::new("r1", "login is finished", IntentSource::AgentClaim);
        assert!(!requirement.is_user_requirement());
        assert_eq!(
            requirement.authority(),
            RequirementAuthority::AgentAssertion
        );
        let intent = ProjectIntent::from_requirements(vec![requirement]);
        assert!(intent.is_after_the_fact());
        assert_eq!(intent.agent_claims().count(), 1);
    }

    #[test]
    fn a_readme_is_documentation_not_a_user_requirement() {
        let requirement = Requirement::new("r1", "run npm start", IntentSource::ProjectSpec);
        assert!(!requirement.is_user_requirement());
        assert_eq!(
            requirement.authority(),
            RequirementAuthority::DocumentedInstruction
        );
        assert!(ProjectIntent::from_requirements(vec![requirement]).is_after_the_fact());
    }

    #[test]
    fn after_the_fact_mode_carries_the_frozen_limitation_sentence() {
        let intent = ProjectIntent::empty();
        assert!(intent.is_after_the_fact());
        let caveat = intent.caveat().expect("after-the-fact mode has a caveat");
        assert_eq!(caveat, crate::status::NO_TRUSTED_INTENT_LIMITATION);
        assert!(intent.user_requirements().next().is_none());
    }

    #[test]
    fn full_fulfilment_requires_a_user_requirement_and_fresh_evidence_for_all_of_them() {
        let intent = ProjectIntent::from_requirements(vec![
            Requirement::new("r1", "a", IntentSource::ExplicitUserGoal),
            Requirement::new("r2", "b", IntentSource::ExplicitUserGoal),
        ]);
        assert!(intent.has_user_requirement());
        assert!(!may_claim_full_fulfilment(&intent, 0));
        assert!(!may_claim_full_fulfilment(&intent, 1));
        assert!(may_claim_full_fulfilment(&intent, 2));
    }

    #[test]
    fn an_empty_intent_record_can_never_claim_fulfilment() {
        assert!(!may_claim_full_fulfilment(&ProjectIntent::empty(), 0));
    }

    #[test]
    fn only_observed_user_requests_need_full_recording() {
        for &source in IntentSource::ALL {
            let expected = source == IntentSource::ObservedUserRequest;
            assert_eq!(source.requires_full_recording(), expected, "{source:?}");
        }
    }

    #[test]
    fn intent_round_trips_through_json() {
        let intent = ProjectIntent::from_requirements(vec![Requirement::new(
            "r1",
            "build a todo app",
            IntentSource::ExplicitUserGoal,
        )]);
        let json = serde_json::to_string(&intent).expect("serialize");
        let back: ProjectIntent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, intent);
    }
}

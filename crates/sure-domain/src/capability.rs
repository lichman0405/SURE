//! Harness capability tiers.
//!
//! Integrations differ enormously in what they can see. SURE reports the tier it
//! actually achieved, and what it could not see, so that a blind spot is never
//! mistaken for a clean result.

use serde::{Deserialize, Serialize};

/// How much of a coding session an integration can observe or influence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityTier {
    /// Project snapshot only. SURE sees the files on disk and nothing about the session.
    Snapshot,
    /// Observed session. SURE sees what the agent did, after the fact.
    Observed,
    /// Protected session. SURE can decide before an action happens.
    Protected,
}

impl CapabilityTier {
    /// Every tier, weakest first.
    pub const ALL: [Self; 3] = [Self::Snapshot, Self::Observed, Self::Protected];

    /// The numeric tier, as used in the event envelope's `capability_tier`.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Snapshot => 0,
            Self::Observed => 1,
            Self::Protected => 2,
        }
    }

    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Observed => "observed",
            Self::Protected => "protected",
        }
    }

    /// Resolve a numeric tier from an event envelope.
    #[must_use]
    pub const fn from_number(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Snapshot),
            1 => Some(Self::Observed),
            2 => Some(Self::Protected),
            _ => None,
        }
    }

    /// Plain-language summary of what this tier means for the user.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Snapshot => {
                "SURE can look at the project as it is now. It cannot see what the AI did while it worked."
            }
            Self::Observed => {
                "SURE can also see what the AI did during the session, after the fact."
            }
            Self::Protected => "SURE can also step in before the AI runs something dangerous.",
        }
    }
}

/// A specific thing SURE could not see at the achieved tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlindSpot {
    /// Short identifier for the gap.
    pub kind: BlindSpotKind,
    /// Plain-language explanation for the user.
    pub explanation: String,
}

/// The kinds of session information an integration may fail to expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlindSpotKind {
    /// The user's original request was never exposed.
    UserGoalNotExposed,
    /// The agent's completion statement was never exposed.
    CompletionClaimNotExposed,
    /// Shell and tool invocations were not exposed.
    ToolCallsNotExposed,
    /// Commands that were started but never finished were not visible.
    FailuresNotExposed,
    /// File edits were not exposed.
    FileEditsNotExposed,
    /// Git activity was not exposed.
    GitActivityNotExposed,
    /// No pre-action hook exists, so dangerous actions cannot be intercepted.
    NoPreActionControl,
    /// The harness exposes nothing beyond the project snapshot.
    NoSessionVisibility,
}

impl BlindSpotKind {
    /// Plain-language explanation, used when a caller has nothing more specific.
    #[must_use]
    pub const fn plain_explanation(self) -> &'static str {
        match self {
            Self::UserGoalNotExposed => {
                "SURE never saw what you asked the AI to build, so it cannot tell whether the AI built it."
            }
            Self::CompletionClaimNotExposed => {
                "SURE never saw the AI say the work was finished, so it cannot check that claim."
            }
            Self::ToolCallsNotExposed => {
                "SURE could not see the commands the AI ran, so it cannot confirm what was actually executed."
            }
            Self::FailuresNotExposed => {
                "SURE could not see which commands failed while the AI worked."
            }
            Self::FileEditsNotExposed => {
                "SURE could not see which files the AI changed during the session."
            }
            Self::GitActivityNotExposed => "SURE could not see the Git activity from the session.",
            Self::NoPreActionControl => {
                "This tool cannot ask before the AI runs something dangerous, so SURE can only report afterwards."
            }
            Self::NoSessionVisibility => {
                "This tool does not tell SURE anything about the session, so SURE only sees the project as it is now."
            }
        }
    }
}

/// What an integration can actually do, reported by the adapter itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    /// The integration this describes, for example `claude-code` or `cli`.
    pub adapter: String,
    /// The tier actually achieved, not the tier intended.
    pub tier: CapabilityTier,
    /// What the integration could not expose.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blind_spots: Vec<BlindSpot>,
    /// Whether the adapter can pause an action for a decision.
    pub pre_action_control: bool,
    /// Whether a hook failure leaves the harness running (`fail_open`) or stops it (`fail_closed`).
    pub hook_failure: HookFailureBehaviour,
}

impl CapabilityReport {
    /// A report for the plain CLI, which has no session visibility at all.
    #[must_use]
    pub fn cli() -> Self {
        Self {
            adapter: "cli".to_owned(),
            tier: CapabilityTier::Snapshot,
            blind_spots: vec![
                BlindSpot {
                    kind: BlindSpotKind::NoSessionVisibility,
                    explanation: BlindSpotKind::NoSessionVisibility
                        .plain_explanation()
                        .to_owned(),
                },
                BlindSpot {
                    kind: BlindSpotKind::NoPreActionControl,
                    explanation: BlindSpotKind::NoPreActionControl
                        .plain_explanation()
                        .to_owned(),
                },
            ],
            pre_action_control: false,
            hook_failure: HookFailureBehaviour::NotApplicable,
        }
    }

    /// Whether the claimed tier is consistent with the rest of the report.
    ///
    /// A tier is an achievement, so a report that claims pre-action control
    /// without the protected tier, or the protected tier without pre-action
    /// control, is internally contradictory and must be rejected rather than
    /// reported to the user.
    #[must_use]
    pub fn is_self_consistent(&self) -> bool {
        self.pre_action_control == (self.tier == CapabilityTier::Protected)
    }

    /// The tier number, for the event envelope.
    #[must_use]
    pub const fn tier_number(&self) -> u8 {
        self.tier.number()
    }

    /// Plain-language description of what the user actually gets.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{} (capability tier {}, {})",
            self.tier.plain_description(),
            self.tier.number(),
            self.tier.as_str()
        )
    }

    /// Whether SURE can honestly claim to have seen the user's request.
    #[must_use]
    pub fn saw_user_request(&self) -> bool {
        !self
            .blind_spots
            .iter()
            .any(|spot| spot.kind == BlindSpotKind::UserGoalNotExposed)
            && self.tier >= CapabilityTier::Observed
    }
}

/// What happens to the harness when a SURE hook fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookFailureBehaviour {
    /// The harness continues without SURE.
    FailOpen,
    /// The action is refused when SURE cannot make a decision.
    FailClosed,
    /// The integration has no hook that can fail.
    NotApplicable,
}

impl HookFailureBehaviour {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FailOpen => "fail_open",
            Self::FailClosed => "fail_closed",
            Self::NotApplicable => "not_applicable",
        }
    }

    /// Plain-language explanation of the consequence.
    #[must_use]
    pub const fn plain_explanation(self) -> &'static str {
        match self {
            Self::FailOpen => "If SURE stops working, your AI tool keeps going without its checks.",
            Self::FailClosed => {
                "If SURE stops working, your AI tool stops too, rather than continuing unchecked."
            }
            Self::NotApplicable => "This integration has no check that can fail this way.",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn tiers_map_to_the_documented_numbers() {
        assert_eq!(CapabilityTier::Snapshot.number(), 0);
        assert_eq!(CapabilityTier::Observed.number(), 1);
        assert_eq!(CapabilityTier::Protected.number(), 2);
        for tier in CapabilityTier::ALL {
            assert_eq!(CapabilityTier::from_number(tier.number()), Some(tier));
        }
        assert_eq!(CapabilityTier::from_number(3), None);
    }

    #[test]
    fn tiers_are_ordered_by_capability() {
        assert!(CapabilityTier::Snapshot < CapabilityTier::Observed);
        assert!(CapabilityTier::Observed < CapabilityTier::Protected);
    }

    #[test]
    fn tier_wire_names_match_the_documented_meanings() {
        assert_eq!(CapabilityTier::Snapshot.as_str(), "snapshot");
        assert_eq!(CapabilityTier::Observed.as_str(), "observed");
        assert_eq!(CapabilityTier::Protected.as_str(), "protected");
    }

    #[test]
    fn the_cli_reports_tier_zero_honestly() {
        let report = CapabilityReport::cli();
        assert_eq!(report.tier, CapabilityTier::Snapshot);
        assert_eq!(report.tier_number(), 0);
        assert!(report.is_self_consistent());
        assert!(!report.pre_action_control);
        assert!(!report.saw_user_request());
        assert!(
            report
                .blind_spots
                .iter()
                .any(|s| s.kind == BlindSpotKind::NoSessionVisibility)
        );
    }

    #[test]
    fn a_contradictory_capability_report_is_rejected() {
        let mut report = CapabilityReport::cli();
        report.pre_action_control = true;
        assert!(!report.is_self_consistent());

        let mut claimed = CapabilityReport::cli();
        claimed.tier = CapabilityTier::Protected;
        assert!(!claimed.is_self_consistent());
    }

    #[test]
    fn a_protected_adapter_must_claim_pre_action_control() {
        let report = CapabilityReport {
            adapter: "claude-code".to_owned(),
            tier: CapabilityTier::Protected,
            blind_spots: Vec::new(),
            pre_action_control: true,
            hook_failure: HookFailureBehaviour::FailOpen,
        };
        assert!(report.is_self_consistent());
        assert!(report.saw_user_request());
    }

    #[test]
    fn seeing_the_user_request_requires_more_than_a_snapshot() {
        let mut observed = CapabilityReport::cli();
        observed.tier = CapabilityTier::Observed;
        observed
            .blind_spots
            .retain(|s| s.kind != BlindSpotKind::UserGoalNotExposed);
        assert!(observed.saw_user_request());

        observed.blind_spots.push(BlindSpot {
            kind: BlindSpotKind::UserGoalNotExposed,
            explanation: String::new(),
        });
        assert!(!observed.saw_user_request());
    }

    #[test]
    fn hook_failure_behaviour_is_always_described_in_consequence_terms() {
        for behaviour in [
            HookFailureBehaviour::FailOpen,
            HookFailureBehaviour::FailClosed,
            HookFailureBehaviour::NotApplicable,
        ] {
            let text = behaviour.plain_explanation();
            assert!(text.ends_with('.') || text.is_empty(), "{behaviour:?}");
            for banned in ["fail-open", "fail-closed", "hook"] {
                assert!(
                    !text.to_lowercase().contains(banned),
                    "{behaviour:?} leaks jargon '{banned}'"
                );
            }
        }
    }

    #[test]
    fn blind_spot_explanations_avoid_technical_jargon() {
        for kind in [
            BlindSpotKind::UserGoalNotExposed,
            BlindSpotKind::CompletionClaimNotExposed,
            BlindSpotKind::ToolCallsNotExposed,
            BlindSpotKind::FailuresNotExposed,
            BlindSpotKind::FileEditsNotExposed,
            BlindSpotKind::GitActivityNotExposed,
            BlindSpotKind::NoPreActionControl,
            BlindSpotKind::NoSessionVisibility,
        ] {
            let text = kind.plain_explanation().to_lowercase();
            for banned in ["hook", "session id", "mcp", "tool use id", "payload"] {
                assert!(!text.contains(banned), "{kind:?} leaks jargon '{banned}'");
            }
        }
    }
}

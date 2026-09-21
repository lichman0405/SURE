//! Harness capability tiers.
//!
//! Integrations differ enormously in what they can see. SURE reports the tier it
//! actually achieved, and what it could not see, so that a blind spot is never
//! mistaken for a clean result.

use crate::variants::variants;
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

variants!(
    /// Every tier, weakest first.
    CapabilityTier { Snapshot, Observed, Protected }
);

impl CapabilityTier {
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

variants!(BlindSpotKind {
    UserGoalNotExposed,
    CompletionClaimNotExposed,
    ToolCallsNotExposed,
    FailuresNotExposed,
    FileEditsNotExposed,
    GitActivityNotExposed,
    NoPreActionControl,
    NoSessionVisibility
});

impl BlindSpotKind {
    /// The stable wire name, as a stored record spells it.
    ///
    /// For callers that carry a blind spot where a person will not read it — the
    /// machine-readable report names them, so that a script asks whether a
    /// *named* gap is present instead of parsing the sentence beside it.
    /// `blind_spot_kind_as_str_matches_the_frozen_wire_names` in
    /// `crates/sure-domain/tests/wire_contract.rs` pins this to the same
    /// spellings `blind_spot_wire_names_are_frozen` pins serde to.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UserGoalNotExposed => "user_goal_not_exposed",
            Self::CompletionClaimNotExposed => "completion_claim_not_exposed",
            Self::ToolCallsNotExposed => "tool_calls_not_exposed",
            Self::FailuresNotExposed => "failures_not_exposed",
            Self::FileEditsNotExposed => "file_edits_not_exposed",
            Self::GitActivityNotExposed => "git_activity_not_exposed",
            Self::NoPreActionControl => "no_pre_action_control",
            Self::NoSessionVisibility => "no_session_visibility",
        }
    }

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

/// The window the events a report counted cover.
///
/// The two ends as the envelopes carried them (RFC 3339), not reformatted: a
/// reader comparing this line with what a harness wrote is comparing strings,
/// and a second rendering of one instant is a second thing to be wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventWindow {
    /// The earliest event counted.
    pub oldest: String,
    /// The latest event counted.
    pub newest: String,
}

/// What a capability report counted, and where it looked.
///
/// Present exactly when the tier was derived from the events a store holds for
/// the project. [`CapabilityReport::evidence`] is `None` when SURE counted
/// nothing at all — the report then describes the command line, which has no
/// session visibility to count — and in that case the report says nothing about
/// counting rather than saying it counted none.
///
/// # Why the counted facts are here rather than in a caller's log
///
/// Because a tier with no account of itself is the shape this product exists to
/// prevent. "Tier 1" alone cannot be told from an adapter's own claim, and
/// "tier 0" alone cannot be told from "events exist and were not counted" —
/// which is the difference between a project nobody recorded a session for and a
/// store whose sessions belong to a different project, or a read that stopped
/// early. Every field below exists to make one of those distinctions readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEvidence {
    /// The harnesses the counted events came from, deduplicated and sorted.
    ///
    /// Empty when nothing was counted for this project, and empty when the store
    /// could not be read at all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub harnesses: Vec<String>,
    /// How many session events were counted for this project.
    pub events: usize,
    /// The window the counted events cover.
    ///
    /// `None` when nothing was counted, and `None` when some counted event's
    /// time is not a timestamp SURE can read — the sentence that uses this
    /// states a span, and a span over the events that happened to parse would
    /// leave the others outside it while still reading as the whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<EventWindow>,
    /// Session events the read found that are about other projects.
    ///
    /// The count is of events that were **read** and not counted towards this
    /// project, which is exactly the sentence the report writes. A store with
    /// session events for a different project is not a project with a session,
    /// and the two must not read alike.
    pub elsewhere: usize,
    /// Whether the read stopped at its own limit before reading everything.
    ///
    /// True means older events than the ones counted may exist; the report says
    /// so rather than presenting the newest page as the whole history.
    pub truncated: bool,
    /// Whether SURE could not read the events its store holds at all.
    ///
    /// The safe direction, and the reason it is a field rather than a silent
    /// empty: an unreadable store reports the snapshot tier with no events
    /// counted, and a reader is told that this is what happened rather than
    /// being left to conclude that no session was ever recorded here.
    pub unreadable: bool,
}

impl CapabilityEvidence {
    /// One sentence (or two) saying what was counted, in a reader's own terms.
    ///
    /// Every branch is a statement about what SURE read and counted, so none of
    /// them can be true of the wrong case: "counted no session events" and
    /// "read events recorded for other projects" are different sentences, and a
    /// reader who sees the first without the second knows the difference.
    #[must_use]
    pub fn counted(&self) -> String {
        let mut sentence = if self.unreadable {
            "SURE could not read the events its store holds, so it counted none for this project."
                .to_owned()
        } else if self.events == 0 {
            "SURE counted no session events for this project: SURE's store holds none for it."
                .to_owned()
        } else {
            let harnesses = join_list(&self.harnesses);
            let by = if harnesses.is_empty() {
                String::new()
            } else {
                format!(" by {harnesses}")
            };
            let window = match &self.window {
                Some(window) if window.oldest == window.newest => {
                    format!(", at {}", window.oldest)
                }
                Some(window) => format!(", between {} and {}", window.oldest, window.newest),
                None => ", at times SURE could not read".to_owned(),
            };
            format!(
                "SURE counted {} session event{} recorded for this project{by}{window}.",
                self.events,
                if self.events == 1 { "" } else { "s" },
            )
        };

        if self.elsewhere > 0 {
            sentence.push_str(&format!(
                " SURE also read {} session event{} recorded for other projects; they are not \
                 part of this project's tier and were not counted.",
                self.elsewhere,
                if self.elsewhere == 1 { "" } else { "s" },
            ));
        }
        if self.truncated {
            sentence.push_str(
                " SURE stopped before it had read every event the store holds, so anything older \
                 than the events it read is not counted here.",
            );
        }
        sentence
    }
}

/// A list of names as a sentence: `a`, `a and b`, `a, b and c`.
///
/// The same shape [`BlindSpotKind::plain_explanation`]'s callers use for lists a
/// person reads, written once so that "a and b and c" cannot appear in one
/// sentence of a report and "a, b and c" in the next.
fn join_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
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
    /// What the tier was counted from, when it came from recorded events.
    ///
    /// `None` means SURE counted nothing: the report is the command line's own,
    /// which has no session to count. A reader must be able to tell the two
    /// apart, so the absence is a field rather than an empty count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<CapabilityEvidence>,
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
            // The command line did not count anything: there is no session for it
            // to count, and a report that carried an empty count would read as a
            // session it looked for and did not find.
            evidence: None,
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
    ///
    /// # What one line has to carry
    ///
    /// The tier, then — when the tier was counted from recorded events — what
    /// was counted, and then what the tier still does not cover. Three things
    /// and not one, because each is a way for this line to be wrong on its own:
    ///
    /// - A tier with no account of itself cannot be told from an adapter's own
    ///   claim, and "tier 0" cannot be told from "events exist and were not
    ///   counted" ([`CapabilityEvidence::counted`] is where that distinction is
    ///   written).
    /// - A tier with no blind spots beside it reads as completeness, which is
    ///   the one thing a tier is not: [`CapabilityTier::Observed`] means SURE
    ///   saw what happened, not that it saw all of it, and
    ///   [`CapabilityTier::Snapshot`] means SURE saw none of it.
    ///
    /// The blind spots are written exactly as
    /// [`BlindSpotKind::plain_explanation`] returns them — this method is a
    /// rendering, not a second wording, and a caller that paraphrases them here
    /// has created a copy free to drift from the one the report renders
    /// elsewhere.
    ///
    /// The two halves appear together or not at all: a report with no evidence
    /// counted nothing, and its summary is the tier line and nothing else, which
    /// is what every report written before this field existed said.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut line = format!(
            "{} (capability tier {}, {})",
            self.tier.plain_description(),
            self.tier.number(),
            self.tier.as_str()
        );
        let Some(evidence) = &self.evidence else {
            return line;
        };
        line.push(' ');
        line.push_str(&evidence.counted());
        if !self.blind_spots.is_empty() {
            line.push_str(" What SURE still cannot see: ");
            let explanations: Vec<&str> = self
                .blind_spots
                .iter()
                .map(|spot| spot.explanation.as_str())
                .collect();
            line.push_str(&explanations.join(" "));
        }
        line
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

variants!(HookFailureBehaviour {
    FailOpen,
    FailClosed,
    NotApplicable
});

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
        for &tier in CapabilityTier::ALL {
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
            evidence: None,
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

    /// The evidence a project with a session produces, with the parts a test
    /// does not care about left neutral.
    fn counted(events: usize, harnesses: &[&str]) -> CapabilityEvidence {
        CapabilityEvidence {
            harnesses: harnesses.iter().map(|name| (*name).to_owned()).collect(),
            events,
            window: None,
            elsewhere: 0,
            truncated: false,
            unreadable: false,
        }
    }

    #[test]
    fn a_report_that_counted_nothing_is_the_line_it_always_was() {
        // Every run without a store, and every report written before the
        // evidence field existed. The two halves are the point: no count is
        // claimed, and no blind spot is named, because the tier line already
        // says SURE sees nothing of the session.
        let line = CapabilityReport::cli().summary();
        assert_eq!(
            line,
            "SURE can look at the project as it is now. It cannot see what the AI did while it \
             worked. (capability tier 0, snapshot)"
        );
    }

    #[test]
    fn a_counted_line_names_the_harnesses_and_the_window() {
        // Criterion 4: a reader must be able to tell "nothing was recorded" from
        // "events exist and were not counted", and neither from "SURE counted
        // them". What makes the third readable is this sentence.
        let evidence = CapabilityEvidence {
            harnesses: vec!["claude-code".to_owned(), "codex".to_owned()],
            events: 2,
            window: Some(EventWindow {
                oldest: "2026-09-14T09:10:56.827Z".to_owned(),
                newest: "2026-09-14T10:22:01.000Z".to_owned(),
            }),
            elsewhere: 0,
            truncated: false,
            unreadable: false,
        };
        let line = evidence.counted();
        assert_eq!(
            line,
            "SURE counted 2 session events recorded for this project by claude-code and codex, \
             between 2026-09-14T09:10:56.827Z and 2026-09-14T10:22:01.000Z."
        );
    }

    #[test]
    fn one_event_is_a_span_of_one_instant_and_one_harness_is_not_a_list() {
        let evidence = CapabilityEvidence {
            harnesses: vec!["codex".to_owned()],
            events: 1,
            window: Some(EventWindow {
                oldest: "2026-09-14T09:10:56.827Z".to_owned(),
                newest: "2026-09-14T09:10:56.827Z".to_owned(),
            }),
            elsewhere: 0,
            truncated: false,
            unreadable: false,
        };
        assert_eq!(
            evidence.counted(),
            "SURE counted 1 session event recorded for this project by codex, at \
             2026-09-14T09:10:56.827Z."
        );
    }

    #[test]
    fn an_empty_count_says_the_store_holds_none_and_not_that_none_exist() {
        // "SURE counted none" is a statement about the read, and the sentence
        // must not become a statement about the project: a store that holds a
        // session for a different directory is exactly the case this wording
        // exists for.
        let mut evidence = counted(0, &[]);
        evidence.elsewhere = 3;
        assert_eq!(
            evidence.counted(),
            "SURE counted no session events for this project: SURE's store holds none for it. \
             SURE also read 3 session events recorded for other projects; they are not part of \
             this project's tier and were not counted."
        );
    }

    #[test]
    fn an_unreadable_store_does_not_report_the_project_as_having_no_session() {
        let evidence = CapabilityEvidence {
            harnesses: Vec::new(),
            events: 0,
            window: None,
            elsewhere: 0,
            truncated: false,
            unreadable: true,
        };
        assert_eq!(
            evidence.counted(),
            "SURE could not read the events its store holds, so it counted none for this project."
        );
    }

    #[test]
    fn a_read_that_stopped_early_says_so_rather_than_presenting_the_page_as_the_whole() {
        let mut evidence = counted(4, &["claude-code"]);
        evidence.window = Some(EventWindow {
            oldest: "2026-09-14T09:10:56.827Z".to_owned(),
            newest: "2026-09-14T09:10:57.827Z".to_owned(),
        });
        evidence.truncated = true;
        let line = evidence.counted();
        assert!(
            line.ends_with(
                "SURE stopped before it had read every event the store holds, so anything older \
                 than the events it read is not counted here."
            ),
            "{line}"
        );
        assert!(line.contains("counted 4 session events"), "{line}");
    }

    #[test]
    fn a_counted_summary_carries_the_tier_the_count_and_the_blind_spots() {
        // The three parts in one line, in the order a reader needs them: what
        // SURE can do, what it counted that lets it say so, and what it still
        // cannot see. Blind spots are the explanations the report uses
        // everywhere, not a second wording of them.
        let report = CapabilityReport {
            adapter: "codex".to_owned(),
            tier: CapabilityTier::Observed,
            blind_spots: vec![
                BlindSpot {
                    kind: BlindSpotKind::FailuresNotExposed,
                    explanation: BlindSpotKind::FailuresNotExposed
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
            evidence: Some(counted(1, &["codex"])),
        };
        let line = report.summary();
        assert!(
            line.starts_with(CapabilityTier::Observed.plain_description()),
            "{line}"
        );
        assert!(line.contains("(capability tier 1, observed)"), "{line}");
        assert!(
            line.contains("SURE counted 1 session event recorded for this project by codex"),
            "{line}"
        );
        assert!(
            line.contains("What SURE still cannot see: "),
            "the blind spots are not named: {line}"
        );
        assert!(
            line.contains(BlindSpotKind::NoPreActionControl.plain_explanation()),
            "{line}"
        );
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

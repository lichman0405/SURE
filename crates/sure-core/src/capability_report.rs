//! Derive a harness capability report from recorded events.
//!
//! Adapters may claim a tier, but the events SURE actually received are what
//! determine what it can honestly say. This module turns a slice of ingested
//! events into a [`CapabilityReport`] whose tier and blind spots reflect what
//! was really observed.

use sure_domain::capability::{
    BlindSpot, BlindSpotKind, CapabilityReport, CapabilityTier, HookFailureBehaviour,
};

use crate::claim_capture::AGENT_CLAIM_EVENT_TYPE;
use crate::harness_event::IngestedEvent;
use crate::intent_capture::USER_REQUEST_EVENT_TYPE;

/// Build a capability report from the events SURE actually stored for a session.
///
/// The returned report is based only on the events; it does not trust an
/// adapter's self-reported tier. If the caller has an adapter report, it can
/// take the lower of the two tiers.
#[must_use]
pub fn report_from_events(
    events: &[IngestedEvent],
    adapter: impl Into<String>,
) -> CapabilityReport {
    let adapter = adapter.into();

    if events.is_empty() {
        return CapabilityReport {
            adapter,
            tier: CapabilityTier::Snapshot,
            blind_spots: vec![BlindSpot {
                kind: BlindSpotKind::NoSessionVisibility,
                explanation: BlindSpotKind::NoSessionVisibility
                    .plain_explanation()
                    .to_owned(),
            }],
            pre_action_control: false,
            hook_failure: HookFailureBehaviour::NotApplicable,
        };
    }

    let has_user_request = events
        .iter()
        .any(|event| event.envelope.event_type == USER_REQUEST_EVENT_TYPE);
    let has_agent_claim = events
        .iter()
        .any(|event| event.envelope.event_type == AGENT_CLAIM_EVENT_TYPE);
    let has_tool_or_command = events.iter().any(|event| {
        let event_type = event.envelope.event_type.as_str();
        event_type.contains("tool")
            || event_type.contains("command")
            || event_type.contains("shell")
    });
    let has_file = events
        .iter()
        .any(|event| event.envelope.event_type.contains("file"));
    let has_git = events
        .iter()
        .any(|event| event.envelope.event_type.contains("git"));
    let has_failure = events.iter().any(|event| {
        let event_type = event.envelope.event_type.as_str();
        event_type.contains("error")
            || event_type.contains("failed")
            || event_type.contains("failure")
    });
    let has_protected = events
        .iter()
        .any(|event| event.envelope.capability_tier == Some(CapabilityTier::Protected));

    let mut blind_spots = Vec::new();
    if !has_user_request {
        blind_spots.push(BlindSpot {
            kind: BlindSpotKind::UserGoalNotExposed,
            explanation: BlindSpotKind::UserGoalNotExposed
                .plain_explanation()
                .to_owned(),
        });
    }
    if !has_agent_claim {
        blind_spots.push(BlindSpot {
            kind: BlindSpotKind::CompletionClaimNotExposed,
            explanation: BlindSpotKind::CompletionClaimNotExposed
                .plain_explanation()
                .to_owned(),
        });
    }
    if !has_tool_or_command {
        blind_spots.push(BlindSpot {
            kind: BlindSpotKind::ToolCallsNotExposed,
            explanation: BlindSpotKind::ToolCallsNotExposed
                .plain_explanation()
                .to_owned(),
        });
    }
    if !has_failure {
        blind_spots.push(BlindSpot {
            kind: BlindSpotKind::FailuresNotExposed,
            explanation: BlindSpotKind::FailuresNotExposed
                .plain_explanation()
                .to_owned(),
        });
    }
    if !has_file {
        blind_spots.push(BlindSpot {
            kind: BlindSpotKind::FileEditsNotExposed,
            explanation: BlindSpotKind::FileEditsNotExposed
                .plain_explanation()
                .to_owned(),
        });
    }
    if !has_git {
        blind_spots.push(BlindSpot {
            kind: BlindSpotKind::GitActivityNotExposed,
            explanation: BlindSpotKind::GitActivityNotExposed
                .plain_explanation()
                .to_owned(),
        });
    }

    let tier = if has_protected {
        CapabilityTier::Protected
    } else {
        CapabilityTier::Observed
    };

    CapabilityReport {
        adapter,
        tier,
        blind_spots,
        pre_action_control: has_protected,
        hook_failure: HookFailureBehaviour::NotApplicable,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;
    use sure_domain::capability::CapabilityTier;
    use sure_protocol::PROTOCOL_VERSION;
    use sure_protocol::documents::DocumentKind;
    use sure_protocol::event::EventEnvelope;

    use crate::harness_event::IngestedEvent;

    fn event(event_type: &str, tier: Option<CapabilityTier>) -> IngestedEvent {
        let mut envelope =
            EventEnvelope::new("claude-code", event_type, "2026-09-14T09:10:56.827Z");
        if let Some(t) = tier {
            envelope = envelope.with_capability_tier(t);
        }
        IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        }
    }

    fn has_blind_spot(report: &CapabilityReport, kind: BlindSpotKind) -> bool {
        report.blind_spots.iter().any(|spot| spot.kind == kind)
    }

    #[test]
    fn no_events_means_snapshot_with_no_session_visibility() {
        let report = report_from_events(&[], "claude-code");
        assert_eq!(report.tier, CapabilityTier::Snapshot);
        assert!(has_blind_spot(&report, BlindSpotKind::NoSessionVisibility));
        assert!(!has_blind_spot(&report, BlindSpotKind::UserGoalNotExposed));
    }

    #[test]
    fn user_request_only_is_observed_but_has_many_blind_spots() {
        let events = vec![event("user.request", Some(CapabilityTier::Observed))];
        let report = report_from_events(&events, "claude-code");
        assert_eq!(report.tier, CapabilityTier::Observed);
        assert!(!has_blind_spot(&report, BlindSpotKind::NoSessionVisibility));
        assert!(!has_blind_spot(&report, BlindSpotKind::UserGoalNotExposed));
        assert!(has_blind_spot(
            &report,
            BlindSpotKind::CompletionClaimNotExposed
        ));
        assert!(has_blind_spot(&report, BlindSpotKind::ToolCallsNotExposed));
        assert!(has_blind_spot(&report, BlindSpotKind::FailuresNotExposed));
        assert!(has_blind_spot(&report, BlindSpotKind::FileEditsNotExposed));
        assert!(has_blind_spot(
            &report,
            BlindSpotKind::GitActivityNotExposed
        ));
    }

    #[test]
    fn full_observed_session_has_no_blind_spots() {
        let events = vec![
            event("user.request", Some(CapabilityTier::Observed)),
            event("agent.claim", Some(CapabilityTier::Observed)),
            event("tool.completed", Some(CapabilityTier::Observed)),
            event("command.failed", Some(CapabilityTier::Observed)),
            event("file.write", Some(CapabilityTier::Observed)),
            event("git.commit", Some(CapabilityTier::Observed)),
        ];
        let report = report_from_events(&events, "claude-code");
        assert_eq!(report.tier, CapabilityTier::Observed);
        assert!(report.blind_spots.is_empty(), "{:?}", report.blind_spots);
    }

    #[test]
    fn protected_event_lifts_tier_to_protected_and_removes_control_blind_spot() {
        let events = vec![
            event("user.request", Some(CapabilityTier::Protected)),
            event("agent.claim", Some(CapabilityTier::Protected)),
            event("tool.completed", Some(CapabilityTier::Protected)),
            event("command.failed", Some(CapabilityTier::Protected)),
            event("file.write", Some(CapabilityTier::Protected)),
            event("git.commit", Some(CapabilityTier::Protected)),
        ];
        let report = report_from_events(&events, "claude-code");
        assert_eq!(report.tier, CapabilityTier::Protected);
        assert!(report.pre_action_control);
        assert!(!has_blind_spot(&report, BlindSpotKind::NoPreActionControl));
    }

    #[test]
    fn missing_failure_events_are_reported_honestly() {
        let events = vec![
            event("user.request", Some(CapabilityTier::Observed)),
            event("agent.claim", Some(CapabilityTier::Observed)),
            event("tool.completed", Some(CapabilityTier::Observed)),
            event("file.write", Some(CapabilityTier::Observed)),
            event("git.commit", Some(CapabilityTier::Observed)),
        ];
        let report = report_from_events(&events, "claude-code");
        assert!(has_blind_spot(&report, BlindSpotKind::FailuresNotExposed));
    }

    #[test]
    fn event_payload_is_not_used_for_capability_detection() {
        // A file-like payload in a non-file event must not create a false
        // file-edits capability.
        let mut envelope =
            EventEnvelope::new("claude-code", "tool.completed", "2026-09-14T09:10:56.827Z")
                .with_payload(json!({"path": "src/main.rs", "operation": "write"}));
        envelope = envelope.with_capability_tier(CapabilityTier::Observed);
        let events = vec![IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        }];
        let report = report_from_events(&events, "claude-code");
        assert!(has_blind_spot(&report, BlindSpotKind::FileEditsNotExposed));
    }
}

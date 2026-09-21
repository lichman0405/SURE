//! Derive a harness capability report from recorded events.
//!
//! Adapters may claim a tier, but the events SURE actually received are what
//! determine what it can honestly say. This module turns a slice of ingested
//! events into a [`CapabilityReport`] whose tier and blind spots reflect what
//! was really observed. It never promotes a report to `Protected` just because
//! an event carries a self-reported `Protected` tier; pre-action control must
//! be proved by something stronger than the adapter's own label.
//!
//! # The store's own events
//!
//! [`for_project`] is that report for a project whose events are in a store:
//! it reads them, hands them to [`report_from_events`], and adds the two things
//! a report derived from events cannot know by itself — the blind spot no event
//! can ever remove, and the account of what was counted. It is the only place a
//! product path reaches this module, and the pipeline that calls it adds no rule
//! of its own about what a tier means.

use std::collections::BTreeSet;

use sure_domain::capability::{
    BlindSpot, BlindSpotKind, CapabilityEvidence, CapabilityReport, CapabilityTier, EventWindow,
    HookFailureBehaviour,
};
use sure_protocol::documents::DocumentKind;
use sure_protocol::event::EventEnvelope;

use crate::claim_capture::AGENT_CLAIM_EVENT_TYPE;
use crate::diagnostics::Timestamp;
use crate::harness_event::IngestedEvent;
use crate::intent_capture::USER_REQUEST_EVENT_TYPE;
use crate::store::{HistoryFilter, RecordKind, Store, StoreError, StoredRecord};

/// How many stored events one capability read looks at.
///
/// A bound rather than no bound, because this read happens on every `sure check`
/// and a store that has been recording for a year is not a reason for a check to
/// slow down. The same figure `crate::recheck_lifecycle` reads its history with,
/// for the same reason: the newest page of a project's events is what a tier is
/// counted from, and a read that stopped says so
/// ([`CapabilityEvidence::truncated`]) rather than presenting the page as the
/// whole history.
pub const EVENT_SCAN_LIMIT: usize = 4096;

/// Build a capability report from the events SURE actually stored for a session.
///
/// The returned report is based only on the events; it does not trust an
/// adapter's self-reported tier. If the caller has an adapter report, it can
/// take the lower of the two tiers.
///
/// The tier is `Snapshot` when no events exist and `Observed` otherwise. Events
/// alone cannot prove pre-action control, so this function never returns
/// `Protected`.
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
            // Empty events are the one case where "nothing was counted" and
            // "there was nothing to count" are the same statement, and this
            // function cannot tell which project it was asked about. The caller
            // that has a store fills this in; a caller that does not is not
            // making an account of a read, and says so by leaving this `None`.
            evidence: None,
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

    CapabilityReport {
        adapter,
        tier: CapabilityTier::Observed,
        blind_spots,
        pre_action_control: false,
        hook_failure: HookFailureBehaviour::NotApplicable,
        evidence: None,
    }
}

/// The capability a project's own recorded events earn, and what was read.
///
/// Returned together because the two are one answer: a report whose read failed
/// is not the same answer as a project with no events, and a caller that only
/// received the report could not tell them apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCapability {
    /// What this run may say about how much of the session it saw.
    pub report: CapabilityReport,
    /// What went wrong reading the store, when something did.
    ///
    /// Text, and it is the store's own — a caller renders it where it renders
    /// every other failure, rather than inside the capability line, which is a
    /// sentence SURE writes about itself and must not be rewritable by whatever
    /// the underlying error happened to say.
    pub read_failure: Option<String>,
}

/// The capability report for one project, from the events its store holds.
///
/// `store` is the store the caller was given — `None` when the run has none —
/// and `project_root` is the project as the run resolved it, because that is
/// what a record's own `project_root` is compared against. The comparison is by
/// that path rather than by the project fingerprint on purpose: a fingerprint
/// names a *state* of a project, so an event recorded before any file changed
/// would stop counting at the first edit, and a project whose session was
/// recorded would be reported as a project nobody ever recorded.
///
/// **What "compared against" means is the directory's identity, not the
/// spelling.** Two `project_root` strings are one project when they name one
/// directory, decided by [`crate::recheck_lifecycle::same_project_root`] under
/// the rule this project's volume gives for case
/// ([`crate::recheck_lifecycle::case_rule_for`]). A record whose
/// `project_root` is `None` is never this project's, and the rule errs toward
/// counting fewer events — see `same_project_root` for the direction and why it
/// is that one.
///
/// # What this adds to [`report_from_events`]
///
/// Two things, and they are here rather than at the call site because both are
/// rules about what a tier means rather than rules about a pipeline:
///
/// 1. **The blind spot no event can remove.** Events cannot prove that SURE can
///    stop an action before it happens — that is proved by a hook that runs
///    before the action and a harness that reads its answer — so a report
///    derived from events always has `pre_action_control: false`, and a report
///    with no pre-action control must admit it. Without this, the report for a
///    project with a session would carry *fewer* blind spots than the report for
///    a project without one, which reads as SURE seeing more the moment it
///    started watching — and it is exactly the shape of false green this module
///    exists to refuse. The rule is the domain's own, applied to the list:
///    [`CapabilityReport::is_self_consistent`] requires pre-action control to be
///    claimed exactly at the protected tier, and the blind spot is what that
///    requirement looks like to a reader.
/// 2. **What was counted.** See [`CapabilityEvidence`] for why a tier with no
///    account of itself cannot be told from an adapter's claim.
#[must_use]
pub fn for_project(store: Option<&Store>, project_root: &str) -> ProjectCapability {
    // No store, nothing counted. The report is the command line's own, which is
    // what every run without a history has always reported.
    let Some(store) = store else {
        return ProjectCapability {
            report: CapabilityReport::cli(),
            read_failure: None,
        };
    };

    let filter = HistoryFilter {
        project_fingerprint: None,
        kind: Some(RecordKind::Document(DocumentKind::Event)),
        include_recordings: false,
    };
    let records = match store.history(&filter, EVENT_SCAN_LIMIT) {
        Ok(records) => records,
        Err(error) => return unreadable(&error),
    };
    let truncated = records.len() == EVENT_SCAN_LIMIT;

    // The volume is asked once, here, and the one answer decides which rows are
    // this project's. The question *is this record about this project* is a
    // question about a directory, and the answer is the identity of that
    // directory rather than a spelling of it: `C:\work\mine`, `C:/work/mine`,
    // `C:\work\mine\` and `C:\work\.\mine` are one directory on Windows, and a
    // user who copied a path out of a tool that writes forward slashes is
    // checking the project SURE has a session for. Comparing the two strings
    // instead reported `(capability tier 0, snapshot) SURE counted no session
    // events for this project: SURE's store holds none for it` — a confident,
    // specific and false statement about the store, for the four spellings out
    // of five a real `sure check` was measured with.
    //
    // The rule is `recheck_lifecycle::same_project_root` rather than one written
    // here, because the re-check asks the same question about the same field of
    // the same rows (`previous_open_findings`) and two rules would let the tier
    // and the comparison disagree. `recheck_lifecycle::case_rule_for` is the
    // other half: the volume decides the case question, and it answers
    // `Sensitive` when it cannot be asked, which counts fewer events rather
    // than another project's. See `same_project_root` for what the rule asks,
    // what it does when it cannot tell, and which way it errs.
    let case = crate::recheck_lifecycle::case_rule_for(project_root);
    let (mine, others): (Vec<StoredRecord>, Vec<StoredRecord>) =
        records.into_iter().partition(|record| {
            record.project_root.as_deref().is_some_and(|recorded| {
                crate::recheck_lifecycle::same_project_root(recorded, project_root, case)
            })
        });

    let events = match parse_event_records(&mine) {
        Ok(events) => events,
        Err(error) => return unreadable(&error),
    };

    let harnesses: Vec<String> = {
        let names: BTreeSet<String> = events
            .iter()
            .map(|event| event.envelope.source.clone())
            .collect();
        names.into_iter().collect()
    };
    let adapter = if harnesses.is_empty() {
        // The command line's own name, which is what this report is when no
        // event earned the project a tier: nothing here describes a harness.
        "cli".to_owned()
    } else {
        harnesses.join(", ")
    };

    let mut report = report_from_events(&events, adapter);
    report.blind_spots = with_unprovable_blind_spots(report.blind_spots, report.pre_action_control);
    report.evidence = Some(CapabilityEvidence {
        harnesses,
        events: events.len(),
        window: window_of(&events),
        elsewhere: others.len(),
        truncated,
        unreadable: false,
    });
    ProjectCapability {
        report,
        read_failure: None,
    }
}

/// A report for a store SURE could not read, and the reason.
///
/// The tier is the snapshot tier, which is the lowest claim available and the
/// safe direction, and the report says in its own words that this is what
/// happened. Reporting "no events" here would be a claim about the project made
/// from a failure to look at it.
fn unreadable(error: &StoreError) -> ProjectCapability {
    let mut report = CapabilityReport::cli();
    report.evidence = Some(CapabilityEvidence {
        harnesses: Vec::new(),
        events: 0,
        window: None,
        elsewhere: 0,
        truncated: false,
        unreadable: true,
    });
    ProjectCapability {
        report,
        read_failure: Some(error.to_string()),
    }
}

/// The blind spots a report derived from events may never drop.
///
/// One, today: no events can prove pre-action control, so a report that does not
/// claim it must say it cannot do it. Written as a rule about the report's own
/// fields rather than as a rule about this function's inputs, so that a later
/// caller that does have proof of pre-action control gets a report without the
/// blind spot, and one that does not cannot lose it by forgetting to add it.
fn with_unprovable_blind_spots(
    mut blind_spots: Vec<BlindSpot>,
    pre_action_control: bool,
) -> Vec<BlindSpot> {
    if !pre_action_control
        && !blind_spots
            .iter()
            .any(|spot| spot.kind == BlindSpotKind::NoPreActionControl)
    {
        blind_spots.push(BlindSpot {
            kind: BlindSpotKind::NoPreActionControl,
            explanation: BlindSpotKind::NoPreActionControl
                .plain_explanation()
                .to_owned(),
        });
    }
    blind_spots
}

/// The span the events cover, when every one of them carries a time SURE reads.
///
/// `None` unless all of them parse: the sentence that uses this says "between
/// X and Y", and a span computed over the events that happened to parse would
/// place the others outside itself while still reading as the whole.
fn window_of(events: &[IngestedEvent]) -> Option<EventWindow> {
    let mut oldest: Option<(i64, &str)> = None;
    let mut newest: Option<(i64, &str)> = None;
    for event in events {
        let text = event.envelope.timestamp.as_str();
        let millis = Timestamp::parse_rfc3339(text)?.as_millis();
        if oldest.is_none_or(|(at, _)| millis < at) {
            oldest = Some((millis, text));
        }
        if newest.is_none_or(|(at, _)| millis > at) {
            newest = Some((millis, text));
        }
    }
    Some(EventWindow {
        oldest: oldest?.1.to_owned(),
        newest: newest?.1.to_owned(),
    })
}

/// The events a store holds, as the reader that derives capabilities needs them.
///
/// The same reconstruction `crate::claim_checker` does, and for the same reason:
/// a row's document is an envelope on the wire, and what a capability is derived
/// from is the envelope SURE ingested rather than a second reading of the JSON.
fn parse_event_records(records: &[StoredRecord]) -> Result<Vec<IngestedEvent>, StoreError> {
    let mut events = Vec::with_capacity(records.len());
    for record in records {
        let json =
            serde_json::to_string(&record.document).map_err(|error| StoreError::MalformedRow {
                id: record.id,
                message: format!("could not serialize the stored document: {error}"),
            })?;
        let envelope =
            EventEnvelope::from_json(&json).map_err(|error| StoreError::MalformedRow {
                id: record.id,
                message: format!("the stored document is not an event envelope: {error}"),
            })?;
        let protocol_version = envelope.schema_version;
        events.push(IngestedEvent {
            envelope,
            protocol_version,
            document_kind: DocumentKind::Event,
        });
    }
    Ok(events)
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
    use crate::paths::CaseSensitivity;

    fn event(event_type: &str) -> IngestedEvent {
        let envelope = EventEnvelope::new("claude-code", event_type, "2026-09-14T09:10:56.827Z");
        IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        }
    }

    trait WithEnvelopeTier {
        fn with_envelope_tier(self, tier: CapabilityTier) -> Self;
    }

    impl WithEnvelopeTier for IngestedEvent {
        fn with_envelope_tier(mut self, tier: CapabilityTier) -> Self {
            self.envelope = self.envelope.with_capability_tier(tier);
            self
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
        let events = vec![event("user.request")];
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
            event("user.request"),
            event("agent.claim"),
            event("tool.completed"),
            event("command.failed"),
            event("file.write"),
            event("git.commit"),
        ];
        let report = report_from_events(&events, "claude-code");
        assert_eq!(report.tier, CapabilityTier::Observed);
        assert!(report.blind_spots.is_empty(), "{:?}", report.blind_spots);
    }

    #[test]
    fn self_reported_protected_tier_is_not_trusted() {
        // Every event claims Protected, but the derived report must stay Observed
        // because the adapter's own labels are not proof of pre-action control.
        let events = vec![
            event("user.request").with_envelope_tier(CapabilityTier::Protected),
            event("agent.claim").with_envelope_tier(CapabilityTier::Protected),
            event("tool.completed").with_envelope_tier(CapabilityTier::Protected),
            event("command.failed").with_envelope_tier(CapabilityTier::Protected),
            event("file.write").with_envelope_tier(CapabilityTier::Protected),
            event("git.commit").with_envelope_tier(CapabilityTier::Protected),
        ];
        let report = report_from_events(&events, "claude-code");
        assert_eq!(report.tier, CapabilityTier::Observed);
        assert!(!report.pre_action_control);
    }

    #[test]
    fn missing_failure_events_are_reported_honestly() {
        let events = vec![
            event("user.request"),
            event("agent.claim"),
            event("tool.completed"),
            event("file.write"),
            event("git.commit"),
        ];
        let report = report_from_events(&events, "claude-code");
        assert!(has_blind_spot(&report, BlindSpotKind::FailuresNotExposed));
    }

    // --- the project's own recorded events ---------------------------------

    use sure_domain::ids::{EventId, FingerprintId};

    use crate::session_event_store::SessionEventStore;
    use crate::store::Store;

    /// A store of this test's own, under the repository's scratch space.
    ///
    /// Unique per test name rather than cleared and reused, because a fixed path
    /// that is cleared on Windows is a path the next run may still be holding.
    fn store_in(name: &str) -> Store {
        let directory = crate::store::scratch_root().join(format!(
            "capability-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        Store::open_at(&directory.join("sure.db")).expect("the store opens")
    }

    /// Record one event for `project_root`, the way a harness hook would.
    fn record(store: &Store, project_root: &str, event: &IngestedEvent) {
        SessionEventStore::new(store)
            .persist(
                event,
                project_root,
                &FingerprintId::generate(),
                &EventId::generate(),
            )
            .expect("the event is stored");
    }

    fn timed(event_type: &str, timestamp: &str) -> IngestedEvent {
        IngestedEvent {
            envelope: EventEnvelope::new("codex", event_type, timestamp),
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        }
    }

    #[test]
    fn a_project_with_its_own_events_reports_the_tier_they_earn() {
        // Criterion 1, and criterion 2 from the other side: the tier is derived
        // from what SURE received, so it is the observed tier the events earned
        // rather than the snapshot tier the command line would report.
        let store = store_in("project-events");
        let root = "C:\\work\\recorded project";
        for event_type in [
            "user.request",
            "agent.claim",
            "tool.completed",
            "command.failed",
            "file.write",
            "git.commit",
        ] {
            record(&store, root, &timed(event_type, "2026-09-14T09:10:56.827Z"));
        }

        let capability = for_project(Some(&store), root);
        assert_eq!(capability.report.tier, CapabilityTier::Observed);
        assert_eq!(capability.report.tier_number(), 1);
        assert_eq!(capability.read_failure, None);

        let evidence = capability
            .report
            .evidence
            .as_ref()
            .expect("a tier counted from events carries the count");
        assert_eq!(evidence.events, 6);
        assert_eq!(evidence.harnesses, vec!["codex".to_owned()]);
        assert_eq!(
            evidence.window,
            Some(EventWindow {
                oldest: "2026-09-14T09:10:56.827Z".to_owned(),
                newest: "2026-09-14T09:10:56.827Z".to_owned(),
            })
        );
        assert_eq!(evidence.elsewhere, 0);
        assert!(!evidence.truncated);
        assert!(!evidence.unreadable);
    }

    #[test]
    fn a_project_with_its_own_events_keeps_the_blind_spot_events_cannot_remove() {
        // The trap, asserted rather than reasoned about: `report_from_events`
        // alone leaves this list **empty** for a session that covers all six
        // kinds (`full_observed_session_has_no_blind_spots` above), and SURE
        // cannot stop an action before it happens, so a report derived from
        // events must still say so. Without this the report would grow *fewer*
        // blind spots the moment a session was recorded — the shape of false
        // green this module exists to refuse.
        let store = store_in("project-union");
        let root = "C:\\work\\union project";
        for event_type in [
            "user.request",
            "agent.claim",
            "tool.completed",
            "command.failed",
            "file.write",
            "git.commit",
        ] {
            record(&store, root, &timed(event_type, "2026-09-14T09:10:56.827Z"));
        }

        let report = for_project(Some(&store), root).report;
        assert!(
            has_blind_spot(&report, BlindSpotKind::NoPreActionControl),
            "a session was recorded and the report stopped saying SURE cannot intervene: {:?}",
            report.blind_spots
        );
        assert!(!report.pre_action_control);
        assert!(report.is_self_consistent());
    }

    #[test]
    fn a_project_with_no_events_gets_the_command_lines_own_report_and_says_which_it_is() {
        // The other half of criterion 1, and criterion 4's first distinction: the
        // tier is the snapshot tier, the two blind spots are the command line's
        // own, and the line says the store holds none for this project rather
        // than leaving a reader to guess why.
        let store = store_in("project-no-events");
        let report = for_project(Some(&store), "C:\\work\\nothing recorded here").report;

        assert_eq!(report.tier, CapabilityTier::Snapshot);
        assert_eq!(report.tier_number(), 0);
        assert_eq!(
            report.blind_spots.len(),
            CapabilityReport::cli().blind_spots.len(),
            "the no-events report has a different number of blind spots than the command \
             line's own, which is a claim about the project made by the act of looking"
        );
        assert!(has_blind_spot(&report, BlindSpotKind::NoSessionVisibility));
        assert!(has_blind_spot(&report, BlindSpotKind::NoPreActionControl));

        let evidence = report
            .evidence
            .expect("the read happened and is accounted for");
        assert_eq!(evidence.events, 0);
        assert_eq!(evidence.elsewhere, 0);
        assert!(evidence.harnesses.is_empty());
        assert!(evidence.window.is_none());
        assert!(!evidence.unreadable);
    }

    #[test]
    fn another_projects_events_are_counted_apart_and_never_borrowed() {
        // The distinction that makes "events exist and were not counted"
        // readable: this project has no session, and the events in the store
        // belong to a different directory. Reporting the tier they would earn
        // would be attributing one project's session to another.
        let store = store_in("project-elsewhere");
        let other = "C:\\work\\a different project";
        record(
            &store,
            other,
            &timed("user.request", "2026-09-14T09:10:56.827Z"),
        );
        record(
            &store,
            other,
            &timed("tool.completed", "2026-09-14T09:10:57.827Z"),
        );

        let report = for_project(Some(&store), "C:\\work\\mine").report;
        assert_eq!(report.tier, CapabilityTier::Snapshot);
        let evidence = report.evidence.as_ref().expect("the read is accounted for");
        assert_eq!(evidence.events, 0);
        assert_eq!(evidence.elsewhere, 2);
        let line = report.summary();
        assert!(
            line.contains("SURE also read 2 session events recorded for other projects"),
            "{line}"
        );
    }

    #[test]
    fn a_store_that_was_not_handed_over_is_the_command_lines_own_report() {
        // `sure check` with no store, and every caller that has no history to
        // read. Not `evidence: Some(0 events)`: nothing was counted, and a report
        // that said it looked and found none would be a claim about a store that
        // was never opened.
        let capability = for_project(None, "C:\\work\\anything");
        assert_eq!(capability.report, CapabilityReport::cli());
        assert_eq!(capability.read_failure, None);
    }

    #[test]
    fn an_adapter_claiming_protected_does_not_raise_the_tier_the_events_earn() {
        // Criterion 2's second half, through the store rather than through the
        // slice: the label an adapter puts on its own events is not proof of
        // pre-action control, and nothing in the path from a stored event to a
        // report may take it upward.
        let store = store_in("project-claim");
        let root = "C:\\work\\claiming project";
        for event_type in ["user.request", "agent.claim", "tool.completed"] {
            let event = timed(event_type, "2026-09-14T09:10:56.827Z")
                .with_envelope_tier(CapabilityTier::Protected);
            record(&store, root, &event);
        }

        let report = for_project(Some(&store), root).report;
        assert_eq!(report.tier, CapabilityTier::Observed);
        assert!(!report.pre_action_control);
        assert_ne!(report.tier_number(), 2);
    }

    // --- one directory, several spellings ----------------------------------

    /// A real directory under `target/tmp` whose volume can be asked, and the
    /// project recorded against it.
    ///
    /// The directory has to exist: the case rule comes from the volume, and a
    /// `project_root` that is not there cannot be probed, so a test that used an
    /// invented path could only ever exercise the `Sensitive` fallback and would
    /// say nothing about the arm this machine is actually on.
    fn scratch_project(name: &str) -> std::path::PathBuf {
        let directory = crate::store::scratch_root().join(format!(
            "capability-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&directory).expect("a scratch project directory");
        // A name with a case to flip, which is what the volume probe reads.
        std::fs::write(directory.join("Cargo.toml"), b"[package]\n").expect("a project file");
        directory
    }

    /// What a report counted, as the three things this module's tests assert.
    fn counted(report: &CapabilityReport) -> (CapabilityTier, usize, usize) {
        let evidence = report
            .evidence
            .as_ref()
            .expect("a read that happened is accounted for");
        (report.tier, evidence.events, evidence.elsewhere)
    }

    /// **Every spelling of one directory is one project**, measured through
    /// [`for_project`] rather than argued about.
    ///
    /// This is the defect `P15-T030` was minted from, and the shape of it is
    /// why this test exists beside
    /// `a_project_with_its_own_events_reports_the_tier_they_earn`: that test
    /// records the session with the same `root` string it later checks, so both
    /// sides of the comparison are one spelling and **any** normalisation passes
    /// it. Here the store holds one session against one spelling and the report
    /// is asked about others.
    ///
    /// A session of four Codex events is recorded for the directory as it is
    /// spelt on disk, and the report is asked about:
    ///
    /// - the forward-slash spelling, which is what a path copied out of a tool
    ///   that writes `/` looks like;
    /// - the spelling with a trailing separator;
    /// - the spelling with a `.` segment inside it.
    ///
    /// Each of those names the directory the session was recorded for on every
    /// platform, so each is asserted on every platform: the tier is the observed
    /// tier, four events were counted, and none was attributed to another
    /// project.
    ///
    /// **The upper-case spelling is the one that is not universal, and this test
    /// says what it is asking.** Whether `.../DEMOPROJ` and `.../demoproj` are
    /// one directory is a fact about the volume and not about the operating
    /// system — CI run `35544579833` measured macOS and Linux answering
    /// differently in one workflow — so the volume is asked here, with
    /// [`crate::paths::case_rule_of_volume`], and the assertion is made against
    /// **the answer that came back**:
    ///
    /// - a volume that folds case: the upper-case spelling is this project, and
    ///   the report counts the four events for it — the understating defect this
    ///   task is about, gone;
    /// - a volume that keeps case: the upper-case spelling is a different
    ///   directory, and the report counts nothing for it and says it read four
    ///   events for other projects. That is the safe direction, asserted rather
    ///   than assumed, and it is also what the mechanism does when it cannot
    ///   tell.
    ///
    /// **What it does if the volume cannot be asked**: it fails. Accepting the
    /// `Sensitive` fallback would make the case-keeping arm below pass without
    /// the volume having been asked anything, which is a test passing for the
    /// wrong reason — the same rule
    /// `recheck_lifecycle`'s volume test states for the same call.
    #[test]
    fn every_spelling_of_one_directory_is_one_project() {
        let store = store_in("project-spellings");
        let directory = scratch_project("project-spellings");
        let root = directory.to_string_lossy().into_owned();
        for event_type in [
            "user.request",
            "agent.claim",
            "tool.completed",
            "command.failed",
        ] {
            record(
                &store,
                &root,
                &timed(event_type, "2026-09-14T09:10:56.827Z"),
            );
        }

        let separators = if cfg!(windows) { '\\' } else { '/' };
        let forward_slashed = root.replace('\\', "/");
        let trailing_separator = format!("{root}{separators}");
        let with_a_dot_segment = directory
            .parent()
            .expect("the scratch directory has a parent")
            .join(".")
            .join(directory.file_name().expect("it has a name"))
            .to_string_lossy()
            .into_owned();

        for spelling in [
            forward_slashed.clone(),
            trailing_separator.clone(),
            with_a_dot_segment.clone(),
        ] {
            assert_eq!(
                counted(&for_project(Some(&store), &spelling).report),
                (CapabilityTier::Observed, 4, 0),
                "the project named {spelling:?} is the directory the session was recorded for, \
                 and the report did not count its events"
            );
        }

        // The volume is asked before the one spelling whose answer is not the
        // same everywhere, and the answer is what the arm below asserts.
        let case = crate::paths::case_rule_of_volume(&directory).unwrap_or_else(|| {
            panic!(
                "the volume holding {} could not be asked, so this test could not say which \
                 answer it was asserting",
                directory.display()
            )
        });
        assert_eq!(
            crate::recheck_lifecycle::case_rule_for(&root),
            case,
            "the rule the report is made with is not the rule this volume gave"
        );

        let upper_case = root.to_uppercase();
        match case {
            CaseSensitivity::Insensitive => {
                assert_eq!(
                    counted(&for_project(Some(&store), &upper_case).report),
                    (CapabilityTier::Observed, 4, 0),
                    "this volume folds case, so {upper_case:?} is the directory the session was \
                     recorded for, and the report did not count its events"
                );
                let line = for_project(Some(&store), &upper_case).report.summary();
                assert!(
                    line.contains("SURE counted 4 session events recorded for this project"),
                    "a spelling Windows treats as this directory is reported as a project SURE \
                     has never seen: {line}"
                );
            }
            CaseSensitivity::Sensitive => {
                assert_eq!(
                    counted(&for_project(Some(&store), &upper_case).report),
                    (CapabilityTier::Snapshot, 0, 4),
                    "this volume keeps case, so {upper_case:?} is a different directory and its \
                     events are not this project's"
                );
            }
        }
    }

    /// **Two directories that are not one are never one project**, and the
    /// events of one are never counted for the other.
    ///
    /// This is the other direction, and it is the one that must not be traded
    /// for the first: a normalisation eager enough to fold `C:\work\mine` into
    /// its sibling would make this project's report count another project's
    /// session as its own, raise the tier on evidence that is not about it, and
    /// name it in the counted sentence — a false green in the product's own
    /// voice, which `CLAUDE.md` ranks above every visible error.
    ///
    /// The five paths below are the shapes an eager normalisation gets wrong,
    /// and every one of them is a genuinely different directory:
    ///
    /// - a **sibling** whose name starts with this one's (`mine`, `mine2`): a
    ///   prefix comparison, or a `starts_with` on the text, matches these;
    /// - a **child** (`mine\sub`) and this project's **parent** (`work`): a
    ///   containment test matches both;
    /// - this project with a `..` segment (**`mine\..`**, which names `work`): a
    ///   normalisation that strips the last component, or that folds `..` into
    ///   the wrong level, matches this;
    /// - this project in **upper case** while the volume cannot be asked.
    ///
    /// That last one is the measured half of the safe direction. These paths do
    /// not exist, so no volume can be probed and
    /// [`crate::recheck_lifecycle::case_rule_for`] answers
    /// [`CaseSensitivity::Sensitive`] — two spellings are two projects, and the
    /// report counts nothing and says how many events it read for other
    /// projects. `every_spelling_of_one_directory_is_one_project` above asserts
    /// the same fallback through a real directory on a volume that keeps case;
    /// this asserts it on every machine, because no machine needs a volume to
    /// answer *there is nothing here to ask*.
    ///
    /// The last reading is the control: the spelling the session was recorded
    /// under does count, so a report that counted nothing for everything would
    /// not satisfy this test either.
    #[test]
    fn two_directories_that_are_not_one_are_never_one_project() {
        let store = store_in("project-not-one");
        let root = "C:\\work\\mine";
        for event_type in [
            "user.request",
            "agent.claim",
            "tool.completed",
            "command.failed",
        ] {
            record(&store, root, &timed(event_type, "2026-09-14T09:10:56.827Z"));
        }

        for other in [
            "C:\\work\\mine2",
            "C:\\work\\mine\\sub",
            "C:\\work",
            "C:\\work\\mine\\..",
            "C:\\work\\MINE",
            "C:/other/mine",
        ] {
            assert_eq!(
                counted(&for_project(Some(&store), other).report),
                (CapabilityTier::Snapshot, 0, 4),
                "{other:?} is a different directory from {root:?}, and the report counted this \
                 project's events for it"
            );
        }

        assert_eq!(
            counted(&for_project(Some(&store), root).report),
            (CapabilityTier::Observed, 4, 0),
            "the control: the spelling the session was recorded under does count"
        );
    }

    #[test]
    fn a_store_sure_could_not_read_reports_the_snapshot_tier_and_the_reason() {
        // The safe direction, and the reason `unreadable` is a field: a read that
        // failed is not a project with no session, and the run must be able to
        // say which happened. The report keeps the command line's own sentence —
        // the store's error text goes in `read_failure`, which a caller renders
        // where it renders every other failure.
        let error = StoreError::Busy {
            path: std::path::PathBuf::from("C:\\sure\\sure.db"),
            waited_ms: 5_000,
        };
        let capability = unreadable(&error);
        assert_eq!(capability.report.tier, CapabilityTier::Snapshot);
        assert_eq!(capability.report.blind_spots.len(), 2);
        let evidence = capability
            .report
            .evidence
            .as_ref()
            .expect("the failure is accounted for");
        assert!(evidence.unreadable);
        assert_eq!(evidence.events, 0);
        assert!(
            capability
                .read_failure
                .as_deref()
                .is_some_and(|detail| detail == error.to_string()),
            "the reason a run gives must be the store's own, so it cannot be invented"
        );
        assert!(
            capability
                .report
                .summary()
                .contains("SURE could not read the events its store holds"),
            "{}",
            capability.report.summary()
        );
    }

    #[test]
    fn a_stored_event_that_is_not_an_envelope_is_a_read_failure_not_an_empty_project() {
        // A row whose document cannot be read back as an envelope is exactly the
        // case that must not be reported as "this project has no session": the
        // read got an answer it could not use, which is a different thing.
        let record = StoredRecord {
            id: 7,
            kind: RecordKind::Document(DocumentKind::Event),
            document_version: 1,
            written_at_ms: 0,
            project_root: Some("C:\\work\\mine".to_owned()),
            project_fingerprint: None,
            document: serde_json::json!({"not": "an event envelope"}),
        };
        let error = parse_event_records(&[record]).expect_err("a non-envelope is not an event");
        assert!(
            matches!(error, StoreError::MalformedRow { id: 7, .. }),
            "{error:?}"
        );
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

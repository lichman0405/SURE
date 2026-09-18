//! Codex-specific normalizer: raw Codex hook payloads → SURE event protocol.
//!
//! P12-T007: the Codex evidence bridge, where the public surface permits one.
//!
//! # The payload is Codex's, not SURE's
//!
//! [`claude_code`](super::claude_code) and [`cursor`](super::cursor) read
//! synthetic documents in the shape their own fixture directories hold. This
//! normalizer reads the object Codex really writes to a command hook's
//! standard input, as the official hooks page documented it on **2026-09-18**
//! (`learn.chatgpt.com/docs/hooks.md`, the redirect target of
//! `developers.openai.com/codex/hooks`): every command hook receives one JSON
//! object carrying `session_id`, `transcript_path`, `cwd`, `hook_event_name`
//! and `model`, with the event's own fields on top — `PreToolUse` adds
//! `tool_name`, `tool_use_id`, `tool_input` and `permission_mode`,
//! `PostToolUse` adds `tool_response` as well, `SessionStart` adds `source`,
//! and `SessionEnd` adds `reason`.
//!
//! Nothing here was captured from a running Codex: no Codex binary was run from
//! this repository, and the field names come from the page rather than from a
//! process. What *is* exercised is SURE's side of the bridge — these payloads
//! are real inputs to `sure hook ingest` and the tests below drive them through
//! the same normalise-validate-store path every other harness uses.
//!
//! # What the payload does not carry, and what SURE does instead
//!
//! **No Codex event carries a timestamp.** SURE's envelope requires one
//! (`schemas/event.schema.json`), and the rule next to that requirement is that
//! missing data is not invented (`docs/architecture/EVENT_PROTOCOL.md`). So the
//! instant SURE records is the moment SURE read the event, and every payload
//! built here says so: `"timestamp_source": "sure_ingest_clock"`. A reader can
//! tell an instant Codex reported from one SURE observed, and the second is
//! never presented as the first.
//!
//! # Which events map, and which do not
//!
//! Four of the twelve events the hooks page defines have an honest counterpart
//! in SURE's event model, and they are the four [`map_event_type`] accepts:
//! `SessionStart`, `PreToolUse`, `PostToolUse` and `SessionEnd`. The other
//! eight are refused by name, each with the reason SURE has nothing to record
//! them as. `Stop` is the one worth reading about: Codex ends *turns* with it,
//! not sessions, and SURE has no turn-end event type — recording it as
//! `session.stopped` would put a session end in the history that did not
//! happen. `integrations/codex/README.md` carries the full table.
//!
//! # What is deliberately strict
//!
//! `session_id` and `cwd` are required rather than defaulted. Both are fields
//! every Codex command hook receives, so their absence means the payload is not
//! the documented shape; and both have a wrong answer available that SURE
//! declines to give. Without a `session_id` the store cannot tie two events to
//! one session, so it would record a new session per event. Without a `cwd`
//! SURE would have to guess which project the evidence belongs to, and
//! evidence bound to the wrong project is worse than evidence SURE says it
//! does not have.

use serde::Deserialize;
use serde_json::Value;
use sure_domain::capability::CapabilityTier;
use sure_protocol::event::EventEnvelope;

use crate::diagnostics::Timestamp;

/// What SURE writes into `payload.timestamp_source`.
///
/// The name is the claim: this instant is when SURE read the event, not when
/// Codex says it happened. Codex sends no timestamp to report, and this
/// constant is how a reader can tell the difference without reading this file.
pub const TIMESTAMP_SOURCE: &str = "sure_ingest_clock";

/// A raw payload from the Codex hook surface.
///
/// The documented fields are here; anything Codex adds later is not refused, it
/// is simply not read. Only the two fields every command hook sends are
/// required — see the module documentation for why.
#[derive(Debug, Clone, Deserialize)]
pub struct CodexRawEvent {
    /// The event name, as Codex names it: `SessionStart`, `PreToolUse`, …
    pub hook_event_name: String,
    /// The harness's own session identifier. Required, not defaulted.
    #[serde(default)]
    pub session_id: Option<String>,
    /// The working directory the hook was run from. Treated as the project
    /// root; required, because SURE will not guess which project this is.
    #[serde(default)]
    pub cwd: Option<String>,
    /// Path to the session transcript, when Codex sends one.
    #[serde(default)]
    pub transcript_path: Option<String>,
    /// The model the session is using, when Codex sends one.
    #[serde(default)]
    pub model: Option<String>,
    /// The permission mode in effect, when Codex sends one.
    #[serde(default)]
    pub permission_mode: Option<String>,
    /// The turn this event belongs to.
    #[serde(default)]
    pub turn_id: Option<String>,
    /// The tool being requested or completed.
    #[serde(default)]
    pub tool_name: Option<String>,
    /// The harness's identifier for one tool call, which ties a `PreToolUse` to
    /// its `PostToolUse`.
    #[serde(default)]
    pub tool_use_id: Option<String>,
    /// What the tool was asked to do.
    #[serde(default)]
    pub tool_input: Option<Value>,
    /// What the tool answered. `PostToolUse` only.
    #[serde(default)]
    pub tool_response: Option<Value>,
    /// Why the session started. `SessionStart` only: startup, resume, clear or
    /// compact. Kept under `codex_source` because `source` is SURE's own field
    /// and the two are not the same thing.
    #[serde(default)]
    pub source: Option<String>,
    /// Why the session ended. `SessionEnd` only. Kept under `codex_reason` for
    /// the same reason `source` is kept under `codex_source`.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Why a Codex payload could not be normalized.
#[derive(Debug, Clone, PartialEq)]
pub enum CodexNormalizeError {
    /// The text was not valid JSON.
    NotJson {
        /// What the parser said.
        message: String,
    },
    /// Codex sends this event and SURE has no event type for what it means.
    ///
    /// Distinct from [`Self::UnknownEventType`] on purpose: this is the
    /// missing-parity case, and the message says what SURE did instead of
    /// guessing.
    NoMapping {
        /// The event name, as Codex sends it.
        event: String,
        /// One sentence on why SURE has nothing to record it as.
        reason: &'static str,
    },
    /// The event name is not one Codex is documented to produce.
    UnknownEventType {
        /// The event name that was read.
        event: String,
    },
    /// A field every Codex command hook sends was absent.
    MissingField {
        /// The field name.
        field: &'static str,
        /// One sentence on what SURE would have had to guess.
        reason: &'static str,
    },
}

impl std::fmt::Display for CodexNormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson { message } => {
                write!(f, "the Codex event is not valid JSON: {message}")
            }
            Self::NoMapping { event, reason } => write!(
                f,
                "Codex sends '{event}', and SURE has no event type for what it means: {reason}. \
                 Nothing was recorded, because SURE will not map an event onto a meaning the \
                 harness did not send."
            ),
            Self::UnknownEventType { event } => write!(
                f,
                "the Codex event type '{event}' is not one SURE knows how to map"
            ),
            Self::MissingField { field, reason } => write!(
                f,
                "the Codex event does not carry '{field}', which every Codex command hook sends, \
                 and {reason}. Nothing was recorded, because SURE will not fill in a field the \
                 harness did not send."
            ),
        }
    }
}

impl std::error::Error for CodexNormalizeError {}

/// Convert a raw Codex hook payload into a SURE [`EventEnvelope`].
///
/// # Capability tier
///
/// Codex is **Observed** (tier 1) through this bridge, and no higher. SURE
/// records what Codex says it did, after the fact; nothing here decides
/// anything before a tool runs. The `PreToolUse` payload could carry a
/// pre-action decision, and `sure hook ingest` does answer one for it, but
/// nothing in this repository has run a Codex hook and seen Codex act on the
/// answer, so Protected (tier 2) is not claimed.
///
/// # Errors
///
/// Returns [`CodexNormalizeError`] if the payload is not JSON, names an event
/// with no honest mapping, or is missing a field every Codex command hook
/// sends.
pub fn normalize(text: &str) -> Result<EventEnvelope, CodexNormalizeError> {
    let raw: CodexRawEvent =
        serde_json::from_str(text).map_err(|error| CodexNormalizeError::NotJson {
            message: error.to_string(),
        })?;

    let event_type = map_event_type(&raw.hook_event_name)?;

    let session_id = raw
        .session_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .ok_or(CodexNormalizeError::MissingField {
            field: "session_id",
            reason: "without it SURE cannot tie two events to one session, and would record one \
                     session per event",
        })?;

    let project_root = raw
        .cwd
        .as_deref()
        .filter(|root| !root.trim().is_empty())
        .ok_or(CodexNormalizeError::MissingField {
            field: "cwd",
            reason: "without it SURE would have to guess which project this evidence belongs \
                         to",
        })?;

    // The instant is SURE's own, and `TIMESTAMP_SOURCE` in the payload is what
    // says so. See the module documentation.
    let envelope = EventEnvelope::new("codex", event_type, Timestamp::now().to_string())
        .with_capability_tier(CapabilityTier::Observed)
        .with_session_id(session_id)
        .with_project_root(project_root.to_owned())
        .with_payload(build_payload(raw));

    Ok(envelope)
}

/// The SURE event type a Codex event name maps onto, when there is one.
///
/// The eight refusals are the deliverable as much as the four acceptances: each
/// names an event Codex really sends and says what SURE has no type for.
fn map_event_type(codex_event: &str) -> Result<&'static str, CodexNormalizeError> {
    match codex_event {
        "SessionStart" => Ok("session.started"),
        "PreToolUse" => Ok("tool.requested"),
        "PostToolUse" => Ok("tool.completed"),
        "SessionEnd" => Ok("session.stopped"),
        other => match unmapped_reason(other) {
            Some(reason) => Err(CodexNormalizeError::NoMapping {
                event: other.to_owned(),
                reason,
            }),
            None => Err(CodexNormalizeError::UnknownEventType {
                event: other.to_owned(),
            }),
        },
    }
}

/// Why an event Codex really sends has no SURE event type, or `None` if it is
/// not an event the hooks page defines.
///
/// The list is the hooks page's event list as read on 2026-09-18, minus the
/// four `map_event_type` accepts. It is a list of *refusals*, and adding an
/// entry means an event Codex sends that SURE will not record.
fn unmapped_reason(codex_event: &str) -> Option<&'static str> {
    match codex_event {
        "Stop" => Some(
            "Codex's Stop ends one turn rather than the session, so `session.stopped` would put a \
             session end in the history that did not happen, and the completion message it \
             carries would be an agent claim, which no SURE integration maps yet",
        ),
        "PermissionRequest" => Some(
            "it asks for approval of an action, and SURE's tool-request event is already recorded \
             from PreToolUse",
        ),
        "UserPromptSubmit" => Some(
            "SURE has no event type for the user's prompt, and neither of the other two \
             integrations records prompt text",
        ),
        "PreCompact" | "PostCompact" => Some("SURE has no event type for compaction"),
        "SubagentStart" | "SubagentStop" => {
            Some("SURE has no event type for a subagent's lifecycle")
        }
        "Interrupt" => Some("SURE has no event type for an interruption"),
        // Not an event the hooks page defines at all: a payload from something
        // else, or from a Codex whose event list SURE has not seen.
        _ => None,
    }
}

/// Build the payload SURE records for one Codex event.
///
/// Every field is copied from the payload under the name Codex gave it, with
/// two renames that exist to stop a collision with SURE's own vocabulary
/// (`codex_source`, `codex_reason`), one provenance field (`codex_event`) and
/// one honesty field (`timestamp_source`). Nothing is computed and nothing is
/// defaulted: a field Codex did not send is absent here, not empty.
fn build_payload(raw: CodexRawEvent) -> Value {
    let mut map = serde_json::Map::new();

    if let Some(tool) = raw.tool_name {
        map.insert("tool".to_owned(), Value::String(tool));
    }
    if let Some(input) = raw.tool_input {
        map.insert("args".to_owned(), input);
    }
    if let Some(response) = raw.tool_response {
        map.insert("result".to_owned(), response);
    }
    if let Some(model) = raw.model {
        map.insert("model".to_owned(), Value::String(model));
    }
    if let Some(mode) = raw.permission_mode {
        map.insert("permission_mode".to_owned(), Value::String(mode));
    }
    if let Some(turn_id) = raw.turn_id {
        map.insert("turn_id".to_owned(), Value::String(turn_id));
    }
    if let Some(tool_use_id) = raw.tool_use_id {
        map.insert("tool_use_id".to_owned(), Value::String(tool_use_id));
    }
    if let Some(path) = raw.transcript_path {
        map.insert("transcript_path".to_owned(), Value::String(path));
    }
    if let Some(source) = raw.source {
        map.insert("codex_source".to_owned(), Value::String(source));
    }
    if let Some(reason) = raw.reason {
        map.insert("codex_reason".to_owned(), Value::String(reason));
    }

    // Preserve the original event type for audit, and say where the instant
    // came from. Both are SURE's own annotations rather than Codex's fields.
    map.insert("codex_event".to_owned(), Value::String(raw.hook_event_name));
    map.insert(
        "timestamp_source".to_owned(),
        Value::String(TIMESTAMP_SOURCE.to_owned()),
    );

    Value::Object(map)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_testkit::repository_root;

    /// The payloads Codex is documented to send, as this package writes them.
    ///
    /// They are the package's, and the package's README says what that means:
    /// the field names come from the hooks page read on 2026-09-18 and the
    /// values are placeholders, because no Codex binary was run here.
    fn fixture(name: &str) -> String {
        let path = repository_root()
            .join("integrations")
            .join("codex")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    #[test]
    fn session_start_maps_to_session_started() {
        let text = fixture("session-start.json");
        let envelope = normalize(&text).expect("the fixture ingests");
        assert_eq!(envelope.event_type, "session.started");
        assert_eq!(envelope.source, "codex");
        assert_eq!(envelope.session_id.as_deref(), Some("codex-session-001"));
        assert_eq!(
            envelope.project_root.as_deref(),
            Some("C:\\Users\\dev\\sample-project")
        );
        assert_eq!(envelope.capability_tier, Some(CapabilityTier::Observed));
        assert_eq!(
            envelope.payload["codex_event"],
            Value::String("SessionStart".to_owned())
        );
        assert_eq!(
            envelope.payload["codex_source"],
            Value::String("startup".to_owned())
        );
    }

    #[test]
    fn pre_tool_use_maps_to_tool_requested() {
        let text = fixture("pre-tool-use.json");
        let envelope = normalize(&text).expect("the fixture ingests");
        assert_eq!(envelope.event_type, "tool.requested");
        assert_eq!(envelope.payload["tool"], Value::String("Bash".to_owned()));
        assert_eq!(
            envelope.payload["args"]["command"],
            Value::String("npm test".to_owned())
        );
        // The harness's own identifier for one tool call, so a request can be
        // tied to the completion that carries the same id.
        assert_eq!(
            envelope.payload["tool_use_id"],
            Value::String("tool-use-001".to_owned())
        );
    }

    #[test]
    fn post_tool_use_maps_to_tool_completed_and_keeps_the_response_verbatim() {
        let text = fixture("post-tool-use.json");
        let envelope = normalize(&text).expect("the fixture ingests");
        assert_eq!(envelope.event_type, "tool.completed");
        assert_eq!(
            envelope.payload["result"]["exit_code"],
            Value::Number(1.into())
        );
        // SURE does not read an error out of `tool_response`. Codex has no
        // tool-failure event and does not say which responses failed, so a
        // `tool.failed` here would be SURE's guess standing in for Codex's
        // silence.
        assert!(
            !envelope.event_type.contains("failed"),
            "a PostToolUse was classified as a failure SURE was never told about"
        );
    }

    #[test]
    fn session_end_maps_to_session_stopped() {
        let text = fixture("session-end.json");
        let envelope = normalize(&text).expect("the fixture ingests");
        assert_eq!(envelope.event_type, "session.stopped");
        assert_eq!(
            envelope.payload["codex_reason"],
            Value::String("other".to_owned())
        );
    }

    #[test]
    fn stop_is_refused_because_sure_has_no_turn_end_event() {
        // The acceptance's other half, as a test: an event Codex really sends
        // that SURE refuses rather than maps onto the nearest thing it has.
        let text = fixture("stop-not-mapped.json");
        let error = normalize(&text).unwrap_err();
        match &error {
            CodexNormalizeError::NoMapping { event, .. } => assert_eq!(event, "Stop"),
            other => panic!("expected NoMapping for Stop, got {other:?}"),
        }
        let message = error.to_string();
        assert!(message.contains("turn"), "{message}");
        // The message names the second reason too, which is the one a reader
        // will care about: Codex's Stop carries the agent's completion message,
        // which is the claim SURE's `agent.claim` type exists for and which no
        // integration maps yet.
        assert!(message.contains("agent claim"), "{message}");
        assert!(message.contains("Nothing was recorded"), "{message}");
    }

    #[test]
    fn every_unmapped_codex_event_is_refused_by_name() {
        // The list is the hooks page's event list minus the four that map. A
        // new event reaching Codex must be refused here rather than silently
        // treated as an unknown string, so this names each one.
        for event in [
            "Stop",
            "PermissionRequest",
            "UserPromptSubmit",
            "PreCompact",
            "PostCompact",
            "SubagentStart",
            "SubagentStop",
            "Interrupt",
        ] {
            let text = serde_json::json!({
                "hook_event_name": event,
                "session_id": "codex-session-001",
                "cwd": "C:\\Users\\dev\\sample-project",
            })
            .to_string();
            let error = normalize(&text).unwrap_err();
            assert!(
                matches!(&error, CodexNormalizeError::NoMapping { event: found, .. } if found == event),
                "{event} was not refused as an event with no mapping: {error:?}"
            );
        }
    }

    #[test]
    fn an_event_codex_does_not_send_is_a_different_refusal() {
        let text = serde_json::json!({
            "hook_event_name": "TotallyMadeUp",
            "session_id": "codex-session-001",
            "cwd": "C:\\Users\\dev\\sample-project",
        })
        .to_string();
        let error = normalize(&text).unwrap_err();
        assert!(
            matches!(error, CodexNormalizeError::UnknownEventType { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("TotallyMadeUp"));
    }

    #[test]
    fn a_payload_without_a_session_id_is_refused_rather_than_defaulted() {
        let text = serde_json::json!({
            "hook_event_name": "SessionStart",
            "cwd": "C:\\Users\\dev\\sample-project",
        })
        .to_string();
        let error = normalize(&text).unwrap_err();
        match &error {
            CodexNormalizeError::MissingField { field, .. } => assert_eq!(*field, "session_id"),
            other => panic!("expected MissingField, got {other:?}"),
        }
        assert!(error.to_string().contains("one session per event"));
    }

    #[test]
    fn a_payload_without_a_cwd_is_refused_rather_than_guessed() {
        let text = serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": "codex-session-001",
        })
        .to_string();
        let error = normalize(&text).unwrap_err();
        match &error {
            CodexNormalizeError::MissingField { field, .. } => assert_eq!(*field, "cwd"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn the_recorded_instant_says_it_was_sures_clock() {
        // Codex sends no timestamp. The envelope needs one, so SURE records
        // when it read the event — and the payload has to say that, or a
        // reader would take SURE's clock for the harness's.
        let text = fixture("session-start.json");
        let envelope = normalize(&text).expect("the fixture ingests");
        assert_eq!(
            envelope.payload["timestamp_source"],
            Value::String(TIMESTAMP_SOURCE.to_owned())
        );
        assert!(
            Timestamp::parse_rfc3339(&envelope.timestamp).is_some(),
            "the envelope timestamp is not an RFC 3339 instant: {}",
            envelope.timestamp
        );
    }

    #[test]
    fn invalid_json_is_rejected() {
        let error = normalize("{not json").unwrap_err();
        assert!(
            matches!(error, CodexNormalizeError::NotJson { .. }),
            "{error:?}"
        );
    }

    #[test]
    fn every_mapped_fixture_ingests_through_the_protocol_path() {
        // The normaliser and the ingestion step are two different validators,
        // and a payload that satisfies one can still be refused by the other.
        for name in [
            "session-start.json",
            "pre-tool-use.json",
            "post-tool-use.json",
            "session-end.json",
        ] {
            let text = fixture(name);
            let envelope = normalize(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let json = envelope.to_json().unwrap_or_else(|e| panic!("{name}: {e}"));
            crate::harness_event::ingest_event_str(&json)
                .unwrap_or_else(|e| panic!("{name} ingestion: {e}"));
        }
    }

    #[test]
    fn non_ascii_text_in_the_payload_survives_the_protocol_path() {
        // A session's paths and tool output are whatever the user's machine
        // holds, so the text has to come out of normalise-validate-store the way
        // it went in. This is SURE's half of it: the launcher's half is
        // `$OutputEncoding` in `integrations/codex/scripts/sure-hook.ps1`, set
        // to UTF-8 because Windows PowerShell 5.1 pipes ASCII by default.
        let project_root = "C:\\Users\\dev\\\u{9879}\u{76ee}\u{2014}caf\u{e9}";
        let command = "echo '\u{5df2}\u{5b8c}\u{6210} \u{2014} 100%'";
        let payload = serde_json::json!({
            "hook_event_name": "PostToolUse",
            "session_id": "codex-session-unicode",
            "cwd": project_root,
            "tool_name": "Bash",
            "tool_input": { "command": command },
            "tool_response": { "stdout": "na\u{ef}ve \u{2713}" }
        })
        .to_string();

        let envelope = normalize(&payload).expect("non-ASCII payload normalises");
        let json = envelope.to_json().expect("serialises");
        let ingested = crate::harness_event::ingest_event_str(&json).expect("the payload ingests");

        assert_eq!(
            ingested.envelope.project_root.as_deref(),
            Some(project_root),
            "the project root must keep its non-ASCII characters"
        );
        assert_eq!(
            ingested.envelope.payload["args"]["command"].as_str(),
            Some(command),
            "the tool arguments must keep their non-ASCII characters"
        );
        assert_eq!(
            ingested.envelope.payload["result"]["stdout"].as_str(),
            Some("na\u{ef}ve \u{2713}"),
            "the tool response must keep its non-ASCII characters"
        );
    }
}

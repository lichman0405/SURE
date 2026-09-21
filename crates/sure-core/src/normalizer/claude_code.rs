//! Claude Code-specific normalizer: raw Claude Code events → SURE event protocol.
//!
//! P10-T003: Claude session evidence ingestion.
//!
//! # Capability tier
//!
//! Claude Code is **Observed** (tier 1). The hook manifest wires `PreToolUse`,
//! but the core protection response path is not yet implemented for Claude Code
//! (P10-T006), so the integration cannot honestly claim Protected (tier 2).

use serde::Deserialize;
use serde_json::Value;
use sure_domain::capability::CapabilityTier;
use sure_protocol::event::EventEnvelope;

/// A raw event from the Claude Code harness.
///
/// Shape matches the fixtures in `integrations/claude-code/fixtures/`.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeCodeRawEvent {
    pub event: String,
    pub harness_session_id: String,
    pub timestamp_utc: String,
    pub source: String,
    #[serde(default)]
    pub project_root: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub args: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<Value>,
    #[serde(default)]
    pub path: Option<String>,
}

/// Why a Claude Code raw event could not be normalized.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeCodeNormalizeError {
    /// The text was not valid JSON.
    NotJson { message: String },
    /// The event type is not one Claude Code is known to produce.
    UnknownEventType { event: String },
    /// The source field is not "claude-code".
    WrongSource { source: String },
}

impl std::fmt::Display for ClaudeCodeNormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson { message } => {
                write!(f, "the Claude Code event is not valid JSON: {message}")
            }
            Self::UnknownEventType { event } => write!(
                f,
                "the Claude Code event type '{event}' is not one SURE knows how to map"
            ),
            Self::WrongSource { source } => write!(
                f,
                "the Claude Code event declares source '{source}', but 'claude-code' was expected"
            ),
        }
    }
}

impl std::error::Error for ClaudeCodeNormalizeError {}

/// Convert a raw Claude Code JSON event into a SURE [`EventEnvelope`].
///
/// # Capability tier
///
/// Claude Code is **Observed** (tier 1). The `PreToolUse` event is wired in the
/// hook manifest, but the core protection response path is not yet implemented
/// for Claude Code (P10-T006), so the integration cannot honestly claim
/// Protected (tier 2).
pub fn normalize(text: &str) -> Result<EventEnvelope, ClaudeCodeNormalizeError> {
    let raw: ClaudeCodeRawEvent =
        serde_json::from_str(text).map_err(|error| ClaudeCodeNormalizeError::NotJson {
            message: error.to_string(),
        })?;

    if raw.source != "claude-code" {
        return Err(ClaudeCodeNormalizeError::WrongSource { source: raw.source });
    }

    let event_type = map_event_type(&raw.event)?;
    let mut envelope = EventEnvelope::new("claude-code", event_type, &raw.timestamp_utc)
        .with_capability_tier(CapabilityTier::Observed)
        .with_session_id(&raw.harness_session_id);

    if let Some(ref project_root) = raw.project_root {
        envelope = envelope.with_project_root(project_root.clone());
    }

    let payload = build_payload(raw);
    envelope = envelope.with_payload(payload);

    Ok(envelope)
}

fn map_event_type(claude_event: &str) -> Result<&'static str, ClaudeCodeNormalizeError> {
    match claude_event {
        "SessionStart" => Ok("session.started"),
        "PreToolUse" => Ok("tool.requested"),
        "PostToolUse" => Ok("tool.completed"),
        "Stop" => Ok("session.stopped"),
        other => Err(ClaudeCodeNormalizeError::UnknownEventType {
            event: other.to_owned(),
        }),
    }
}

fn build_payload(raw: ClaudeCodeRawEvent) -> Value {
    let mut map = serde_json::Map::new();

    if let Some(tool) = raw.tool {
        map.insert("tool".to_owned(), Value::String(tool));
    }
    if let Some(args) = raw.args {
        map.insert("args".to_owned(), args);
    }
    if let Some(result) = raw.result {
        map.insert("result".to_owned(), result);
    }
    if let Some(error) = raw.error {
        map.insert("error".to_owned(), error);
    }
    if let Some(path) = raw.path {
        map.insert("path".to_owned(), Value::String(path));
    }

    // Preserve the original event type for audit.
    map.insert("claude_event".to_owned(), Value::String(raw.event));

    Value::Object(map)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_testkit::repository_root;

    fn fixture(name: &str) -> String {
        let path = repository_root()
            .join("integrations")
            .join("claude-code")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    #[test]
    fn session_start_maps_correctly() {
        let text = fixture("session-start.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "session.started");
        assert_eq!(envelope.source, "claude-code");
        assert_eq!(envelope.session_id.as_deref(), Some("claude-session-001"));
        assert_eq!(
            envelope.project_root.as_deref(),
            Some("C:\\Users\\dev\\sample-project")
        );
        assert_eq!(envelope.timestamp, "2026-09-18T12:00:00Z");
        assert_eq!(envelope.capability_tier, Some(CapabilityTier::Observed));
        assert_eq!(
            envelope.payload["claude_event"],
            Value::String("SessionStart".to_owned())
        );
    }

    #[test]
    fn pre_tool_use_maps_correctly() {
        let text = fixture("pre-tool-use.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "tool.requested");
        assert_eq!(envelope.payload["tool"], Value::String("Bash".to_owned()));
        assert_eq!(
            envelope.payload["args"]["command"],
            Value::String("npm test".to_owned())
        );
    }

    #[test]
    fn post_tool_use_maps_correctly() {
        let text = fixture("post-tool-use.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "tool.completed");
        assert_eq!(
            envelope.payload["result"]["exit_code"],
            Value::Number(0.into())
        );
    }

    #[test]
    fn stop_maps_correctly() {
        let text = fixture("stop.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "session.stopped");
    }

    #[test]
    fn post_tool_failure_maps_correctly() {
        let text = fixture("post-tool-failure.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "tool.completed");
        assert_eq!(
            envelope.payload["error"]["message"],
            Value::String("command not found".to_owned())
        );
    }

    #[test]
    fn unknown_event_type_is_rejected() {
        let text = r#"{"event":"UnknownThing","harness_session_id":"x","timestamp_utc":"2026-09-18T12:00:00Z","source":"claude-code"}"#;
        let err = normalize(text).unwrap_err();
        assert!(matches!(
            err,
            ClaudeCodeNormalizeError::UnknownEventType { .. }
        ));
        assert!(err.to_string().contains("UnknownThing"));
    }

    #[test]
    fn wrong_source_is_rejected() {
        let text = r#"{"event":"SessionStart","harness_session_id":"x","timestamp_utc":"2026-09-18T12:00:00Z","source":"not-claude"}"#;
        let err = normalize(text).unwrap_err();
        assert!(matches!(err, ClaudeCodeNormalizeError::WrongSource { .. }));
    }

    #[test]
    fn invalid_json_is_rejected() {
        let err = normalize("{not json").unwrap_err();
        assert!(matches!(err, ClaudeCodeNormalizeError::NotJson { .. }));
    }

    #[test]
    fn normalized_envelope_passes_ingestion() {
        let text = fixture("session-start.json");
        let envelope = normalize(&text).expect("valid fixture");
        let json = envelope.to_json().expect("serializes");
        let ingested = crate::harness_event::ingest_event_str(&json).expect("ingests");
        assert_eq!(ingested.envelope.event_type, "session.started");
        assert_eq!(ingested.envelope.source, "claude-code");
    }

    #[test]
    fn all_fixture_events_ingest_successfully() {
        for name in [
            "session-start.json",
            "pre-tool-use.json",
            "post-tool-use.json",
            "post-tool-failure.json",
            "stop.json",
        ] {
            let text = fixture(name);
            let envelope = normalize(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let json = envelope.to_json().unwrap_or_else(|e| panic!("{name}: {e}"));
            crate::harness_event::ingest_event_str(&json)
                .unwrap_or_else(|e| panic!("{name} ingestion: {e}"));
        }
    }
}

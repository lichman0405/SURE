//! Cursor-specific normalizer: raw Cursor events → SURE event protocol.
//!
//! P11-T004: Cursor session evidence ingestion.

use serde::Deserialize;
use serde_json::Value;
use sure_domain::capability::CapabilityTier;
use sure_protocol::event::EventEnvelope;

/// A raw event from the Cursor harness.
///
/// Shape matches the fixtures in `integrations/cursor/fixtures/`.
#[derive(Debug, Clone, Deserialize)]
pub struct CursorRawEvent {
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

/// Why a Cursor raw event could not be normalized.
#[derive(Debug, Clone, PartialEq)]
pub enum CursorNormalizeError {
    /// The text was not valid JSON.
    NotJson { message: String },
    /// The event type is not one Cursor is known to produce.
    UnknownEventType { event: String },
    /// The source field is not "cursor".
    WrongSource { source: String },
}

impl std::fmt::Display for CursorNormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson { message } => {
                write!(f, "the Cursor event is not valid JSON: {message}")
            }
            Self::UnknownEventType { event } => write!(
                f,
                "the Cursor event type '{event}' is not one SURE knows how to map"
            ),
            Self::WrongSource { source } => write!(
                f,
                "the Cursor event declares source '{source}', but 'cursor' was expected"
            ),
        }
    }
}

impl std::error::Error for CursorNormalizeError {}

/// Convert a raw Cursor JSON event into a SURE [`EventEnvelope`].
///
/// # Capability tier
///
/// Cursor is **Observed** (tier 1). The `preToolUse` event is wired in the hook
/// manifest and SURE now evaluates a protection decision (P11-T006), but the
/// manifest does not confirm that Cursor interprets the response, so the
/// integration cannot honestly claim Protected (tier 2). Protection remains
/// advisory: the decision is returned, but the harness may or may not act on it.
pub fn normalize(text: &str) -> Result<EventEnvelope, CursorNormalizeError> {
    let raw: CursorRawEvent =
        serde_json::from_str(text).map_err(|error| CursorNormalizeError::NotJson {
            message: error.to_string(),
        })?;

    if raw.source != "cursor" {
        return Err(CursorNormalizeError::WrongSource { source: raw.source });
    }

    let event_type = map_event_type(&raw.event)?;
    let mut envelope = EventEnvelope::new("cursor", event_type, &raw.timestamp_utc)
        .with_capability_tier(CapabilityTier::Observed)
        .with_session_id(&raw.harness_session_id);

    if let Some(ref project_root) = raw.project_root {
        envelope = envelope.with_project_root(project_root.clone());
    }

    let payload = build_payload(raw);
    envelope = envelope.with_payload(payload);

    Ok(envelope)
}

fn map_event_type(cursor_event: &str) -> Result<&'static str, CursorNormalizeError> {
    match cursor_event {
        "sessionStart" => Ok("session.started"),
        "preToolUse" => Ok("tool.requested"),
        "postToolUse" => Ok("tool.completed"),
        "postToolUseFailure" => Ok("tool.failed"),
        "afterFileEdit" => Ok("file.edited"),
        "stop" => Ok("session.stopped"),
        other => Err(CursorNormalizeError::UnknownEventType {
            event: other.to_owned(),
        }),
    }
}

fn build_payload(raw: CursorRawEvent) -> Value {
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
    map.insert("cursor_event".to_owned(), Value::String(raw.event));

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
            .join("cursor")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    #[test]
    fn session_start_maps_correctly() {
        let text = fixture("session-start.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "session.started");
        assert_eq!(envelope.source, "cursor");
        assert_eq!(envelope.session_id.as_deref(), Some("cursor-session-001"));
        assert_eq!(
            envelope.project_root.as_deref(),
            Some("C:\\Users\\dev\\sample-project")
        );
        assert_eq!(envelope.timestamp, "2026-09-18T12:00:00Z");
        assert_eq!(envelope.capability_tier, Some(CapabilityTier::Observed));
        assert_eq!(
            envelope.payload["cursor_event"],
            Value::String("sessionStart".to_owned())
        );
    }

    #[test]
    fn pre_tool_use_maps_correctly() {
        let text = fixture("pre-tool-use.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "tool.requested");
        assert_eq!(envelope.payload["tool"], Value::String("Shell".to_owned()));
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
    fn post_tool_failure_maps_correctly() {
        let text = fixture("post-tool-failure.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "tool.failed");
        assert_eq!(
            envelope.payload["error"]["message"],
            Value::String("deployment target unreachable".to_owned())
        );
    }

    #[test]
    fn after_file_edit_maps_correctly() {
        let text = fixture("after-file-edit.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "file.edited");
        assert_eq!(
            envelope.payload["path"],
            Value::String("C:\\Users\\dev\\sample-project\\src\\email\\send.rs".to_owned())
        );
    }

    #[test]
    fn stop_maps_correctly() {
        let text = fixture("stop.json");
        let envelope = normalize(&text).expect("valid fixture");
        assert_eq!(envelope.event_type, "session.stopped");
    }

    #[test]
    fn unknown_event_type_is_rejected() {
        let text = r#"{"event":"unknownThing","harness_session_id":"x","timestamp_utc":"2026-09-18T12:00:00Z","source":"cursor"}"#;
        let err = normalize(text).unwrap_err();
        assert!(matches!(err, CursorNormalizeError::UnknownEventType { .. }));
        assert!(err.to_string().contains("unknownThing"));
    }

    #[test]
    fn wrong_source_is_rejected() {
        let text = r#"{"event":"sessionStart","harness_session_id":"x","timestamp_utc":"2026-09-18T12:00:00Z","source":"not-cursor"}"#;
        let err = normalize(text).unwrap_err();
        assert!(matches!(err, CursorNormalizeError::WrongSource { .. }));
    }

    #[test]
    fn invalid_json_is_rejected() {
        let err = normalize("{not json").unwrap_err();
        assert!(matches!(err, CursorNormalizeError::NotJson { .. }));
    }

    #[test]
    fn normalized_envelope_passes_ingestion() {
        let text = fixture("session-start.json");
        let envelope = normalize(&text).expect("valid fixture");
        let json = envelope.to_json().expect("serializes");
        let ingested = crate::harness_event::ingest_event_str(&json).expect("ingests");
        assert_eq!(ingested.envelope.event_type, "session.started");
        assert_eq!(ingested.envelope.source, "cursor");
    }

    #[test]
    fn all_fixture_events_ingest_successfully() {
        for name in [
            "session-start.json",
            "pre-tool-use.json",
            "post-tool-use.json",
            "post-tool-failure.json",
            "after-file-edit.json",
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

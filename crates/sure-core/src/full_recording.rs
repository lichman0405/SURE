//! Full recording opt-in projection.
//!
//! The full recording projection retains raw transcript and terminal payload
//! content, but only when the user has explicitly opted in. This is distinct
//! from the privacy-conscious [`StandardProjection`](crate::recording_projection::StandardProjection),
//! which never stores raw content.
//!
//! Full recordings are stored as [`RecordKind::Recording`] with an explicit
//! `full_recording` marker and a short retention period (3 days by default).

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sure_domain::ids::{EventId, FingerprintId};

use crate::harness_event::IngestedEvent;
use crate::redact;
use crate::store::{HistoryFilter, RecordKind, Store, StoreError, StoredRecord};

/// Default retention for full recordings: 3 days.
pub const DEFAULT_FULL_RECORDING_RETENTION_DAYS: i64 = 3;

/// Whether the user has consented to full recording retention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FullRecordingConsent {
    /// Default: no raw transcript is retained.
    ProjectionOnly,
    /// User explicitly opted in; raw transcript/terminal payload may be retained.
    Full,
}

/// Why a full recording could not be persisted.
#[derive(Debug, Clone, PartialEq)]
pub enum FullRecordingError {
    /// The underlying store reported an error.
    Store {
        /// What the store said.
        message: String,
    },
    /// The recording document could not be serialized.
    Serialize {
        /// What serde said.
        message: String,
    },
    /// Full recording was requested without explicit opt-in.
    NotOptedIn,
}

impl std::fmt::Display for FullRecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store { message } => {
                write!(
                    f,
                    "SURE could not store the full recording.\n\n{}\n\n\
                     Nothing was written.",
                    redact::escape_control_characters(message)
                )
            }
            Self::Serialize { message } => {
                write!(
                    f,
                    "SURE could not turn the full recording into JSON: {}\n\n\
                     Nothing was written.",
                    redact::escape_control_characters(message)
                )
            }
            Self::NotOptedIn => {
                write!(
                    f,
                    "Full recording storage requires explicit opt-in.\n\n\
                     Set the recording consent level to Full before calling this function.\n\n\
                     Nothing was written."
                )
            }
        }
    }
}

impl std::error::Error for FullRecordingError {}

impl From<StoreError> for FullRecordingError {
    fn from(error: StoreError) -> Self {
        Self::Store {
            message: error.to_string(),
        }
    }
}

/// A full recording as read back from the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFullRecording {
    /// The row id in `records`.
    pub record_id: i64,
    /// When it was written.
    pub written_at_ms: i64,
    /// The envelope event type.
    pub event_type: String,
    /// The envelope timestamp.
    pub timestamp: String,
    /// The harness session id, if any.
    pub session_id: Option<String>,
    /// The SURE event id, if any.
    pub event_id: Option<String>,
    /// The harness source.
    pub source: String,
    /// The redacted payload that was stored.
    pub payload: Value,
    /// When this recording should be discarded, milliseconds since epoch.
    pub retained_until_ms: i64,
}

/// Persist a full recording for a project.
///
/// # Errors
///
/// Returns [`FullRecordingError::NotOptedIn`] if `consent` is
/// [`FullRecordingConsent::ProjectionOnly`].
///
/// Returns [`FullRecordingError::Store`] or [`FullRecordingError::Serialize`]
/// if the recording cannot be persisted.
pub fn persist_full_recording(
    store: &Store,
    event: &IngestedEvent,
    event_id: &EventId,
    project_root: &str,
    fingerprint: &FingerprintId,
    consent: FullRecordingConsent,
) -> Result<i64, FullRecordingError> {
    if consent == FullRecordingConsent::ProjectionOnly {
        return Err(FullRecordingError::NotOptedIn);
    }

    let now_ms = now_ms();
    let retained_until_ms = retention_deadline_ms(now_ms, DEFAULT_FULL_RECORDING_RETENTION_DAYS);

    // Redact the payload before storage, even under full opt-in.
    let redacted_payload = crate::store::redact_document(&event.envelope.payload);

    let wrapper = json!({
        "full_recording": true,
        "consent": "full",
        "retained_until_ms": retained_until_ms,
        "event_type": event.envelope.event_type,
        "timestamp": event.envelope.timestamp,
        "session_id": event.envelope.session_id,
        "event_id": event_id.as_str(),
        "source": event.envelope.source,
        "payload": redacted_payload,
    });

    let id = store
        .append_for(RecordKind::Recording, &wrapper, project_root, fingerprint)
        .map_err(FullRecordingError::from)?;
    Ok(id)
}

/// Read back every full recording for a project, newest first.
///
/// # Errors
///
/// Returns [`FullRecordingError::Store`] if the query fails.
pub fn full_recordings_for_project(
    store: &Store,
    fingerprint: &FingerprintId,
    limit: usize,
) -> Result<Vec<StoredFullRecording>, FullRecordingError> {
    let filter = HistoryFilter {
        project_fingerprint: Some(fingerprint),
        kind: Some(RecordKind::Recording),
        include_recordings: true,
    };

    let records = store
        .history(&filter, limit)
        .map_err(FullRecordingError::from)?;

    let mut results = Vec::new();
    for record in records {
        if let Some(sfr) = extract_full_recording(&record) {
            results.push(sfr);
        }
    }
    Ok(results)
}

/// List full recordings whose retention has expired.
///
/// # Errors
///
/// Returns [`FullRecordingError::Store`] if the query fails.
pub fn full_recordings_past_retention(
    store: &Store,
    fingerprint: &FingerprintId,
    before_ms: i64,
) -> Result<Vec<StoredFullRecording>, FullRecordingError> {
    let all = full_recordings_for_project(store, fingerprint, 10_000)?;
    Ok(all
        .into_iter()
        .filter(|sfr| sfr.retained_until_ms < before_ms)
        .collect())
}

/// Determine whether a [`StoredRecord`] is a full recording (as opposed to a
/// standard projection or other recording kind).
#[must_use]
pub fn is_full_recording(record: &StoredRecord) -> bool {
    record
        .document
        .get("full_recording")
        .and_then(Value::as_bool)
        == Some(true)
}

fn extract_full_recording(record: &StoredRecord) -> Option<StoredFullRecording> {
    if !is_full_recording(record) {
        return None;
    }

    let doc = &record.document;
    let event_type = doc.get("event_type")?.as_str()?.to_owned();
    let timestamp = doc.get("timestamp")?.as_str()?.to_owned();
    let session_id = doc
        .get("session_id")
        .and_then(Value::as_str)
        .map(|s| s.to_owned());
    let event_id = doc
        .get("event_id")
        .and_then(Value::as_str)
        .map(|s| s.to_owned());
    let source = doc.get("source")?.as_str()?.to_owned();
    let payload = doc.get("payload")?.clone();
    let retained_until_ms = doc.get("retained_until_ms")?.as_i64()?;

    Some(StoredFullRecording {
        record_id: record.id,
        written_at_ms: record.written_at_ms,
        event_type,
        timestamp,
        session_id,
        event_id,
        source,
        payload,
        retained_until_ms,
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Compute a retention deadline from a start time and retention days.
#[must_use]
pub fn retention_deadline_ms(start_ms: i64, retention_days: i64) -> i64 {
    start_ms.saturating_add(retention_days.saturating_mul(86_400_000))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;
    use sure_domain::capability::CapabilityTier;
    use sure_protocol::PROTOCOL_VERSION;
    use sure_protocol::event::EventEnvelope;

    use crate::harness_event::IngestedEvent;
    use crate::store::{HistoryFilter, Store};

    fn scratch(name: &str) -> std::path::PathBuf {
        crate::store::scratch_root().join(format!("{name}-{}", std::process::id()))
    }

    fn store_in(name: &str) -> Store {
        let dir = scratch(name);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("cannot clear {}: {e}", dir.display()),
        }
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Store::open_at(&dir.join("sure.db")).expect("the store opens")
    }

    fn make_event(event_type: &str, payload: Value) -> IngestedEvent {
        IngestedEvent {
            envelope: EventEnvelope::new("claude-code", event_type, "2026-09-14T09:10:56.827Z")
                .with_capability_tier(CapabilityTier::Observed)
                .with_session_id("session-42")
                .with_project_root("C:\\work\\my project")
                .with_payload(payload),
            protocol_version: PROTOCOL_VERSION,
            document_kind: sure_protocol::documents::DocumentKind::Event,
        }
    }

    #[test]
    fn full_recording_is_stored_when_opt_in_is_full() {
        let store = store_in("full_recording_opted_in");
        let event = make_event(
            "tool.completed",
            json!({
                "tool": "Bash",
                "output": "hello world"
            }),
        );
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        let id = persist_full_recording(
            &store,
            &event,
            &event_id,
            "C:\\work\\my project",
            &fingerprint,
            FullRecordingConsent::Full,
        )
        .expect("persist succeeds");
        assert!(id > 0);

        let back = full_recordings_for_project(&store, &fingerprint, 10).expect("query succeeds");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].event_type, "tool.completed");
        assert_eq!(back[0].source, "claude-code");
        assert_eq!(back[0].session_id, Some("session-42".to_owned()));
        assert_eq!(back[0].event_id, Some(event_id.as_str().to_owned()));
        assert_eq!(
            back[0].payload,
            json!({"tool": "Bash", "output": "hello world"})
        );
    }

    #[test]
    fn full_recording_is_refused_when_opt_in_is_projection_only() {
        let store = store_in("full_recording_refused");
        let event = make_event("tool.completed", json!({"tool": "Bash"}));
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        let err = persist_full_recording(
            &store,
            &event,
            &event_id,
            "C:\\work\\my project",
            &fingerprint,
            FullRecordingConsent::ProjectionOnly,
        )
        .unwrap_err();

        assert!(matches!(err, FullRecordingError::NotOptedIn), "{err:?}");
        let text = err.to_string();
        assert!(
            text.contains("explicit opt-in"),
            "error should explain opt-in: {text}"
        );
        assert!(
            text.contains("Nothing was written"),
            "error should say nothing was written: {text}"
        );

        // Verify nothing was stored.
        let back = full_recordings_for_project(&store, &fingerprint, 10).unwrap();
        assert!(back.is_empty());
    }

    #[test]
    fn stored_full_recording_is_distinguishable_from_standard_projection() {
        let store = store_in("full_recording_distinguishable");
        let event = make_event("tool.completed", json!({"tool": "Bash"}));
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        persist_full_recording(
            &store,
            &event,
            &event_id,
            "C:\\work\\my project",
            &fingerprint,
            FullRecordingConsent::Full,
        )
        .unwrap();

        // Also store a standard projection for the same project.
        use crate::recording_projection::{StandardProjection, ToolStatus, persist_projection};
        let projection = StandardProjection::Tool {
            name: "Bash".to_owned(),
            status: ToolStatus::Completed,
            duration_ms: None,
            summary: "ran tests".to_owned(),
        };
        persist_projection(&store, &projection, "C:\\work\\my project", &fingerprint).unwrap();

        // Read all recordings.
        let filter = HistoryFilter {
            project_fingerprint: Some(&fingerprint),
            kind: Some(RecordKind::Recording),
            include_recordings: true,
        };
        let records = store.history(&filter, 10).unwrap();
        assert_eq!(records.len(), 2);

        let full_count = records.iter().filter(|r| is_full_recording(r)).count();
        let projection_count = records.iter().filter(|r| !is_full_recording(r)).count();
        assert_eq!(full_count, 1);
        assert_eq!(projection_count, 1);
    }

    #[test]
    fn raw_payload_secrets_are_redacted_before_storage() {
        let store = store_in("full_recording_redacted");
        let event = make_event(
            "tool.completed",
            json!({
                "command": "curl https://user:secret@example.com",
                "env": {"API_KEY": "supersecret"},
                "token": "bearer abc123xyz"
            }),
        );
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        persist_full_recording(
            &store,
            &event,
            &event_id,
            "C:\\work\\my project",
            &fingerprint,
            FullRecordingConsent::Full,
        )
        .unwrap();

        let back = full_recordings_for_project(&store, &fingerprint, 10).unwrap();
        assert_eq!(back.len(), 1);
        let payload_text = back[0].payload.to_string();
        assert!(
            !payload_text.contains("user:secret@example.com"),
            "stored payload still contained URL credentials: {payload_text}"
        );
        assert!(
            !payload_text.contains("abc123xyz"),
            "stored payload still contained token value: {payload_text}"
        );
        assert!(
            payload_text.contains("REDACTED") || payload_text.contains("***"),
            "redaction marker missing from payload: {payload_text}"
        );
    }

    #[test]
    fn retention_metadata_is_recorded() {
        let store = store_in("full_recording_retention");
        let event = make_event("tool.completed", json!({"tool": "Bash"}));
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        persist_full_recording(
            &store,
            &event,
            &event_id,
            "C:\\work\\my project",
            &fingerprint,
            FullRecordingConsent::Full,
        )
        .unwrap();

        let back = full_recordings_for_project(&store, &fingerprint, 10).unwrap();
        assert_eq!(back.len(), 1);
        assert!(
            back[0].retained_until_ms > 0,
            "retention deadline should be positive"
        );

        // 3 days = 259_200_000 ms; allow one minute of tolerance for clock drift.
        let expected_min = now_ms() + 259_200_000 - 60_000;
        let expected_max = now_ms() + 259_200_000 + 60_000;
        assert!(
            back[0].retained_until_ms >= expected_min && back[0].retained_until_ms <= expected_max,
            "retention should be roughly 3 days from now"
        );
    }

    #[test]
    fn full_recordings_are_excluded_from_default_history() {
        let store = store_in("full_recording_hidden");
        let event = make_event("tool.completed", json!({"tool": "Bash"}));
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        persist_full_recording(
            &store,
            &event,
            &event_id,
            "C:\\work\\my project",
            &fingerprint,
            FullRecordingConsent::Full,
        )
        .unwrap();

        // Default history excludes recordings.
        let default = store
            .history(&HistoryFilter::for_project(&fingerprint), 10)
            .unwrap();
        assert!(default.is_empty());

        // Including recordings reveals the full recording.
        let with_recordings = HistoryFilter {
            include_recordings: true,
            ..HistoryFilter::for_project(&fingerprint)
        };
        let found = store.history(&with_recordings, 10).unwrap();
        assert_eq!(found.len(), 1);
        assert!(is_full_recording(&found[0]));
    }

    #[test]
    fn error_messages_are_plain_language_and_escape_attacker_input() {
        let err = FullRecordingError::Serialize {
            message: "bad\ninput".to_owned(),
        };
        let text = err.to_string();
        assert!(
            text.contains("bad\\ninput"),
            "attacker-controlled newline was not escaped: {text}"
        );

        let err = FullRecordingError::Store {
            message: "evil\rmsg".to_owned(),
        };
        let text = err.to_string();
        assert!(
            text.contains("evil\\rmsg"),
            "attacker-controlled carriage return was not escaped: {text}"
        );

        let err = FullRecordingError::NotOptedIn;
        let text = err.to_string();
        assert!(
            text.contains("explicit opt-in"),
            "error should be plain language: {text}"
        );
        assert!(
            text.contains("Nothing was written"),
            "error should say nothing was written: {text}"
        );
    }

    #[test]
    fn past_retention_query_finds_only_expired_items() {
        let store = store_in("full_recording_past_retention");
        let event = make_event("tool.completed", json!({"tool": "Bash"}));
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        persist_full_recording(
            &store,
            &event,
            &event_id,
            "C:\\work\\my project",
            &fingerprint,
            FullRecordingConsent::Full,
        )
        .unwrap();

        // Nothing is past retention at time 0.
        assert!(
            full_recordings_past_retention(&store, &fingerprint, 0)
                .unwrap()
                .is_empty()
        );

        // Everything is past retention at i64::MAX.
        let expired = full_recordings_past_retention(&store, &fingerprint, i64::MAX).unwrap();
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn retention_deadline_does_not_overflow() {
        let max = i64::MAX;
        let result = retention_deadline_ms(max, 1);
        assert_eq!(result, i64::MAX);
    }
}

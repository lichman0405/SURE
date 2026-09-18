//! Observed user-request capture path.
//!
//! Turns a harness `user.request` event into a [`ProjectIntent`] requirement,
//! respecting the full-recording opt-in and redacting secrets before anything
//! is stored.
//!
//! # Security
//!
//! The raw prompt text is not durably retained unless the user has explicitly
//! granted full recording. The [`crate::intent_model::observed_user_request`]
//! door enforces this; this module does not bypass it.
//!
//! Every string that becomes part of a stored requirement is passed through
//! [`crate::redact::redact`] first.

use sure_domain::ids::{EventId, FingerprintId};
use sure_domain::intent::ProjectIntent;

use crate::config::Authority;
use crate::full_recording::{FullRecordingConsent, FullRecordingError, persist_full_recording};
use crate::harness_event::IngestedEvent;
use crate::project_intent::{RecordError, record};
use crate::store::Store;

/// The event type that carries an observed user request.
pub const USER_REQUEST_EVENT_TYPE: &str = "user.request";

/// Why a harness event could not be captured as intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
    /// The payload does not contain a `"text"` field.
    MissingText,
    /// The `"text"` field is present but is not a JSON string.
    TextNotAString,
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingText => f.write_str(
                "The event payload does not contain 'text', so SURE cannot capture the \
                 request. Nothing was kept.",
            ),
            Self::TextNotAString => f.write_str(
                "The event payload has 'text' but it is not a string, so SURE cannot \
                 capture the request. Nothing was kept.",
            ),
        }
    }
}

impl std::error::Error for CaptureError {}

/// What happened when SURE tried to capture an observed user request.
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureOutcome {
    /// The event type is not [`USER_REQUEST_EVENT_TYPE`]; this module ignores it.
    NotARequest,
    /// The door refused to produce a requirement.
    Refused(crate::intent_model::IntentRefusal),
    /// A requirement was produced and wrapped in a [`ProjectIntent`].
    Captured(ProjectIntent),
}

/// Capture an observed user request from a harness event, if the event is a
/// `user.request`.
///
/// - Events whose `event_type` is not [`USER_REQUEST_EVENT_TYPE`] return
///   [`CaptureOutcome::NotARequest`].
/// - The `"text"` field is extracted from the payload, redacted, and passed to
///   [`crate::intent_model::observed_user_request`].
/// - If the door refuses, [`CaptureOutcome::Refused`] is returned.
/// - Otherwise [`CaptureOutcome::Captured`] wraps the resulting requirement.
///
/// # Errors
///
/// [`CaptureError`] if the payload is missing `"text"` or it is not a string.
pub fn capture_observed_intent(
    event: &IngestedEvent,
    authority: &Authority,
) -> Result<CaptureOutcome, CaptureError> {
    if event.envelope.event_type != USER_REQUEST_EVENT_TYPE {
        return Ok(CaptureOutcome::NotARequest);
    }

    let text = event
        .envelope
        .payload
        .get("text")
        .ok_or(CaptureError::MissingText)?
        .as_str()
        .ok_or(CaptureError::TextNotAString)?;

    let redacted = crate::redact::redact(text);

    match crate::intent_model::observed_user_request(&redacted, authority) {
        Ok(requirement) => {
            let intent = ProjectIntent::from_requirements(vec![requirement]);
            Ok(CaptureOutcome::Captured(intent))
        }
        Err(refusal) => Ok(CaptureOutcome::Refused(refusal)),
    }
}

/// The result of persisting a captured observed intent.
#[derive(Debug, Clone, PartialEq)]
pub struct PersistOutcome {
    /// Row ids written for the [`ProjectIntent`] requirements.
    pub intent_rows: Vec<i64>,
    /// Row id of the full recording, when consent is [`FullRecordingConsent::Full`].
    pub recording_row: Option<i64>,
}

/// Why a captured intent could not be persisted.
#[derive(Debug, Clone, PartialEq)]
pub enum PersistObservedError {
    /// The intent could not be written to the store.
    Intent(RecordError),
    /// The full recording could not be written to the store.
    Recording(FullRecordingError),
}

impl std::fmt::Display for PersistObservedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intent(error) => write!(f, "{error}"),
            Self::Recording(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PersistObservedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Intent(error) => Some(error),
            Self::Recording(error) => Some(error),
        }
    }
}

impl From<RecordError> for PersistObservedError {
    fn from(error: RecordError) -> Self {
        Self::Intent(error)
    }
}

impl From<FullRecordingError> for PersistObservedError {
    fn from(error: FullRecordingError) -> Self {
        Self::Recording(error)
    }
}

/// Persist a captured observed intent and, when permitted, the raw event.
///
/// The [`ProjectIntent`] is always persisted via [`crate::project_intent::record`].
/// The original [`IngestedEvent`] is persisted as a full recording **only** when
/// `recording_days` is `Some`.
///
/// # Why one argument rather than a consent and a duration
///
/// `recording_days` is `Some(days)` to write the raw event and keep it that many
/// days, and `None` to write no recording at all — which is what
/// [`FullRecordingConsent::ProjectionOnly`] means. The caller decides that first,
/// from [`crate::config::authority::Authority::full_recording_retention_days`] and
/// the consent it resolved; this function does not take both a consent and a
/// duration it would have to keep consistent with each other. Two arguments that
/// can contradict each other are two arguments that eventually will.
///
/// # Errors
///
/// [`PersistObservedError::Intent`] if the intent cannot be written,
/// [`PersistObservedError::Recording`] if the full recording cannot be written.
pub fn persist_observed_intent(
    store: &Store,
    intent: &ProjectIntent,
    project_root: &str,
    fingerprint: &FingerprintId,
    event: &IngestedEvent,
    event_id: &EventId,
    recording_days: Option<i64>,
) -> Result<PersistOutcome, PersistObservedError> {
    let intent_rows =
        record(store, project_root, fingerprint, intent).map_err(PersistObservedError::Intent)?;

    let recording_row = match recording_days {
        Some(days) => Some(
            persist_full_recording(
                store,
                event,
                event_id,
                project_root,
                fingerprint,
                FullRecordingConsent::Full,
                days,
            )
            .map_err(PersistObservedError::Recording)?,
        ),
        None => None,
    };

    Ok(PersistOutcome {
        intent_rows,
        recording_row,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;
    use sure_domain::capability::CapabilityTier;
    use sure_domain::intent::IntentSource;
    use sure_protocol::PROTOCOL_VERSION;
    use sure_protocol::documents::DocumentKind;
    use sure_protocol::event::EventEnvelope;

    use super::*;
    use crate::config::{Config, ConfigSource, LoadedConfig};
    use crate::full_recording::{
        DEFAULT_FULL_RECORDING_RETENTION_DAYS, full_recordings_for_project,
    };
    use crate::store::{HistoryFilter, RecordKind, Store};

    fn file(text: &str) -> LoadedConfig {
        LoadedConfig {
            config: Config::from_yaml(text).expect("the settings must parse"),
            source: ConfigSource::File(PathBuf::from("sure.yaml")),
            searched: PathBuf::from("sure.yaml"),
        }
    }

    fn both(user: &str, project: &str) -> Authority {
        Authority::new(Some(file(user)), file(project))
    }

    fn project_only(text: &str) -> Authority {
        Authority::new(None, file(text))
    }

    fn scratch(name: &str) -> PathBuf {
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

    fn make_user_request_event(text: impl Into<String>) -> IngestedEvent {
        IngestedEvent {
            envelope: EventEnvelope::new(
                "claude-code",
                USER_REQUEST_EVENT_TYPE,
                "2026-09-14T09:10:56.827Z",
            )
            .with_capability_tier(CapabilityTier::Observed)
            .with_session_id("session-42")
            .with_project_root("C:\\work\\my project")
            .with_payload(json!({"text": text.into()})),
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        }
    }

    fn make_other_event(event_type: &str) -> IngestedEvent {
        IngestedEvent {
            envelope: EventEnvelope::new("claude-code", event_type, "2026-09-14T09:10:56.827Z")
                .with_capability_tier(CapabilityTier::Observed)
                .with_session_id("session-42")
                .with_project_root("C:\\work\\my project")
                .with_payload(json!({"tool": "Bash"})),
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        }
    }

    #[test]
    fn user_request_event_is_captured_when_full_recording_is_granted() {
        let authority = both("privacy:\n  full_recording: true\n", "");
        let event = make_user_request_event("add CSV export");

        let outcome = capture_observed_intent(&event, &authority).expect("capture succeeds");

        let intent = match outcome {
            CaptureOutcome::Captured(intent) => intent,
            other => panic!("expected Captured, got {other:?}"),
        };
        assert_eq!(intent.requirements.len(), 1);
        let req = &intent.requirements[0];
        assert_eq!(req.id, crate::intent_model::OBSERVED_REQUEST_ID);
        assert_eq!(req.text, "add CSV export");
        assert_eq!(req.source, IntentSource::ObservedUserRequest);
        assert!(req.raw_retained);
        assert!(req.is_user_requirement());
    }

    #[test]
    fn user_request_event_is_refused_when_full_recording_is_not_granted() {
        let authority = project_only("");
        let event = make_user_request_event("add CSV export");

        let outcome = capture_observed_intent(&event, &authority).expect("capture succeeds");

        assert_eq!(
            outcome,
            CaptureOutcome::Refused(crate::intent_model::IntentRefusal::RecordingNotPermitted)
        );
    }

    #[test]
    fn empty_text_is_refused_with_no_words() {
        let authority = both("privacy:\n  full_recording: true\n", "");
        for text in ["", " ", "\t\n"] {
            let event = make_user_request_event(text);
            let outcome = capture_observed_intent(&event, &authority).expect("capture succeeds");
            assert_eq!(
                outcome,
                CaptureOutcome::Refused(crate::intent_model::IntentRefusal::NoWords),
                "{text:?} was not refused"
            );
        }
    }

    #[test]
    fn non_user_request_event_returns_not_a_request() {
        let authority = both("privacy:\n  full_recording: true\n", "");
        for event_type in ["tool.completed", "command.executed", "telemetry.heartbeat"] {
            let event = make_other_event(event_type);
            let outcome = capture_observed_intent(&event, &authority).expect("capture succeeds");
            assert_eq!(
                outcome,
                CaptureOutcome::NotARequest,
                "{event_type} was treated as a user request"
            );
        }
    }

    #[test]
    fn credential_shaped_text_is_redacted_before_storage() {
        let authority = both("privacy:\n  full_recording: true\n", "");
        let event = make_user_request_event("api_key=supersecret");

        let outcome = capture_observed_intent(&event, &authority).expect("capture succeeds");
        let intent = match outcome {
            CaptureOutcome::Captured(intent) => intent,
            other => panic!("expected Captured, got {other:?}"),
        };
        assert_eq!(
            intent.requirements[0].text, "api_key=***",
            "credential was not redacted in captured intent"
        );

        // Verify the redaction survives persistence.
        let store = store_in("intent_capture_redacted");
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        persist_observed_intent(
            &store,
            &intent,
            "C:\\work\\my project",
            &fingerprint,
            &event,
            &event_id,
            Some(DEFAULT_FULL_RECORDING_RETENTION_DAYS),
        )
        .unwrap();

        let filter = HistoryFilter {
            project_fingerprint: Some(&fingerprint),
            kind: Some(RecordKind::Document(DocumentKind::ProjectIntent)),
            include_recordings: false,
        };
        let records = store.history(&filter, 10).unwrap();
        assert_eq!(records.len(), 1);
        let stored_text = records[0]
            .document
            .get("text")
            .and_then(serde_json::Value::as_str)
            .expect("stored requirement has text");
        assert_eq!(
            stored_text, "api_key=***",
            "credential was not redacted in store"
        );
        assert!(
            !stored_text.contains("supersecret"),
            "secret survived in stored requirement: {stored_text}"
        );
    }

    #[test]
    fn raw_event_is_not_persisted_when_consent_is_projection_only() {
        let store = store_in("intent_capture_projection_only");
        let authority = both("privacy:\n  full_recording: true\n", "");
        let event = make_user_request_event("add CSV export");
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        let outcome = capture_observed_intent(&event, &authority).expect("capture succeeds");
        let intent = match outcome {
            CaptureOutcome::Captured(intent) => intent,
            other => panic!("expected Captured, got {other:?}"),
        };

        let result = persist_observed_intent(
            &store,
            &intent,
            "C:\\work\\my project",
            &fingerprint,
            &event,
            &event_id,
            None,
        )
        .expect("persist succeeds");

        assert!(!result.intent_rows.is_empty(), "intent should be persisted");
        assert_eq!(
            result.recording_row, None,
            "raw event should not be persisted"
        );

        let back = full_recordings_for_project(&store, &fingerprint, 10).unwrap();
        assert!(back.is_empty(), "no full recording should exist");
    }

    #[test]
    fn raw_event_is_persisted_when_consent_is_full() {
        let store = store_in("intent_capture_full");
        let authority = both("privacy:\n  full_recording: true\n", "");
        let event = make_user_request_event("add CSV export");
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        let outcome = capture_observed_intent(&event, &authority).expect("capture succeeds");
        let intent = match outcome {
            CaptureOutcome::Captured(intent) => intent,
            other => panic!("expected Captured, got {other:?}"),
        };

        let result = persist_observed_intent(
            &store,
            &intent,
            "C:\\work\\my project",
            &fingerprint,
            &event,
            &event_id,
            Some(DEFAULT_FULL_RECORDING_RETENTION_DAYS),
        )
        .expect("persist succeeds");

        assert!(!result.intent_rows.is_empty(), "intent should be persisted");
        assert!(
            result.recording_row.is_some(),
            "raw event should be persisted as full recording"
        );

        let back = full_recordings_for_project(&store, &fingerprint, 10).unwrap();
        assert_eq!(back.len(), 1, "exactly one full recording should exist");
        assert_eq!(back[0].event_type, USER_REQUEST_EVENT_TYPE);
    }

    #[test]
    fn missing_text_field_is_an_error() {
        let authority = both("privacy:\n  full_recording: true\n", "");
        let event = IngestedEvent {
            envelope: EventEnvelope::new(
                "claude-code",
                USER_REQUEST_EVENT_TYPE,
                "2026-09-14T09:10:56.827Z",
            )
            .with_payload(json!({"other": "value"})),
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };

        let err = capture_observed_intent(&event, &authority).unwrap_err();
        assert!(matches!(err, CaptureError::MissingText), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains("does not contain 'text'"), "{msg}");
        assert!(msg.contains("Nothing was kept"), "{msg}");
    }

    #[test]
    fn text_that_is_not_a_string_is_an_error() {
        let authority = both("privacy:\n  full_recording: true\n", "");
        let event = IngestedEvent {
            envelope: EventEnvelope::new(
                "claude-code",
                USER_REQUEST_EVENT_TYPE,
                "2026-09-14T09:10:56.827Z",
            )
            .with_payload(json!({"text": 42})),
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };

        let err = capture_observed_intent(&event, &authority).unwrap_err();
        assert!(matches!(err, CaptureError::TextNotAString), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains("not a string"), "{msg}");
        assert!(msg.contains("Nothing was kept"), "{msg}");
    }

    #[test]
    fn error_messages_are_plain_language() {
        let err = CaptureError::MissingText;
        let text = err.to_string();
        assert!(
            text.contains("does not contain 'text'"),
            "error should be plain language: {text}"
        );
        assert!(
            text.contains("Nothing was kept"),
            "error should say nothing was kept: {text}"
        );

        let err = CaptureError::TextNotAString;
        let text = err.to_string();
        assert!(
            text.contains("not a string"),
            "error should be plain language: {text}"
        );
    }
}

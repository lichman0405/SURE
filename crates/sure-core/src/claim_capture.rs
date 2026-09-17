//! Capture agent completion claims from harness events.
//!
//! When a harness adapter sends an `agent.claim` event, this module extracts the
//! claim, validates its shape, redacts credential-shaped text, and stores it as
//! a `Claim` document. A claim is always stored with `assessment: cannot_confirm`
//! because SURE has not checked it yet.

use serde_json::{Value, json};
use sure_domain::ids::FingerprintId;
use sure_protocol::documents::DocumentKind;

use crate::harness_event::IngestedEvent;
use crate::redact;
use crate::store::{RecordKind, Store, StoreError};

/// The event type that carries an agent completion claim.
pub const AGENT_CLAIM_EVENT_TYPE: &str = "agent.claim";

/// Why a claim could not be captured.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaimCaptureError {
    /// The payload did not contain an `id`.
    MissingId,
    /// The payload's `id` was present but not a string.
    IdNotAString { found: String },
    /// The payload did not contain `claim_text`.
    MissingClaimText,
    /// The payload's `claim_text` was present but not a string.
    ClaimTextNotAString { found: String },
    /// The payload's `claim_type` was present but not a string.
    ClaimTypeNotAString { found: String },
    /// The underlying store reported an error.
    Store { message: String },
}

impl std::fmt::Display for ClaimCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingId => {
                f.write_str("the agent claim did not have an id, so nothing was kept")
            }
            Self::IdNotAString { found } => write!(
                f,
                "the agent claim's id was not a string (found {}), so nothing was kept",
                redact::escape_control_characters(found)
            ),
            Self::MissingClaimText => {
                f.write_str("the agent claim did not have claim_text, so nothing was kept")
            }
            Self::ClaimTextNotAString { found } => write!(
                f,
                "the agent claim's claim_text was not a string (found {}), so nothing was kept",
                redact::escape_control_characters(found)
            ),
            Self::ClaimTypeNotAString { found } => write!(
                f,
                "the agent claim's claim_type was not a string (found {}), so nothing was kept",
                redact::escape_control_characters(found)
            ),
            Self::Store { message } => write!(
                f,
                "SURE could not store the agent claim.\n\n{}\n\nNothing was written.",
                redact::escape_control_characters(message)
            ),
        }
    }
}

impl std::error::Error for ClaimCaptureError {}

impl From<StoreError> for ClaimCaptureError {
    fn from(error: StoreError) -> Self {
        Self::Store {
            message: error.to_string(),
        }
    }
}

/// The result of attempting to capture a claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimCaptureOutcome {
    /// The event type was not `agent.claim`; nothing to do.
    NotAClaim,
    /// The claim was stored, and this is its row id.
    Captured(i64),
}

/// Capture an agent completion claim from an ingested event.
///
/// Returns [`ClaimCaptureOutcome::NotAClaim`] when the event type is not
/// [`AGENT_CLAIM_EVENT_TYPE`]. Otherwise validates the payload, redacts the
/// claim text, builds a `Claim` document, and stores it.
///
/// # Errors
///
/// Returns [`ClaimCaptureError`] when the payload is malformed or the store
/// reports an error. In those cases nothing is stored.
pub fn capture_agent_claim(
    event: &IngestedEvent,
    store: &Store,
    project_root: &str,
    fingerprint: &FingerprintId,
) -> Result<ClaimCaptureOutcome, ClaimCaptureError> {
    if event.envelope.event_type != AGENT_CLAIM_EVENT_TYPE {
        return Ok(ClaimCaptureOutcome::NotAClaim);
    }

    let payload = &event.envelope.payload;

    let id = match payload.get("id") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(ClaimCaptureError::IdNotAString {
                found: other.to_string(),
            });
        }
        None => return Err(ClaimCaptureError::MissingId),
    };

    let claim_text = match payload.get("claim_text") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(ClaimCaptureError::ClaimTextNotAString {
                found: other.to_string(),
            });
        }
        None => return Err(ClaimCaptureError::MissingClaimText),
    };

    let claim_type = match payload.get("claim_type") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => {
            return Err(ClaimCaptureError::ClaimTypeNotAString {
                found: other.to_string(),
            });
        }
        None => None,
    };

    let redacted_claim_text = redact::redact(&claim_text);

    let mut document = json!({
        "id": id,
        "claim_text": redacted_claim_text,
        "assessment": "cannot_confirm",
        "event_type": event.envelope.event_type,
        "session_id": event.envelope.session_id,
        "source": event.envelope.source,
        "timestamp": event.envelope.timestamp,
    });

    if let Some(ct) = claim_type {
        document["claim_type"] = json!(ct);
    }

    let row_id = store.append_for(
        RecordKind::Document(DocumentKind::Claim),
        &document,
        project_root,
        fingerprint,
    )?;

    Ok(ClaimCaptureOutcome::Captured(row_id))
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
    use crate::store::{HistoryFilter, RecordKind, Store};

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

    fn claims_for_project(
        store: &Store,
        fingerprint: &FingerprintId,
    ) -> Vec<crate::store::StoredRecord> {
        let filter = HistoryFilter {
            project_fingerprint: Some(fingerprint),
            kind: Some(RecordKind::Document(DocumentKind::Claim)),
            include_recordings: false,
        };
        store.history(&filter, 100).expect("history query succeeds")
    }

    #[test]
    fn a_valid_agent_claim_is_captured_and_stored() {
        let store = store_in("valid_claim");
        let event = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({
                "id": "claim-123",
                "claim_text": "I finished the CSV export endpoint",
                "claim_type": "feature-complete"
            }),
        );
        let fingerprint = FingerprintId::generate();

        let outcome = capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint)
            .expect("capture succeeds");
        assert!(matches!(outcome, ClaimCaptureOutcome::Captured(id) if id > 0));

        let claims = claims_for_project(&store, &fingerprint);
        assert_eq!(claims.len(), 1);
        let doc = &claims[0].document;
        assert_eq!(doc["id"], "claim-123");
        assert_eq!(doc["claim_text"], "I finished the CSV export endpoint");
        assert_eq!(doc["claim_type"], "feature-complete");
        assert_eq!(doc["assessment"], "cannot_confirm");
    }

    #[test]
    fn non_agent_claim_event_returns_not_a_claim() {
        let store = store_in("not_a_claim");
        let event = make_event("tool.completed", json!({"tool": "Bash"}));
        let fingerprint = FingerprintId::generate();

        let outcome = capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint)
            .expect("capture returns NotAClaim");
        assert_eq!(outcome, ClaimCaptureOutcome::NotAClaim);

        let claims = claims_for_project(&store, &fingerprint);
        assert!(claims.is_empty());
    }

    #[test]
    fn missing_id_returns_error_and_writes_nothing() {
        let store = store_in("missing_id");
        let event = make_event(AGENT_CLAIM_EVENT_TYPE, json!({"claim_text": "foo"}));
        let fingerprint = FingerprintId::generate();

        let err =
            capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint).unwrap_err();
        assert!(matches!(err, ClaimCaptureError::MissingId));
        let msg = err.to_string();
        assert!(msg.contains("nothing was kept"), "{msg}");

        let claims = claims_for_project(&store, &fingerprint);
        assert!(claims.is_empty());
    }

    #[test]
    fn id_not_a_string_returns_error_and_writes_nothing() {
        let store = store_in("id_not_string");
        let event = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({"id": 42, "claim_text": "foo"}),
        );
        let fingerprint = FingerprintId::generate();

        let err =
            capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint).unwrap_err();
        assert!(matches!(err, ClaimCaptureError::IdNotAString { .. }));
        let claims = claims_for_project(&store, &fingerprint);
        assert!(claims.is_empty());
    }

    #[test]
    fn missing_claim_text_returns_error_and_writes_nothing() {
        let store = store_in("missing_text");
        let event = make_event(AGENT_CLAIM_EVENT_TYPE, json!({"id": "claim-1"}));
        let fingerprint = FingerprintId::generate();

        let err =
            capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint).unwrap_err();
        assert!(matches!(err, ClaimCaptureError::MissingClaimText));
        let claims = claims_for_project(&store, &fingerprint);
        assert!(claims.is_empty());
    }

    #[test]
    fn claim_text_not_a_string_returns_error_and_writes_nothing() {
        let store = store_in("text_not_string");
        let event = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({"id": "claim-1", "claim_text": true}),
        );
        let fingerprint = FingerprintId::generate();

        let err =
            capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint).unwrap_err();
        assert!(matches!(err, ClaimCaptureError::ClaimTextNotAString { .. }));
        let claims = claims_for_project(&store, &fingerprint);
        assert!(claims.is_empty());
    }

    #[test]
    fn claim_type_is_optional_but_must_be_a_string_when_present() {
        let store = store_in("claim_type_optional");
        let event_without = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({"id": "claim-1", "claim_text": "foo"}),
        );
        let fingerprint = FingerprintId::generate();

        let outcome =
            capture_agent_claim(&event_without, &store, "C:\\work\\my project", &fingerprint)
                .expect("capture succeeds without claim_type");
        assert!(matches!(outcome, ClaimCaptureOutcome::Captured(_)));

        let claims = claims_for_project(&store, &fingerprint);
        assert_eq!(claims.len(), 1);
        assert!(claims[0].document.get("claim_type").is_none());

        // Invalid claim_type must be rejected.
        let event_with_bad = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({"id": "claim-2", "claim_text": "foo", "claim_type": 123}),
        );
        let err = capture_agent_claim(
            &event_with_bad,
            &store,
            "C:\\work\\my project",
            &fingerprint,
        )
        .unwrap_err();
        assert!(matches!(err, ClaimCaptureError::ClaimTypeNotAString { .. }));

        // Still only the first claim.
        let claims = claims_for_project(&store, &fingerprint);
        assert_eq!(claims.len(), 1);
    }

    #[test]
    fn credential_shaped_text_in_claim_text_is_redacted() {
        let store = store_in("redacted_claim");
        let event = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({
                "id": "claim-1",
                "claim_text": "I finished the endpoint with api_key=supersecret123"
            }),
        );
        let fingerprint = FingerprintId::generate();

        capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint).unwrap();

        let claims = claims_for_project(&store, &fingerprint);
        assert_eq!(claims.len(), 1);
        let text = claims[0].document["claim_text"].as_str().unwrap();
        assert!(
            !text.contains("supersecret123"),
            "secret survived in stored claim: {text}"
        );
        assert!(
            text.contains("***"),
            "redaction marker missing from stored claim: {text}"
        );
    }

    #[test]
    fn stored_document_retains_original_event_reference() {
        let store = store_in("retains_reference");
        let event = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({"id": "claim-1", "claim_text": "done"}),
        );
        let fingerprint = FingerprintId::generate();

        capture_agent_claim(&event, &store, "C:\\work\\my project", &fingerprint).unwrap();

        let claims = claims_for_project(&store, &fingerprint);
        assert_eq!(claims.len(), 1);
        let doc = &claims[0].document;
        assert_eq!(doc["event_type"], AGENT_CLAIM_EVENT_TYPE);
        assert_eq!(doc["session_id"], "session-42");
        assert_eq!(doc["source"], "claude-code");
        assert_eq!(doc["timestamp"], "2026-09-14T09:10:56.827Z");
    }

    #[test]
    fn after_an_error_the_store_contains_no_claim_for_that_project() {
        let store = store_in("error_empty");
        let fingerprint = FingerprintId::generate();

        // Write a valid claim to a different fingerprint to show the store works.
        let other_fp = FingerprintId::generate();
        let valid = make_event(
            AGENT_CLAIM_EVENT_TYPE,
            json!({"id": "ok", "claim_text": "ok"}),
        );
        capture_agent_claim(&valid, &store, "C:\\work\\my project", &other_fp).unwrap();

        // Now try an invalid one for the target fingerprint.
        let invalid = make_event(AGENT_CLAIM_EVENT_TYPE, json!({"claim_text": "no id"}));
        let err = capture_agent_claim(&invalid, &store, "C:\\work\\my project", &fingerprint)
            .unwrap_err();
        assert!(matches!(err, ClaimCaptureError::MissingId));

        // The target fingerprint should have no claims.
        let claims = claims_for_project(&store, &fingerprint);
        assert!(claims.is_empty());

        // The other fingerprint should still have its claim.
        let other_claims = claims_for_project(&store, &other_fp);
        assert_eq!(other_claims.len(), 1);
    }

    #[test]
    fn error_messages_escape_attacker_controlled_input() {
        let err = ClaimCaptureError::IdNotAString {
            found: "bad\ninput".to_owned(),
        };
        let text = err.to_string();
        assert!(
            text.contains("bad\\ninput"),
            "attacker-controlled newline was not escaped: {text}"
        );

        let err = ClaimCaptureError::Store {
            message: "evil\rmsg".to_owned(),
        };
        let text = err.to_string();
        assert!(
            text.contains("evil\\rmsg"),
            "attacker-controlled carriage return was not escaped: {text}"
        );
    }
}

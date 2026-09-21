//! Ingestion side of the harness event protocol.
//!
//! SURE reads event documents produced by a harness plugin or hook, validates
//! the protocol version, parses the envelope, and surfaces the contained event.
//! The ingestion is strict about versions: a mismatch is a refusal, not a
//! best-effort parse. Malformed or hostile input is never treated as trustworthy
//! evidence.
//!
//! # Why this is a separate module
//!
//! [`EventEnvelope::from_json`] lives in `sure-protocol` because it is part of
//! the wire contract. This module lives in `sure-core` because it is the
//! ingestion policy layer: it decides what to do with a parse failure, enriches
//! version errors with direction (older vs newer), and wraps I/O so that every
//! failure path produces a plain-language diagnostic.

use std::io::Read;

use sure_protocol::documents::DocumentKind;
use sure_protocol::event::{EnvelopeError, EventEnvelope};
use sure_protocol::handshake::{Handshake, negotiate};
use sure_protocol::schema::Violation;

/// A successfully ingested harness event, with metadata about the ingestion.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestedEvent {
    /// The parsed event envelope.
    pub envelope: EventEnvelope,
    /// The protocol version that was agreed.
    ///
    /// Always the current build's version, because any mismatch is refused
    /// before the envelope is accepted.
    pub protocol_version: u32,
    /// The kind of document that was ingested.
    pub document_kind: DocumentKind,
}

/// Why a harness event could not be ingested.
///
/// Each variant carries enough detail for a plain-language diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum IngestError {
    /// Could not read from the input.
    Io {
        /// What the reader said.
        message: String,
    },
    /// The text was not valid JSON.
    NotJson {
        /// What the parser said.
        message: String,
    },
    /// The document did not declare a format version.
    MissingVersion,
    /// The document is a format version this build does not speak.
    VersionMismatch {
        /// The handshake result, which names which side has to move.
        handshake: Handshake,
    },
    /// The document does not match the event schema.
    SchemaViolation {
        /// Every way it disagreed with the schema.
        violations: Vec<Violation>,
    },
    /// The document matched the schema but could not be read into the envelope.
    ///
    /// This includes unknown fields (the envelope is `deny_unknown_fields`),
    /// type mismatches, and any gap between the schema and the Rust type.
    Malformed {
        /// What the parser said.
        message: String,
    },
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message } => {
                write!(f, "SURE could not read the event: {message}")
            }
            Self::NotJson { message } => {
                write!(
                    f,
                    "this event is not valid JSON, so SURE cannot read it: {message}"
                )
            }
            Self::MissingVersion => f.write_str(
                "this event does not say which event format it is, so SURE cannot tell \
                 whether it understands it.",
            ),
            Self::VersionMismatch { handshake } => {
                write!(f, "{handshake}")
            }
            Self::SchemaViolation { violations } => {
                write!(
                    f,
                    "this event does not match the SURE event format in {} way(s):",
                    violations.len()
                )?;
                for violation in violations {
                    write!(f, "\n  - {violation}")?;
                }
                Ok(())
            }
            Self::Malformed { message } => write!(
                f,
                "this event matches the SURE event format but could not be read: {message}"
            ),
        }
    }
}

impl std::error::Error for IngestError {}

/// Read one event document from a reader and ingest it.
///
/// # Errors
///
/// Returns [`IngestError`] if the document could not be read or accepted.
pub fn ingest_event(reader: impl Read) -> Result<IngestedEvent, IngestError> {
    let mut text = String::new();
    let mut reader = reader;
    reader
        .read_to_string(&mut text)
        .map_err(|error| IngestError::Io {
            message: error.to_string(),
        })?;
    ingest_event_str(&text)
}

/// Ingest one event document from a string.
///
/// # Errors
///
/// Returns [`IngestError`] if the document could not be accepted.
pub fn ingest_event_str(text: &str) -> Result<IngestedEvent, IngestError> {
    match EventEnvelope::from_json(text) {
        Ok(envelope) => Ok(IngestedEvent {
            protocol_version: envelope.schema_version,
            document_kind: DocumentKind::Event,
            envelope,
        }),
        Err(EnvelopeError::NotJson { message }) => Err(IngestError::NotJson { message }),
        Err(EnvelopeError::MissingVersion) => Err(IngestError::MissingVersion),
        Err(EnvelopeError::UnsupportedVersion { found, .. }) => {
            let handshake = negotiate(found);
            Err(IngestError::VersionMismatch { handshake })
        }
        Err(EnvelopeError::NotConformant { violations }) => {
            Err(IngestError::SchemaViolation { violations })
        }
        Err(EnvelopeError::Malformed { message }) => Err(IngestError::Malformed { message }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;
    use sure_domain::capability::CapabilityTier;
    use sure_protocol::PROTOCOL_VERSION;
    use sure_protocol::event::EventEnvelope;

    fn valid_envelope() -> EventEnvelope {
        EventEnvelope::new("claude-code", "tool.completed", "2026-09-14T09:10:56.827Z")
    }

    #[test]
    fn a_valid_event_at_the_current_protocol_version_round_trips() {
        let original = valid_envelope()
            .with_capability_tier(CapabilityTier::Observed)
            .with_session_id("session-42")
            .with_project_root("C:\\work\\my project")
            .with_payload(json!({"tool": "Bash", "exit_code": 0}));
        let text = original.to_json().unwrap();
        let ingested = ingest_event_str(&text).expect("the event should ingest");
        assert_eq!(ingested.envelope, original);
        assert_eq!(ingested.protocol_version, PROTOCOL_VERSION);
        assert_eq!(ingested.document_kind, DocumentKind::Event);
    }

    #[test]
    fn an_older_protocol_version_is_refused_as_caller_is_older() {
        let text = json!({
            "schema_version": PROTOCOL_VERSION - 1,
            "source": "x",
            "event_type": "y",
            "timestamp": "2026-09-14T09:10:56Z"
        })
        .to_string();
        let error = ingest_event_str(&text).unwrap_err();
        match error {
            IngestError::VersionMismatch { handshake } => {
                assert!(matches!(handshake, Handshake::CallerIsOlder { .. }));
                let msg = handshake.to_string();
                assert!(msg.contains("Update the caller"), "{msg}");
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_newer_protocol_version_is_refused_as_caller_is_newer() {
        let text = json!({
            "schema_version": PROTOCOL_VERSION + 1,
            "source": "x",
            "event_type": "y",
            "timestamp": "2026-09-14T09:10:56Z"
        })
        .to_string();
        let error = ingest_event_str(&text).unwrap_err();
        match error {
            IngestError::VersionMismatch { handshake } => {
                assert!(matches!(handshake, Handshake::CallerIsNewer { .. }));
                let msg = handshake.to_string();
                assert!(msg.contains("Update SURE"), "{msg}");
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_is_refused() {
        let error = ingest_event_str("{not json").unwrap_err();
        assert!(matches!(error, IngestError::NotJson { .. }), "{error:?}");
        let msg = error.to_string();
        assert!(msg.contains("not valid JSON"), "{msg}");
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let text = json!({
            "schema_version": PROTOCOL_VERSION,
            "source": "x",
            "event_type": "y",
            "timestamp": "2026-09-14T09:10:56Z",
            "tool_name": "Bash"
        })
        .to_string();
        let error = ingest_event_str(&text).unwrap_err();
        match error {
            IngestError::SchemaViolation { violations } => {
                assert_eq!(violations.len(), 1, "{violations:?}");
                assert!(
                    violations[0].to_string().contains("tool_name"),
                    "{violations:?}"
                );
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
    }

    #[test]
    fn missing_required_fields_are_refused() {
        let text = json!({"schema_version": PROTOCOL_VERSION}).to_string();
        let error = ingest_event_str(&text).unwrap_err();
        match error {
            IngestError::SchemaViolation { violations } => {
                let missing: Vec<_> = violations
                    .iter()
                    .filter_map(|v| match &v.kind {
                        sure_protocol::schema::ViolationKind::Missing { key } => Some(key.as_str()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(missing, vec!["source", "event_type", "timestamp"]);
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
    }

    #[test]
    fn the_payload_inside_a_valid_envelope_is_preserved_and_reachable() {
        let payload = json!({"tool": "Bash", "exit_code": 0, "nested": {"key": "value"}});
        let original = valid_envelope().with_payload(payload.clone());
        let text = original.to_json().unwrap();
        let ingested = ingest_event_str(&text).expect("the event should ingest");
        assert_eq!(ingested.envelope.payload, payload);
    }

    #[test]
    fn io_error_is_reported_when_reader_fails() {
        struct BrokenReader;
        impl Read for BrokenReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("the pipe broke"))
            }
        }
        let error = ingest_event(BrokenReader).unwrap_err();
        assert!(matches!(error, IngestError::Io { .. }), "{error:?}");
        let msg = error.to_string();
        assert!(msg.contains("the pipe broke"), "{msg}");
    }

    #[test]
    fn error_messages_are_plain_language() {
        let err = IngestError::MissingVersion;
        assert!(err.to_string().contains("does not say which event format"));

        let err = IngestError::Io {
            message: "something went wrong".to_owned(),
        };
        assert!(err.to_string().contains("SURE could not read the event"));
    }
}

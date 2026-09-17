//! Deterministic claim checkers.
//!
//! Checks agent completion claims against recorded harness events without
//! using a model, reading files, or calling external processes.

use serde_json::Value;
use sure_domain::ids::FingerprintId;
use sure_protocol::documents::DocumentKind;
use sure_protocol::event::EventEnvelope;

use crate::harness_event::IngestedEvent;
use crate::recording_projection::{BuildTestKind, FileOperation, StandardProjection, project};
use crate::store::{HistoryFilter, RecordKind, Store, StoreError};

/// Claim type constant: tests ran.
pub const CLAIM_TYPE_TEST_RAN: &str = "test_ran";
/// Claim type constant: a file was changed.
pub const CLAIM_TYPE_FILE_CHANGED: &str = "file_changed";
/// Claim type constant: git state claim.
pub const CLAIM_TYPE_GIT_STATE: &str = "git_state";
/// Claim type constant: current code claim.
pub const CLAIM_TYPE_CURRENT_CODE: &str = "current_code";

/// A claim as read from the store, before checking.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimDocument {
    /// The row id in `records`.
    pub record_id: i64,
    /// The claim's own identifier.
    pub id: String,
    /// The text of the claim.
    pub claim_text: String,
    /// The optional type that determines how it is checked.
    pub claim_type: Option<String>,
}

/// The result of checking one claim.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedClaim {
    /// The row id in `records`.
    pub record_id: i64,
    /// The claim's own identifier.
    pub id: String,
    /// The text of the claim.
    pub claim_text: String,
    /// The claim type that was checked.
    pub claim_type: String,
    /// The assessment outcome.
    pub assessment: sure_domain::evidence::ClaimAssessment,
    /// Evidence that supports the assessment.
    pub evidence: Vec<sure_domain::evidence::Evidence>,
    /// Plain-language explanation of the assessment.
    pub reason: String,
}

/// Why a claim could not be checked.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaimCheckError {
    /// The underlying store reported an error.
    Store(StoreError),
    /// A stored claim document was malformed.
    MalformedClaim {
        /// The row id of the malformed record.
        record_id: i64,
        /// What was wrong with it.
        message: String,
    },
    /// A stored event document was malformed.
    MalformedEvent {
        /// The row id of the malformed record.
        record_id: i64,
        /// What was wrong with it.
        message: String,
    },
}

impl std::fmt::Display for ClaimCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(err) => {
                write!(
                    f,
                    "store error: {}",
                    crate::redact::escape_control_characters(&err.to_string())
                )
            }
            Self::MalformedClaim { record_id, message } => {
                write!(
                    f,
                    "malformed claim at record {}: {}",
                    record_id,
                    crate::redact::escape_control_characters(message)
                )
            }
            Self::MalformedEvent { record_id, message } => {
                write!(
                    f,
                    "malformed event at record {}: {}",
                    record_id,
                    crate::redact::escape_control_characters(message)
                )
            }
        }
    }
}

impl std::error::Error for ClaimCheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(err) => Some(err),
            _ => None,
        }
    }
}

impl From<StoreError> for ClaimCheckError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

/// Read claim and event records for a fingerprint and check every claim.
///
/// # Errors
///
/// Returns [`ClaimCheckError::Store`] if the store query fails,
/// [`ClaimCheckError::MalformedClaim`] if a claim record is missing required
/// fields, or [`ClaimCheckError::MalformedEvent`] if an event record cannot be
/// reconstructed.
pub fn check_claims_in_store(
    store: &Store,
    fingerprint: &FingerprintId,
) -> Result<Vec<CheckedClaim>, ClaimCheckError> {
    let claim_filter = HistoryFilter {
        project_fingerprint: Some(fingerprint),
        kind: Some(RecordKind::Document(DocumentKind::Claim)),
        include_recordings: false,
    };
    let claim_records = store.history(&claim_filter, 10_000)?;

    let event_filter = HistoryFilter {
        project_fingerprint: Some(fingerprint),
        kind: Some(RecordKind::Document(DocumentKind::Event)),
        include_recordings: false,
    };
    let event_records = store.history(&event_filter, 10_000)?;

    let claims = parse_claim_records(&claim_records)?;
    let events = parse_event_records(&event_records)?;

    Ok(check_claims_against_events(&claims, &events, fingerprint))
}

/// Check a slice of claims against a slice of events.
///
/// Events are assumed to be in newest-first order; the most recent matching
/// event is the one that determines the assessment.
pub fn check_claims_against_events(
    claims: &[ClaimDocument],
    events: &[IngestedEvent],
    fingerprint: &FingerprintId,
) -> Vec<CheckedClaim> {
    claims
        .iter()
        .map(|claim| check_claim(claim, events, fingerprint))
        .collect()
}

/// Check one claim against the available events.
///
/// Returns [`sure_domain::evidence::ClaimAssessment::Confirmed`] when a
/// matching event is found, [`ClaimAssessment::CannotConfirm`] when no
/// matching event exists, and [`ClaimAssessment::NotCheckable`] when the
/// claim type is missing or unknown. Never returns
/// [`ClaimAssessment::Contradicted`].
pub fn check_claim(
    claim: &ClaimDocument,
    events: &[IngestedEvent],
    fingerprint: &FingerprintId,
) -> CheckedClaim {
    let claim_type = match claim.claim_type.as_deref() {
        Some(ct) => ct,
        None => {
            return CheckedClaim {
                record_id: claim.record_id,
                id: claim.id.clone(),
                claim_text: claim.claim_text.clone(),
                claim_type: String::new(),
                assessment: sure_domain::evidence::ClaimAssessment::NotCheckable,
                evidence: Vec::new(),
                reason: String::from("Claim has no claim_type, so SURE cannot check it."),
            };
        }
    };

    if !is_known_claim_type(claim_type) {
        return CheckedClaim {
            record_id: claim.record_id,
            id: claim.id.clone(),
            claim_text: claim.claim_text.clone(),
            claim_type: claim_type.to_owned(),
            assessment: sure_domain::evidence::ClaimAssessment::NotCheckable,
            evidence: Vec::new(),
            reason: format!("Claim type '{}' is not a type SURE can check.", claim_type),
        };
    }

    let matching_event = events
        .iter()
        .find(|event| event_matches_claim_type(event, claim_type));

    if let Some(event) = matching_event {
        let projection = project(event);
        let category_name = match projection {
            Some(StandardProjection::Tool { .. }) => "Tool",
            Some(StandardProjection::Command { .. }) => "Command",
            Some(StandardProjection::Git { .. }) => "Git",
            Some(StandardProjection::File { .. }) => "File",
            Some(StandardProjection::BuildTest { .. }) => "BuildTest",
            Some(StandardProjection::Outcome { .. }) => "Outcome",
            None => "unknown",
        };
        let evidence = sure_domain::evidence::Evidence::new(
            sure_domain::evidence::EvidenceClass::ObservedFact,
            format!("Recorded {category_name} event in harness session"),
            sure_domain::evidence::EvidenceAnchor::new(
                sure_domain::evidence::AnchorSubject::Event,
                event.envelope.event_type.clone(),
                event.envelope.timestamp.clone(),
            ),
            Some(fingerprint.clone()),
            sure_domain::severity::Severity::Note,
        );
        CheckedClaim {
            record_id: claim.record_id,
            id: claim.id.clone(),
            claim_text: claim.claim_text.clone(),
            claim_type: claim_type.to_owned(),
            assessment: sure_domain::evidence::ClaimAssessment::Confirmed,
            evidence: vec![evidence],
            reason: format!("A recorded harness event supports this {claim_type} claim."),
        }
    } else {
        CheckedClaim {
            record_id: claim.record_id,
            id: claim.id.clone(),
            claim_text: claim.claim_text.clone(),
            claim_type: claim_type.to_owned(),
            assessment: sure_domain::evidence::ClaimAssessment::CannotConfirm,
            evidence: Vec::new(),
            reason: String::from("SURE has no recorded event that supports this claim."),
        }
    }
}

fn is_known_claim_type(claim_type: &str) -> bool {
    matches!(
        claim_type,
        CLAIM_TYPE_TEST_RAN
            | CLAIM_TYPE_FILE_CHANGED
            | CLAIM_TYPE_GIT_STATE
            | CLAIM_TYPE_CURRENT_CODE
    )
}

fn event_matches_claim_type(event: &IngestedEvent, claim_type: &str) -> bool {
    let projection = match project(event) {
        Some(p) => p,
        None => return false,
    };

    match claim_type {
        CLAIM_TYPE_TEST_RAN => matches!(
            projection,
            StandardProjection::BuildTest {
                kind: BuildTestKind::Test,
                ..
            }
        ),
        CLAIM_TYPE_FILE_CHANGED => matches!(
            projection,
            StandardProjection::File {
                operation: FileOperation::Write,
                ..
            }
        ),
        CLAIM_TYPE_GIT_STATE => matches!(projection, StandardProjection::Git { .. }),
        CLAIM_TYPE_CURRENT_CODE => matches!(
            projection,
            StandardProjection::BuildTest {
                kind: BuildTestKind::Build | BuildTestKind::Compile | BuildTestKind::Run,
                success: true,
                ..
            }
        ),
        _ => false,
    }
}

fn parse_claim_records(
    records: &[crate::store::StoredRecord],
) -> Result<Vec<ClaimDocument>, ClaimCheckError> {
    let mut claims = Vec::new();
    for record in records {
        let doc = &record.document;
        let id = doc.get("id").and_then(Value::as_str).ok_or_else(|| {
            ClaimCheckError::MalformedClaim {
                record_id: record.id,
                message: String::from("missing or non-string 'id'"),
            }
        })?;
        let claim_text = doc
            .get("claim_text")
            .and_then(Value::as_str)
            .ok_or_else(|| ClaimCheckError::MalformedClaim {
                record_id: record.id,
                message: String::from("missing or non-string 'claim_text'"),
            })?;
        let claim_type = doc
            .get("claim_type")
            .and_then(Value::as_str)
            .map(String::from);
        if doc.get("claim_type").is_some() && claim_type.is_none() {
            return Err(ClaimCheckError::MalformedClaim {
                record_id: record.id,
                message: String::from("non-string 'claim_type'"),
            });
        }
        claims.push(ClaimDocument {
            record_id: record.id,
            id: id.to_owned(),
            claim_text: claim_text.to_owned(),
            claim_type,
        });
    }
    Ok(claims)
}

fn parse_event_records(
    records: &[crate::store::StoredRecord],
) -> Result<Vec<IngestedEvent>, ClaimCheckError> {
    let mut events = Vec::new();
    for record in records {
        let json = serde_json::to_string(&record.document).map_err(|error| {
            ClaimCheckError::MalformedEvent {
                record_id: record.id,
                message: format!("could not serialize document to JSON: {error}"),
            }
        })?;
        let envelope =
            EventEnvelope::from_json(&json).map_err(|error| ClaimCheckError::MalformedEvent {
                record_id: record.id,
                message: format!("could not parse event envelope: {error}"),
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
    use sure_domain::ids::{EventId, FingerprintId};
    use sure_protocol::PROTOCOL_VERSION;
    use sure_protocol::documents::DocumentKind;
    use sure_protocol::event::EventEnvelope;

    use crate::claim_capture::capture_agent_claim;
    use crate::session_event_store::SessionEventStore;
    use crate::store::Store;

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
            document_kind: DocumentKind::Event,
        }
    }

    fn make_claim_event(id: &str, claim_text: &str, claim_type: Option<&str>) -> IngestedEvent {
        let mut payload = json!({
            "id": id,
            "claim_text": claim_text,
        });
        if let Some(ct) = claim_type {
            payload["claim_type"] = json!(ct);
        }
        IngestedEvent {
            envelope: EventEnvelope::new(
                "claude-code",
                crate::claim_capture::AGENT_CLAIM_EVENT_TYPE,
                "2026-09-14T09:10:56.827Z",
            )
            .with_capability_tier(CapabilityTier::Observed)
            .with_session_id("session-42")
            .with_project_root("C:\\work\\my project")
            .with_payload(payload),
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        }
    }

    #[test]
    fn test_ran_claim_confirmed_by_test_event() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("tests ran"),
            claim_type: Some(String::from(CLAIM_TYPE_TEST_RAN)),
        };
        let event = make_event(
            "test.finished",
            json!({"target": "unit tests", "success": false, "counts": {"passed": 10, "failed": 2}}),
        );
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[event], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::Confirmed
        );
        assert_eq!(result.evidence.len(), 1);
        assert_eq!(
            result.reason,
            "A recorded harness event supports this test_ran claim."
        );
    }

    #[test]
    fn file_changed_claim_confirmed_by_write_event() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("file was changed"),
            claim_type: Some(String::from(CLAIM_TYPE_FILE_CHANGED)),
        };
        let event = make_event("file.write", json!({"path": "src/main.rs", "size": 1024}));
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[event], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::Confirmed
        );
        assert_eq!(result.evidence.len(), 1);
    }

    #[test]
    fn git_state_claim_confirmed_by_git_event() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("git state"),
            claim_type: Some(String::from(CLAIM_TYPE_GIT_STATE)),
        };
        let event = make_event(
            "git.commit",
            json!({"subcommand": "commit", "branch": "main", "files_affected": 3}),
        );
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[event], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::Confirmed
        );
        assert_eq!(result.evidence.len(), 1);
    }

    #[test]
    fn current_code_claim_confirmed_by_successful_build_event() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("code builds"),
            claim_type: Some(String::from(CLAIM_TYPE_CURRENT_CODE)),
        };
        let event = make_event(
            "build.finished",
            json!({"target": "sure-core", "success": true}),
        );
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[event], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::Confirmed
        );
        assert_eq!(result.evidence.len(), 1);
    }

    #[test]
    fn claim_without_matching_evidence_is_cannot_confirm() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("tests ran"),
            claim_type: Some(String::from(CLAIM_TYPE_TEST_RAN)),
        };
        let event = make_event(
            "git.commit",
            json!({"subcommand": "commit", "branch": "main"}),
        );
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[event], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::CannotConfirm
        );
        assert!(result.evidence.is_empty());
        assert!(result.reason.contains("no recorded event"));
    }

    #[test]
    fn unknown_claim_type_is_not_checkable() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("something"),
            claim_type: Some(String::from("unknown_type")),
        };
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::NotCheckable
        );
        assert!(result.evidence.is_empty());
        assert!(result.reason.contains("unknown_type"));
    }

    #[test]
    fn missing_claim_type_is_not_checkable() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("something"),
            claim_type: None,
        };
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::NotCheckable
        );
        assert!(result.evidence.is_empty());
        assert!(result.reason.contains("no claim_type"));
    }

    #[test]
    fn cannot_confirm_is_not_contradicted() {
        let claim = ClaimDocument {
            record_id: 1,
            id: String::from("claim-1"),
            claim_text: String::from("tests ran"),
            claim_type: Some(String::from(CLAIM_TYPE_TEST_RAN)),
        };
        let fingerprint = FingerprintId::generate();
        let result = check_claim(&claim, &[], &fingerprint);
        assert_eq!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::CannotConfirm
        );
        assert_ne!(
            result.assessment,
            sure_domain::evidence::ClaimAssessment::Contradicted
        );
    }

    #[test]
    fn check_claims_in_store_reads_claims_and_events() {
        let store = store_in("store_roundtrip");
        let fingerprint = FingerprintId::generate();

        // Store a claim.
        let claim_event = make_claim_event("claim-1", "tests ran", Some(CLAIM_TYPE_TEST_RAN));
        capture_agent_claim(&claim_event, &store, "C:\\work\\my project", &fingerprint)
            .expect("capture succeeds");

        // Store a matching test event.
        let test_event = make_event(
            "test.finished",
            json!({"target": "unit tests", "success": true}),
        );
        let event_id = EventId::generate();
        SessionEventStore::new(&store)
            .persist(&test_event, "C:\\work\\my project", &fingerprint, &event_id)
            .expect("persist succeeds");

        let results = check_claims_in_store(&store, &fingerprint).expect("check succeeds");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].assessment,
            sure_domain::evidence::ClaimAssessment::Confirmed
        );
    }

    #[test]
    fn confirmed_evidence_names_current_fingerprint() {
        let store = store_in("evidence_fingerprint");
        let fingerprint = FingerprintId::generate();

        let claim_event = make_claim_event("claim-1", "tests ran", Some(CLAIM_TYPE_TEST_RAN));
        capture_agent_claim(&claim_event, &store, "C:\\work\\my project", &fingerprint)
            .expect("capture succeeds");

        let test_event = make_event(
            "test.finished",
            json!({"target": "unit tests", "success": true}),
        );
        let event_id = EventId::generate();
        SessionEventStore::new(&store)
            .persist(&test_event, "C:\\work\\my project", &fingerprint, &event_id)
            .expect("persist succeeds");

        let results = check_claims_in_store(&store, &fingerprint).expect("check succeeds");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].evidence.len(), 1);
        assert_eq!(
            results[0].evidence[0].fingerprint,
            Some(fingerprint.clone())
        );
    }

    #[test]
    fn error_messages_escape_attacker_controlled_input() {
        let err = ClaimCheckError::MalformedClaim {
            record_id: 1,
            message: "bad\ninput".to_owned(),
        };
        let text = err.to_string();
        assert!(
            text.contains("bad\\ninput"),
            "attacker-controlled newline was not escaped: {text}"
        );

        let err = ClaimCheckError::MalformedEvent {
            record_id: 2,
            message: "evil\rmsg".to_owned(),
        };
        let text = err.to_string();
        assert!(
            text.contains("evil\\rmsg"),
            "attacker-controlled carriage return was not escaped: {text}"
        );
    }
}

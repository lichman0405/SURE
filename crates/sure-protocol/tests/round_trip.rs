//! Documents survive being written and read back.
//!
//! The second acceptance criterion for P1-T007. Conformance
//! (`tests/conformance.rs`) asks "does this match the schema"; this asks "does
//! it come back as what it was".
//!
//! # What "round trip" means here, precisely
//!
//! Two properties, and the second is the one that catches real mistakes:
//!
//! 1. **Rust → JSON → Rust is the same value.** Catches a field that is written
//!    but not read, which is how an `Option` silently becomes `None` on the way
//!    back in.
//! 2. **JSON → Rust → JSON is the same text.** Catches a field that is read but
//!    written differently — a renamed key, a default that substitutes for an
//!    absent value, an enum that comes out under a different name. Property 1
//!    alone would miss all of those.
//!
//! # The limit of the claim
//!
//! These documents carry no `schema_version`, so there is no way to tell
//! "written by this build" from "written by a newer one". A field this build
//! does not know is dropped on read. The last test in this file states that
//! plainly, because the alternative — writing "round-trip tests pass" as though
//! it covered every document — is the kind of quiet overclaim this repository
//! exists to catch. Which build wrote a stored document, and what to do about a
//! newer one, is P1-T005's migration decision.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{
    check_result, claim, content_fingerprint, documents, envelope, finding, fingerprint,
    git_fingerprint, not_run_result, repair, requirement,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sure_domain::capability::CapabilityTier;
use sure_domain::evidence::EvidenceClass;
use sure_domain::finding::FindingStatus;
use sure_domain::status::CheckStatus;
use sure_domain::{
    evidence::ClaimAssessment, intent::IntentSource, severity::Severity, vocabulary::Finding,
};
use sure_protocol::documents::DocumentKind;
use sure_protocol::event::{EnvelopeError, EventEnvelope};

/// Write, read back, and check both properties.
///
/// Generic so that adding a document to the round-trip suite is one line, and so
/// that no type can be excused from it by being awkward to call.
fn round_trips<T>(original: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let text = serde_json::to_string(original).expect("the document serializes");
    let back: T = serde_json::from_str(&text).expect("the document deserializes");
    assert_eq!(&back, original, "the value changed:\n{text}");

    let again = serde_json::to_string(&back).expect("the document re-serializes");
    assert_eq!(again, text, "the text changed on the second write");
}

#[test]
fn every_document_survives_a_round_trip_through_json() {
    round_trips(&finding());
    round_trips(&claim());
    round_trips(&check_result());
    round_trips(&requirement());
    round_trips(&repair());
    round_trips(&envelope());
}

#[test]
fn every_document_survives_a_round_trip_through_a_value() {
    // The path the storage layer and the schema check both take: through
    // `serde_json::Value` rather than straight from text. A `#[serde(flatten)]`
    // or a custom serializer that only works from a string shows up here.
    for (kind, original) in documents() {
        let text = serde_json::to_string(&original).expect("the document serializes");
        let back: Value = serde_json::from_str(&text).expect("the document is JSON");
        assert_eq!(back, original, "{kind} changed through a value");
    }
}

#[test]
fn the_json_sure_writes_is_the_json_sure_reads_back() {
    // Property 2 over the whole registry: take the exact document, read it back
    // into a value, write it out again, and require the same bytes.
    for (kind, document) in documents() {
        let text = serde_json::to_string(&document).expect("the document serializes");
        let reparsed: Value = serde_json::from_str(&text).expect("the document is JSON");
        let rewritten = serde_json::to_string(&reparsed).expect("the value serializes");
        assert_eq!(rewritten, text, "{kind} did not survive a repeat write");
    }
}

#[test]
fn a_finding_comes_back_with_its_status_and_severity_intact() {
    // The two fields a report is built from. An enum that came back as a
    // different variant, or as the first variant because a name was not
    // recognised, would change what the user is told.
    let original = finding();
    let text = serde_json::to_string(&original).expect("the finding serializes");
    let back: Finding = serde_json::from_str(&text).expect("the finding deserializes");
    assert_eq!(back.status, FindingStatus::Open);
    assert_eq!(back.severity, Severity::MustFix);
    assert_eq!(back.evidence.len(), 1);
    assert_eq!(
        back.evidence[0].class,
        EvidenceClass::DeterministicCheck,
        "the evidence class changed, which would change what the finding is worth"
    );
}

#[test]
fn an_absent_optional_field_comes_back_absent_rather_than_null() {
    // `docs/architecture/EVENT_PROTOCOL.md`: "Missing data is not invented." A
    // field that came back as `null` would let a reader see "the adapter said
    // nothing is there", which is a different claim from "the adapter did not
    // say".
    let bare = EventEnvelope::new("claude-code", "session.started", "2026-09-14T09:10:56Z");
    let document = serde_json::to_value(&bare).expect("the envelope serializes");
    for key in ["capability_tier", "session_id", "project_root"] {
        assert!(
            document.get(key).is_none(),
            "\"{key}\" was written as {} rather than omitted",
            document[key]
        );
    }
    // And the payload is an empty object, not null: an event with no body has an
    // empty body.
    assert_eq!(document["payload"], json!({}));

    let back = EventEnvelope::from_json(&serde_json::to_string(&bare).unwrap())
        .expect("the envelope is readable");
    assert_eq!(back, bare);
}

#[test]
fn a_present_optional_field_is_not_lost_on_the_way_back() {
    // The failure mode property 1 exists for: written but not read.
    let full = envelope();
    assert!(
        full.session_id.is_some(),
        "the fixture is not exercising this"
    );
    let back = EventEnvelope::from_json(&full.to_json().unwrap()).expect("readable");
    assert_eq!(back.capability_tier, Some(CapabilityTier::Observed));
    assert_eq!(back.session_id.as_deref(), Some("harness-7"));
    assert_eq!(
        back.project_root.as_deref(),
        Some("C:\\work\\my project"),
        "a Windows path must survive as the same text, backslashes included"
    );
    assert_eq!(back.payload, json!({"tool": "Bash", "exit_code": 0}));
}

#[test]
fn the_envelope_reads_and_writes_the_same_document() {
    // The one document that is read *from outside*, so it goes through the
    // version check and the schema check as well as serde. All three must agree.
    let original = envelope();
    let text = original.to_json().expect("the envelope serializes");
    let back = EventEnvelope::from_json(&text).expect("the envelope is readable");
    assert_eq!(back, original);
    assert_eq!(back.to_json().expect("re-serializes"), text);
}

#[test]
fn an_empty_collection_is_omitted_and_a_populated_one_is_not() {
    // Several domain fields are `skip_serializing_if = "Vec::is_empty"`. That is
    // right — an empty evidence list and an absent one mean the same thing — but
    // it means a round trip must not turn a populated list into an empty one.
    let with_evidence = claim();
    assert!(
        !with_evidence.evidence.is_empty(),
        "the fixture is not exercising this"
    );
    round_trips(&with_evidence);

    let mut bare = claim();
    bare.evidence.clear();
    bare.session = None;
    let document = serde_json::to_value(&bare).expect("the claim serializes");
    assert!(document.get("evidence").is_none(), "{document}");
    assert!(document.get("session").is_none(), "{document}");
    round_trips(&bare);
}

#[test]
fn both_fingerprint_kinds_survive_a_round_trip() {
    // A content fingerprint has no `git` key and a Git one does, so the optional
    // sub-object is exercised in both directions.
    round_trips(&content_fingerprint());
    round_trips(&git_fingerprint());
    let document = serde_json::to_value(content_fingerprint()).expect("serializes");
    assert!(document.get("git").is_none(), "{document}");
    let document = serde_json::to_value(git_fingerprint()).expect("serializes");
    assert_eq!(document["git"]["branch"], json!("main"));
    assert!(
        document["git"].get("untracked_digest").is_none(),
        "an unset digest must be omitted, not written as null"
    );
}

#[test]
fn a_check_that_did_not_run_keeps_its_reason_and_its_unknown_class() {
    // The two fields that keep a skipped check from reading as a pass. The
    // evidence class is the one that matters most, and it is the one a careless
    // serializer would default.
    let original = not_run_result();
    let back = round_trip_one(&original);
    assert_eq!(back.status, CheckStatus::Skipped);
    assert_eq!(back.evidence_class, EvidenceClass::Unknown);
    assert!(back.not_checked_reason.is_some(), "{back:?}");
    assert_eq!(back.project_fingerprint, fingerprint());
    assert!(!back.reason.is_empty(), "the plain-language line was lost");
}

#[test]
fn every_enum_in_every_document_round_trips_through_its_wire_name() {
    // A sweep rather than a spot check: an enum variant whose serde name was
    // mistyped comes back as a deserialization error rather than as the wrong
    // variant, and this is what makes that visible for all of them at once.
    for status in FindingStatus::ALL {
        let mut f = finding();
        f.status = *status;
        round_trips(&f);
    }
    for severity in Severity::ALL {
        let mut f = finding();
        f.severity = *severity;
        round_trips(&f);
    }
    for assessment in ClaimAssessment::ALL {
        let mut c = claim();
        c.assessment = *assessment;
        round_trips(&c);
    }
    for source in IntentSource::ALL {
        let mut r = requirement();
        r.source = *source;
        round_trips(&r);
    }
    for class in EvidenceClass::ALL {
        let mut c = check_result();
        c.evidence_class = *class;
        round_trips(&c);
    }
    for &tier in CapabilityTier::ALL {
        round_trips(&envelope().with_capability_tier(tier));
    }
}

#[test]
fn a_claim_with_no_session_omits_it_rather_than_writing_null() {
    let mut bare = claim();
    bare.session = None;
    let document = serde_json::to_value(&bare).expect("the claim serializes");
    assert!(document.get("session").is_none(), "{document}");
    round_trips(&bare);
}

#[test]
fn a_document_written_by_a_newer_sure_loses_the_fields_this_build_does_not_know() {
    // Stated rather than discovered by a user.
    //
    // The six stored documents carry no `schema_version`, so nothing here can
    // tell a field this build forgot from a field a future build added. The
    // event envelope is different: it is versioned, and it refuses both an
    // unknown version and an unknown field (see `event.rs`).
    //
    // This test asserts the behaviour so that it is a decision on the record
    // rather than an accident of serde's defaults. P1-T005 (local storage and
    // migrations) owns the fix, because the version belongs on the stored record
    // and not on the document.
    let mut document = serde_json::to_value(finding()).expect("the finding serializes");
    document["confidence"] = json!(0.87);
    document["raised_by"] = json!("a newer SURE");
    let text = serde_json::to_string(&document).expect("the document is JSON");

    let back: Finding = serde_json::from_str(&text).expect("the known part still reads");
    let rewritten = serde_json::to_value(&back).expect("the finding serializes");
    assert!(
        rewritten.get("confidence").is_none(),
        "this build does not know \"confidence\", so it cannot have kept it"
    );
    assert!(rewritten.get("raised_by").is_none());
    // The part it does know is intact, so the loss is bounded and visible rather
    // than a failed read.
    assert_eq!(rewritten["status"], json!("open"));
}

#[test]
fn the_versioned_document_refuses_a_newer_format_rather_than_losing_fields() {
    // The contrast with the test above, and the reason the envelope has a
    // version and the stored documents do not: an event from an adapter is read
    // by a build it was not written for, so it gets a version gate rather than a
    // silent drop.
    let mut text: Value =
        serde_json::from_str(&envelope().to_json().expect("the envelope serializes"))
            .expect("the envelope is JSON");
    text["schema_version"] = json!(sure_protocol::PROTOCOL_VERSION + 1);
    text["new_field"] = json!(true);
    assert!(matches!(
        EventEnvelope::from_json(&text.to_string()),
        Err(EnvelopeError::UnsupportedVersion { .. })
    ));
}

/// Round-trip one value and hand it back.
fn round_trip_one<T>(original: &T) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let text = serde_json::to_string(original).expect("the document serializes");
    let back: T = serde_json::from_str(&text).expect("the document deserializes");
    assert_eq!(&back, original, "the value changed:\n{text}");
    back
}

#[test]
fn every_document_in_the_registry_is_round_tripped() {
    // The companion to the same guard in `conformance.rs`: a schema with no
    // round-trip case is a document nobody has proved can be read back.
    let excused = [DocumentKind::FixtureExpectation];
    let covered: Vec<DocumentKind> = documents().into_iter().map(|(kind, _)| kind).collect();
    for &kind in sure_protocol::documents::ALL {
        assert!(
            covered.contains(&kind) || excused.contains(&kind),
            "{kind} has a schema and no round-trip case"
        );
    }
}

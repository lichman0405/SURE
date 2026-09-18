//! Every structure SURE writes, checked against its schema.
//!
//! `docs/architecture/FROZEN_SEMANTICS.md` treats `schemas/` as the wire
//! contract. Before this test existed, only the *enum names* were checked
//! against the schemas — `crates/sure-domain/tests/wire_contract.rs` compares
//! each `enum` array to its Rust list — so a field that was renamed, removed or
//! never added went unnoticed. Two had:
//!
//! - `RepairContract` serialized its issue as `issue`, and `repair.schema.json`
//!   requires `issue_id`.
//! - `CheckResult` had no `evidence_class` and no `project_fingerprint`, both of
//!   which `check-result.schema.json` requires.
//!
//! The first test in this file is what would have caught them. It is written
//! against the real types rather than against a hand-written sample, so it
//! cannot pass while the types and the documents disagree.
//!
//! # Why the "not vacuous" tests are here too
//!
//! A conformance test that always passed would be worse than none: it is a
//! false green inside the check that exists to prevent false greens. So each
//! document is also checked with a required key removed, and the test fails
//! unless a violation comes back. That makes "this document conforms" a claim
//! with something behind it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{check_result, documents, finding, fingerprint, repair, requirement};
use serde_json::{Value, json};
use sure_domain::capability::CapabilityTier;
use sure_domain::ids::CheckId;
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, CheckStatus, NotCheckedReason};
use sure_domain::vocabulary::{GitState, ProjectFingerprint};
use sure_protocol::documents::DocumentKind;

#[test]
fn every_structure_matches_the_schema_that_describes_it() {
    // The acceptance criterion for P1-T007, as one assertion per document.
    for (kind, document) in documents() {
        let schema = kind.schema().expect("the embedded schema is enforceable");
        let violations = schema.validate(&document);
        assert!(
            violations.is_empty(),
            "{kind} does not match {}:\n  {}\n\ndocument: {}",
            kind.file_name(),
            violations
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  "),
            serde_json::to_string_pretty(&document).unwrap_or_default()
        );
    }
}

#[test]
fn every_required_key_is_actually_enforced_for_every_document() {
    // The test above is only worth something if the validator can fail. For
    // each document, remove each key its schema requires and insist on a
    // violation that names the key. A validator that accepted everything would
    // pass the test above and fail here.
    for (kind, document) in documents() {
        let schema = kind.schema().expect("the embedded schema is enforceable");
        let required = kind.required_keys();
        assert!(!required.is_empty(), "{kind} requires nothing");

        let object = document
            .as_object()
            .unwrap_or_else(|| panic!("{kind} is not a JSON object"));
        for key in required {
            assert!(
                object.contains_key(&key),
                "{kind} was serialized without its required key \"{key}\": {document}"
            );

            let mut broken = object.clone();
            broken.remove(&key);
            let violations = schema.validate(&Value::Object(broken));
            assert!(
                !violations.is_empty(),
                "{kind} was accepted with \"{key}\" removed, so the schema's \
                 requirement is not enforced"
            );
        }
    }
}

#[test]
fn a_wrongly_typed_required_field_is_refused_for_every_document() {
    // The other direction: present but wrong. Catches a schema whose `type` was
    // dropped, which `required` alone would not.
    for (kind, document) in documents() {
        let schema = kind.schema().expect("the embedded schema is enforceable");
        let mut broken = document
            .as_object()
            .expect("a document is a JSON object")
            .clone();
        // Replace the first required key with a value of a type no schema here
        // accepts. `null` is the one value that is never a string, an object or
        // an array.
        let key = kind.required_keys().into_iter().next().expect("a key");
        broken.insert(key.clone(), Value::Null);
        // `session_id` in the event schema is explicitly nullable, and no other
        // document's first required key is, so a null here must be refused.
        assert!(
            !schema.validate(&Value::Object(broken)).is_empty(),
            "{kind} accepted null for \"{key}\""
        );
    }
}

#[test]
fn the_repair_contract_names_its_issue_the_way_the_schema_does() {
    // A regression test with a name. The field was `issue` in Rust and
    // `issue_id` in the schema; both spellings looked right in isolation, and
    // only a serialization check could tell them apart.
    let document = serde_json::to_value(repair()).expect("a repair contract serializes");
    assert!(document.get("issue_id").is_some(), "{document}");
    assert!(document.get("issue").is_none(), "{document}");
}

#[test]
fn a_repair_contract_carries_its_identity_and_recheck_list() {
    // The schema and the type must agree on the fields that make a contract
    // actionable: its own id and the checks to re-run after repair.
    let document = serde_json::to_value(repair()).expect("a repair contract serializes");
    assert!(document.get("id").is_some(), "{document}");
    let recheck = document
        .get("recheck")
        .expect("recheck is present")
        .as_array()
        .expect("recheck is an array");
    assert!(!recheck.is_empty(), "{document}");
}

#[test]
fn a_check_result_says_how_it_was_established_and_which_state_it_is_about() {
    // The second regression. `evidence_class` and `project_fingerprint` are
    // what keep a file read and a live run from being summarised into the same
    // green, and what keep a stale pass from being read against a later state.
    let document = serde_json::to_value(check_result()).expect("a check result serializes");
    assert_eq!(document["evidence_class"], json!("deterministic_check"));
    assert_eq!(
        document["project_fingerprint"],
        json!(fingerprint().as_str())
    );
}

#[test]
fn a_check_that_did_not_run_declares_that_it_established_nothing() {
    // The property the constructor enforces rather than the caller: a check that
    // did not run cannot be given an evidence class, so it can never be counted
    // as an `observed_fact` or a `deterministic_check`.
    let not_run = CheckResult::not_run(
        CheckId::generate(),
        "start the app and load the home page",
        Severity::MustFix,
        true,
        NotCheckedReason::ExecutionNotAuthorized,
        fingerprint(),
    );
    assert_eq!(not_run.status, CheckStatus::Skipped);
    let document = serde_json::to_value(&not_run).expect("a check result serializes");
    assert_eq!(document["evidence_class"], json!("unknown"));
    assert_eq!(document["status"], json!("skipped"));

    let errored = CheckResult::errored(
        CheckId::generate(),
        "run the test suite",
        Severity::MustFix,
        true,
        "the test runner exited before reporting anything",
        fingerprint(),
    );
    let document = serde_json::to_value(&errored).expect("a check result serializes");
    assert_eq!(document["evidence_class"], json!("unknown"));
}

#[test]
fn a_finding_serializes_its_evidence_as_objects() {
    // `finding.schema.json` types `evidence` as an array of objects. An array
    // of strings or of ids would satisfy neither this nor anything that reads a
    // finding later.
    let document = serde_json::to_value(finding()).expect("a finding serializes");
    let evidence = document["evidence"]
        .as_array()
        .expect("evidence is an array");
    assert!(!evidence.is_empty(), "{document}");
    for entry in evidence {
        assert!(entry.is_object(), "{entry}");
        assert!(entry.get("class").is_some(), "{entry}");
        assert!(entry.get("anchor").is_some(), "{entry}");
    }
}

#[test]
fn a_project_fingerprint_is_serialized_as_its_identifier() {
    // `check-result.schema.json` types `project_fingerprint` as a string. The
    // domain type is a newtype over one, so this holds — but it holds because
    // of a `serde` attribute in another crate, which is exactly the kind of
    // coupling that drifts unnoticed.
    let document = serde_json::to_value(ProjectFingerprint::git(
        "abc123",
        GitState {
            head: "deadbeef".to_owned(),
            dirty: false,
            dirty_digest: None,
            untracked_digest: None,
            branch: Some("main".to_owned()),
        },
    ))
    .expect("a fingerprint serializes");
    assert!(document["id"].is_string(), "{document}");
    assert!(document["digest"].is_string(), "{document}");
}

#[test]
fn the_fixture_expectation_schema_has_no_rust_type_yet_and_that_is_recorded() {
    // `docs/architecture/FROZEN_SEMANTICS.md` records this as conformance gap 2:
    // the schema exists, and the fourteen `fixtures/adversarial/*/scenario.json`
    // files do not match it. P14-T001–T011 resolves it. This test exists so that
    // the omission is visible here rather than as a missing row in a table, and
    // so that adding the type means deleting this test deliberately.
    let kind = DocumentKind::FixtureExpectation;
    kind.schema()
        .expect("the schema is enforceable even though nothing produces it yet");
    assert!(
        kind.required_keys()
            .contains(&"required_outcomes".to_owned()),
        "the recorded gap is that fixtures do not carry this key"
    );
}

#[test]
fn every_document_in_the_registry_is_covered_by_this_test_or_excused() {
    // Keeps the table honest as documents are added. A new schema with no
    // conformance case fails here, which is the prompt to write one or to add
    // it to the excused list with a reason.
    let excused = [DocumentKind::FixtureExpectation];
    let covered: Vec<DocumentKind> = documents().into_iter().map(|(kind, _)| kind).collect();
    for &kind in sure_protocol::documents::ALL {
        assert!(
            covered.contains(&kind) || excused.contains(&kind),
            "{kind} has a schema and no conformance case"
        );
    }
}

#[test]
fn the_capability_tier_numbers_are_the_ones_the_event_schema_allows() {
    // The tiers are written twice: as the numbers 0/1/2 in the event schema, and
    // as `CapabilityTier::number` in the domain. `wire_contract.rs` compares the
    // *string* names of the tiers against nothing, because the envelope's wire
    // form is numeric and lives in a schema that test does not read.
    //
    // This is the pair that would drift, and the drift would be silent in a
    // specific and dangerous way: a tier added to the domain but not to the
    // schema is a capability adapters cannot report, and one added to the schema
    // but not the domain is a number SURE would refuse from a correct adapter.
    let schema: Value =
        serde_json::from_str(DocumentKind::Event.schema_text()).expect("the event schema is JSON");
    let declared: Vec<u64> = schema["properties"]["capability_tier"]["enum"]
        .as_array()
        .expect("the event schema bounds the tier")
        .iter()
        .map(|value| value.as_u64().expect("a tier is a whole number"))
        .collect();
    let rust: Vec<u64> = CapabilityTier::ALL
        .iter()
        .map(|tier| u64::from(tier.number()))
        .collect();
    assert_eq!(declared, rust);

    // And the numbers are contiguous from zero, so the tier count is what the
    // schema's bounds imply.
    assert_eq!(rust, (0..rust.len() as u64).collect::<Vec<_>>());
}

#[test]
fn a_requirement_is_the_document_the_intent_schema_describes() {
    // `ProjectIntent` is a container of requirements and has no source of its
    // own, so the schema describes the element. Serializing the container would
    // fail on the missing `id`, `source` and `text`.
    let container = sure_domain::intent::ProjectIntent::from_requirements(vec![requirement()]);
    let kind = DocumentKind::ProjectIntent;
    let schema = kind.schema().expect("the schema is enforceable");
    assert!(
        !schema
            .validate(&serde_json::to_value(&container).expect("the container serializes"))
            .is_empty(),
        "the container is not the document the schema describes"
    );
}

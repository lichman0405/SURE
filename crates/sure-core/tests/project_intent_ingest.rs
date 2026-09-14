//! A goal supplied to SURE, recorded and read back.
//!
//! P2-T010 acceptance:
//!
//! > `sure check` can receive/store a trusted explicit goal without requiring raw
//! > transcript recording.
//!
//! The unit tests in `src/project_intent.rs` decide what a goal *becomes*. These
//! decide what happens when it is written: that it lands in the store under the
//! schema for a project intent, that it comes back as the user stated it, that
//! the record carries the trust label rather than being any requirement, and that
//! **no recording row is created** — which is the half of the acceptance that is
//! about privacy rather than about storage, and the half that a test asserting
//! only "a row appeared" would miss.
//!
//! # Why this file opens its own store
//!
//! `Store::open_at` rather than `Store::open`, because the user-level store is
//! the user's: a test that wrote a goal into `%LOCALAPPDATA%\SURE\sure.db` would
//! be adding an invented requirement to somebody's history. Every store here is
//! under `target/tmp`.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};
use sure_core::project_intent::{EXPLICIT_GOAL_ID, explicit_goal, record};
use sure_core::store::{HistoryFilter, RecordKind, Store, StoredRecord};
use sure_domain::ids::FingerprintId;
use sure_domain::intent::{IntentSource, Requirement};
use sure_protocol::documents::DocumentKind;

/// A scratch directory that removes itself.
///
/// Unique per call — process id plus a counter — so nothing has to be cleared
/// before it is used and a leftover from an interrupted run cannot be mistaken
/// for a fresh store.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{test}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("sure project intent")
            .join(unique);
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    /// The project root these records are about, as the store takes it.
    fn project_root(&self) -> String {
        self.path
            .to_str()
            .expect("the fixture path is text")
            .to_owned()
    }

    fn store(&self) -> Store {
        Store::open_at(&self.path.join("sure.db"))
            .unwrap_or_else(|error| panic!("cannot open the store: {error}"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        // Windows keeps file handles longer than Unix does, so the store may
        // still be open somewhere.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Everything the store holds, recordings included.
fn everything(store: &Store) -> Vec<StoredRecord> {
    store
        .history(
            &HistoryFilter {
                include_recordings: true,
                ..HistoryFilter::default()
            },
            100,
        )
        .unwrap_or_else(|error| panic!("cannot read the history: {error}"))
}

/// The one project-intent row in a store, or a panic naming what was there.
fn the_intent_row(store: &Store) -> StoredRecord {
    let rows = store
        .history(&HistoryFilter::default(), 100)
        .unwrap_or_else(|error| panic!("cannot read the history: {error}"));
    let mut intents: Vec<StoredRecord> = rows
        .into_iter()
        .filter(|row| row.kind == RecordKind::Document(DocumentKind::ProjectIntent))
        .collect();
    assert_eq!(
        intents.len(),
        1,
        "expected exactly one project-intent row, found {}",
        intents.len()
    );
    intents.remove(0)
}

#[test]
fn a_goal_supplied_to_sure_is_stored_and_read_back_as_the_user_stated_it() {
    let scratch = Scratch::new("goal-round-trip");
    let store = scratch.store();
    let fingerprint = FingerprintId::generate();

    let goal = "make the upload reject a file over 10 MB instead of failing silently";
    let intent = explicit_goal(goal).expect("a goal with words in it");
    let rows = record(&store, &scratch.project_root(), &fingerprint, &intent).expect("recorded");
    assert_eq!(rows.len(), 1);

    let row = the_intent_row(&store);
    assert_eq!(row.id, rows[0]);
    assert_eq!(
        row.project_root.as_deref(),
        Some(scratch.project_root().as_str())
    );
    assert_eq!(
        row.project_fingerprint.as_deref(),
        Some(fingerprint.as_str())
    );

    // Decoded as the domain type rather than compared as JSON, so that a change
    // to the wire shape that this build cannot read is a failure here rather
    // than a row that reads as something else.
    let requirement: Requirement = row.decode().expect("a requirement");
    assert_eq!(requirement.id, EXPLICIT_GOAL_ID);
    assert_eq!(requirement.text, goal);
    assert_eq!(requirement.source, IntentSource::ExplicitUserGoal);
    assert!(requirement.raw_retained);
    assert!(requirement.is_user_requirement());
}

#[test]
fn storing_a_goal_writes_no_recording() {
    // The acceptance's second half. "Without requiring raw transcript recording"
    // is not only about whether an opt-in was consulted: it is about what is in
    // the file afterwards. A store holding a session transcript next to the goal
    // would satisfy the first reading and fail this one.
    let scratch = Scratch::new("goal-no-recording");
    let store = scratch.store();
    let fingerprint = FingerprintId::generate();

    let intent = explicit_goal("stop writing the API key into the log").expect("a goal");
    record(&store, &scratch.project_root(), &fingerprint, &intent).expect("recorded");

    for row in everything(&store) {
        assert!(
            !row.kind.is_recording(),
            "recording a goal wrote a recording: {}",
            row.document
        );
    }

    let recordings = store
        .history(&HistoryFilter::recordings(), 100)
        .unwrap_or_else(|error| panic!("cannot read the recordings: {error}"));
    assert!(
        recordings.is_empty(),
        "a goal produced {} raw recording(s)",
        recordings.len()
    );
}

#[test]
fn the_row_carries_the_trust_label_and_not_just_the_words() {
    // A stored requirement is only worth anything if a later report can tell
    // where it came from. The label is in the document, which is the only place
    // it survives the store and a later read by a different build.
    let scratch = Scratch::new("goal-trust-label");
    let store = scratch.store();
    let intent = explicit_goal("add a health endpoint").expect("a goal");
    record(
        &store,
        &scratch.project_root(),
        &FingerprintId::generate(),
        &intent,
    )
    .expect("recorded");

    let row = the_intent_row(&store);
    assert_eq!(row.document["source"], json!("explicit_user_goal"));
    assert_eq!(row.document["id"], json!(EXPLICIT_GOAL_ID));
    assert_eq!(row.document["raw_retained"], json!(true));
}

#[test]
fn what_is_written_is_checked_against_the_project_intent_schema() {
    // "The store accepted it" has to mean "it matches the schema", or the tests
    // above would be proving only that some JSON was written. This is the same
    // path with one required field removed, and it must be refused.
    let scratch = Scratch::new("goal-schema-check");
    let store = scratch.store();

    let missing_source = json!({ "id": EXPLICIT_GOAL_ID, "text": "do the thing" });
    let error = store
        .append_for(
            RecordKind::Document(DocumentKind::ProjectIntent),
            &missing_source,
            &scratch.project_root(),
            &FingerprintId::generate(),
        )
        .expect_err("a requirement with no source is not a requirement");
    match error {
        sure_core::store::StoreError::Rejected { kind, violations } => {
            assert_eq!(kind, RecordKind::Document(DocumentKind::ProjectIntent));
            assert!(!violations.is_empty());
        }
        other => panic!("expected a schema refusal, got {other:?}"),
    }

    // And the document this module actually builds does match it, which is what
    // makes the refusal above evidence about the schema rather than about the
    // store refusing everything.
    let intent = explicit_goal("do the thing").expect("a goal");
    record(
        &store,
        &scratch.project_root(),
        &FingerprintId::generate(),
        &intent,
    )
    .expect("a requirement this module built is schema-valid");
}

#[test]
fn a_goal_the_user_replaces_is_added_rather_than_overwritten() {
    // SURE does not edit history. A user who states a new goal has stated
    // something, and the record of the previous one is what lets a report say
    // that what was asked for changed — which is a different finding from a
    // project that failed to do what it was asked.
    let scratch = Scratch::new("goal-replaced");
    let store = scratch.store();

    for goal in [
        "ship the export feature",
        "ship the export feature and the import one",
    ] {
        let intent = explicit_goal(goal).expect("a goal");
        record(
            &store,
            &scratch.project_root(),
            &FingerprintId::generate(),
            &intent,
        )
        .expect("recorded");
    }

    let rows = store
        .history(&HistoryFilter::default(), 100)
        .unwrap_or_else(|error| panic!("cannot read the history: {error}"));
    assert_eq!(rows.len(), 2, "the first goal was overwritten or lost");

    // Newest first, and both readable. The older row is the one a report would
    // use to say that the user changed their mind.
    let texts: Vec<String> = rows
        .iter()
        .map(|row| row.document["text"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert!(texts.contains(&"ship the export feature".to_owned()));
    assert!(
        texts.contains(&"ship the export feature and the import one".to_owned()),
        "the replaced goal was not stored"
    );
}

#[test]
fn a_goal_with_no_words_is_refused_before_the_store_is_touched() {
    // The refusal happens in the value, not in the write, so a caller that
    // checks the error never opens a database to fail at it.
    let scratch = Scratch::new("goal-empty");
    let store = scratch.store();

    let error = explicit_goal("   ").expect_err("whitespace is not a goal");
    assert_eq!(error, sure_core::project_intent::IntentError::EmptyGoal);
    assert!(everything(&store).is_empty());
}

#[test]
fn the_stored_row_survives_a_reopen_and_a_schema_check() {
    // A store is a file another process reads. `integrity_check` is what says
    // the file is sound rather than merely readable by the handle that wrote it.
    let scratch = Scratch::new("goal-reopen");
    let fingerprint = FingerprintId::generate();
    let intent = explicit_goal("keep the nightly backup under 20 minutes").expect("a goal");
    {
        let store = scratch.store();
        record(&store, &scratch.project_root(), &fingerprint, &intent).expect("recorded");
        store
            .integrity_check()
            .unwrap_or_else(|error| panic!("the store is damaged: {error}"));
    }

    let reopened = scratch.store();
    let row = the_intent_row(&reopened);
    let requirement: Requirement = row.decode().expect("a requirement");
    assert_eq!(requirement.text, "keep the nightly backup under 20 minutes");
    assert_eq!(
        row.project_fingerprint.as_deref(),
        Some(fingerprint.as_str())
    );
}

#[test]
fn the_document_is_a_requirement_and_not_the_container_around_it() {
    // `schemas/project-intent.schema.json` describes one requirement. Storing a
    // serialized `ProjectIntent` would produce a document with no `id`, `source`
    // or `text` and be refused — the failure `sure_protocol::documents` warns
    // about — so the shape written is asserted here rather than inferred from
    // the write having succeeded.
    let scratch = Scratch::new("goal-shape");
    let store = scratch.store();
    let intent = explicit_goal("log every rejected login").expect("a goal");
    record(
        &store,
        &scratch.project_root(),
        &FingerprintId::generate(),
        &intent,
    )
    .expect("recorded");

    let row = the_intent_row(&store);
    let object = row
        .document
        .as_object()
        .expect("a requirement is a JSON object");
    assert!(
        !object.contains_key("requirements"),
        "the container was stored instead of its element: {}",
        row.document
    );
    for key in ["id", "source", "text"] {
        assert!(
            object.contains_key(key),
            "the stored requirement has no {key:?}: {}",
            row.document
        );
    }
    let stored: Value = row.document.clone();
    assert_ne!(
        stored,
        serde_json::to_value(&intent).expect("the container")
    );
}

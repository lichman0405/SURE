//! The audit trail of protection decisions: what SURE decided, and why.
//!
//! `P13-T006`'s acceptance is that a **warn, a block and an allow that came from
//! a one-time allowance are recorded locally without secrets**. The vocabulary
//! is not new — [`crate::hook_protection`] has had the three kinds and the three
//! dangers since `P13-T005`, and this module invents none of it. What was
//! missing is a row: [`crate::hook_protection::assess_request`] is pure, its
//! only shipping caller is `sure hook ingest`, and after that process exited the
//! decision did not exist anywhere. This is where it exists.
//!
//! # What is in the row, and what is deliberately not
//!
//! A decision is *about* a request, and for a shell tool the request is a
//! command line — the single string most likely to carry a credential. So the
//! question this module had to answer is not "may SURE store it" but "what does
//! a *reader* need", and the answer is narrower than [`crate::allowance`]'s. A
//! grant stores the subject exactly as the user typed it, because a grant that
//! stored something else could not be matched against the request it is for
//! (`allowance.rs`'s module note). **Nothing matches against a decision.** It is
//! read by a person afterwards, and the request's own words are already recorded
//! once, redacted, in the event payload this row hangs from — so a second copy
//! would buy a reader nothing and widen what a leak of this file hands over.
//! [`DecisionRecord`] therefore carries no subject, no command line and no path.
//!
//! # Where the row hangs
//!
//! [`DecisionRecord::event_id`] is the id of the event the request produced, and
//! it is the join: `SessionEventStore::decisions_for_session` reads the rows back
//! by it and `SessionEventStore::delete_sessions` removes them by it, exactly as
//! they do for a full recording. That is what gives a decision the event's
//! retention and the user's delete — a decision nobody could delete would be a
//! row outside [`docs/security/PRIVACY.md`]'s promise. It is also why
//! `SessionEventStore::persist_decision` refuses to write a row whose event is
//! not in the store: a decision with nothing to hang from would be a row no
//! shipped command can show or remove.
//!
//! # Telling an allow-once from an ordinary allow
//!
//! [`crate::hook_protection::ProtectionDecisionKind`] has three variants, so an
//! allow that happened because a one-time allowance was spent and an allow that
//! happened because nothing was dangerous are the same kind. They are told apart
//! by the two fields beside it: [`DecisionRecord::danger`] is `Some` only where
//! the request was one SURE itself named as dangerous, and
//! [`DecisionRecord::allowance`] names the grant row that was spent. Both are
//! carried from the decision SURE already took — nothing here re-reads a command
//! line or re-decides anything.
//!
//! # A row that could not be written
//!
//! Writing the row is bookkeeping about a decision, and the decision is the
//! answer a harness acts on: `sure hook ingest` exits 0 for allow and warn and 1
//! for block, and that is a contract a launcher depends on. So a failed write
//! **does not change the kind and does not change the exit code**; what it does
//! is say so, in the reason the user is shown ([`not_recorded_reason`]). The
//! alternative — losing the row silently — is the shape of a false green: a
//! history that looks complete and is not.
//!
//! What this module does not claim: both integrations are Observed (Tier 1), so
//! a row here records what SURE decided, never what a harness did with the
//! answer. The same limit every sentence in [`crate::hook_protection`] has.

use serde::{Deserialize, Serialize};
use sure_domain::ids::EventId;

use crate::hook_protection::{Danger, ProtectionDecision, ProtectionDecisionKind};
use crate::store::{RecordKind, StoreError, StoredRecord};

/// What one row of the decision kind holds.
///
/// The fields are the decision SURE reached and the few facts about the request
/// that make it readable later. They are the decision *as it was returned*:
/// [`DecisionRecord::of`] takes it before any note about a missing row is added,
/// because a note about the record is not part of the decision the record is of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// The event this decision was reached about.
    ///
    /// The join, and the only thing that makes the row reachable: it is what
    /// `SessionEventStore` reads the back and deletes by, and it is why a
    /// decision is inside the user's history rather than beside it.
    pub event_id: EventId,
    /// What SURE would do about the request.
    ///
    /// Stored as its wire name, `"allow"` / `"warn"` / `"block"`, which is
    /// [`ProtectionDecisionKind`]'s own `as_str` — the same three words
    /// `sure hook ingest` answers a harness with.
    pub decision: ProtectionDecisionKind,
    /// Which danger the request was held for, when it was one of the three.
    ///
    /// `None` is a real answer and not a missing one: an allow has no danger,
    /// and neither has a block SURE reached for a reason a one-time allowance
    /// may not cover. Its name is [`Danger::wire_name`], not the sentence
    /// [`Danger::as_str`] gives a user.
    pub danger: Option<Danger>,
    /// The tool the harness named, exactly as it sent it.
    ///
    /// The one part of the request that is stored, and the reason is that it is
    /// not *content*: it is the harness's own identifier for the tool (`Bash`,
    /// `Delete`, `Read`), it is what makes one row in a history of many
    /// readable, and the event this row hangs from already holds it in its
    /// payload. Nothing here treats it as evidence of what the tool would do.
    pub tool: String,
    /// The sentence the user was shown, exactly as it was written.
    ///
    /// Stored rather than recomputed, because a later build's sentence is not
    /// the reason *this* decision was taken, and a history that re-worded itself
    /// would be describing a decision nobody made. It is a sentence SURE wrote —
    /// [`crate::hook_protection::danger_reason`] and its siblings never echo the
    /// request's words — and it is redacted on the way in like every other
    /// document.
    pub reason: Option<String>,
    /// The row id of the one-time allowance this decision spent, if it spent
    /// one.
    ///
    /// The grant's id rather than a flag or a copy of the grant, for the reason
    /// [`crate::allowance`] states: the store is append-only and a row id is
    /// stable, so *which* allowance was spent stays answerable from the two rows
    /// rather than from a copy that could drift. **This is what tells an
    /// allow-once from an ordinary allow** — see the module note.
    pub allowance: Option<i64>,
}

impl DecisionRecord {
    /// The row for a decision SURE has just reached.
    ///
    /// Takes the decision rather than its parts so that the one thing a caller
    /// can get wrong — recording a reason that has already been added to, after
    /// a failed write told the user the decision was not recorded — is not a
    /// thing this call site can do.
    #[must_use]
    pub fn of(
        event_id: EventId,
        decision: &ProtectionDecision,
        danger: Option<Danger>,
        tool: &str,
        allowance: Option<i64>,
    ) -> Self {
        Self {
            event_id,
            decision: decision.decision,
            danger,
            tool: tool.to_owned(),
            reason: decision.reason.clone(),
            allowance,
        }
    }
}

/// Read one stored row as a decision.
///
/// # Errors
///
/// [`StoreError::Decode`] if the row is not a decision, which stops the read
/// rather than being skipped — [`crate::allowance`]'s rule and
/// [`crate::approval`]'s, and [`Store::history`]'s.
///
/// [`Store::history`]: crate::store::Store::history
pub fn read_back(row: StoredRecord) -> Result<DecisionRecord, StoreError> {
    debug_assert!(
        row.kind == RecordKind::Decision,
        "the query asked for one kind and got another"
    );
    row.decode()
}

/// Why a decision is not in the local history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotRecorded {
    /// The event the decision was about is not in the store.
    ///
    /// The store is readable and the decision has nowhere to hang: writing the
    /// row anyway would put it outside `sure history` and outside
    /// `sure history delete`, which is worse than not writing it.
    EventMissing,
    /// SURE's store refused the write.
    WriteFailed,
}

/// What a user is told when a decision is not in the local history.
///
/// One sentence per case rather than one for both, because they are different
/// facts: the first says SURE's own event was not kept, the second says the
/// store did not take the row. Both close with the decision standing, because
/// that is the part a user needs to know before they act on it — the answer they
/// were given is the answer SURE reached, and only the record of it is missing.
///
/// It names the command a user would look in, because "not recorded" without
/// "where" is a sentence about SURE's internals. It is deliberately not
/// phrased as a failure of the request: the exit code a harness reads is the
/// decision's, and `sure_core::hook_protection` owns that.
#[must_use]
pub fn not_recorded_reason(why: NotRecorded) -> &'static str {
    match why {
        NotRecorded::EventMissing => {
            "SURE could not record this decision in the local history: the event it belongs to \
             was not stored, and a decision with nothing to hang from would be outside `sure \
             history` and outside `sure history delete`. The decision above stands."
        }
        NotRecorded::WriteFailed => {
            "SURE could not record this decision in the local history: its store refused the \
             write. Nothing else about the decision changed, and `sure history` will not show \
             it."
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::hook_protection::{ProtectionDecision, danger_reason};
    use crate::store::{Store, StoredRecord};

    fn scratch(name: &str) -> std::path::PathBuf {
        crate::store::scratch_root().join(format!("{name}-{}", std::process::id()))
    }

    fn store_in(name: &str) -> Store {
        let dir = scratch(name);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear {}: {error}", dir.display()),
        }
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Store::open_at(&dir.join("sure.db")).expect("the store opens")
    }

    fn a_decision(allowance: Option<i64>) -> (EventId, DecisionRecord) {
        let event_id = EventId::generate();
        let decision = ProtectionDecision::block(danger_reason(Danger::BroadDelete));
        let record = DecisionRecord::of(
            event_id.clone(),
            &decision,
            Some(Danger::BroadDelete),
            "Bash",
            allowance,
        );
        (event_id, record)
    }

    #[test]
    fn a_decision_survives_the_store_and_comes_back_with_every_field() {
        // Through the store rather than through serde alone: what a reader has
        // is a row, and a shape that round-trips in memory and not through
        // `records` would be a row nobody could read.
        let store = store_in("decision-round-trip");
        let (event_id, record) = a_decision(Some(42));
        let row = store
            .append_for(
                RecordKind::Decision,
                &serde_json::to_value(&record).expect("a document"),
                "C:\\work\\app",
                &sure_domain::ids::FingerprintId::generate(),
            )
            .expect("the decision is written");
        let stored = store
            .record(row)
            .expect("a lookup")
            .expect("the row this test just wrote");

        assert_eq!(read_back(stored.clone()).expect("readable"), record);
        assert_eq!(stored.kind.as_str(), "decision");
        assert_eq!(
            stored.document.get("event_id").and_then(|v| v.as_str()),
            Some(event_id.as_str()),
            "the join is in the document, and it is what the delete reads"
        );
    }

    #[test]
    fn a_row_that_is_not_a_decision_stops_the_read_rather_than_being_guessed_at() {
        // The failure this refuses is a reader that shows a plausible-looking
        // decision that SURE never took.
        let row = StoredRecord {
            id: 7,
            kind: RecordKind::Decision,
            document_version: sure_protocol::DOCUMENT_VERSION,
            written_at_ms: 0,
            project_root: None,
            project_fingerprint: None,
            document: serde_json::json!({"decision": "block"}),
        };
        match read_back(row) {
            Err(StoreError::Decode { .. }) => {}
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_row_is_explained_in_the_users_words() {
        // The two cases are different facts and must not arrive as one sentence,
        // and neither may read as a failure of the request: the decision stands
        // either way, and what the user needs is where it would have been.
        let missing = not_recorded_reason(NotRecorded::EventMissing);
        let refused = not_recorded_reason(NotRecorded::WriteFailed);
        assert_ne!(missing, refused);
        for sentence in [missing, refused] {
            assert!(sentence.contains("sure history"), "{sentence}");
            assert!(
                sentence.contains("stands") || sentence.contains("changed"),
                "{sentence}"
            );
            for jargon in [
                "protection.mode",
                "can_grant",
                "session_events",
                "records table",
            ] {
                assert!(
                    !sentence.contains(jargon),
                    "jargon reached a user: {sentence}"
                );
            }
        }
    }
}

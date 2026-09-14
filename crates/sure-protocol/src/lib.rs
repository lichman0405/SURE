//! The versioned wire contract between SURE and everything outside it.
//!
//! Three things live here and nothing else: the documents SURE writes down
//! ([`documents`]), the envelope a harness sends ([`event`]), and the schema
//! checker that keeps the two honest ([`schema`]).
//!
//! # Why it is a separate crate
//!
//! `sure_testkit`'s boundary policy allows `sure-domain` no internal
//! dependencies at all, so anything that needs to know about the schemas cannot
//! live there. It could have gone in `sure-core`, but `sure-core` is where
//! checking logic goes and this is a contract, not a check. Adapting a harness
//! should not mean linking the check engine.
//!
//! # The version
//!
//! [`PROTOCOL_VERSION`] is the version of the event document SURE speaks, and it
//! is the value written into every [`event::EventEnvelope`]'s `schema_version`.
//! It changes when the *format* changes in a way an older reader would get
//! wrong. Adding an optional field does not change it; removing one, renaming
//! one or reinterpreting one does.
//!
//! A reader that meets a version it does not know refuses the document. It does
//! not guess. `docs/architecture/EVENT_PROTOCOL.md` is the prose form of the
//! same contract, and `docs/architecture/PROTOCOL.md` records what the version
//! covers and what it does not.
//!
//! # Two versions, not one
//!
//! [`PROTOCOL_VERSION`] covers the **event** — the one document that arrives
//! from outside, written by an adapter this build did not ship with.
//! [`DOCUMENT_VERSION`] covers the **six documents SURE writes for itself**,
//! and it is stamped onto the stored record rather than onto the document, so
//! that a build reading its own history can tell which build wrote each row.
//!
//! They are separate because they change for different reasons and at different
//! rates. An adapter gaining a field it can now report is a protocol change and
//! nothing to do with how a finding is shaped; a finding gaining a field is a
//! document change and must not force every adapter in the field to renegotiate.
//! Collapsing them would mean every harness integration had to be re-released
//! whenever SURE's own report model moved.

#![forbid(unsafe_code)]

pub mod documents;
pub mod event;
pub mod schema;

/// The event format version this build speaks.
///
/// Written into `EventEnvelope::schema_version` and checked by
/// `EventEnvelope::from_json` before any other field is looked at.
pub const PROTOCOL_VERSION: u32 = 1;

/// The version of the six documents SURE writes for itself.
///
/// Not written into the documents — they carry no version of their own, and
/// `docs/architecture/PROTOCOL.md` says why — but stamped onto the record that
/// stores one, by `sure_core::store`. A stored record whose stamp is newer than
/// this constant was written by a build that knew more, and reading it as
/// though it were current is how a field disappears without anyone noticing.
///
/// It changes under the same rule as [`PROTOCOL_VERSION`]: on a change an older
/// reader would get *wrong*, not on an addition it can safely ignore.
pub const DOCUMENT_VERSION: u32 = 1;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_the_one_the_schemas_advertise() {
        // Pins the pairing. Changing either without the other is the mistake
        // this test exists to catch, because the two are written in different
        // files and would otherwise drift silently.
        assert_eq!(PROTOCOL_VERSION, 1);
        let schema: serde_json::Value =
            serde_json::from_str(documents::DocumentKind::Event.schema_text()).unwrap();
        assert_eq!(
            schema["properties"]["schema_version"]["minimum"],
            serde_json::json!(PROTOCOL_VERSION)
        );
    }

    #[test]
    fn the_stored_document_version_is_not_the_event_version_by_accident() {
        // They are allowed to be equal — they are both 1 today. What is not
        // allowed is for one to be *defined* as the other, which is what
        // `DOCUMENT_VERSION: u32 = PROTOCOL_VERSION` would do and what no test
        // could then tell apart from the current state. This states the pairing
        // so that a change to either is a deliberate change to both.
        assert_eq!(DOCUMENT_VERSION, 1);
        assert_eq!(PROTOCOL_VERSION, 1);
    }
}

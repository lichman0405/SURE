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

#![forbid(unsafe_code)]

pub mod documents;
pub mod event;
pub mod schema;

/// The event format version this build speaks.
///
/// Written into `EventEnvelope::schema_version` and checked by
/// `EventEnvelope::from_json` before any other field is looked at.
pub const PROTOCOL_VERSION: u32 = 1;

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
}

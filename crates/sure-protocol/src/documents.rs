//! The eight documents SURE writes down, and the schemas that describe them.
//!
//! Everything SURE stores, prints as machine-readable output or hands to a
//! harness is one of these. Each has a schema in `schemas/`, and the schemas are
//! compiled into the binary with `include_str!` rather than read from disk at
//! run time. Two reasons, and the second is the important one:
//!
//! 1. An installed `sure.exe` has no `schemas/` directory next to it. A
//!    document that could only be validated from a checkout would be validated
//!    in CI and nowhere else.
//! 2. The schema a document is checked against is therefore the same schema the
//!    release was built from. There is no way for an installation to be
//!    validating against a newer or older schema than its own code, which is the
//!    situation `PROTOCOL_VERSION` exists to detect.
//!
//! # Which Rust type is which document
//!
//! | Schema | Wire form | Rust type |
//! | --- | --- | --- |
//! | `event.schema.json` | harness event | [`crate::event::EventEnvelope`] |
//! | `finding.schema.json` | finding | `sure_domain::vocabulary::Finding` |
//! | `claim.schema.json` | AI claim | `sure_domain::vocabulary::Claim` |
//! | `check-result.schema.json` | check result | `sure_domain::status::CheckResult` |
//! | `project-intent.schema.json` | **one** requirement | `sure_domain::intent::Requirement` |
//! | `repair.schema.json` | repair contract | `sure_domain::vocabulary::RepairContract` |
//! | `repair-envelope.schema.json` | repair envelope | `sure_domain::repair::RepairEnvelope` |
//! | `fixture-expectation.schema.json` | test fixture | `fixtures/` scenario files |
//!
//! The intent row is the one worth reading twice. `ProjectIntent` is a
//! *container* of requirements, so the schema describes its element and not the
//! container. Serializing the container and checking it against the schema would
//! fail on the missing `id`, `source` and `text` — and "fixing" that by adding
//! those fields to the container would be wrong, because the container has no
//! source of its own: it is the union of its requirements' sources.
//!
//! # The schemas are lower bounds
//!
//! None of them sets `additionalProperties: false`, so a Rust type with more
//! fields than the schema mentions is conformant, and that is deliberate: the
//! domain types carry SURE's own identity and provenance (`fingerprint`,
//! `session`, `next_step`) which the wire form does not need. What the schemas
//! pin is the part a *reader* depends on.

use std::fmt;

use crate::schema::{Schema, SchemaError};

/// One of the documents SURE writes.
///
/// Exhaustive on purpose: adding a document means adding a schema and a variant,
/// and the round-trip test in `tests/conformance.rs` fails until both exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentKind {
    /// A normalized harness event, as an adapter sends it.
    Event,
    /// A material problem or uncertainty worth reporting.
    Finding,
    /// A statement a coding agent made, and what checking it produced.
    Claim,
    /// The outcome of one check.
    CheckResult,
    /// One requirement or goal, with where it came from.
    ProjectIntent,
    /// Bounded repair instructions for a coding harness.
    Repair,
    /// Harness-neutral wrapper that delivers the same repair contract to any
    /// adapter.
    RepairEnvelope,
    /// What an evaluation fixture requires a run to produce.
    FixtureExpectation,
}

/// Every document, in the order the schemas are listed above.
pub const ALL: &[DocumentKind] = &[
    DocumentKind::Event,
    DocumentKind::Finding,
    DocumentKind::Claim,
    DocumentKind::CheckResult,
    DocumentKind::ProjectIntent,
    DocumentKind::Repair,
    DocumentKind::RepairEnvelope,
    DocumentKind::FixtureExpectation,
];

impl DocumentKind {
    /// The stable name, without the file extension.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Finding => "finding",
            Self::Claim => "claim",
            Self::CheckResult => "check-result",
            Self::ProjectIntent => "project-intent",
            Self::Repair => "repair",
            Self::RepairEnvelope => "repair-envelope",
            Self::FixtureExpectation => "fixture-expectation",
        }
    }

    /// The schema file's name inside `schemas/`.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Event => "event.schema.json",
            Self::Finding => "finding.schema.json",
            Self::Claim => "claim.schema.json",
            Self::CheckResult => "check-result.schema.json",
            Self::ProjectIntent => "project-intent.schema.json",
            Self::Repair => "repair.schema.json",
            Self::RepairEnvelope => "repair-envelope.schema.json",
            Self::FixtureExpectation => "fixture-expectation.schema.json",
        }
    }

    /// The schema text, compiled into the binary.
    ///
    /// `include_str!` rather than a run-time read: see the module documentation.
    #[must_use]
    pub const fn schema_text(self) -> &'static str {
        match self {
            Self::Event => include_str!("../../../schemas/event.schema.json"),
            Self::Finding => include_str!("../../../schemas/finding.schema.json"),
            Self::Claim => include_str!("../../../schemas/claim.schema.json"),
            Self::CheckResult => include_str!("../../../schemas/check-result.schema.json"),
            Self::ProjectIntent => include_str!("../../../schemas/project-intent.schema.json"),
            Self::Repair => include_str!("../../../schemas/repair.schema.json"),
            Self::RepairEnvelope => {
                include_str!("../../../schemas/repair-envelope.schema.json")
            }
            Self::FixtureExpectation => {
                include_str!("../../../schemas/fixture-expectation.schema.json")
            }
        }
    }

    /// The schema, parsed and checked for enforceability.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaError`] if the embedded schema is not JSON, or uses a
    /// keyword the validator does not enforce. Both are build-time mistakes that
    /// this call turns into a visible failure rather than a check that quietly
    /// does less than it says.
    pub fn schema(self) -> Result<Schema, SchemaError> {
        Schema::parse(self.file_name(), self.schema_text())
    }

    /// How many keys the schema requires at the top level.
    ///
    /// Used by the conformance test to prove the validator is not vacuous, and
    /// by nothing else. A schema with no required keys would validate `{}`,
    /// which would make every conformance claim in this module worthless.
    #[must_use]
    pub fn required_keys(self) -> Vec<String> {
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(self.schema_text()) else {
            return Vec::new();
        };
        parsed
            .get("required")
            .and_then(|required| required.as_array())
            .map(|keys| {
                keys.iter()
                    .filter_map(|key| key.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl fmt::Display for DocumentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_document_has_a_distinct_name_and_file() {
        let names: BTreeSet<_> = ALL.iter().map(|kind| kind.as_str()).collect();
        let files: BTreeSet<_> = ALL.iter().map(|kind| kind.file_name()).collect();
        assert_eq!(names.len(), ALL.len());
        assert_eq!(files.len(), ALL.len());
    }

    #[test]
    fn every_embedded_schema_parses_and_is_fully_enforceable() {
        // The build-time half of the conformance story: a schema that grew a
        // keyword the validator does not implement stops here rather than
        // passing unnoticed.
        for &kind in ALL {
            kind.schema()
                .unwrap_or_else(|error| panic!("{kind} did not parse: {error}"));
        }
    }

    #[test]
    fn every_schema_requires_at_least_one_key() {
        // A schema with nothing required would accept `{}`, and a conformance
        // test against it would prove nothing at all.
        for &kind in ALL {
            assert!(
                !kind.required_keys().is_empty(),
                "{kind}'s schema requires nothing"
            );
        }
    }

    #[test]
    fn each_schema_is_titled_and_declares_the_draft() {
        for &kind in ALL {
            let parsed: serde_json::Value =
                serde_json::from_str(kind.schema_text()).expect("the schema is JSON");
            assert!(
                parsed.get("title").and_then(|t| t.as_str()).is_some(),
                "{kind} has no title"
            );
            assert_eq!(
                parsed.get("$schema").and_then(|s| s.as_str()),
                Some("https://json-schema.org/draft/2020-12/schema"),
                "{kind} does not declare the draft"
            );
        }
    }

    #[test]
    fn the_validator_rejects_an_empty_document_for_every_schema() {
        // The blunt instrument check: if this passed for some schema, that
        // schema's conformance test would be worthless.
        for &kind in ALL {
            let schema = kind.schema().expect("the schema is enforceable");
            assert!(
                !schema.validate(&serde_json::json!({})).is_empty(),
                "{kind} accepted an empty document"
            );
        }
    }
}

//! Why the store could not do what it was asked.
//!
//! Every variant follows the rule `crate::paths::PathError` follows: the message
//! says what SURE did *instead*, not only what went wrong. A user who reads
//! "the database is locked" does not know whether their event was recorded. A
//! user who reads "nothing was written and this event was not recorded" does.
//!
//! `PartialEq` and not `Eq`: [`StoreError::Rejected`] carries
//! [`sure_protocol::schema::Violation`], which holds an `f64` minimum read out
//! of a schema, and an `f64` has no total order. Claiming `Eq` here would be a
//! claim about a type that does not have one — the same reasoning
//! `sure_protocol::schema` gives for its own types.

use std::fmt;
use std::path::PathBuf;

use sure_protocol::schema::Violation;

use crate::paths::PathError;
use crate::store::migrations::MigrationError;
use crate::store::record::RecordKind;

/// Why the store could not do what it was asked.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// The store would have been inside the project it describes.
    ///
    /// Carried through from [`crate::paths::Paths::ensure_outside`] rather than
    /// restated, so there is one wording for the rule.
    Location(PathError),
    /// The directory the store lives in could not be created.
    CreateDirectory {
        /// The directory.
        path: PathBuf,
        /// What the filesystem said.
        message: String,
    },
    /// The database file could not be opened.
    Open {
        /// The file.
        path: PathBuf,
        /// What SQLite said.
        message: String,
    },
    /// The schema could not be brought up to date.
    Migration(MigrationError),
    /// Another SURE process held the write lock for longer than SURE waits.
    ///
    /// This is a *defined* outcome, not an accident: see the concurrency
    /// contract in `crate::store`. Nothing was written, and the write that
    /// timed out is reported rather than retried forever or dropped.
    Busy {
        /// The file.
        path: PathBuf,
        /// How long SURE waited before giving up.
        waited_ms: u64,
    },
    /// The document does not match the schema for its kind, so it was not
    /// written.
    Rejected {
        /// The kind it was offered as.
        kind: RecordKind,
        /// Every way it disagreed with the schema.
        violations: Vec<Violation>,
    },
    /// The kind is one SURE never writes as a record about a project.
    ///
    /// One kind is in this position, [`sure_protocol::documents::DocumentKind::FixtureExpectation`],
    /// and it is a bug in SURE if this is ever returned — the same standing as
    /// [`StoreError::Schema`].
    NotStorable {
        /// The kind it was offered as.
        kind: RecordKind,
    },
    /// A stored row names a kind this build does not know.
    UnknownKind {
        /// The row.
        id: i64,
        /// The name it carried.
        name: String,
    },
    /// A stored row's document is not readable.
    MalformedRow {
        /// The row.
        id: i64,
        /// What serde said.
        message: String,
    },
    /// The row was written by a build that knew a newer document format.
    NewerDocument {
        /// The kind it claims to be.
        kind: RecordKind,
        /// The version it was stamped with.
        found: u32,
        /// The version this build knows.
        supported: u32,
    },
    /// The document is readable JSON but not the shape the type expects.
    Decode {
        /// The row.
        id: i64,
        /// The kind it claims to be.
        kind: RecordKind,
        /// What serde said.
        message: String,
    },
    /// A schema SURE ships could not be parsed or is not fully enforceable.
    ///
    /// A bug in SURE rather than a user condition: the schemas are compiled
    /// into the binary and `crates/sure-protocol/tests/conformance.rs` checks
    /// them at build time. It is reported rather than unwrapped because the
    /// alternative is a check that silently does less than it says.
    Schema {
        /// The kind whose schema could not be used.
        kind: RecordKind,
        /// What the schema parser said.
        message: String,
    },
    /// The database failed its own consistency check.
    Integrity {
        /// What SQLite reported.
        problems: Vec<String>,
    },
}

impl From<MigrationError> for StoreError {
    fn from(error: MigrationError) -> Self {
        Self::Migration(error)
    }
}

impl From<PathError> for StoreError {
    fn from(error: PathError) -> Self {
        Self::Location(error)
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The path message already says what SURE did instead, so wrapping
            // it in another sentence would only bury it.
            Self::Location(error) => write!(f, "{error}"),
            Self::CreateDirectory { path, message } => write!(
                f,
                "SURE could not create its data directory at {}.\n\n\
                 {message}\n\n\
                 Nothing was written. SURE keeps evidence outside the project, so it has nowhere \
                 else it is willing to put it.",
                path.display()
            ),
            Self::Open { path, message } => write!(
                f,
                "SURE could not open its history file at {}.\n\n\
                 {message}\n\n\
                 Nothing was written or read.",
                path.display()
            ),
            Self::Migration(error) => write!(f, "{error}"),
            Self::Busy { path, waited_ms } => write!(
                f,
                "SURE waited {waited_ms} ms for another SURE process to finish writing to {} and \
                 then gave up.\n\n\
                 Another SURE process holds the write lock. Nothing was written, so the file is \
                 unchanged — but the record this command was saving was not saved.\n\n\
                 Run the command again, or wait for the other SURE process to finish.",
                path.display()
            ),
            Self::Rejected { kind, violations } => {
                writeln!(
                    f,
                    "SURE refused to store this {kind}: it does not match \
                     schemas/{}.schema.json.",
                    kind.document()
                        .map_or("(none)", |document| document.as_str())
                )?;
                for violation in violations {
                    writeln!(f, "  - {violation}")?;
                }
                f.write_str(
                    "\nNothing was written. A stored record that does not match its schema would \
                     be read back later as though it did, so SURE rejects it here where the \
                     mistake can still be seen.",
                )
            }
            Self::NotStorable { kind } => write!(
                f,
                "SURE was asked to store a {kind} as a record about a project, and a {kind} is \
                 not one.\n\n\
                 Nothing was written. A fixture expectation says what an evaluation run must \
                 produce; it is read from `fixtures/`, and stored in a history it would read \
                 back as a result.\n\n\
                 This is a bug in SURE rather than a problem with your project."
            ),
            Self::UnknownKind { id, name } => write!(
                f,
                "Record {id} in SURE's history is a \"{name}\", which this build does not know \
                 about.\n\n\
                 SURE skipped it. A record written by a newer SURE cannot be described in the \
                 vocabulary of an older one, and guessing would put it in a report under a name \
                 that means something else.\n\n\
                 Update SURE to read it.",
            ),
            Self::MalformedRow { id, message } => write!(
                f,
                "Record {id} in SURE's history could not be read.\n\n\
                 {message}\n\n\
                 SURE skipped it. The rest of the history is unaffected.",
            ),
            Self::NewerDocument {
                kind,
                found,
                supported,
            } => write!(
                f,
                "This {kind} was written by a newer SURE, in document format {found}; this build \
                 understands up to format {supported}.\n\n\
                 SURE stopped rather than read it as though it were current. A field this build \
                 does not know would disappear on the way in, and the record would then read as \
                 one that never had it.\n\n\
                 Update SURE to read this record.",
            ),
            Self::Decode { id, kind, message } => write!(
                f,
                "Record {id} in SURE's history is a {kind} that SURE could not read back.\n\n\
                 {message}\n\n\
                 This is damaged or hand-edited data rather than a newer SURE. SURE skipped it; \
                 the rest of the history is unaffected.",
            ),
            Self::Schema { kind, message } => write!(
                f,
                "SURE could not use the schema for a {kind}.\n\n\
                 {message}\n\n\
                 Nothing was written. This is a bug in SURE rather than a problem with your \
                 project or your machine."
            ),
            Self::Integrity { problems } => {
                f.write_str("SURE's history file failed its own consistency check.\n\n")?;
                for problem in problems.iter().take(10) {
                    writeln!(f, "  - {problem}")?;
                }
                f.write_str(
                    "\nSURE stopped rather than read or write a file it cannot trust. A record \
                     read out of a damaged file is a wrong answer about a project.\n\n\
                     Move the file aside and let SURE create a new one; the history it held is \
                     already lost.",
                )
            }
        }
    }
}

impl std::error::Error for StoreError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_protocol::documents::DocumentKind;

    /// Every variant, so a new one cannot be added without a message.
    fn every_error() -> Vec<StoreError> {
        vec![
            StoreError::Location(PathError::Unavailable {
                what: "its history",
            }),
            StoreError::CreateDirectory {
                path: PathBuf::from("/data/SURE"),
                message: "access denied".to_owned(),
            },
            StoreError::Open {
                path: PathBuf::from("/data/SURE/sure.db"),
                message: "not a database".to_owned(),
            },
            StoreError::Migration(MigrationError::NewerSchema {
                found: 9,
                supported: 1,
            }),
            StoreError::Busy {
                path: PathBuf::from("/data/SURE/sure.db"),
                waited_ms: 5000,
            },
            StoreError::Rejected {
                kind: RecordKind::Document(DocumentKind::Finding),
                violations: Vec::new(),
            },
            StoreError::NotStorable {
                kind: RecordKind::Document(DocumentKind::FixtureExpectation),
            },
            StoreError::UnknownKind {
                id: 3,
                name: "telemetry".to_owned(),
            },
            StoreError::MalformedRow {
                id: 4,
                message: "expected value".to_owned(),
            },
            StoreError::NewerDocument {
                kind: RecordKind::Document(DocumentKind::Finding),
                found: 2,
                supported: 1,
            },
            StoreError::Decode {
                id: 5,
                kind: RecordKind::Recording,
                message: "expected a string".to_owned(),
            },
            StoreError::Schema {
                kind: RecordKind::Document(DocumentKind::Repair),
                message: "unsupported keyword \"$ref\"".to_owned(),
            },
            StoreError::Integrity {
                problems: vec!["row 3 is missing from index".to_owned()],
            },
        ]
    }

    #[test]
    fn every_error_says_what_sure_did_instead() {
        for error in every_error() {
            let text = error.to_string();
            assert!(
                text.contains("SURE"),
                "no statement of what SURE did:\n{text}"
            );
            assert!(
                text.lines().filter(|line| !line.trim().is_empty()).count() >= 2,
                "a single statement with no next step:\n{text}"
            );
        }
    }

    #[test]
    fn no_error_message_is_empty_or_ends_mid_sentence() {
        for error in every_error() {
            let text = error.to_string();
            assert!(!text.trim().is_empty(), "{error:?} has no message");
            let last = text.trim_end().chars().last().unwrap_or(' ');
            assert!(
                matches!(last, '.' | '!' | '?'),
                "{error:?} does not end in a sentence:\n{text}"
            );
        }
    }

    #[test]
    fn a_busy_error_says_the_record_was_not_saved() {
        // The most important message in this file. "Database is locked" leaves
        // the reader to guess whether their event survived; a silent drop would
        // be a claim that nothing happened.
        let text = StoreError::Busy {
            path: PathBuf::from("/data/SURE/sure.db"),
            waited_ms: 5000,
        }
        .to_string();
        assert!(text.contains("was not saved"), "{text}");
        assert!(text.contains("5000"), "{text}");
    }

    #[test]
    fn a_rejected_document_lists_every_violation() {
        let text = StoreError::Rejected {
            kind: RecordKind::Document(DocumentKind::Finding),
            violations: vec![
                Violation {
                    schema: "finding.schema.json".to_owned(),
                    path: "".to_owned(),
                    kind: sure_protocol::schema::ViolationKind::Missing {
                        key: "status".to_owned(),
                    },
                },
                Violation {
                    schema: "finding.schema.json".to_owned(),
                    path: "severity".to_owned(),
                    kind: sure_protocol::schema::ViolationKind::NotAllowed {
                        allowed: vec!["must_fix".to_owned()],
                    },
                },
            ],
        }
        .to_string();
        assert!(text.contains("finding.schema.json"), "{text}");
        assert!(text.contains("status"), "{text}");
        assert!(text.contains("severity"), "{text}");
    }
}

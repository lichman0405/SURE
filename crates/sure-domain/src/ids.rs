//! Typed identifiers.
//!
//! Every domain entity that can be referenced from evidence, a report or a
//! repair contract carries a typed ID rather than a bare string. The string
//! form is deliberately narrow so that an ID can be safely embedded in a file
//! name, a URL fragment or a terminal report: `<prefix>_<lowercase base32>`.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::variants::variants;
use serde::{Deserialize, Serialize};

/// Which kind of entity an identifier refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdKind {
    /// A local project under inspection.
    Project,
    /// A fingerprint of a project state.
    Fingerprint,
    /// A recorded harness session.
    Session,
    /// A normalized harness event.
    Event,
    /// One execution of SURE itself.
    ///
    /// Distinct from [`IdKind::Session`], which is a *harness* session that
    /// SURE observed. A run is SURE's own work: it exists whether or not a
    /// harness was present, and it is what a stored verdict, a diagnostic line
    /// and a history entry are all correlated by.
    Run,
    /// A planned check.
    Check,
    /// A material user-facing finding.
    Finding,
    /// A bounded repair contract.
    Repair,
    /// A claim made by a coding agent.
    Claim,
    /// A captured piece of evidence.
    Evidence,
}

variants!(IdKind {
    Project,
    Fingerprint,
    Session,
    Event,
    Run,
    Check,
    Finding,
    Repair,
    Claim,
    Evidence
});

impl IdKind {
    /// The canonical string prefix for this kind.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Project => "prj",
            Self::Fingerprint => "fp",
            Self::Session => "ses",
            Self::Event => "evt",
            Self::Run => "run",
            Self::Check => "chk",
            Self::Finding => "fnd",
            Self::Repair => "rep",
            Self::Claim => "clm",
            Self::Evidence => "evd",
        }
    }

    /// Resolve a prefix back to its kind.
    #[must_use]
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.prefix() == prefix)
    }
}

/// Why a string could not be accepted as an identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    /// The string was empty.
    Empty,
    /// The string did not contain the `<prefix>_<body>` separator.
    MissingSeparator,
    /// The prefix is not one of the known kinds.
    UnknownPrefix,
    /// The body was empty or contained characters outside `[a-z0-9]`.
    MalformedBody,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Empty => "identifier is empty",
            Self::MissingSeparator => "identifier has no '<prefix>_<body>' separator",
            Self::UnknownPrefix => "identifier prefix is not a known SURE id kind",
            Self::MalformedBody => "identifier body must be non-empty lowercase alphanumeric",
        };
        f.write_str(text)
    }
}

impl std::error::Error for IdError {}

fn is_body_char(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_lowercase()
}

fn validate(value: &str) -> Result<IdKind, IdError> {
    if value.is_empty() {
        return Err(IdError::Empty);
    }
    let Some((prefix, body)) = value.split_once('_') else {
        return Err(IdError::MissingSeparator);
    };
    let kind = IdKind::from_prefix(prefix).ok_or(IdError::UnknownPrefix)?;
    if body.is_empty() || !body.chars().all(is_body_char) {
        return Err(IdError::MalformedBody);
    }
    Ok(kind)
}

/// Declare a typed identifier newtype.
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $kind:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// The identifier kind.
            pub const KIND: IdKind = $kind;

            /// Parse and validate an identifier string.
            ///
            /// # Errors
            /// Returns [`IdError`] when the value is not a well-formed identifier of this kind.
            pub fn parse(value: impl Into<String>) -> Result<Self, IdError> {
                let value = value.into();
                match validate(&value)? {
                    k if k == Self::KIND => Ok(Self(value)),
                    _ => Err(IdError::UnknownPrefix),
                }
            }

            /// The identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Generate a fresh identifier.
            #[must_use]
            pub fn generate() -> Self {
                Self(format!("{}_{}", Self::KIND.prefix(), fresh_body()))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

define_id!(
    /// Stable identity for a project under inspection.
    ProjectId,
    IdKind::Project
);
define_id!(
    /// Identity of exactly one project state.
    FingerprintId,
    IdKind::Fingerprint
);
define_id!(
    /// Identity of one observed harness session.
    SessionId,
    IdKind::Session
);
define_id!(
    /// Identity of one normalized harness event.
    EventId,
    IdKind::Event
);
define_id!(
    /// Identity of one execution of SURE.
    RunId,
    IdKind::Run
);
define_id!(
    /// Identity of one planned or executed check.
    CheckId,
    IdKind::Check
);
define_id!(
    /// Identity of one user-facing finding.
    FindingId,
    IdKind::Finding
);
define_id!(
    /// Identity of one repair contract.
    RepairId,
    IdKind::Repair
);
define_id!(
    /// Identity of one agent claim.
    ClaimId,
    IdKind::Claim
);
define_id!(
    /// Identity of one evidence record.
    EvidenceId,
    IdKind::Evidence
);

/// The number of generated characters in an identifier body.
const BODY_LEN: usize = 20;
const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Process-wide entropy source for generated identifier bodies.
///
/// A monotonic counter plus a per-process seed derived from the clock and the
/// process id. This is not a cryptographic identifier and is not used for
/// anything security-relevant; it only has to be unique on one machine. A
/// persisted, project-scoped seed is added in P1 with local storage.
fn entropy() -> u64 {
    static SEED: AtomicU64 = AtomicU64::new(0);
    let existing = SEED.load(Ordering::Relaxed);
    if existing != 0 {
        return existing;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mixed = now
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .rotate_left(31)
        .wrapping_add(u64::from(std::process::id()));
    // Only the first writer wins; a racing writer observes the same seed.
    let _ = SEED.compare_exchange(0, mixed, Ordering::Relaxed, Ordering::Relaxed);
    let resolved = SEED.load(Ordering::Relaxed);
    if resolved == 0 { mixed } else { resolved }
}

/// Build a fresh random body. Never returns an empty string.
fn fresh_body() -> String {
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut state = entropy() ^ counter.wrapping_mul(0x2545_f491_4f6c_dd1d);
    let mut out = String::with_capacity(BODY_LEN);
    for _ in 0..BODY_LEN {
        // xorshift64*: adequate for identifier uniqueness, never used as a secret.
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let bits = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        let idx = (bits >> 59) as usize;
        out.push(char::from(ALPHABET.get(idx).copied().unwrap_or(b'0')));
    }
    out
}

/// A kind-checked identifier of any known kind.
///
/// Used where a caller must accept "some identifier" — for example an evidence
/// anchor that may point at a check, a finding or an event.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AnyId {
    kind: IdKind,
    value: String,
}

impl AnyId {
    /// Parse an identifier of any known kind.
    ///
    /// # Errors
    /// Returns [`IdError`] when the value is not a well-formed identifier.
    pub fn parse(value: &str) -> Result<Self, IdError> {
        let kind = validate(value)?;
        Ok(Self {
            kind,
            value: value.to_owned(),
        })
    }

    /// The kind of entity this identifier refers to.
    #[must_use]
    pub const fn kind(&self) -> IdKind {
        self.kind
    }

    /// The identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for AnyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl TryFrom<String> for AnyId {
    type Error = IdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<AnyId> for String {
    fn from(value: AnyId) -> Self {
        value.value
    }
}

/// Implement the conversions that let a concrete ID be used as an [`AnyId`].
macro_rules! any_id_from {
    ($($name:ident),* $(,)?) => {
        $(
            impl From<$name> for AnyId {
                fn from(value: $name) -> Self {
                    Self { kind: <$name>::KIND, value: value.0 }
                }
            }

            impl From<&$name> for AnyId {
                fn from(value: &$name) -> Self {
                    Self { kind: <$name>::KIND, value: value.0.clone() }
                }
            }
        )*
    };
}

any_id_from!(
    ProjectId,
    FingerprintId,
    SessionId,
    EventId,
    RunId,
    CheckId,
    FindingId,
    RepairId,
    ClaimId,
    EvidenceId,
);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_round_trip_and_carry_the_right_prefix() {
        let id = CheckId::generate();
        assert!(id.as_str().starts_with("chk_"));
        assert_eq!(CheckId::parse(id.as_str()), Ok(id.clone()));
        assert_eq!(
            AnyId::parse(id.as_str()).map(|a| a.kind()),
            Ok(IdKind::Check)
        );
    }

    #[test]
    fn every_kind_has_a_distinct_prefix() {
        let prefixes: std::collections::BTreeSet<_> =
            IdKind::ALL.iter().map(|k| k.prefix()).collect();
        assert_eq!(prefixes.len(), IdKind::ALL.len());
        for &kind in IdKind::ALL {
            assert_eq!(IdKind::from_prefix(kind.prefix()), Some(kind));
        }
    }

    #[test]
    fn generated_ids_are_unique_across_a_burst() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..2000 {
            assert!(seen.insert(FindingId::generate().as_str().to_owned()));
        }
    }

    #[test]
    fn an_id_cannot_be_used_as_a_different_kind() {
        let finding = FindingId::generate();
        assert_eq!(
            CheckId::parse(finding.as_str()),
            Err(IdError::UnknownPrefix)
        );
    }

    #[test]
    fn malformed_ids_are_rejected() {
        assert_eq!(CheckId::parse(""), Err(IdError::Empty));
        assert_eq!(CheckId::parse("chk"), Err(IdError::MissingSeparator));
        assert_eq!(CheckId::parse("zzz_abc"), Err(IdError::UnknownPrefix));
        assert_eq!(CheckId::parse("chk_"), Err(IdError::MalformedBody));
        assert_eq!(CheckId::parse("chk_ABC"), Err(IdError::MalformedBody));
        assert_eq!(CheckId::parse("chk_a-b"), Err(IdError::MalformedBody));
        assert_eq!(CheckId::parse("chk_has space"), Err(IdError::MalformedBody));
    }

    #[test]
    fn an_id_body_is_safe_for_filenames_and_urls() {
        for _ in 0..200 {
            let id = EventId::generate();
            let body = id.as_str().trim_start_matches("evt_");
            assert!(
                body.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            );
        }
    }

    #[test]
    fn ids_round_trip_through_json() {
        let id = SessionId::generate();
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, format!("\"{id}\""));
        let back: SessionId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn a_malformed_id_is_rejected_when_deserialized() {
        assert!(serde_json::from_str::<CheckId>("\"nonsense\"").is_err());
        assert!(serde_json::from_str::<CheckId>("\"fnd_abc\"").is_err());
    }

    #[test]
    fn any_id_accepts_every_kind_and_round_trips() {
        for &kind in IdKind::ALL {
            let value = format!("{}_abc123", kind.prefix());
            let any = AnyId::parse(&value).expect("valid");
            assert_eq!(any.kind(), kind);
            assert_eq!(any.as_str(), value);
        }
        let json = serde_json::to_string(&AnyId::parse("chk_abc").expect("valid")).expect("ser");
        assert_eq!(json, "\"chk_abc\"");
        assert!(serde_json::from_str::<AnyId>("\"notanid\"").is_err());
    }

    #[test]
    fn an_error_message_never_leaks_the_rejected_value_shape_as_advice() {
        // Sanity check that Display is implemented and readable for every variant.
        for error in [
            IdError::Empty,
            IdError::MissingSeparator,
            IdError::UnknownPrefix,
            IdError::MalformedBody,
        ] {
            assert!(!error.to_string().is_empty());
        }
    }
}

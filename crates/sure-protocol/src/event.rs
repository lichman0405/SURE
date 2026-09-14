//! The harness event envelope: what an adapter sends SURE.
//!
//! # Why this is not `sure_domain::vocabulary::Event`
//!
//! They are two ends of the same event and they answer different questions.
//!
//! | | [`EventEnvelope`] | `Event` |
//! | --- | --- | --- |
//! | Direction | in, from an adapter | stored, in SURE's own records |
//! | Identity | the harness's | SURE's `EventId` |
//! | Versioned | yes, `schema_version` | no, it is already SURE's own shape |
//! | Required fields | `schema_version`, `source`, `event_type`, `timestamp` | `id`, `session`, `event_type`, `timestamp` |
//!
//! The domain `Event` cannot satisfy `schemas/event.schema.json`: it has no
//! `schema_version` and no `source`, because by the time SURE has stored an
//! event those two questions are answered. Giving the domain type a
//! `schema_version` so that one document could serve both would mean a stored
//! record claiming a wire version it is not governed by.
//!
//! # The session id is the harness's, not SURE's
//!
//! [`EventEnvelope::session_id`] is `Option<String>`, not
//! `Option<SessionId>`. It is whatever the harness calls its own session, which
//! SURE has no way to validate and no right to interpret. SURE's `SessionId` is
//! assigned when the session is recorded, and it means "this session as SURE
//! stored it". Converting one into the other would be a lossy guess dressed up
//! as a type conversion, so the two are kept apart and the adapter records the
//! mapping where it can.
//!
//! # Versions
//!
//! `schema_version` is the version of *this document*, and it is checked before
//! anything else in [`EventEnvelope::from_json`]. A document from a newer SURE
//! is refused as a version mismatch rather than as a parse failure, because the
//! two need different things from the person reading the message: one needs a
//! newer SURE, the other needs a bug report.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use sure_domain::capability::CapabilityTier;

use crate::PROTOCOL_VERSION;
use crate::documents::DocumentKind;
use crate::handshake::negotiate;
use crate::schema::Violation;

/// A normalized harness event, in the shape an adapter sends it.
///
/// `docs/architecture/EVENT_PROTOCOL.md` is the prose form of this type. Every
/// field except the four required ones is optional, because "adapters may omit
/// fields not exposed by their harness", and missing data is not invented: an
/// absent `capability_tier` means the adapter did not say, which is a different
/// statement from tier 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    /// The version of this document. See [`PROTOCOL_VERSION`].
    pub schema_version: u32,
    /// The harness that produced the event, for example `claude-code`.
    pub source: String,
    /// The semantic event name, for example `tool.completed`.
    pub event_type: String,
    /// When it happened, as an RFC 3339 timestamp.
    pub timestamp: String,
    /// How much of the session the integration could see.
    ///
    /// A number on the wire, matching the tiers in
    /// `sure_domain::capability::CapabilityTier`. Serialized through
    /// `CapabilityTier::number` rather than the domain type's own serde form,
    /// which is a string for use in reports.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "tier::serialize",
        deserialize_with = "tier::deserialize"
    )]
    pub capability_tier: Option<CapabilityTier>,
    /// The harness's own identifier for the session. See the module docs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The project the event happened in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    /// The event body, already normalized and redacted.
    ///
    /// Always an object, never `null`. An event with no body has an empty body;
    /// writing `null` would make "the adapter sent nothing" and "the adapter
    /// sent an empty body" the same document, and the schema would refuse the
    /// former anyway.
    #[serde(default = "empty_payload")]
    pub payload: Value,
}

/// The default event body: an empty object.
fn empty_payload() -> Value {
    Value::Object(serde_json::Map::new())
}

/// `capability_tier` is a number on the wire and a tier in Rust.
///
/// The domain type serializes as a string for reports; the envelope needs the
/// numeric form `docs/architecture/EVENT_PROTOCOL.md` shows. Keeping the
/// conversion here rather than on the domain type means a report cannot
/// accidentally start printing `1` instead of `observed`.
mod tier {
    use super::{CapabilityTier, Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        value: &Option<CapabilityTier>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(tier) => serializer.serialize_u8(tier.number()),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<CapabilityTier>, D::Error> {
        let Some(number) = Option::<u8>::deserialize(deserializer)? else {
            return Ok(None);
        };
        CapabilityTier::from_number(number)
            .map(Some)
            .ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "capability_tier {number} is not a tier SURE knows about. \
                 The tiers are 0 (snapshot), 1 (observed) and 2 (protected)."
                ))
            })
    }
}

/// Why an envelope could not be accepted.
///
/// Not `Eq`, because [`EnvelopeError::NotConformant`] carries violations and a
/// violation can carry an `f64` minimum. See [`ViolationKind`].
#[derive(Debug, Clone, PartialEq)]
pub enum EnvelopeError {
    /// The text was not JSON.
    NotJson {
        /// What the parser said.
        message: String,
    },
    /// The document did not say which format version it is.
    MissingVersion,
    /// The document is a format version this build does not speak.
    UnsupportedVersion {
        /// The version in the document.
        found: u32,
        /// The version this build speaks.
        supported: u32,
    },
    /// The document is JSON but not an event envelope.
    NotConformant {
        /// Every way it disagreed with the schema.
        violations: Vec<Violation>,
    },
    /// The document satisfied the schema but not this type.
    ///
    /// Kept separate from [`EnvelopeError::NotConformant`] rather than merged:
    /// a violation here is a gap between the schema and this type, which is a
    /// mistake in SURE rather than in the sender.
    Malformed {
        /// What the parser said.
        message: String,
    },
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson { message } => {
                write!(
                    f,
                    "this event is not valid JSON, so SURE cannot read it: {message}"
                )
            }
            Self::MissingVersion => f.write_str(
                "this event does not say which event format it is, so SURE cannot tell \
                 whether it understands it.",
            ),
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "this event is in format {found}, and this copy of SURE reads format \
                 {supported}. SURE will not guess at an event it does not understand, \
                 because a misread event becomes wrong evidence."
            ),
            Self::NotConformant { violations } => {
                write!(
                    f,
                    "this event does not match the SURE event format in {} way(s):",
                    violations.len()
                )?;
                for violation in violations {
                    write!(f, "\n  - {violation}")?;
                }
                Ok(())
            }
            Self::Malformed { message } => write!(
                f,
                "this event matches the SURE event format but could not be read: {message}"
            ),
        }
    }
}

impl std::error::Error for EnvelopeError {}

impl EventEnvelope {
    /// An envelope for this build's format version, with only the required
    /// fields set.
    ///
    /// The version is filled in rather than passed, so no call site can write an
    /// event claiming a format it was not built for.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        event_type: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: PROTOCOL_VERSION,
            source: source.into(),
            event_type: event_type.into(),
            timestamp: timestamp.into(),
            capability_tier: None,
            session_id: None,
            project_root: None,
            payload: empty_payload(),
        }
    }

    /// The same, reporting how much of the session could be seen.
    #[must_use]
    pub fn with_capability_tier(mut self, tier: CapabilityTier) -> Self {
        self.capability_tier = Some(tier);
        self
    }

    /// The same, naming the harness's session.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// The same, naming the project the event happened in.
    #[must_use]
    pub fn with_project_root(mut self, project_root: impl Into<String>) -> Self {
        self.project_root = Some(project_root.into());
        self
    }

    /// The same, carrying an event body.
    #[must_use]
    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    /// Read an envelope from JSON, refusing anything this build cannot trust.
    ///
    /// Four steps, in this order, because the order is what makes the error
    /// message the right one:
    ///
    /// 1. **Is it JSON?** A syntax error is the sender's bug.
    /// 2. **Which format is it?** Checked before the shape, so a document from a
    ///    newer SURE is reported as a version mismatch. Reporting it as a shape
    ///    error would send someone looking for a bug in an adapter that is
    ///    simply newer than this build.
    /// 3. **Does it match the event schema?** Every violation at once, in SURE's
    ///    own words.
    /// 4. **Does it deserialize?** A failure here means the schema and this type
    ///    disagree, which is SURE's mistake and is reported as such.
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError`] for any of the four.
    pub fn from_json(text: &str) -> Result<Self, EnvelopeError> {
        let probe: VersionProbe =
            serde_json::from_str(text).map_err(|error| EnvelopeError::NotJson {
                message: error.to_string(),
            })?;
        let Some(found) = probe.schema_version else {
            return Err(EnvelopeError::MissingVersion);
        };
        // The rule is `handshake::negotiate`'s, not a comparison written here.
        // The CLI answers a caller that asks *before* it sends anything, and it
        // asks the same function, so the two cannot tell a caller "yes" and then
        // refuse the document that follows.
        if !negotiate(found).is_agreed() {
            return Err(EnvelopeError::UnsupportedVersion {
                found,
                supported: PROTOCOL_VERSION,
            });
        }

        let value: Value = serde_json::from_str(text).map_err(|error| EnvelopeError::NotJson {
            message: error.to_string(),
        })?;
        let schema = DocumentKind::Event
            .schema()
            .map_err(|error| EnvelopeError::Malformed {
                message: error.to_string(),
            })?;
        let violations = schema.validate(&value);
        if !violations.is_empty() {
            return Err(EnvelopeError::NotConformant { violations });
        }

        serde_json::from_value(value).map_err(|error| EnvelopeError::Malformed {
            message: error.to_string(),
        })
    }

    /// The envelope as JSON.
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError`] if the envelope cannot be written, which cannot
    /// happen for a well-formed one and is not swallowed.
    pub fn to_json(&self) -> Result<String, EnvelopeError> {
        serde_json::to_string(self).map_err(|error| EnvelopeError::Malformed {
            message: error.to_string(),
        })
    }
}

/// Reads `schema_version` and nothing else.
///
/// Deliberately permissive about every other field: the version has to be
/// checked before the shape, or a document from a future format is reported as
/// malformed rather than as too new.
#[derive(Deserialize)]
struct VersionProbe {
    #[serde(default)]
    schema_version: Option<u32>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    fn envelope() -> EventEnvelope {
        EventEnvelope::new("claude-code", "tool.completed", "2026-09-14T09:10:56.827Z")
    }

    #[test]
    fn the_required_four_fields_are_the_ones_the_schema_requires() {
        let kind = DocumentKind::Event;
        assert_eq!(
            kind.required_keys(),
            vec![
                "schema_version".to_owned(),
                "source".to_owned(),
                "event_type".to_owned(),
                "timestamp".to_owned()
            ]
        );
    }

    #[test]
    fn a_minimal_envelope_carries_the_current_version_and_omits_the_rest() {
        let json: Value = serde_json::from_str(&envelope().to_json().unwrap()).unwrap();
        assert_eq!(json["schema_version"], json!(PROTOCOL_VERSION));
        assert_eq!(json["source"], json!("claude-code"));
        // Omitted, not null: a reader that saw `"capability_tier": null` could
        // read it as tier 0 rather than as "the adapter did not say".
        assert!(json.get("capability_tier").is_none(), "{json}");
        assert!(json.get("session_id").is_none(), "{json}");
        assert!(json.get("project_root").is_none(), "{json}");
    }

    #[test]
    fn the_capability_tier_is_a_number_on_the_wire() {
        // `docs/architecture/EVENT_PROTOCOL.md` shows `"capability_tier": 1`,
        // and `CapabilityTier`'s own serde form is the string `"observed"`,
        // which is for reports. The envelope must not use the report form.
        let json: Value = serde_json::from_str(
            &envelope()
                .with_capability_tier(CapabilityTier::Observed)
                .to_json()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["capability_tier"], json!(1));
    }

    #[test]
    fn every_tier_round_trips_through_the_numeric_wire_form() {
        for &tier in CapabilityTier::ALL {
            let text = envelope().with_capability_tier(tier).to_json().unwrap();
            let back = EventEnvelope::from_json(&text).expect("the envelope is readable");
            assert_eq!(back.capability_tier, Some(tier), "{tier:?}");
        }
    }

    #[test]
    fn an_unknown_tier_number_is_refused_rather_than_clamped() {
        // Clamping to the nearest known tier would be a capability claim SURE
        // was never given. The alternative, silently treating it as absent,
        // would understate what the harness can do — also a lie, in the other
        // direction.
        let text = json!({
            "schema_version": PROTOCOL_VERSION,
            "source": "x",
            "event_type": "y",
            "timestamp": "2026-09-14T09:10:56Z",
            "capability_tier": 7
        })
        .to_string();
        let error = EventEnvelope::from_json(&text).unwrap_err();
        match error {
            EnvelopeError::NotConformant { violations } => {
                assert_eq!(violations.len(), 1, "{violations:?}");
                assert_eq!(violations[0].path, "capability_tier");
            }
            other => panic!("expected a conformance failure, got {other:?}"),
        }
    }

    #[test]
    fn a_newer_format_is_refused_as_a_version_mismatch_not_as_a_shape_error() {
        // The whole point of checking the version first: the person reading this
        // needs a newer SURE, not a bug hunt.
        let text = json!({
            "schema_version": PROTOCOL_VERSION + 1,
            "source": "x",
            "event_type": "y",
            "timestamp": "2026-09-14T09:10:56Z",
            "a_field_from_the_future": true
        })
        .to_string();
        assert_eq!(
            EventEnvelope::from_json(&text).unwrap_err(),
            EnvelopeError::UnsupportedVersion {
                found: PROTOCOL_VERSION + 1,
                supported: PROTOCOL_VERSION
            }
        );
    }

    #[test]
    fn a_document_with_no_version_is_refused_rather_than_assumed_current() {
        // Assuming the current version would mean a document from an older
        // format was read as if it were this one, which is how an event gets
        // misread into wrong evidence.
        let text = json!({"source": "x", "event_type": "y"}).to_string();
        assert_eq!(
            EventEnvelope::from_json(&text).unwrap_err(),
            EnvelopeError::MissingVersion
        );
    }

    #[test]
    fn the_reader_and_the_handshake_refuse_the_same_versions() {
        // Two places decide whether SURE and a caller can talk: this reader,
        // when a document has already arrived, and `handshake::negotiate`, when
        // the CLI is asked before anything is sent. If they could disagree, an
        // adapter that the CLI told "yes" would then be refused at the document,
        // and the person would go looking for a bug in the adapter.
        //
        // Every version compared, including ones no SURE ever spoke, so that the
        // rule is checked rather than sampled.
        for version in 0..=PROTOCOL_VERSION + 3 {
            let text = json!({
                "schema_version": version,
                "source": "x",
                "event_type": "y",
                "timestamp": "2026-09-14T09:10:56Z"
            })
            .to_string();

            let refused = matches!(
                EventEnvelope::from_json(&text),
                Err(EnvelopeError::UnsupportedVersion { .. })
            );
            assert_eq!(
                refused,
                !negotiate(version).is_agreed(),
                "the reader and the handshake disagree about protocol {version}"
            );
        }
    }

    #[test]
    fn every_missing_required_field_is_reported_at_once() {
        let text = json!({"schema_version": PROTOCOL_VERSION}).to_string();
        match EventEnvelope::from_json(&text).unwrap_err() {
            EnvelopeError::NotConformant { violations } => {
                let missing: Vec<_> = violations
                    .iter()
                    .filter_map(|v| match &v.kind {
                        crate::schema::ViolationKind::Missing { key } => Some(key.as_str()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(missing, vec!["source", "event_type", "timestamp"]);
            }
            other => panic!("expected a conformance failure, got {other:?}"),
        }
    }

    #[test]
    fn a_timestamp_that_is_not_a_timestamp_is_refused() {
        let text = json!({
            "schema_version": PROTOCOL_VERSION,
            "source": "x",
            "event_type": "y",
            "timestamp": "2026-09-14"
        })
        .to_string();
        match EventEnvelope::from_json(&text).unwrap_err() {
            EnvelopeError::NotConformant { violations } => {
                assert_eq!(violations[0].path, "timestamp", "{violations:?}");
            }
            other => panic!("expected a conformance failure, got {other:?}"),
        }
    }

    #[test]
    fn a_field_the_format_does_not_define_is_refused_rather_than_ignored() {
        // An adapter sending something SURE does not understand must be a
        // visible error. Ignoring it would leave SURE reporting a capability it
        // silently dropped on the floor.
        let text = json!({
            "schema_version": PROTOCOL_VERSION,
            "source": "x",
            "event_type": "y",
            "timestamp": "2026-09-14T09:10:56Z",
            "tool_name": "Bash"
        })
        .to_string();
        match EventEnvelope::from_json(&text).unwrap_err() {
            EnvelopeError::NotConformant { violations } => {
                assert_eq!(violations.len(), 1, "{violations:?}");
                assert!(
                    violations[0].to_string().contains("tool_name"),
                    "{violations:?}"
                );
            }
            other => panic!("expected a conformance failure, got {other:?}"),
        }
    }

    #[test]
    fn text_that_is_not_json_is_reported_as_such() {
        let error = EventEnvelope::from_json("{not json").unwrap_err();
        assert!(matches!(error, EnvelopeError::NotJson { .. }), "{error:?}");
    }

    #[test]
    fn a_full_envelope_round_trips_unchanged() {
        let original = envelope()
            .with_capability_tier(CapabilityTier::Protected)
            .with_session_id("harness-session-7")
            .with_project_root("C:\\work\\my project")
            .with_payload(json!({"tool": "Bash", "exit_code": 0}));
        let text = original.to_json().unwrap();
        let back = EventEnvelope::from_json(&text).expect("the envelope is readable");
        assert_eq!(back, original);
    }

    #[test]
    fn the_session_id_is_the_harness_string_and_is_not_interpreted() {
        // Not a `SessionId`: SURE cannot validate a harness's identifier and has
        // no right to. An arbitrary string survives untouched.
        let text = envelope()
            .with_session_id("not-a-sure-id/with:punctuation")
            .to_json()
            .unwrap();
        assert_eq!(
            EventEnvelope::from_json(&text)
                .unwrap()
                .session_id
                .as_deref(),
            Some("not-a-sure-id/with:punctuation")
        );
    }

    #[test]
    fn an_error_message_says_what_to_do_about_it() {
        let unsupported = EnvelopeError::UnsupportedVersion {
            found: 9,
            supported: PROTOCOL_VERSION,
        };
        let message = unsupported.to_string();
        assert!(message.contains("format 9"), "{message}");
        assert!(
            message.contains(&format!("format {PROTOCOL_VERSION}")),
            "{message}"
        );
    }
}

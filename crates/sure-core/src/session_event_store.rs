//! Persist harness events into the local SQLite store with session binding,
//! retention metadata, and idempotent ingestion.
//!
//! # Idempotency
//!
//! Events are identified by their [`EventId`]. If the same event id is ingested
//! twice, the second ingestion returns [`PersistResult::AlreadyExists`] and the
//! store is not modified. This is enforced by a `UNIQUE` constraint on
//! `session_events.event_id`.
//!
//! Sessions are identified by the combination of the harness's session id,
//! project root, and harness source. When the harness does not provide a
//! session id, a new session is created for every event, because SURE has no
//! basis on which to correlate them.

use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use serde_json::Value;
use sure_domain::capability::CapabilityTier;
use sure_domain::ids::{EventId, FingerprintId, SessionId};
use sure_protocol::documents::DocumentKind;
use sure_protocol::event::EventEnvelope;

use crate::harness_event::IngestedEvent;
use crate::redact;
use crate::store::{RecordKind, Store, StoreError};

/// Default retention for raw events: 30 days.
pub const DEFAULT_EVENT_RETENTION_DAYS: i64 = 30;

/// Default retention for session metadata: 90 days.
pub const DEFAULT_SESSION_RETENTION_DAYS: i64 = 90;

/// How many days events at a given capability tier are kept.
///
/// Higher tiers get longer retention because they represent more complete
/// evidence.
#[must_use]
pub fn event_retention_days(tier: Option<CapabilityTier>) -> i64 {
    match tier {
        Some(CapabilityTier::Protected) => 90,
        Some(CapabilityTier::Observed) => 60,
        Some(CapabilityTier::Snapshot) | None => DEFAULT_EVENT_RETENTION_DAYS,
    }
}

/// How many days session metadata is kept.
#[must_use]
pub fn session_retention_days(_tier: Option<CapabilityTier>) -> i64 {
    DEFAULT_SESSION_RETENTION_DAYS
}

/// Compute a retention deadline from a start time and retention days.
#[must_use]
pub fn retention_deadline_ms(start_ms: i64, retention_days: i64) -> i64 {
    start_ms.saturating_add(retention_days.saturating_mul(86_400_000))
}

/// The outcome of persisting one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistResult {
    /// The event was stored for the first time.
    Stored,
    /// The event was already present and was not modified.
    AlreadyExists,
}

/// Why a session or event could not be persisted.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionEventStoreError {
    /// The underlying store reported an error.
    Store {
        /// What the store said.
        message: String,
    },
    /// The event envelope did not contain enough information to bind a session.
    MissingBinding {
        /// What was missing.
        field: &'static str,
    },
}

impl std::fmt::Display for SessionEventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store { message } => {
                write!(
                    f,
                    "SURE could not store the session or event.\n\n{}\n\n\
                     Nothing was written. The event was not recorded.",
                    redact::escape_control_characters(message)
                )
            }
            Self::MissingBinding { field } => write!(
                f,
                "The event does not contain '{}', which SURE needs to bind it \
                 to a session. Nothing was written.",
                redact::escape_control_characters(field)
            ),
        }
    }
}

impl std::error::Error for SessionEventStoreError {}

impl From<StoreError> for SessionEventStoreError {
    fn from(error: StoreError) -> Self {
        Self::Store {
            message: error.to_string(),
        }
    }
}

/// A session row as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSession {
    /// Row id in the database.
    pub row_id: i64,
    /// The harness's session identifier, if any.
    pub harness_session_id: Option<String>,
    /// SURE's identifier for this session.
    pub sure_session_id: SessionId,
    /// Project root path.
    pub project_root: String,
    /// Project fingerprint.
    pub project_fingerprint: String,
    /// Harness source name.
    pub harness_source: String,
    /// Capability tier reported when the session started.
    pub capability_tier: Option<CapabilityTier>,
    /// Session start time, RFC 3339.
    pub started_at: String,
    /// Retention period in days.
    pub retention_days: i64,
    /// Retention deadline, milliseconds since epoch.
    pub retained_until_ms: i64,
}

/// A stored event with its binding information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEvent {
    /// Row id in the database.
    pub row_id: i64,
    /// SURE's identifier for this event.
    pub event_id: EventId,
    /// Row id of the parent session.
    pub session_row_id: i64,
    /// Event type.
    pub event_type: String,
    /// Event timestamp, RFC 3339.
    pub timestamp: String,
    /// Event timestamp in milliseconds.
    pub timestamp_ms: i64,
    /// Capability tier reported at ingestion.
    pub capability_tier: Option<CapabilityTier>,
    /// Event payload.
    pub payload: Value,
    /// Project root path.
    pub project_root: String,
    /// Project fingerprint.
    pub project_fingerprint: String,
    /// Retention period in days.
    pub retention_days: i64,
    /// Retention deadline, milliseconds since epoch.
    pub retained_until_ms: i64,
    /// Row id in the `records` table, if linked.
    pub record_row_id: Option<i64>,
}

/// Wraps a [`Store`] to provide session and event persistence.
#[derive(Debug)]
pub struct SessionEventStore<'a> {
    store: &'a Store,
}

impl<'a> SessionEventStore<'a> {
    /// Wrap an open store.
    #[must_use]
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Persist an ingested event, creating or reusing its session.
    ///
    /// # Idempotency
    ///
    /// If `event_id` has already been stored, returns [`PersistResult::AlreadyExists`]
    /// without modifying anything.
    ///
    /// # Errors
    ///
    /// Returns [`SessionEventStoreError`] if the event cannot be persisted.
    pub fn persist(
        &self,
        ingested: &IngestedEvent,
        project_root: &str,
        fingerprint: &FingerprintId,
        event_id: &EventId,
    ) -> Result<PersistResult, SessionEventStoreError> {
        // Fast idempotency check outside the transaction.
        if self.event_exists(event_id)? {
            return Ok(PersistResult::AlreadyExists);
        }

        let envelope = &ingested.envelope;
        let capability_tier = envelope.capability_tier;

        let connection = self.store.connection();
        connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|error| self.write_error(error))?;

        let outcome: Result<PersistResult, SessionEventStoreError> = (|| {
            // Re-check idempotency inside the transaction.
            if self.event_exists_in_connection(connection, event_id)? {
                return Ok(PersistResult::AlreadyExists);
            }

            // Find or create the session.
            let session = self.find_or_create_session_in_tx(
                connection,
                envelope,
                project_root,
                fingerprint,
                capability_tier,
            )?;

            // The record document is the envelope itself, which matches the
            // Event schema (schema_version, source, event_type, timestamp, ...).
            let document =
                serde_json::to_value(envelope).map_err(|error| SessionEventStoreError::Store {
                    message: format!("could not serialize event: {error}"),
                })?;

            // Insert into records.
            let record_id = self.insert_record_in_tx(
                connection,
                RecordKind::Document(DocumentKind::Event),
                &document,
                project_root,
                fingerprint,
            )?;

            // Insert into session_events.
            let now_ms = Self::now_ms();
            let event_retention = event_retention_days(capability_tier);
            let event_retained_until = retention_deadline_ms(now_ms, event_retention);

            connection
                .execute(
                    "INSERT INTO session_events \
                     (event_id, session_row_id, event_type, timestamp, timestamp_ms, \
                      capability_tier, payload, project_root, project_fingerprint, \
                      retention_days, retained_until_ms, record_row_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    rusqlite::params![
                        event_id.as_str(),
                        session.row_id,
                        envelope.event_type,
                        envelope.timestamp,
                        now_ms,
                        capability_tier.map(|t| i64::from(t.number())),
                        envelope.payload.to_string(),
                        project_root,
                        fingerprint.as_str(),
                        event_retention,
                        event_retained_until,
                        record_id,
                    ],
                )
                .map_err(|error| self.write_error(error))?;

            Ok(PersistResult::Stored)
        })();

        match outcome {
            Ok(result) => {
                connection
                    .execute_batch("COMMIT")
                    .map_err(|error| self.write_error(error))?;
                Ok(result)
            }
            Err(error) => {
                let _ = connection.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    /// Read every event for a given session, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`SessionEventStoreError`] if the query fails.
    pub fn events_for_session(
        &self,
        session_row_id: i64,
    ) -> Result<Vec<StoredEvent>, SessionEventStoreError> {
        let connection = self.store.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, event_id, session_row_id, event_type, timestamp, \
                 timestamp_ms, capability_tier, payload, project_root, \
                 project_fingerprint, retention_days, retained_until_ms, \
                 record_row_id \
                 FROM session_events \
                 WHERE session_row_id = ?1 \
                 ORDER BY timestamp_ms DESC, id DESC",
            )
            .map_err(|error| self.write_error(error))?;
        let rows = statement
            .query_map([session_row_id], |row| {
                Ok(StoredEvent {
                    row_id: row.get(0)?,
                    event_id: parse_event_id(row.get::<_, String>(1)?)?,
                    session_row_id: row.get(2)?,
                    event_type: row.get(3)?,
                    timestamp: row.get(4)?,
                    timestamp_ms: row.get(5)?,
                    capability_tier: row
                        .get::<_, Option<i64>>(6)?
                        .and_then(|n| CapabilityTier::from_number(n as u8)),
                    payload: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or(Value::Null),
                    project_root: row.get(8)?,
                    project_fingerprint: row.get(9)?,
                    retention_days: row.get(10)?,
                    retained_until_ms: row.get(11)?,
                    record_row_id: row.get(12)?,
                })
            })
            .map_err(|error| self.write_error(error))?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row.map_err(|error| self.write_error(error))?);
        }
        Ok(events)
    }

    /// List sessions whose retention has expired.
    ///
    /// # Errors
    ///
    /// Returns [`SessionEventStoreError`] if the query fails.
    pub fn sessions_past_retention(
        &self,
        before_ms: i64,
    ) -> Result<Vec<StoredSession>, SessionEventStoreError> {
        let connection = self.store.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, harness_session_id, sure_session_id, project_root, \
                 project_fingerprint, harness_source, capability_tier, started_at, \
                 retention_days, retained_until_ms \
                 FROM sessions \
                 WHERE retained_until_ms < ?1 \
                 ORDER BY retained_until_ms ASC",
            )
            .map_err(|error| self.write_error(error))?;
        let rows = statement
            .query_map([before_ms], |row| {
                Ok(StoredSession {
                    row_id: row.get(0)?,
                    harness_session_id: row.get(1)?,
                    sure_session_id: parse_session_id(row.get::<_, String>(2)?)?,
                    project_root: row.get(3)?,
                    project_fingerprint: row.get(4)?,
                    harness_source: row.get(5)?,
                    capability_tier: row
                        .get::<_, Option<i64>>(6)?
                        .and_then(|n| CapabilityTier::from_number(n as u8)),
                    started_at: row.get(7)?,
                    retention_days: row.get(8)?,
                    retained_until_ms: row.get(9)?,
                })
            })
            .map_err(|error| self.write_error(error))?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row.map_err(|error| self.write_error(error))?);
        }
        Ok(sessions)
    }

    /// List events whose retention has expired.
    ///
    /// # Errors
    ///
    /// Returns [`SessionEventStoreError`] if the query fails.
    pub fn events_past_retention(
        &self,
        before_ms: i64,
    ) -> Result<Vec<StoredEvent>, SessionEventStoreError> {
        let connection = self.store.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, event_id, session_row_id, event_type, timestamp, \
                 timestamp_ms, capability_tier, payload, project_root, \
                 project_fingerprint, retention_days, retained_until_ms, \
                 record_row_id \
                 FROM session_events \
                 WHERE retained_until_ms < ?1 \
                 ORDER BY retained_until_ms ASC",
            )
            .map_err(|error| self.write_error(error))?;
        let rows = statement
            .query_map([before_ms], |row| {
                Ok(StoredEvent {
                    row_id: row.get(0)?,
                    event_id: parse_event_id(row.get::<_, String>(1)?)?,
                    session_row_id: row.get(2)?,
                    event_type: row.get(3)?,
                    timestamp: row.get(4)?,
                    timestamp_ms: row.get(5)?,
                    capability_tier: row
                        .get::<_, Option<i64>>(6)?
                        .and_then(|n| CapabilityTier::from_number(n as u8)),
                    payload: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or(Value::Null),
                    project_root: row.get(8)?,
                    project_fingerprint: row.get(9)?,
                    retention_days: row.get(10)?,
                    retained_until_ms: row.get(11)?,
                    record_row_id: row.get(12)?,
                })
            })
            .map_err(|error| self.write_error(error))?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row.map_err(|error| self.write_error(error))?);
        }
        Ok(events)
    }

    fn find_or_create_session_in_tx(
        &self,
        connection: &Connection,
        envelope: &EventEnvelope,
        project_root: &str,
        fingerprint: &FingerprintId,
        capability_tier: Option<CapabilityTier>,
    ) -> Result<StoredSession, SessionEventStoreError> {
        // Try to find an existing session by harness session id + project + source.
        if let Some(ref harness_session_id) = envelope.session_id {
            let existing = connection.query_row(
                "SELECT id, harness_session_id, sure_session_id, project_root, \
                 project_fingerprint, harness_source, capability_tier, started_at, \
                 retention_days, retained_until_ms \
                 FROM sessions \
                 WHERE harness_session_id = ?1 AND project_root = ?2 AND harness_source = ?3",
                rusqlite::params![harness_session_id, project_root, envelope.source],
                |row| {
                    Ok(StoredSession {
                        row_id: row.get(0)?,
                        harness_session_id: row.get(1)?,
                        sure_session_id: parse_session_id(row.get::<_, String>(2)?)?,
                        project_root: row.get(3)?,
                        project_fingerprint: row.get(4)?,
                        harness_source: row.get(5)?,
                        capability_tier: row
                            .get::<_, Option<i64>>(6)?
                            .and_then(|n| CapabilityTier::from_number(n as u8)),
                        started_at: row.get(7)?,
                        retention_days: row.get(8)?,
                        retained_until_ms: row.get(9)?,
                    })
                },
            );
            if let Ok(session) = existing {
                return Ok(session);
            }
        }

        // Create a new session.
        let sure_session_id = SessionId::generate();
        let started_at = envelope.timestamp.clone();
        let now_ms = Self::now_ms();
        let session_retention = session_retention_days(capability_tier);
        let retained_until = retention_deadline_ms(now_ms, session_retention);

        connection
            .execute(
                "INSERT INTO sessions \
                 (harness_session_id, sure_session_id, project_root, project_fingerprint, \
                  harness_source, capability_tier, started_at, retention_days, retained_until_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    envelope.session_id.as_ref(),
                    sure_session_id.as_str(),
                    project_root,
                    fingerprint.as_str(),
                    envelope.source,
                    capability_tier.map(|t| i64::from(t.number())),
                    started_at,
                    session_retention,
                    retained_until,
                ],
            )
            .map_err(|error| self.write_error(error))?;

        let row_id = connection.last_insert_rowid();
        Ok(StoredSession {
            row_id,
            harness_session_id: envelope.session_id.clone(),
            sure_session_id,
            project_root: project_root.to_owned(),
            project_fingerprint: fingerprint.as_str().to_owned(),
            harness_source: envelope.source.clone(),
            capability_tier,
            started_at,
            retention_days: session_retention,
            retained_until_ms: retained_until,
        })
    }

    fn event_exists(&self, event_id: &EventId) -> Result<bool, SessionEventStoreError> {
        self.event_exists_in_connection(self.store.connection(), event_id)
    }

    fn event_exists_in_connection(
        &self,
        connection: &Connection,
        event_id: &EventId,
    ) -> Result<bool, SessionEventStoreError> {
        let count: i64 = connection
            .query_row(
                "SELECT count(*) FROM session_events WHERE event_id = ?1",
                [event_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|error| self.write_error(error))?;
        Ok(count > 0)
    }

    fn insert_record_in_tx(
        &self,
        connection: &Connection,
        kind: RecordKind,
        document: &Value,
        project_root: &str,
        fingerprint: &FingerprintId,
    ) -> Result<i64, SessionEventStoreError> {
        let text = self.store.validate_and_stringify(kind, document)?;
        let written_at_ms = Self::now_ms();
        let version = i64::from(sure_protocol::DOCUMENT_VERSION);

        connection
            .execute(
                "INSERT INTO records (kind, document_version, written_at_ms, \
                 project_root, project_fingerprint, document) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    kind.as_str(),
                    version,
                    written_at_ms,
                    project_root,
                    fingerprint.as_str(),
                    text,
                ],
            )
            .map_err(|error| self.write_error(error))?;
        Ok(connection.last_insert_rowid())
    }

    fn write_error(&self, error: rusqlite::Error) -> SessionEventStoreError {
        if crate::store::is_busy(&error) {
            SessionEventStoreError::Store {
                message: String::from(
                    "another SURE process held the write lock and SURE gave up waiting.",
                ),
            }
        } else {
            SessionEventStoreError::Store {
                message: error.to_string(),
            }
        }
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64)
    }
}

fn parse_session_id(value: String) -> Result<SessionId, rusqlite::Error> {
    SessionId::parse(value).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn parse_event_id(value: String) -> Result<EventId, rusqlite::Error> {
    EventId::parse(value).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;
    use sure_domain::capability::CapabilityTier;
    use sure_protocol::PROTOCOL_VERSION;
    use sure_protocol::event::EventEnvelope;

    use crate::store::{HistoryFilter, Store};

    fn scratch(name: &str) -> std::path::PathBuf {
        crate::store::scratch_root().join(format!("{name}-{}", std::process::id()))
    }

    fn store_in(name: &str) -> Store {
        let dir = scratch(name);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("cannot clear {}: {e}", dir.display()),
        }
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Store::open_at(&dir.join("sure.db")).expect("the store opens")
    }

    fn valid_envelope(event_type: &str) -> EventEnvelope {
        EventEnvelope::new("claude-code", event_type, "2026-09-14T09:10:56.827Z")
            .with_capability_tier(CapabilityTier::Observed)
            .with_session_id("harness-session-42")
            .with_project_root("C:\\work\\my project")
            .with_payload(json!({"tool": "Bash", "exit_code": 0}))
    }

    #[test]
    fn a_session_is_created_on_first_event() {
        let store = store_in("session_created");
        let ses = SessionEventStore::new(&store);
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();

        let result = ses
            .persist(&ingested, "C:\\work\\my project", &fingerprint, &event_id)
            .expect("persist succeeds");
        assert_eq!(result, PersistResult::Stored);

        let sessions = ses
            .sessions_past_retention(i64::MAX)
            .expect("query succeeds");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].harness_source, "claude-code");
        assert_eq!(
            sessions[0].harness_session_id.as_deref(),
            Some("harness-session-42")
        );
        assert_eq!(sessions[0].project_root, "C:\\work\\my project");
        assert_eq!(sessions[0].project_fingerprint, fingerprint.as_str());
        assert_eq!(sessions[0].capability_tier, Some(CapabilityTier::Observed));
        assert_eq!(sessions[0].retention_days, DEFAULT_SESSION_RETENTION_DAYS);
        assert!(sessions[0].retained_until_ms > 0);
    }

    #[test]
    fn subsequent_events_for_the_same_session_reuse_the_session_row() {
        let store = store_in("session_reuse");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };

        let event1 = EventId::generate();
        let event2 = EventId::generate();

        ses.persist(&ingested, "C:\\work\\my project", &fingerprint, &event1)
            .unwrap();
        ses.persist(&ingested, "C:\\work\\my project", &fingerprint, &event2)
            .unwrap();

        let sessions = ses.sessions_past_retention(i64::MAX).unwrap();
        assert_eq!(sessions.len(), 1, "only one session should exist");

        let events = ses.events_for_session(sessions[0].row_id).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn events_are_bound_to_project_session_time_and_capability() {
        let store = store_in("event_binding");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        let event_id = EventId::generate();

        ses.persist(&ingested, "C:\\work\\my project", &fingerprint, &event_id)
            .unwrap();

        let sessions = ses.sessions_past_retention(i64::MAX).unwrap();
        let events = ses.events_for_session(sessions[0].row_id).unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.event_id, event_id);
        assert_eq!(event.event_type, "tool.completed");
        assert_eq!(event.capability_tier, Some(CapabilityTier::Observed));
        assert_eq!(event.project_root, "C:\\work\\my project");
        assert_eq!(event.project_fingerprint, fingerprint.as_str());
        assert_eq!(event.payload, json!({"tool": "Bash", "exit_code": 0}));
    }

    #[test]
    fn retention_metadata_is_recorded() {
        let store = store_in("retention");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        let event_id = EventId::generate();

        ses.persist(&ingested, "C:\\work\\my project", &fingerprint, &event_id)
            .unwrap();

        let sessions = ses.sessions_past_retention(i64::MAX).unwrap();
        assert_eq!(sessions[0].retention_days, DEFAULT_SESSION_RETENTION_DAYS);
        assert!(sessions[0].retained_until_ms > 0);

        let events = ses.events_past_retention(i64::MAX).unwrap();
        assert_eq!(
            events[0].retention_days,
            event_retention_days(Some(CapabilityTier::Observed))
        );
        assert!(events[0].retained_until_ms > 0);
    }

    #[test]
    fn duplicate_event_ingestion_is_handled_deterministically() {
        let store = store_in("duplicate");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        let event_id = EventId::generate();

        let first = ses
            .persist(&ingested, "C:\\work\\my project", &fingerprint, &event_id)
            .unwrap();
        let second = ses
            .persist(&ingested, "C:\\work\\my project", &fingerprint, &event_id)
            .unwrap();

        assert_eq!(first, PersistResult::Stored);
        assert_eq!(second, PersistResult::AlreadyExists);

        let sessions = ses.sessions_past_retention(i64::MAX).unwrap();
        let events = ses.events_for_session(sessions[0].row_id).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn malformed_events_are_refused_before_persistence() {
        // This is handled by the ingestion layer (harness_event.rs), not the
        // store layer. The store layer only sees validated IngestedEvent values.
        // We verify here that the ingestion layer rejects bad input.
        use crate::harness_event::ingest_event_str;

        let bad = json!({
            "schema_version": PROTOCOL_VERSION,
            "source": "x",
            // missing event_type and timestamp
        })
        .to_string();
        let err = ingest_event_str(&bad).unwrap_err();
        assert!(err.to_string().contains("does not match"), "{err}");
    }

    #[test]
    fn events_can_be_read_back_via_store_history() {
        let store = store_in("history_roundtrip");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        let event_id = EventId::generate();

        ses.persist(&ingested, "C:\\work\\my project", &fingerprint, &event_id)
            .unwrap();

        let records = store
            .history(&HistoryFilter::for_project(&fingerprint), 10)
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, RecordKind::Document(DocumentKind::Event));
        assert_eq!(
            records[0].project_fingerprint.as_deref(),
            Some(fingerprint.as_str())
        );
        assert_eq!(
            records[0].project_root.as_deref(),
            Some("C:\\work\\my project")
        );
    }

    #[test]
    fn retention_query_lists_only_past_retention_items() {
        let store = store_in("retention_query");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        let event_id = EventId::generate();

        ses.persist(&ingested, "C:\\work\\my project", &fingerprint, &event_id)
            .unwrap();

        // All items were created at now > 0, so querying for items past retention
        // with a cutoff at time 0 should return nothing.
        assert!(ses.sessions_past_retention(0).unwrap().is_empty());
        assert!(ses.events_past_retention(0).unwrap().is_empty());

        // Querying with a cutoff in the distant future should return everything
        // because every finite retention deadline is before i64::MAX.
        assert_eq!(ses.sessions_past_retention(i64::MAX).unwrap().len(), 1);
        assert_eq!(ses.events_past_retention(i64::MAX).unwrap().len(), 1);
    }

    #[test]
    fn error_messages_are_plain_language_and_escape_attacker_input() {
        let err = SessionEventStoreError::MissingBinding {
            field: "session_id\nfoo",
        };
        let text = err.to_string();
        assert!(
            text.contains("session_id\\nfoo"),
            "attacker-controlled newline was not escaped: {text}"
        );
        assert!(
            !text.contains('\n'),
            "message contains a real newline: {text}"
        );
    }

    #[test]
    fn retention_deadline_does_not_overflow() {
        let max = i64::MAX;
        let result = retention_deadline_ms(max, 1);
        assert_eq!(result, i64::MAX);
    }
}

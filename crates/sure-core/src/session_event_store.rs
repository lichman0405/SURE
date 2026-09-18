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
use rusqlite::types::Value as SqlValue;
use serde_json::Value;
use sure_domain::capability::CapabilityTier;
use sure_domain::ids::{EventId, FingerprintId, SessionId};
use sure_protocol::documents::DocumentKind;
use sure_protocol::event::EventEnvelope;

use crate::harness_event::IngestedEvent;
use crate::protection_history::DecisionRecord;
use crate::redact;
use crate::store::{RecordKind, Store, StoreError, StoredRecord};

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

/// Which sessions a read or a delete is about.
///
/// One filter for both, so that a scope cannot mean one thing when a user reads
/// their history and another when they delete it — the mistake that lets
/// `sure history delete` remove rows `sure history` never showed. See
/// [`SessionScope::sql`].
///
/// Every variant is a *named* scope. There is deliberately no `Default` and no
/// "whatever matches": the caller that reaches the delete path has already had
/// to say which of these it meant, and `docs/architecture/CLI.md` records why
/// that argument is required rather than prompted for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionScope<'a> {
    /// Every session in the store.
    All,
    /// One session, by SURE's own identifier for it.
    One(&'a str),
    /// Every session recorded against one project root, **exactly as recorded**.
    ///
    /// Exact rather than normalised, and the narrow direction is deliberate: a
    /// match that folded case or separators would make a path the user typed
    /// delete rows a different path named. `sure history` prints the root
    /// verbatim, which is what a caller copies.
    Project(&'a str),
}

impl SessionScope<'_> {
    /// The `WHERE` clause and its parameters.
    ///
    /// Built here rather than written out at each call site, and returned as a
    /// value the caller binds rather than as a finished statement: the values
    /// are bound, never interpolated, because one of them is a path the user
    /// typed and the other is a string out of the store.
    fn sql(&self) -> (String, Vec<SqlValue>) {
        match self {
            Self::All => (String::new(), Vec::new()),
            Self::One(sure_session_id) => (
                String::from(" WHERE sure_session_id = ?"),
                vec![SqlValue::Text((*sure_session_id).to_owned())],
            ),
            Self::Project(project_root) => (
                String::from(" WHERE project_root = ?"),
                vec![SqlValue::Text((*project_root).to_owned())],
            ),
        }
    }
}

/// What one delete removed, counted by table.
///
/// Five numbers rather than one, because they are five different claims: the
/// sessions are what the user asked to be rid of, the events are what was
/// recorded inside them, and the records are the envelopes those events wrote.
/// A single total would let a delete that removed nothing but rows already
/// orphaned read as the delete the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeletedSessions {
    /// Rows removed from `sessions`.
    pub sessions: u64,
    /// Rows removed from `session_events`.
    pub events: u64,
    /// Rows removed from `records`, which those events owned.
    pub records: u64,
    /// Rows removed from `records` of kind `decision`.
    ///
    /// Counted apart from [`DeletedSessions::records`], and the counts are
    /// disjoint by construction: this one holds the rows a
    /// [`crate::protection_history::DecisionRecord`] was written as, which the
    /// user asked the history for, and `records` holds the event envelopes the
    /// events owned. A user asking *is the verdict SURE reached about my project
    /// still here* is asking about these, and one total would let a delete that
    /// removed the events and left every decision behind read as complete.
    pub decisions: u64,
    /// Rows removed from `records` of kind `recording`.
    ///
    /// Counted apart from [`DeletedSessions::records`] because they are a
    /// different claim: those rows hold the raw content a full recording opted
    /// in to keeping, and a user asking *is it gone* is asking about these. A
    /// delete that removed the session row and left the transcript behind would
    /// be a deletion that reads as complete in a single total.
    pub recordings: u64,
}

impl DeletedSessions {
    /// Whether nothing at all was removed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sessions == 0
            && self.events == 0
            && self.records == 0
            && self.decisions == 0
            && self.recordings == 0
    }
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

        self.in_transaction(|connection| {
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

            // Insert into session_events first. `INSERT OR IGNORE` is the last
            // line of defense against a duplicate event id: if another hook
            // process committed the same event id while this one was waiting for
            // the write lock, the insert is ignored and the store is left
            // unchanged.
            let now_ms = Self::now_ms();
            let event_retention = event_retention_days(capability_tier);
            let event_retained_until = retention_deadline_ms(now_ms, event_retention);

            let inserted = connection
                .execute(
                    "INSERT OR IGNORE INTO session_events \
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
                        crate::store::redact_document(&envelope.payload).to_string(),
                        project_root,
                        fingerprint.as_str(),
                        event_retention,
                        event_retained_until,
                        None::<i64>,
                    ],
                )
                .map_err(|error| self.write_error(error))?;

            if inserted == 0 {
                return Ok(PersistResult::AlreadyExists);
            }

            let event_row_id = connection.last_insert_rowid();

            // The record document is the envelope itself, which matches the
            // Event schema (schema_version, source, event_type, timestamp, ...).
            let document =
                serde_json::to_value(envelope).map_err(|error| SessionEventStoreError::Store {
                    message: format!("could not serialize event: {error}"),
                })?;

            // Insert into records and link the event row to it.
            let record_id = self.insert_record_in_tx(
                connection,
                RecordKind::Document(DocumentKind::Event),
                &document,
                project_root,
                fingerprint,
            )?;

            connection
                .execute(
                    "UPDATE session_events SET record_row_id = ?1 WHERE id = ?2",
                    rusqlite::params![record_id, event_row_id],
                )
                .map_err(|error| self.write_error(error))?;

            Ok(PersistResult::Stored)
        })
    }

    /// Record the protection decision SURE reached about one event.
    ///
    /// Returns the row id of the decision that was written, or `None` when the
    /// event it belongs to is not in the store — which is an answer, not a
    /// failure: see below.
    ///
    /// # Why the event is looked for first, inside the transaction
    ///
    /// A decision is not a session record the way an event is: nothing in the
    /// schema points at it, and the event id inside its own document is the only
    /// thing that ties it to the session it came from (the same join
    /// [`crate::protection_history`] and [`Self::delete_sessions`] describe). A
    /// row written for an event that is not there would therefore be a row
    /// **outside `sure history`, outside `sure history delete` and outside
    /// session retention** — one the user can neither see nor remove, which is
    /// the shape of record `docs/security/PRIVACY.md` does not allow SURE to
    /// keep. So it is not written, and the caller is told by the return value.
    ///
    /// The look and the insert are one `BEGIN IMMEDIATE` transaction, so a
    /// `sure history delete` running at the same moment either commits before
    /// this read — and this returns `None` — or waits for this commit and then
    /// removes both rows. There is no interleaving in which the decision row
    /// outlives its event with nothing to delete it by.
    ///
    /// # Errors
    ///
    /// [`SessionEventStoreError`] if the document cannot be built or the write
    /// fails. A caller that cannot write this row is expected to say so rather
    /// than to swallow it: [`crate::protection_history::not_recorded_reason`] is
    /// the sentence for that, and the decision itself is not changed by it.
    pub fn persist_decision(
        &self,
        record: &DecisionRecord,
        project_root: &str,
        fingerprint: &FingerprintId,
    ) -> Result<Option<i64>, SessionEventStoreError> {
        self.in_transaction(|connection| {
            if !self.event_exists_in_connection(connection, &record.event_id)? {
                return Ok(None);
            }

            let document =
                serde_json::to_value(record).map_err(|error| SessionEventStoreError::Store {
                    message: format!("could not serialize the decision: {error}"),
                })?;

            let id = self.insert_record_in_tx(
                connection,
                RecordKind::Decision,
                &document,
                project_root,
                fingerprint,
            )?;
            Ok(Some(id))
        })
    }

    /// Run one write as a single transaction.
    ///
    /// `BEGIN IMMEDIATE` rather than `BEGIN`, for `sure_core::store`'s reason:
    /// the write lock is taken now, so contention is met where the busy timeout
    /// applies and where it can be reported as contention, instead of at the
    /// first write, where a partial transaction would have to be unwound.
    ///
    /// One helper rather than the block written out at each of the two writes
    /// this module makes. The two have to agree, and a transaction boundary is
    /// not a thing to keep in step by hand — the second copy is where a
    /// `ROLLBACK` goes missing from the path that needed it.
    fn in_transaction<T>(
        &self,
        body: impl FnOnce(&Connection) -> Result<T, SessionEventStoreError>,
    ) -> Result<T, SessionEventStoreError> {
        let connection = self.store.connection();
        connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|error| self.write_error(error))?;

        match body(connection) {
            Ok(value) => {
                connection
                    .execute_batch("COMMIT")
                    .map_err(|error| self.write_error(error))?;
                Ok(value)
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

    /// Read the protection decisions recorded for a session's events.
    ///
    /// # Why this is a query of its own
    ///
    /// A decision is a `records` row of kind `decision` and **nothing links it
    /// to the `session_events` row it is about**: no column, no foreign key —
    /// `PRAGMA foreign_keys` is not set anywhere in this store, so a `REFERENCES`
    /// clause would be documentation rather than a link. The event id is copied
    /// into the decision's document (`"event_id"`), and that is the join. It is
    /// read with `json_extract` for the same reason [`Self::delete_sessions`]
    /// reads a full recording that way, and it is the *same* join, so a decision
    /// cannot be shown under one event and deleted under another.
    ///
    /// In the order SURE wrote them — the row id — like every other listing in
    /// this module. The ids are read in one query and the rows through
    /// [`Store::record`], so a row this build cannot describe stops the read
    /// rather than being skipped.
    ///
    /// [`Store::record`]: crate::store::Store::record
    ///
    /// # Errors
    ///
    /// [`SessionEventStoreError`] if the query fails or a row cannot be read.
    pub fn decisions_for_session(
        &self,
        session_row_id: i64,
    ) -> Result<Vec<StoredRecord>, SessionEventStoreError> {
        let connection = self.store.connection();
        let ids = {
            let mut statement = connection
                .prepare(
                    "SELECT id FROM records WHERE kind = ?1 \
                     AND json_extract(document, '$.event_id') IN \
                     (SELECT event_id FROM session_events WHERE session_row_id = ?2) \
                     ORDER BY id ASC",
                )
                .map_err(|error| self.write_error(error))?;
            let rows = statement
                .query_map(
                    rusqlite::params![RecordKind::Decision.as_str(), session_row_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| self.write_error(error))?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(|error| self.write_error(error))?);
            }
            ids
        };

        let mut records = Vec::new();
        for id in ids {
            if let Some(record) = self.store.record(id)? {
                records.push(record);
            }
        }
        Ok(records)
    }

    /// Read the recorded sessions, newest first.
    ///
    /// Newest first by the order SURE wrote them — the row id — rather than by
    /// the `started_at` string, which is whatever the harness said and is not
    /// SURE's to compare: two harnesses that spell the same instant differently
    /// would order differently here, and a listing that reordered itself between
    /// two runs over an unchanged store would be a listing a user could not
    /// check against anything.
    ///
    /// # Errors
    ///
    /// Returns [`SessionEventStoreError`] if the query fails.
    pub fn sessions(&self, limit: usize) -> Result<Vec<StoredSession>, SessionEventStoreError> {
        let connection = self.store.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, harness_session_id, sure_session_id, project_root, \
                 project_fingerprint, harness_source, capability_tier, started_at, \
                 retention_days, retained_until_ms \
                 FROM sessions \
                 ORDER BY id DESC \
                 LIMIT ?1",
            )
            .map_err(|error| self.write_error(error))?;
        let rows = statement
            .query_map([limit as i64], read_session_row)
            .map_err(|error| self.write_error(error))?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row.map_err(|error| self.write_error(error))?);
        }
        Ok(sessions)
    }

    /// How many sessions the store holds, whatever the caller's page size.
    ///
    /// Read separately from [`SessionEventStore::sessions`] rather than returned
    /// beside it, so that a listing which showed fewer rows than there are can
    /// say so — "12 of 340" is a different sentence from "12", and a report that
    /// could only say the second would be telling the user their history is
    /// smaller than it is.
    ///
    /// # Errors
    ///
    /// Returns [`SessionEventStoreError`] if the query fails.
    pub fn session_count(&self) -> Result<u64, SessionEventStoreError> {
        let count: i64 = self
            .store
            .connection()
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .map_err(|error| self.write_error(error))?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// One session, by SURE's own identifier for it.
    ///
    /// `None` when there is no such session. SURE's identifier rather than the
    /// harness's, because the harness's is only unique per project and per
    /// source: deleting by one would be deleting by something that names
    /// several rows, and a delete is the wrong place to discover that.
    ///
    /// # Errors
    ///
    /// Returns [`SessionEventStoreError`] if the query fails.
    pub fn session_by_sure_id(
        &self,
        sure_session_id: &str,
    ) -> Result<Option<StoredSession>, SessionEventStoreError> {
        let connection = self.store.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, harness_session_id, sure_session_id, project_root, \
                 project_fingerprint, harness_source, capability_tier, started_at, \
                 retention_days, retained_until_ms \
                 FROM sessions \
                 WHERE sure_session_id = ?1",
            )
            .map_err(|error| self.write_error(error))?;

        let mut rows = statement
            .query_map([sure_session_id], read_session_row)
            .map_err(|error| self.write_error(error))?;
        match rows.next() {
            None => Ok(None),
            Some(row) => Ok(Some(row.map_err(|error| self.write_error(error))?)),
        }
    }

    /// Delete the sessions a scope names, their events, the records those events
    /// own, and the full recordings of those events.
    ///
    /// # What goes, and in what order
    ///
    /// The order is the whole of the safety argument. A `session_events` row
    /// carries `session_row_id REFERENCES sessions(id)` and
    /// `record_row_id REFERENCES records(id)`, so the events are the child of
    /// both and go first; then the `records` rows those events own, so that
    /// nothing points at a row that is about to disappear; then the `sessions`
    /// rows themselves.
    ///
    /// **`PRAGMA foreign_keys` is not set anywhere in this store, and the
    /// default is off.** So `REFERENCES` here is documentation rather than
    /// enforcement, there is no cascade, and a delete that removed `sessions`
    /// first would leave the events behind as rows nothing points at and nothing
    /// will ever clean up. Deleting children first is what makes this correct
    /// whether or not the pragma is ever turned on, which is why the order is
    /// stated rather than inherited from a constraint that is not in force.
    ///
    /// # Why the full recordings and the decisions are in here
    ///
    /// A full recording is a second `records` row — kind `recording`, written by
    /// [`crate::full_recording::persist_full_recording`] — and **nothing links
    /// it to the `session_events` row it came from**: no column, no foreign key.
    /// The two are written from one event with one [`EventId`] in
    /// `crates/sure-cli/src/hook.rs`, and the event id is copied into the
    /// recording's document (`"event_id"`), so that is the join. It is read with
    /// `json_extract` rather than by rewriting the document to carry a row id,
    /// because the recording's own document is a promise about what was kept and
    /// a schema change to `records` is not what this is.
    ///
    /// Leaving them behind would be the worst shape of answer this module could
    /// give: the session rows gone, the raw transcript still on the disk, and
    /// the report saying the delete succeeded. `docs/security/PRIVACY.md` allows
    /// raw content only on an explicit opt-in, and the delete the user asked for
    /// has to reach it.
    ///
    /// A decision SURE reached about one of those events is the same shape of
    /// row — a `records` row of kind `decision`, hanging from the event id in its
    /// own document ([`crate::protection_history`]) — and it goes by the same
    /// join for the same reason. A verdict is not raw content, but it is
    /// something this machine recorded about the user's work, and the promise is
    /// about what SURE keeps and not only about the parts of it the user would
    /// most want gone. It is one clause rather than an omission, and
    /// `a_delete_reaches_the_decision_recorded_about_its_session_event` is what
    /// holds it.
    ///
    /// # What does not go
    ///
    /// Records that no event of a deleted session points at — a recorded goal,
    /// an approval, a standard projection, a recording whose event was written
    /// by some other path — are not session records and are left alone; they
    /// have their own retention and their own delete path.
    ///
    /// The counts are returned rather than assumed. A caller that reports them
    /// tells the user how many rows went; a caller that reported nothing on a
    /// delete whose scope matched nothing would be reporting a deletion that did
    /// not happen.
    ///
    /// # Errors
    ///
    /// [`SessionEventStoreError::Store`] if the delete fails. Everything is one
    /// transaction, so a failure removes nothing at all — there is no state in
    /// which half a session is gone.
    pub fn delete_sessions(
        &self,
        scope: SessionScope<'_>,
    ) -> Result<DeletedSessions, SessionEventStoreError> {
        let (where_clause, parameters) = scope.sql();

        self.in_transaction(|connection| {
            let selected = format!("SELECT id FROM sessions{where_clause}");
            let mut statement = connection
                .prepare(&selected)
                .map_err(|error| self.write_error(error))?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(parameters.clone()), |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|error| self.write_error(error))?;

            let mut session_ids = Vec::new();
            for row in rows {
                session_ids.push(row.map_err(|error| self.write_error(error))?);
            }
            // The statement borrows the connection, and every statement below
            // needs it too.
            drop(statement);

            let Some(in_sessions) = id_list(&session_ids) else {
                return Ok(DeletedSessions::default());
            };

            // What the events being deleted own and what they were written as.
            // Read before the events go, because after that there is nothing
            // left to read them from.
            let (record_ids, event_ids) = {
                let sql = format!(
                    "SELECT record_row_id, event_id FROM session_events \
                     WHERE session_row_id IN ({in_sessions})"
                );
                let mut statement = connection
                    .prepare(&sql)
                    .map_err(|error| self.write_error(error))?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|error| self.write_error(error))?;
                let mut record_ids = Vec::new();
                let mut event_ids = Vec::new();
                for row in rows {
                    let (record_id, event_id) = row.map_err(|error| self.write_error(error))?;
                    record_ids.extend(record_id);
                    event_ids.push(event_id);
                }
                (record_ids, event_ids)
            };

            let events = connection
                .execute(
                    &format!("DELETE FROM session_events WHERE session_row_id IN ({in_sessions})"),
                    [],
                )
                .map_err(|error| self.write_error(error))?;

            let records = match id_list(&record_ids) {
                None => 0,
                Some(in_records) => connection
                    .execute(
                        &format!("DELETE FROM records WHERE id IN ({in_records})"),
                        [],
                    )
                    .map_err(|error| self.write_error(error))?,
            };

            // The full recordings and the protection decisions those same events
            // wrote. `event_id` is the only thing either row shares with its
            // event — see the note above `delete_sessions` — so it is what the
            // join is made of. Both keep the ids bound rather than interpolated:
            // they are text out of the store, and a text value is the one kind
            // that can end a SQL literal.
            let decisions =
                self.delete_records_joined_to_events(connection, RecordKind::Decision, &event_ids)?;
            let recordings = self.delete_records_joined_to_events(
                connection,
                RecordKind::Recording,
                &event_ids,
            )?;

            let sessions = connection
                .execute(
                    &format!("DELETE FROM sessions WHERE id IN ({in_sessions})"),
                    [],
                )
                .map_err(|error| self.write_error(error))?;

            Ok(DeletedSessions {
                sessions: u64::try_from(sessions).unwrap_or(0),
                events: u64::try_from(events).unwrap_or(0),
                records: u64::try_from(records).unwrap_or(0),
                decisions: u64::try_from(decisions).unwrap_or(0),
                recordings: u64::try_from(recordings).unwrap_or(0),
            })
        })
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
            .query_map([before_ms], read_session_row)
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
        if let Some(ref harness_session_id) = envelope.session_id
            && let Ok(session) = self.find_session_by_harness(
                connection,
                harness_session_id,
                project_root,
                &envelope.source,
            )
        {
            return Ok(session);
        }

        // Create a new session. `INSERT OR IGNORE` plus a re-query is what makes
        // concurrent hook writes deterministic: if another short-lived process
        // created the same session while this one was reading, the insert is
        // ignored and the existing row is returned instead of failing.
        let sure_session_id = SessionId::generate();
        let started_at = envelope.timestamp.clone();
        let now_ms = Self::now_ms();
        let session_retention = session_retention_days(capability_tier);
        let retained_until = retention_deadline_ms(now_ms, session_retention);

        let inserted = connection
            .execute(
                "INSERT OR IGNORE INTO sessions \
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

        if inserted > 0 {
            let row_id = connection.last_insert_rowid();
            return Ok(StoredSession {
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
            });
        }

        // Another process won the race. Re-query and return the row it wrote.
        if let Some(ref harness_session_id) = envelope.session_id
            && let Ok(session) = self.find_session_by_harness(
                connection,
                harness_session_id,
                project_root,
                &envelope.source,
            )
        {
            return Ok(session);
        }

        Err(SessionEventStoreError::Store {
            message: String::from(
                "SURE could not find or create the session this event belongs to.",
            ),
        })
    }

    fn find_session_by_harness(
        &self,
        connection: &Connection,
        harness_session_id: &str,
        project_root: &str,
        harness_source: &str,
    ) -> Result<StoredSession, rusqlite::Error> {
        connection.query_row(
            "SELECT id, harness_session_id, sure_session_id, project_root, \
             project_fingerprint, harness_source, capability_tier, started_at, \
             retention_days, retained_until_ms \
             FROM sessions \
             WHERE harness_session_id = ?1 AND project_root = ?2 AND harness_source = ?3",
            rusqlite::params![harness_session_id, project_root, harness_source],
            read_session_row,
        )
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

    /// Delete the `records` rows of one kind that a set of events wrote.
    ///
    /// The join is the event id inside the row's own document, which is the only
    /// thing a full recording and a protection decision share with the
    /// `session_events` row they came from. One helper rather than the same
    /// `DELETE` written out twice: the two kinds must be reached by the *same*
    /// join, or a delete would be complete for one of them and partial for the
    /// other, and that difference would be invisible in a count.
    ///
    /// The ids and the kind are bound, never interpolated — they are text out of
    /// the store, and a text value is the one kind that can end a SQL literal.
    /// The placeholder list is the one thing built from a length, and it is a
    /// list of `?` and nothing else.
    ///
    /// # Errors
    ///
    /// [`SessionEventStoreError::Store`] if the delete fails.
    fn delete_records_joined_to_events(
        &self,
        connection: &Connection,
        kind: RecordKind,
        event_ids: &[String],
    ) -> Result<usize, SessionEventStoreError> {
        if event_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; event_ids.len()].join(", ");
        let sql = format!(
            "DELETE FROM records WHERE kind = ? \
             AND json_extract(document, '$.event_id') IN ({placeholders})"
        );
        let mut values: Vec<SqlValue> = vec![SqlValue::Text(kind.as_str().to_owned())];
        values.extend(event_ids.iter().cloned().map(SqlValue::Text));
        connection
            .execute(&sql, rusqlite::params_from_iter(values))
            .map_err(|error| self.write_error(error))
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

/// Read one `sessions` row, in the column order every query in this module
/// selects.
///
/// One reader for the three places a session row is read — by identifier, by
/// harness binding, and by age — so that a column added to the query is a column
/// added here rather than to two of the three.
fn read_session_row(row: &rusqlite::Row<'_>) -> Result<StoredSession, rusqlite::Error> {
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
}

/// A comma-separated list of row ids for an `IN (…)` clause, or `None` for an
/// empty one.
///
/// `None` rather than `String::new()`, because `IN ()` is a syntax error in
/// SQLite and a caller that spelled it would get a failed delete reported as a
/// parse failure rather than as "nothing matched". The ids are `i64`s this
/// process read out of the store, so nothing a project controls is interpolated
/// here.
fn id_list(ids: &[i64]) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    Some(
        ids.iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    )
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
    use std::sync::mpsc;
    use std::thread;

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
    fn payload_secrets_are_redacted_before_storage() {
        let store = store_in("redacted_payload");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let mut envelope = valid_envelope("tool.completed");
        envelope.payload = json!({
            "command": "curl https://user:secret@example.com",
            "env": {"API_KEY": "supersecret"},
            "token": "bearer abc123xyz"
        });
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
        let stored = &events[0];
        let payload_text = stored.payload.to_string();
        assert!(
            !payload_text.contains("user:secret@example.com"),
            "stored payload still contained URL credentials: {payload_text}"
        );
        assert!(
            !payload_text.contains("abc123xyz"),
            "stored payload still contained token value: {payload_text}"
        );
        assert!(
            payload_text.contains("REDACTED") || payload_text.contains("***"),
            "redaction marker missing from payload: {payload_text}"
        );
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
    fn events_without_session_id_get_distinct_sessions() {
        // A missing harness session id means SURE has no basis for correlation.
        // The unique index is on the harness id, so NULLs remain distinct and
        // every event gets its own session.
        let store = store_in("no_session_id");
        let ses = SessionEventStore::new(&store);
        let fingerprint = FingerprintId::generate();
        let mut envelope = valid_envelope("tool.completed");
        envelope.session_id = None;
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
        assert_eq!(
            sessions.len(),
            2,
            "events with no harness session id should not share a session"
        );
    }

    #[test]
    fn concurrent_duplicate_event_id_is_idempotent() {
        // Two hook processes ingesting the same event id at the same time must
        // not create two authoritative event rows. One wins, the other gets
        // AlreadyExists, and the database ends with exactly one event.
        let dir = scratch("concurrent_duplicate_event_id");
        let database = dir.join("sure.db");

        let fingerprint = FingerprintId::generate();
        let envelope = valid_envelope("tool.completed");
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        let event_id = EventId::generate();

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();

        let ingested1 = ingested.clone();
        let path1 = database.clone();
        let fp1 = fingerprint.clone();
        let id1 = event_id.clone();
        let handle1 = thread::spawn(move || {
            let store = Store::open_at(&path1).expect("the store opens");
            let result = SessionEventStore::new(&store)
                .persist(&ingested1, "C:\\work\\my project", &fp1, &id1)
                .expect("persist does not error");
            tx1.send(result).expect("result sent");
        });

        let ingested2 = ingested.clone();
        let path2 = database.clone();
        let fp2 = fingerprint.clone();
        let id2 = event_id.clone();
        let handle2 = thread::spawn(move || {
            let store = Store::open_at(&path2).expect("the store opens");
            let result = SessionEventStore::new(&store)
                .persist(&ingested2, "C:\\work\\my project", &fp2, &id2)
                .expect("persist does not error");
            tx2.send(result).expect("result sent");
        });

        handle1.join().expect("thread one finishes");
        handle2.join().expect("thread two finishes");

        let results = vec![rx1.recv().unwrap(), rx2.recv().unwrap()];
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == PersistResult::Stored)
                .count(),
            1,
            "exactly one concurrent ingest should store the event: {results:?}"
        );
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == PersistResult::AlreadyExists)
                .count(),
            1,
            "exactly one concurrent ingest should find the event already exists: {results:?}"
        );

        let store = Store::open_at(&database).expect("the store reopens");
        let ses = SessionEventStore::new(&store);
        let sessions = ses.sessions_past_retention(i64::MAX).unwrap();
        assert_eq!(sessions.len(), 1);
        let events = ses.events_for_session(sessions[0].row_id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, event_id);
    }

    #[test]
    fn concurrent_session_creation_reuses_one_session_row() {
        // Two events for the same harness session arriving at once must not
        // create two session rows. The unique index and INSERT OR IGNORE path
        // make the second writer find the row the first wrote.
        let dir = scratch("concurrent_session_creation");
        let database = dir.join("sure.db");

        let fingerprint = FingerprintId::generate();
        let base_envelope = valid_envelope("tool.completed");
        let event1_id = EventId::generate();
        let event2_id = EventId::generate();

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();

        let path1 = database.clone();
        let fp1 = fingerprint.clone();
        let env1 = base_envelope.clone();
        let handle1 = thread::spawn(move || {
            let store = Store::open_at(&path1).expect("the store opens");
            let ingested = IngestedEvent {
                envelope: env1,
                protocol_version: PROTOCOL_VERSION,
                document_kind: DocumentKind::Event,
            };
            let result = SessionEventStore::new(&store)
                .persist(&ingested, "C:\\work\\my project", &fp1, &event1_id)
                .expect("persist does not error");
            tx1.send(result).expect("result sent");
        });

        let path2 = database.clone();
        let fp2 = fingerprint.clone();
        let env2 = base_envelope.clone();
        let handle2 = thread::spawn(move || {
            let store = Store::open_at(&path2).expect("the store opens");
            let ingested = IngestedEvent {
                envelope: env2,
                protocol_version: PROTOCOL_VERSION,
                document_kind: DocumentKind::Event,
            };
            let result = SessionEventStore::new(&store)
                .persist(&ingested, "C:\\work\\my project", &fp2, &event2_id)
                .expect("persist does not error");
            tx2.send(result).expect("result sent");
        });

        handle1.join().expect("thread one finishes");
        handle2.join().expect("thread two finishes");

        assert_eq!(rx1.recv().unwrap(), PersistResult::Stored);
        assert_eq!(rx2.recv().unwrap(), PersistResult::Stored);

        let store = Store::open_at(&database).expect("the store reopens");
        let ses = SessionEventStore::new(&store);
        let sessions = ses.sessions_past_retention(i64::MAX).unwrap();
        assert_eq!(
            sessions.len(),
            1,
            "two events for the same harness session should create one session row"
        );
        let events = ses.events_for_session(sessions[0].row_id).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn retention_deadline_does_not_overflow() {
        let max = i64::MAX;
        let result = retention_deadline_ms(max, 1);
        assert_eq!(result, i64::MAX);
    }

    /// One event of the shape `sure hook ingest` writes for a tool request,
    /// which is the only thing a decision is ever written about.
    fn ingest_one_event(store: &Store, event_id: &EventId, fingerprint: &FingerprintId) {
        let envelope = valid_envelope("tool.requested")
            .with_payload(json!({"tool": "Bash", "args": {"command": "rm -rf build/"}}));
        let ingested = IngestedEvent {
            envelope,
            protocol_version: PROTOCOL_VERSION,
            document_kind: DocumentKind::Event,
        };
        SessionEventStore::new(store)
            .persist(&ingested, "C:\\work\\my project", fingerprint, event_id)
            .expect("the event is written");
    }

    fn a_decision(event_id: &EventId, allowance: Option<i64>) -> DecisionRecord {
        let decision = crate::hook_protection::ProtectionDecision::block(
            crate::hook_protection::danger_reason(crate::hook_protection::Danger::BroadDelete),
        );
        DecisionRecord::of(
            event_id.clone(),
            &decision,
            Some(crate::hook_protection::Danger::BroadDelete),
            "Bash",
            allowance,
        )
    }

    fn decision_rows(store: &Store) -> Vec<StoredRecord> {
        store
            .history(
                &HistoryFilter {
                    project_fingerprint: None,
                    kind: Some(RecordKind::Decision),
                    include_recordings: false,
                },
                crate::allowance::SCAN_LIMIT,
            )
            .expect("the rows read back")
    }

    #[test]
    fn a_decision_written_for_an_event_is_read_back_with_its_session() {
        let store = store_in("decision_read_back");
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();
        ingest_one_event(&store, &event_id, &fingerprint);

        let ses = SessionEventStore::new(&store);
        let session = ses.sessions(1).expect("the session")[0].clone();
        let written = ses
            .persist_decision(
                &a_decision(&event_id, Some(11)),
                "C:\\work\\my project",
                &fingerprint,
            )
            .expect("the decision is written");
        assert!(written.is_some(), "the event was there to hang from");

        let read = ses
            .decisions_for_session(session.row_id)
            .expect("the decisions of a session this test wrote");
        assert_eq!(read.len(), 1, "{read:?}");
        assert_eq!(
            crate::protection_history::read_back(read[0].clone()).expect("readable"),
            a_decision(&event_id, Some(11))
        );
    }

    /// The row a decision would be is a row nothing could show or delete, so it
    /// is not written — and the caller is told, because "the store refused" and
    /// "there was no event" are different sentences for the user.
    #[test]
    fn a_decision_for_an_event_that_is_not_there_is_not_written() {
        let store = store_in("decision_without_event");
        let ses = SessionEventStore::new(&store);
        let absent = EventId::generate();

        let written = ses
            .persist_decision(
                &a_decision(&absent, None),
                "C:\\work\\my project",
                &FingerprintId::generate(),
            )
            .expect("nothing failed: there was simply nothing to hang from");
        assert_eq!(written, None);
        assert!(
            decision_rows(&store).is_empty(),
            "a decision row with no event points at nothing and is outside every path that \
             could show or delete it"
        );
    }

    #[test]
    fn a_delete_reaches_the_decision_recorded_about_its_session_event() {
        let store = store_in("decision_deleted");
        let fingerprint = FingerprintId::generate();
        let event_id = EventId::generate();
        ingest_one_event(&store, &event_id, &fingerprint);

        let ses = SessionEventStore::new(&store);
        let row = ses
            .persist_decision(
                &a_decision(&event_id, None),
                "C:\\work\\my project",
                &fingerprint,
            )
            .expect("the decision is written")
            .expect("the event was there to hang from");

        let deleted = ses
            .delete_sessions(SessionScope::All)
            .expect("the delete runs");
        assert_eq!(deleted.decisions, 1, "{deleted:?}");
        assert_eq!(deleted.records, 1, "the event's own row, counted apart");
        assert!(!deleted.is_empty());
        assert!(
            store.record(row).expect("a lookup").is_none(),
            "the decision survived a delete that reported removing it"
        );
    }
}

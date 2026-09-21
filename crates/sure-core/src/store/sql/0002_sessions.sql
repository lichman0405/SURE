-- Session and event persistence for harness events.
--
-- `sessions` holds one row per harness session SURE has observed.
-- `session_events` holds one row per ingested event, bound to its session.
--
-- Both tables carry retention metadata so that pruning can be done later
-- without re-computing policy.

CREATE TABLE sessions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    harness_session_id  TEXT,                       -- the harness's own identifier
    sure_session_id     TEXT    NOT NULL UNIQUE,    -- SURE's SessionId
    project_root        TEXT    NOT NULL,
    project_fingerprint TEXT    NOT NULL,
    harness_source      TEXT    NOT NULL,
    capability_tier     INTEGER,
    started_at          TEXT    NOT NULL,
    retention_days      INTEGER NOT NULL,
    retained_until_ms   INTEGER NOT NULL
) STRICT;

CREATE INDEX sessions_lookup
    ON sessions (harness_session_id, project_root, harness_source);

CREATE INDEX sessions_retention
    ON sessions (retained_until_ms);

CREATE TABLE session_events (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id            TEXT    NOT NULL UNIQUE,    -- SURE's EventId
    session_row_id      INTEGER NOT NULL REFERENCES sessions(id),
    event_type          TEXT    NOT NULL,
    timestamp           TEXT    NOT NULL,
    timestamp_ms        INTEGER NOT NULL,
    capability_tier     INTEGER,
    payload             TEXT    NOT NULL,
    project_root        TEXT    NOT NULL,
    project_fingerprint TEXT    NOT NULL,
    retention_days      INTEGER NOT NULL,
    retained_until_ms   INTEGER NOT NULL,
    record_row_id       INTEGER REFERENCES records(id)
) STRICT;

CREATE INDEX session_events_session
    ON session_events (session_row_id, timestamp_ms DESC);

CREATE INDEX session_events_retention
    ON session_events (retained_until_ms);

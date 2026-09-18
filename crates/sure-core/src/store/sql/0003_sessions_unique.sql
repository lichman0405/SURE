-- Make session binding deterministic under concurrent hook writes.
--
-- A harness may fire several hook processes at once for the same session.
-- Without this index two events for the same harness session could create two
-- session rows. The index is unique over the non-null harness session id, the
-- project root and the harness source; events that do not carry a harness
-- session id remain free to create a fresh session each, because SURE has no
-- basis on which to correlate them.
CREATE UNIQUE INDEX IF NOT EXISTS sessions_unique
    ON sessions (harness_session_id, project_root, harness_source);

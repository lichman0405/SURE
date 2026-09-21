-- The first schema: one table, one row per stored document.
--
-- `STRICT` is not decoration. Without it SQLite applies type affinity, so a
-- `TEXT` column accepts an integer and hands it back as text, and a mistake in
-- the write path becomes a value that reads as plausible rather than as an
-- error. With it, the column types are enforced and a wrong-typed write fails
-- where it happened.
CREATE TABLE records (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    kind                TEXT    NOT NULL,
    document_version    INTEGER NOT NULL,
    written_at_ms       INTEGER NOT NULL,
    project_root        TEXT,
    project_fingerprint TEXT,
    document            TEXT    NOT NULL,
    CHECK (kind <> ''),
    CHECK (document_version >= 1)
) STRICT;

-- `AUTOINCREMENT` above, rather than a plain `INTEGER PRIMARY KEY`, because
-- rows can be deleted: without it SQLite reuses the rowid of a deleted row, so
-- a record id quoted in a report could later name a different record. The cost
-- is one extra table and one extra write per insert, which is worth paying to
-- keep an identifier meaning one thing for the life of the file.

-- `written_at_ms DESC, id DESC` is the read order, so it is the index order.
-- The `id` is not a tiebreak for tidiness: short-lived hook processes write
-- several rows inside the same millisecond, and without it their order is
-- whatever the query planner returns, which is not stable between runs.
CREATE INDEX records_recent ON records (written_at_ms DESC, id DESC);

CREATE INDEX records_kind_recent ON records (kind, written_at_ms DESC);

CREATE INDEX records_project_recent ON records (project_fingerprint, written_at_ms DESC);

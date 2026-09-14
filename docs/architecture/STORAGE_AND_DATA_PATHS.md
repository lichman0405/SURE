# Storage and data paths

## Project-local

`.sure/` may hold ephemeral project/run caches and is gitignored by default.

A user may commit `sure.yaml` intentionally.

## User-level durable data

Use OS-appropriate application-data locations through a well-tested path abstraction rather than hard-coded `~/.sure` everywhere.

Windows must use the chosen Rust path library's per-user application-data/config conventions (for example LocalAppData/RoamingAppData as appropriate), not hard-coded home-directory paths. macOS/Linux use their native equivalents through the same abstraction.

### Resolved locations

The abstraction is `sure_core::paths`. It asks the platform, through the `dirs`
crate, rather than reading environment variables directly: the correct Windows
API is `SHGetKnownFolderPath`, this workspace forbids `unsafe`, and a hand-rolled
read of `%LOCALAPPDATA%` silently produces a *relative* path when the variable is
unset — which resolves against the current directory, which is the project being
checked.

| What | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Data (`Paths::data_dir`) | `%LOCALAPPDATA%\SURE` | `~/Library/Application Support/SURE` | `~/.local/share/SURE` |
| Config (`Paths::config_dir`) | `%APPDATA%\SURE` | `~/Library/Application Support/SURE` | `~/.config/SURE` |
| Project cache | `<project>\.sure` | `<project>/.sure` | `<project>/.sure` |

Data is the machine-local location and config is the roaming one, deliberately:
history is not something a roaming profile should carry between machines, while
settings are exactly what roaming is for. On macOS the two coincide, which is
correct there and is not a reason to collapse them on Windows.

There is no separate user-level *cache* root. On Windows `data_local_dir` is
already `%LOCALAPPDATA%`, so a third location would name the same place twice.
What is regenerable is the project cache, and that is `project_cache_dir`, a
separate function from `Paths` so a caller cannot reach for the trusted location
by accident.

Expose:

```text
sure doctor
sure history
sure history delete ...
sure config paths
```

so users can see/delete what is stored.

## Storage rules

- schema migrations are versioned;
- do not hold synchronous DB handles across async `.await`;
- event writes are transactional/robust against short-lived concurrent hook processes;
- full recordings are clearly distinguishable and deletable;
- secrets are redacted before durable write where possible.

### The store

`sure_core::store` is the implementation of the rules above. One SQLite file,
`sure.db`, in the data directory from the table earlier in this document.

**The engine is compiled in.** `rusqlite` is used with its `bundled` feature, so
SQLite is built from source into `sure.exe` and an installation needs nothing
from the machine. Linking a system `sqlite3` was rejected: SURE's behaviour would
then depend on a library the user did not install for SURE and cannot tell us
about, and a lock or page-size difference there is not something SURE could
detect or report honestly. The choice is asserted in
`crates/sure-core/tests/store_packaging.rs`, because the build does not catch its
removal — a workspace without `bundled` still compiles, and fails only when
something that links is built.

**Migrations are the only way to a schema.** There is no schema-creation path
that is not a migration, so a fresh install and an upgrade run identical code.
The version lives in `PRAGMA user_version` rather than in a table, because it is
written in the same transaction as the DDL and therefore cannot disagree with the
schema it describes. A file whose version is newer than the build is refused
rather than opened, and a SQLite file that already has tables and reports version
0 is refused as foreign.

**The concurrency contract** is stated in full in `sure_core::store`'s module
documentation and is not repeated here; the part that belongs in this document is
the one a reader of the storage rules would ask about. Every write is a single
`BEGIN IMMEDIATE` … `COMMIT`, so several short-lived hook processes writing at
once serialise in SQLite's busy handler. The wait is bounded, and a write that
times out is reported as not saved rather than retried forever or dropped — an
unbounded retry turns contention into a hang, and a dropped event turns it into a
project with nothing recorded about it. `synchronous = FULL`, so a committed
write survives power loss; each hook process writes one row, which is one fsync
per event.

Migrations are re-checked inside the write transaction, so two processes opening
a fresh file at the same time do not both try to create the tables.

**Records carry a document version.** The six documents SURE writes carry no
version of their own — `PROTOCOL.md` says why — so the version is stamped on the
stored record instead. A row written by a newer SURE is refused rather than
decoded, because a field this build does not know would disappear on the way in
and the record would then read as one that never had it. This closes conformance
gap 3 in `FROZEN_SEMANTICS.md` and known gap 1 in `PROTOCOL.md`.

**Full recordings** are stored under a kind of their own, excluded from history
unless a caller asks for them by name, and deletable on their own
(`Store::delete_recordings`). They have no schema, because they are not a
statement about a project; they must still be a JSON object, so that a later
reader is not left guessing what shape they are.

**Redaction happens before the schema check, not after.** The order is what makes
the guarantee a guarantee: the document that was validated is the document that
was written. Redaction only rewrites the *contents* of strings, never their
types, so a document that matched its schema still does. The test for this reads
the file's bytes rather than the value in memory — a claim about what was written
down is a claim about bytes.


## Authoritative evidence boundary

The checked project is not a trusted place to store authoritative evidence. An agent working in the repository may edit `.sure/`.

Therefore durable evidence/history used for verdict/claim truth must live in user-level application data outside the working tree. Project `.sure/` may contain cache IDs, local non-authoritative artifacts or links, but modifying it must not let project code fabricate a passing authoritative history.

This is enforced, not merely documented. `Paths::ensure_outside(project_root)`
refuses a project root that contains the data or config directory, and
`Paths::from_roots` refuses a relative location so that a store can never be
resolved against the current directory. The case is not hypothetical: a user who
runs SURE on their home directory has a project root containing
`%LOCALAPPDATA%`, and the correct answer there is to stop rather than write
anyway.

The comparison is `sure_core::paths::compare`, which is component-wise rather
than textual (`C:\project-evil` is not inside `C:\project`), case-folding on the
platforms that case-fold (reporting "within" when a comparison is genuinely
ambiguous, because that is the direction that refuses to write), and treats a
canonicalised `\\?\C:\...` path as the same location as `C:\...`.

Cryptographic tamper-proof attestation is not a v0.1 claim.

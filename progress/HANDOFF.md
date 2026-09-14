# Autonomous handoff

Last updated: 2026-09-14
Branch: `claude/v0.1-autonomous`
Progress: 16 / 166 tasks accepted. Phase P0 complete (9/9). Phase P1 in progress
(7/13). `P1-T005` is accepted with the commit that carries this file.

Primary development host: Windows 11 x64 / native MSVC.

Canonical remote: `https://github.com/lichman0405/SURE.git`
Autonomous branch: `claude/v0.1-autonomous`

## Exact current state

`node scripts/taskctl.mjs status` reports:

```
Project: SURE | status: in_progress | phase: P1
{ accepted: 16, queued: 150, ready: 10 }
READY: P1-T008, P1-T010, P1-T011, P2-T001, P2-T010, P3-T001, P6-T007, P8-T001, P12-T008, P13-T001
```

The next one this session takes is **`P1-T008`** (CLI command framework), because
`P1-T009` (`sure doctor`) and `P1-T010` (protocol version handshake) both need a
command to hang off, and `P1-T005` has just given `sure doctor` something to
report: the journal mode, the schema version and the integrity check are all
public on `Store` already and have no command that reaches them.

`P1-T011` (configuration authority layers) is the alternative and is
independent.

## Gate set, as run at `P1-T005`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features` | 21 `test result: ok` lines, 408 passed, 0 failed |
| `node scripts/validate-bootstrap.mjs` | 17 phases, 166 tasks |

`sure-core` alone is 179 lib tests + 9 config_loading + 6 store_concurrency + 4
store_packaging. The concurrency file spawns real child processes, so it takes
about a second and its children appear in the output as `child_writer`.

## What `P1-T005` added

`crates/sure-core/src/store/` — one SQLite file, at `Paths::store_file()`:

- `mod.rs` — `Store`, `StoreOptions`, `HistoryFilter`, the five-point concurrency
  contract, redaction-then-validation on the write path, and the one bounded
  retry in the module (`establish_journal_mode`).
- `migrations.rs` — `PRAGMA user_version`, one transaction per migration, an
  append-only `MIGRATIONS` list, and refusals for a newer file, a foreign file
  and a version that is not a version.
- `sql/0001_records.sql` — one `STRICT` table with `AUTOINCREMENT` and three
  indexes. `include_str!`, so an installed `sure.exe` migrates against what its
  code was built from.
- `record.rs` — `RecordKind` (the six documents plus `Recording`) and
  `StoredRecord`, which carries `document_version` and refuses a row from a newer
  build. This closes `FROZEN_SEMANTICS.md` conformance gap 3 and `PROTOCOL.md`
  known gap 1.
- `error.rs` — `StoreError`, every message saying what SURE did instead.
- `tests/store_concurrency.rs` — real child processes, with a barrier.
- `tests/store_packaging.rs` — `bundled`, no async runtime, no unsafe.

`docs/architecture/STORAGE_AND_DATA_PATHS.md` gained a §The store.

## Two real bugs this task found, both by mutation-checking a green test

Recorded because both are the kind of thing that ships silently.

1. **`PRAGMA journal_mode = WAL` bypasses the busy handler.** SQLite's
   `sqlite3_busy_handler` documentation names the journal-mode change as a case
   where it declines to invoke the handler, because waiting could deadlock. Five
   of six processes opening a *fresh* database therefore died with
   `database is locked` before reaching a single write — reported as
   `StoreError::Open`, so the message said the file could not be opened rather
   than that anyone had contended. Fixed by `establish_journal_mode`, which waits
   for the same `busy_timeout` and reports `StoreError::Busy`.
2. **`Store::write_error` reported `DEFAULT_BUSY_TIMEOUT`, not the configured
   one.** A store opened with a 50 ms timeout told the user "SURE waited 5000 ms",
   which is a false statement about what just happened with no way for the reader
   to tell.

Neither was visible until `tests/store_concurrency.rs` was made to *fail*: the
first version of that test passed with the bug present, because spawning six
processes takes longer than migrating a database and the children never
collided. The barrier (`a_moment_from_now`, an 800 ms spin) is what made the
contention real. **A test that cannot fail is worse than no test, because it is
read as evidence.**

## Adversarial (mutation) verifications in this session

Each was reverted after confirming the check fires.

- Removing `features = ["bundled"]` from the workspace `rusqlite` entry fails
  `store_packaging::sqlite_is_compiled_into_sure…`. Worth knowing *why* the test
  exists rather than leaving it to the build: without `bundled`,
  `cargo build -p sure-core` still **succeeds** — it produces an rlib, and an
  rlib is never linked. The failure arrives later as
  `LNK1181: cannot open input file 'sqlite3.lib'`, and only on a machine that
  has no system SQLite. On this machine it does not, so this is also the direct
  evidence for the acceptance criterion.
- Adding `tokio` to `[workspace.dependencies]` fails
  `no_async_runtime_has_arrived`. Confirmed the test is not vacuously green: the
  same line-based reader finds `rusqlite` in the same section.
- Removing the in-transaction version re-read in `apply_one` fails
  `a_migration_another_process_already_ran_is_not_run_a_second_time` with
  `table records already exists`, and **does not** fail the cross-process test.
  With the journal-mode wait in place the children are serialised past that
  window. Both tests carry a comment saying so; the cross-process test says what
  it does not cover.
- Earlier in this branch: `P1-T007`'s `additionalProperties: false` bug (checked
  inside the `properties` lookup, so a closed object with no `properties` allowed
  every key) and the `issue` → `issue_id` repair-contract gap.

## Accepted work on this branch

- `4f2d75d` P0-T009 — foundational ADRs, plus `FROZEN_SEMANTICS.md`.
- `4ce2ce6` P1-T001 — declared crate boundaries, mechanically enforced by
  `sure_testkit::workspace` and `sure_testkit::integrations`.
- `6f63801` P1-T002 — `variants!` `ALL` lists and the pinned wire contract.
- `P1-T003` — `crates/sure-core/src/config/`: the `sure.yaml` model, loader,
  diagnostics and redaction, plus `docs/architecture/CONFIG_REFERENCE.md` and
  `docs/adr/0011-project-configuration-is-a-request.md`.
- `b6a14b4` P1-T004 — OS-native data paths and the outside-the-project rule.
- `ad05ee1` P1-T006 — diagnostics as records, and redaction.
- `P1-T007` — `crates/sure-protocol/`: the schema validator, the document
  registry, the event envelope, 27 conformance/round-trip tests and
  `docs/architecture/PROTOCOL.md`.
- `P1-T005` — `crates/sure-core/src/store/`, the concurrency and packaging tests,
  and §The store in `STORAGE_AND_DATA_PATHS.md`.

## Next concrete action

1. `node scripts/taskctl.mjs start P1-T008` (CLI command framework).
2. Then `P1-T009` (`sure doctor`) and `P1-T010` (protocol version handshake),
   which depend on it. `P1-T009` has three things ready to report and no command
   to report them from: `Store::journal_mode`, `Store::schema_version` and
   `Store::integrity_check`.

`taskctl accept` takes `--note`, not `--evidence`; `--evidence` is silently
ignored, which is how the earliest tasks came to record an empty note.

## Environment notes for the next session

- **Run `cargo fmt --all` before the gate set, not after.** New files written by
  hand are not rustfmt-shaped (let-else bodies, long `assert!` messages) and
  `--check` fails on them. `P1-T007`'s commit was blocked once by this.
- **Every integration test file needs
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the
  top.** The workspace lints are `warn` but the gate runs `-D warnings`, so a new
  test file fails clippy until it carries the opt-out. `cargo fmt --all` will
  place it correctly if the file starts with it.
- Write repository files with LF endings. `core.autocrlf=true` plus
  `.gitattributes` (`* text=auto eol=lf`) means a Python `write_text` on Windows
  leaves CRLF in the working tree that shows as a phantom ` M` until
  `git add -A` re-hashes it. Write with `newline='\n'` or `write_bytes`.
- **`.gitattributes` and `git ls-files --eol` are the authority on line
  endings.** `grep -c $'\r'` gives a false positive on some files and a false
  negative on others; `store_packaging.rs` was `w/crlf` with zero `\r` bytes.
  Use `git ls-files --eol`.
- **`rusqlite` is a `dev-dependency` of `sure-core` as well as a real one**,
  which looks like a mistake and is not: `tests/store_concurrency.rs` holds the
  write lock from outside the store, and the lock is SQLite's, so the test has to
  take it with SQLite. `[dev-dependencies]` do not appear in
  `normal_edges()`, so this does not affect the crate-boundary test.
- **A spawned test child must be given `--exact <name> --ignored`, not a bare
  filter.** Without `--exact` a filter that is a prefix of another test's name
  runs that one too, and the child recurses into the parent's tests.
- **A cross-process contention test needs a barrier or it tests nothing.** See
  the two-bugs section above. The pattern that worked is a wall-clock moment
  passed in an environment variable and a spin loop in the child; Windows sleeps
  in ~15 ms steps, which is too coarse.
- The child's environment variable holds a path, a count and a time,
  newline-separated. Any other separator can appear in a path.
- `toml` 1.x parses a *document* into `toml::Table`, not `toml::Value`;
  `Value`'s `FromStr` reads a single value and fails on the second key.
- `variants!` `ALL` is a `&'static [Self]` slice, so iterate with
  `for &x in T::ALL` and use `.iter().copied()` where a value is needed. The
  macro is `#[macro_export]`ed, so `sure-core` reaches it as
  `sure_domain::variants::variants`.
- `serde_yaml_ng` error text is load-bearing for `config`'s diagnostics: it
  prepends a `parent.path: ` prefix, quotes field names in backticks and values
  in double quotes, and reports a repeated key with the *parent* path. The three
  shapes are pinned by `serde_error_shapes_are_what_this_classifier_expects`.
- YAML 1.2 (what `serde_yaml_ng` implements) does **not** read `yes`, `no`, `on`
  or `off` as booleans. `checks.existing_tests: yes` is a string, and the
  classifier turns that into "use one of: true, false".
- Repository test fixtures that need a writable scratch directory belong under
  `target/tmp/` (already git-ignored, same volume as the checkout). See
  `crates/sure-core/tests/config_loading.rs`.
- Clippy's `derivable_impls` and `result_large_err` are enforced by
  `-D warnings`. `ConfigError` boxes its `ErrorKind` for the second reason; the
  non-derivable `Default` impls (`ExecutionConfig`, `ChecksConfig`) carry their
  reason in a comment.
- **An `f64` read out of JSON has no total order, so a type holding one must not
  derive `Eq`.** `ViolationKind::BelowMinimum` is why `Violation`,
  `ViolationKind` and `EnvelopeError` are `PartialEq` only; each carries a
  comment saying so. `assert_eq!` needs only `PartialEq`.
- **`DocumentKind::ALL` is a free const** (`sure_protocol::documents::ALL`), not
  an associated one, and `DocumentKind::FixtureExpectation` is deliberately not
  storable — `RecordKind::is_storable` is the predicate and `Store::append`
  refuses it with `StoreError::NotStorable`.
- The event schema has `additionalProperties: false`. A test fixture that needs
  to carry its own data must put it under `payload`, which is the only open
  object.
- **A raw string in Rust ends at the first `"#`.** `r#"{"$ref":"#/x"}"#` does
  not compile; write `r##"…"##`.
- Cross-target clippy needs the target installed. `x86_64-pc-windows-msvc` and
  `x86_64-unknown-linux-gnu` are; `aarch64-apple-darwin` is not, and asking for
  it fails with `E0463`.
- **MSVC builds without `cl.exe` on `PATH`.** The `cc` crate locates Visual
  Studio itself, so `rusqlite`'s `bundled` feature compiles SQLite 3.50.2 here
  with no environment setup. This was checked by building a probe, not argued.
- `jsonschema` was evaluated for `P1-T007` and **rejected**: it pulls
  `reqwest` + `rustls/aws-lc-rs` and roughly fifty transitive crates, which is
  the wrong shape for a local-first tool. Do not reintroduce it without reading
  `docs/architecture/PROTOCOL.md` §The validator, which records the two
  invariants the hand-written replacement must keep (an unsupported keyword is
  an error, not a skip; the schema is not treated as an annotation).

## External blockers

None. No task has been marked `block-external`. No credential or authorization
outside this machine has been needed yet.

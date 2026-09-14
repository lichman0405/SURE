# Autonomous handoff

Last updated: 2026-09-14
Branch: `claude/v0.1-autonomous`
Progress: 15 / 166 tasks accepted. Phase P0 complete (9/9). Phase P1 in progress
(6/13). `P1-T007` is accepted with the commit that carries this file.

Primary development host: Windows 11 x64 / native MSVC.

Canonical remote: `https://github.com/lichman0405/SURE.git`
Autonomous branch: `claude/v0.1-autonomous`

## Exact current state

`node scripts/taskctl.mjs status` reports:

```
Project: SURE | status: in_progress | phase: P1
{ accepted: 15, queued: 150 }
READY: P1-T005, P1-T011, P2-T001, P3-T001, P13-T001
```

Those five are independent of each other. `P1-T005` is the next one this session
takes, because `P1-T007` and `P1-T004` each left it an explicit decision to make:

- `docs/architecture/PROTOCOL.md` §Versions: the six stored documents carry no
  `schema_version`, and the version that matters for them is the **storage**
  version, which belongs on the stored record rather than on the document.
  `crates/sure-protocol/tests/round_trip.rs` has a test asserting the current
  silent-drop behaviour, so fixing it means deliberately deleting that test.
- `docs/architecture/STORAGE_AND_DATA_PATHS.md` and `sure_core::paths::Paths`
  already decide *where* storage lives and that authoritative evidence is
  outside the project. `P1-T004` built the paths; `P1-T005` puts bytes in them.

## Gate set, as run at `P1-T007`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace --all-targets` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo clippy --target x86_64-unknown-linux-gnu` | clean |
| `cargo clippy --target x86_64-apple-darwin` | clean |
| `cargo test --workspace --all-features` | 15 `test result: ok` lines, 341 passed, 0 failed |
| `node scripts/validate-bootstrap.mjs` | 17 phases, 166 tasks |
| `node scripts/taskctl.mjs validate` | state OK |

Test counts by target: `sure-domain` 133 + 87 + 29 + 9, `sure-protocol` 39 lib
+ 12 conformance + 15 round_trip, `sure-core` 8 + 7 + 2, `sure-testkit` and
`sure-cli` 0 (they carry no tests of their own yet).

## Adversarial (mutation) verifications in this session

Each was reverted after confirming the check fires. These are what make the
green above worth something; a conformance suite that cannot fail is a false
green inside the machinery that exists to prevent false greens.

- Renaming `RepairContract::issue_id` back to the wire name `issue` failed 3
  tests with `the document is missing "issue_id", which repair.schema.json
  requires`. This is how the `issue` → `issue_id` gap was found in the first
  place and how the fix was proved to be load-bearing.
- The validator's own test `an_unexpected_key_is_only_reported_where_the_schema
  _forbids_it` failed (left 0, right 1) and exposed a **real bug**:
  `additionalProperties: false` was checked inside the `properties` lookup, so a
  closed object with no `properties` silently allowed every key. Fixed by
  hoisting `closed` out of that guard; `a_closed_object_with_no_properties
  _allows_no_keys_at_all` is the named regression test.
- `every_supported_keyword_is_either_enforced_or_an_annotation` builds a
  violating document for each constraining keyword, so a keyword cannot be added
  to `SUPPORTED_KEYWORDS` and then parsed without being enforced.
- Earlier in this branch: a disallowed `sure-protocol -> sure-testkit` normal
  dependency made `repository_shape` fail; `"probably_fine"` added to
  `check-result.schema.json`'s `status` enum made the schema cross-check fail;
  making `config::read_if_present` treat every failure as "absent" made
  `a_directory_named_sure_yaml_is_reported_rather_than_treated_as_absent` fail.

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

## Next concrete action

1. `node scripts/taskctl.mjs start P1-T005` (local storage and migrations).
   Acceptance, verbatim: "Fresh/upgrade migrations pass." / "Storage packages
   without requiring system SQLite." / "Short-lived concurrent event writes have
   a defined safe behavior."
2. Implement it, run the gate set above, commit as `P1-T005: ...`, then
   `node scripts/taskctl.mjs accept P1-T005 --note "..."`.

`taskctl accept` takes `--note`, not `--evidence`; `--evidence` is silently
ignored, which is how the earliest tasks came to record an empty note.

## Environment notes for the next session

- **Run `cargo fmt --all` before the gate set, not after.** New files written by
  hand are not rustfmt-shaped (let-else bodies, long `assert!` messages) and
  `--check` fails on them. `P1-T007`'s commit was blocked once by this.
- Write repository files with LF endings. `core.autocrlf=true` plus
  `.gitattributes` (`* text=auto eol=lf`) means a Python `write_text` on Windows
  leaves CRLF in the working tree that shows as a phantom ` M` until
  `git add -A` re-hashes it. Write with `newline='\n'` or `write_bytes`.
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
- **`.gitattributes` and `git ls-files --eol` are the authority on line
  endings.** `grep -c $'\r'` gives a false positive on every file in this
  checkout. Use `git ls-files --eol` (`i/lf w/lf attr/text=auto eol=lf`) or
  `od -c` on the file.
- **A raw string in Rust ends at the first `"#`.** `r#"{"$ref":"#/x"}"#` does
  not compile; write `r##"…"##`.
- Cross-target clippy needs the target installed. `x86_64-apple-darwin`,
  `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` are; `aarch64-apple
  -darwin` is not, and asking for it fails with `E0463`.
- `jsonschema` was evaluated for `P1-T007` and **rejected**: it pulls
  `reqwest` + `rustls/aws-lc-rs` and roughly fifty transitive crates, which is
  the wrong shape for a local-first tool. Do not reintroduce it without reading
  `docs/architecture/PROTOCOL.md` §The validator, which records the two
  invariants the hand-written replacement must keep (an unsupported keyword is
  an error, not a skip; the schema is not treated as an annotation).

## External blockers

None. No task has been marked `block-external`. No credential or authorization
outside this machine has been needed yet.

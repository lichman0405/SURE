# Autonomous handoff

Last updated: 2026-09-14
Branch: `claude/v0.1-autonomous` @ `6f63801` (+ the `P1-T003` commit)
Progress: 12 / 166 tasks accepted. Phase P0 complete (9/9). Phase P1 in progress.

Primary development host: Windows 11 x64 / native MSVC.

Canonical remote: `https://github.com/lichman0405/SURE.git`
Autonomous branch: `claude/v0.1-autonomous`

## Exact current state

`node scripts/taskctl.mjs status` reports:

```
Project: SURE | status: in_progress | phase: P1
{ accepted: 12, queued: 154 }
READY: P1-T004, P1-T006, P1-T007, P2-T001
```

`P1-T003` was accepted with the commit that carries this file. `P1-T004`,
`P1-T006` and `P1-T007` are READY and independent of each other, as is
`P2-T001`.

## Completed commands and their evidence

The full gate set was run at every accepted task and passed. At `P1-T003`:

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace --all-targets` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features` | 87 + 61 + 29 + 9 + 8 + 7 + 2 passed, 0 failed |
| `node scripts/validate-bootstrap.mjs` | 17 phases, 166 tasks |
| `node scripts/git-guard.mjs check` | `lichman0405/SURE / claude/v0.1-autonomous` |

Adversarial (mutation) verifications, each reverted after confirming the check
fires:

- appending a disallowed `sure-protocol -> sure-testkit` normal dependency made
  `repository_shape` fail with the test-only and disallowed-edge messages;
- appending `"probably_fine"` to `schemas/check-result.schema.json`'s `status`
  enum made the schema cross-check fail and print both lists;
- making `config::read_if_present` treat every failure as "absent" made
  `a_directory_named_sure_yaml_is_reported_rather_than_treated_as_absent` fail,
  confirming that test guards the silent-fallthrough-to-defaults path.

## Accepted work in this session

- `4f2d75d` P0-T009 — foundational ADRs, plus `FROZEN_SEMANTICS.md`.
- `4ce2ce6` P1-T001 — declared crate boundaries, mechanically enforced by
  `sure_testkit::workspace` and `sure_testkit::integrations`.
- `6f63801` P1-T002 — `variants!` `ALL` lists and the pinned wire contract.
- `P1-T003` — `crates/sure-core/src/config/`: the `sure.yaml` model, loader,
  diagnostics and redaction, plus `docs/architecture/CONFIG_REFERENCE.md` and
  `docs/adr/0011-project-configuration-is-a-request.md`.

## Next concrete action

1. `node scripts/taskctl.mjs start P1-T004` (OS-native data path abstraction).
2. Implement it, then run the gate set above, commit as `P1-T004: ...`, then
   `node scripts/taskctl.mjs accept P1-T004 --note "..."` — the tool takes
   `--note`, not `--evidence`; `--evidence` is silently ignored, which is how
   the earlier tasks came to record an empty note.

## Environment notes for the next session

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

## External blockers

None. No task has been marked `block-external`.

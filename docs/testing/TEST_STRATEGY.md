# Test strategy

## Core rule

A false green is worse than a visible error/unknown.

## Unit tests

- IDs/enums/status aggregation;
- project fingerprints;
- intent-source trust labels;
- command classification;
- check planning;
- claim classification;
- evidence freshness;
- finding severity;
- repair schemas;
- redaction;
- protection decisions.

## Integration tests

Real temporary directories/processes/Git/SQLite/local HTTP servers.

Test Windows/macOS/Linux path and process semantics. Native Windows path/process behavior is release-critical, not optional CI coverage.

## E2E stacks

At minimum:
- TypeScript web/service fixture;
- Python API fixture;
- Rust CLI/service fixture.

## Adversarial acceptance

See `docs/testing/ADVERSARIAL_FIXTURES.md` and `evaluation/acceptance-manifest.json`.

## Golden language tests

Default report must not regress into engineering jargon or claim unknown evidence as fact.

## Harness tests

Use recorded synthetic hook/event payloads so CI does not require paid/authenticated live sessions.

Live harness smoke tests may be manual/optional and are recorded separately.

### A count of the tests a run reported must name its instrument

`cargo test` prints two numbers that both look like "the number of tests", and
they differ by the number of children that re-run a parent's test in a second
process. The parent figure is one `test <name> ... ok` line per test a parent
ran, and it answers how many tests there are. The raw sum is every
`test result:` line added up — the number cargo prints on the summary line —
and it counts a re-run test twice. In this repository ten children do that
(`store_concurrency`'s), so the raw sum is exactly 10 above the parent figure,
and a quote of `2508` or `2518` for the same tree is a quote of two different
instruments rather than a disagreement about the tree.

    node scripts/measure-tests.mjs target/tmp/gates-<label>-test.txt

prints both, states which is which, and prints no figure at all with exit 1 when
the log does not let it attribute one (a run that was interrupted, a log whose
output passed through a filter, a file that is not a test log). It reads a log
and needs nothing else — no build, no Git Bash, no administrator rights. It is
not one of the six gates: it measures a log another command produced, it decides
nothing about whether SURE is correct, and its header gives that choice, its
measured cost, and the instrument's history in writing.

The figure to write into `progress/state.json` and `progress/HANDOFF.md` is the
parent figure. The `passed=` line in `target/tmp/gates-<label>.txt` is not it:
measured on `gates-p15t016-record.txt`, that line reads `passed=2708` against
2698 `test ... ok` lines in `gates-p15t016-record-test.txt`.

### A recorded payload must not be load-bearing for one platform

A fixture value that is a path belongs to the platform it was written on. The
shipped hook fixtures keep a Windows one — `"project_root":
"C:\\Users\\dev\\sample-project"` in the cursor and claude-code
`session-start.json`, and the codex fixtures' `cwd` — because that is the example
worth shipping in a Windows-primary repository. What a test may not do is stand
on it: a test that needs a root SURE can *use* points the event at a directory it
made, the way `session_event_at` and `codex_event_at` in
`crates/sure-cli/src/hook.rs` rewrite that one field and read every other field
from the file.

`P15-T035` is the measurement behind the rule. Two session-start tests read their
fixture verbatim, so an absolute-on-Windows value reached
`Paths::ensure_settings_outside` and was refused there, and the branch's
`cargo test --workspace --all-features --no-fail-fast` was red on
`rust (macos-latest)` and `rust (ubuntu-latest)` — 188 passed, 2 failed, the same
two both times — while `rust (windows-latest)` and every Windows gate were green.

Windows path literals elsewhere in the tests are text rather than roots: a stored
session key (`adversarial_fixture_detection.rs`'s `CLAIM_PROJECT_ROOT`, written
through `Store::open_at`, which has no project boundary check), a command name
handed to the classifier (`command_safety.rs`), a rendered error
(`capability_report.rs`), a value whose *absence* is asserted
(`acceptance_report_runner.rs`, `release_gate_runner.rs`), a protocol round-trip
string, or a host path behind `#[cfg(windows)]`. The line to hold: a fixture value
may spell a path one platform's way; it must not be a path that a rule *resolves*
during the default test run.

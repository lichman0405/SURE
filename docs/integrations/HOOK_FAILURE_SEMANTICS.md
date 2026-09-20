# Hook failure semantics, per harness and per event

`docs/security/PROTECTION_MODE.md:105` requires this page to exist: *"Every
integration must document whether its hook failure behavior is fail-open or
fail-closed for the relevant event."* `docs/architecture/CONFIG_AUTHORITY.md:152`
names **P13-T007** as the task that owes it. `docs/security/THREAT_MODEL.md`'s
**T18** — *"A protection hook crashes and the user incorrectly assumes the action
was blocked"* — is the failure it exists to make visible.

The table in §3 is machine-checked. `crates/sure-testkit/tests/hook_failure_semantics.rs`
reads it, derives the harnesses and events from `integrations/*/hooks/hooks.json`
and from the launcher scripts actually present in the tree, and fails if a wired
event has no row, if a row's semantics cell is not one of the three permitted
values, if a `cannot confirm` row does not say what would settle it, or if an
evidence cell names a test that does not exist in this repository. Adding a hook
event to a manifest without answering this page is therefore a test failure, not
an omission a reader has to notice.

## 1. Two questions, kept apart

Every cell in §3 answers two different questions, and blending them is how a
false green gets written:

- **What SURE answers** when it cannot answer. This is measurable here, it is
  measured here, and §2 gives the measurements. It is the same for every
  harness: exit 5, a plain-language sentence on stderr (or a failure frame on
  stdout in `--format json`), and nothing written to the store.
- **What the harness does about it.** This repository cannot measure that. No
  test in it runs a harness. Every statement in that column is either a
  quotation from a vendor page named in §7, read on 2026-09-19, or `cannot
  confirm`. A sentence here without a source would be a claim about software
  this repository never ran.

**Fail-open** below means: when the hook fails, the action the hook was asked
about proceeds. **Fail-closed** means: when the hook fails, the action does not
proceed. Both are statements about the harness, not about SURE — SURE's side is
always "exit 5 and say so", and an exit status is not a policy until something
reads it.

### 1.1 The block path is a separate question, and one answer is already known

This page is about failure, but the same reading produced one finding that
belongs next to it, because it is T18 with a different trigger. SURE exits **1**
for a block (`crates/sure-cli/src/report.rs:597-601`) and **2** only for a usage
error. The Claude Code hooks page (read 2026-09-19) states that *"For most hook
events, exit code 2 is the only exit code that blocks through the code alone.
Without valid JSON on stdout, Claude Code treats exit code 1 as a non-blocking
error and proceeds with the action, even though 1 is the conventional Unix
failure code."* On that reading **SURE's block does not block a Claude Code tool
call**, in either output shape: the `claude-code` launcher passes no `--format
json`, so stdout is prose, not the JSON that could carry a decision.

That is recorded from upstream, not measured here, and it is not this task's to
fix — it is named so that the failure table is not read as if exit 1 and exit 5
were different kinds of thing to a harness. `crates/sure-domain/src/capability.rs`
already assigns Tier 1 (Observed) to Claude Code and Cursor, which is consistent
with it. For Cursor, Codex and Copilot this repository has no citable sentence
about what a non-blocking status does to a block, so §3 says `cannot confirm`
there and the same caveat applies to those rows' normal path.

## 2. What SURE emits — measured on this machine, 2026-09-19

### 2.1 The four failure inputs, at the process level

Measured by running `target/debug/sure.exe` directly with `--store-dir` pointed
at a scratch directory. `[F]` in §3 refers to this block, and
`crates/sure-cli/tests/cli_contract.rs`'s
`the_four_ways_a_hook_event_can_fail_exit_5_and_record_nothing` asserts it:
after each of these, the store directory contains no file at all and
`sure history --format json` reports zero total events with `store_present:
false`.

| Input | Exit | stdout | stderr (first two lines after the header) |
| --- | --- | --- | --- |
| empty stdin | 5 | empty | `No event was read from standard input.` / `The harness did not provide an event.` |
| stdin that is not JSON | 5 | empty | `The event could not be normalised.` / `the Cursor event is not valid JSON: …` |
| an event type with no mapping | 5 | empty | `The event could not be normalised.` / `the Cursor event type 'whatIsThis' is not one SURE knows how to map` |
| `--source copilot` | 5 | empty | `The event source is not supported.` / `Source 'copilot' is not one SURE knows how to ingest.` |
| no `--source` at all | 5 | empty | `No source was given.` / `Use --source <name> …` |

The same four inputs in `--format json` put a machine-readable failure frame on
**stdout** and leave stderr empty:

```json
{"command":"hook","details":{"detail":"The harness did not provide an event.","what":"No event was read from standard input."},"exit_code":5,"outcome":"failed","protocol_version":1,"sure_version":"0.0.0-bootstrap"}
```

There is no `decision` key in that frame, in either shape. A consumer that reads
`decision` and does not check `outcome` or `exit_code` reads nothing where the
one word that matters used to be — which is the safest reading available, and is
why the launchers never synthesise one.

### 2.2 The launcher layer, measured by running the launchers

`crates/sure-testkit/tests/hook_failure_semantics.rs` runs each launcher this
platform can run (`sure-hook.ps1` on Windows, `sure-hook.sh` through `/bin/sh`
on Unix) against a stand-in binary that writes a marker to stdout and stderr and
exits with a chosen status. Nothing below is read out of the script's text.

| Launcher | Passes `--format json` | SURE missing | SURE exits 0 | SURE exits 1 | SURE exits 5 |
| --- | --- | --- | --- | --- | --- |
| `claude-code/scripts/sure-hook.ps1` | no | exit 0, `allow` note on stderr | relayed | relayed | relayed |
| `claude-code/scripts/sure-hook.sh` | no | same | relayed | relayed | relayed |
| `cursor/scripts/sure-hook.ps1` | yes | exit 0, `allow` note on stderr | relayed | relayed | relayed |
| `cursor/scripts/sure-hook.sh` | **no** | exit 0, **both streams empty** | relayed | relayed | relayed |
| `codex/scripts/sure-hook.ps1` | yes | exit 0, stderr note with **no** `decision` | relayed | relayed | relayed |
| `codex/scripts/sure-hook.sh` | yes | same | relayed | relayed | relayed |
| `copilot/scripts/sure-hook.ps1` | yes | exit 0, `allow` note on stderr | relayed | relayed | relayed |

*Relayed* means: the launcher exits with the status the binary exited with, and
both streams pass through unchanged. The tests assert both, so a launcher that
started swallowing a status or rewriting a stream fails.

Every launcher fails open when SURE cannot be found: exit 0 with no stdout. That
is a deliberate choice with a consequence worth stating plainly — a missing SURE
binary is indistinguishable, to the harness, from SURE deciding "nothing to
report". Three of the four write a note on stderr saying so; the codex launchers'
note carries no `decision` field, and `cursor/scripts/sure-hook.sh` writes
nothing at all, so on that one path the only trace is the exit status.

The asymmetry in the second column is not cosmetic: it changes what a harness
receives when SURE fails. `cursor`'s `.ps1` and the `codex` launchers hand their
harness a well-formed failure frame on stdout, while `claude-code` and `cursor`'s
`.sh` hand theirs an empty stdout and prose on stderr.

That silence has one dependency. `cursor/scripts/sure-hook.sh`'s missing-binary
branch drains standard input with `cat`, so it is silent in an environment that
has the ordinary utilities on `PATH` — which is what a harness gives a hook — and
prints the shell's own `cat: command not found` on stderr in one that does not.
Measured 2026-09-19 under Git Bash by running the launcher with `PATH` pointing at
an empty directory. It is a property of the environment and not of the failure,
which is why the test that pins the silence gives the child `/usr/bin:/bin`: an
empty `PATH` there would have measured the test's setup instead of the launcher.

## 3. The table

`[F]` is §2.1. The `Failure semantics` cell holds exactly one of `fail-open`,
`fail-closed` or `cannot confirm`.

| Harness | Event | What SURE answers normally | When SURE cannot answer | Evidence in this repository | Failure semantics | The harness side, and its source |
| --- | --- | --- | --- | --- | --- | --- |
| claude-code | SessionStart | exit 0, allow | exit 5, nothing recorded `[F]` | `session_start_maps_correctly` | fail-open | Claude Code hooks page (2026-09-19): exit 2 is "the only exit code that blocks through the code alone"; any other non-zero, with no valid JSON on stdout, is a non-blocking error and "proceeds with the action". The launcher passes no `--format json`, so stdout is prose. |
| claude-code | PreToolUse | exit 0 or 1 depending on the decision | exit 5, nothing recorded `[F]` | `pre_tool_use_maps_correctly` | fail-open | Same sentence. A `PreToolUse` hook blocks on exit 2 only; 1 and 5 are non-blocking errors, so a SURE failure lets the tool call proceed. |
| claude-code | PostToolUse | exit 0, allow | exit 5, nothing recorded `[F]` | `post_tool_use_maps_correctly` | fail-open | The page: a `PostToolUse` hook "can't undo the call"; exit 2 there only surfaces stderr to the model. Nothing a failure can block. |
| claude-code | Stop | exit 0, allow | exit 5, nothing recorded `[F]` | `stop_maps_correctly` | fail-open | The page lists `Stop` as an event exit 2 can block; a failure is not exit 2, so the stop proceeds and the session ends. |
| cursor | sessionStart | exit 0, allow | exit 5, nothing recorded `[F]` | `session_start_maps_correctly` | fail-open | Cursor hooks page (2026-09-19): "Crashes, timeouts, and non-zero exit codes other than 2 fail open by default", and for `sessionStart` "the agent loop does not wait for or enforce a blocking response". |
| cursor | preToolUse | exit 0 or 1 depending on the decision | exit 5, nothing recorded `[F]` | `pre_tool_use_maps_correctly` | cannot confirm | The same page gives two rules that point opposite ways here: "Other exit codes - Hook failed, action proceeds (fail-open by default)", but also "invalid JSON or a response that doesn't match the hook's schema blocks the action", and "Permission hooks block on invalid JSON or an invalid response even when this [`failClosed`] is `false`" — `preToolUse` is one of the six hooks named. What would confirm it: the page does not say which rule wins when a permission hook both exits 5 and prints a well-formed frame that is not the hook's schema, which is exactly what `cursor/scripts/sure-hook.ps1` hands it. One Cursor run with this launcher and a broken SURE settles it. |
| cursor | postToolUse | exit 0, allow | exit 5, nothing recorded `[F]` | `post_tool_use_maps_correctly` | fail-open | The page: `postToolUse` is a post-observation hook that cannot block; "the run continues" on a hook failure other than exit 2. |
| cursor | postToolUseFailure | exit 0, allow | exit 5, nothing recorded `[F]` | `post_tool_failure_maps_correctly` | fail-open | The page: same post-observation class as `postToolUse`, which cannot block; a hook failure is logged and the run continues. |
| cursor | afterFileEdit | exit 0, allow | exit 5, nothing recorded `[F]` | `after_file_edit_maps_correctly` | fail-open | The page: `afterFileEdit` is listed among the post-observation hooks that cannot block; a non-2 non-zero exit fails open by default. |
| cursor | stop | exit 0, allow | exit 5, nothing recorded `[F]` | `stop_maps_correctly` | fail-open | The page: `stop` is "Not blocking" — it can only return an optional `followup_message` to auto-continue. A failure returns none, so the turn ends as it would have. |
| codex | SessionStart | exit 0, allow | exit 5, nothing recorded `[F]` | `session_start_maps_to_session_started` | cannot confirm | Codex hooks page (2026-09-19) defines exit 0 and exit 2 and no other status. What would confirm it: a sentence covering a non-2 non-zero exit for `SessionStart`, or a run of Codex with this launcher against a broken SURE. |
| codex | PreToolUse | exit 0 or 1 depending on the decision | exit 5, nothing recorded `[F]` | `pre_tool_use_maps_to_tool_requested` | cannot confirm | The page says exit 2 blocks `PreToolUse` and says nothing about 1 or 5. `integrations/codex/README.md:365-371` already records the neighbouring limit in the same words this cell needs: the deny path is "documented, not exercised … Whether Codex reads SURE's frame as a deny was not observed." What would confirm it: a Codex hooks statement about non-2 non-zero statuses. |
| codex | PostToolUse | exit 0, allow | exit 5, nothing recorded `[F]` | `post_tool_use_maps_to_tool_completed_and_keeps_the_response_verbatim` | cannot confirm | The page gives exit 2 for `PostToolUse` as feedback after the tool ran and no rule for 1 or 5. What would confirm it: the same statement asked for above. |
| codex | SessionEnd | exit 0, allow | exit 5, nothing recorded `[F]` | `session_end_maps_to_session_stopped` | fail-open | The page: "If a command times out or exits with an error, Codex reports it as a hook failure." For `SessionEnd` that report is the whole effect — the page describes its hooks as advisory with nothing left to gate. `integrations/codex/README.md:375-377` records that SURE sends the event anyway. |
| copilot | sessionStart | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-open | Copilot hooks reference (2026-09-19), for command hooks on Copilot CLI and the cloud agent: "Logged as a hook failure. The run continues (fail-open)." This package does not state which Copilot surface loads its manifest. |
| copilot | preToolUse | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-closed | The same reference, same section: "Exception: `preToolUse` is fail-closed—a non-zero exit (other than exit 2) denies the tool call", reported as `Denied by preToolUse hook (hook errored)`, and that is so "even if that JSON reports `permissionDecision: \"allow\"`". Read with §5: as things stand, a Copilot `preToolUse` hook that reaches SURE denies every tool call. |
| copilot | postToolUse | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-open | The same reference, same section: failures other than exit 2 "are logged and skipped" for events that are not `preToolUse` or `permissionRequest`. |
| copilot | postToolUseFailure | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-open | The same reference, same section. It also says exit 2 for this event is treated as `additionalContext`; exit 5 is in the fail-open set. |
| copilot | afterFileEdit | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | cannot confirm | The reference's event list does not contain `afterFileEdit`. What would confirm it: a Copilot document that defines this event, or a manifest rewritten to an event name one does. Until then this row says SURE exits 5 for an event whose failure rule nobody here can look up. |
| copilot | stop | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | cannot confirm | The reference defines `agentStop` (the VS Code configuration style spells it `Stop`), not `stop`. What would confirm it: a Copilot source that defines `stop` as a hook event, so that the row has a failure rule to quote at all. |

Reading the evidence column: a bare name is a test; `[F]` is §2.1. The
per-event tests in the evidence column are in
`crates/sure-core/src/normalizer/` — one `mod tests` per harness — and each one
feeds that harness's fixture for that event through the normaliser and asserts
the semantic event it maps to. The four failure rows share one process-level
test, `the_four_ways_a_hook_event_can_fail_exit_5_and_record_nothing`, in
`crates/sure-cli/tests/cli_contract.rs`; the launcher rows in §2.2 are the four
tests in `crates/sure-testkit/tests/hook_failure_semantics.rs` that spawn a
launcher. Copilot's rows cite `unsupported_source_fails` because that is the
branch every Copilot event lands in, and §5 is what that means.

## 4. What `cannot confirm` means here, and what would settle each

Six rows say `cannot confirm`. They are not one kind of uncertainty:

- **cursor / preToolUse** — the vendor page states both a general fail-open rule
  for non-2 non-zero exits and a permission-hook rule that blocks on a response
  that does not match the hook's schema, and does not say which wins when both
  apply. This is a real fork, and the launcher's behaviour sits on it: with
  `--format json` it hands Cursor a well-formed frame that is not Cursor's
  schema.
- **codex / SessionStart, PreToolUse, PostToolUse** — the vendor page defines
  only exit 0 and exit 2. There is no rule to quote, so there is no answer to
  give. `integrations/codex/README.md:365-371` already takes this position about
  the neighbouring deny path.
- **copilot / afterFileEdit, stop** — the event names in
  `integrations/copilot/hooks/hooks.json` do not appear in the vendor reference
  read here. Whether Copilot accepts, ignores or rejects the manifest is the
  prior question, and this repository does not answer it either.

Each cell names its own remedy. The common one is: run the harness once, with
this launcher, against a SURE that exits 5, and look at whether the action
happened. Nothing in this repository can do that, and no test in it pretends to.

## 5. The Copilot contradiction, measured

`grep -rn copilot crates/ --include=*.rs` returns nothing: there is no Copilot
normaliser. `--source copilot` therefore lands in `crates/sure-cli/src/hook.rs`'s
`Source '<name>' is not one SURE knows how to ingest.` branch, which is
`Report::Failed` → **exit 5, for every event in the manifest, always**.

Three statements about Copilot are in the tree at the same time:

1. The code above: every Copilot hook exits 5 and records nothing.
2. `integrations/copilot/README.md:156`: *"If `sure.exe` is missing, or if the
   hook cannot communicate with SURE, the adapter must fail open and must not
   fabricate evidence."* The launcher keeps that promise — it exits 0 when the
   binary is missing. SURE itself does not, because it never gets to answer.
3. `integrations/copilot/hooks/hooks.json`'s own `description`: a *"placeholder
   template. Replace Copilot-specific event names and response handling once
   Copilot's hook contract is documented."* The event names in it are Cursor's,
   and two of them are not names the Copilot reference defines.

Read against the reference in §3, (1) and (2) do not merely disagree, they
disagree in the direction that hurts: for `preToolUse` the documented rule is
**fail-closed on a non-zero exit**, so a package promising to fail open would,
if it were installed and wired up, deny every tool call.

This task does not settle it. It does not add a Copilot normaliser, and it does
not soften the README to match the code, because which of the two should change
is a product decision: either Copilot is a supported harness — in which case it
needs a normaliser and an event name list checked against the reference — or the
package is a template, in which case its README's "must fail open" sentence is
about a future adapter and should say so. The recommendation is the second, plus
moving `integrations/copilot/` out of any path a user could mistake for an
installable integration until the first is true. Nothing in this page should be
read as closing the contradiction; §3 records it.

## 6. What this page does not cover

- **The agent-plugin launcher.** `integrations/agent-plugin/` ships no hook
  script, so it has no row. Its launcher contract is the MCP one, and that one
  fails closed: `sure-mcp.ps1` exits 3 when it cannot find the binary, and its
  README says so. `crates/sure-testkit/tests/integration_thinness.rs` runs that
  script in four `#[cfg(windows)]` tests, so it is the one launcher in this tree
  whose failure path was already executed before this task.
- **Timeouts.** No launcher implements one (`integrations/codex/README.md:375-377`
  says why Codex has none), and the manifests set harness-side timeouts (10 s and
  20 s). What each harness does when a hook times out is not recorded anywhere in
  this repository, so no row above is about it. The pages in §7 disagree on how
  much it matters: Claude Code's says a timed-out hook "doesn't block the tool
  call … the call continues through the normal permission flow", Cursor's and
  Copilot's both say timeouts fail open by default (including for Copilot's
  `preToolUse`, which is otherwise fail-closed), and Codex's gives a default of
  600 s with no statement about the outcome. A SURE that hangs is a different
  failure mode from a SURE that exits 5, and the two must not be collapsed.
- **Permission settings.** A settings file SURE cannot read makes it decide under
  `InspectOnly` + `Strict` (`docs/architecture/CONFIG_AUTHORITY.md:146-152`) —
  that is a fail-closed *decision*, not a failure, and the status stays 0 or 1.
  It is out of scope for this table, and `crates/sure-cli/src/hook.rs`'s
  `a_refused_protection_mode_falls_to_the_firmest_one_this_build_has` covers it.
- **Evidence that could not be stored.** `crates/sure-cli/src/hook.rs:332-341`
  swallows a persistence failure on purpose: the decision stands and the exit
  status does not change. `a_decision_that_could_not_be_recorded_keeps_the_answer_and_says_so`
  asserts it. It is a second way for a harness to see an answer SURE could not
  record, and it is not in the failure table because the answer itself is real.

### 6.1 A packaging gap found while writing this, and closed by P15-T008

When this page was written, `git ls-files -s` reported every launcher script in
this repository at mode **100644**, including the `.sh` ones. A fresh Unix
checkout therefore could not execute `sure-hook.sh` by path, and the codex
manifest's `"${PLUGIN_ROOT}/scripts/sure-hook.sh"` is a direct invocation. The
tests here run `.sh` launchers through `/bin/sh <script>`, which works at 100644,
so this page's Unix claims were true of the scripts' contents; they were never
proof that a user could run them as shipped.

`P15-T008` closed the gap. The three `.sh` launchers are now recorded at
**100755** in the index, and
`the_unix_launchers_carry_the_executable_bit_in_the_index` fails if that stops
being true. Two things this page's numbers do *not* gain from that:

- Every Unix row above is still a measurement taken through `/bin/sh <script>`.
  Making the files executable did not re-measure them, and the
  `#!/usr/bin/env bash` shebang is still not exercised — these tests hand the
  file to a shell rather than starting it by path.
- No launcher's bytes changed. The mode is the only thing that moved.

`docs/integrations/INSTALLATION_MATRIX.md` carries the decision, the option that
was not taken and the cost of the one that was.

## 7. Sources

Read on 2026-09-19. These are citations, not measurements: nothing in this
repository fetches or pins them, and a page that changes does not fail a test
here. Every quotation in §3 is attributed to the row's own page.

- Claude Code hooks — <https://code.claude.com/docs/en/hooks>.
- Cursor agent hooks — <https://cursor.com/docs/agent/hooks>.
- Codex hooks — <https://learn.chatgpt.com/docs/hooks> (a 308 redirect from
  <https://developers.openai.com/codex/hooks>).
- Copilot hooks reference — <https://docs.github.com/en/copilot/reference/hooks-reference>
  (command hooks for Copilot CLI and the Copilot cloud agent). Earlier attempts
  to settle Copilot's failure rules from third-party sources produced
  contradictory answers; the first-party reference above is the only Copilot
  source quoted here, and where it is silent this page says `cannot confirm`
  rather than filling the gap.

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
with it. For Cursor and Codex this repository has no citable sentence about what
a non-blocking status does to a block, so §3 says `cannot confirm` there and the
same caveat applies to those rows' normal path. Copilot is the harness whose
reference does answer this, and answers it the other way: a non-zero exit from a
command `preToolUse` hook denies the tool call (§3, `copilot / preToolUse`), so
on that row SURE's block and SURE's failure would both be a denial to the
harness. That is one of the reasons the Copilot package ships as a template
nothing loads, and §5 is where that is recorded.

## 2. What SURE emits — measured on this machine, 2026-09-19

### 2.1 The failure inputs, at the process level

Measured by running `target/debug/sure.exe` directly with `--store-dir` pointed
at a scratch directory. `[F]` in §3 refers to this block. Every row in the table
below ends the run the same way: exit 5, a failure frame carrying **no
`decision` key**, and a store directory that afterward contains no file at all —
`sure history --format json` reports zero total events with `store_present:
false`.

The first five rows are inputs a harness produces **by accident** — a payload it
did not send, a source SURE does not know, an event SURE has no mapping for, a
command line with no source on it. They are asserted by
`the_inputs_a_harness_produces_by_accident_exit_5_and_record_nothing` in
`crates/sure-cli/tests/cli_contract.rs`, which runs each of them as a process.
The sixth row is the one a project writes **on purpose**, and it is the reason
this heading carries no count: `P15-T025` added it after the block was written,
and a count in a heading is a claim that goes false when a branch is added. It
is a decision SURE made rather than an accident it suffered, it is asserted —
with its own control — by
`a_hook_event_that_names_a_settings_file_inside_the_project_is_refused` in the
same file, and §2.4 argues why it is answered as a failure rather than as a
`decision`. **The first five rows were measured on 2026-09-19, the sixth on
2026-09-21**; the heading above carries the date the block was written.

| Input | Exit | stdout | stderr (first two lines after the header) |
| --- | --- | --- | --- |
| empty stdin | 5 | empty | `No event was read from standard input.` / `The harness did not provide an event.` |
| stdin that is not JSON | 5 | empty | `The event could not be normalised.` / `the Cursor event is not valid JSON: …` |
| an event type with no mapping | 5 | empty | `The event could not be normalised.` / `the Cursor event type 'whatIsThis' is not one SURE knows how to map` |
| `--source copilot` | 5 | empty | `The event source is not supported.` / `Source 'copilot' is not one SURE knows how to ingest.` |
| no `--source` at all | 5 | empty | `No source was given.` / `Use --source <name> …` |
| `--settings-file` naming a file **inside the project** (deliberate, §2.4) | 5 | empty | `SURE did not read the settings it was pointed at.` / `SURE reads the user's settings from …, and that is inside …, the project it was asked about.` |

Those inputs in `--format json` put a machine-readable failure frame on
**stdout** instead and leave stderr empty:

```json
{"command":"hook","details":{"detail":"The harness did not provide an event.","what":"No event was read from standard input."},"exit_code":5,"outcome":"failed","protocol_version":1,"sure_version":"0.1.0"}
```

There is no `decision` key in that frame, in either shape — and the last row of
the table is the same frame with the same two fields missing, which is what its
own test reads off the process rather than off the sentence (§2.4). A consumer
that reads `decision` and does not check `outcome` or `exit_code` reads nothing
where the one word that matters used to be — which is the safest reading
available, and is why the launchers never synthesise one.

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

The drain is not only about silence. The harness is the writer of that event, so
a launcher that exits without reading kills the harness's write — SIGPIPE on
Unix, a broken pipe on Windows — while answering exit 0, which loses the event
under a success. All three `.sh` launchers drain on the missing-binary branch
(`claude-code/scripts/sure-hook.sh:22`, `cursor/scripts/sure-hook.sh:13`,
`codex/scripts/sure-hook.sh:17`); `claude-code`'s was the third, repaired
2026-09-21 under `P17-T003` after the test began writing an event larger than a
pipe buffer (1 MiB, against a measured 69632-byte Windows anonymous pipe and
Linux's 64 KiB), because a payload the pipe cannot buffer is the only one whose
write cannot land on a launcher that never read it — measured before the repair
at 50/50 runs failing and after it at 0/50, where the old 200-byte payload had
failed 0/50 either way.

One exit in those three files still leaves the event unread: when `exec` itself
fails, the shell exits 126 with the write broken (measured 2026-09-21: writer
141, exit 126, all three). Reading the event to a temporary file and re-feeding
it to the exec would close that hole, and it is rejected on cost rather than on
impossibility: it would put a write of unbounded size to disk on every
invocation — the success path, which runs on every harness event — to protect a
path that already reports failure. Nothing is hidden from the harness either
way: it reads a hook that failed.

### 2.3 A project root SURE cannot use, and what happens to a `SessionStart`

A way a hook event stops before it answers that SURE chose rather than suffered.
The event is well formed, SURE knows the source and the event type, and what
stops it is that `project_root` is not an absolute path. It is not the only one
of those — `--settings-file` naming a file inside the project is the other, one
question earlier, and §2.4 is about it — and the count in this sentence is
therefore left out rather than guessed at, for the reason §2.1 gives. Both
shipped `SessionStart` fixtures carry
`"project_root": "C:\\Users\\dev\\sample-project"`
(`integrations/cursor/fixtures/session-start.json`,
`integrations/claude-code/fixtures/session-start.json`), which is absolute on
Windows and relative everywhere else — so on macOS and Linux the two examples
this repository ships are refused by this rule as written. That is what `P15-T035`
found in CI and repaired in the tests that read them; the fixtures keep their
value, and the tests now point the event at a directory they made themselves.

Measured at the process level, like §2.1 and asserted by
`a_project_root_that_is_not_absolute_is_refused_and_records_nothing` in
`crates/sure-cli/tests/cli_contract.rs`, which runs the binary against a relative
root and against a control that differs in nothing but the root: exit 5, a
failure frame on stdout in `--format json` with **no `decision` key**, nothing at
all on stdout in the human shape with the sentence on stderr, and no store and no
row afterwards.

**What SURE does, and why.** The root is read by two questions that each need one
location: the settings question, which asks whether the file that decides what
SURE may record and run is inside the project
(`Paths::ensure_settings_outside`), and the store's key for the session and the
event. SURE **refuses rather than resolves**, because a path resolved against its
own working directory is a project SURE *inferred* rather than read — it would
make which project an event belongs to depend on where the harness started the
hook — and because `sure check`'s `project_of` already refuses to do that much
repair to a path a person typed. The third option, skipping the settings question
and answering anyway, is not available either: the store asks the same question
of the same root and refuses it, so the decision would be the only thing left and
it would be an `allow` about nothing.

It errs toward **no evidence and a visible sentence**: a `SessionStart` has no
action for a decision to block, so the cost of the refusal is that session's
history, where the cost of the alternative is a stored project root naming a
directory the user never meant. A root that is *absent* is a different case and
not this rule — an event that names no project gets the process's own directory
(`crates/sure-cli/src/hook.rs:447`) — while an event that names an empty one has
named something SURE cannot use, and is refused.

**What a `SessionStart` gets, in each pack this repository ships.** The harness
column is §3's and the sources are §3's, applied to this one event:

| Pack | A `SessionStart` whose project root SURE refuses | Where that comes from |
| --- | --- | --- |
| `claude-code` | **fail-open.** Exit 5 is not exit 2, and the launcher passes no `--format json`, so Claude Code reads a non-blocking error and the session starts. SURE records nothing for it. | §3, `claude-code / SessionStart`, from the Claude Code hooks page |
| `cursor` | **fail-open.** "Non-zero exit codes other than 2 fail open by default", and for `sessionStart` "the agent loop does not wait for or enforce a blocking response". The `.ps1` launcher hands Cursor a failure frame carrying no `decision`; the session starts. | §3, `cursor / sessionStart`, from the Cursor hooks page |
| `codex` | **cannot confirm.** The page defines exit 0 and exit 2 and no other status, so what Codex does with 5 is not knowable from it. Both launchers relay the status, and the frame they pass on carries no `decision`. | §3, `codex / SessionStart`, from the Codex hooks page |
| `copilot` | **Nothing here reaches this rule.** The package is a template that nothing loads (§5), so no Copilot `sessionStart` arrives at SURE through it and the project-root question never arises. A user who wired the manifest up by hand would stop one branch earlier, for a different reason and with the same status — `--source copilot` is refused for *every* event (§5) — and the reference's rule for a non-zero exit on `sessionStart` is then the one their session follows. | §3, `copilot / sessionStart`, from the Copilot hooks reference |
| `agent-plugin` | **Nothing to answer.** The pack ships no hook script and has no `SessionStart` row (§6); this rule cannot reach it, and the launcher it does ship fails closed with exit 3 when the binary is missing. | §6, first bullet |

Two of the five — `claude-code` and `cursor` — are fail-open, `codex` cannot be
confirmed, and `copilot` and `agent-plugin` are not reached by this rule at all.
None is fail-closed. Where a config is reached, the direction is the safe one,
because a `SessionStart` has no action to block and the failure this rule exists
to prevent is the other one — a root SURE never read, written into the evidence
as though it had.

### 2.4 A settings file the project could have written: a decision, answered as a failure

The question this section exists to answer, stated before it is answered:

> When SURE refuses to read a settings file because it is inside the project the
> event is about, is it **deciding** — the shape every other pre-action answer
> takes, a `decision` of `block` and exit 1 — or has it **failed to produce an
> answer** — `outcome: "failed"` and exit 5?

The answer taken by `P15-T033` is that it **failed to produce an answer**: exit
5, `outcome: "failed"`, no `decision` key, nothing recorded. The last row of
§2.1's table is the measurement on this machine, and
`a_hook_event_that_names_a_settings_file_inside_the_project_is_refused` in
`crates/sure-cli/tests/cli_contract.rs` holds it against a real process, beside
the control that names no settings file and gets a `block` and a written store.

**The two shapes, side by side, on the same pre-action event.** Measured
2026-09-21 by running `target/debug/sure.exe hook ingest --source cursor
pre-tool-use` over one `preToolUse` event asking to run `rm -rf /`, against a
project that wrote `privacy: {full_recording: true}` into its own `sure.yaml`.
The two runs differ in one thing: whether that file was named with
`--settings-file`.

| Field | The refusal (`--settings-file` inside the project) | The control (no flag) |
| --- | --- | --- |
| exit status | 5 | 1 |
| `outcome` | `failed` | `not_green` |
| `decision` key | **absent** | `block` |
| `reason` key | absent | `The current execution mode does not permit this action.` |
| `details` | `what`: `SURE did not read the settings it was pointed at.`; `detail`: the file's path, that it is inside the project it was asked about, and `SURE stopped rather than treat it as the user's own word` | not present — a decision frame carries `reason` instead of `details` |
| store file afterwards | none | `sure.db`, one session row |
| `sure history --format json` | `store_present: false`, `total: 0` | `store_present: true`, `total: 1` |

Same event, same project, same command line apart from the flag, so the table is
a comparison rather than two anecdotes. Two neighbouring runs are worth one line
here, because they are the shapes a reader will meet next: a project `sure.yaml`
that does not parse, and one naming `protection.mode: custom`, both answer exit
1 / `block` / `not_green` and both write a store. Neither of them is this refusal
— neither names a `--settings-file` inside the project, so nothing is refused,
and what happens is only that the project's own file will not load. `sure hook
ingest` is the one caller that does not stop there: it answers under the settings
it falls back to, `InspectOnly` + `inspect_only` + `Strict` — `load_execution_config`'s
failure arm in `crates/sure-cli/src/hook.rs`, argued in
`docs/architecture/CONFIG_AUTHORITY.md:172-181` — which is a fail-closed
*decision* and is what §6's "Permission settings" bullet is about. The refusal
differs in kind: SURE did not read the file at all, so there is no fall-back
setting to decide under, which is why §2.4 exists.

**Why a failure rather than a verdict.**

1. **A `decision` is a verdict about the request, and this refusal read none of
   the request's facts.** It stops one question earlier than any decision does:
   no execution mode, no permission list, no protection mode, no classification
   of the tool or the command. `block`'s own sentence — *"The current execution
   mode does not permit this action."* — would be false here about a mode SURE
   never read, and a harness that trusts `decision` would be told SURE judged the
   tool call when SURE never looked at the tool call at all.
2. **The failure shape is what this repository already documents for this
   refusal, at every surface.** `docs/architecture/CLI.md` records it: *"A file
   inside it is status 5, with a sentence saying SURE stopped rather than treat
   it as the user's own word, and nothing is recorded"*, for `sure check`, for a
   hook and for `sure config set` alike. `check.rs` and both sites in `hook.rs`
   answer with `Report::Failed`, whose human form reads *"sure hook could not
   finish."* and closes with *"SURE exited with status 5, which is what it
   returns when it tried and did not finish."* This is a statement about what the
   tree says and not about what must stay: a reader who rejects the argument
   above is rejecting those documents too, and would have to change them rather
   than only this page.
3. **The block shape carries an invariant this refusal cannot satisfy
   honestly.** `hook.rs`'s `record_the_decision` holds that every decision the
   rule reached for a tool request is written down — *"absence is also what a
   decision SURE never reached looks like"*. A `block` here would have to persist
   the event to keep that rule, and what it would persist is a session and a tool
   request recorded **under settings SURE refused to read**; a `block` that wrote
   nothing would keep the store clean and break the invariant instead. The last
   row of §2.1 is the way out that was taken: nothing is recorded because nothing
   was decided.
4. **The measured benefit of the other shape is zero.** §3's per-harness column
   is where that belongs, and it says: on Claude Code, exit 1 and exit 5 are the
   same thing to the harness — a non-2 non-zero error, and the launcher passes no
   `--format json`, so no frame is read at all — and on Cursor and Codex what a
   non-2 non-zero status does to a block is `cannot confirm`. On Copilot a
   non-zero exit from `preToolUse` denies the call, so both shapes deny. A change
   of shape there buys nothing a harness was measured to notice.
5. **What the refusal owes is a sentence a person can act on, and it pays it.**
   The detail names the file, says it is inside the project it was asked about,
   and offers both remedies: name a settings file outside the project, or check
   the project from outside the folder that holds the settings.

**The case against, in its strongest form.** A consumer that reads only
`decision` sees *nothing* here, and that is not a hypothetical reader: `decision`
is the field every other pre-action answer puts its verdict in, and
`crates/sure-testkit/tests/hook_failure_semantics.rs` measures that the launchers
never synthesise one. So the project that writes its own `sure.yaml` — the case
`P15-T025` exists for — gets no answer about its tool call, where the same
project with a merely *broken* `sure.yaml` gets `block`. A protection layer that
answers "nothing" to the deliberate input and "block" to the accidental one has
its two cases round the wrong way.

**Why that case does not win.** The tension it names is real, and it is the
reason this shape is a decision rather than an oversight: whatever else a
`block` owed a reader here, it would have to be **written down**, and the one
place to write it is the store the refusal leaves absent. A `block` made
consistent with `record_the_decision` would create the very file
`fixtures/privacy/manifest.json`'s `a-settings-file-the-project-could-write-grants-nothing`
forbids — a session and a tool request recorded under settings SURE refused to
read, which is what the refusal exists to prevent — and a `block` made
inconsistent would answer `block` while recording nothing, which is the state the
invariant calls "a decision SURE never reached". The two shapes cannot both be
honest at once, so the question becomes which one SURE owes, and the answer is
the one it can back: it did not finish, and there is no verdict to write because
there was nothing to judge. The severity is worth stating plainly rather than
inflating: on the two harnesses whose rules §3 can cite, exit 1 and exit 5 are
the same thing, so no action proceeds here that a `block` would have stopped.
What is at stake is what SURE says about itself, not a hole through which a tool
call escapes.

**`check.rs` and the two hook sites, and the one thing that differs.** All three
answer the same way about the same input. Measured 2026-09-21, with the same
project and the same inside-the-project file named by `--settings-file`, each
answers `outcome: "failed"`, exit 5, no `decision` key, the same
`details.detail` (the file's path, that it is inside the project, and `SURE
stopped rather than treat it as the user's own word`), and no store file:

| Site | `details.what` |
| --- | --- |
| `sure check <project>` | `Nothing was recorded, and nothing was checked.` |
| `sure hook ingest` | `SURE did not read the settings it was pointed at.` |
| `sure hook allow-once` | `SURE did not read the settings it was pointed at.` |

The shape is the same and one sentence differs, because the consequence differs:
`check` was asked to check, so what did not happen is the check; a hook was asked
for an answer about an action, so what did not happen is the answer — and the
hook's sentence names the thing a person running it by hand has to fix. `what` is
a sentence and not a status: a harness reads `outcome`, `exit_code` and the
absence of `decision`, and those three agree at all three sites. No site falls
back to another settings file either — the refusal is reached before any settings
are read, and the runs above end in a failure with an empty store where a run
that had read *some* settings would have ended in a decision and a row, which is
what their controls do.

**What this section does not claim.** It does not claim that refusing
*authority a project asked for* should always be a failure — that is
`exit::REFUSED` (status 4), reserved in `crates/sure-cli/src/report.rs` for
exactly that family, unreachable without editing a file this task does not own,
and recorded as a lead rather than taken here. It does not claim any harness was
run: §3's harness column is untouched by this task, every `cannot confirm` in it
is still `cannot confirm`, and nothing here was measured about Cursor or Codex.

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
| copilot | sessionStart | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-open | Copilot hooks reference (2026-09-19), for command hooks on Copilot CLI and the cloud agent: "Logged as a hook failure. The run continues (fail-open)." Read with §5: this package is a template that nothing loads, so no Copilot session reaches SURE through it, and the cell is the harness's rule for a user who wired the manifest up by hand. Which Copilot surface loads a manifest is still not established here. |
| copilot | preToolUse | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-closed | The same reference, same section: "Exception: `preToolUse` is fail-closed—a non-zero exit (other than exit 2) denies the tool call", reported as `Denied by preToolUse hook (hook errored)`, and that is so "even if that JSON reports `permissionDecision: \"allow\"`". Read with §5: this is what a Copilot user would experience for **every tool call** if they wired the template up — SURE refuses `--source copilot` with exit 5, and 5 is a non-zero exit — so the row is a warning about installing the package and not a protection it provides. Nothing in this repository installs it. |
| copilot | postToolUse | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-open | The same reference, same section: failures other than exit 2 "are logged and skipped" for events that are not `preToolUse` or `permissionRequest`. Read with §5: no Copilot event reaches SURE through this package, so this is the harness's rule where the row above is the one a user would feel. |
| copilot | postToolUseFailure | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | fail-open | The same reference, same section. It also says exit 2 for this event is treated as `additionalContext`; exit 5 is in the fail-open set. Read with §5, as the rows above. |
| copilot | afterFileEdit | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | cannot confirm | The reference's event list does not contain `afterFileEdit`. What would confirm it: a Copilot document that defines this event, or a manifest rewritten to an event name one does. Until then this row says SURE would exit 5 for an event whose failure rule nobody here can look up — and §5 records that nothing loads the manifest naming it. |
| copilot | stop | exit 5 for every event — see §5 | exit 5, nothing recorded `[F]` | `unsupported_source_fails` | cannot confirm | The reference defines `agentStop` (the VS Code configuration style spells it `Stop`), not `stop`. What would confirm it: a Copilot source that defines `stop` as a hook event, so that the row has a failure rule to quote at all. §5 records that no manifest wiring `stop` is loaded by anything today, so the row is a name to fix before it is a rule to quote. |

Reading the evidence column: a bare name is a test; `[F]` is §2.1. The
per-event tests in the evidence column are in
`crates/sure-core/src/normalizer/` — one `mod tests` per harness — and each one
feeds that harness's fixture for that event through the normaliser and asserts
the semantic event it maps to. The failure inputs in §2.1 share two process-level
tests, and which input fell to which is the §2.1/§2.4 split: the accidental ones
by `the_inputs_a_harness_produces_by_accident_exit_5_and_record_nothing`, and the
deliberate one, with its control, by
`a_hook_event_that_names_a_settings_file_inside_the_project_is_refused` — both in
`crates/sure-cli/tests/cli_contract.rs`; the launcher rows in §2.2 are the four
tests in `crates/sure-testkit/tests/hook_failure_semantics.rs` that spawn a
launcher. Copilot's rows cite `unsupported_source_fails` because that is the
branch every Copilot event lands in. The same refusal is pinned one layer out,
at the process boundary and under the name itself, by
`every_harness_the_doctor_report_offers_is_one_sure_will_take_an_event_from` in
the same file: it runs `hook ingest --source copilot` and asserts the refusal,
rather than asserting it of a name a test made up. §5 is what the two of them
mean for the six Copilot rows.

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
  read here (`agentStop` is what it defines for a stop). The question that used
  to sit in front of this one is settled by §5 rather than answered: the manifest
  is a template that nothing loads, so no Copilot surface accepts, ignores or
  rejects it — none of them is handed it. What is still unresolved is the name
  itself, and a manifest that used a defined one would have a rule to quote
  where this row has none.

Each cell names its own remedy. The common one is: run the harness once, with
this launcher, against a SURE that exits 5, and look at whether the action
happened. Nothing in this repository can do that, and no test in it pretends to.

## 5. The Copilot package: what the command returns, and what it is

`grep -rn copilot crates/ --include=*.rs` returns hits, and what a reader should
find in them is what is not there. Measured 2026-09-21, the command returns
eleven lines in four files:

| File | Lines | What the hits are |
| --- | --- | --- |
| `crates/sure-cli/tests/cli_contract.rs` | 4 | the doctor test that asserts Copilot is not an offered source and that `hook ingest --source copilot` refuses it (a comment and the loop), and the `a source SURE does not know` case of `the_inputs_a_harness_produces_by_accident_exit_5_and_record_nothing` |
| `crates/sure-core/src/doctor.rs` | 1 | a doc comment on the doctor's integration list: a harness SURE ships a package for but cannot ingest from is deliberately not on it |
| `crates/sure-testkit/tests/hook_failure_semantics.rs` | 3 | that file's own platform note and the failure-shape arm for the package's launcher |
| `crates/sure-testkit/tests/integration_thinness.rs` | 3 | the argv the copilot launcher is expected to hand the core, and one text-shape assertion's package list |

Every one of them is a test, a test's message, or a comment. **None is a branch
that reads a Copilot event.** The narrower form of the same command is what
settles it, and a reader can run this one too:
`grep -rn copilot crates/sure-core/src/normalizer/ crates/sure-cli/src/hook.rs
--include=*.rs` returns nothing at all — no Copilot normaliser, and no Copilot
branch in the ingest path.

`--source copilot` therefore lands in `crates/sure-cli/src/hook.rs`'s
`Source '<name>' is not one SURE knows how to ingest.` branch, which is
`Report::Failed` → **exit 5, for every event in the manifest, always**. Measured
at the process level like §2.1: exit 5, a failure frame on stdout in
`--format json` saying `Source 'copilot' is not one SURE knows how to ingest.`,
nothing on stderr, and an empty store directory afterwards.

**The three statements that used to disagree here now agree, and this section
records the decision rather than the contradiction.** As of `P15-T023`:

1. The code: every Copilot hook exits 5 and records nothing. Unchanged, and
   unchanged on purpose — a normaliser is not what this package needed first.
2. `integrations/copilot/README.md` now opens by saying the package is a
   **template that does not answer**, that nothing in this repository installs
   it, that `sure doctor` does not offer it, and that its manifest is not to be
   loaded. Its fail-open sentence, *"If `sure.exe` is missing, or if the hook
   cannot communicate with SURE, the adapter must fail open and must not
   fabricate evidence"*, is now marked as **a requirement on the future adapter
   and not a description of this package** — because the shipped package cannot
   keep it. The launcher keeps half of it, which was measured here as well as in
   §2.2: with no SURE to find it exits 0 and writes no stdout, and with a SURE in
   front of it, whatever status that SURE returned is the status it relays.
3. `integrations/copilot/hooks/hooks.json`'s `description` says the same in the
   manifest a user would open before wiring anything up: it is a template, SURE
   refuses every event in it, and Copilot's fail-closed `preToolUse` rule is why
   loading it would deny every tool call. The event names in it are still
   Cursor's, and two of them are still not names the Copilot reference defines
   (§4).

Read against the reference in §3, the consequence of (1) is worth stating in one
sentence, because it is why (2) and (3) changed rather than the code: for
`preToolUse` the documented rule is **fail-closed on a non-zero exit**, so a
package that answered exit 5 to every event while its README promised to fail
open would, if it were installed and wired up, deny every tool call — the
opposite of what it said, with no evidence recorded for any of it.

What would make Copilot a supported harness is written down in the package's own
[Next steps](../../integrations/copilot/README.md#next-steps-to-make-this-real): a
normaliser, `--source copilot` wired into the ingest path, fixtures, and a
manifest whose event names the reference defines. Until the first two land,
**nothing installs this package by default and nothing describes it as Observed
or Protected tier** — no installer in the tree copies it, `sure doctor` reports
three integrations and Copilot is not among them, and the package's own
capability section claims no tier because a package that answers no event
contributes no event.

Two things this section still cannot settle, both named in §4: whether a Copilot
surface loads a hook manifest at all (nothing here can observe one), and the two
event names that the reference read here does not define.

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
  `InspectOnly` + `Strict` (`docs/architecture/CONFIG_AUTHORITY.md:172-181`) —
  that is a fail-closed *decision*, not a failure, and the status stays 0 or 1.
  It is out of scope for this table, and `crates/sure-cli/src/hook.rs`'s
  `a_refused_protection_mode_falls_to_the_firmest_one_this_build_has` covers it.
  A settings file **inside the project** is the neighbouring case and is not out
  of scope: SURE refuses to read it before that fallback is reached, so it is
  exit 5 and nothing recorded, and §2.4 is about it. The difference is whether
  the file was read at all: one that will not parse was read, and SURE fell back
  to the firmest settings it has; one inside the project was never read.
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

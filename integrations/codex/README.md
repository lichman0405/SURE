# SURE + Codex

A thin integration for OpenAI Codex (CLI, IDE extension and desktop app, which
share one configuration surface). It teaches Codex to invoke the local SURE
engine, to treat a SURE repair contract as the acceptance contract, and — since
this task — to hand SURE the session events Codex already emits.

Everything here is a manifest, a prompt, a config template or a launcher. There
is no checking logic in this package: every verdict comes from the local Rust
core (ADR 0004). The launchers read the payload Codex sends, find `sure.exe`,
and hand the bytes over.

## What is in this package

| Path | Codex surface | What it does |
| --- | --- | --- |
| `skills/sure-check/SKILL.md` | Skills (current) | Runs `sure check` and reports SURE's statuses without softening them. |
| `skills/sure-fix/SKILL.md` | Skills (current) | Runs `sure repair`, applies the contract, then runs `sure recheck`. |
| `prompts/check.md` | Custom prompts (deprecated upstream) | The same check flow for users who still invoke `/prompts:check`. |
| `prompts/fix.md` | Custom prompts (deprecated upstream) | The same repair flow for `/prompts:fix`. |
| `mcp.toml` | MCP config (`~/.codex/config.toml`) | Declares `sure mcp serve` as an MCP server so Codex can call the core directly. |
| `hooks/hooks.json` | Hooks (template) | Registers the four mapped events with a launcher. **A template, not an installed file**: `${PLUGIN_ROOT}` must be rendered first (step 3 below). |
| `scripts/sure-hook.ps1` | Hook launcher (Windows) | Reads the hook payload from stdin, invokes `sure hook ingest --source codex`, returns SURE's exit code. |
| `scripts/sure-hook.sh` | Hook launcher (POSIX) | The same for macOS and Linux. |
| `fixtures/*.json` | Test input | Codex-shaped payloads: four that map, one that is refused. Values are placeholders, not captured from a real session. |

The Agent Plugin package under `integrations/agent-plugin/` remains the portable
baseline that `docs/integrations/CODEX.md` names. This package is the
Codex-native surface on top of it.

## Capability tier

**This package installs at Tier 0 and reaches Tier 1 once the hooks are
installed. Tier 2 is deliberately not claimed.**

| Tier | State | Why |
| --- | --- | --- |
| Tier 0 — snapshot | What an install without the hooks gives you | The skills, prompts and MCP server all run `sure check`, which sees the project on disk. Nothing forwards session events. |
| Tier 1 — observed | **Achieved by this package, with the hooks installed** | Four documented Codex events (`SessionStart`, `PreToolUse`, `PostToolUse`, `SessionEnd`) are normalised, validated and stored, so SURE can say which tools Codex asked for, what it asked them to do, what they answered and when the session started and ended. A `sure check` of that project reads the tier back out of those stored events and says how many it counted. |
| Tier 2 — protected | **not claimed** | See below. |

The Tier 1 evidence, on **2026-09-18**, from this repository:

- The four mapped fixtures were driven through the real binary,
  `target\debug\sure.exe hook ingest --source codex`, both directly and through
  `scripts/sure-hook.ps1`, and ingested with `outcome: "ok"`.
- One run through the launcher moved the record count in
  `%LOCALAPPDATA%\SURE\sure.db` from 302 to 303 — the event landed in the
  store, keyed to the project directory the payload named.
- `crates/sure-cli/src/hook.rs` covers the same path with a scratch store: one
  session, four events, in order, for four payloads sharing one `session_id`.
- The one unmapped fixture is refused with `exit 5` and a sentence naming the
  reason, and leaves the store as it was.
- The round trip now runs end to end. On **2026-09-20**, four Codex fixtures
  were ingested into a scratch store under `target\tmp`, and
  `target\debug\sure.exe check <project> --store-dir <that store>` then reported
  `(capability tier 1, observed)` with
  `SURE counted 4 session events recorded for this project by codex, between
  2026-09-20T09:14:30.803Z and 2026-09-20T09:14:30.988Z.` The same command
  against a store with no store file reported only `(capability tier 0,
  snapshot)`; against a store holding another project's events it reported
  `(capability tier 0, snapshot)` followed by
  `SURE counted no session events for this project: SURE's store holds none for
  it. SURE also read 1 session event recorded for other projects; they are not
  part of this project's tier and were not counted.`

**What Tier 1 still cannot say.** SURE cannot confirm that Codex invoked the
hook. Nothing in this repository watched a Codex process call it: the payload
shape comes from Codex's documentation (see "Verified surface"), not from a
capture, and the exercised half of the bridge is SURE's side of it. SURE also
cannot say what Codex did between the events it receives: no Codex event carries
a timestamp, and turn structure, permission prompts and the user's own prompts
are not mapped (see the tables below).

**Tier 2 is deliberately not claimed.** Codex's public `PreToolUse` hook does
document a real pre-action decision — `permissionDecision: "deny"`, or exit code
`2` with a reason on stderr — so the platform can in principle carry one, and
`scripts/sure-hook.ps1` does return SURE's block decision and its exit code
unchanged. That is not the same as SURE having demonstrated enforcement. No
Codex binary was run from this repository; nothing here observed Codex acting on
a deny; the official docs describe tool hooks as "a guardrail, not a complete
enforcement boundary"; and non-managed hooks are skipped until the user reviews
and trusts them by hash (step 3 below). A Tier 2 claim needs an exercised
`PreToolUse` denial behind it. Until then, claiming it would be a false green.

## The bridge: what Codex sends and what SURE records

Codex runs each registered command hook with one JSON object on standard input.
Its documented fields are `session_id`, `transcript_path`, `cwd`,
`hook_event_name` and `model`, plus the event's own fields. The normaliser
(`crates/sure-core/src/normalizer/codex.rs`) reads that object as Codex wrote it
— it is the one normaliser that does not read a SURE-shaped fixture.

### Mapped

| Codex event | SURE event | What is kept from the payload |
| --- | --- | --- |
| `SessionStart` | `session.started` | `codex_source` (startup / resume / clear / compact), `model`, `transcript_path` |
| `PreToolUse` | `tool.requested` | `tool`, `args`, `turn_id`, `tool_use_id`, `permission_mode` |
| `PostToolUse` | `tool.completed` | the above plus `result`, which is the tool's response as Codex sent it |
| `SessionEnd` | `session.stopped` | `codex_reason` (Codex sends `"other"`) |

### Refused, by name, with the reason

Refused means: `exit 5`, nothing stored, and a sentence saying what SURE has
nothing to record the event as. This is the missing half of the parity, and it
is deliberate — mapping one of these onto the nearest-looking SURE event would
put a fact in the history that Codex did not send.

| Codex event | Why it is not mapped |
| --- | --- |
| `Stop` | It ends one *turn*, not the session, so `session.stopped` would record a session end that did not happen. The `last_assistant_message` it carries is the agent's own completion claim: that is `agent.claim`, and no SURE integration maps it yet. |
| `PermissionRequest` | SURE's model has no "the harness asked for permission" event; the decision that follows is a `PreToolUse` decision, which is already recorded. |
| `UserPromptSubmit` | The user's prompt is `user.request`, which nothing in SURE consumes yet; storing prompts with nothing to read them would be collection without a purpose. |
| `PreCompact`, `PostCompact` | SURE has no compaction event, and Codex does not report what was dropped, so a "compaction happened" row would say nothing SURE could act on. |
| `SubagentStart`, `SubagentStop` | SURE's session model has no subagent relationship yet, so the rows could not be tied to the session that spawned them. |
| `Interrupt` | SURE has no cancelled-run event type. |

Two fields every Codex payload lacks, and what SURE does instead:

- **No event carries a timestamp.** SURE's envelope requires one
  (`schemas/event.schema.json`) and does not invent missing data, so the instant
  recorded is when SURE read the event, and every payload says so:
  `"timestamp_source": "sure_ingest_clock"`.
- **`session_id` and `cwd` are required, not defaulted.** Without a `session_id`
  the store would record one session per event; without a `cwd` SURE would have
  to guess which project the evidence belongs to, and a payload that omits
  either is refused rather than guessed at.

## What these commands answer in the current build

Re-checked on **2026-09-19** by running `target\debug\sure.exe --format json`
against this repository (an absolute path), from the repository root, with
`--store-dir` naming an empty directory under `target/tmp` so that no row
depends on the store of whoever ran it. The first observation was 2026-09-18 and
the rows below are the ones that still hold; the `sure recheck` row is the one
whose wording changed, and it changed because the store it ran against was
empty.

| Command | What this build answers |
| --- | --- |
| `sure check <abs>` | Runs the whole twelve-stage pipeline and stops at `not_green`, exit 1: "Not enough could be checked to say whether this is ready." 0 of 18 planned checks produced a result: the 14 static checks "read your project's files and run nothing" and this build has no runner for them, so each is recorded as unknown rather than passed, and the 4 dynamic ones were stopped by `inspect_only`. The report's capability line is "capability tier 0, snapshot", which is what a run whose `--store-dir` holds no store yet reports; with a store in place the same line says what it counted (see "What a recorded session changes" below). |
| `sure repair <abs>` | Exit 1, `not_green`. Stage 11 ran and answered "no check produced a finding, so there is nothing to write instructions for." No repair contract was produced. |
| `sure recheck <abs>` | Exit 1, `not_green`. Stage 12 ran and answered "SURE has no recorded history for this machine, so there is no earlier run to compare this one with." — which is what an empty store produces; with history in the store the sentence names what the earlier run left open. |
| `sure mcp serve` | Runs and answers, and says nothing on stdout until it is asked something. With stdin closed at once: **0 bytes on stdout**, the session summary on stderr — one `{"command":"mcp",…,"answered":0,…}` frame under `--format json`, the same summary in words under the default format — and exit 0. |
| `sure version`, `sure protocol`, `sure doctor` | Exit 0. |
| `sure check crates` | Refused, exit 5: "SURE was asked to look at "crates", which is not a full path." A short path would be resolved against wherever SURE happened to start, "so the same command would read a different project depending on where it ran". Always pass an absolute path. |

Every artifact here resolves the binary, invokes the command, and reports a
refusal as a refusal; none of them turns one into a result. `docs/architecture/CLI.md`
is the authority for which commands a given build carries out.

## What a recorded session changes

Recording Codex events changes what `sure check` says about the project, and it
changed on **2026-09-20**: the check pipeline now reads the capability tier from
the project's own recorded events
(`crates/sure-core/src/capability_report.rs::for_project`, reached at stage 10 of
the pipeline) instead of from the command line. Until that task, the line said
tier 0 even with a session's events in the store; the paragraph that said so is
gone because the behaviour it described is gone.

Four cases, and they read differently on purpose — the point is that a reader can
tell them apart:

| The store | What the capability line says |
| --- | --- |
| No store at all (a bare `sure check`, or `--store-dir` naming a directory with no store file) | `(capability tier 0, snapshot)` and nothing about counting: nothing was counted, so the report does not claim it looked. |
| A store that holds no events for this project | `(capability tier 0, snapshot) SURE counted no session events for this project: SURE's store holds none for it.` |
| A store that holds events for *other* projects | The same, followed by `SURE also read N session events recorded for other projects; they are not part of this project's tier and were not counted.` |
| A store that holds this project's session events | `(capability tier 1, observed) SURE counted N session events recorded for this project by codex, between <oldest> and <newest>.` — which is Tier 1, from the events rather than from a claim. |

Each line below the first — the store was read, and counted nothing, something
else's, or this project's events — is followed by `What SURE still cannot see:`
and the gaps that remain at that tier. A project SURE counted nothing for
therefore still says in full that it cannot see the session and cannot ask
before a dangerous action; the counted sentence adds to that account rather than
replacing it. The first line is the command line's own report and is unchanged
by any of this.

Three things this deliberately does **not** do:

- **It does not raise the tier to Tier 2.** Events prove what happened, never
  that SURE could have stopped it, so a report derived from events says it cannot
  ask before a dangerous action whatever `capability_tier` the adapter put on its
  own envelopes. Tier 2 stays [not claimed](#capability-tier).
- **It does not borrow another project's session.** The events are matched to the
  project directory they were recorded for, so a sibling checkout's session never
  becomes this project's tier.
- **It does not report a failed read as an empty one.** A store SURE could not
  read reports the tier it can prove without those events and says, in the run's
  own stage log, what stopped it — rather than reporting "no session" about a
  store it never managed to look inside.

A session's events also name the gaps that remain, and they are not the same gaps
as at tier 0. At tier 0 the honest statement is the single "SURE only sees the
project as it is now"; at Tier 1 SURE says which kinds of event it did **not**
get, and for the four events this package maps those are: the user's request, the
agent's completion claim, command failures, file edits and Git activity. They are
gaps by construction rather than by accident — the Codex events that would carry
them are in the "Refused, by name" table above, and a `PostToolUse` becomes a
`tool.completed` whatever tool it describes — and the report names them on every
run. More gaps, named more sharply, is what knowing more looks like here.

## Install (Windows, no administrator rights)

Everything installs into your own user profile. No elevated shell is required,
and nothing is installed machine-wide.

### 1. Get `sure.exe` onto the resolution path

The integration, the prompts, the MCP template and the hook launchers all
resolve the SURE binary in this order:

1. `$env:SURE_BIN` — explicit override.
2. `sure` on PATH (via `Get-Command sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` — per-user install location.

If none of the three is found, the hook launcher writes
`{"acknowledged":false,"reason":"SURE binary not found","source":"codex"}` to
stderr and exits 0, so a missing SURE never blocks the session, and no decision
appears that SURE did not make.

### 2. Install the skills (recommended)

Codex loads skills from `$HOME/.agents/skills` (personal) and from
`.agents/skills` in the repository (project-scoped). From the repository root:

```powershell
$dst = Join-Path $HOME ".agents\skills"
New-Item -ItemType Directory -Force -Path $dst | Out-Null
Copy-Item -Recurse -Force "integrations\codex\skills\sure-check", "integrations\codex\skills\sure-fix" $dst
```

Restart Codex, or open a new chat, so the skills load. In Codex, `$sure-check`
and `$sure-fix` (or `/skills`) invoke them; the `description` in each `SKILL.md`
also lets Codex select them implicitly when the task matches.

For a project-scoped install visible to the whole team, copy the same two
directories to `.agents\skills\` at the repository root and commit them.

### 3. Install the evidence hooks (this is what reaches Tier 1)

Copy the launchers, render the template, and put the result where Codex reads
hooks from (`~/.codex/hooks.json`, or a `[hooks]` table in `~/.codex/config.toml`):

```powershell
# a. Copy the launchers somewhere they will stay.
$hooks = Join-Path $env:LOCALAPPDATA "SURE\codex-hooks"
New-Item -ItemType Directory -Force -Path $hooks | Out-Null
Copy-Item -Recurse -Force "integrations\codex\scripts\*" $hooks

# b. Render the template. ${PLUGIN_ROOT} is SURE's placeholder, not a Codex
#    variable: Codex would run the literal text and the hook would fail.
#    The path is backslash-escaped because the placeholder sits inside a JSON
#    string: a plain substitution produces `C:\Users\...` and invalid JSON.
New-Item -ItemType Directory -Force -Path (Join-Path $HOME ".codex") | Out-Null
$escaped = $hooks.Replace('\', '\\')
(Get-Content "integrations\codex\hooks\hooks.json" -Raw).Replace('${PLUGIN_ROOT}', $escaped) |
    Set-Content (Join-Path $HOME ".codex\hooks.json") -Encoding utf8
```

Then **review the file and trust it**. Codex skips non-managed hooks until the
user has reviewed and trusted them, tracked by hook hash — so a copy of
`hooks.json` that was never trusted is a hook that never runs. If you edit the
file afterwards the hash changes and Codex asks again. That review step is
Codex's, not SURE's, and SURE cannot see whether it happened.

The rendered entry for Windows runs:

```
powershell -NoProfile -File "…\SURE\codex-hooks\sure-hook.ps1"
```

No `-ExecutionPolicy Bypass` on purpose: SURE does not override the machine's
script-execution policy for a command the harness runs unattended. On a machine
whose policy forbids local scripts the hook fails visibly, with the policy error
on stderr, instead of the launcher silently doing nothing about it. Set
`$env:SURE_BIN` if `sure.exe` is not on PATH and not in the per-user location.

What was measured for the launchers, on **2026-09-18**:

- All four host/delivery combinations deliver the payload unchanged:
  `powershell` 5.1 and `pwsh` 7, each with stdin as a pipe and as a redirected
  file. A refused event came back as the same JSON frame with exit 5 in all
  four.
- Windows PowerShell 5.1 writes a UTF-8 byte-order mark in front of whatever it
  pipes to a native command (bytes `EF BB BF`, with stdin as a pipe and as a
  file, whatever `$OutputEncoding` is set to; `pwsh` 7 writes none). SURE drops
  a leading mark before reading the event, which is what makes the 5.1 path
  work at all; before that it refused every such event as invalid JSON.
- The launcher declares no `param` block, deliberately: with one, PowerShell 5.1
  tries to bind each line of the event as a parameter and the event never
  arrives.
- With SURE absent, all four combinations wrote
  `{"source":"codex","reason":"SURE binary not found","acknowledged":false}` to
  stderr and exited 0.
- `scripts/sure-hook.sh` was exercised under Git Bash as an interoperability
  check only; the Windows path is the PowerShell launcher.

### 4. Or install the custom prompts

Custom prompts are invoked as `/prompts:<name>` and must sit directly in the
prompts folder (Codex reads only top-level Markdown files there):

```powershell
$dst = Join-Path $HOME ".codex\prompts"
New-Item -ItemType Directory -Force -Path $dst | Out-Null
Copy-Item -Force "integrations\codex\prompts\check.md", "integrations\codex\prompts\fix.md" $dst
```

Then `/prompts:check` and `/prompts:fix` are available after a restart.

> Upstream status: the official Codex documentation states "Custom prompts are
> deprecated. Use skills for reusable instructions." The prompts folder still
> works, so this package keeps both. Prefer the skills.

### 5. Register the MCP server (optional)

Append the block in `mcp.toml` to `%USERPROFILE%\.codex\config.toml`:

```powershell
Get-Content "integrations\codex\mcp.toml" -Raw | Add-Content "$HOME\.codex\config.toml"
```

Or use the Codex CLI, which writes the same entry:

```powershell
codex mcp add sure -- sure mcp serve
```

`sure mcp serve` is the stdio MCP bridge described in
`docs/architecture/MCP_BRIDGE.md`, and it is implemented (P12-T009). It is the
same surface the portable Agent Plugin declares in
`integrations/agent-plugin/mcp.json`, and the packages were wired to it in
P12-T010.

**The template's `command = "sure"` is a `PATH` requirement**, which is why the
comment at the top of `mcp.toml` gives the absolute-path alternative: a Codex
config is a file a person edits, so it can name either. Use the Codex CLI
(`codex mcp add sure -- sure mcp serve`) or replace the command with the
absolute path to the per-user install — `C:\Users\you\AppData\Local\SURE\bin\sure.exe`.
The resolution order everywhere in this package is `$env:SURE_BIN`, then `sure`
on `PATH`, then that per-user location.

What a caller sees when the binary is missing or too old, for the packages whose
manifests SURE controls rather than the user: the Claude Code launcher refuses
with exit 3 and one paragraph on stderr naming where it looked, and never with an
empty tool list;
the decisions are written down in `integrations/claude-code/README.md` and
`docs/architecture/MCP_BRIDGE.md`. A Codex config that names a binary which is
not there is Codex's own error to report, since the file is the user's.

### Uninstall

Remove the `hooks` entry from `~/.codex/hooks.json` and delete the launchers,
then undo whichever of the other steps you took. Nothing is installed
machine-wide.

```powershell
Remove-Item -Recurse -Force (Join-Path $env:LOCALAPPDATA "SURE\codex-hooks") -ErrorAction SilentlyContinue
Remove-Item -Force (Join-Path $HOME ".codex\hooks.json") -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force (Join-Path $HOME ".agents\skills\sure-check"), (Join-Path $HOME ".agents\skills\sure-fix") -ErrorAction SilentlyContinue
Remove-Item -Force (Join-Path $HOME ".codex\prompts\check.md"), (Join-Path $HOME ".codex\prompts\fix.md") -ErrorAction SilentlyContinue
```

The events already recorded stay in `%LOCALAPPDATA%\SURE\sure.db`; removing the
hooks stops new ones being added.

## What this package deliberately does not do

- **No checking logic.** No file here decides whether a project passes. The
  integration tree is checked for this in
  `crates/sure-testkit/tests/integration_thinness.rs`, which also bounds how
  long a launcher may be.
- **No inventing the payload.** The normaliser reads Codex's documented field
  names; a payload that is missing a required field, or carries an event SURE
  has no meaning for, is refused with a reason instead of being bent onto the
  nearest SURE event.
- **No fabricated timestamp.** Codex sends none, so SURE records when it read
  the event and labels it as such in the payload.
- **No copied user-facing verdict wording.** The core owns the sentences SURE
  says to users, including the after-the-fact limitation. A copy in an
  integration would be a second source of truth that goes stale.
- **No Tier 2 claim.** See above.
- **No invented evidence.** If `sure` is not found, every artifact here says so
  and stops. None of them produce a plausible-looking result instead.
- **No duplicated engine or vendored binary.** `integrations/` may not contain
  `.rs`, `.dll`, `.so`, `.dylib`, `.a`, `.rlib` or `.exe`.

## Verified surface

Checked on **2026-09-18** against the official OpenAI Codex documentation.
`developers.openai.com/codex/*` currently 308-redirects to
`learn.chatgpt.com/docs/*`; the pages below were read at the redirected
location. Append `.md` to any docs URL for the raw Markdown.

| Claim | Where it was verified |
| --- | --- |
| Skills load from `$HOME/.agents/skills` (user), `$CWD/.agents/skills`, `$CWD/../.agents/skills`, `$REPO_ROOT/.agents/skills` (repo); `SKILL.md` must carry `name` and `description` front matter. | `learn.chatgpt.com/docs/build-skills.md` |
| Custom prompts live in `~/.codex/prompts/*.md`, are invoked as `/prompts:<name>`, must be top-level Markdown, and are **deprecated** in favour of skills. | `learn.chatgpt.com/docs/custom-prompts.md` |
| MCP servers are declared as `[mcp_servers.<name>]` in `~/.codex/config.toml` (or a trusted project's `.codex/config.toml`), with `command`, `args`, `env`, `env_vars`, `cwd`, `enabled`, `startup_timeout_sec` and `tool_timeout_sec`. | `learn.chatgpt.com/docs/extend/mcp` |
| `config.toml` precedence: CLI flags, project config (trusted projects only), profile files, user config, cloud defaults, system config, built-in defaults. | `learn.chatgpt.com/docs/config-file/config-basic` |
| `AGENTS.md` is the project instruction file, loaded from `~/.codex/AGENTS.md` (or `AGENTS.override.md`) and from each directory between the project root and the working directory. | `learn.chatgpt.com/docs/agent-configuration/agents-md.md` |
| Hooks are read from `~/.codex/hooks.json` or `[hooks]` in `config.toml`; the twelve events are `SessionStart`, `SessionEnd`, `SubagentStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStop`, `Stop`, `Interrupt`; every command hook receives one JSON object with `session_id`, `transcript_path`, `cwd`, `hook_event_name` and `model`; `PreToolUse` adds `tool_name`, `tool_use_id`, `tool_input`, `permission_mode` and `turn_id`; `PostToolUse` adds `tool_response`; `SessionStart` adds `source`; `SessionEnd` adds `reason`; `PreToolUse` can deny with `hookSpecificOutput.permissionDecision: "deny"`, or exit code `2` with a reason on stderr; non-managed hooks require review and trust, tracked by hook hash. | `learn.chatgpt.com/docs/hooks.md` |
| Codex's tool vocabulary for the hooks includes `Bash` (also `exec_command`), `apply_patch` (matched as `apply_patch|Edit|Write`), MCP tools such as `mcp__filesystem__read_file`, `update_plan` and `spawn_agent`. | `learn.chatgpt.com/docs/hooks.md` |

`AGENTS.md` is a verified surface that this package does not ship a file for. A
team that wants SURE to be consulted without anyone typing a slash command can
add one line to its own `AGENTS.md`. That is the project's instruction file, not
this package's to write.

### Not verified

- **No Codex binary was run from this repository**, so nothing here is evidence
  that Codex invokes these hooks, that it accepts the rendered `hooks.json`, or
  that it acts on SURE's decision. Everything above about Codex's side is from
  documentation, and that surface is version-dependent (Codex hooks became
  inline-`config.toml` capable in the v0.124 line). Re-check the pages before
  relying on exact key names.
- **The `PreToolUse` deny path is documented, not exercised.** `sure hook`
  returns `{"command":"hook","decision":"block","reason":…}` with exit 1, which
  is the shape `hook_protection` already used for Claude Code and Cursor. The
  hooks page also mentions an older accepted top-level `decision: "block"` with
  a `reason`, but the documented current shape is `hookSpecificOutput` with
  `permissionDecision`. Whether Codex reads SURE's frame as a deny was not
  observed. Nothing in this package's tier claim rests on it.
- **The exact `tool_input` / `tool_response` shapes were not observed.** They are
  kept in the event payload verbatim, so SURE records what Codex sent without
  interpreting it; no check reads them yet.
- **`timeout` and `statusMessage` are not set** in the template, because their
  current keys and semantics for Codex were not verified. A hook that hangs
  therefore hangs by Codex's default, not by a value SURE chose.
- **Behaviour on non-Windows hosts was not tested end to end.** The POSIX
  launcher was run under Git Bash on Windows only. The paths in this README are
  written for Windows; the Codex-side locations (`$HOME/.agents/skills`,
  `~/.codex/config.toml`, `~/.codex/hooks.json`) are the documented user-scoped
  ones and should be identical elsewhere.
- **`sure hook ingest` writes to the store the platform's own location names unless
  the caller says otherwise, and a hook launcher does not say otherwise.**
  Running it as recorded above wrote `%LOCALAPPDATA%\SURE\sure.db`. `--store-dir
  DIR` (P1-T012) is the way to point it at a store of your own, and this is what
  the tests that need one use: `crates/sure-cli/tests/cli_contract.rs` runs a real
  ingest against a scratch store under `target/tmp/` and asserts the machine's own
  store is byte-identical afterwards. The launcher scripts here deliberately do
  **not** pass it: a hook whose events landed in a scratch store would be a hook
  whose events the verdict never reads.

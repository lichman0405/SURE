# SURE Copilot adapter boundary

**This package is a template, and it does not answer.** Nothing in this repository installs it, `sure doctor` does not offer it, and no event sent through it is recorded: SURE has no Copilot normaliser, so `sure hook ingest --source copilot` exits 5 for every event `hooks/hooks.json` wires, and writes nothing. **Do not load that manifest or wire this package up.** Command `preToolUse` hooks are fail-closed in Copilot's own reference — a non-zero exit denies the tool call, and it is reported as `Denied by preToolUse hook (hook errored)` — so a session that loaded the manifest would have every tool call denied while SURE recorded no evidence for any of them. `docs/integrations/HOOK_FAILURE_SEMANTICS.md` §5 carries the measurement.

Everything below is the boundary between SURE core and a hypothetical GitHub Copilot adapter: what a future Copilot integration must implement to reuse the same event/repair protocols the Claude Code and Cursor integrations use without duplicating core logic. Read it as a specification for that adapter. Where it says *must*, it states a requirement on code that does not exist yet — including the fail-open requirement in [Fail-safe behavior](#fail-safe-behavior), which the launcher shipped beside this file keeps only for the case where the binary is missing.

## Design principle: thin adapter, thick core

The adapter is only a harness-specific translator. It does not check the project, decide what is safe, or store evidence. It forwards events to SURE core and returns SURE's decisions back to Copilot. This is the same rule the existing integrations follow:

- `integrations/claude-code/README.md` — "The hook is intentionally thin: it forwards stdin JSON to the local SURE core. Core checking/protection logic does not live in the script."
- `integrations/cursor/README.md` — "SURE uses Cursor Plugin + hooks/commands rather than starting with a heavy VS Code/TypeScript extension."
- `docs/adr/0004-thin-harness-integrations.md` — the rationale for keeping integrations thin.

## Protocol surface the adapter must implement

### 1. Event ingestion: `sure hook ingest --source copilot`

The adapter must pipe a normalized JSON event to SURE on stdin and invoke:

```text
sure hook ingest --source copilot <event-kind>
```

`<event-kind>` is one of the hook-specific tokens SURE uses to decide whether to evaluate a protection decision. The current code maps the token to the envelope's `event_type` in `crates/sure-cli/src/hook.rs`:

```rust
let is_pre_tool_use =
    event_kind == Some("pre-tool-use") || envelope.event_type == "tool.requested";
```

The adapter must therefore pass `pre-tool-use` for the pre-action hook; other lifecycle events can use the matching token or leave SURE to infer the type from the envelope.

The stdin contract is documented in `docs/architecture/EVENT_PROTOCOL.md` and enforced by `sure_protocol::event::EventEnvelope`. A minimal valid envelope is:

```json
{
  "schema_version": 1,
  "source": "copilot",
  "event_type": "tool.requested",
  "timestamp": "2026-09-18T12:01:00Z",
  "capability_tier": 1,
  "session_id": "copilot-session-001",
  "project_root": "C:\\Users\\dev\\sample-project",
  "payload": {
    "tool": "terminal",
    "args": { "command": "npm test" }
  }
}
```

Required fields are exactly `schema_version`, `source`, `event_type`, and `timestamp`. The adapter may omit optional fields it cannot fill, but it must not invent data. Unknown fields are refused (`deny_unknown_fields` in `crates/sure-protocol/src/event.rs`).

The adapter should call `sure protocol --speaks <version>` before sending events, using the same version handshake rule the other integrations rely on (`docs/architecture/CLI.md` § `sure protocol`).

### 2. Expected event kinds

A Copilot adapter should produce the same lifecycle events the Claude Code and Cursor adapters already normalize. The mapping to SURE `event_type` values is:

| Copilot event | SURE `event_type` | Notes |
| --- | --- | --- |
| `session-start` | `session.started` | Mirrors `claude-code` `SessionStart` and `cursor` `sessionStart`. |
| `pre-tool-use` | `tool.requested` | Pre-action protection decision point. |
| `post-tool-use` | `tool.completed` | Successful tool result. |
| `post-tool-failure` | `tool.failed` | Optional; Cursor exposes this as `postToolUseFailure`. |
| `after-file-edit` | `file.edited` | Optional; Cursor exposes this as `afterFileEdit`. |
| `stop` | `session.stopped` | Mirrors `claude-code` `Stop` and `cursor` `stop`. |

These normalized names are the ones stored by `crates/sure-core/src/session_event_store.rs`. The raw Copilot event name should be preserved inside the payload under a `copilot_event` key, exactly as the Cursor normalizer keeps `cursor_event` and the Claude Code normalizer keeps `claude_event` (see `crates/sure-core/src/normalizer/cursor.rs` and `crates/sure-core/src/normalizer/claude_code.rs`).

### 3. Tool-name mapping to SURE `ActionKind`

For `pre-tool-use` events, SURE maps the harness tool name to a domain `ActionKind` in `crates/sure-core/src/hook_protection.rs`. A Copilot adapter must use the same mappings as Cursor, because Copilot's tool surface is closer to Cursor's than to Claude Code's:

| Copilot tool name | SURE `ActionKind` | Reason |
| --- | --- | --- |
| terminal / shell | `ArbitraryCommand` | Any shell command is unclassified execution. |
| `Read` / `readFile` | `ReadFile` | Reading a project file. |
| `Write` / `writeFile` / `Edit` / `applyEdit` | `WriteProjectFile` | Creating or modifying a project file. |
| `Delete` / `deleteFile` | `DeleteProjectFile` | Deleting a project file. |
| anything else | `ArbitraryCommand` | Unknown tools are treated as unclassified execution. |

The exact Rust function to add is analogous to `cursor_tool_to_action_kind` in `crates/sure-core/src/hook_protection.rs`:

```rust
fn copilot_tool_to_action_kind(tool: &str) -> ActionKind {
    match tool {
        "terminal" | "shell" => ActionKind::ArbitraryCommand,
        "Read" | "readFile" => ActionKind::ReadFile,
        "Write" | "writeFile" | "Edit" | "applyEdit" => ActionKind::WriteProjectFile,
        "Delete" | "deleteFile" => ActionKind::DeleteProjectFile,
        _ => ActionKind::ArbitraryCommand,
    }
}
```

This function must delegate to the existing `sure_domain::execution::decide`, not introduce a new rule engine.

### 4. Protection response contract

For `pre-tool-use`, SURE returns a JSON decision on stdout. The adapter must parse it and relay it to Copilot. The shape is defined by `sure_core::hook_protection::ProtectionDecision` in `crates/sure-core/src/hook_protection.rs`:

```json
{
  "decision": "allow" | "warn" | "block",
  "reason": "optional human-readable explanation"
}
```

Exit-code contract (from `crates/sure-cli/src/report.rs`):

- `0` — `allow` or `warn`: the operation may proceed.
- `1` — `block`: the operation should not proceed.
- `5` or other non-zero — SURE could not finish; the adapter must fail safely (see below).

The adapter must not add debug output to stdout, because stdout is the wire format Copilot reads. Diagnostics belong on stderr or in local SURE logs (`docs/architecture/EVENT_PROTOCOL.md` § StdIO contract).

## What the adapter must NOT do

1. **Do not duplicate the check engine.** `sure check`, `sure repair`, and `sure recheck` live in core. The adapter only invokes them.
2. **Do not invent evidence.** If Copilot does not expose an event, the adapter omits it. It must not synthesize a session, tool result, or user goal.
3. **Do not copy core-owned frozen wording.** Plain-language verdicts, capability-tier descriptions, and error messages are owned by SURE core (`docs/architecture/FROZEN_SEMANTICS.md`). The adapter should return SURE's output verbatim, not paraphrase it.
4. **Do not reimplement execution-mode semantics.** The adapter reads SURE's decision; it does not reinterpret `InspectOnly`, `HostConfirmed`, or `Container`.

## Mapping check / repair / recheck to SURE CLI commands

A Copilot command surface should delegate exactly as the Cursor and Claude Code command files do:

| User-facing Copilot command | SURE CLI command | Reference |
| --- | --- | --- |
| Check project | `sure check [PATH]` | `integrations/claude-code/commands/check.md`, `integrations/cursor/commands/check.md` |
| Apply repair and re-check | `sure repair [PATH]` then `sure recheck [PATH]` | `integrations/claude-code/commands/fix.md`, `integrations/cursor/commands/fix.md` |
| Get status | `sure doctor` or `sure history` | `integrations/claude-code/commands/status.md` |

The command prompt instructions must require the adapter to:

- Resolve `sure.exe` in the same order as the existing hooks: `$env:SURE_BIN`, then `sure` on PATH, then `%LOCALAPPDATA%\SURE\bin\sure.exe`.
- Stop and tell the user SURE is not installed if none of those resolve; do not fabricate a result.
- Present SURE's result faithfully, treating `unknown`, `skipped`, `error`, and `cannot_confirm` as non-pass states.
- For repair: load the repair contract from `sure repair`, implement only the required change while preserving listed behavior, run the acceptance checks, and then invoke `sure recheck`. Do not mark the finding resolved from the agent's own statement.

## Privacy and execution-mode semantics

Privacy settings (`privacy.full_recording` in `sure.yaml`) and execution mode (`execution.mode` in `sure.yaml`) are read by SURE core, not by the adapter. The adapter's only responsibility is to send events and return decisions. Full recording opt-in is handled by `crates/sure-core/src/full_recording.rs`; the adapter must not store raw transcript content itself.

## Capability-tier honesty

**This package claims no tier, because it observes nothing.** The tier a `sure check` reports is derived from the events SURE actually received, not from any field an adapter sets, and a package that answers no event contributes none: a project whose only harness is this template reports Snapshot, exactly as it would with no harness at all. `crates/sure-domain/src/capability.rs` defines the tiers and names no Copilot one, and `sure doctor` does not offer Copilot among the integrations it reports.

A future Copilot adapter — one that answers at least one event — would be **Observed (Tier 1)** until Copilot's hook contract guarantees that SURE's pre-action decision is enforced before the tool runs. This is the same tier assigned to Claude Code and Cursor:

- `crates/sure-core/src/hook_protection.rs` documents that Cursor and Claude Code are Observed because the manifest does not confirm the harness interprets the response.
- `crates/sure-domain/src/capability.rs` defines the tiers: `0` snapshot, `1` observed, `2` protected.
- The event envelope must set `"capability_tier": 1`.
- The tier a `sure check` reports is derived from the events SURE actually
  received, not from this field: a project whose store holds a session's events
  reports Observed, an envelope claiming Protected never raises it, and a project
  with no recorded events still reports Snapshot. The envelope's field is what
  the adapter says about itself; the report is what SURE can prove.

Claiming Protected (Tier 2) without an enforceable pre-action hook would be a false green: SURE would appear to block dangerous actions while Copilot might still run them. Claiming Observed for this package today would be the same false green in a smaller size — a tier for an adapter that answers nothing.

## Fail-safe behavior

**This section is a requirement on the future adapter, not a description of this package.** A Copilot adapter must fail open — it must not block the agent and must not fabricate evidence — when `sure.exe` is missing or when it cannot communicate with SURE. The existing PowerShell launchers show the pattern in `integrations/claude-code/scripts/sure-hook.ps1` and `integrations/cursor/scripts/sure-hook.ps1`, and the snippet below is the shape a future adapter's launcher should follow:

```powershell
if (-not $bin) {
    @{acknowledged=$false; reason='SURE binary not found'; decision='allow'; source='copilot'} | ConvertTo-Json -Compress
    exit 0
}
```

The decision is `allow` so the user's agent keeps working; the `acknowledged=$false` and `reason` make it clear that SURE did not see the event. The launcher beside this file writes that note to **stderr** and leaves stdout empty (measured: exit 0, nothing on stdout, the note on stderr), so a harness reading stdout sees no decision at all rather than one SURE did not make.

**The shipped template cannot keep this requirement as a whole, and the launcher is not what breaks it.** `integrations/copilot/scripts/sure-hook.ps1` keeps the missing-binary half and relays SURE's status and both streams unchanged otherwise — but `sure hook ingest --source copilot` exits 5 for every event, because there is no Copilot normaliser, and Copilot's reference makes a non-zero `preToolUse` exit deny the tool call. A fail-open requirement and a fail-closed harness rule cannot both hold for the same exit status, which is why the manifest says not to load it until `--source copilot` answers (see [Next steps](#next-steps-to-make-this-real)).

## File references

| File | Why it matters for Copilot |
| --- | --- |
| `docs/architecture/EVENT_PROTOCOL.md` | Wire envelope contract. |
| `docs/architecture/REPAIR_PROTOCOL.md` | Repair contract shape. |
| `docs/architecture/CLI.md` | `sure check`, `sure repair`, `sure recheck`, `sure hook ingest`, `sure protocol`. |
| `docs/architecture/FROZEN_SEMANTICS.md` | Wording owned by core; adapter must not copy or paraphrase. |
| `crates/sure-protocol/src/event.rs` | `EventEnvelope` schema and validation. |
| `crates/sure-core/src/hook_protection.rs` | Protection decision and tool-to-`ActionKind` mapping. |
| `crates/sure-core/src/harness_event.rs` | Event ingestion policy. |
| `crates/sure-core/src/full_recording.rs` | Full recording opt-in and retention. |
| `crates/sure-core/src/session_event_store.rs` | Persisted session/event storage. |
| `crates/sure-core/src/normalizer/cursor.rs` | Model for the Copilot normalizer. |
| `crates/sure-cli/src/hook.rs` | `sure hook ingest` implementation. |
| `crates/sure-cli/src/report.rs` | Exit codes and response frame shape. |
| `integrations/claude-code/hooks/hooks.json` | Example hook manifest. |
| `integrations/cursor/hooks/hooks.json` | Example hook manifest. |
| `integrations/claude-code/scripts/sure-hook.ps1` | Windows launcher pattern. |
| `integrations/cursor/scripts/sure-hook.ps1` | Windows launcher pattern. |

## Next steps to make this real

None of these is done, and the package must not be installed or loaded until the first two are. `sure hook ingest --source copilot` exits 5 today for every event in the manifest, and the fail-closed `preToolUse` rule is what a user would feel.

1. Add a `copilot` normalizer module under `crates/sure-core/src/normalizer/` following `cursor.rs`/`claude_code.rs`.
2. Wire `--source copilot` into `crates/sure-cli/src/hook.rs` and add a `decide_copilot_tool` function in `crates/sure-core/src/hook_protection.rs`.
3. Add fixtures under `integrations/copilot/fixtures/` mirroring the Cursor fixtures.
4. Replace the placeholder `hooks/hooks.json` below with a manifest validated against Copilot's actual hook contract. Two of the six event names it wires today — `afterFileEdit` and `stop` — are not names the reference defines (`agentStop` is what it calls a stop, and `afterFileEdit` is not in it), and a manifest is only the first of the two halves: the source has to answer before the event names matter.

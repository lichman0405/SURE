# SURE + Codex

A thin integration for OpenAI Codex (CLI, IDE extension and desktop app, which
share one configuration surface). It teaches Codex to invoke the local SURE
engine and to treat a SURE repair contract as the acceptance contract.

Everything here is a manifest, a prompt or a config template. There is no
checking logic in this package: every verdict comes from the local Rust core
(ADR 0004).

## What is in this package

| Path | Codex surface | What it does |
| --- | --- | --- |
| `skills/sure-check/SKILL.md` | Skills (current) | Runs `sure check` and reports SURE's statuses without softening them. |
| `skills/sure-fix/SKILL.md` | Skills (current) | Runs `sure repair`, applies the contract, then runs `sure recheck`. |
| `prompts/check.md` | Custom prompts (deprecated upstream) | The same check flow for users who still invoke `/prompts:check`. |
| `prompts/fix.md` | Custom prompts (deprecated upstream) | The same repair flow for `/prompts:fix`. |
| `mcp.toml` | MCP config (`~/.codex/config.toml`) | Declares `sure mcp serve` as an MCP server so Codex can call the core directly. |

The Agent Plugin package under `integrations/agent-plugin/` remains the portable
baseline that `docs/integrations/CODEX.md` names. This package is the
Codex-native surface on top of it.

## Capability tier

**This package achieves and documents Tier 0 — snapshot.**

It is explicit-invocation only. The user or Codex runs `sure check`, and SURE
sees the project on disk. Nothing in this package forwards Codex session events
to SURE, so SURE cannot say what Codex did while it worked. Tier 1 is not
claimed here.

That is a statement about this package, not about Codex. `docs/integrations/CODEX.md`
sets Tier 1 as the v0.1 target for Codex, and the public Codex surface does now
expose the events a Tier 1 evidence bridge needs (see "Verified surface"
below). Delivering that bridge is P12-T007; until it ships, Tier 0 is the honest
description of what is installed here.

**Tier 2 is deliberately not claimed.** Codex's public `PreToolUse` hook does
document a real pre-action decision — `permissionDecision: "deny"`, or exit code
`2` with a reason on stderr — so the platform can in principle carry one. That is
not the same as SURE having demonstrated it. This package ships no hook; no such
hook has been run against Codex from this repository; the official docs describe
tool hooks as "a guardrail, not a complete enforcement boundary"; and non-managed
hooks are skipped until the user reviews and trusts them by hash. A Tier 2 claim
needs a shipped, exercised `PreToolUse` hook and a recorded run behind it. Until
then, claiming it would be a false green.

## Install (Windows, no administrator rights)

Everything installs into your own user profile. No elevated shell is required.

### 1. Get `sure.exe` onto the resolution path

The integration, the prompts and the MCP template all resolve the SURE binary in
this order:

1. `$env:SURE_BIN` — explicit override.
2. `sure` on PATH (via `Get-Command sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` — per-user install location.

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

### 3. Or install the custom prompts

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

### 4. Register the MCP server (optional)

Append the block in `mcp.toml` to `%USERPROFILE%\.codex\config.toml`:

```powershell
Get-Content "integrations\codex\mcp.toml" -Raw | Add-Content "$HOME\.codex\config.toml"
```

Or use the Codex CLI, which writes the same entry:

```powershell
codex mcp add sure -- sure mcp serve
```

`sure mcp serve` is the stdio MCP bridge described in
`docs/architecture/MCP_BRIDGE.md`. It is the same surface the portable Agent
Plugin declares in `integrations/agent-plugin/mcp.json`. Full MCP wiring and
tool-surface work is P12-T010; this package only supplies the template.

### Uninstall

Delete what you copied. Nothing else is touched and nothing is installed
machine-wide.

```powershell
Remove-Item -Recurse -Force (Join-Path $HOME ".agents\skills\sure-check"), (Join-Path $HOME ".agents\skills\sure-fix") -ErrorAction SilentlyContinue
Remove-Item -Force (Join-Path $HOME ".codex\prompts\check.md"), (Join-Path $HOME ".codex\prompts\fix.md") -ErrorAction SilentlyContinue
```

## What this package deliberately does not do

- **No checking logic.** No file here decides whether a project passes. The
  integration tree is checked for this in
  `crates/sure-testkit/tests/integration_thinness.rs`.
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
| Hooks are read from `~/.codex/hooks.json` or `[hooks]` in `config.toml`; events include `SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`, `SessionEnd`; `PreToolUse` can deny with `permissionDecision: "deny"` or exit code `2`; non-managed hooks require review and trust, tracked by hook hash. | `learn.chatgpt.com/docs/hooks.md` |

`AGENTS.md` is a verified surface that this package does not ship a file for. A
team that wants SURE to be consulted without anyone typing a slash command can
add one line to its own `AGENTS.md`. That is the project's instruction file, not
this package's to write.

### Not verified

- No Codex binary was run from this repository to confirm any of the above
  end-to-end. Every claim in the table is from documentation, and the surface is
  version-dependent (Codex hooks became inline-`config.toml` capable in the
  v0.124 line). Re-check the pages before relying on exact key names.
- The `PreToolUse` deny path is documented, not exercised here. Nothing in this
  package depends on it, and no tier claim rests on it.
- Behaviour on non-Windows hosts was not tested. The paths in this README are
  written for Windows; the Codex-side locations (`$HOME/.agents/skills`,
  `~/.codex/config.toml`) are the documented user-scoped ones and should be
  identical elsewhere.

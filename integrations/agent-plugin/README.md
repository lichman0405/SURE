# SURE portable Agent Plugin

The portable baseline package: a plugin manifest, a skill, and the MCP server
declaration that a harness with an Agent Plugin loader can read. It is the
package `docs/integrations/CODEX.md` names as the portable surface; the
Codex-native package under `integrations/codex/` is the same integration with
Codex's own skills, prompts and hook events on top.

Everything here is a manifest, a skill, or a launcher. No file decides whether a
project passes: every verdict comes from the local Rust core (ADR 0004), and
`crates/sure-testkit/tests/integration_thinness.rs` checks that this package
stays thin.

## What is in this package

| Path | What it is |
| --- | --- |
| `plugin.json` | The plugin manifest: `name`, `description`, `version`, and the two files it points at (`mcpServers: "mcp.json"`, `skills: "skills/"`). |
| `mcp.json` | The MCP server declaration for the portable loader. |
| `skills/sure-check/SKILL.md` | Runs `sure check` and reports SURE's statuses without softening them. |
| `scripts/install.ps1` | Per-user install into `%LOCALAPPDATA%\agent-plugins\sure`. No administrator rights. |
| `scripts/uninstall.ps1` | Removes it, and says whether it was there. |

> **Not verified:** the `$schema` that `plugin.json` names,
> `https://agent-plugins.org/schemas/1.0.0/plugin.schema.json`, does not, as
> fetched, define an `mcpServers` key and closes its root with
> `additionalProperties: false`. Either the field name or the schema version is
> wrong for that surface, and this repository has no installed loader to test
> against. This is recorded rather than guessed at; the package is honest about
> being a portable template, not about being loaded somewhere.

## Installation (Windows)

No administrator rights are required. The plugin installs into your own user
profile:

```powershell
# From the repository root:
powershell -ExecutionPolicy Bypass -File integrations\agent-plugin\scripts\install.ps1
```

Or with an explicit SURE binary location:

```powershell
$env:SURE_BIN = "C:\Path\To\sure.exe"
powershell -ExecutionPolicy Bypass -File integrations\agent-plugin\scripts\install.ps1
```

The install directory can be overridden with `$env:AGENT_PLUGIN_DIR`; the
default is `%LOCALAPPDATA%\agent-plugins\sure`.

### Binary resolution

The installer resolves the SURE binary in the same order as the hook launchers
in the other packages:

1. `$env:SURE_BIN` — explicit override.
2. `sure` on `PATH` (via `Get-Command sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` — the per-user install location.

If none of the three is found the install stops before touching the plugin
directory: `Write-Error 'SURE not found. Install SURE or set SURE_BIN.'`, exit 1.

### Copy vs. symlink

As in the Cursor package: a symlink when Windows Developer Mode allows one, a
copy otherwise or with `-ForceCopy`. Copied files have any `{{SURE_BIN}}` or
`{{PLUGIN_ROOT}}` placeholders rendered to absolute paths.

## MCP server (`mcp.json`)

`mcp.json` declares one MCP server and names `sure` directly:

```json
{ "mcpServers": { "sure": { "command": "sure", "args": ["mcp", "serve"] } } }
```

**This package's documented requirement is that `sure` resolves on `PATH`**, the
same decision the Cursor package records and the Codex template makes in its own
comments. The alternative — launching through a resolver script bundled in the
package — needs a plugin-root variable that the loader substitutes, and this
repository has no installed loader for this format to verify one against (see
the schema note above). A manifest that names a placeholder nobody substitutes
is a server that never starts and never says why, so the requirement is stated
instead of guessed at.

Two ways to satisfy it:

1. Put `sure` on `PATH` (the per-user install directory
   `%LOCALAPPDATA%\SURE\bin` is the one to add).
2. Edit the installed `mcp.json` so the command names the binary by absolute
   path, for example
   `"command": "C:\\Users\\you\\AppData\\Local\\SURE\\bin\\sure.exe"`.

`install.ps1` prints exactly this when the binary it resolved is not the one
`PATH` would find, which is the case on the machine this package is developed
on: `sure` is not on `PATH` there at all.

If neither is done, no server starts, so there is no tool list and no verdict —
a visible failure rather than a fabricated one. The failure this package is
careful not to create is the opposite one: a server that starts with no binary
and answers an empty tool list, which reads as SURE having looked and found
nothing. SURE's own launcher, which fails closed with exit 3 and one paragraph on
stderr, is in `integrations/claude-code/scripts/sure-mcp.ps1`, in the package
whose manifest can name a script.

## What this package deliberately does not do

- **No checking logic.** Nothing here decides whether a project passes: the
  skill invokes `sure check` and reports what it says, including a refusal.
- **No fabricated result.** If SURE cannot be reached, the answer is a sentence
  saying so. `Cannot confirm` is a valid answer; a plausible-looking result is
  not.
- **No second engine.** `integrations/` may not contain `.rs`, `.dll`, `.so`,
  `.dylib`, `.a`, `.rlib` or `.exe`; the check is in
  `crates/sure-testkit/tests/integration_thinness.rs`.
- **No administrator rights.** Everything installs per user.

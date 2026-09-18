# SURE Cursor Plugin

SURE uses Cursor Plugin + hooks/commands rather than starting with a heavy VS Code/TypeScript extension.

The bootstrap hook template is Windows-first and invokes a thin PowerShell launcher. Cursor hooks communicate through stdio JSON. Before release, tasks must validate exact current Cursor event/input/output semantics and produce/render appropriate launchers for Windows plus secondary macOS/Linux packaging.

Cursor can also load Claude Code third-party hooks, but SURE keeps an explicit Cursor package so it can use native Cursor events/capability reporting.

If a capability truly requires an extension API, record an ADR and keep core checking logic in Rust.

## Installation (Windows)

No administrator rights are required. The plugin installs into your per-user Cursor data directory.

```powershell
# From the repository root:
powershell -ExecutionPolicy Bypass -File integrations\cursor\scripts\install.ps1
```

Or with an explicit SURE binary location:

```powershell
$env:SURE_BIN = "C:\Path\To\sure.exe"
powershell -ExecutionPolicy Bypass -File integrations\cursor\scripts\install.ps1
```

### Binary resolution

The install script resolves the SURE binary in the same order as the hook launcher:

1. `$env:SURE_BIN` — explicit override.
2. `sure` on PATH (via `Get-Command sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` — per-user install location.

If SURE is not found, the install fails safely before touching the Cursor directory.

### MCP server (`mcp.json`)

`mcp.json` declares one MCP server, and it names `sure` directly:

```json
{ "mcpServers": { "sure": { "command": "sure", "args": ["mcp", "serve"] } } }
```

**This package's documented requirement is that `sure` resolves on `PATH`** — it
does not launch through a resolver script. Nothing here has been run against an
installed Cursor, so what this repository cannot confirm is a Cursor plugin-root
variable that a manifest could name a bundled script through, and a manifest
that names a placeholder nobody substitutes is a server that never starts and
never says why. The requirement is stated instead of guessed at, and the piece
SURE can control is made explicit: the installer resolves the binary per user
and tells you when the manifest's own requirement does not hold.

Two ways to satisfy it:

1. Put `sure` on `PATH` (the per-user install directory
   `%LOCALAPPDATA%\SURE\bin` is the one to add).
2. Edit the installed `mcp.json` so the command names the binary by absolute
   path, for example
   `"command": "C:\\Users\\you\\AppData\\Local\\SURE\\bin\\sure.exe"`.

`install.ps1` prints exactly this when the binary it resolved is not the one
`PATH` would find — which is the case on the machine this package is developed
on, where `sure` is not on `PATH` at all.

If neither is done, the harness cannot start the server: Cursor reports a server
that failed to start (Cursor's own reporting of that was not observed here), and
no tool list and no verdict can come out of a server that never ran. That is a
visible failure and not a fabricated result — the failure mode this package is
careful to avoid is the opposite one, a server that starts without a binary and
answers an empty tool list. SURE's own launcher, which fails closed with exit 3
and one paragraph on stderr, is in `integrations/claude-code/scripts/sure-mcp.ps1`
for the harness whose manifest can name a script.

### Plugin directory

The script installs into the Cursor per-user plugin directory. The default is:

```
%APPDATA%\Cursor\plugins\sure
```

You can override this with `$env:CURSOR_PLUGIN_DIR` before running the script.

> **Assumption:** The default `%APPDATA%\Cursor\plugins` is chosen because it is the most plausible per-user location for Cursor's native plugin format (distinct from VS Code extensions). If Cursor documents a different user-plugin path, update the default and this note.

### Copy vs. symlink

By default, the script tries to detect whether Windows Developer Mode is enabled (which allows unprivileged symlinks). If so, it creates a symbolic link to the repository's `integrations/cursor/` directory. This is convenient for development because changes to the source are reflected immediately.

If Developer Mode is not detected, or if you pass `-ForceCopy`, the script copies the files instead. Copied files also have any `{{SURE_BIN}}` or `{{PLUGIN_ROOT}}` placeholders rendered to absolute paths.

```powershell
# Always copy, never symlink:
powershell -ExecutionPolicy Bypass -File integrations\cursor\scripts\install.ps1 -ForceCopy
```

## Uninstallation

```powershell
powershell -ExecutionPolicy Bypass -File integrations\cursor\scripts\uninstall.ps1
```

This removes the `sure` folder from the Cursor plugin directory. It reports whether the folder was present.

The uninstall script respects `$env:CURSOR_PLUGIN_DIR` if set; otherwise it uses the same `%APPDATA%\Cursor\plugins` default as the install script.

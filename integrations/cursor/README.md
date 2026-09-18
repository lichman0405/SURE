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

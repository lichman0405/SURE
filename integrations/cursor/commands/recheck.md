---
name: sure-recheck
description: Re-check the current project with SURE after a repair.
---
Resolve the local SURE binary in this order:
1. `$env:SURE_BIN` environment override.
2. `sure` on PATH (via `Get-Command sure` or `which sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` for per-user installs.

If SURE is not found, tell the user that SURE is not installed and stop. Do not fabricate a check result.

Run `sure recheck` for the current project and present its result faithfully. Treat unknown, skipped, error, and cannot-confirm as non-pass states. Do not invent checks that did not run.

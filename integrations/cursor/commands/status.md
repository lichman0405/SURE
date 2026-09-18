---
name: sure-status
description: Show SURE status/history for this project.
---
Resolve the local SURE binary in this order:
1. `$env:SURE_BIN` environment override.
2. `sure` on PATH (via `Get-Command sure` or `which sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` for per-user installs.

If SURE is not found, tell the user that SURE is not installed and stop. Do not fabricate status.

Run `sure history` for the current project and report what SURE has actually recorded. Distinguish missing integration evidence from clean status.

---
description: Check the current project with SURE and explain the result in plain language.
---
Resolve the local SURE binary in this order:
1. `$env:SURE_BIN` environment override.
2. `sure` on PATH (via `Get-Command sure` or `which sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` for per-user installs.

If SURE is not found, tell the user that SURE is not installed and stop. Suggest running `sure doctor` or following the installation instructions. Do not fabricate a check result.

Run `sure check` for the current project using its configured execution/privacy mode. Summarize SURE's result faithfully. Do not turn `unknown`, `skipped`, `error`, or `cannot_confirm` into a pass. Do not invent checks that did not run.

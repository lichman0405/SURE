---
description: Apply a selected SURE repair contract, then ask SURE to re-check it.
---
Resolve the local SURE binary in this order:
1. `$env:SURE_BIN` environment override.
2. `sure` on PATH (via `Get-Command sure` or `which sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` for per-user installs.

If SURE is not found, tell the user that SURE is not installed and stop. Suggest running `sure doctor` or following the installation instructions. Do not fabricate a repair result.

Run `sure repair` for the current project to obtain the repair contract. Load the contract and implement only the required repair while preserving listed behavior. Run the acceptance checks. Then invoke `sure recheck`. Do not mark the SURE finding resolved solely from your own statement.

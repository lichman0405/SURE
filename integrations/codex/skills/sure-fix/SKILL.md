---
name: sure-fix
description: Apply a SURE repair contract as the acceptance contract and close the loop with a SURE re-check. Use when the user asks to fix, repair, or resolve findings that SURE reported.
---

# Repair with SURE

Resolve the local SURE binary in this order:

1. `$env:SURE_BIN` environment override.
2. `sure` on PATH (via `Get-Command sure` or `which sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` for per-user installs.

If SURE is not found, tell the user that SURE is not installed and stop. Do not
fabricate a repair result.

Then:

1. Run `sure repair` for the current project to obtain the repair contract.
2. Treat that contract as the acceptance contract. Implement only the repair it
   requires, and preserve the behavior it lists as to be preserved. Do not widen
   the change because something else looked wrong.
3. Run the acceptance checks the contract names.
4. Run `sure recheck`.

Your own message saying the fix works is not evidence and does not close the
finding. A finding stays open until `sure recheck` reports it closed. If the
re-check returns `unknown`, `skipped`, `error` or `cannot_confirm` for the
finding, leave it open and say plainly that the repair is unconfirmed.

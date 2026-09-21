---
name: sure-check
description: Check the current project with the local SURE engine and report its result without softening it. Use when the user asks whether the project is done, working, ready, or should be verified before hand-off.
---

# Check a project with SURE

Resolve the local SURE binary in this order:

1. `$env:SURE_BIN` environment override.
2. `sure` on PATH (via `Get-Command sure` or `which sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` for per-user installs.

If SURE is not found, tell the user that SURE is not installed and stop.
Suggest `sure doctor` or the installation instructions. Do not fabricate a check
result, and do not describe what SURE would probably have said.

Then:

1. Run `sure check` for the current project using its configured
   execution/privacy mode.
2. Summarize SURE's result faithfully, in plain language.
3. Keep every status SURE reported. `unknown`, `skipped`, `error` and
   `cannot_confirm` are not passes and must not be presented, counted or
   summarized as one. Do not drop them from the summary either.
4. Do not invent checks SURE did not run, and do not claim SURE verified
   something it reported as not checked.

If you report anything SURE did not itself state, mark it as your own reading
rather than as SURE's result.

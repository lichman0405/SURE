# Claude Code live smoke-test procedure

This document describes an **optional, manual** smoke test for the SURE Claude
Code integration. It exercises the real hook path with an authenticated Claude
Code session. Results may be recorded as task evidence; this procedure is **not**
a CI gate.

## When to run

- After changing the hook manifest (`hooks/hooks.json`), launcher scripts
  (`scripts/sure-hook.ps1`, `scripts/sure-hook.sh`), or the ingest/protection
  path in `sure-cli`.
- Before marking a release that claims Claude Code compatibility.
- Whenever you want confidence that the installed plugin still talks to the
  local SURE core correctly.

## Prerequisites

1. A Windows machine with Claude Code installed.
2. `sure.exe` installed and resolvable by one of:
   - `$env:SURE_BIN`
   - `sure` on `PATH`
   - `%LOCALAPPDATA%\SURE\bin\sure.exe`
3. The SURE Claude Code plugin installed in Claude Code's plugin directory.
4. A throw-away project directory for the test, e.g.
   `%USERPROFILE%\sure-smoke-test-project`.
5. `sure.yaml` in the test project configured for `InspectOnly` mode (the
   default for a new project is acceptable).

## Test steps

### 1. Prepare the test project

```powershell
$testRoot = "$env:USERPROFILE\sure-smoke-test-project"
New-Item -ItemType Directory -Force -Path $testRoot | Out-Null
Set-Location $testRoot

# Create a minimal project file so SURE recognizes the directory.
"# SURE smoke test project" | Set-Content -Path README.md

# Initialize SURE configuration if it does not exist.
if (-not (Test-Path sure.yaml)) {
    & sure init
}
```

Confirm `sure.yaml` contains:

```yaml
execution:
  mode: InspectOnly
```

### 2. Open the project in Claude Code with the SURE plugin

Start a Claude Code session in `$testRoot` with the SURE plugin enabled. The
`SessionStart` hook should run automatically.

Expected result: Claude Code starts normally. The hook output is not shown to
the user unless it fails.

### 3. Verify the session-start event was recorded

In another terminal, run:

```powershell
Set-Location $testRoot
sure history
```

`sure history` lists the sessions SURE has recorded on this machine — not only
this project's — most recently recorded first, with each session's `session` id.
Take the id from the top row and run:

```powershell
sure history show <SURE_SESSION_ID>
```

(The steps below write `sure history show <SURE_SESSION_ID>` for short: it reads
the events of the session that list named. `sure history --latest` was written
into this document before the command existed and is not a flag this build has.)

Expected result: a `session-start` event appears with the current session id.
If no event appears, the hook did not run or `sure.exe` could not be resolved.

### 4. Trigger a read-only tool

In Claude Code, ask:

```
Read README.md
```

Expected result: the `Read` succeeds. The `PreToolUse` hook should return
`allow` for a read in `InspectOnly` mode.

Verify with:

```powershell
sure history show <SURE_SESSION_ID>
```

Expected result: a `pre-tool-use` event for `Read` and a `post-tool-use` event
are recorded. The `pre-tool-use` decision is `allow`.

### 5. Trigger a blocked write

In Claude Code, ask:

```
Write a short file named smoke.txt with the content "test"
```

Expected result for the default `InspectOnly` configuration: the write should
be blocked by the `PreToolUse` hook. Claude Code may report that the tool was
not allowed.

Verify with:

```powershell
sure history show <SURE_SESSION_ID>
```

Expected result: a `pre-tool-use` event for `Write` is recorded with decision
`block` and a reason such as "The current execution mode does not permit this
action." No `post-tool-use` event should appear for the blocked write.

### 6. Trigger a blocked arbitrary command

In Claude Code, ask:

```
Run `whoami` in Bash
```

Expected result: the `Bash` request is blocked. Arbitrary commands always need
explicit consent, and the hook cannot obtain consent, so it fails closed.

Verify with:

```powershell
sure history show <SURE_SESSION_ID>
```

Expected result: a `pre-tool-use` event for `Bash` with decision `block`.

### 7. Run the SURE `check` command

In Claude Code, use the SURE `check` custom command (or run directly):

```powershell
sure check
```

Expected result: `sure check` completes. The result is summarized faithfully
by the custom command. Because the project is empty/healthy, the check should
report no findings.

### 8. End the session

Exit Claude Code. The `Stop` hook should run.

Verify with:

```powershell
sure history show <SURE_SESSION_ID>
```

Expected result: a `stop` event appears with the same session id as the
`session-start` event.

## Recording results

1. Copy the relevant output from `sure history show <SURE_SESSION_ID>` for each
   step, or the listing from `sure history` — a session that recorded nothing is
   a result too, and the command says which of the two it is.
2. Redact any absolute paths, user names, or repository-specific values.
3. Save the redacted transcript as task evidence or attach it to the
   `P10-T009` progress note.
4. Note the versions tested:
   - `sure --version`
   - Claude Code version
   - Plugin commit or package version

## Failure triage

| Symptom | Likely cause |
| --- | --- |
| No events in `sure history` | Hook not installed, `sure.exe` not resolved, or plugin not loaded. |
| `Read` is blocked | Configuration is not `InspectOnly`, or the protection adapter mapped `Read` incorrectly. |
| `Write` is allowed in `InspectOnly` | Hook response is not being honored by Claude Code (expected at Tier 1/Observed), or the mode is misconfigured. |
| Hook times out | Launcher script cannot find `sure.exe` or `sure hook ingest` is hanging. Check `$env:SURE_BIN` and PATH. |

## Scope and limitations

- This is a **manual** procedure. It is not run automatically in CI because it
  requires an authenticated Claude Code session.
- A passing smoke test does not imply Tier 2/Protected status. The integration
  remains **Observed (Tier 1)** until Claude Code's hook contract guarantees
  enforcement.
- Recorded results are evidence of compatibility on a specific configuration,
  not proof of correctness for all environments.

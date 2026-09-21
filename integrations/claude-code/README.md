# SURE Claude Code Plugin

Priority: first-class; target Tier 2 where current Claude Code hooks permit.

The tier a `sure check` reports for a project is the one its **recorded events**
earn, not this line and not the adapter's own claim: Tier 1 for a project whose
store holds a session's events, and Tier 0 for a project with none — and never
Tier 2 from events alone, because events prove what happened rather than that
SURE could have stopped it.

The bootstrap package is **Windows-first**. Its current hook template uses PowerShell, which Claude Code officially supports for Windows command hooks. Release tasks must validate the installed/current hook schema and generate/package appropriate launchers for secondary macOS/Linux support.

The hook is intentionally thin: it forwards stdin JSON to the local SURE core. Core checking/protection logic does not live in the script.

The integration must fail safely if `sure.exe` is not installed and must never invent evidence for a hook that did not run.

## Installation (Windows)

No administrator rights are required, and none are asked for: `scripts/install.ps1`
writes only inside your own user profile, and it creates no symlink it cannot
create.

```powershell
# From the repository root:
powershell -ExecutionPolicy Bypass -File integrations\claude-code\scripts\install.ps1
```

Or with an explicit SURE binary location:

```powershell
$env:SURE_BIN = "C:\Path\To\sure.exe"
powershell -ExecutionPolicy Bypass -File integrations\claude-code\scripts\install.ps1
```

To remove it:

```powershell
powershell -ExecutionPolicy Bypass -File integrations\claude-code\scripts\uninstall.ps1
```

`-ExecutionPolicy Bypass` there is the scoped, per-invocation choice of the
person typing the command — the same one the Cursor and agent-plugin READMEs
document — and not this package's position on policy: see
[Execution policy](#execution-policy) below for what a command this repository
starts on its own is allowed to do, and is not.

### Where it installs

| | |
| --- | --- |
| default | `%LOCALAPPDATA%\claude-plugins\sure` |
| override | set `$env:CLAUDE_PLUGIN_DIR` before running either script |

The installer resolves the SURE binary in the same order as the launchers above:
`$env:SURE_BIN`, then `sure` on `PATH`, then `%LOCALAPPDATA%\SURE\bin\sure.exe`.
If none of the three is found it stops before touching the plugin directory —
`Write-Error 'SURE not found. Install SURE or set SURE_BIN.'`, exit 1 — because a
package placed where nothing can start it, with no word said, is the failure
`docs/integrations/CLAUDE_CODE.md` asks this step to avoid.

### Copy vs. symlink

As in the Cursor and agent-plugin packages: a symlink when Windows Developer Mode
allows one, a copy otherwise or with `-ForceCopy`. Copied files have any
`{{SURE_BIN}}` or `{{PLUGIN_ROOT}}` placeholders rendered to absolute paths; this
package carries no such token today, and the render step has to leave
`${CLAUDE_PLUGIN_ROOT}` alone — it is Claude Code's substitution, not SURE's, and
a render keyed on the wrong string would produce a plugin that names a path
nobody substitutes. A test asserts that of the installed copy.

**The symlink is an optimisation of the development loop, never a requirement.**
Measured on 2026-09-21 on the machine this package is developed on, which has no
Developer Mode: `New-Item -ItemType SymbolicLink` answered
`UnauthorizedAccessException` — *this operation requires administrator
privileges* — and the installer completed anyway, exit 0, `Installed (copy)`.
That is acceptance line 2 of `P15-T009` as a run rather than a claim.

### Loading the plugin is Claude Code's step, and this repository did not watch it

`scripts/install.ps1` places the package. It does not register it with Claude
Code, and no plugin from this repository has been loaded into a running session,
so **every loading mechanism below is Claude Code's documented behaviour read
from its documentation on 2026-09-21 and is not something SURE observed**. The
line is the same one the MCP server section above draws.

- **Skills-directory plugin.** Documented: any folder under a skills directory
  that holds a `.claude-plugin/plugin.json` is loaded as a plugin named
  `<name>@skills-dir` on the next session, with no marketplace and no install
  step. This package already has that manifest, so
  `$env:CLAUDE_PLUGIN_DIR = "$env:USERPROFILE\.claude\skills"` puts it at
  `%USERPROFILE%\.claude\skills\sure` and Claude Code loads it. It is *not* the
  default, and that is a decision rather than an oversight: this package carries
  hooks, personal skills load in every project on the machine, and turning
  SURE's checks on for every session a person opens is a choice for that person
  to make rather than one for an installer to make quietly.
- **`claude plugin install <plugin>@<marketplace>`**, with
  `claude plugin uninstall` to remove it, and `--scope` to choose which settings
  file records it. Documented as the non-interactive CLI for plugin management.
  It installs *from a marketplace*, and this repository ships no marketplace
  file, so there is nothing here to install from yet; that is release-packaging
  work, not something this installer can stand in for.
- **`claude --plugin-dir <path>`** — documented as loading a plugin directory
  for that session only. Nothing is installed, so there is nothing to uninstall;
  point it at `%LOCALAPPDATA%\claude-plugins\sure` to try the package without
  installing it.

What was observed here is the half this repository owns: the copy on disk, at a
path the commands above can name. What was not observed is Claude Code reading
it, and this page does not claim otherwise.

## MCP server

`.mcp.json` declares one MCP server, and it does not name `sure` directly:

```json
{
  "mcpServers": {
    "sure": {
      "command": "powershell",
      "args": ["-NoProfile", "-File", "${CLAUDE_PLUGIN_ROOT}/scripts/sure-mcp.ps1"]
    }
  }
}
```

`${CLAUDE_PLUGIN_ROOT}` is Claude Code's own substitution, not SURE's. Checked
on **2026-09-19** at `code.claude.com/docs/en/plugins-reference`: for a plugin's
MCP stdio server the placeholders resolve in `command`, `args` and `env`, and
the page's own example puts `${CLAUDE_PLUGIN_ROOT}` in a `command` and in an
`args` entry, which is the shape this file uses. The placeholder is also
*exported* to MCP server subprocesses, so a launcher may read
`$env:CLAUDE_PLUGIN_ROOT` too — this one does not need to.

The command line in the manifest is written for the exec form the same page
prescribes for commands with paths in them ("use exec form with `args` so each
path is passed as one argument with no quoting"): `-File` and the script path
are separate `args` entries, so a plugin root containing a space or non-ASCII
characters is passed as one argument. Run the same way on 2026-09-19 from a
plugin root under `target/tmp/plugin root wîth space/`, with the manifest
rendered as Claude Code renders it (forward slashes), the launcher exited 0 and
wrote 3459 bytes of stdout, byte for byte what `sure mcp serve` wrote when run
directly. What was **not** observed here is Claude Code itself spawning the
server: this repository has no plugin installed into a running session, so that
step is Claude Code's documented behaviour rather than something SURE watched.

**This package launches through a resolver script on purpose.** `"command":
"sure"` starts a server only where `sure.exe` is on `PATH`, and this package
cannot assume that. It installs per user, it requires no administrator rights,
and the location it documents — `%LOCALAPPDATA%\SURE\bin\sure.exe` — is on no
`PATH` by itself. On the machine this package is developed on, `sure` is not on
`PATH` at all, so a manifest that named it would have shipped untested against
the only machine here that runs it. A plugin that claims Windows support and
cannot start its own server from the Start menu has not been tested on the
machine it was built on; this one resolves the binary at start time instead.

`scripts/sure-mcp.ps1` resolves the binary in the same order as
`scripts/sure-hook.ps1`:

1. `$env:SURE_BIN` — explicit override.
2. `sure` on `PATH` (via `Get-Command sure`).
3. `%LOCALAPPDATA%\SURE\bin\sure.exe` — the per-user install location.

Then it hands the session over unchanged: `& $bin mcp serve`, with the caller's
stdin, stdout and stderr passing straight through and the server's own exit
status returned. `crates/sure-cli/tests/mcp_protocol.rs::the_packaged_mcp_launcher_reaches_sure_and_hands_the_stream_over_untouched`
runs exactly what the manifest names, against this build's binary, and compares
the protocol stream it produces with a direct `sure mcp serve` run, byte for
byte.

### When SURE is not there: exit 3, and a message

The launcher **fails closed**, which is the opposite of what the hooks do. A
hook exits 0 with `{"acknowledged":false,"reason":"SURE binary not found",…}` on
stderr so that a missing local tool never blocks a session. A *server* that
started without a binary would answer `tools/list` with an empty list, and a
caller would read that as SURE having looked at the project and found nothing —
a fabricated result. So the launcher refuses to start a session at all: exit 3,
one paragraph on stderr naming `SURE_BIN`, `PATH` and
`%LOCALAPPDATA%\SURE\bin\sure.exe` and ending *"SURE has checked nothing, and this
session has no `sure` tools."*, and nothing on stdout — no protocol message and
no tool list. The harness reports a server that did not start, which is what
happened.

The exit codes are recorded in `fixtures/launcher/mcp-exit-codes.json`, and
`crates/sure-testkit/tests/integration_thinness.rs` runs the script with its
environment rearranged to check each refusal: no binary anywhere, a `SURE_BIN`
that names nothing, a stand-in that answers a status SURE does not serve, and a
path the operating system will not start at all.

### When the binary is not a build this package can talk to

A build older than the bridge has no `mcp` subcommand and answers one with the
command line's own usage error and status 2. The launcher asks
`sure mcp serve --help` before it hands the session over, and refuses with exit
3 and one paragraph naming the binary, the status it answered with, and the
revision this package needs — Model Context Protocol revision `2025-11-25`.
Without that probe the caller would meet a server that dies on its first
message. A path that exists but holds something the operating system will not
start has no status to quote, and the refusal says so rather than printing a
blank one: `the_claude_code_mcp_launcher_does_not_invent_a_status_it_was_not_given`.
The probe establishes that the command carries the subcommand and not that the
file is SURE; a `SURE_BIN` is your statement of which binary it is.

The other mismatch — a build that speaks MCP at a different revision — is not
detected by the launcher, and does not pretend to be: the handshake settles it.
Read at revision `2025-11-25` (`.../basic/lifecycle`), a server that does not
support the requested revision "**MUST** respond with another protocol version
that it supports". SURE supports one revision and always answers with it, so it
never agrees to a revision it does not speak, and the caller can see from the
`protocolVersion` in the result whether to go on. The statements and the tests
behind them are in `docs/architecture/MCP_BRIDGE.md`.

### Execution policy

The manifest runs `powershell -NoProfile -File …` and deliberately passes no
`-ExecutionPolicy Bypass`: this package does not override the machine's script
execution policy for a process the harness starts on its own. On a machine whose
policy forbids local scripts, the server fails visibly with the policy error on
stderr instead of the launcher quietly doing nothing. The same position is taken
for the hook launchers in `integrations/codex/README.md`.

## Platform

Windows-first, like the hook template in `hooks/hooks.json`, and for the same
reason: the template is what is tested here. `scripts/sure-hook.sh` is the POSIX
hook launcher for release packaging to pick up; the MCP entry point has no POSIX
launcher in this bootstrap package yet, and `hooks.json` says the same thing
about itself — release packaging must render and test per-platform launchers.
On a host without Windows PowerShell the MCP server declared here does not
start, and that is a gap in packaging rather than a claim that it works.

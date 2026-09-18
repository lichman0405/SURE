# MCP bridge

Purpose: portable active invocation, not passive surveillance.

Command:

```text
sure mcp serve
```

Transport: stdio for v0.1.

Candidate tools:
- `sure_check`
- `sure_get_report`
- `sure_get_repair`
- `sure_recheck`
- `sure_status`

Requirements:
- same execution/privacy rules as CLI;
- no hidden network listener;
- structured errors preserve unknown/skipped states;
- caller cannot use project-controlled arguments to bypass user-level execution/protection policy;
- protocol version handshake is explicit.

Hooks and MCP solve different problems:
- hooks: passive evidence + pre-action protection;
- MCP/CLI: explicit check/report/repair actions.

## What this build implements

`sure mcp serve` is implemented (P12-T009). It speaks Model Context Protocol
revision `2025-11-25` over newline-delimited JSON-RPC 2.0 on its own standard
input and output, and it answers four methods: `initialize`, `ping`,
`tools/list` and `tools/call`. A notification is never answered and is never
run, so a `tools/call` that arrives without an identifier does nothing. A
request that arrives before the handshake is refused with `-32002`, which is
the code the reference implementations use and not one the specification
assigns. `initialize` is answered with this build's revision whether the caller
asked for it or for something else, which is the negotiation rule with one
supported revision in it.

The process is launched by the caller and talks to nobody else. There is no
socket, no port and no listener anywhere in the bridge; `sure mcp serve` is a
child process holding two pipes.

### How a harness starts it, and what happens when it cannot

A harness starts the server from a manifest, and `"command": "sure"` works only
when `sure.exe` is on `PATH`. Whether a package may assume that is decided per
package and written down in that package's README, because the answer differs
with what the harness offers:

| Package | What its MCP manifest names | Why |
| --- | --- | --- |
| `integrations/claude-code` | `powershell -NoProfile -File <plugin root>/scripts/sure-mcp.ps1` | Claude Code substitutes `${CLAUDE_PLUGIN_ROOT}` in `command`, `args` and `env` for a plugin's MCP stdio server, so this package can name a script it bundles; the script then resolves the binary the way the hooks do. Checked 2026-09-19 at `code.claude.com/docs/en/plugins-reference`; `integrations/claude-code/README.md` carries the details and what was not observed. |
| `integrations/cursor` | `sure` (in `mcp.json`) | The installed manifest names `sure` and requires it on `PATH`, or the absolute path to `%LOCALAPPDATA%\SURE\bin\sure.exe` edited in by hand. `install.ps1` resolves the binary per user and prints which of the two applies, because a plugin installed per user cannot assume a `PATH` entry. |
| `integrations/agent-plugin` | `sure` (in `mcp.json`) | The same decision as Cursor, for the same reason, and stated in its README. |
| `integrations/codex` | `sure` (in a `config.toml` block the user pastes) | The user edits the file, so it can name either; the template says so in its own comments. |

The resolution order is the one every launcher and installer here uses:
`$env:SURE_BIN`, then `sure` on `PATH` (via `Get-Command sure`), then
`%LOCALAPPDATA%\SURE\bin\sure.exe`, the per-user install location. Nothing in
this path needs administrator rights, and nothing installs machine-wide.

#### A missing binary refuses; it does not answer

A server that starts without a binary is the worst of the available failures. It
would answer `tools/list` with an empty list, and a caller would read that as
SURE having looked at the project and having nothing to say. That is a
fabricated result, so the MCP path fails closed — the opposite of the hooks,
which exit 0 with `{"acknowledged":false,"reason":"SURE binary not found",…}` so
that a missing local tool never blocks a session.

What a caller sees instead, from the Claude Code launcher: **exit 3**, one
paragraph on stderr naming `SURE_BIN`, `PATH` and
`%LOCALAPPDATA%\SURE\bin\sure.exe`, and nothing at all on stdout — no tool list,
no protocol message. The harness reports a server that did not start, which is
what happened. `integrations/claude-code/fixtures/launcher/mcp-exit-codes.json`
records the two codes (0 for a session that ended, 3 for a launcher that
refused), and `crates/sure-testkit/tests/integration_thinness.rs` runs the
script with its environment rearranged to check each branch: no binary anywhere,
a `SURE_BIN` that names nothing, a stand-in that is not SURE, and a path the
operating system will not start at all.

#### An incompatible binary

Two mismatches, and they are not the same failure.

**A build that predates the bridge** has no `mcp` subcommand, and answers one
with the command line's own usage error and status 2 — the status this CLI
returns for a command line it does not have
(`crates/sure-cli/tests/cli_contract.rs`). The launcher asks
`sure mcp serve --help` before it hands the session over, so a caller gets one
paragraph naming the binary, the status it answered with, and the revision this
package needs, instead of a server that dies on the caller's first message. A
binary the operating system refuses to start is the same refusal with "could not
be run at all" in place of the status: there is no status to quote, and the
launcher does not print a blank one
(`the_claude_code_mcp_launcher_does_not_invent_a_status_it_was_not_given`). The
probe is checked with a stand-in that exits 2
(`the_claude_code_mcp_launcher_refuses_a_binary_that_does_not_carry_the_bridge`),
not against a real pre-bridge build: this tree does not contain one. What the
probe establishes is that the named command carries the subcommand, not that the
file is SURE — a different program that accepts `sure mcp serve --help` and exits
0 passes it, and `where.exe`, which ships with Windows, is one. A `SURE_BIN` is
the user's own statement of which binary this is.

**A build that speaks another revision** is not detected by the launcher, and it
does not pretend to be: the revision is settled by the handshake, which is what
the protocol provides for it. Read at revision `2025-11-25`,
`.../basic/lifecycle`: "If the server supports the requested protocol version,
it **MUST** respond with the same version. Otherwise, the server **MUST**
respond with another protocol version that it supports. This **SHOULD** be the
latest version supported by the server." SURE supports one revision and answers
with it, whatever the client asked for, so it never agrees to a revision it does
not speak: a caller that expects `2025-11-25` and reads a different
`protocolVersion` in the result knows it must stop, and a caller that ignores
the field is answered in this build's vocabulary. That is the behaviour
`a_caller_that_asks_for_another_revision_is_answered_with_sures_own`
(`crates/sure-cli/src/mcp.rs`) and
`a_caller_that_asks_for_another_revision_is_still_told_what_sure_speaks`
(`crates/sure-cli/tests/mcp_protocol.rs`) pin, and the revision the negotiation
was checked against, `2025-11-25`, is the one in `PROTOCOL_VERSION`.

### What each tool answers today

Every tool runs the command line behind it, through the same dispatch
(`Command::report`) that a person's command line goes through, and returns that
command's own report: its frame, unmodified, under `structuredContent.sure`,
and its own words, unmodified, as the text block. Nothing is re-checked,
re-worded or summarized in the bridge, so a tool cannot disagree with the CLI
about the same project.

| Tool | Runs | Answers today |
| --- | --- | --- |
| `sure_check` | `sure check [project]` | the check's own report: the twelve-stage record, what was checked, what was not and why, and the verdict |
| `sure_get_report` | `sure history list` | `sure history is not implemented in this build.`, as a tool error |
| `sure_get_repair` | `sure repair [project]` | the check's own report, carried on through the repair-contract stage |
| `sure_recheck` | `sure recheck [project]` | the check's own report, carried on through the re-check stage |
| `sure_status` | `sure doctor` | the diagnostic report, plus the MCP revision, the server name and version, and which commands this build implements |

A tool error is `"isError": true` in the result, and a refusal carries the same
status the command line would return (`exit_code` 3). `isError` says whether the
command answered at all, not what its answer was. Three of the five tools now
run real checks, and **a check that ran and found the project not clean is not a
tool error**: `sure_check` on such a project is `"isError": false` with
`outcome` `not_green` and `exit_code` 1, exactly as `sure doctor` finding
something wrong is `"isError": false`. A caller that took `isError` for the
verdict would read the transport as the result and miss the verdict sitting in
`structuredContent.sure.outcome`. There is no path in the bridge that turns
either "this build cannot do that" or "this project is not clean" into something
an agent reads as success.

The tool descriptions and the handshake instructions are derived from
`crate::commands::IMPLEMENTED`, so when a command starts working, the sentences
that say it does not stop appearing without anyone editing them. That happened in
`P7-T010`: three of the four refusal rows above became real answers, and no
description in this table had to be touched for it.

### Arguments

A tool call may carry a `project` and nothing else, and only on the tools whose
command takes a path. The schemas are closed (`additionalProperties: false`)
and any other key is refused with `-32602` and a sentence saying where the
policy comes from: execution mode, privacy settings and protection policy are
read from the user's own SURE configuration, and no argument can select or
override them. A project path is passed to the command unchanged, so whatever
the command line does with a relative, missing or unreadable path is what the
tool does with it, and the answer is the same refusal in the same words. There
is no argument that sets a goal, a depth, a mode or a target.

### Streams and limits

stdout carries protocol messages and nothing else, in every mode: the
specification forbids a server writing anything else there
(`.../2025-11-25/basic/transports`: "The server **MUST NOT** write anything to
its `stdout` that is not a valid MCP message"), and the bridge can only build a
protocol message, because every message goes out through the same `Report` →
`Format` path as every other command's output. The session summary is a
diagnostic and goes to stderr — under `--format json` as well as under the
default, since an envelope is not an MCP message and there is no flag that makes
one. Until P12-T010 the machine format put one envelope line on stdout after the
last message; `Format::emit` in `crates/sure-cli/src/output.rs` is where that
stopped, and
`crates/sure-cli/tests/mcp_protocol.rs::no_line_that_is_not_a_protocol_message_reaches_stdout_in_any_mode`
is what reads the process back and checks.

A message longer than 4 MiB is refused as a parse error and the rest of that
line is discarded, so the next line is read as a message rather than as the tail
of the last one. The specification sets no such limit; this one is SURE's. The
tool list is a single page: `tools/list` accepts no cursor and never sends a
`nextCursor`.

### What is not here yet

`resources/*`, `prompts/*` and `logging/*` are not implemented and are refused
as unknown methods. SURE sends no notifications and no requests of its own, so
a client has nothing to answer and `listChanged` is `false`. Nothing in the
bridge writes to a project.

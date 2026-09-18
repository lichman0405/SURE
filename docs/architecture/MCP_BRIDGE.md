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

### What each tool answers today

Every tool runs the command line behind it, through the same dispatch
(`Command::report`) that a person's command line goes through, and returns that
command's own report: its frame, unmodified, under `structuredContent.sure`,
and its own words, unmodified, as the text block. Nothing is re-checked,
re-worded or summarized in the bridge, so a tool cannot disagree with the CLI
about the same project.

| Tool | Runs | Answers today |
| --- | --- | --- |
| `sure_check` | `sure check [project]` | `sure check is not implemented in this build.`, as a tool error |
| `sure_get_report` | `sure history list` | `sure history is not implemented in this build.`, as a tool error |
| `sure_get_repair` | `sure repair [project]` | `sure repair is not implemented in this build.`, as a tool error |
| `sure_recheck` | `sure recheck [project]` | `sure recheck is not implemented in this build.`, as a tool error |
| `sure_status` | `sure doctor` | the diagnostic report, plus the MCP revision, the server name and version, and which commands this build implements |

A tool error is `"isError": true` in the result, and the refusal carries the
same status the command line would return (`exit_code` 3). `isError` says
whether the command answered at all, not what its answer was: `sure_status` is
`"isError": false` even when `sure doctor` finds something wrong, and the
verdict is in the frame, under `outcome` and `exit_code`, in the CLI's own
vocabulary. There is no path in the bridge that turns "this build cannot do
that" into something an agent reads as success.

The tool descriptions and the handshake instructions are derived from
`crate::commands::IMPLEMENTED`, so when `sure check` starts working, the
sentences that say it does not stop appearing without anyone editing them.

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

stdout carries protocol messages and nothing else: the specification forbids a
server writing anything else there, and the bridge can only build a protocol
message, because every message goes out through the same `Report` → `Format`
path as every other command's output. The session summary is a diagnostic and
goes to stderr with the default format. One wart, named here because a caller
has to know it: `sure --format json mcp serve` writes one CLI envelope line to
stdout after the last protocol message, because `--format json` means
"everything SURE says goes to stdout as one frame". A caller that reads stdout
as a protocol stream must not ask for the machine format; the harness
integrations do not.

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

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

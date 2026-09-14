# Architecture

```text
Claude Code plugin/hooks ─┐
Cursor Plugin/hooks ──────┼──── thin integration protocol ────┐
Codex Agent Plugin/skill ─┤                                   │
Other harness adapters ───┘                                   v
                                                       SURE local Rust core
                                                               │
        ┌───────────────┬───────────────┬──────────────────────┼──────────────┐
        v               v               v                      v              v
  Project discovery   Intent        Safe execution          Evidence      Reporting
        │               │               │                      │              │
        v               v               v                      v              v
   Project graph     Requirements   Checks/probes          Claims        Repair loop
```

## v0.1 runtime decision

No persistent daemon is required.

Hooks/plugins call short-lived `sure hook ...` / `sure check ...` processes and write to local storage through controlled locking/transactions. A short-lived stdio MCP server (`sure mcp serve`) is also allowed as a harness-neutral active tool surface; it is not a persistent background daemon.

A future daemon is allowed only if measured startup/contention/performance shows it is necessary.

This avoids:

- background-service installation complexity;
- Windows service/background-process requirements; macOS launchd equivalents;
- extra local network surface;
- a second lifecycle to debug during MVP.

## Core forms

CLI:

```text
sure check
sure recheck
sure repair
sure history
sure doctor
sure config
sure hook ingest
sure explain
```

Project-local config:

```text
sure.yaml
```

Project-local ephemeral metadata may use `.sure/`, which is gitignored by default.

User-level durable history uses OS-appropriate application data paths and is discoverable/deletable through the CLI.

## Active integration surface

CLI remains universal. Where a harness supports MCP, SURE may expose bounded tools such as:

- `sure_check`
- `sure_get_report`
- `sure_get_repair`
- `sure_recheck`
- `sure_status`

MCP tools must preserve the same evidence/permission semantics as the CLI. Hooks remain necessary for passive session evidence and pre-action protection.

# ADR 0003 — No persistent daemon in v0.1

Status: Accepted

## Decision

v0.1 uses short-lived local processes only.

- `sure check`, `sure doctor`, `sure hook ingest` and the other CLI commands are
  one-shot invocations that exit when done.
- The stdio MCP server (`sure mcp serve`) is a short-lived stdio process. There
  is no hidden network listener.
- There is no background service, no autostart entry, and no resident process
  holding project or session state.

Consequences that must be designed for rather than discovered:

- Short-lived hook processes may contend for the same local database. Busy,
  retry and transaction semantics must be deliberate and tested (P1-T005,
  P8-T011).
- Any startup cost is paid on every check, so startup performance is a real
  constraint, not a later optimisation.

A daemon is permitted later only if measured startup, contention or performance
evidence shows it is necessary.

## Frozen semantics

Config default `execution.mode: inspect_only` and the absence of any listening
socket mean a fresh install executes nothing and opens nothing.

## Revisit when

Measured evidence shows startup, contention or performance materially blocks the
core user job.

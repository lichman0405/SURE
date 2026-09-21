# ADR 0004 — Thin harness integrations

Status: Accepted

## Decision

Harness packages under `integrations/` contain manifests, hooks, commands and
skills. They contain no checking logic and no duplicated engine.

Three surfaces exist, and they solve different problems:

| Surface | Role |
| --- | --- |
| CLI | Universal. `sure check` works without any plugin installed. |
| stdio MCP | Explicit check, report, repair and re-check calls from a harness. |
| Hooks | Passive evidence capture and pre-action protection decisions. |

Every adapter reports the capability tier it actually achieved, and the blind
spots it has, through `sure_domain::capability::CapabilityReport`. An adapter
that cannot see the user's request says so instead of implying it could.

Hook launchers receive harness JSON on stdin and pass it to `sure hook ingest`.
Stdout must conform to the harness hook response contract and must contain no
debug noise; diagnostics go to stderr and local SURE logs.

## Frozen semantics

Tier definitions and blind-spot vocabulary: `docs/architecture/FROZEN_SEMANTICS.md`.

## Revisit when

A required capability demonstrably cannot be delivered through hooks, commands
or the MCP surface.

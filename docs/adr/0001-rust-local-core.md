# ADR 0001 — Rust local core

Status: Accepted

## Decision

Use Rust for the portable local checker/CLI core. Thin harness integrations call
the same core.

Crate boundaries are fixed by `Cargo.toml` and are acyclic:

| Crate | Responsibility |
| --- | --- |
| `sure-domain` | Frozen vocabulary and semantics. No storage, process, network or async dependency. |
| `sure-protocol` | Versioned wire types shared with integrations. |
| `sure-core` | Discovery, planning, checking, reporting engine. |
| `sure-testkit` | Deterministic fixtures and fake harnesses for tests. |
| `sure-cli` | The `sure` binary. |

Harness packages contain manifests, hooks, commands and skills. They never
contain a second copy of the check engine, and they reach the core only through
its CLI, its hook ingestion path or its stdio MCP surface.

## Frozen semantics

The vocabulary the engine implements is indexed in
`docs/architecture/FROZEN_SEMANTICS.md`. `sure-domain` deliberately has no
dependency that could tempt a semantics change into a plumbing change.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

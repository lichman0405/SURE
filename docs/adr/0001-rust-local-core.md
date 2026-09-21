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

## Allowed dependencies

`allowed` is an upper bound, not a target: a crate may depend on less than it is
permitted to. Anything not listed is a boundary violation.

| Crate | May depend on |
| --- | --- |
| `sure-domain` | nothing inside the workspace |
| `sure-protocol` | `sure-domain` |
| `sure-core` | `sure-domain`, `sure-protocol` |
| `sure-testkit` | `sure-domain`, `sure-protocol` |
| `sure-cli` | `sure-core`, `sure-protocol`, `sure-domain` |

Two of those rows are load-bearing rather than stylistic:

- `sure-domain` must stay a leaf. If the frozen vocabulary could reach the
  engine or storage, a plumbing change could alter a stored meaning without
  anyone deciding to (ADR 0010).
- `sure-testkit` must never appear as another crate's normal dependency, or
  fixtures would ship inside the product.

Cargo permits a cycle through `dev-dependencies`, so acyclicity is a property of
normal edges only.

## Enforcement

The boundary is not a document that people are asked to remember. It is checked:

- `sure_testkit::workspace::BoundaryPolicy` holds the table above.
- `crates/sure-testkit/tests/repository_shape.rs` parses the real manifests,
  builds the real graph, and fails on any disallowed edge, any member missing
  from the policy, and any cycle of normal edges.
- `crates/sure-testkit/tests/integration_thinness.rs` holds ADR 0004's "no
  duplicated engine" rule in place: launchers must reach the core, must stay
  within the launcher size bound, must not restate core-owned wording, and hook
  manifests must not reference a script that is not there.

## Frozen semantics

The vocabulary the engine implements is indexed in
`docs/architecture/FROZEN_SEMANTICS.md`. `sure-domain` deliberately has no
dependency that could tempt a semantics change into a plumbing change.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

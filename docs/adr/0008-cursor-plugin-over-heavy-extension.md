# ADR 0008 — Cursor Plugin first

Status: Accepted

## Decision

Use current Cursor Plugin/hooks packaging. Add a TypeScript extension only if required by a concrete missing capability.

## Frozen semantics

Packaging choice only; it fixes no domain rule. It is bounded by the frozen
capability-tier vocabulary in `docs/architecture/FROZEN_SEMANTICS.md`: whichever
surface ships, the adapter must report the tier it actually reached and the
blind spots it has, and must not imply a capability the platform does not grant.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

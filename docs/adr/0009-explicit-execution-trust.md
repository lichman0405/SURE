# ADR 0009 — Explicit execution trust

Status: Accepted

## Decision

Running AI-built code is itself risky; dynamic checks require an explicit execution mode.

Three modes exist, and each declares its own baseline:

| Mode | Meaning |
| --- | --- |
| `inspect_only` | Read files and metadata. Execute nothing. The default. |
| `host_confirmed` | Execute in the project, on the host, only with recorded per-command consent. |
| `container` | Execute inside an isolated environment. |

Consent is not one switch but six independent permissions, and granting one
never grants another (`ExecutionPermissions::allows`). Mode is not a substitute
for consent: `container` is not a blank cheque, and an arbitrary command always
requires its own approval in every mode
(`execution::decide` → `ExecutionDecision::NeedsConsent`).

Authority is layered so a project cannot escalate itself: project configuration
is a request, and `ConsentGrantor::can_grant` is false for
`ProjectRequestEscalated`. An approval made after the fact is recorded as such
rather than presented as pre-authorisation.

## Frozen semantics

Permissions, decisions and grantors: `docs/architecture/FROZEN_SEMANTICS.md`.
Enforced by `crates/sure-domain/src/execution.rs`.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

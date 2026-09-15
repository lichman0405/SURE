# ADR 0009 — Explicit execution trust

Status: Accepted

## Decision

Running AI-built code is itself risky; dynamic checks require an explicit execution mode.

Three modes exist, and each declares its own baseline:

| Mode | Meaning |
| --- | --- |
| `inspect_only` | Read files and metadata. Execute nothing. The default. |
| `host_confirmed` | Execute in the project, on the host, only with recorded per-command consent. |
| `container` | Execute in a container, which is limited isolation rather than a sandbox. |

Consent is not one switch but six independent permissions, and granting one
never grants another (`ExecutionPermissions::allows`). Mode is not a substitute
for consent: `container` is not a blank cheque, and an arbitrary command always
requires its own approval in every mode
(`execution::decide` → `ExecutionDecision::NeedsConsent`).

Authority is layered so a project cannot escalate itself: project configuration
is a request, and `ConsentGrantor::can_grant` is false for
`ProjectRequestEscalated`. An approval made after the fact is recorded as such
rather than presented as pre-authorisation.

## Wording correction (`P3-T008`)

The `container` row above used to describe the mode with one word and stop. The
word was *isolated*; it is the industry's default for this, it is not what this
product establishes, and `P3-T008` took it out of that row and out of two
sentences in `sure_domain::execution` and one in
`docs/architecture/EXECUTION_SAFETY.md`. **The old wording is named rather than
quoted**, which is a rule and not a style: the check that holds this fails on a
reproduction of the overclaim whether or not the sentence around it is correcting
it, because a rule that read intent could be argued with.

The decision is unchanged. The correction is recorded rather than silently edited
because an ADR is a dated record — a reader comparing this file with an older copy
is entitled to know which line moved and why. Tested by
`crates/sure-core/tests/container_isolation_claim.rs`.

## Frozen semantics

Permissions, decisions and grantors: `docs/architecture/FROZEN_SEMANTICS.md`.
Enforced by `crates/sure-domain/src/execution.rs`.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

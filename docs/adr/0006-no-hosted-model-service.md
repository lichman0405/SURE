# ADR 0006 — No hosted model service

Status: Accepted

## Decision

v0.1 uses disabled/local/user-owned providers; SURE is not an LLM reseller.

`analysis.provider` defaults to `disabled`. When no model is configured, SURE
must still be useful: deterministic checks, project inspection and the verdict
all work without one. A missing model degrades the *commentary*, never the
verdict.

No model output can be promoted into a deterministic fact. Model assessment is a
distinct evidence class with a truth rank below observed fact and deterministic
check, and it cannot by itself carry a blocking `must_fix` finding
(`EvidenceClass::can_alone_support_must_fix`).

If a provider is configured, disclosure is explicit: what leaves the machine,
to whom, under which privacy mode, and the local-only alternative are all stated
before anything is sent.

## Frozen semantics

Evidence ranks: `docs/architecture/FROZEN_SEMANTICS.md`. Privacy modes:
`docs/security/PRIVACY.md`, `docs/architecture/PRIVACY_AND_MODEL_STRATEGY.md`.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

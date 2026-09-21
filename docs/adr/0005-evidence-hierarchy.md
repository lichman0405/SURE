# ADR 0005 — Evidence hierarchy

Status: Accepted

## Decision

Runtime/current-state/deterministic evidence outranks model assessment, inference
and agent self-report.

Every claim assessment carries an evidence class with an explicit truth rank, and
a claim with no fresh evidence for the code being judged is reported as
`Cannot confirm`, never as confirmed:

| Class | May alone support a `must_fix`? |
| --- | --- |
| `observed_fact` | yes |
| `deterministic_check` | yes |
| `model_assessment` | no |
| `inference` | no |
| `unknown` | no, it is not evidence |

An important claim needs a location and a locator, not just prose
(`EvidenceAnchor::is_checkable`). Evidence that was collected against an earlier
fingerprint of the code is stale, and staleness is named in plain language
rather than silently dropped.

## Frozen semantics

Rank table, anchor rules and staleness wording:
`docs/architecture/FROZEN_SEMANTICS.md`. Enforced by
`crates/sure-domain/src/evidence.rs`
(`EvidenceClass::can_alone_support_must_fix`, `ClaimAssessment::label`).

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

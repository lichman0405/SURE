# ADR 0010 — Frozen domain semantics live in code, not only in prose

Status: Accepted

Date: 2026-09-14

## Context

P0 requires the core vocabulary, check statuses, aggregation rules, intent
trust, execution trust and capability tiers to be *frozen* before any check
engine is built on top of them. The product documents state these rules in
prose. Prose does not fail a build.

Every rule in this area is one that a later, well-meaning implementation change
could quietly weaken: a "probably fine" seventh status, an inference promoted to
a requirement because the pattern looked convincing, a denied execution consent
that stops appearing in the summary, or a capability tier claimed by an adapter
that cannot actually deliver it. Each of those produces a false green, which
`CLAUDE.md` defines as more serious than a visible error.

## Decision

Encode the frozen semantics in `crates/sure-domain` as types whose construction
makes the wrong answer impossible or visible, and cover them with tests that
assert the *rule*, not just the happy path.

Specifically:

1. `status::aggregate` is the only aggregation entry point, and no input makes
   it return green when a critical check failed, errored, is unknown, or was
   skipped for a reason other than an honest scope limit. An empty plan is
   `not_enough_checked`, not green.
2. `intent::may_claim_full_fulfilment` is the only gate for claiming the user's
   requirements were met. `IntentSource::is_user_requirement` is false for
   `Inferred` and `AgentClaim`, so an inference can never become a requirement.
3. The after-the-fact limitation sentence is a `const`, not a style guideline,
   so reports cannot paraphrase it into a weaker claim.
4. `execution` models six independent permissions rather than one switch.
   Granting one never grants another, and an arbitrary command always needs its
   own approval regardless of mode.
5. `capability::CapabilityReport::is_self_consistent` rejects a tier claim that
   the adapter's own pre-action-control flag contradicts.
6. `evidence::EvidenceClass::can_alone_support_must_fix` is false for
   `ModelAssessment` and `Inference`, so model opinion cannot carry a blocking
   finding by itself.
7. `docs/architecture/FROZEN_SEMANTICS.md` indexes every rule to the code that
   enforces it and records known conformance gaps.
8. Every wire name is pinned by a literal in
   `crates/sure-domain/tests/wire_contract.rs`, whose matches have no wildcard
   arm. A variant cannot be added without deciding its wire name, and the six
   enums that also appear in a JSON schema are compared against that schema, so
   the two statements of the contract cannot drift apart.

`DOMAIN_SEMANTICS_VERSION` marks the meaning of stored records. Changing any of
the above in a way that reinterprets an existing stored verdict, finding or
evidence record requires bumping it and recording the reason.

## Consequences

- The wrong answers are still *expressible* — a caller can construct a
  `Finding` by hand — but they are testably wrong rather than merely discouraged.
- `sure-domain` stays free of storage, process and integration dependencies, so
  the semantics cannot be changed as a side effect of a plumbing change.
- The test suite asserts rules that are easy to break, so a regression shows up
  as a failing test rather than a false green in a user's report.
- Two conformance gaps between the prose documents and the JSON schemas were
  found while writing this down and are recorded in
  `docs/architecture/FROZEN_SEMANTICS.md` rather than silently resolved.

## Revisit when

A future edition needs a status, severity or tier that cannot be expressed
within this vocabulary. Adding one is a deliberate act with an ADR, not a patch.

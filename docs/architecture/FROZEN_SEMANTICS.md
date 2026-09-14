# Frozen semantics

This document is the authoritative index of what SURE's words mean, and where
each meaning is enforced in code. It is written for implementers. User-facing
language rules live in `docs/product/UX_AND_LANGUAGE.md`.

The rule this document exists to support: **the meaning of a stored verdict,
finding or evidence record must not change by accident.** When a meaning changes
on purpose, bump `sure_domain::DOMAIN_SEMANTICS_VERSION` and record why.

## Related decisions

The ADRs in `docs/adr/` record the *decisions*; this document records what those
decisions *mean* and where that meaning is enforced.

| ADR | Topic |
| --- | --- |
| 0001 | Rust local core and crate boundaries |
| 0002 | Local-first privacy |
| 0003 | No persistent daemon in v0.1 |
| 0004 | Thin harness integrations |
| 0005 | Evidence hierarchy |
| 0006 | No hosted model service |
| 0007 | Windows primary development |
| 0008 | Cursor Plugin before a heavy extension |
| 0009 | Explicit execution trust |
| 0010 | Frozen domain semantics live in code |
| 0011 | Project configuration is a request, not a grant |
| 0012 | Diagnostics are records, not a logging framework |

Each of those ADRs carries a "Frozen semantics" section pointing back here.

## Where each decision is enforced

| Decision | Frozen in | Enforced by |
| --- | --- | --- |
| Core domain entities | `docs/architecture/DOMAIN_MODEL.md` | `crates/sure-domain/src/vocabulary.rs` |
| Entity identity format | this document | `crates/sure-domain/src/ids.rs` |
| Every wire name | this document, `schemas/` | `crates/sure-domain/tests/wire_contract.rs` |
| Every document's structure | this document, `schemas/` | `crates/sure-protocol/tests/conformance.rs` |
| The event format version | this document | `sure_protocol::PROTOCOL_VERSION` |
| Check statuses | `schemas/check-result.schema.json` | `crates/sure-domain/src/status.rs` |
| Aggregation and the false-green rule | `MASTER_PROMPT.md` §9 | `sure_domain::status::aggregate` |
| Severity levels | `docs/product/UX_AND_LANGUAGE.md` | `crates/sure-domain/src/severity.rs` |
| Evidence classes and truth order | `docs/architecture/EVIDENCE_MODEL.md` | `crates/sure-domain/src/evidence.rs` |
| A check result declares its evidence class and its project state | `docs/architecture/EVIDENCE_MODEL.md`, `DOMAIN_MODEL.md` | `sure_domain::status::CheckResult` |
| Claim assessments | `schemas/claim.schema.json` | `sure_domain::evidence::ClaimAssessment` |
| Intent trust | `docs/architecture/PROJECT_INTENT.md` | `crates/sure-domain/src/intent.rs` |
| After-the-fact limitation | `docs/product/UX_AND_LANGUAGE.md` | `sure_domain::status::NO_TRUSTED_INTENT_LIMITATION` |
| Execution modes and permissions | `docs/architecture/EXECUTION_SAFETY.md` | `crates/sure-domain/src/execution.rs` |
| Capability tiers | `MASTER_PROMPT.md` §6 | `crates/sure-domain/src/capability.rs` |
| Repair contract fields | `docs/architecture/REPAIR_PROTOCOL.md` | `sure_domain::vocabulary::RepairContract` |
| Project settings surface | `docs/architecture/CONFIG_REFERENCE.md` | `crates/sure-core/src/config/` |
| Project settings are inert requests | `docs/architecture/CONFIG_AUTHORITY.md` | `sure_core::config::Config::requested_privileges` |
| Project goal trust | `docs/architecture/PROJECT_INTENT.md` | `sure_core::config::Config::goal_source` |
| Storage locations | `docs/architecture/STORAGE_AND_DATA_PATHS.md` | `crates/sure-core/src/paths/mod.rs` |
| Authoritative evidence is outside the project | `docs/architecture/STORAGE_AND_DATA_PATHS.md` | `sure_core::paths::Paths::ensure_outside` |
| Path comparison direction | `docs/architecture/STORAGE_AND_DATA_PATHS.md` | `crates/sure-core/src/paths/compare.rs` |
| A diagnostic is never evidence | `docs/architecture/DIAGNOSTICS.md` | `crates/sure-core/src/diagnostics/` |
| Correlation fields and their scope | `docs/architecture/DIAGNOSTICS.md` | `sure_core::diagnostics::Correlation` |
| A log line cannot be restructured by a value | `docs/architecture/DIAGNOSTICS.md` | `crates/sure-core/src/diagnostics/field.rs` |
| Redaction markers | `docs/security/SECRET_REDACTION.md` | `sure_core::diagnostics::{NOT_RECORDED, redact::REDACTED}` |

## Statuses

Six statuses, no others:

| Status | Means | Produced a result about the project? |
| --- | --- | --- |
| `pass` | It ran and the project satisfied it. | yes |
| `fail` | It ran and the project did not satisfy it. | yes |
| `warning` | It ran and the project partially satisfied it. | yes |
| `skipped` | It was deliberately not run. | **no** |
| `error` | It was attempted and the checker failed. | **no** |
| `unknown` | SURE has no basis for a verdict. | **no** |

`skipped`, `error` and `unknown` all mean the project was not checked. They are
never a pass and never disappear from the report.

## Aggregation rules

When results are aggregated:

1. A **critical** check that `fail`ed makes the run `not_ready`.
2. A **critical** check that `error`ed, is `unknown`, or was `skipped` for any
   reason other than an honest scope limit makes the run `not_enough_checked`.
   A denied execution consent lands here: it is not a pass.
3. A run where nothing produced a result is `not_enough_checked`. This includes
   an empty plan.
4. Otherwise, any `fail`, `warning`, `error`, `unknown`, `skipped`, or a
   critical check that was out of scope makes the run `needs_attention`.
5. Only when every recorded check passed and at least one ran is the run
   `green`.

A critical check is **out of scope** when its reason is `not_applicable`,
`unsupported_stack` or `disabled_by_configuration`. Those reasons are honest
scope limits the user chose or the project shape implies. Every other
not-checked reason is a gap and counts against a green verdict.

Independently of the aggregate, `ProjectVerdict::is_ready_for_hand_off` returns
false whenever an open `must_fix` finding exists. A green aggregate next to an
open `must_fix` finding is a contradiction that must never be presented as ready.

## Severity

Four levels, in the exact wording used with users:

| Level | Wire name | Label | Blocks hand-off alone |
| --- | --- | --- | --- |
| `MustFix` | `must_fix` | Must fix | yes |
| `ShouldFixFirst` | `should_fix_first` | Should fix first | no |
| `CanFixLater` | `can_fix_later` | Can fix later | no |
| `Note` | `note` | Note | no |

Ordering is written by hand so that "greater" always means "more serious".
There is no numeric confidence score: uncertainty is `cannot_confirm`.

## Evidence

Five classes, strongest first. `truth_rank` numbers the order.

| Class | Wire name | Rank | Can alone support a `must_fix` finding |
| --- | --- | --- | --- |
| `ObservedFact` | `observed_fact` | 0 | yes |
| `DeterministicCheck` | `deterministic_check` | 1 | yes |
| `ModelAssessment` | `model_assessment` | 2 | **no** |
| `Inference` | `inference` | 3 | **no** |
| `Unknown` | `unknown` | 4 | **no** |

An anchor is checkable only when it has both a location and a locator. Evidence
carries the fingerprint it applies to; evidence whose fingerprint differs from
the current one is stale, and evidence with no fingerprint is never fresh.

Claim assessments: `confirmed`, `contradicted`, `cannot_confirm`,
`not_checkable`. `cannot_confirm` is not `contradicted` and never renders as one.

## Intent trust

| Source | Wire name | Rank | Is a user requirement |
| --- | --- | --- | --- |
| Explicit user goal | `explicit_user_goal` | 0 | yes |
| Observed user request | `observed_user_request` | 1 | yes |
| Project specification | `project_spec` | 2 | no — documentation |
| Agent claim | `agent_claim` | 3 | no — a claim, not proof |
| Inference | `inferred` | 4 | **never** |

Without a user-requirement source, every report must state
`NO_TRUSTED_INTENT_LIMITATION`, verbatim:

> I can check whether the current project runs and whether anything obviously
> looks incomplete. I cannot confirm that it matches your original request
> because that request was not provided to SURE.

`may_claim_full_fulfilment` is the only gate for the sentence "everything you
asked for is done". It requires a trusted source *and* fresh supporting evidence
for every user requirement.

## Execution trust

Three modes: `inspect_only`, `host_confirmed`, `container`.

Six **independent** permissions: `inspect`, `run_project_code`,
`install_dependencies`, `network`, `write_project`, `connect_service`. Granting
one never grants another. `inspect_only` grants only `inspect`.
`host_confirmed` grants `inspect` and `run_project_code`, and nothing more.

An `ArbitraryCommand` always resolves to `needs_consent`, in every mode,
because SURE cannot classify it. Consent carries the exact argument vector and
the working directory; execution never round-trips through a shell string.

A consent grantor can be the interactive user, user-level configuration, or
organization policy. A project-controlled file can **request** authority and is
recorded as `ProjectRequestEscalated`, which `can_grant()` rejects.

## Capability tiers

| Tier | Number | Wire name | Means |
| --- | --- | --- | --- |
| `Snapshot` | 0 | `snapshot` | The project as it is now. Nothing about the session. |
| `Observed` | 1 | `observed` | What the agent did, after the fact. |
| `Protected` | 2 | `protected` | SURE can decide before an action happens. |

`CapabilityReport::is_self_consistent` requires `pre_action_control` to be true
exactly at the protected tier. A contradictory report is a bug, not a weaker
claim. Blind spots are enumerated so a missing observation is reported as a gap.

## Identity format

Every entity ID is `<prefix>_<body>`, where body is lowercase
`[a-z0-9]` and non-empty:

| Kind | Prefix |
| --- | --- |
| Project | `prj` |
| Fingerprint | `fp` |
| Session | `ses` |
| Event | `evt` |
| Run | `run` |
| Check | `chk` |
| Finding | `fnd` |
| Repair | `rep` |
| Claim | `clm` |
| Evidence | `evd` |

The narrow alphabet is a deliberate choice: an ID is safe in a file name, a URL
fragment and a terminal report without escaping. Generated IDs use a counter
plus a per-process seed and are not cryptographic. They are local identifiers,
never security tokens.

A run is not a session. A run is one execution of SURE; a session is observed
harness activity, and either can exist without the other. `run` was added in
P1-T006 and `DOMAIN_SEMANTICS_VERSION` was not bumped: no stored verdict,
finding or evidence record is reinterpreted by a new kind existing, and
`docs/adr/0010-frozen-domain-semantics-in-code.md` requires a bump for a change
of meaning, not for an addition.

## How the wire names are held still

The tables above are the contract. `crates/sure-domain/tests/wire_contract.rs`
is what makes breaking it visible, in three ways:

1. Every enum's `ALL` is declared by `variants!` in the module that defines the
   enum, so nothing that needs to visit the whole vocabulary can quietly skip a
   variant.
2. The test spells every wire name out as a literal and matches on each enum
   with **no wildcard arm**. Adding a variant stops the test crate from
   compiling until someone decides what the new variant is called on the wire.
3. For the six enums that also appear in a JSON schema, the test reads the
   schema's `enum` array and compares it to the Rust list, so the two statements
   of the contract cannot drift apart.

Order is deliberately not asserted: serde is name-based, and reordering variants
is not a wire change.

## Known conformance gaps

These are recorded so they are not forgotten. They are resolved by the tasks
named, not by this document.

1. ~~**Event envelope drift.**~~ **Resolved by P1-T007.**
   `schemas/event.schema.json` now defines `capability_tier` and `project_root`,
   and closes the envelope with `additionalProperties: false` so that an adapter
   field SURE does not understand is a visible error rather than silence.
   `docs/architecture/PROTOCOL.md` records the reasoning.
2. **Fixture metadata vs expectation schema.** The fourteen
   `fixtures/adversarial/*/scenario.json` files carry
   `expected_severity`/`description`/`fixture_status`, while
   `schemas/fixture-expectation.schema.json` requires `required_outcomes`.
   `evaluation/acceptance-manifest.json` lists twenty cases, which is the
   authority for the release corpus. Resolved by P14-T001–T011.
3. **The six stored documents have no version.** Unlike the event envelope, a
   finding or a check result cannot say which build wrote it, so a field from a
   newer SURE is dropped silently on read. Recorded by a test in
   `crates/sure-protocol/tests/round_trip.rs`. Resolved by P1-T005, because the
   version belongs on the stored record rather than on the document.

Two gaps were found and closed while writing the P1-T007 conformance test, both
invisible until then because only the *enum names* in the schemas were being
compared:

- `RepairContract` serialized its issue as `issue`, and `repair.schema.json`
  requires `issue_id`. The Rust field is now `issue_id`.
- `CheckResult` carried neither `evidence_class` nor `project_fingerprint`, both
  of which `check-result.schema.json` requires.

Both are described in `docs/architecture/PROTOCOL.md`.

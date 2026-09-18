# Threat model

## T1 — false completion
Feature appears present but is mocked/stubbed/no-op/incomplete.

## T2 — self-reported testing
Agent claims tests passed without current matching evidence.

## T3 — build success mistaken for product success
Compile/build works while real flow is broken.

## T4 — demo/mock presented as real functionality
Hard-coded data/fake payment/fake email/etc on real paths.

## T5 — broken integration glue
Frontend/backend/config/schema/routes disagree.

## T6 — missing configuration
Required env/config/setup is absent or undocumented.

## T7 — material unsafe behavior
Auth bypass, secret logging, destructive action, missing authorization, obvious injection risk, etc.

## T8 — AI code quality collapse
Conflicting duplicate implementations, swallowed errors, runaway retries, dead code that affects behavior.

## T9 — stale conclusion
Evidence belongs to an earlier project state.

## T10 — incomplete observation
Harness did not expose all actions.

## T11 — checker hallucination
Model-assisted checker invents a defect/pass/evidence.

## T12 — privacy leakage
Prompt/source/log/secret leaves machine unexpectedly.

## T13 — dangerous agent action
Force push, broad deletion, sensitive read, destructive DB/shell action.

Where a harness offers a pre-action hook, SURE answers a tool request with the
action it would take and a sentence saying why, and a `strict` protection mode
holds the sensitive reads and broad changes that
[PROTECTION_MODE.md](PROTECTION_MODE.md) lists. **The answer is advisory in this
release**: both integrations are capability tier 1, so what a decision states is
what SURE would do and not what the harness did — see T18.

## T14 — repair regression
Fixing one issue breaks previous behavior.

## T15 — false release verdict
SURE overstates readiness.

## T16 — malicious/unsafe project execution
Build/test/start scripts themselves execute harmful code on the user's machine.

## T17 — requirement hallucination
SURE infers a likely feature/goal and incorrectly calls it a user requirement.

## T18 — hook fail-open/fail-closed confusion
A protection hook crashes and the user incorrectly assumes the action was blocked.

Addressed per harness and per event in `docs/integrations/HOOK_FAILURE_SEMANTICS.md`,
which keeps "what SURE emits" (measured here) apart from "what the harness does
with it" (quoted upstream, or `cannot confirm`). It records one unresolved case
rather than closing it: `integrations/copilot/` promises to fail open while
`--source copilot` exits 5 for every event.

## T19 — local event tampering
Project code or an agent modifies SURE history/evidence to fabricate a pass.

v0.1 must at minimum make provenance/source and tamper limitations explicit; stronger cryptographic attestation is future work.

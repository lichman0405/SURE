# Domain model

Core entities:

## Project
A local software project or workspace under inspection.

Carries `stacks`, one level per technology found, and `support`, the single level
for the project as a whole with the sentence saying why. The two are different
answers to different questions and are not collapsed: the level `support` holds
is the weakest of what SURE read and what SURE can do with it, so a project whose
manifests were all read can still be at a lower level than any of its stacks, and
its `reason` names both. `sure_core::support` is the only rule that fills it;
`docs/product/SUPPORTED_STACKS.md` is the authority for the levels.

## ProjectFingerprint
Identity of the exact relevant project state to which evidence/results apply.

## ProjectIntent
A goal/requirement source with provenance/trust class.

## Session
Observed coding-harness activity related to a project.

## Event
Normalized observed harness event.

## CheckPlan
Approved set of deterministic/behavioral checks for a project state.

## CheckResult
Outcome of one check: pass/fail/warning/skipped/error/unknown.

## Evidence
Observed fact, deterministic output, model assessment, inference or unknown.

## Claim
A statement made by the coding agent that can sometimes be checked.

## Finding
A material user-facing problem/uncertainty with evidence anchors.

## RepairContract
Bounded instructions and acceptance conditions handed to a coding harness.

## Verdict
Overall recommendation derived from findings + check coverage, never a free-form LLM opinion.

## CapabilityTier
Snapshot / observed-session / protected-session capability of an integration.

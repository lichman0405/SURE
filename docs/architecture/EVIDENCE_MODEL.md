# Evidence model

## Evidence classes

1. `observed_fact` — captured directly from OS/harness/current project state.
2. `deterministic_check` — a known command/tool produced a result bound to a project fingerprint.
3. `model_assessment` — model interpretation grounded in specific anchors; may be wrong.
4. `inference` — pattern-derived conclusion with uncertainty.
5. `unknown` — insufficient evidence.

## Fingerprint

For Git projects, consider:
- HEAD;
- dirty tracked diff digest;
- relevant untracked manifest/digest;
- ignored/generated exclusions appropriate to the check.

For non-Git projects, use a deterministic scoped content manifest/fingerprint.

A result must become stale when relevant project state changes.

## Claim assessment

- `confirmed`
- `contradicted`
- `cannot_confirm`
- `not_checkable`

`cannot_confirm` is not the same as `contradicted`.

## Test freshness

A test run against an older relevant project fingerprint cannot prove the final code passes.

The report may show:

> "These tests passed before the latest code changes, so I cannot use them to confirm the final version."

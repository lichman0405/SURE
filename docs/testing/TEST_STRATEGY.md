# Test strategy

## Core rule

A false green is worse than a visible error/unknown.

## Unit tests

- IDs/enums/status aggregation;
- project fingerprints;
- intent-source trust labels;
- command classification;
- check planning;
- claim classification;
- evidence freshness;
- finding severity;
- repair schemas;
- redaction;
- protection decisions.

## Integration tests

Real temporary directories/processes/Git/SQLite/local HTTP servers.

Test Windows/macOS/Linux path and process semantics. Native Windows path/process behavior is release-critical, not optional CI coverage.

## E2E stacks

At minimum:
- TypeScript web/service fixture;
- Python API fixture;
- Rust CLI/service fixture.

## Adversarial acceptance

See `docs/testing/ADVERSARIAL_FIXTURES.md` and `evaluation/acceptance-manifest.json`.

## Golden language tests

Default report must not regress into engineering jargon or claim unknown evidence as fact.

## Harness tests

Use recorded synthetic hook/event payloads so CI does not require paid/authenticated live sessions.

Live harness smoke tests may be manual/optional and are recorded separately.

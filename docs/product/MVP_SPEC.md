# MVP specification — v0.1

## Required modes

### A. Check this project

Input: a local project directory, plus optional explicit goal/specification.

Must work without any previous SURE recording.

Outputs:

- what SURE identified;
- what was inspected;
- which dynamic checks were allowed/run;
- whether the project builds/tests/starts where verified;
- material incomplete/fake/broken behavior;
- obvious high-impact security/reliability problems;
- what could not be checked;
- overall recommendation;
- repair contracts for actionable findings.

Without an explicit/observed goal, this mode must **not** claim full fulfillment of the user's original request.

### B. Full-session checking

Adds evidence from supported harness integrations:

- session lifecycle;
- tool/command calls where exposed;
- outcomes;
- files touched/read where exposed and permitted;
- Git activity;
- build/test/start actions;
- selected agent completion claims;
- optional full transcripts/terminal output only in full-recording mode.

## Project stacks

First-class v0.1 discovery/check support:

- JavaScript/TypeScript (Node.js web/server projects);
- Python applications/APIs;
- Rust applications/services.

Generic fallback:

- Go and other projects when commands/config are discoverable without pretending framework-specific certainty.

## Checks

- manifest/dependency consistency;
- build/type/lint/test detection and approved execution;
- local start/service smoke probes;
- optional browser behavior probe;
- environment-variable/config references;
- README/setup claim validation;
- DB schema/migration consistency framework;
- frontend/backend route/API consistency where deterministically discoverable;
- mocks/fakes/stubs/TODO/placeholders on real user paths;
- hard-coded demo behavior presented as real;
- no-op/fake-success handlers;
- material auth/security mistakes;
- error-path gaps;
- missing critical tests;
- AI claims versus observed evidence/current state;
- project intent versus observable implementation when a trusted intent source exists.

## Explicit non-goals

- production cloud deployment verification;
- live production DB mutation;
- DNS/domain management;
- real production payment execution;
- multi-agent scheduling;
- replacement IDE;
- hosted LLM proxy/service;
- enterprise SSO/RBAC/dashboard;
- automatic PR merge gate;
- cryptographic attestation/signing platform.

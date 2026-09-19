# Adversarial fixtures

Mandatory scenarios:

- fake payment;
- fake auth;
- fake email;
- dead primary action;
- hard-coded demo analytics;
- missing DB migration;
- frontend/backend route mismatch;
- lying README;
- tests claimed but never run;
- stale test run after later changes;
- external payment/email behavior cannot actually be verified locally;
- repair regression;
- critical checker crash/error;
- missing evidence => cannot confirm;
- benign test mocks do not become production must-fix findings;
- dangerous delete;
- force push;
- sensitive file read;
- missing user intent => no requirement-fulfillment claim;
- dynamic check not authorized => visible not-checked state.

Every fixture has machine-readable expected outcomes. A mandatory false green blocks release: the
block is `sure_core::release_gate`'s `decision` in `target/tmp/release-gate.json`, which reads
`blocked` while any case the manifest marks `release_blocking` is unmet — or is a case nothing
observed, which is not a pass either.

## Where the machine-readable outcomes are

| corpus | data | what runs it |
| --- | --- | --- |
| the scenarios above | `fixtures/adversarial/<id>/scenario.json`, bound to `evaluation/acceptance-manifest.json` | `crates/sure-testkit/tests/fixture_apps.rs` |
| the release contract those scenarios are graded against, observed | `evaluation/acceptance-manifest.json`, into `target/tmp/acceptance-report.json` | `crates/sure-core/tests/acceptance_report_runner.rs` |
| the release decision taken from that report | the same report, into `target/tmp/release-gate.json` | `crates/sure-core/src/release_gate.rs`, driven by `crates/sure-core/tests/release_gate_runner.rs` |
| secret redaction and protection mode (`DEFINITION_OF_DONE.md`: "secret-redaction fixtures pass;", `PRODUCT_EVALS.md`: "secret redaction mandatory fixtures") | `fixtures/privacy/manifest.json` | `crates/sure-cli/tests/privacy_suite.rs` |

The privacy corpus is not a list of scenarios under `fixtures/adversarial/`: its
entries are cases about a *run* — what a real `sure` process stored with no
settings file of any kind, what a hook answered, what a refusal printed — rather
than projects with an expected verdict. Each entry names the documents that
promise it and the sentence in each, so an entry fails when the promise it was
written for is withdrawn, and each one that points at a test somewhere else is
checked to still name a test that exists. `fixtures/privacy/README.md` is the
human half; `not_confirmed` in the manifest, printed on every run, is what the
corpus could not confirm and what would settle it.

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

Every fixture has machine-readable expected outcomes. A mandatory false green blocks release.

# Roadmap

The machine-readable source of truth is `tasks/phases.json` + `tasks/tasks.json`.

High-level v0.1 sequence:

1. freeze truth/safety semantics;
2. production Rust core and local state;
3. project discovery + ProjectIntent + fingerprinting;
4. safe execution and explicit execution trust;
5. deterministic and behavioral checks;
6. false-completion analysis;
7. plain-language findings/verdicts;
8. session evidence + AI claim checking;
9. repair contract + independent re-check;
10. Claude Code integration;
11. Cursor Plugin integration;
12. Codex/portable plugin and optional analysis providers;
13. privacy/redaction/protection hardening;
14. adversarial product evals;
15. packaging/install/release;
16. final clean-system acceptance and owner review.

v0.1 deliberately does not include a team dashboard, hosted model service, multi-agent scheduler or production deployment verifier.

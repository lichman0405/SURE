# SURE

**Software Understanding & Reality Evaluation**

> **AI says it's done. Be SURE.**

SURE is a local-first project checking layer for software built with AI coding tools.

It helps an individual builder or a small team answer:

1. **Can this project actually work?**
2. **Is anything clearly broken, unsafe, fake or incomplete?**
3. **Can the AI's claims about what it completed be confirmed?**

When SURE finds a real problem, it explains the consequence in plain language, creates a bounded repair contract, hands that contract back to the user's existing coding agent, and checks again after the repair.

## Product rule

> **Never replace the harness. Sit beside it.**

Claude Code users keep using Claude Code. Cursor users keep using Cursor. Codex users keep using Codex.

## Two modes

### Check this project

Works even if SURE did not observe the development session.

It can inspect the current project, run approved checks, identify obvious incomplete/fake implementation and produce a plain-language report.

It **cannot** claim that the project matches the user's original request unless that request/goal was supplied or observed.

### Full-session checking

With a supported harness plugin/hook, SURE also records selected development facts and can compare the AI's completion claims with what actually happened.

## Current repository status

This repository is being implemented autonomously from a v0.1 bootstrap. It is **not the completed SURE product**, but it is already a working Rust workspace with substantial checking infrastructure in place.

- **Progress:** 86 of 166 v0.1 tasks accepted (phase P8 in progress).
- **Branch for active work:** `claude/v0.1-autonomous`.
- **Canonical remote:** `https://github.com/lichman0405/SURE.git`.

Implemented so far:

- Multi-ecosystem project discovery (Node, Python, Rust) and component graph.
- Static checks: config references, environment completeness, dependency state, database migration consistency.
- Runtime checks: local service smoke tests, HTTP route probes, browser-driven flow probes, with cancellation and cleanup.
- False-completion analysis: candidate scanner for TODO/mock/stub/placeholder patterns, production-path/context filter, no-op/fake-success heuristics, hard-coded demo-data heuristics, frontend/backend route consistency, UI-action completeness bridge, grounded semantic-analysis request/response contract, project-intent versus implementation comparison, and a candidate aggregator that deduplicates by evidence anchor and prioritizes user impact over style noise.
- Finding model: four-level severity, `AssessmentSource`, `SeverityRationale`, concrete evidence anchors including intent/claim/model, a builder that refuses unsupported `MustFix` sources, and a plain-language finding contract (`what`/`impact`/`severity`/`next action`).
- Coverage and not-checked summary: joins a scheduled check plan to its run report, counts checked/skipped/could-not-run checks, surfaces critical gaps, and reports the adapter support level in plain language.
- Overall project verdict: assembles findings, aggregate result, coverage gaps, capability level, and intent into a `ProjectVerdict`, then renders a plain-language summary with independent false-green protection.
- Terminal human report: renders a `ProjectVerdict` to plain text for a standard terminal, with no ANSI codes by default, material findings first, coverage gaps, and escaped attacker-controlled text.
- Stable JSON report: deterministic, versioned, schema-validated JSON output from a `ProjectVerdict` for downstream tools, with safe handling of control characters.
- Portable Markdown/HTML reports: self-contained reports from a `ProjectVerdict` with HTML entity escaping, control-character escaping, and no external resources.
- Plain-language golden tests: integration tests lock down exact user-facing wording across terminal, Markdown, HTML, and JSON reports, including false-green protection and after-the-fact caveat cases.
- Evidence-grounded reporting with plain-language verdicts and `Cannot confirm` as a valid result.
- Versioned harness event ingestion: validates handshake, document kind, and schema before accepting a harness event, with plain-language diagnostics for version mismatches and malformed payloads.
- Session/event persistence: stores harness events bound to project/session/time/capability source in the local SQLite store, with retention metadata, idempotent ingestion, and redaction before storage.
- Standard recording projection: summarizes tool/command/git/file/build-test/outcome activity from harness events and stores the privacy-conscious summary without the full transcript.
- Full recording opt-in projection: retains raw transcript/terminal payloads only under explicit opt-in, with a distinct storage marker, redaction before storage, and short retention.
- Observed ProjectIntent capture path: converts harness `user.request` events into `ObservedUserRequest` requirements when full recording is granted, redacts credential-shaped prompt text before storage, and retains the raw event only under explicit full-recording consent.
- Agent completion-claim extraction: validates harness `agent.claim` events, redacts credential-shaped `claim_text`, and stores a `Claim` document with `assessment: cannot_confirm` plus original event provenance.
- Deterministic claim checkers: checks stored `Claim` documents against recorded harness events via the standard recording projection. `test_ran`, `file_changed`, `git_state`, and `current_code` claims can be `Confirmed` when evidence exists, `CannotConfirm` when it does not, or `NotCheckable` for unknown/missing types; the checker never treats absence of evidence as `Contradicted`.
- Stale test/result evidence detection: `test_ran` and `current_code` claims are downgraded to `CannotConfirm` when a later file write/delete or relevant git operation supersedes the proof event, so a passing run before the latest code changes is not reported as proof of the current version.
- Capability/blind-spot reporting from recorded events: `report_from_events` derives an honest [`CapabilityReport`] from stored harness events, reflecting the actual observed tier and listing blind spots for missing user-request, agent-claim, tool/command, failure, file, or git visibility rather than trusting an adapter's self-reported capability.

Start with `START_HERE.md` for the Windows bootstrap and development workflow.

## Development bootstrap

Primary v0.1 development host: **Windows 11 x64 / native MSVC**. See `START_HERE.md`.

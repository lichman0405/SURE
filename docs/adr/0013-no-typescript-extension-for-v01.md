# ADR 0013 — No TypeScript extension for v0.1

Status: Accepted

Date: 2026-09-18

## Context

ADR 0008 decided to use current plugin/hook packaging for Cursor and to add a
TypeScript extension only when a concrete missing capability demands it. P11-T009
revisits that decision after the v0.1 harness integrations were implemented:

- Claude Code integration uses the documented command-hook packaging:
  `integrations/claude-code/commands/*.md`, `integrations/claude-code/hooks/hooks.json`,
  and `integrations/claude-code/scripts/sure-hook.ps1`.
- Cursor integration uses the same hook protocol:
  `integrations/cursor/hooks/hooks.json` and `integrations/cursor/scripts/sure-hook.ps1`.
- GitHub Copilot adapter boundary is documented to reuse the same event/repair
  protocols: `integrations/copilot/README.md`.

The question is whether any v0.1 user job cannot be satisfied without a
TypeScript extension that runs inside the harness editor.

## Decision

**For v0.1, plugins/hooks satisfy the required surface. No TypeScript extension
is added.**

Concretely:

1. Protection decisions remain in the Rust core (`crates/sure-core/src/hook_protection.rs`).
   The harness integrations are thin launchers that forward stdin JSON to
   `sure hook ingest` and return the resulting JSON decision.
2. Custom commands (`sure check`, `sure repair`, `sure recheck`, `sure status`)
   are surfaced through the harness's command system rather than an editor
   extension. The commands are documented in `integrations/claude-code/commands/`.
3. Event persistence, history, and reporting stay core-owned. The integration
   must not duplicate the check engine or invent evidence.
4. If a future harness cannot expose command hooks, or needs UI affordances that
   cannot be provided by a command/hook manifest, a concrete missing capability
   must be documented before any TypeScript extension is built. Such an
   extension must remain thin and must not reimplement core logic.

## Alternatives rejected

**Build a TypeScript extension now for parity or polish.** There is no concrete
v0.1 capability gap that an extension would close. Adding one would create a
second implementation surface, increase packaging complexity, and risk core logic
leaking into a language with weaker sandbox guarantees.

**Ship a hybrid where some integrations use hooks and others use an extension.**
This would fragment the integration surface and make tier-honesty reporting
harder. A single thin-hook pattern is easier to audit.

## Consequences

- v0.1 release packaging is simpler: editor plugins only need a hook/command
  manifest and a thin launcher script.
- The core remains the single source of truth for protection, checks, and
  evidence.
- Future integrations (Copilot, Windsurf, etc.) should target the same
  event/repair protocol rather than a per-editor extension API.
- A TypeScript extension is still allowed later, but only after a documented
  concrete missing capability and a plan that keeps the extension thin.

## Frozen semantics

Capability tiers and integration thinness are bounded by
`docs/architecture/FROZEN_SEMANTICS.md`. Whichever surface ships, the adapter
must report the tier it actually reached and its blind spots, and must not imply
a capability the platform does not grant.

## Revisit when

A target harness cannot implement one of the v0.1 user jobs (observe, protect,
check, repair, recheck, report status) through its existing command/hook
mechanism, and that limitation is confirmed with concrete product evidence.

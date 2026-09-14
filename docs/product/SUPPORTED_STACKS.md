# Supported stacks and support levels

Support claims must use levels.

## Level A — first-class

Framework-aware discovery and meaningful deterministic checks.

Initial target families:
- Node.js / TypeScript web and server projects;
- Python app/API projects;
- Rust app/service projects.

## Level B — generic

SURE can discover common manifests/commands and run approved generic checks, but does not claim framework-specific completeness.

## Level C — inspect-only

SURE can inspect files/config and report obvious issues but lacks safe/reliable run semantics.

Every report should state the achieved support level rather than silently pretending all stacks are equal.

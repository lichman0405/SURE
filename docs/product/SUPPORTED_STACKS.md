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

## The achieved level, and what sets it

`Project::support` is the level for the project as a whole, and
`sure_core::support::classify` is the one rule that fills it. The rule is **the
weakest of two things: what SURE read, and what SURE can do with it** — this same
document's definitions, applied to a build that currently reads manifests and
runs nothing.

Both level A and level B include running something, so **the level any project
can reach in this build is C**, held in one named constant,
`sure_core::support::CEILING`. Where a project's manifest was read and graded
`generic` by discovery, the report says so and says why the answer is still C
rather than leaving a reader to find the difference; see
`docs/architecture/ECOSYSTEM_DISCOVERY.md`, whose per-ecosystem `grade` measures
what SURE **understands**, while this document's levels measure what SURE can
**do**. Raising `CEILING` is the whole of the change when checks land, and the
tests around it are written to fail at that moment.

**This is an open decision, recorded rather than settled.** The other reading —
that a level states what SURE understands, that discovery's grade is the whole
answer, and that the ceiling should be `generic` — is a real one. It is not
taken here because `SupportLevel::plain_description` renders `generic` as *"SURE
can find how this project is built and run"*, and SURE cannot run it: a claim a
user would take to mean checks exist. `progress/HANDOFF.md` carries the
alternatives and the change either way.

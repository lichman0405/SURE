# Product thesis

## Problem

AI coding tools can build complete-looking applications for people who cannot reliably inspect the implementation themselves.

The dangerous failure is not only "bad code". It is **false completion**:

- a feature looks present but is still fake or mocked;
- the UI exists but the real backend action is missing;
- tests are claimed but were never run against the final code;
- setup instructions do not work;
- a repair fixes one thing and breaks another;
- an agent confidently says a project is complete even though external behavior was never verified.

## Thesis

SURE is a local-first, harness-independent project checking layer for AI-built software.

It asks:

1. Can this actually work?
2. What is clearly wrong, unsafe or incomplete?
3. Is what the AI said actually true?
4. What should the current AI fix next, and how do we verify it?

## Primary user

An individual builder using Claude Code, Cursor, Codex or another AI coding harness, potentially without formal CS/IT training.

Their question is:

> "The AI says it's done. Can I trust this project?"

## Secondary user

A small fast-moving team that wants the same checking behavior without replacing its chosen coding tools.

## Differentiation target

SURE is not primarily a PR reviewer, security scanner, test generator, session recorder or orchestration framework.

The differentiated combination is:

- whole-project-first;
- false-completion detection;
- explicit project-intent semantics;
- AI claim checking against evidence;
- honest unknowns;
- plain-language explanation;
- repair contracts handed back to the current harness;
- re-check after repair;
- local-first and harness-neutral.

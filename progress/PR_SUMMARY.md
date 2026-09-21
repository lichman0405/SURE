# PR summary — `claude/v0.1-autonomous` → `main`

**This file is prepared for the owner's review. No merge to `main` was performed,
and none will be without the owner's instruction.**

## What this branch is

SURE — **Software Understanding & Reality Evaluation**. *"AI says it's done. Be
SURE."* A project truth/checking layer for AI-built software: it decides what can
actually be confirmed about a project, and says `Cannot confirm` where it cannot.
It is not a coding-agent orchestrator.

## The one-line case for merging

The product's central rule is that **a false green is more serious than a visible
error**, and this branch is the record of that rule being applied to SURE itself —
including to the supervisor who was building it.

## What is in it

- **A working CLI** — `sure check`, `repair`, `recheck`, `doctor`, `hook ingest`,
  `mcp`, `config`, `history` — with a human report, a machine frame, Markdown and
  HTML forms, and a re-check loop that closes a finding only when every check its
  repair contract named has passed.
- **Seven local gates** in `scripts/gates.ps1`, run from PowerShell. CI runs six of
  the seven on all three platforms; the seventh, `check-non-windows.mjs`, is local
  only, because its subject is the *other* platform's shape.
- **`FINAL_REPORT.md`** (1050 lines) — the deliverable, naming the exact commit its
  readings were taken at, and carrying ten sections including a "what this report
  did not measure" section.
- **`docs/development/DOGFOOD.md`** — SURE pointed at its own repository, which is
  the hardest project it will ever be pointed at, because SURE's repo contains both
  SURE's detection patterns as source text and SURE's deliberate false-completion
  fixtures.

## Evidence a reviewer can re-run

    pwsh -NoProfile -File scripts/gates.ps1 -Label review

Expected at this tip: all seven gates exit 0, `passed=2818 failed=0`,
`red: none`, `store identical: True`.

The readings for every accepted task are in `progress/state.json` under that
task's `evidence`, and the narrative record is `progress/HANDOFF.md` and
`progress/DECISIONS.md`.

## What this branch does NOT claim

These are carried deliberately, each with its measurement, and a reviewer should
read them before deciding:

- **Nothing is released.** No tag, no GitHub Release, no package manager. And
  `release.yml` **is not on `main`** — `git ls-tree origin/main
  --name-only .github/workflows/` returns only `ci.yml` and `release-dry-run.yml`
  — so the release workflow cannot be dispatched at all. No Windows artifact has
  ever been built on a runner; the only one that exists was built by hand from an
  earlier commit and is stale.
- **No check runs anything.** `support.rs:116` is `CEILING: InspectOnly` and a test
  fails the day that changes. Every check is `unknown` rather than passed.
- **No model is consulted in this build**, and a test asserts that no run can say
  one was.
- **`sure config show` refuses** rather than answering (exit 3).
- **Two launcher defects are carried**: under Windows PowerShell 5.1 the hook write
  path hands SURE `??` and stores 0 rows; and codex's declared `-File` invocation
  still mangles a non-ASCII payload. Both are measured, both have their reason for
  being carried rather than fixed.
- **The suppression census reports zero about a tree that contains five.** All four
  shipped hook launchers open with `$ErrorActionPreference='SilentlyContinue'`;
  the census reads six harness files and none of them is a launcher. The zero is
  true of the census's six and false of the tree.

## The corrections this branch made to its own record

Listed because they are the strongest evidence the method works:

- **CI was red for nine consecutive pushes and the supervisor was not looking.**
  One stale fixture pinned a sentence a document had rewritten. Resolved at
  `e8ba638`; the green run is what confirms the diagnosis rather than a second
  opinion about it.
- **Six of the twelve P16 tasks were accepted on deferred readings**, and one of
  the reasons given for deferring — "this task moved Markdown and no code" — was
  false as a justification, because pages are *operands* here: `privacy_suite`
  reads a Markdown page as its input.
- **Three of the supervisor's own readings were withdrawn on this branch**, with
  the cause recorded as *unestablished* rather than explained away, and two
  measurement errors of the supervisor's are written down with them.
- **A fixture that required SURE to lie was found and deleted.** It had been green.

## Reviewing this

`gh pr create` was deliberately **not** run. Open the PR from the GitHub UI, or
run it yourself, when you are ready.

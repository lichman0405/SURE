# Contributing

SURE is currently in autonomous v0.1 bootstrap/development.

Before a change:
- read `MASTER_PROMPT.md` and relevant product/architecture docs;
- preserve evidence/uncertainty semantics;
- add tests for behavior changes;
- never weaken a false-green guard merely to make a test pass.

Run the project gates documented in `MASTER_PROMPT.md`.

## Before you commit: `SHA256SUMS.txt`

`SHA256SUMS.txt` records a SHA-256 for a curated subset of the tree, and it is
checked — on every platform, in every clone, by
`crates/sure-testkit/tests/source_manifest.rs`, which runs as part of
`cargo test --workspace --all-features`. That command is gate 3 of the local gate
set and step 1 of every leg of the `rust` matrix in `.github/workflows/ci.yml`,
so this check cannot be forgotten: there is nothing to remember to run.

The digests are of the bytes **git will store in the commit** — the index blob —
and not of the bytes on disk. `.gitattributes` sets `*.ps1 text eol=crlf`, so
those two differ for the `.ps1` paths the manifest lists, and the index form is
the one that is the same on Windows, macOS and Linux.

**That makes the order of the three steps load-bearing:**

```sh
git add <the files you changed>                              # 1
cargo run -p sure-testkit --bin source-manifest -- --write   # 2
git add SHA256SUMS.txt                                       # 3
```

Regenerating before staging would hash the previous commit's bytes and write
digests that the next `git add` invalidates. If you forget step 2 entirely, the
check fails and tells you both digests and the same three steps; nothing goes
quietly wrong.

Three of the files it lists — `progress/HANDOFF.md`, `progress/state.json` and
`tasks/tasks.json` — move on every acceptance commit, so step 2 is not optional
on the commit that records an acceptance.

The tool never adds a path and never removes one: which paths belong in a curated
selection is a judgement, not a program's call. Adding coverage is therefore a
deliberate edit to `SHA256SUMS.txt`, followed by step 2.

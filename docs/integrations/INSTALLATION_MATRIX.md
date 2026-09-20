# Integration installation matrix

## Claude Code
Development: local plugin install/symlink using Claude Code's supported plugin workflow.
Release: documented plugin package/marketplace route as appropriate.

## Cursor
Development: local Cursor Plugin under `~/.cursor/plugins/local/sure` (symlink preferred for iteration when allowed).
Release: Cursor Marketplace submission is external/account-review work and is not required for core v0.1 code completion.

## Codex
Development: local Agent Plugin/skill install according to the current Codex/OpenAI plugin/skill surface.
Release: package and instructions; marketplace/hosted publication is external if applicable.

## CLI-only fallback
Every user can run `sure check` without installing a harness plugin.

## What "supported" means for a secondary platform

`P15-T008` had to define this, because the acceptance line it answers — that
secondary platform launchers are *included as supported* — does not say what the
word carries. A launcher ships as **supported** on a platform when three things
hold together, and a test in
`crates/sure-testkit/tests/integration_thinness.rs` fails if any of them stops
holding:

1. **It is in the package and names its own interpreter.** The first line is a
   shebang (`#!/usr/bin/env bash`), so nothing has to guess how to start it.
2. **The git index records it executable.** `git ls-files -s` reports `100755`.
   The mode is the one property that "can be started by path" needs and that no
   file content can carry: it is what a checkout has to materialise.
3. **A manifest that names it by path is checked beside it**, so the invocation
   and the file agree. `the_codex_manifest_gives_each_platform_a_launcher_that_platform_can_start`
   asserts that the codex manifest still names the `.sh` directly, and that its
   `commandWindows` still starts `powershell -NoProfile -File`.

Three launchers are covered:
`integrations/claude-code/scripts/sure-hook.sh`,
`integrations/cursor/scripts/sure-hook.sh` and
`integrations/codex/scripts/sure-hook.sh`.

### The decision, and the option not taken

**The index bit was set** — `git update-index --chmod=+x` on those three files.
The alternative was to leave every mode at `100644` and name an interpreter in
the manifest instead (`"sh ${PLUGIN_ROOT}/scripts/sure-hook.sh"`), which makes
the mode irrelevant to that one invocation.

It was not taken, for three reasons:

- **It repairs one invocation out of the set.** The codex manifest is the only
  manifest in this repository that names a `.sh` by path; claude-code's, cursor's
  and copilot's all name `sure-hook.ps1` in PowerShell syntax. Naming an
  interpreter there would have left the claude-code and cursor `.sh` launchers
  exactly as unstartable-by-path as they were, with no manifest — and so no test
  — covering them.
- **It edits a contract with a loader this machine cannot run.** Whatever the
  resulting string has to survive is Codex's command parsing. The index bit
  changes no runtime contract at all; it changes what a checkout writes to disk.
- **It restates what the file already says.** Each launcher's shebang already
  names its interpreter. An interpreter in the manifest is a second copy of that
  choice, in a different file, free to drift from the first.

The shebangs were left alone. The bit was the missing half, and it is this
repository's own to set.

### What the decision costs

- **A Windows contributor who opts into `core.filemode=true` sees them as
  modified.** Measured on this checkout: `git status --porcelain` reports them
  clean at the default, and `git -c core.filemode=true status --porcelain`
  reports `MM` for all three. The default is what Git for Windows sets and what
  this repository's `.git/config` records, so the tree stays clean as configured.
- **The promise is only as good as the clone.** A filesystem that cannot
  represent the bit (FAT, some network mounts) will not carry it. A checkout on
  a filesystem that can, gets `0755`.
- **macOS is not measured.** No macOS machine was touched by this task; see
  below.

### Cannot confirm

- **What macOS does with a launcher it finds at `100644`.** The POSIX rule is
  that the mode is what `execve` checks; whether a given harness reaches that
  check or falls back to running the file through a shell is a property of that
  harness's spawn path, and no macOS harness was run. This page therefore does
  not claim the codex hook is startable on macOS — only that the repository now
  ships the mode that makes it startable by path.
- **What Codex does with the codex manifest's command string.**
  `${PLUGIN_ROOT}` is not a Codex variable, as that file says itself and as
  `integrations/codex/README.md` repeats: the user renders it before the file is
  copied into place. The loading behaviour of a third-party harness is not
  verifiable from here.

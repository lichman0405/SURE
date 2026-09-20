# Integration installation matrix

## Claude Code
Development: local plugin install/symlink using Claude Code's supported plugin workflow.
Release: documented plugin package/marketplace route as appropriate.

`P15-T009` adds the pair of per-user PowerShell commands the other packages
already had — `integrations/claude-code/scripts/install.ps1` and
`uninstall.ps1` — which place the package under `%LOCALAPPDATA%\claude-plugins\sure`.
What "install" means for this package, and which part of it stays Claude Code's,
is decided in [What installing the Claude Code package means](#what-installing-the-claude-code-package-means)
below.

## Cursor
Development: local Cursor Plugin under `~/.cursor/plugins/local/sure` (symlink preferred for iteration when allowed).
Release: Cursor Marketplace submission is external/account-review work and is not required for core v0.1 code completion.

## Codex
Development: local Agent Plugin/skill install according to the current Codex/OpenAI plugin/skill surface.
Release: package and instructions; marketplace/hosted publication is external if applicable.

## CLI-only fallback
Every user can run `sure check` without installing a harness plugin.

## What installing the Claude Code package means

`P15-T009`'s acceptance is that a Windows user can install and remove the Claude
and Cursor packages with documented PowerShell commands, and that a normal
per-user install needs neither symlink privileges nor administrator rights. The
Cursor package already had that pair of scripts. The Claude Code package had
neither, and its README, its `docs/smoke-test.md` and this matrix all referred to
"Claude Code's plugin directory" without a command that puts anything there.

**The install is a per-user copy of the package, and the load stays Claude
Code's step.** `install.ps1` copies `integrations/claude-code/` to
`%LOCALAPPDATA%\claude-plugins\sure` (`$env:CLAUDE_PLUGIN_DIR` overrides it),
resolving the SURE binary in the order every launcher and installer here uses
and refusing before it touches the directory when there is none.
`uninstall.ps1` removes it and reports whether it was there.

A copy is the right shape for *this* package because of how the package names
its own files: through `${CLAUDE_PLUGIN_ROOT}`, which is Claude Code's
substitution, so the package is relocatable by design and any copy of it is a
valid plugin root. Nothing has to be rendered into it and no absolute path has
to be baked in, which is the one job the Cursor package's installer does that
this one does not need.

### The option not taken, and the argument against it

The alternative was to document Claude Code's own plugin commands as the install
— `claude plugin install <plugin>@<marketplace>` and `claude plugin uninstall`,
which the documentation describes as the non-interactive CLI for plugin
management. It was not taken:

- **There is nothing here to install from.** That command installs *from a
  marketplace*, and this repository ships no `marketplace.json` anywhere. A
  command that cannot be run from this tree is not an install path; it is a
  placeholder for one.
- **It cannot be measured from here.** It writes to the user's own Claude Code
  state under `%USERPROFILE%\.claude`, which this task is forbidden to touch, and
  no plugin was installed into a running session. The copy installer has a test;
  a command nobody ran has a citation.
- **It makes the acceptance depend on a second product's packaging.** What SURE
  can own and test is the copy of its own package on disk. What it cannot own is
  a marketplace entry that does not exist yet.

What the copy gives up is stated where a user meets it rather than here:
`integrations/claude-code/README.md` says at the command that the script places
the package and does not register it, and names the documented loading
mechanisms — skills-directory plugins, `claude plugin install`, and
`claude --plugin-dir` for a session-only load — each marked as documented
behaviour this repository has not watched.

### What is measured, and by what

Every item below is an assertion in
`crates/sure-testkit/tests/integration_thinness.rs`:

- the install and the remove, run into a scratch directory through whichever
  PowerShell host on the machine will start the script, with no
  `-ExecutionPolicy Bypass` passed;
- the copy path, exercised by pointing `TEMP` at a directory that does not exist
  so the symlink probe cannot succeed whatever the machine allows;
- the refusal when no SURE binary can be found, and that it touches no
  directory;
- the installed copy still carrying `${CLAUDE_PLUGIN_ROOT}`, which the render
  step would destroy if it were keyed on the wrong string;
- the two scripts naming the same default directory, since an uninstaller whose
  default drifts leaves the package in place and reports that it was never
  installed;
- the README naming both commands and that same directory.

Five of those six were falsified by breaking the property and watching the test
fail — the copy path, the refusal, the placeholder, the shared default, and the
README's destination. The hand-back names the break and the failure for each.
The install-and-remove assertion is not one of the five: it runs the scripts and
reads the filesystem back, and it fails when the package is not there, which is
what it says.

The per-user install was also run by hand on 2026-09-21 on this machine, which
has no Developer Mode: `New-Item -ItemType SymbolicLink` answered
`UnauthorizedAccessException` — *this operation requires administrator
privileges* — and the installer completed with exit 0 on the copy path. The
process that ran it was not elevated
(`WindowsPrincipal.IsInRole(Administrator)` returned `False`).

### Cannot confirm

- **That Claude Code loads a plugin by any of the documented mechanisms.** No
  plugin was installed into a running session and none was watched loading. The
  mechanisms are cited from Claude Code's documentation
  (`code.claude.com/docs/en/plugins-reference`, `.../discover-plugins`,
  `.../cli-reference`), reached on 2026-09-21 by a documentation read rather
  than by a run, and they are labelled as documented wherever they appear.
- **That a real symlink installs and uninstalls cleanly.** No host here will
  create one, so that branch has never run. Measured instead, in its place: a
  reparse point at the same path is removed as a link and its target left whole
  — `Remove-Item -Recurse -Force` on a junction reported
  `link exists=False ; keep.txt exists=True` under PowerShell 7.6.6 and under
  Windows PowerShell 5.1.26100.9444.

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

# GitHub workflow

Canonical remote:

```text
https://github.com/lichman0405/SURE.git
```

Default branch: `main`.

Autonomous implementation branch:

```text
claude/v0.1-autonomous
```

## Bootstrap on Windows

Run:

```powershell
.\scripts\Publish-Bootstrap.ps1
```

The script performs the first explicit publish. It aborts rather than overwrite incompatible remote history and never force-pushes.

A POSIX `publish-bootstrap.sh` may remain for secondary macOS/Linux developer use, but PowerShell is canonical.

## Commit policy

Accepted tasks use:

```text
P4-T003: implement Python deterministic checks
```

Do not bundle unrelated tasks merely to reduce commit count.

## Push policy

- no force push;
- phase-boundary checkpoint pushes when authenticated;
- local work continues if GitHub temporarily fails;
- final implementation is presented as a branch/PR for owner review;
- autonomous flow does not merge to `main` without explicit active-session authorization.

## Continuous integration

`.github/workflows/ci.yml` runs on every push to `claude/**` and on pull
requests. Three jobs, and each is the only check of something:

| Job | What nothing else checks |
| --- | --- |
| `rust` × windows, macos, ubuntu | `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast` |
| `shellcheck-secondary` | the POSIX shell scripts, on a machine that has `shellcheck` |
| `bootstrap-validate-windows` | the Windows bootstrap scripts, `validate-bootstrap.mjs` and the task state |

**A push is not finished until its run has been read.** CI is the only verifier
for platform-gated code, and the local Windows gate set cannot stand in for it:
`#[cfg(unix)]` and `#[cfg(not(target_os = "macos"))]` code is never compiled
here, so dead code, unused imports and platform-specific test premises are
invisible until a macOS or Linux runner compiles them. Running the CI commands
locally does not help — they compile the same platform. A cross-target `clippy`
does not help either: `libsqlite3-sys` compiles C for the target and there is no
`cc` for `x86_64-apple-darwin` or `x86_64-unknown-linux-gnu` on a Windows
machine.

This is not hypothetical. **Every run on this branch failed until `c735a2f`,
including the runs for both accepted tasks** — the acceptance commit's own run
was red — because each handoff recorded the local gate set as "the gates" and
the run was never opened. Four separate causes, in two rounds:

| Cause | Where it showed |
| --- | --- |
| a comment above the shebang in `preflight.sh` (SC1128) | `shellcheck-secondary` |
| a `use` and a helper gated `#[cfg(unix)]` while their only caller excluded macOS | `rust (macos-latest)`, as dead code under `-D warnings` |
| a test under a bare `#[cfg(unix)]` asserting **Linux's** case rule | `rust (macos-latest)` |
| a test whose fixture premise — a drive letter with no path semantics — holds only on Windows | `rust (ubuntu-latest)` |

The last two are one mistake seen twice, and it is the one to remember:
**`#[cfg(unix)]` is a set of platforms, not a statement about the code.** It is
not "where this is used" and not "where this is true", and macOS is in it while
behaving differently from Linux about case.

**The first fix was written from local reasoning and did not work.** It named
three plausible causes, changed three files, and pushed; the same three jobs
failed again — and it *introduced* a fourth error, because the comment written to
explain the SC1128 fix began with the word `shellcheck` and was parsed as a
directive (SC1072/SC1073). Reading `--log-failed` then named all three remaining
causes exactly, and that is what fixed them. The rule is not "think about CI
after pushing"; it is that **the log is the diagnosis and reasoning about it is
not a substitute**.

**Each platform runs a different set of tests, and a green job is not a green
job.** At `9f13f0d` the Windows run had 628 tests and the Linux run 629, and
neither number is a subset of the other: 8 test names exist only on Windows (the
seven in `paths::compare::tests::windows` and one in `fingerprint::git::status`)
and 9 only on Linux. macOS is a third set again — it runs
`…folds_case_on_a_case_insensitive_platform` where the others run neither that
nor `…folds_nothing_on_a_case_sensitive_platform`. Do not compare totals across
platforms as a check that something ran; compare names.

Two consequences worth knowing when reading a run:

- **`cargo test` stops at the first failing target unless it is given
  `--no-fail-fast`**, and a failure in `sure-core --lib` then hides every
  integration target after it, so a run can look like it tested one thing when
  it stopped early. **CI passed no such flag until `P3-T003`, and now does**;
  the local gate set had it all along, so the two command sets differed in the
  one direction that costs a reading. The targets are where the
  platform-specific tests live, which is what made the difference matter: the
  run whose whole job is to say what *this* platform does could return before
  saying it.
- **A green `windows-latest` job says nothing about the other two.** The
  platform that is easiest to satisfy is the one this repository is developed on.

## The release workflow, which is not part of CI

`.github/workflows/release-dry-run.yml` is the one workflow that produces
release *artifacts*, and it is `workflow_dispatch`-only. It is deliberately not
in `ci.yml`: a release build plus a package plus the acceptance corpus's release
gate in front of every push would make the branch's red or green signal depend
on a release decision rather than on the code. The two answer different
questions.

```
gh workflow run release-dry-run.yml --ref claude/v0.1-autonomous
```

It has to exist on the default branch for `workflow_dispatch` to accept a ref at
all, and it does. Its `validate` job is the three-platform test-and-build matrix;
its `package-macos` job (`P15-T005`) builds, packages, checksums and reads back
the macOS Apple Silicon artifact with `scripts/Build-Release.sh`.
`docs/development/RELEASE_PROCESS.md` states what that artifact is.

**Its `package-macos-intel` job has run, and it built and ran the Intel artifact.**
`P15-T006` added it so that one dispatch answers whether an `x86_64-apple-darwin`
artifact can be built and run, and run `35514769749` answered it: job
`106088732392` concluded `success` on a native Intel host, and the Apple Silicon
job in the same run succeeded too. The job runs on `macos-26-intel`, which is the
x64 label in `actions/runner-images`' own image table — as against `macos-latest`,
which that table lists under macOS 26 **Arm64**, and which is why the arm64 job is
on an arm64 machine. The job prints `uname -m`, `RUNNER_ARCH` and `rustc -vV`'s
host rather than leaving the label to be believed, and that step is what says the
machine was `x86_64`. **This is not a release**: both macOS archives are
`actions/upload-artifact` workflow artifacts and neither is published. The
boundary, and the command that reproduces the measurement, are written down in
`docs/development/RELEASE_PROCESS.md`,
`### What the macOS Intel archive is, concretely`.

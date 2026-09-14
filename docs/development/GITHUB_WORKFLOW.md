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
| `rust` × windows, macos, ubuntu | `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` |
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

This is not hypothetical. CI was red from the bootstrap commit through two
accepted tasks — a comment above a shebang, a helper gated more widely than its
only caller, and an assertion whose premise holds on Windows and not on Unix —
because each handoff recorded the local gate set as "the gates" and the run was
never opened.

Two consequences worth knowing when reading a run:

- **`cargo test --workspace` in CI has no `--no-fail-fast`**, so the first
  failing target aborts the rest. A failure in `sure-core --lib` hides every
  integration target after it, which can make a run look like it tested one thing
  when it stopped early.
- **A green `windows-latest` job says nothing about the other two.** The
  platform that is easiest to satisfy is the one this repository is developed on.

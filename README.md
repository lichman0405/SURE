# SURE

**Software Understanding & Reality Evaluation**

> **AI says it's done. Be SURE.**

[English](README.md) · [中文](README.zh-CN.md)

SURE checks software that was built with AI coding tools, and tells you what is
actually true about it — in plain language, with evidence behind every claim.

It answers three questions:

1. **Can this project actually work?**
2. **Is anything clearly broken, unsafe, fake or incomplete?**
3. **Can the AI's claims about what it completed be confirmed?**

When SURE finds a real problem, it explains the consequence, writes a bounded
repair contract you can hand back to your coding agent, and checks again after
the repair. When it cannot confirm something, it says "cannot confirm" instead
of guessing. **A false "all clear" is treated as more serious than a visible
error.**

SURE works beside the tool you already use. Claude Code users keep using Claude
Code. Cursor users keep using Cursor. Codex users keep using Codex.

## Status: working v0.1, no download yet

Read this before anything else.

- **The v0.1 development plan is complete** — 204 of 204 planned tasks, each
  verified. The checker runs, the CLI works, and the checks are real.
- **There is nothing to download.** No GitHub release, no `winget` package, no
  installer you can fetch. The only way to get SURE today is to build it from
  source, below.
- **It is a bootstrap build.** `sure version` prints `0.0.0-bootstrap`. It is an
  honest v0.1, not a finished product. `FINAL_REPORT.md` lists exactly what it
  can and cannot do, with measurements.

## Install on Windows

You need two things on the machine that builds SURE:

- **Rust** — install [rustup](https://rustup.rs); the repository's
  `rust-toolchain.toml` pins the version, so rustup installs the right one for
  you.
- **Visual Studio Build Tools** with the "Desktop development with C++"
  workload.

Then, from the root of this repository, in PowerShell:

```powershell
# 1. The release gate — SURE's own acceptance tests. Takes a few minutes.
cargo test -p sure-core --test acceptance_report_runner

# 2. Build and package the archive. Also a few minutes.
& .\scripts\Build-Release.ps1 -Phase All

# 3. Install for your user account. No administrator, no PATH change.
& .\scripts\Install-Sure.ps1 -Archive .\target\tmp\release\sure-0.0.0-bootstrap-x86_64-pc-windows-msvc.zip

# 4. Check that it works.
& "$env:LOCALAPPDATA\SURE\bin\sure.exe" version
```

The program lands at `%LOCALAPPDATA%\SURE\bin\sure.exe`. To remove it later:
`& .\scripts\Uninstall-Sure.ps1` — it never deletes `sure.db`, the file where
your check history lives.

The full walk is `docs/development/QUICKSTART_WINDOWS.md`; the installer
reference is `docs/development/INSTALL_WINDOWS.md`.

## macOS and Linux

No published archives here either — build from source:

```sh
cargo install --path crates/sure-cli   # installs `sure` into ~/.cargo/bin
```

That is also where the Unix launcher scripts look for it. The Rust core is
portable and CI-tested on both platforms; `docs/development/MACOS.md` and
`docs/development/LINUX.md` state the support boundaries.

## Use it

```powershell
# Is SURE itself set up correctly on this machine?
& "$env:LOCALAPPDATA\SURE\bin\sure.exe" doctor

# Check a project. The path must be absolute.
& "$env:LOCALAPPDATA\SURE\bin\sure.exe" check "C:\path\to\your\project"
```

Read the exit status as the answer:

| status | meaning |
| --- | --- |
| `0` | clean — every check that ran passed, and nothing was skipped |
| `1` | checked, and not clean — findings, or not enough could be checked |
| `5` | the run did not finish — bad path, unreadable project |
| `2` | the command line itself was wrong |

**Expect `1` on your first runs.** With no AI model configured — the default —
SURE runs its deterministic checks but marks the model-assessment stage "not
checked", and a run with any stage not checked is never reported as clean. That
is the tool being honest, not your project failing.

Other commands worth knowing:

- `sure repair` — turn findings into a repair contract for your coding agent
- `sure recheck` — check again after a repair, and compare with last time
- `sure history` — see (and delete) what SURE has recorded on this machine
- add `--format json` to any command for machine-readable output

A first check writes nothing to your machine: SURE opens your history only if
one already exists.

## Connect it to your AI coding tool

SURE can also record a coding session (only with your consent) and compare the
AI's "done" claims against what actually happened. That needs the plugin for
your harness:

```powershell
# Claude Code — copies the plugin to %LOCALAPPDATA%\claude-plugins\sure
& .\integrations\claude-code\scripts\install.ps1 -ForceCopy
```

Loading the plugin into Claude Code is Claude Code's own step;
`integrations/claude-code/README.md` explains it. Cursor and Codex packages
follow the same layout — see `docs/integrations/INSTALLATION_MATRIX.md`. The
Copilot package is a template today; nothing installs it.

No plugin? The CLI still checks projects. Claim-checking then reports "cannot
confirm", which is the honest answer rather than a missing feature.

## Where SURE keeps things

`%LOCALAPPDATA%\SURE\` on Windows — the program, and `sure.db`, your evidence
history. SURE is local-first: everything stays on your machine and nothing is
sent anywhere.

## Read more

- `FINAL_REPORT.md` — what this v0.1 is and is not, with measurements
- `docs/` — architecture, security, integrations, development
- `START_HERE.md` — if you want to work on SURE itself

## License

Apache-2.0. See `LICENSE`.

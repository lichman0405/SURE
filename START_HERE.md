# START HERE — SURE autonomous development on Windows

Repository: `https://github.com/lichman0405/SURE.git`

# SURE

**Software Understanding & Reality Evaluation**

> **AI says it's done. Be SURE.**

This package is the complete autonomous-development bootstrap for SURE v0.1. It is designed to be handed to Claude Code and implemented from bootstrap through a release candidate with durable progress tracking.

## Primary local development platform

- Windows 11 x64
- PowerShell 7+
- Git for Windows
- Rust 1.98.1 stable via rustup, MSVC host
- Visual Studio 2022 Build Tools (Desktop development with C++) + Windows SDK
- Node.js 24 LTS
- Claude Code
- Cursor for integration smoke tests
- Codex optional for integration smoke tests
- Docker Desktop / Podman optional for isolated project execution tests

The product remains cross-platform. Windows is the primary local development and release-validation environment; macOS and Linux remain mandatory Rust-core CI targets.

## 1. Check the Windows machine

Open **PowerShell 7** in the extracted repository:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\Test-SureEnvironment.ps1
```

Read:

```text
diagnostics\sure-env-report.md
```

`CORE_READY=True` is required before autonomous development. `FULL_DEV_READY=True` is preferred for integration work.

The checker performs a real Rust compile/link smoke test. It also checks Git behavior with spaces in paths, Node 24, Claude Code, optional Cursor/Codex/Docker, and Windows-specific path/toolchain concerns.

## 2. Validate/bootstrap the repository

```powershell
.\scripts\Bootstrap-Sure.ps1
```

This validates the package, task DAG, bootstrap Rust workspace and development prerequisites. It does not silently push anything.

## 3. Populate the GitHub repository

Canonical repository:

```text
https://github.com/lichman0405/SURE.git
```

For the first explicit publish:

```powershell
.\scripts\Publish-Bootstrap.ps1
```

The script does **not** force-push. If the remote gains incompatible history, it stops.

It creates/uses:

```text
main
claude/v0.1-autonomous
```

Autonomous implementation happens on `claude/v0.1-autonomous`.

## 4. Start Claude Code

```powershell
claude
```

Then:

```text
/build-sure
```

Later sessions resume with:

```text
/resume-sure
```

Claude reconstructs state from Git + `progress/state.json` + `progress/HANDOFF.md` + the task graph. It must not rely on chat memory.

## Important

SURE is not rddev and is not a multi-agent orchestrator.

SURE does not replace Claude Code, Cursor, Codex or Copilot. It checks what those tools build.

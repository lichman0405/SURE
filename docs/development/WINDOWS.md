# Windows development — primary local environment

Windows 11 x64 is the primary local development and manual release-validation environment for SURE v0.1.

## Required

- Windows 11 x64
- PowerShell 7+
- Git for Windows
- rustup + Rust 1.98.1 stable
- Rust host `x86_64-pc-windows-msvc`
- `rustfmt` and `clippy`
- Visual Studio 2022 Build Tools with **Desktop development with C++**
- Windows 10/11 SDK
- Node.js 24 LTS + npm
- Claude Code

## Recommended

- GitHub CLI `gh`
- ripgrep (`rg`)
- Cursor
- Codex CLI
- Docker Desktop or Podman Desktop for optional isolated execution tests
- `winget`

Git Bash is useful for cross-platform shell-script checks but is **not** required for the product core or normal Windows development.

## Setup

Use PowerShell 7:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\Test-SureEnvironment.ps1
.\scripts\Bootstrap-Sure.ps1
```

## Native MSVC, not WSL

The canonical Windows build is native:

```text
x86_64-pc-windows-msvc
```

Do not hide Windows-specific bugs by developing only inside WSL.

WSL may later be used to test Linux interoperability, but release acceptance requires native Windows process and filesystem behavior.

## Paths

Test:
- spaces in repository path;
- Unicode path components;
- long path pressure;
- case-insensitive filesystem behavior;
- drive-letter/UNC normalization where relevant;
- project paths under ordinary user-writable directories.

Warn if the developer works inside OneDrive/other sync roots when file locking or rapid temporary-file churn causes problems, but do not ban it without evidence.

Git long-path support should be checked. The project should avoid generating needlessly deep paths.

## Process lifecycle

SURE launches untrusted/AI-written project commands. Windows process-tree cancellation is therefore a product requirement, not a testing detail.

Do not assume Unix signals. Implement and test an explicit Windows process-tree strategy.

## Symlinks

Do not make Windows Developer Mode or administrator privileges a normal requirement.

Plugin/install flows should prefer copy/render/per-user installation. If a development symlink/junction optimization is used, detect capability and provide a no-symlink fallback.

## Hooks

Claude Code explicitly supports PowerShell command hooks on Windows. Cursor command hooks are spawned processes over stdio. Windows integration tests must verify exact invocation, stdin JSON, stdout JSON, timeouts and blocking semantics against installed/current harness versions.

## Data directories

Authoritative evidence/history must use an OS-native per-user application-data directory outside the checked repository. The implementation must expose the exact path via:

```text
sure config paths
sure doctor
```

## Release validation

Before v0.1 release, run a clean native Windows validation covering:

```text
environment check
install
sure doctor
sure check
Claude plugin
Cursor plugin
repair/re-check
uninstall
```

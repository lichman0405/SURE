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

The first line is not decoration and it is not only for the two scripts beside
it. It sets the **process** scope, which is the scope a `cargo test` run needs;
see the next section before you run the test suite from anywhere else.

## Which shell the tests are run from

**The rule, in one sentence.** The session that starts `cargo test` must carry a
process-scoped execution policy that permits scripts, because six of this
workspace's test targets start `powershell.exe` with a `.ps1` path and Windows
PowerShell's own default when no scope sets anything is `Restricted`.

**The rule is not "use PowerShell", and this is the part that costs a reader an
afternoon.** A window of PowerShell 7 opened by hand on a machine in this state
reports an effective policy of `RemoteSigned` and its `powershell.exe` child
*still refuses a local `.ps1`*, because the two hosts default differently and
only a `Process` value is inherited. Measured on 2026-09-21:

```powershell
# In a plain PowerShell 7 window with nothing set for this session:
Get-ExecutionPolicy                      # RemoteSigned
$env:PSExecutionPolicyPreference         # (empty)
powershell.exe -NoProfile -Command 'Get-ExecutionPolicy -List'
# MachinePolicy Undefined / UserPolicy Undefined / Process Undefined
# CurrentUser   Undefined / LocalMachine Undefined   -> Windows PowerShell's default is Restricted
```

A session started with `-ExecutionPolicy Bypass` instead reports
`Process  Bypass` in that same child, and the tests pass. That is measured rather
than assumed: reading the command line of the PowerShell process the tool that
runs the local gates starts, with
`(Get-CimInstance Win32_Process -Filter "ProcessId=$PID").CommandLine`, returns
`pwsh.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command …`. What
travels to a child
is the environment variable `PSExecutionPolicyPreference`, which is where the
process scope lives; it is written to no registry key at any scope and it ends
with the process tree. So `Set-ExecutionPolicy -Scope Process Bypass` in the
session that starts `cargo test` is enough for that session and for everything it
starts, and for nothing outside it.

**What a reader sees without it.** Six targets start Windows PowerShell with a
`.ps1`, and all six fail; measured from a shell with no process-scoped policy on
2026-09-21, on one worktree:

| target | result in that shell |
| --- | --- |
| `sure-testkit --test hook_failure_semantics` | 1 passed, 4 failed |
| `sure-testkit --test integration_thinness` | 49 passed, 5 failed |
| `sure-cli --test install_flow` | 3 passed, 10 failed |
| `sure-cli --test winget_manifest` | 5 passed, 7 failed |
| `sure-cli --test quickstart_flow` | 7 passed, 2 failed |
| `sure-cli --test mcp_protocol` | 24 passed, 1 failed |

**29 failures across six targets**, and every one of them is Windows PowerShell
refusing to load a file:

```text
File C:\...\sure-hook.ps1 cannot be loaded because running scripts is disabled on
this system. For more information, see about_Execution_Policies at
https:/go.microsoft.com/fwlink/?LinkID=135170.
    + CategoryInfo          : SecurityError: (:) [], ParentContainsErrorRecordException
    + FullyQualifiedErrorId : UnauthorizedAccess
```

On a machine whose display language is not English the sentence is localised —
this one reads `无法加载文件 …，因为在此系统上禁止运行脚本。` — so the two
identifiers are the part to read, and they are the part the tests match.

**Those six are the population that was run; the other 60 were searched, and the
difference is worth keeping.** `git grep` over `crates/*/tests/*.rs` finds
`powershell`, `pwsh` or `.ps1` in 16 of the 66 targets, and 10 of those 16 name
one only as text — a path in a manifest, a name in a command-classification
table, a filename in a fixture — with no call site that starts it. The remaining
50 name none at all. So the 60 either drive `sure.exe` directly or use the
library, and that is a statement about their sources rather than a measurement of
them under a shell with no process-scoped policy.

`hook_failure_semantics` no longer reports this as a launcher defect. When the
host refuses, it says so, naming the interpreter, the script, whether the test
process inherited a process-scoped policy, and the scope that fixes it —
`a_host_that_cannot_run_scripts_is_named_rather_than_reported_as_a_launcher_failure`
is the test that holds that wording, and it runs under either shell because it
pins the policy on a child process of its own. The other five targets still
report the raw `UnauthorizedAccess`; a refusal is never *skipped*, in any of
them, because a skipped run still prints `5 passed` and nothing in that line
would say four of the five measured nothing.

**A machine-scoped or user-scoped policy change is not the fix, and must not be
proposed as one.** It would make this refusal invisible on every machine it was
applied to, including machines where it is hiding a real defect — and a check
that cannot report a refusal is the false green this repository treats as more
serious than a visible error. The rule is a process-scoped one or no change at
all.

**Reproducing the red on demand.** From any shell on any machine:

```text
cargo test -p sure-testkit --test hook_failure_semantics -- a_host_that_cannot_run_scripts
```

That test goes red from either shell if the tree stops naming a refusal, because
it makes the refusal happen with `-ExecutionPolicy Restricted` on a process it
owns — a *tightening* for one process that exits with the test, which writes
nothing to any scope and cannot outlive it. Nothing in this repository sets an
execution policy for the machine, for the user, or for the gate.

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

`install` and `uninstall` are `scripts/Install-Sure.ps1` and
`scripts/Uninstall-Sure.ps1`, and `docs/development/INSTALL_WINDOWS.md` is the
procedure: where it installs, what it writes, what it never touches, and what is
not covered by a test. Read it before running either script against a real
`%LOCALAPPDATA%`, because that directory also holds the user's `sure.db`.

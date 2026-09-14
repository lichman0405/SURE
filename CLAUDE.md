# Claude Code repository instructions — SURE

## Identity

SURE = **Software Understanding & Reality Evaluation**.

> AI says it's done. Be SURE.

This repository builds a project truth/checking layer for AI-built software. It is not a coding-agent orchestrator.

## Primary development environment

- Windows 11 x64.
- PowerShell 7+ for repository bootstrap/maintenance scripts.
- Native Rust MSVC (`x86_64-pc-windows-msvc`) with Visual Studio Build Tools + Windows SDK.
- Rust 1.98.1, edition 2024.
- Node.js 24 LTS for bootstrap/plugin tooling.
- Git for Windows.

Do not move the primary implementation into WSL just because a Unix command is easier. Core behavior must work natively on Windows. WSL may be used as an optional interoperability test only.

The Rust core must remain portable to macOS and Linux CI.

## Product priorities

1. Correctness of verdicts.
2. Honest uncertainty.
3. Plain-language UX.
4. Whole-project checks.
5. Safe execution of untrusted/AI-written projects.
6. Harness independence.
7. Local-first privacy.
8. Repair/re-check loop.
9. Performance.

A false green is more serious than a visible error.

## Evidence discipline

Never turn agent text, model output or inferred intent into deterministic fact.

Important claims require evidence anchors. `Cannot confirm` is a valid result.

## Windows engineering discipline

- Test paths with spaces, Unicode and long path pressure.
- Use `std::process::Command`/typed args rather than shell-string construction where possible.
- Process-tree cancellation on Windows is a first-class requirement.
- Do not assume `/bin/sh`, `chmod`, symlink privileges, case-sensitive paths, or Unix signals.
- Do not require Git Bash for the product core.
- Windows user-data/config paths must use OS-native known-folder conventions through a path abstraction.
- Plugin installers must not require administrator rights for normal per-user installation.
- Prefer copy/render/install flows over symlinks on Windows unless Developer Mode/admin capability is explicitly detected.

## Integrations

Integrations are thin. Core checking logic stays in Rust.

Claude Code and Cursor hooks must use current documented hook schemas. Windows hook execution should use PowerShell or direct `sure.exe` invocation as appropriate; do not assume Bash.

## Git / progress

Small task-scoped commits. Task ID in commit message.

Update `progress/state.json` only with truthful status/evidence. Before session handoff/compaction, update `progress/HANDOFF.md`.

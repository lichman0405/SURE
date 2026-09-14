# ADR 0007 — Windows primary development

Status: Accepted

## Decision

Windows 11 x64 with native Rust MSVC is the primary local development and manual release-validation environment for SURE v0.1.

PowerShell 7 is the canonical bootstrap/release scripting shell on Windows. Git Bash/WSL are optional, not required by the core.

Rust core remains cross-platform with macOS and Linux CI/release coverage.

## Why

The primary owner/developer workflow is Windows, and SURE needs correct native Windows process, path, hook and installer behavior. Developing only in WSL or a Unix host would hide exactly the class of cross-platform defects that would affect the intended development workflow.

## Frozen semantics

This is a platform decision, not a vocabulary decision, so it freezes no domain
rule. It does constrain the implementation behind three frozen outcomes:

- Capability tiers must be reported honestly on Windows, where hook and
  pre-action control differ from Unix (`docs/architecture/FROZEN_SEMANTICS.md`).
- User-level data and config paths use OS-native known folders, never a
  Unix-style `~/.config` literal.
- Process-tree cancellation is a real requirement, not a POSIX signal
  assumption.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

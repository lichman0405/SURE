# ADR 0007 — Windows primary development

Status: Accepted

## Decision

Windows 11 x64 with native Rust MSVC is the primary local development and manual release-validation environment for SURE v0.1.

PowerShell 7 is the canonical bootstrap/release scripting shell on Windows. Git Bash/WSL are optional, not required by the core.

Rust core remains cross-platform with macOS and Linux CI/release coverage.

## Why

The primary owner/developer workflow is Windows, and SURE needs correct native Windows process, path, hook and installer behavior. Developing only in WSL or a Unix host would hide exactly the class of cross-platform defects that would affect the intended development workflow.

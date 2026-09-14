# Release process

Initial line: `0.1.x`.

Suggested prerelease progression:
- `v0.1.0-alpha.1`
- `v0.1.0-beta.1`
- `v0.1.0`

## Primary artifact

- Windows x64 (`x86_64-pc-windows-msvc`) CLI/core archive/package
- SHA-256 checksum
- PowerShell per-user install/uninstall flow

A future user-friendly installer may use MSIX/WiX or another appropriate Windows packaging system after CLI behavior stabilizes. Do not add installer complexity before core acceptance.

## Cross-platform artifacts

Also produce/document:
- macOS Apple Silicon CLI archive;
- macOS Intel support/artifact where feasible under the chosen release policy;
- Linux x64 CLI archive;
- SHA-256 checksums;
- Claude Code plugin package/instructions;
- Cursor Plugin package/instructions;
- Agent Plugin/Codex package/instructions.

## Package managers

A WinGet manifest/template is useful after the release artifact is stable. Homebrew formula support is secondary.

## Signing

Authenticode/code-signing credentials are external. If unavailable, document the unsigned state and expected Windows warnings. Do not fake signing.

Apple Developer ID signing/notarization is likewise an optional external credential-dependent enhancement for macOS artifacts.

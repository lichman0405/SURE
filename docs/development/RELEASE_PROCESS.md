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

### What that archive is, concretely

Decided by `P15-T002` and produced by `scripts/Build-Release.ps1`:

| | |
| --- | --- |
| name | `sure-<version>-x86_64-pc-windows-msvc.zip`, where `<version>` is the version `sure` itself reports |
| layout | one top-level directory of the same name, holding `sure.exe`, `LICENSE` and `RELEASE.txt` |
| checksum | `<archive>.sha256` beside it: one `sha256sum`-format line, `<64 lowercase hex><two spaces><file name>`, ASCII, no BOM |
| where | `target/tmp/release/` by default. `target/` is gitignored, so no binary is committed |
| signature | none. See `## Signing` below |

`RELEASE.txt` travels inside the archive and carries the commit it was built
from, whether the worktree was clean at the time, the `rustc` that built it, the
SHA-256 of `sure.exe`, and the release gate's decision.

The checksum is **over the artifact**. It is deliberately not an entry in
`SHA256SUMS.txt`: that file is a curated manifest over a subset of the source
tree, it names no `target/` path and no `sure.exe`, and nothing in this
repository generates or verifies it, so an artifact digest placed there would
look like assurance and would not be.

The archive is **not byte-for-byte reproducible**. A ZIP records a timestamp per
entry, so two builds of the same commit produce two different archives with two
different digests; measured 2026-09-19 over one unchanged checkout, every run
produced a digest no earlier run had. The `.sha256` is an integrity check over
the bytes that were shipped — that they arrived unchanged — and it is not a
claim that a rebuild would produce them again.

`scripts/Build-Release.ps1` will not package this checkout unless
`target/tmp/release-gate.json` exists and reads `permitted`; that document's own
`contract` field is the rule. It then extracts the archive to a fresh directory
and runs the extracted `sure.exe` from there, checking the `running_from` the
binary reports against the directory it extracted to, so that "the artifact was
tested" means the bytes in the archive were run rather than a debug build that
happened to be on `PATH`.

`P15-T003` installs this archive, per user and without administrator rights, and
`docs/development/INSTALL_WINDOWS.md` is where that flow is written down: the
install location, the checksum, the unsigned-build warning, what is and is not
put on `PATH`, and what the uninstaller does with the user's evidence. `P15-T004`
references it.

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

`P15-T004` writes that template: `packaging/winget/template/` holds the three
files, `scripts/New-WingetManifest.ps1` renders them from a release archive and
refuses any value it cannot derive from those bytes, and
`docs/development/INSTALL_WINGET.md` records what the package would install,
where each value comes from, which parts of the update process are manual, and
what none of it covers. **Nothing is published**: submitting a manifest to
`microsoft/winget-pkgs` is a pull request against a repository this project does
not own, and no package exists there.

## Signing

Authenticode/code-signing credentials are external. If unavailable, document the unsigned state and expected Windows warnings. Do not fake signing.

Apple Developer ID signing/notarization is likewise an optional external credential-dependent enhancement for macOS artifacts.

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
- macOS Apple Silicon CLI archive — see `### What the macOS Apple Silicon archive is, concretely`, below;
- macOS Intel support/artifact where feasible under the chosen release policy;
- Linux x64 CLI archive;
- SHA-256 checksums;
- Claude Code plugin package/instructions;
- Cursor Plugin package/instructions;
- Agent Plugin/Codex package/instructions.

### What the macOS Apple Silicon archive is, concretely

Decided by `P15-T005` and produced by `scripts/Build-Release.sh`. That script is
a POSIX shell script rather than a second PowerShell script because the artifact
is built on macOS and Linux runners, where `scripts/Build-Release.ps1` cannot
run at all: it is `ZipFile::CreateFromDirectory`, `%LOCALAPPDATA%`, a `sure.exe`
and a MAX_PATH guard. The workflow invokes it as `sh scripts/Build-Release.sh`
and not by path: this file's mode in the git index is 100644, like every other
`.sh` in the repository, because Git on Windows does not record the executable
bit, so on a runner it is a text file whose shebang nothing acts on.

| | |
| --- | --- |
| name | `sure-<version>-aarch64-apple-darwin.tar.gz`, where `<version>` is the version `sure` itself reports |
| layout | one top-level directory of the same name, holding `sure`, `LICENSE` and `RELEASE.txt` |
| checksum | `<archive>.sha256` beside it: one `sha256sum`-format line, `<64 lowercase hex><two spaces><file name>`, ASCII, no BOM |
| where | `target/tmp/release/` by default. `target/` is gitignored, so no binary is committed |
| mode | `sure` is recorded as mode 755. A tar records the mode as a field of the format, which is why this artifact is not a ZIP |
| signature | none. See `## Signing` below |
| produced by | `.github/workflows/release-dry-run.yml`, job `package-macos`, on a `macos-latest` runner |

The two platforms' archives do not have to be the same shape: a Windows user
expects a ZIP and `[System.IO.Compression.ZipFile]` writes one, and a macOS user
expects a tarball. What is the same is the naming, the single top-level
directory, the three files inside it, and the `.sha256` beside it.

**The architecture is read from the artifact and never from the runner's name.**
`macos-latest` is believed to be an arm64 image; the label's meaning has changed
before and will change again, and a belief is not a measurement. So the build
names its target explicitly (`--target aarch64-apple-darwin`), and the script
then reads the extracted binary's **first eight bytes** — the 4-byte Mach-O
magic and the 4-byte `cputype` — prints them as hex, and refuses anything other
than `cf fa ed fe` / `0c 00 00 01`. A thin x86_64 Mach-O, a universal ("fat")
binary and a PE or ELF image are each refused by name. `/usr/bin/file` is asked
the same question as a second reader where it exists; its absence is reported
rather than passed over, and a disagreement fails the run. The runner's own
`uname -m` is printed too, so a reader can see whether "and it ran" was possible
on that machine at all.

**What "builds/tests" means here is two things.** The workspace's tests are
`cargo test --workspace --no-fail-fast` in the workflow's own step, on the same
runner. The artifact is separately exercised: the script extracts the archive to
a fresh directory and runs the extracted `sure` **from there**, by absolute
path, requiring it to report `running from` **its own path** — that line is the
path of the running executable, not of the directory holding it — and to
describe itself as built for `macos aarch64`. The property that makes it
falsifiable is the one the Windows script states — *the bytes that are
checksummed are the bytes that were run* — and it is established by the order of
the steps, not by a claim.

The release gate applies here exactly as it does on Windows. The document is
`target/tmp/release-gate.json`, written by
`cargo test -p sure-core --test acceptance_report_runner`; on a CI runner
`target/` does not exist until the job creates it, so a `permitted` gate there
is evidence that the acceptance corpus ran **on that runner**.

The macOS artifact is **not byte-for-byte reproducible** for the same reason the
Windows one is not, by a different mechanism: a gzip stream records a
modification time in its header and a tar records one per entry. The `.sha256`
is an integrity check over the bytes that were shipped and is not a claim that a
rebuild would produce them again.

**This artifact has not been signed, notarized, published or downloaded by
anyone.** `scripts/Build-Release.sh` has no signing step and reads the binary's
signature with `codesign -d`, failing only if a signature *chain* appears while
`RELEASE.txt` says there is none; an ad-hoc signature, which an arm64 binary can
carry so the kernel will load it, is not a certificate and does not contradict
that. Nothing in this repository has observed what Gatekeeper does with the
result on a Mac, and `RELEASE.txt` inside the archive says so rather than
guessing.

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

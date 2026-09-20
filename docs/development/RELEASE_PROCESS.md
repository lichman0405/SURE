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
- macOS Intel CLI archive — **not produced, and not claimed.** See
  `### What the macOS Intel archive is, and why none exists yet` below for the
  measurement, the exact boundary of what is therefore true, and the one
  dispatch that would settle it;
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
magic and the 4-byte `cputype` — prints them as hex, and requires exactly the
pair the named target calls for: `cf fa ed fe` / `0c 00 00 01` for this artifact.
A Mach-O of the other architecture, a universal ("fat") binary and a PE or ELF
image are each refused by name, and the refusal prints the bytes it found against
the bytes it required. `/usr/bin/file` is asked the same question as a second
reader where it exists; its absence is reported rather than passed over, and a
disagreement fails the run. The runner's own `uname -m` is printed too, so a
reader can see whether "and it ran" was possible on that machine at all.

`P15-T006` extended the same script to a second target, `x86_64-apple-darwin`,
so the pair above is the arm64 row of a two-row table rather than a constant.
**That extension produced no artifact** — see
`### What the macOS Intel archive is, and why none exists yet`, which is also
where the four-direction measurement of this check is written down.

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

### What the macOS Intel archive is, and why none exists yet

Decided by `P15-T006`. The short form is the one a reader should leave with:

> **No Intel (`x86_64-apple-darwin`) macOS artifact is produced, published or
> claimed. An Intel Mac cannot run the artifact this project does produce. If
> you are on an Intel Mac, there is nothing here for you yet, and that is a
> measured limitation rather than a roadmap item.**

The rest of this section is what that sentence rests on, and what it does not.

**What was measured, and where the run stopped.** The build was attempted on the
machine that wrote this — Windows 11, native Rust MSVC — with the target
installed:

```
$ cargo build --workspace --release --locked --target x86_64-apple-darwin
cargo:warning=Compiler family detection failed due to error:
  ToolNotFound: failed to find tool "cc": program not found
error occurred in cc-rs: failed to find tool "cc": program not found
$ echo $?
101
$ ls target/x86_64-apple-darwin/release/sure
ls: cannot access '...': No such file or directory
```

The `x86_64-apple-darwin` `std` **is** installed there, so the run got past the
target's own absence and stopped at the C dependency: `rusqlite`'s `bundled`
feature compiles SQLite from its own C source through `libsqlite3-sys`, whose
`build.rs` invokes `cc`, and a Windows host has no C compiler that emits x86_64
Mach-O objects. **This is a fact about the host, not about the target**, and it
is the same fact `docs/development/GITHUB_WORKFLOW.md` records about
cross-target `clippy`. It is not evidence that the artifact cannot be built; it
is evidence that it cannot be built *here*.

**What was verified here, and what that does and does not buy.** The architecture
check in `scripts/Build-Release.sh` was extended to serve both macOS targets and
is a discriminating check, in four directions, over real Mach-O bytes:

| archive name | `sure`'s first eight bytes | Architecture step |
| --- | --- | --- |
| `...-aarch64-apple-darwin.tar.gz` | `cffaedfe0c000001` | accepted, `Mach-O 64-bit, arm64` |
| `...-x86_64-apple-darwin.tar.gz` | `cffaedfe07000001` | accepted, `Mach-O 64-bit, x86_64` |
| `...-x86_64-apple-darwin.tar.gz` | `cffaedfe0c000001` | refused, *not* `Mach-O 64-bit, x86_64` |
| `...-aarch64-apple-darwin.tar.gz` | `cffaedfe07000001` | refused, *not* `Mach-O 64-bit, arm64` |

The arm64 bytes are the ones `P15-T005`'s CI run shipped; the x86_64 bytes are
genuine linker-produced Mach-O bytes. The arm64 path was also compared against
itself before and after the Intel target was added, over the artifact CI actually
uploaded, and the two outputs are byte-identical. **What this buys is that the
packaging and checking half is ready and known to discriminate. What it does not
buy is an artifact**: no `x86_64-apple-darwin` archive exists, so no checksum of
one exists, and no x86_64 macOS binary has ever been executed by this project.

**What is not measured, stated plainly.**

- **Whether the Intel artifact can be built at all.** The job that would build it
  is in `.github/workflows/release-dry-run.yml` and **has never run**. Its
  existence is not evidence; a green log would be.
- **Whether the runner label queues for this repository.** `macos-26-intel` is
  the x64 label according to `actions/runner-images`' own image table, but
  whether a dispatch on this account obtains one is a property of the repository
  and not of the image. Nothing here has measured it.
- **Anything about running an Intel artifact**, on any machine. No x86_64 macOS
  binary has been executed, so the standard `P15-T005` set for the arm64 artifact
  — *the bytes that are checksummed are the bytes that were run* — is **not met
  for Intel by anything in this repository**, and nothing here should be read as
  if it were.

**The one command that would settle it**, and the reason it has not been run: the
workflow is `workflow_dispatch`-only and needs the commit on the remote, and
dispatching is the supervisor's step rather than a packaging task's.

```
gh workflow run release-dry-run.yml --ref claude/v0.1-autonomous
```

That dispatch answers three questions at once, in one log. The job's first step
prints `uname -m`, `RUNNER_ARCH` and `rustc -vV`'s host, so the log says which
machine actually ran rather than leaving the label to be believed. It prints
`command -v cc`, which is the specific tool whose absence stopped the local
attempt. And `scripts/Build-Release.sh --target x86_64-apple-darwin` runs the
extracted binary **from the directory it was extracted into** and compares the
`running from` it reports, so an x64 runner turns "and it ran" from an assumption
into a reading. If that job lands on an arm64 host instead, `uname -m` says so
and a Rosetta run is visibly weaker than a native one; the log distinguishes them
rather than the reader having to assume.

**Why the arm64 artifact does not cover Intel Macs.** They are different
architectures and the artifact is a thin `aarch64` binary; an Apple Silicon
translation layer runs x86_64 code on arm64 hardware, not the reverse. This is
stated as the shape of the two architectures rather than as a measurement, for
the reason the rest of this section gives: no Intel Mac has been used anywhere in
this repository, so the honest statement is about which artifacts exist, not
about what one machine did.

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

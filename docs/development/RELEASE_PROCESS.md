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
- macOS Intel CLI archive — **produced and run, not published.** See
  `### What the macOS Intel archive is, concretely` below for the run that does
  it, the exact boundary of what is therefore true, and how to reproduce it;
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
**That extension produces its own artifact**, built, checksummed and run by
`.github/workflows/release-dry-run.yml`'s `package-macos-intel` job — see
`### What the macOS Intel archive is, concretely`, which is also where the
four-direction measurement of this check is written down. Both macOS archives
are `actions/upload-artifact` workflow artifacts and **neither is published**;
that section says what that does and does not mean.

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

### What the macOS Intel archive is, concretely

Decided by `P15-T006`, and produced and run by `.github/workflows/release-dry-run.yml`'s
`package-macos-intel` job. The short form is the one a reader should leave with:

> **This project produces and runs a `x86_64-apple-darwin` macOS artifact, on a
> native Intel host, whenever that job runs. It is not published, signed or
> notarized. No release carries it, and the same is true of the Apple Silicon
> artifact.**

The rest of this section is what that sentence rests on, and what it does not.

**What was measured, and where.** Run `35514769749`, dispatched
2026-09-20T13:52:17Z against head SHA
`a009f57afcb23522afd594a86690bf46763fc4aa`, concluded `success` with all five jobs
green. The job that builds this artifact is `106088732392`. Its *"What this
runner is"* step prints the runner's identity rather than leaving the label to be
believed:

| reading | value |
| --- | --- |
| `uname -s` | `Darwin` |
| `uname -m` | **`x86_64`** |
| `RUNNER_ARCH` | **`X64`** |
| `rustc host` | **`x86_64-apple-darwin`** |
| `rustc release` | `1.98.1` |
| `targets installed` | `x86_64-apple-darwin` |
| `cc` | **`/usr/bin/cc`** |
| `clang` | `/usr/bin/clang` |
| runner image | `macos-26`, version `20260824.0517.1`; macOS 26.6.1 (25G76) |

This is a **native Intel host**. It is not an arm64 runner, and nothing here ran
under Rosetta. Note `cc` in particular: that is the exact tool whose absence
stopped the local Windows-host attempt recorded below.

**What was built, and what was run.** `cargo build --workspace --release --locked
--target x86_64-apple-darwin` exited 0 with `Finished release profile [optimized]
target(s) in 2m 41s`. The staged binary was 10443320 bytes, SHA-256
`c6da9be42e232fd56146b4dfd33e840d63db7c318daafb1f3c2e076ed304e7ed`. The archive
`sure-0.0.0-bootstrap-x86_64-apple-darwin.tar.gz` was 4307011 bytes, and its
`.sha256` was 114 bytes (expected 114), 1 LF, 0 CR, digest
`5b6c6ecc30840c2a0d807ff3e9b6d533e3cf336e4ee26732d48a0ac04ee00ec5`. The layout
was 4 entries, exactly the promised set. Verification read `expected` == `actual`,
`shasum -a 256 -c` agreed, and the archive on disk was the archive the checksum
file names. Extraction produced `same as the sure cargo built`. The extracted
binary's header bytes were `cffaedfe07000001`, which the reader returned as
`Mach-O 64-bit, x86_64`, and `/usr/bin/file` said `Mach-O 64-bit executable
x86_64`. The job then invoked `<extracted>/sure doctor`, which exited 0,
reporting `SURE 0.0.0-bootstrap (harness protocol 1), built for macos x86_64, C
library none`, and reporting `from` its own extracted path. The gate read
`permitted`. The verdict line was:

```
OK  the bytes that are checksummed are the bytes that were run
```

The Apple Silicon job in the same run, `106088732521`, also succeeded and also
printed that verdict line.

**Produced and run — not published.** The job uploads the archive and its
`.sha256` with `actions/upload-artifact@v4` as the workflow artifact
`sure-x86_64-apple-darwin`, retained for 14 days. That is what both macOS
archives are: retained by GitHub, downloadable from the run page by someone with
access to this repository, attached to no release and reachable by no public URL.
`gh release list` on this repository returns nothing and
`gh api repos/lichman0405/SURE/releases --jq length` returns 0, so this repository
has no GitHub Releases. Neither macOS archive has been signed, notarized,
published or downloaded by anyone, and nothing here should be read as saying
otherwise.

**The signature boundary is the same as the Apple Silicon artifact's.** The job's
*Signature* step read `code object is not signed at all` from `codesign -d`, exit
1, authority `none`. The archive is not signed and not notarized, and nothing in
this repository has observed what Gatekeeper does with it on a Mac. That is the
boundary `## Signing` records, and the same one the Apple Silicon section above
states.

**Why the Apple Silicon artifact does not cover Intel Macs.** They are different
architectures and that artifact is a thin `aarch64` binary; an Apple Silicon
translation layer runs x86_64 code on arm64 hardware, not the reverse. That is
the shape of the two architectures rather than a measurement of one Mac, and it
is why the Intel target is a separate job producing a separate archive rather
than a reason to expect the arm64 one to be usable on an Intel host. What an
Intel Mac can use is the artifact this section describes, subject to the
publication, signature and Gatekeeper boundaries it records.

**What was measured here, and what that does and does not buy.** The build was
attempted on the machine that wrote this — Windows 11, native Rust MSVC — with
the target installed:

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
Mach-O objects. **This was always a fact about the host, not about the target**,
and the CI log is what settles it: job `106088732392` found `/usr/bin/cc`, and
the same build there exited 0. It is the same fact
`docs/development/GITHUB_WORKFLOW.md` records about cross-target `clippy` — not
evidence that the artifact cannot be built, only that it could not be built
*here*.

**The architecture check discriminates, in four directions, over real Mach-O
bytes.** The check in `scripts/Build-Release.sh` was extended to serve both macOS
targets and refuses a wrong reading by name:

| archive name | `sure`'s first eight bytes | Architecture step |
| --- | --- | --- |
| `...-aarch64-apple-darwin.tar.gz` | `cffaedfe0c000001` | accepted, `Mach-O 64-bit, arm64` |
| `...-x86_64-apple-darwin.tar.gz` | `cffaedfe07000001` | accepted, `Mach-O 64-bit, x86_64` |
| `...-x86_64-apple-darwin.tar.gz` | `cffaedfe0c000001` | refused, *not* `Mach-O 64-bit, x86_64` |
| `...-aarch64-apple-darwin.tar.gz` | `cffaedfe07000001` | refused, *not* `Mach-O 64-bit, arm64` |

**Both accepted rows are now read off archives this project's own workflow
built.** The arm64 bytes are the ones `P15-T005`'s CI run built and uploaded; the x86_64
bytes are the ones job `106088732392` read off a real linker-produced binary. An
x86_64 little-endian Mach-O beginning `cffaedfe07000001` was believed but not yet
measured when `P15-T006` was first briefed; that is no longer a belief — the job
printed exactly those bytes from the artifact it had just built, so this constant
is read off real bytes in the same way the arm64 one is. The arm64 path was also
compared against itself before and after the Intel target was added, over the
artifact CI actually uploaded, and the two outputs are byte-identical.

**The Intel archive is not byte-for-byte reproducible** either, by the mechanism
the Apple Silicon archive has rather than the Windows ZIP's: a gzip stream
records a modification time in its header and a tar records one per entry. The
`.sha256` is an integrity check over the bytes that were built and uploaded and is
not a claim that a rebuild would produce them again. One Intel build has been measured,
so for this artifact that follows from the format rather than from a second
observation of it.

**What is still not measured, stated plainly.**

- **Whether a second build of the same commit produces the same archive.** One
  Intel build has been measured; the reproducibility statement above rests on the
  archive format's mechanism, not on a repeat.
- **Whether the uploaded workflow artifact survives a download intact.** The job
  built, checksummed and ran the archive on its own runner, and then uploaded it.
  Nobody has downloaded that artifact and re-checked it against its `.sha256`, so
  that round trip is unmeasured — and a 14-day retention window is not a
  distribution channel.
- **What Gatekeeper does with it on a Mac.** The job's own `codesign` reading says
  the binary is unsigned. No launch through Gatekeeper has been observed here; a
  CI runner executing a binary from its own scratch directory is not that
  observation.
- **Whether this artifact behaves the same on an Intel Mac that is not this
  runner image.** The machine was hosted image `macos-26`, version
  `20260824.0517.1`. Nothing here has run the artifact on any other Intel Mac.

**How to reproduce this.** The workflow is still `workflow_dispatch`-only, so the
measurement is reproduced by dispatching it and reading the `package-macos-intel`
job's log:

```
gh workflow run release-dry-run.yml --ref claude/v0.1-autonomous
```

That job's *"What this runner is"* step prints `uname -m`, `RUNNER_ARCH` and
`rustc -vV`'s host, so the log says which machine actually ran rather than leaving
the label to be believed; it prints `command -v cc`, the specific tool whose
absence stopped the local attempt above; and `scripts/Build-Release.sh --target
x86_64-apple-darwin` runs the extracted binary **from the directory it was
extracted into** and compares the `running from` it reports. If a future dispatch
lands on an arm64 host instead, `uname -m` says so, and the log distinguishes a
native run from a Rosetta one rather than the reader having to assume.

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

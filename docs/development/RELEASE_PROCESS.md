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
- Linux x64 CLI archive — **build path in place, not yet produced by any run.**
  See `### What the Linux x64 archive is, concretely` below for what the path
  is, what has been verified and where, and what is still unmeasured;
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
error: failed to run custom build command for `ring v0.17.14`
cargo:warning=Compiler family detection failed due to error:
  ToolNotFound: failed to find tool "cc": program not found
error occurred in cc-rs: failed to find tool "cc": program not found
$ echo $?
101
$ ls target/x86_64-apple-darwin/release/sure
ls: cannot access '...': No such file or directory
```

That is an excerpt of a 74-line log, which is at
`target/tmp/p15t006/local-x86_64-apple-darwin-build.txt` on the machine that
wrote this and is not committed. The first line is the log's line 23 — the line
that says *which* crate failed — `Compiling ring v0.17.14` is its line 18, and
the two `cc-rs` lines are its lines 52 and 70 (line 65 repeats line 52; one of
the two is shown), given here without the two-space indent they carry in the log.
The line numbers are named because the omission of line 23 from this excerpt is
what let the wrong crate be named underneath it.

The `x86_64-apple-darwin` `std` **is** installed there, so the run got past the
target's own absence and stopped at the C dependency: a crate's build script
compiles C for this target through `cc-rs`, and a Windows host has no C compiler
that emits x86_64 Mach-O objects. The crate this log names is `ring v0.17.14`.
The workspace does carry the other C a reader of this section may be thinking of
— `rusqlite`'s `bundled` feature compiles SQLite from its own C source through
`libsqlite3-sys` — but `cargo` stops at `ring`'s build script and never compiles
`libsqlite3-sys`. The sentence here used to name that crate, which this log does
not mention. **This was always a fact about the host, not about the target**, and
the CI log is what settles it: job `106088732392` found `/usr/bin/cc`, and the
same build there exited 0. It is the same fact
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

### What the Linux x64 archive is, concretely

Decided and written by `P15-T007`. It is produced by the same
`scripts/Build-Release.sh` as the two macOS archives — one script, three targets,
because the version read, the stage, the three files inside the archive, the
`.sha256` format, the extraction and the run of the extracted binary are the same
for all three and two scripts holding one archive contract are two places for it
to drift — and by a third job in the same workflow, `package-linux`, on an
`ubuntu-latest` runner.

**No run of that job has happened.** The workflow is `workflow_dispatch`-only and
the job was written by a task that could not dispatch it, so the artifact does not
exist. That is a statement about this job's history and not about this job, and
the sentence a reader should leave this section with is:

> **This project's Linux x64 artifact is built, tested, packaged, checksummed and
> run by the `package-linux` job whenever it is dispatched. Until that has
> happened, no Linux archive exists and none is claimed here.**

| | |
| --- | --- |
| name | `sure-<version>-x86_64-unknown-linux-gnu.tar.gz`, where `<version>` is the version `sure` itself reports |
| layout | one top-level directory of the same name, holding `sure`, `LICENSE` and `RELEASE.txt` |
| checksum | `<archive>.sha256` beside it: one `sha256sum`-format line, `<64 lowercase hex><two spaces><file name>`, ASCII, no BOM |
| where | `target/tmp/release/` by default. `target/` is gitignored, so no binary is committed |
| mode | `sure` is recorded as mode 755, as in the macOS archives |
| signature | none, and there is no signing step and no reading to take: Linux has no `codesign` and an ELF has no field a signature lives in. See `## Signing` below |
| produced by | `.github/workflows/release-dry-run.yml`, job `package-linux`, on an `ubuntu-latest` runner |
| `aarch64-unknown-linux-gnu` | refused by name, exit 2. This repository commits to an x86_64 Linux artifact only, and the script will not produce a file named after a platform nothing here builds |

**The architecture is read from the artifact and never from the runner's name.**
`ubuntu-latest` being x86_64 is a belief about a label, and a belief is not a
measurement — the same rule the macOS rows are held to. The Linux reader is where
that rule is easiest to get wrong, so it is worth stating exactly:

| offset | field | x86_64 value | aarch64 value |
| --- | --- | --- | --- |
| 0–3 | magic | `7f 45 4c 46` | `7f 45 4c 46` |
| 4 | `EI_CLASS` | `02` (`ELFCLASS64`) | `02` |
| 5 | `EI_DATA` | `01` (`ELFDATA2LSB`) | `01` |
| 18–19 | `e_machine`, little-endian | `3e00` (`EM_X86_64`) | `b700` (`EM_AARCH64`) |

**The first eight bytes of an ELF are `7f454c4602010100` for an x86_64 image and
for an aarch64 one alike.** A check built by analogy with the macOS eight-byte
read — magic plus a field that names the machine — would accept an arm64 ELF under
a name that says x86_64 and report that it had checked. That is the false green
this reader exists to avoid, and it is why the `e_machine` field at offset 18 is
what the check compares: the script reads **twenty bytes**, prints them as hex,
and compares the four discriminating fields one at a time (`magic`, `EI_CLASS`,
`EI_DATA`, `e_machine`), refusing a mismatch by name and printing found against
required. Offsets 16–17 (`e_type`) are deliberately **not** compared: `0300` is a
PIE and `0200` is not, which is a property of the linker invocation rather than a
promise this repository has made, and a fixed twenty-byte prefix would refuse a
legitimate artifact for a reason that has nothing to do with architecture.
`/usr/bin/file` is asked the same question as a second reader where it exists,
and a disagreement fails the run.

**The machine's glibc is part of the artifact's contract, and the archive says
so.** An ELF built on a CI runner image links that image's GNU C library, so the
oldest distribution it runs on is set by the glibc of the machine that built it —
not by this repository, and not by anything a reader can assume. The
`RELEASE.txt` inside the Linux archive carries a paragraph on this rather than a
version number: it names the mechanism (a dynamically linked `x86_64-unknown-linux-gnu`
binary, not static and not musl, so not Alpine), states that the requirement is
readable **from the binary** as `GLIBC_x.yy` lines among its undefined symbols,
and points at the two readings that give it — `objdump -p` on the extracted
`sure`, and `ldd --version` on the reader's own machine. The `package-linux`
job prints the runner's own `ldd --version` and `/etc/os-release` in its
*"What this runner is"* step, and reads the `GLIBC_` versions off the packaged
binary in a later step, so the log carries both ends of that number. **The floor
itself is not stated here because no artifact has been built to read it off**;
stating a version now would be a guess dressed as a measurement.

**What has been verified, and where.** The build path cannot be finished on the
machine that wrote it — Windows 11, native Rust MSVC — for the reason the Intel
section above records for `x86_64-apple-darwin` and a second one of its own: this
host has the `x86_64-unknown-linux-gnu` `std` installed, so the local build gets
past the target's absence, and then stops in `cc-rs` with

```
error: failed to run custom build command for `ring v0.17.14`
  cargo:warning=Compiler family detection failed due to error: ToolNotFound: failed to find tool "x86_64-linux-gnu-gcc": program not found (see https://… for help)
  error occurred in cc-rs: failed to find tool "x86_64-linux-gnu-gcc": program not found (see https://… for help)
$ echo $?
101
```

(An excerpt of a 75-line log, at `target/tmp/p15t007/local-linux-build.txt` on
the machine that wrote this and not committed. The `https://…` is the log's own
`docs.rs/cc` link, elided here for width, and the two-space indent is the log's
own. The first line is the log's line 26, which is the line that says *which*
crate failed; `Compiling ring v0.17.14` is its line 17, and the two `cc-rs` lines
are its lines 56 and 72, with line 67 repeating line 56. Those numbers are named
for the reason the Intel excerpt above names its own.)

That is a fact about the host and not about the target: `package-linux`'s runner
has that compiler. What **was** verified locally is the half that does not need a
Linux compiler — the reader, against real ELF bytes rather than invented ones:

| archive name | `sure`'s first twenty bytes | Architecture step | bytes |
| --- | --- | --- | --- |
| `...-x86_64-unknown-linux-gnu.tar.gz` | `7f454c4602010100…03003e00` | accepted, `ELF 64-bit, little-endian, x86_64` | **real** — `/bin/ls` copied out of WSL |
| `...-x86_64-unknown-linux-gnu.tar.gz` | `7f454c4602010100…0300b700` | refused, *not* x86_64 | **constructed** — the same file with bytes 18–19 changed `3e00`→`b700`, nothing else touched |
| `...-x86_64-unknown-linux-gnu.tar.gz` | `7f454c4601010161…02002800` | refused, `ELF 32-bit, little-endian, ARM` | **real** — Qualcomm WLAN firmware from a Windows driver store |
| `...-x86_64-unknown-linux-gnu.tar.gz` | `7f454c4601010100…02000300` | refused, `ELF 32-bit, little-endian, i386` | **real** — the same driver store's `m3.bin` |
| `...-x86_64-unknown-linux-gnu.tar.gz` | `cffaedfe07000001…` | refused, `a Mach-O image` | **real** — `P15-T006`'s x86_64 Mach-O |
| `...-x86_64-unknown-linux-gnu.tar.gz` | `4d5a9000…` | refused, `a PE image` | **real** — a PE from `C:\Windows\System32` |

**The arm64 refusal is exercised against constructed bytes and is labelled as
such.** WSL hands out linker-produced x86_64 ELF images and no aarch64 one, and no
package may be installed to obtain one, so two bytes of a real ELF are the
refusal fixture rather than an invented file: the file is otherwise byte-for-byte
`/bin/ls`, `file(1)` calls the result `ARM aarch64`, and the two bytes changed are
exactly the two the check compares. The acceptance direction is exercised against
real bytes, and the two other-machine refusals (`EM_ARM`, `EM_386`) are real
objects that were shipped by someone else.

**What the local runs cannot reach, stated plainly.** The Windows host cannot make
an extracted file executable, so the mode check stops every local run of the
script and the steps after it are unreachable here: the *Signature* step, the
*Run the extracted binary* step and the final verdict line have never run for a
Linux target on any machine. `package-linux` is what runs them. Until it does,
"the extracted `sure` runs on Linux and reports itself built for Linux" is
**unmeasured**, and the acceptance sentence at the top of this section is not yet
earned.

**How to reproduce this.** Dispatch the workflow and read the `package-linux`
job's log:

```
gh workflow run release-dry-run.yml --ref claude/v0.1-autonomous
```

The job's *"What this runner is"* step prints `uname -m`, `RUNNER_ARCH`,
`rustc -vV`'s host, the image's `/etc/os-release` and its `ldd --version`, so the
log says which machine and which C library produced the artifact rather than
leaving the label to be believed; a later step reads the `GLIBC_` versions out of
the packaged binary; and `scripts/Build-Release.sh --target
x86_64-unknown-linux-gnu` extracts the archive to a fresh directory and runs the
extracted `sure` **from the directory it was extracted into**, comparing the
`running from` it reports with its own path — so *the bytes that are checksummed
are the bytes that were run* is established by the order of the steps and not
asserted. The job uploads the archive and its `.sha256` as the workflow artifact
`sure-x86_64-unknown-linux-gnu`, retained for 14 days, like the two macOS ones:
**not published, attached to no release and reachable by no public URL.**

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

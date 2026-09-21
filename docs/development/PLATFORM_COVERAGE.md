# Platform coverage

This page answers one question per platform: **what is covered, by what
instrument, what the most recent reading is, and what none of that establishes.**

Two kinds of claim appear below and they are kept apart everywhere. *A run
passed* is a reading of a machine that ran; a *workflow file says what it does*
is a reading of a file. The first is evidence about a commit; the second is
evidence about intent. `docs/development/RELEASE_PROCESS.md` carries the same
distinction and is where the artifacts themselves are specified — this page
links to it rather than restating it, and where the two disagree the
disagreement is named in `## Where the files disagree with each other`, below,
rather than smoothed over.

Nothing here is a publication claim. **No artifact this project has produced has
been published**, and `docs/development/RELEASE_PROCESS.md`, `## Signing` and
`## The release workflow`, is the authoritative statement of what that means for
a person who downloads one.

## What builds what

| target triple | how it is built | job (`name:`, verbatim) | runner label | runner image actually observed | attached to anything outward-facing? | most recent observed result |
| --- | --- | --- | --- | --- | --- | --- |
| `aarch64-apple-darwin` | `sh scripts/Build-Release.sh --phase all` — no `--target`, because the script's default *is* this triple (see below) | `macOS Apple Silicon artifact (aarch64-apple-darwin)` | `macos-latest` | `macos-26-arm64`, macOS 26.6.2 (25G83) | no — `actions/upload-artifact` as `sure-aarch64-apple-darwin`, 14 days, no public URL | run `35555938432`, job `106199477354`, `success`, 2026-09-21 |
| `x86_64-apple-darwin` | `sh scripts/Build-Release.sh --phase all --target x86_64-apple-darwin` | `macOS Intel artifact (x86_64-apple-darwin)` | `macos-26-intel` | `macos-26`, macOS 26.6.1 (25G76) | no — `actions/upload-artifact` as `sure-x86_64-apple-darwin`, 14 days, no public URL | run `35555938432`, job `106199477470`, `success`, 2026-09-21 |
| `x86_64-unknown-linux-gnu` | `sh scripts/Build-Release.sh --phase all --target x86_64-unknown-linux-gnu` | `Linux x64 artifact (x86_64-unknown-linux-gnu)` | `ubuntu-latest` | `ubuntu-24.04`, Ubuntu 24.04.5 LTS | no — `actions/upload-artifact` as `sure-x86_64-unknown-linux-gnu`, 14 days, no public URL | run `35555938432`, job `106199477500`, `success`, 2026-09-21 |
| `x86_64-pc-windows-msvc` | `scripts/Build-Release.ps1 -Phase All` — **by hand, on a developer's machine.** No workflow job listed here produces it | *(none in `release-dry-run.yml`; `Windows x64 artifact (x86_64-pc-windows-msvc)` exists only in `release.yml`)* | *(none reachable — `release.yml` is not on `origin/main`)* | the machine that ran it: Windows 11, native Rust MSVC | no, on the artifacts that exist: `target/tmp/release/` is gitignored | **no run id.** The only Windows archive on disk was built by hand at `ae3671…`, 2026-09-19; **it has never been built on a runner** |

The first three rows are produced by `.github/workflows/release-dry-run.yml`.
The fourth row is the one worth reading slowly, and the rest of this section is
about it.

### The Windows row

`release-dry-run.yml` contains four jobs — `validate`, `package-macos`,
`package-macos-intel` and `package-linux`. **There is no Windows packaging job
in it**, and that is a measurement rather than an impression: the file's four
`runs-on:` lines are `${{ matrix.os }}`, `macos-latest`, `macos-26-intel` and
`ubuntu-latest`, and its only `windows-latest` is a leg of the `validate`
matrix, which uploads nothing. `scripts/Build-Release.ps1` is named on six lines
under `.github/`, and all six are in `release.yml` — its `package-windows` job
and the comments around it.

So on the evidence of this repository, `x86_64-pc-windows-msvc` is produced in
exactly one place that has ever run: **`scripts/Build-Release.ps1 -Phase All`,
invoked by hand, on a Windows machine, into `target/tmp/release/`.** The archive
that exists on the machine this page was written on is
`sure-0.0.0-bootstrap-x86_64-pc-windows-msvc.zip`, 4045010 bytes, dated
2026-09-19, and its own `RELEASE.txt` — read out of the ZIP without extracting
it — reads:

```
  target        x86_64-pc-windows-msvc
  built from    ae367198370557d7a3d5fcd1a67b28a8fbd94db0
  worktree      clean
  built at      2026-09-19T11:38:04.5250682Z
  built with    rustc 1.98.1 (48a229cea 2026-09-01)
  release gate  permitted; 13 of 20 cases release-blocking, 13 observed
```

Two things follow and both matter. First, that archive was built from
`ae367198370557d7a3d5fcd1a67b28a8fbd94db0` — **not** from `eab162e`, the commit
every other reading on this page is taken at — so it is a reading of a tree this
page is not otherwise describing, and it is stale by construction. Second, the
sentence in `docs/development/RELEASE_PROCESS.md`, `## The release workflow`,
that the Windows packaging path "has only ever been run on the machine
`P15-T002` was written on" is still exactly true today, and this page is the
second place to say so rather than a place that changes it.

**The Windows test coverage is not the Windows artifact coverage.** Windows is
in the CI matrix and in `validate`, on a real `windows-latest` runner
(`windows-2025-vs2026`, Windows Server 2025, 10.0.26100), and those legs run
`cargo test --workspace --no-fail-fast` and `cargo build --workspace
--release`. That is a statement about the code compiling and its tests passing
on Windows. It is not a statement that a Windows **archive** can be built there,
because nothing in either workflow asks for one.

### The three targets that do have a packaging job

Each of the three packaging jobs runs the workspace's tests on the runner
before it packages, and the packaging script refuses to run unless
`target/tmp/release-gate.json` exists and reads `permitted`. On a fresh runner
`target/` does not exist until the job creates it, so a `permitted` gate in the
log is evidence the acceptance corpus ran **on that runner**. The jobs then
build, package, checksum, extract, read the architecture out of the extracted
binary's own header bytes, and **run the extracted binary from the directory it
was extracted into**. `docs/development/RELEASE_PROCESS.md` specifies each
archive and gives the earlier runs' readings; this page does not repeat them.

One detail a reader of the arm64 job's log will meet and should not misread: the
`macOS Apple Silicon artifact (aarch64-apple-darwin)` job passes **no**
`--target`, where its two siblings both do. That is not an omission and not a
host default. `scripts/Build-Release.sh` line 626 reads `TARGET=aarch64-apple-darwin`
— the triple is hard-coded by name, so the target does not drift with whatever
machine runs the script. The effect is the one
`docs/development/RELEASE_PROCESS.md` describes; the mechanism is in the script
rather than on the command line.

## What ran

### Run `35555938432` — `release-dry-run.yml`

| | |
| --- | --- |
| run id | `35555938432`, attempt 1 |
| commit | `eab162e3af51270a5187083003d81511e48da43d` |
| trigger | `workflow_dispatch` |
| created / last updated | `2026-09-21T02:58:55Z` / `2026-09-21T03:05:48Z` |
| conclusion | **`success`** — all six jobs green |

The six jobs, their conclusions, and what each one established:

| job (`name:`) | id | conclusion | what it established |
| --- | --- | --- | --- |
| `validate (macos-latest)` | `106199477433` | `success` | `cargo test --workspace --no-fail-fast` then `cargo build --workspace --release`. **2766 passed, 0 failed, 12 ignored**, across 89 test targets. No artifact. |
| `validate (windows-latest)` | `106199477443` | `success` | the same two commands. **2810 passed, 0 failed, 13 ignored**, 89 targets. No artifact. |
| `validate (ubuntu-latest)` | `106199477512` | `success` | the same two commands, plus `sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0` first — without which the sandboxed-browser tests cannot start. **2768 passed, 0 failed, 12 ignored**, 89 targets. No artifact. |
| `macOS Apple Silicon artifact (aarch64-apple-darwin)` | `106199477354` | `success` | see below |
| `macOS Intel artifact (x86_64-apple-darwin)` | `106199477470` | `success` | see below |
| `Linux x64 artifact (x86_64-unknown-linux-gnu)` | `106199477500` | `success` | see below |

The three counts differ, and that is the expected shape rather than a
discrepancy: `docs/development/GITHUB_WORKFLOW.md` records that each platform
runs a different set of tests and that the totals are not subsets of one
another, so a count is not a check that the right tests ran.

**`macOS Apple Silicon artifact (aarch64-apple-darwin)`, job `106199477354`.**
Its *"What this runner is"* step printed `uname -s Darwin`, `uname -m arm64`,
`RUNNER_ARCH ARM64`, `rustc host aarch64-apple-darwin`, `sh /bin/sh -> /bin/sh`
version `3.2.57(1)-release`, `codesign /usr/bin/codesign`. So the label
`macos-latest` resolved to an arm64 machine and "and it ran" was possible there
— a measurement, not the label's reputation. Its tests step reported the 2766
above. Its *"Build, package, checksum and check the artifact"* step produced
`sure-0.0.0-bootstrap-aarch64-apple-darwin.tar.gz`, 4158254 bytes, sha256
`7e9242c88df1fc8e6db14ee9bd74719f6c6db775ff30362ec41d271ff4edec53`, in 4
entries, with a `.sha256` that agreed with the archive re-read from disk. Its
architecture step read the first eight bytes as `cffaedfe0c000001`,
`Mach-O 64-bit, arm64`. It then ran the extracted `sure doctor` from the
extraction directory; it exited 0 reporting `SURE 0.0.0-bootstrap (harness
protocol 1), built for macos aarch64, C library none`, and `from` its own
extracted path. The gate read `permitted`, and the verdict line was:

```
  OK          the bytes that are checksummed are the bytes that were run
```

Its checksum step asked `shasum -a 256 -c` from the directory the file was
written into — `OK` — and then required the same command to refuse a copy
modified after it was written. It did refuse it: `shasum: WARNING: 1 computed
checksum did NOT match`. The control is why that step is a check rather than a
ceremony.

**The arm64 artifact carries an ad-hoc linker signature, and that is not a
certificate.** Its *Signature* step read, through `codesign -d`:

```
   codesign    CodeDirectory v=20400 size=78648 flags=0x20002(adhoc,linker-signed) hashes=2454+0 location=embedded
   codesign    Signature=adhoc
```

`docs/development/RELEASE_PROCESS.md`, `### What the macOS Apple Silicon
archive is, concretely`, already says an ad-hoc signature "is not a certificate
and does not contradict" the unsigned claim. This run is the reading behind that
sentence, and it is worth noting that the two macOS archives are in **different**
signature states — see the Intel job below.

**`macOS Intel artifact (x86_64-apple-darwin)`, job `106199477470`.** Its
identity step printed `uname -m x86_64`, `RUNNER_ARCH X64`, `rustc host
x86_64-apple-darwin`, `cc /usr/bin/cc`, `clang /usr/bin/clang`. The machine was
**native Intel**, not an arm64 host running the x86_64 shell under Rosetta —
which the job's own comment says is the weaker reading the log has to
distinguish, and the log distinguishes it. Its artifact was
`sure-0.0.0-bootstrap-x86_64-apple-darwin.tar.gz`, 4415512 bytes, sha256
`1d9f473f8047f9d3b49eeebb704f40cc52f102024f4fc78d363c5a322eaf7f54`, 4 entries,
architecture read as `cffaedfe07000001` → `Mach-O 64-bit, x86_64`. The extracted
`sure doctor` exited 0 reporting `built for macos x86_64, C library none`. The
verdict line was the same one. Its *Signature* step read:

```
   codesign    …/sure: code object is not signed at all
   exit        1 (0 with 'Signature=adhoc' is a linker signature, not a certificate)
   authority   none, so the unsigned sentence in RELEASE.txt is what the bytes say
```

**So the two macOS archives are not in one state described twice.** The Intel
artifact is `not signed at all`; the arm64 artifact carries an ad-hoc,
linker-signed signature. Neither is a Developer ID, neither is notarized and
neither is stapled — see `## What is not covered` below — but a sentence that
says "both macOS archives are unsigned" is true of the Intel one and
imprecise about the arm64 one, whose Mach-O does carry a signature block that no
certificate produced.

**`Linux x64 artifact (x86_64-unknown-linux-gnu)`, job `106199477500`.** Its
identity step printed `uname -m x86_64`, `RUNNER_ARCH X64`, `rustc host
x86_64-unknown-linux-gnu`, `cc /usr/bin/cc`, `sh /usr/bin/sh -> /usr/bin/dash`
version `0.5.12-6ubuntu5`, `PRETTY_NAME="Ubuntu 24.04.5 LTS"`, and
`ldd (Ubuntu GLIBC 2.39-0ubuntu8.8) 2.39`. Its artifact was
`sure-0.0.0-bootstrap-x86_64-unknown-linux-gnu.tar.gz`, 4708936 bytes, sha256
`1fa569f63ef832ee3c0875d8900fa0cc476c9137f6be41923d0239c6ffc7953e`, 4 entries.
The header's first twenty bytes were
`7f454c4602010100000000000000000003003e00`, read as `ELF 64-bit,
little-endian, x86_64`. The extracted `sure doctor` exited 0 reporting `SURE
0.0.0-bootstrap (harness protocol 1), built for linux x86_64, C library gnu`,
from its own extracted path; the gate read `permitted`; the verdict line was the
same one. The platform's own `sha256sum -c` answered `OK`, and the modified copy
failed as it must. Its *Signature* step read `not read`, and gave the reason
this repository gives: an ELF carries no signature field and no step of this
build applies one, so the run measured nothing about whether the binary is
signed. **That is a measurement that did not happen, not a pass.**

### Run `35555838167` — `ci.yml` (context, not this page's subject)

| | |
| --- | --- |
| run id | `35555838167` |
| commit | `eab162e3af51270a5187083003d81511e48da43d` |
| trigger | `push` |
| created / last updated | `2026-09-21T02:56:57Z` / `2026-09-21T03:04:14Z` |
| conclusion | **`success`** — all five jobs green |

`rust (macos-latest)` `106199189573`, `rust (ubuntu-latest)` `106199189710` and
`rust (windows-latest)` `106199189730` each ran `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings` and `cargo test --workspace
--all-features --no-fail-fast`. Their test counts were **2766 / 2768 / 2810
passed, 0 failed, 12 / 12 / 13 ignored**, across 89 targets in each — the same
three numbers the `validate` legs of `35555938432` reported for the same commit.
`bootstrap-validate-windows` `106199189656` and `shellcheck-secondary`
`106199189661` concluded `success` and run `cargo test` not at all.

Two readings fall out of comparing those two runs, and both are measurements
rather than expectations. **`--all-features` selects nothing today**: the `ci`
job passes it and the `validate` job does not, and on the same commit and the
same runner images they report the same counts to the test. **And the counts
differ by platform**: 2810 on Windows, 2768 on Linux, 2766 on macOS. A green
matrix job is three different sets of tests, not one set run three times.

**This run is another task's primary evidence.** It is recorded here as context
because the three packaging jobs' tests and the three `validate` legs overlap,
and a reader comparing them needs both. Nothing on this page is built around it.

## What is not covered

Each item below is a limit this page is stating rather than discovering, and
each names the file or the absence that carries it.

**There is no Windows packaging job in `release-dry-run.yml`.** Verified by
reading the file, not inferred: its jobs are `validate`, `package-macos`,
`package-macos-intel` and `package-linux`. The consequence is that the only
artifact this project ships for its **primary** platform —
`docs/adr/0007-windows-primary-development.md` makes Windows primary — is the
one artifact that no run in this repository has ever produced. Every green
`windows-latest` job on this page ran tests and a debug-profile build of the
*crates*; none of them made a ZIP, and none of them could.

**`release.yml` is not on the default branch, so its `workflow_dispatch` cannot
be reached at all.** `git ls-tree origin/main --name-only .github/workflows/`
returns exactly two paths, `ci.yml` and `release-dry-run.yml`. That is the
precondition `release-dry-run.yml`'s own header names — "This workflow has to
exist on the default branch for `workflow_dispatch` to accept a ref at all" —
and by that same rule `release.yml`, which does not exist there, cannot be
dispatched from any ref. This is a stronger statement than the one
`docs/development/RELEASE_PROCESS.md`, `## The release workflow`, makes when it
says the workflow "has never run". It has never run **and cannot currently be
made to run by a dispatch**: the file is on `claude/v0.1-autonomous` and
`origin/main` has no copy, so GitHub has nothing to offer a dispatcher. A
confirmation independent of the tree: `gh workflow list --all` reports `ci`,
`release-dry-run` and `Dependabot Updates`, and **`release` is not among them.**
What follows is that the four-archive draft release, the nine named assets, the
download-download-rehash round trip and the Windows ZIP on a runner are all
still statements about what a file says, and the file is not in a place where
anything can act on it.

**macOS signing: no Developer ID, no notarization, no staple.**
`docs/development/RELEASE_PROCESS.md`, `### macOS: no Developer ID, no
notarization and no staple`, is the authoritative statement of what is missing,
who would have to provide it, and what must not be claimed about it; this page
does not restate it. What this run adds to it is only the two readings quoted
above — `not signed at all` for the Intel artifact and `Signature=adhoc` for the
arm64 one — and those are readings of a `codesign` invocation **on a CI runner**.
**A runner executing a binary out of its own scratch directory is not a launch
through Gatekeeper**, and nothing in this repository has observed what Gatekeeper
does with either archive on a Mac. So what a user actually meets on first launch
is unobserved here. What can be said without observing it is that the artifact
carries no Apple-vouched signature and no ticket, and that a browser download
acquires the `com.apple.quarantine` attribute; the outcome then belongs to
Gatekeeper, on that person's machine, under their settings and their
administrator's policy. **Nothing in this repository asks anyone to turn
Gatekeeper off.**

**The Linux glibc floor is read, and the workflow asserts nothing about it.**
This is worth stating precisely because it is easy to assume the opposite. The
`package-linux` job's step is named *"What this archive asks of its glibc, read
off the binary"*, and the file says of it:

> It is a reading and not a check: nothing here fails the job, because a
> requirement printed to a log is evidence someone can check, while a
> threshold asserted here would be a rule this repository has not decided
> anything must satisfy.

The step's own code matches: it greps `GLIBC_[0-9.]*` out of the packaged
binary, prints the five highest, and prints the sentence `The highest of these
is the oldest glibc that can run this binary.` It has no `exit 1` on any GLIBC
condition. On run `35555938432` it read:

```
GLIBC symbol versions this binary requires, highest last:
  GLIBC_2.30
  GLIBC_2.32
  GLIBC_2.33
  GLIBC_2.34
  GLIBC_2.39
```

So the artifact requires **glibc 2.39 or newer** — the highest of them, and the
runner image's own libc, not a choice this repository made. It does not run on
Ubuntu 22.04 (glibc 2.35) or Debian 12 (glibc 2.36).
`docs/development/RELEASE_PROCESS.md` and `docs/development/LINUX.md` both say
so and both record that lowering the floor is a follow-up that is owed and not
done. The consequence of it being a reading rather than a threshold is this: **a
future runner image that moved the floor would not fail any job.** The number in
the log would simply change, and nothing would redden. That is the intended
design and it is also the thing to know when reading the number.

**A plugin installing is not an editor loading it, on any platform.**
`docs/integrations/INSTALLATION_MATRIX.md` is where this is written down, under
`### Cannot confirm`, and it is not this page's finding to restate: no plugin
was installed into a running session and none was watched loading, on Windows or
anywhere else. The install scripts place a package on disk and have tests; the
load is the harness's step and no harness was run. **This project has not
observed a real editor loading a SURE plugin on any platform**, and no run on
this page changes that — no job in `ci.yml` or `release-dry-run.yml` starts an
editor.

**Other limits the files already admit, each with its carrier.**

- **Zero tags.** `git tag` on this checkout lists nothing, and `release.yml`'s
  one required input is a tag that must already exist. Even on a branch where it
  were dispatchable, it would have nothing to release.
- **No GitHub Releases.** Measured here as well as recorded in
  `docs/development/RELEASE_PROCESS.md`: `gh release list` prints nothing and
  `gh api repos/lichman0405/SURE/releases --jq length` answers `0`, so the only
  outward-facing object in this repository remains the draft release `release.yml`
  would create.
- **Nothing is in a package manager.** No WinGet package, no Visual Studio
  Marketplace extension, no Open VSX extension, no npm package. A WinGet
  manifest *template* exists and is rendered by a script; submitting it is a
  pull request against a repository this project does not own.
- **`SHA256SUMS.txt` is not a release artifact.** It is a curated manifest over
  a subset of the source tree; no release attaches it and no build writes it.
- **The archives are not byte-for-byte reproducible**, by the mechanism each
  format has rather than by a measurement of a repeat.
- **SmartScreen is not measurable here.** It is a Microsoft service driven by
  telemetry this project does not hold.
- **No `aarch64-unknown-linux-gnu` artifact exists or would be produced** — the
  build script refuses that triple by name. The Linux row is x86_64 only.

## What these readings do not establish

**A green CI job on a hosted runner is not a statement about a user's machine.**
Every reading on this page was taken on GitHub-hosted ephemeral images, listed
above by their own identity steps, and each of those images is a specific
machine with a specific OS build, a specific libc and a specific toolchain. None
of them is the reader's machine. A matrix run can establish that a source tree
compiles, that its tests pass, and that a named artifact was produced, checksummed
and executed **on that image**; it cannot establish anything about a machine it
has never run on.

What a run of a matrix can tell a reader, and what it cannot, in the same
vocabulary this repository uses elsewhere: it **can** tell them what the code
does on the three images named in the tables above, at the one commit named, on
the one date named. It **cannot** tell them that the tests it ran are the tests
that would run on their machine — the three platforms run different sets, with
counts of 2810, 2768 and 2766 that are not nested. It **cannot** tell them that
an archive will run on their OS, because the Linux one carries a glibc floor set
by its builder and the macOS ones carry no Apple-vouched signature. And it
**cannot** tell them that a green result here is a green result anywhere else,
because a hosted runner is a machine this project does not control and did not
choose.

The specific things **no run here has ever covered**:

- **Any Windows archive on a runner.** No job builds one; the only one that
  exists was built by hand at a different commit.
- **`release.yml` in any respect.** It has not run, it is not on the default
  branch, and GitHub does not list it as a dispatchable workflow, so no reading
  of a run of it exists or can currently be taken.
- **A launch through Gatekeeper on a Mac**, for either macOS archive.
- **A download of either macOS archive, re-checked against its `.sha256`.** The
  Linux one has been round-tripped once, by hand; both macOS rows are unmeasured
  for that, per `docs/development/RELEASE_PROCESS.md`.
- **Any machine other than the runner images named above.** For the Intel
  artifact, no Intel Mac other than `macos-26` build `20260819.586`.
- **Any distribution whose glibc is older than 2.39**, for the Linux artifact.
- **A real editor loading a SURE plugin**, on any platform.
- **A rebuild producing the same archive bytes**, for any of the four.
- **What happens to the Linux floor when the runner label moves.** GitHub's own
  annotation on run `35555938432` says: *"The ubuntu-latest label will migrate
  to Ubuntu 26 beginning October 19, 2026."* That would change the image, and
  therefore the glibc the artifact is built against. Because the floor is a
  reading and not a threshold, no job would fail when it happens — the log would
  carry a different number and the recorded floor on this page would be out of
  date. That is a prediction about a label this project does not control, not a
  reading, and it is labelled as one.

## Where the files disagree with each other

Named rather than smoothed over, because these are the parts a reader would
otherwise have to find by holding two files side by side.

**`.github/workflows/release.yml` uses a phrasing its own authority forbids.**
`docs/development/RELEASE_PROCESS.md`, `### What must not be written`, rules out
the definite prompt in either direction — "*Windows will warn* over-commits for
the same reason *Windows will not warn* does… The honest form is **may**." The
release-notes heredoc inside `release.yml` says *"Windows will warn about an
unknown publisher"*. **This is a known, pinned exemption rather than an
unreported defect**: `crates/sure-testkit/tests/signing_status.rs` asserts of
that file that it *still* contains the string `Windows will warn`, so that the
exemption from the phrase rules can be deleted when the wording is. The test's
message says so in as many words — "this task reported instead of editing". A
reader should know that a string matching a forbidden phrasing is deliberately
held in place by a passing test.

**A generated archive on this machine carries the older wording.** The
`RELEASE.txt` read out of the local Windows ZIP says *"Windows will show a
SmartScreen or 'unknown publisher' warning the first time it is run"*. The
current packager does not emit that sentence: `scripts/Build-Release.ps1` now
reads *"Windows may show a SmartScreen or 'unknown publisher' prompt"*, and
`scripts/Build-Release.ps1` is one of the four files the phrase rules scan. So
this is not a live defect in the tree — it is a consequence of the archive being
stale, built 2026-09-19 from `ae3671…`, before the wording changed. It is
recorded here because a reader who opens that ZIP will meet the old sentence and
should not conclude the tree still produces it.

**Nothing else on this page was found to disagree.** The one candidate that
looked like a disagreement — the arm64 job passing no `--target` where
`docs/development/RELEASE_PROCESS.md` says the build names its target explicitly
— resolves on reading `scripts/Build-Release.sh` line 626, and is written up
above rather than listed here.

## Every reading's source on disk

The logs behind every quotation above were saved under
`target/tmp/platform-coverage/`, which is gitignored and is not committed. In
each, the job-level log was fetched from the API and then stripped of its ANSI
colour codes and leading timestamps before reading; the strip is the second
command in the block below and changes no reading.

| file | what it holds |
| --- | --- |
| `gh-run-list-release-dry-run.json` | `gh run list --workflow=release-dry-run.yml --limit 10` |
| `main-workflows.txt` | `git ls-tree origin/main --name-only .github/workflows/` |
| `watch-35555938432.txt` | `gh run watch 35555938432 --exit-status`, including GitHub's own annotations |
| `job-106199477354-clean.log` | the arm64 packaging job, run `35555938432` |
| `job-106199477470-clean.log` | the Intel packaging job, run `35555938432` |
| `job-106199477500-clean.log` | the Linux packaging job, run `35555938432` |
| `job-106199477433-clean.log`, `-106199477512-`, `-106199477443-` | the three `validate` legs, run `35555938432` |
| `ci-job-106199189573-clean.log`, `-106199189710-`, `-106199189730-` | the three `rust` legs, run `35555838167` |

Where a log was elided in this page it was elided by selection — the lines
quoted are contiguous where they are shown contiguous — and never rewritten.

## Commands

```bash
# The dispatch was already in flight; this waits for it and exits non-zero if it failed.
gh run watch 35555938432 --exit-status > target/tmp/platform-coverage/watch-35555938432.txt 2>&1; echo "EXIT=$?"

# Every prior dispatch of the same workflow, to check the ones this page cites.
gh run list --workflow=release-dry-run.yml --limit 10 --json databaseId,headSha,status,conclusion,createdAt,event,displayTitle,attempt

# Is release.yml on the default branch? Two paths came back.
git ls-tree origin/main --name-only .github/workflows/

# Is it dispatchable at all? Three workflows came back, and 'release' was not one of them.
gh workflow list --all

# Run-level and job-level conclusions.
gh run view 35555938432 --json databaseId,headSha,status,conclusion,event,createdAt,updatedAt
gh run view 35555938432 --json jobs --jq '.jobs[] | "\(.databaseId)\t\(.name)\t\(.status)\t\(.conclusion)"'
gh run view 35555838167 --json databaseId,headSha,status,conclusion,event,createdAt,updatedAt
gh run view 35555838167 --json jobs --jq '.jobs[] | "\(.databaseId)\t\(.name)\t\(.status)\t\(.conclusion)"'

# The logs. `gh run view --log` refuses while a run is in progress; the job-level API does not
# care about the run's state, so that is what was used for the jobs that finished first.
gh api "repos/lichman0405/SURE/actions/jobs/106199477354/logs" > target/tmp/platform-coverage/job-106199477354-api.log
gh api "repos/lichman0405/SURE/actions/jobs/106199477470/logs" > target/tmp/platform-coverage/job-106199477470-api.log
gh api "repos/lichman0405/SURE/actions/jobs/106199477500/logs" > target/tmp/platform-coverage/job-106199477500-api.log
gh api "repos/lichman0405/SURE/actions/jobs/106199477433/logs" > target/tmp/platform-coverage/job-106199477433-api.log
gh api "repos/lichman0405/SURE/actions/jobs/106199477512/logs" > target/tmp/platform-coverage/job-106199477512-api.log
gh api "repos/lichman0405/SURE/actions/jobs/106199477443/logs" > target/tmp/platform-coverage/job-106199477443-api.log
gh api "repos/lichman0405/SURE/actions/jobs/106199189573/logs" > target/tmp/platform-coverage/ci-job-106199189573.log
gh api "repos/lichman0405/SURE/actions/jobs/106199189710/logs" > target/tmp/platform-coverage/ci-job-106199189710.log
gh api "repos/lichman0405/SURE/actions/jobs/106199189730/logs" > target/tmp/platform-coverage/ci-job-106199189730.log

# Strip ANSI colour and the leading timestamp, then read. Applied to every log above.
sed -e 's/\x1b\[[0-9;]*m//g' -e 's/^[0-9T:.Z-]* //' <in.log> > <out>-clean.log

# Test counts, as the logs print them.
grep -a -o "^test result: ok\. [0-9]* passed; [0-9]* failed; [0-9]* ignored" <clean.log> | awk '{p+=$4; f+=$6; i+=$8} END {print p " passed; " f " failed; " i " ignored (" NR " targets)"}'

# Runner images and OS versions, out of each job's own header.
grep -a -A3 "^##\[group\]Operating System" <clean.log>
grep -a -m1 -E "^Image: " <clean.log>

# The glibc reading, and the step that says it as a reading.
grep -a -B2 -A14 "GLIBC symbol versions this binary requires" <clean.log>

# Which jobs exist, and on which runners. Four runs-on lines, one of them a matrix.
grep -n "runs-on" .github/workflows/release-dry-run.yml

# Does any workflow build the Windows archive, and where is that script named?
grep -rn "Build-Release.ps1" .github/

# The arm64 job's target: not on its command line, hard-coded in the script.
grep -n "TARGET=" scripts/Build-Release.sh

# The local Windows archive, its checksum, and the RELEASE.txt inside it.
ls -l target/tmp/release/*.zip*
cat target/tmp/release/sure-0.0.0-bootstrap-x86_64-pc-windows-msvc.zip.sha256

# The ZIP's contents and its RELEASE.txt, read without extracting anything to disk.
# Git Bash's tar cannot read a ZIP; this is .NET's reader, opened read-only.
```

```powershell
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead("C:\Users\lishi\code\SURE\target\tmp\release\sure-0.0.0-bootstrap-x86_64-pc-windows-msvc.zip")
$zip.Entries | ForEach-Object { "{0}`t{1} bytes" -f $_.FullName, $_.Length }
$e = $zip.Entries | Where-Object { $_.FullName -like "*RELEASE.txt" }
$r = New-Object System.IO.StreamReader($e.Open()); $r.ReadToEnd(); $r.Close()
$zip.Dispose()
```

```bash
# Is there a release, or a tag to release? Nothing, and nothing.
gh release list
gh api repos/lichman0405/SURE/releases --jq length
git tag | wc -l

# The SmartScreen phrasing rules, and the files they scan.
grep -n "SCANNED_FOR_PHRASING\|DEFINITE_PROMPT_CLAIMS" crates/sure-testkit/tests/signing_status.rs
grep -rn "will show a SmartScreen\|SmartScreen will warn\|will warn" scripts/ docs/development/INSTALL_WINDOWS.md packaging/ .github/workflows/release.yml
```

## What this page is not

It is not a release checklist and it is not a support statement. It is a
snapshot of what has been observed at `eab162e`, and its most perishable
entries are the run ids and the glibc floor, both of which move without anything
in this repository deciding to move them. A reader who needs the state of a
platform on a later commit should dispatch `release-dry-run.yml` and read the
job logs, which is what the command block above does.

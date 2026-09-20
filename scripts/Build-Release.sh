#!/usr/bin/env bash
# SURE - build, package and check a macOS release artifact: Apple Silicon
# (aarch64-apple-darwin) or Intel (x86_64-apple-darwin).
#
#   sh scripts/Build-Release.sh --phase all
#   sh scripts/Build-Release.sh --phase all --target x86_64-apple-darwin
#   sh scripts/Build-Release.sh --phase verify --output-dir <dir>
#
# Run from anywhere; every path is derived from this file's own location.
#
# =============================================================================
# Why this file is a shell script and not a second PowerShell script
# =============================================================================
#
# `scripts/Build-Release.ps1` is the Windows artifact's build, and it is
# PowerShell and Windows-only by construction: `ZipFile::CreateFromDirectory`,
# `%LOCALAPPDATA%`, a `sure.exe` and a MAX_PATH guard. `CLAUDE.md` requires the
# Rust core to stay portable to macOS and Linux, and `P15-T005`'s acceptance is
# an aarch64 macOS artifact produced **in CI/release environment**, which means
# on a macOS runner — so the macOS build path cannot be a script that assumes
# Windows, and it cannot be a Windows-only wrapper around one either. A POSIX
# shell script is what runs there: `/bin/sh` on macOS, `sh` on a Linux runner.
# It is written to run under `sh` as well as under `bash` — no arrays, no
# `[[ ]]`, no `mapfile`, no process substitution, no `${var,,}`, and no `${x//y}`
# — because macOS's `/bin/sh` is bash 3.2 in POSIX mode and a Linux runner's
# `/bin/sh` can be `dash`, and a script that needs bash 4 is a script that runs
# on the machine it was written on. The workflow
# invokes it as `sh scripts/Build-Release.sh` rather than by path, for the reason
# `crates/sure-testkit/tests/hook_failure_semantics.rs` gives about the `.sh`
# hook launchers: Git on Windows does not carry the executable bit, so a script
# invoked by path is a script that works on the machine it was written on and
# fails with "Permission denied" on the runner.
#
# =============================================================================
# What a release artifact is, decided here and recorded in RELEASE_PROCESS.md
# =============================================================================
#
# `docs/development/RELEASE_PROCESS.md` states the Windows artifact concretely
# and lists the macOS one as "macOS Apple Silicon CLI archive" with no format,
# no naming and no layout. This script decides those, and the same decision is
# written into that document so a reader has it in a document rather than in a
# script's source:
#
#   artifact   sure-<version>-<target>.tar.gz
#   checksum   sure-<version>-<target>.tar.gz.sha256, one sha256sum-format line
#   layout     one top-level directory, sure-<version>-<target>/, holding
#              sure, LICENSE and RELEASE.txt
#   location   target/tmp/release/ by default
#
# `target/` is gitignored, so nothing binary is committed.
#
# **`tar.gz` rather than `zip`, and the reason is not taste.** A ZIP records a
# file's mode in an "external file attributes" field that is a per-entry
# extension: a writer that does not set it produces an archive whose extracted
# `sure` is not executable, and the extraction is then a file the platform
# refuses to run for a reason that has nothing to do with the build. `tar`
# records the mode as a field of the format, on every writer, and this script
# asserts the extracted file is executable below. The Windows artifact is a ZIP
# because that is what a Windows user expects and what
# `[System.IO.Compression.ZipFile]` writes; the two do not have to match, and
# `P15-T011` (the release workflow) will carry both as what they are.
#
# =============================================================================
# The architecture is established from the artifact, not from the runner
# =============================================================================
#
# **A file named `sure-<version>-aarch64-apple-darwin.tar.gz` is not evidence
# that it holds an ARM binary, and a checksum of it is not evidence that it
# runs.** `macos-latest` is *believed* to be an arm64 image; a belief is not a
# measurement, and the label's meaning has changed before (`macos-13` is Intel,
# `macos-14` and later are Apple Silicon) and will change again. So nothing here
# reads the runner's name:
#
# * the build names its target explicitly — `--target aarch64-apple-darwin` —
#   so the artifact is what the invocation says it is rather than what the host
#   happens to default to, and it would be an arm64 binary even on an Intel
#   runner (Xcode's clang cross-compiles it);
# * the architecture this script *checks* is read from the artifact's own first
#   eight bytes: the 4-byte Mach-O magic and the 4-byte `cputype`. Those are
#   `cf fa ed fe` (64-bit Mach-O, little-endian, the only shape a modern arm64
#   binary has) and `0c 00 00 01` (`CPU_TYPE_ARM64`). The bytes are printed, so
#   a reader can check the reading by hand rather than take it on trust;
# * `/usr/bin/file` is asked the same question as a second reader where it
#   exists, its answer is printed, and a disagreement is a failure. It is not
#   the required reader: the header read above is the measurement, and `file`
#   is a second one, so its absence is reported rather than silently tolerated;
# * the runner's own `uname -m` is printed and carried into the log, so a reader
#   can see whether "and it ran" was possible on this machine at all.
#
# **What a thin header read does not say.** It does not say the file is a whole
# executable, and it is not treated as if it did: the checksum ties the archive
# to the bytes that were built, and the run step below is what says the bytes
# execute. A universal ("fat") binary is refused rather than accepted, because
# the build asks for one thin architecture and a fat binary appearing here would
# mean that invocation did not do what it says.
#
# =============================================================================
# How the version is obtained, and why not by running the artifact
# =============================================================================
#
# `docs/development/RELEASE_PROCESS.md` names the archive after the version
# `sure` itself reports. That value has to be read before the archive is named,
# and the artifact cannot be the thing that reports it: on an Intel runner an
# arm64 binary will not execute, and a packaging step that depends on running
# the artifact is a packaging step that fails on a machine where the artifact is
# perfectly good. So the version comes from `cargo run --bin sure -- --version`
# — a build for **this** host, which can run here — and the shape of that output
# is asserted rather than assumed. `cargo run --bin sure` selects the binary
# named `sure`, which is the same selection `Build-Release.ps1` makes through
# `cargo metadata`, and `crates/sure-cli/tests/cli_contract.rs` holds the two
# spellings of the version to the same number.
#
# =============================================================================
# The release gate, and why this script refuses to package without it
# =============================================================================
#
# `sure_core::release_gate`'s `GATE_CONTRACT` is the repository's own statement
# about packaging: "P15 may build, package or publish a release of this checkout
# only when this document's `decision` is `permitted`." This is a P15 packaging
# task and the sentence is not about Windows, so it is a precondition here too
# and not a report: with no gate document, or with a `blocked` one, this script
# produces no artifact.
#
# The document is written by `cargo test -p sure-core --test acceptance_report_runner`,
# which is part of `cargo test --workspace`. That is what makes the presence of a
# `permitted` gate on a fresh CI checkout evidence that the workspace's tests
# ran **on this runner** rather than merely that someone ran them somewhere:
# `target/` does not exist until the job creates it.
#
# This script reads exactly one field of that document, `decision`. It does not
# reproduce the document's corpus summary the way `Build-Release.ps1` does with
# `ConvertFrom-Json`: this script has no JSON parser, and a hand-rolled reader of
# a nested document is the kind of second implementation that agrees with the
# first one until it does not. The document itself is the place to read the rest
# of it.
#
# =============================================================================
# What "builds/tests" means here, and what it deliberately does not
# =============================================================================
#
# `P15-T005`'s acceptance is "aarch64 macOS artifact builds/tests and checksum
# generated in CI/release environment", and there are two readings of "tests",
# the same two `Build-Release.ps1` separates:
#
#   the workspace's tests pass  - about a *debug* build of the host, and about
#                                 the source tree rather than about the bytes in
#                                 the archive. This script does not run them; the
#                                 workflow does, in its own step, where the log
#                                 says so.
#   the artifact is exercised  - this script extracts the archive to a fresh
#                                 directory and runs `sure doctor` from **there**,
#                                 by absolute path, requiring the binary to
#                                 report `running from` **its own path** — the
#                                 executable the archive was extracted into,
#                                 which is a file inside that directory and not
#                                 the directory itself.
#
# The property that makes the second falsifiable is the one the Windows script
# states: **the bytes that are checksummed are the bytes that were run.** It is
# established by the order of the steps — the digest is written, re-read and
# checked, and only then is the archive extracted — and by a second comparison:
# in `--phase all` the `sure` inside the archive is hashed and compared with the
# one cargo built, so a package that silently contained something else reddens.
#
# =============================================================================
# Falsifiability: what was run on the machine that wrote this, and what was not
# =============================================================================
#
# **No macOS artifact was produced on the machine this was written on, and none
# is claimed.** There is no `aarch64-apple-darwin` `std` installed there and no C
# toolchain for it, so `cargo build --workspace --release --locked --target
# aarch64-apple-darwin` stops with rustc's own `E0463`, "the
# `aarch64-apple-darwin` target may not be installed", and independently with
# cc-rs's `failed to find tool "cc"`; both readings are in the log the script
# names. The run reaches the Build step, prints
# `FAILED: cargo build exited 101`, and exits 1. The half of this file that
# produces an artifact runs on a macOS runner or nowhere.
#
# **`P15-T006` asked the same question of the Intel target and got the same
# answer for a different reason.** `x86_64-apple-darwin` is a value this script
# accepts now, so whether *this* host can produce that artifact was measured
# rather than inherited. The `x86_64-apple-darwin` `std` **is** installed here —
# `rustup target list --installed` lists it — so the run gets past the target's
# own absence and stops at the C dependency: `libsqlite3-sys` compiles bundled
# SQLite through `cc-rs`, which needs a C compiler that emits x86_64 Mach-O
# objects, and a Windows host has none.
#
#   $ cargo build --workspace --release --locked --target x86_64-apple-darwin
#   cargo:warning=Compiler family detection failed due to error:
#     ToolNotFound: failed to find tool "cc": program not found
#   error occurred in cc-rs: failed to find tool "cc": program not found
#   $ echo $?
#   101
#   $ ls target/x86_64-apple-darwin/release/sure
#   ls: cannot access '...': No such file or directory
#
# The full log is `target/tmp/p15t006/local-x86_64-apple-darwin-build.txt` on
# the machine that wrote this and is not committed. So neither macOS artifact is
# built on a Windows host, and for the Intel one the missing piece is a C
# toolchain for the target and not the target itself — the same fact
# `docs/development/GITHUB_WORKFLOW.md` records about cross-target `clippy`.
#
# What *was* run there, so that the unrun half is the only unrun half:
#
# * `sh -n` parses the whole file, so it is a script and not a text file that
#   looks like one.
# * The argument surface: `--help` exits 0, `--phase nonsense` exits 2, and each
#   target refusal exits 2 with its reason named — `x86_64-unknown-linux-gnu`
#   and `aarch64-unknown-linux-gnu` as `P15-T007`'s artifact, whose header is an
#   ELF and not a Mach-O, and an unknown triple by name.
# * The architecture reader, against **real Mach-O bytes** rather than invented
#   ones, in four directions. The installed `x86_64-apple-darwin` `std` supplies
#   a genuine linker-produced Mach-O `sure`; it is refused, with the message
#   naming what was found against what was required. That same binary patched
#   **only** in its 4-byte `cputype` field, `07000001` -> `0c000001`, is
#   accepted, with the eight bytes printed as `cffaedfe0c000001` and `file`'s
#   answer agreeing it is `Mach-O 64-bit arm64`. A real universal ("fat") binary
#   is refused as fat. A real PE is refused as "the Windows artifact shape". So
#   the check is known to refuse three shapes and accept a fourth: it
#   discriminates, and it is not a check that says yes to everything.
# * The architecture reader as a **discriminating** check, in four directions,
#   over archives assembled here from bytes this repository can name. Each row
#   below is one `--phase verify` run; the two middle rows hold bytes of one
#   architecture under the other's name and are the rows that make this a check
#   rather than a spelling:
#
#       archive name          sure's first bytes   Architecture step
#       aarch64-apple-darwin  cffaedfe0c000001     accepted, "Mach-O 64-bit, arm64"
#       x86_64-apple-darwin   cffaedfe07000001     accepted, "Mach-O 64-bit, x86_64"
#       x86_64-apple-darwin   cffaedfe0c000001     refused, "not ... x86_64"
#       aarch64-apple-darwin  cffaedfe07000001     refused, "not ... arm64"
#
#   The arm64 bytes are the real ones `P15-T005`'s CI run shipped, extracted
#   from the archive that run uploaded; the x86_64 bytes are the genuine
#   linker-produced Mach-O named in the target table above. Each refusal prints
#   the bytes it found against the bytes it required, so a reader can see that
#   the two rows differ in the reading and not only in the verdict. Every row
#   exits 1, at the *mode* check and not at this one, for the host reason named
#   below; the Architecture step is read in all four before that happens.
# * The arm64 path, unchanged. `--phase verify` over **the artifact CI actually
#   shipped** — `sure-0.0.0-bootstrap-aarch64-apple-darwin.tar.gz`, 4065656
#   bytes, SHA-256
#   `3308bb2c12aee7008ed7734565775e96cc092a6c46c1b2efaea87e6d2c820c23`, from run
#   `35512934506` — run through this file as it stood at `6226ce8` and again as
#   it stands here, both into the same output directory so that the paths the
#   run prints are the same strings. Stdout, stderr and exit status are
#   **byte-identical**: 39 lines of stdout, 0 lines of stderr, status 1 both
#   times. The refusal direction was compared the same way (aarch64 name over
#   x86_64 bytes, above) and is byte-identical too. That is the measurement
#   behind "the arm64 path is unchanged" rather than a reading of the diff.
# * The arm64 `RELEASE.txt`, unchanged in its **bytes** and not merely in its
#   words. `--phase all` was run through the file as it stood at `6226ce8` and
#   again as it stands here, both over the same arm64 binary via a stand-in
#   `cargo` on `PATH`, and the two `RELEASE.txt` files out of the two archives
#   are **3490 bytes each and identical apart from the `built at` timestamp**,
#   which differs by construction. That is the measurement, not the argument,
#   behind the newline carried inside `SIGNATURE_LOAD_NOTE` below: a
#   parameterisation that rewrapped that paragraph would have reproduced the
#   same words in different bytes, and this run is what says it did not.
# * The Intel **packaging** path, end to end, by the same stand-in `cargo`:
#   `--phase all --target x86_64-apple-darwin` over the real x86_64 Mach-O bytes
#   reached the Release gate, the version, the stage, the `RELEASE.txt`, the
#   package, the checksum write (114 bytes, 1 LF, 0 CR), the entry list, the
#   checksum re-read, the extraction and the architecture read, which returned
#   `cffaedfe07000001` / `Mach-O 64-bit, x86_64` with `file` agreeing, and then
#   stopped at the mode check for the host reason below. **That is what found the
#   `_` defect named above**: before the pattern was widened, this run failed at
#   the checksum-shape check with a message about the archive's own name. What
#   the stand-in does not do is compile anything, so this says the packaging is
#   right and says nothing about whether a real Intel build succeeds.
#
# **One line the arm64 path does execute was changed, and it is named here
# rather than left in the diff.** The checksum-shape pattern below was
# `[0-9A-Za-z.+-]` as inherited from `P15-T005`, which does not admit `_`; the
# target `x86_64-apple-darwin` contains one, so *every* Intel archive would have
# been refused at that check with a message about its own name. The fixture run
# above is what found it: the Intel row failed there, and only there, before the
# pattern gained the character. The arm64 output cannot differ from the change,
# because `aarch64-apple-darwin` contains no `_` and the new class is a strict
# superset of the old one — and that argument is a claim about a character class
# rather than a reading, which is why the byte-identical comparison above is the
# evidence offered for it.
# * The packaging half, end to end, by `--phase verify` over archives assembled
#   by hand: the entry list against the promised set, the digest recomputed from
#   the bytes on disk and cross-checked with a second tool, the extraction, and
#   the read of the extracted file. `--phase verify` does not build, which is
#   why it is the half that runs on this host whole.
# * The Signature and Run steps, reached by temporarily deleting the mode check
#   below — one mutation, reverted, the file's digest after the revert being the
#   one it had before. `codesign` is absent there, so that step printed that the
#   run says nothing about signing: that is the absent-tool branch working, and
#   it is **not** the `Authority=` branch being tested. The Run step was then
#   reached with a Mach-O this host cannot execute and failed loudly and
#   correctly — exit 126, `cannot execute binary file: Exec format error`, and a
#   message attributing it to the host rather than to the artifact.
# * The `running from` comparison, corrected here, exercised **directly** with
#   planted inputs rather than through the Run step, which this host cannot
#   reach at all: the extracted binary answering passes; a `sure` in another
#   directory, and one with another name in the extraction directory, both
#   fail with the same message shape; and a path reached through a Windows
#   directory junction passes after folding, which is the macOS
#   `/var` -> `/private/var` shape made with a real reparse point. What that
#   reading does **not** cover is the comparison running inside a whole macOS
#   job: nothing in this change has been run on a Mac.
#
# What this host cannot exercise at all, named rather than glossed over:
#
# * **The Run step over a correct artifact.** That host is MSYS/Cygwin, where
#   `chmod` is a no-op: an extracted `sure` comes out `-rw-r--r--` even from a
#   tar that records 755, so the mode check below fails for a *correct* archive
#   and nothing after it runs without the mutation above. That is a property of
#   the host. On macOS, where `chmod` is real, the step after that check is the
#   one that says the bytes execute — and that step has not run here.
# * **Building an arm64 artifact here.** There is no `aarch64-apple-darwin` `std`
#   on that machine, so the header patch `P15-T005` used to reach the accept case
#   is still the only way to *build* one locally. It is no longer the only way to
#   exercise the accept path over genuine arm64 bytes: `P15-T006` ran
#   `--phase verify` over the binary inside the archive `35512934506` uploaded,
#   which Apple's own toolchain linked, and the reader returned
#   `cffaedfe0c000001` / `Mach-O 64-bit, arm64` over it with `file` agreeing.
# * **The `Authority=` failure branch of the Signature step**, which needs a
#   signature chain to refuse and a `codesign` to read one with.
# * **Anything about Gatekeeper**, which nothing in this repository has observed.
# * **The Intel artifact's Run step, on any machine.** No x86_64 macOS artifact
#   has been produced by any machine in this repository's history, so no x86_64
#   macOS binary has been executed either. Locally the same two host limits
#   apply at once: MSYS cannot set the extract mode, and a Mach-O cannot execute
#   on Windows at all. Whether an Intel runner can build and run one is the
#   question `.github/workflows/release-dry-run.yml`'s `package-macos-intel` job
#   exists to answer, and **that job has never run**.
# * **Whether an Intel Mac can run anything this project ships today.** The
#   aarch64 artifact is the only macOS artifact that exists, and an Intel Mac
#   cannot execute an arm64 Mach-O. `docs/development/RELEASE_PROCESS.md` states
#   that as the current boundary; it is a statement about which artifacts exist
#   rather than a measurement on an Intel Mac, because there is none here.
#
# A comment claiming the whole file is measured would be the defect this
# repository exists to prevent. So: the build half was not run; the check half
# was; and the Run step has not been run over a genuine artifact.

set -euo pipefail

usage() {
    cat <<'EOF'
SURE - build, package and check a macOS release artifact.

  sh scripts/Build-Release.sh [options]

  --phase all|verify    all (default) builds, packages, checksums and checks.
                        verify re-checks an archive this script already
                        produced and does not build.
  --target TRIPLE       aarch64-apple-darwin or x86_64-apple-darwin (the
                        only two values accepted).
  --output-dir DIR      where the archive, its .sha256 and the scratch
                        extraction go. Default: target/tmp/release.
  --help                this text.
EOF
}

PHASE=all
TARGET=aarch64-apple-darwin
OUTPUT_DIR=''

while [ $# -gt 0 ]; do
    case "$1" in
        --phase)
            [ $# -ge 2 ] || { usage >&2; echo "FAILED: --phase needs a value" >&2; exit 2; }
            PHASE="$2"
            shift 2
            ;;
        --target)
            [ $# -ge 2 ] || { usage >&2; echo "FAILED: --target needs a value" >&2; exit 2; }
            TARGET="$2"
            shift 2
            ;;
        --output-dir)
            [ $# -ge 2 ] || { usage >&2; echo "FAILED: --output-dir needs a value" >&2; exit 2; }
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help | -h)
            usage
            exit 0
            ;;
        *)
            usage >&2
            echo "FAILED: unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

case "$PHASE" in
    all | verify) ;;
    *)
        usage >&2
        echo "FAILED: --phase is $PHASE; it is all or verify" >&2
        exit 2
        ;;
esac

# The script's own location, resolved. `cd -P` and `pwd -P` because on macOS
# `/var` is a symlink to `/private/var` and this repository's scratch directory
# can end up under either spelling; a repository root compared as a string would
# then disagree with itself.
SCRIPT_DIR="$(cd -P "$(dirname "$0")" && pwd -P)"
ROOT="$(cd -P "$SCRIPT_DIR/.." && pwd -P)"

if [ -z "$OUTPUT_DIR" ]; then
    OUTPUT_DIR="$ROOT/target/tmp/release"
fi
case "$OUTPUT_DIR" in
    /*) ;;
    *)
        echo "FAILED: --output-dir must be an absolute path: $OUTPUT_DIR" >&2
        exit 2
        ;;
esac
case "$OUTPUT_DIR" in
    */) OUTPUT_DIR="${OUTPUT_DIR%/}" ;;
esac

GATE_PATH="$ROOT/target/tmp/release-gate.json"
MANIFEST_PATH="$ROOT/evaluation/acceptance-manifest.json"
LOGS="$OUTPUT_DIR/logs"
SCRATCH="$OUTPUT_DIR/scratch"
EXTRACT_ROOT="$SCRATCH/extracted"
STAGE_ROOT="$SCRATCH/stage"

# =============================================================================
# The target, and the facts the artifact has to agree with
# =============================================================================
#
# Two members, for the reason `Build-Release.ps1` gives about its own set: a
# script that accepted any triple would happily produce a file named after a
# platform it never built for. Linux is `P15-T007`'s artifact and is not a value
# here; the refusal below names that rather than leaving a reader to guess.
#
# **The two macOS targets are one code path and one table row apart.** `P15-T006`
# added the second row and changed nothing else: the architecture is read from
# the artifact's own first eight bytes for both, the second reader, the
# extraction, the checksum and the run are the same steps reached the same way,
# and the only values that differ are the four the check compares against. A
# second script for the Intel artifact would have been a second place for those
# reads to drift apart.
#
# **`cffaedfe07000001` was confirmed against real bytes before it was written
# here**, because the supervisor's brief said not to trust it and was right not
# to. The real bytes are the `x86_64-apple-darwin` `std`'s own
# `libstd-*.dylib` — shipped by rustup, linked by Apple's own toolchain, and
# therefore a genuine 64-bit little-endian Intel Mach-O rather than a header
# someone typed:
#
#   $ head -c 8 "$HOME/.rustup/toolchains/1.98.1-x86_64-pc-windows-msvc/
#       lib/rustlib/x86_64-apple-darwin/lib/libstd-62092c68b9eb1b56.dylib" \
#     | od -An -tx1
#    cf fa ed fe 07 00 00 01
#
# and this host's own `file(1)` calls the same bytes `Mach-O 64-bit x86_64
# dynamically linked shared library`. The file's SHA-256 is
# `1cab997f5855c6ca803cad73478108c207180b91c84247960f7c52c6f243c150`, so the
# reading is over a file anyone can name again. `cf fa ed fe` is `MH_MAGIC_64`
# little-endian and `07 00 00 01` is `CPU_TYPE_X86_64` little-endian; both were
# read off the file rather than recalled.
#
# **`EXPECT_ARCH` is not a guess either.** The run step below requires the
# artifact to describe itself as built for `$EXPECT_OS $EXPECT_ARCH`, and those
# two words come from `std::env::consts::OS` and `std::env::consts::ARCH`
# (`crates/sure-core/src/doctor.rs`), which are the `target_os` and
# `target_arch` cfgs of the target the binary was built for. `rustc --print cfg
# --target <T>` prints the strings that will be compiled in:
#
#   x86_64-apple-darwin    target_arch="x86_64"   target_os="macos"
#   aarch64-apple-darwin   target_arch="aarch64"  target_os="macos"
#
# so `macos x86_64` is a measurement of the target rather than a spelling
# chosen here. Note that `file(1)`'s word for the same cputype is different
# again (`arm64`, not `aarch64`), which is why the second reader has its own
# value rather than being compared against `EXPECT_ARCH`.
case "$TARGET" in
    aarch64-apple-darwin)
        EXPECT_OS='macos'
        EXPECT_ARCH='aarch64'
        EXPECT_MACHO_BYTES='cffaedfe0c000001'
        EXPECT_MACHO_DESCRIPTION='Mach-O 64-bit, arm64'
        EXPECT_FILE_ARCH='arm64'
        ARTIFACT_LABEL='macOS Apple Silicon'
        ARTIFACT_MACHINES='Apple Silicon Macs'
        # **The newline in this value is deliberate and is not a typo.** It is
        # the line break the shipped aarch64 `RELEASE.txt` already carries, put
        # inside the value so that parameterising this sentence cannot silently
        # rewrap a paragraph of an artifact `P15-T005` already produced. A
        # value that is one long line would reproduce the same *words* and
        # different *bytes*, and this change's whole claim about the arm64 path
        # is that it is unchanged rather than merely equivalent.
        SIGNATURE_LOAD_NOTE='which the toolchain applies so
that the kernel will load the image'
        ;;
    x86_64-apple-darwin)
        EXPECT_OS='macos'
        EXPECT_ARCH='x86_64'
        EXPECT_MACHO_BYTES='cffaedfe07000001'
        EXPECT_MACHO_DESCRIPTION='Mach-O 64-bit, x86_64'
        EXPECT_FILE_ARCH='x86_64'
        ARTIFACT_LABEL='macOS Intel'
        ARTIFACT_MACHINES='Intel Macs'
        # Not the arm64 sentence: that one gives the *reason* the toolchain
        # applies an ad-hoc signature, and an arm64 Mach-O needs one before the
        # kernel will load it where an x86_64 one is not known here to. What is
        # claimed for the Intel artifact is only what is measured somewhere in
        # this repository — that the reading happens and how to read it.
        SIGNATURE_LOAD_NOTE='which the toolchain may apply; whether macOS requires an Intel binary to carry one
before it will load it has not been measured here'
        ;;
    x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu)
        echo "FAILED: $TARGET is the Linux artifact, which is P15-T007's task and is deliberately" >&2
        echo "not a value this script accepts: this script reads a Mach-O header, and an ELF has" >&2
        echo "a different one. Nothing was built." >&2
        exit 2
        ;;
    *)
        echo "FAILED: unknown target: $TARGET" >&2
        echo "The values this script accepts are aarch64-apple-darwin and x86_64-apple-darwin." >&2
        exit 2
        ;;
esac

# =============================================================================
# Output, and the two rules that make a failure readable
# =============================================================================

step() {
    printf '\n== %s\n' "$1"
}

detail() {
    printf '   %s\n' "$1"
}

fail() {
    printf '\nFAILED: %s\n' "$1"
    exit 1
}

# Run one program with a typed argument array and both streams sent to files.
#
# Returns the exit status; never prints or throws on the text. **Nothing here is
# read out of a pipeline**, for the reason `Build-Release.ps1` records: a
# Windows capture once produced a wrong number about a child process by taking
# it from one. The status is `$?` on the line after the call and nowhere else,
# and the text a reader sees is read back from a file.
#
# `set +e` around the call is deliberate and is the only place this script turns
# it off: without it a failing program would abort the shell before the caller
# could name which program failed and print its stderr.
run_captured() {
    local program="$1"
    local out="$2"
    local err="$3"
    shift 3
    rm -f "$out" "$err"
    set +e
    "$program" "$@" >"$out" 2>"$err"
    local status=$?
    set -e
    return "$status"
}

# A program on PATH, or nothing. Used for the readers that are not required.
have() {
    command -v "$1" >/dev/null 2>&1
}

# -----------------------------------------------------------------------------
# The digest. One value, from one tool, printed with the name of the tool.
# -----------------------------------------------------------------------------
#
# `sha256sum` is not part of a macOS base install — it arrives with GNU
# coreutils — and `shasum` is, so the tool is discovered rather than assumed.
# Whichever is used is printed, because "the digest is X" without "computed by
# Y" is a number with nothing behind it.
#
# The extraction takes everything before the first space of the tool's first
# line: `sha256sum` writes `<hex>  <name>` for a text-mode read and
# `<hex> *<name>` for a binary-mode one, and **which of the two it writes is the
# platform's, not this script's** — on Git for Windows all three tools below
# print the `*` form, measured here, because MSYS opens files in binary mode.
# `openssl dgst -sha256 -r` writes `<hex> *<name>` everywhere. Only the hex is
# taken from these lines, so the separator is not part of any decision.
#
# The value is then required to be exactly 64 lowercase hex characters, so a
# tool that printed something else fails the check rather than producing a
# `.sha256` file holding the wrong shape.
digest_tool=''
second_digest_tool=''

choose_digest_tools() {
    local candidate
    for candidate in sha256sum shasum openssl; do
        if have "$candidate"; then
            if [ -z "$digest_tool" ]; then
                digest_tool="$candidate"
            else
                second_digest_tool="$candidate"
                break
            fi
        fi
    done
    if [ -z "$digest_tool" ]; then
        fail "none of sha256sum, shasum or openssl is on PATH, so no digest can be computed here"
    fi
}

# `digest_of <path> <label>` - sets DIGEST.
#
# `DIGEST_ERROR` is the first line of the tool's stderr when it failed, and
# `DIGEST_FAILS_FATALLY` decides what a failure in the tool means here. It is 1
# for every call whose value is the digest the artifact is named by, and 0 for
# the cross-check below, where a second tool erroring is reported as "this run
# has one measurement" rather than as a failure of the archive.
#
# **Why the cross-check is tolerant, and this is a judgement rather than a
# measurement.** The second tool on a macOS runner is `openssl`, whose
# coreutils-format flag (`dgst -sha256 -r`) is the reason it is used, and
# nothing here can run it: this script was developed on Windows, which has no
# macOS. Making its failure fatal would let one unsupported flag redden the only
# run that produces the artifact — a false red on the one job whose whole
# purpose is to say what the bytes are. A *disagreement* between two tools that
# both answered is still fatal, which is the check that matters, and the tool
# whose value names the artifact is always fatal on failure.
DIGEST=''
DIGEST_ERROR=''
DIGEST_FAILS_FATALLY=1

digest_of() {
    local path="$1"
    local label="$2"
    local out="$LOGS/digest-$label.txt"
    local err="$LOGS/digest-$label.err.txt"
    local status=0
    case "$digest_tool" in
        sha256sum) run_captured sha256sum "$out" "$err" -- "$path" || status=$? ;;
        shasum) run_captured shasum "$out" "$err" -a 256 -- "$path" || status=$? ;;
        openssl) run_captured openssl "$out" "$err" dgst -sha256 -r "$path" || status=$? ;;
    esac
    DIGEST_ERROR=''
    if [ $status -ne 0 ]; then
        DIGEST_ERROR="$(head -n 1 "$err" 2>/dev/null || true)"
        if [ "$DIGEST_FAILS_FATALLY" -eq 1 ]; then
            printf '   %s\n' "stderr      $DIGEST_ERROR"
            fail "$digest_tool exited $status on $path; its stderr is $err"
        fi
        return 1
    fi
    local line
    line="$(head -n 1 "$out")"
    DIGEST="${line%% *}"
    # Upper case would be a different string for the same value, so it is folded
    # rather than accepted: `RELEASE_PROCESS.md` and `sha256sum -c` both say
    # lowercase, and a `.sha256` that a verification tool rejects over its case
    # is a file that does not work for the reason it exists.
    DIGEST="$(printf '%s' "$DIGEST" | tr 'A-F' 'a-f')"
    printf '%s\n' "$DIGEST" >"$LOGS/digest-$label.hex.txt"
    if ! grep -Eq '^[0-9a-f]+$' "$LOGS/digest-$label.hex.txt"; then
        fail "$digest_tool printed a digest this script does not recognise: '$line'"
    fi
    if [ "${#DIGEST}" -ne 64 ]; then
        fail "$digest_tool printed ${#DIGEST} hex characters, and SHA-256 is 64: '$line'"
    fi
}

# The same value from a *different* implementation, where a second one exists.
#
# This is not decoration. Two tools written by different people agreeing on one
# file is a stronger statement than one tool's output, and the class of defect
# it catches is exactly the one a checksum exists to catch: a file that changed
# between the write and the read. `Build-Release.ps1` gets its second reading
# from the clock (write, then re-read) because Windows has one hashing API; here
# there can be two implementations, so there are.
cross_check_digest() {
    local path="$1"
    local label="$2"
    local expected="$3"
    if [ -z "$second_digest_tool" ]; then
        detail "agree       no second digest tool on PATH, so this run has one measurement and not two"
        return 0
    fi
    local saved=''
    saved="$digest_tool"
    digest_tool="$second_digest_tool"
    local read_status=0
    DIGEST_FAILS_FATALLY=0
    digest_of "$path" "$label-second" || read_status=$?
    DIGEST_FAILS_FATALLY=1
    local other=''
    other="$DIGEST"
    digest_tool="$saved"
    if [ "$read_status" -ne 0 ]; then
        detail "agree       $second_digest_tool exited $read_status and produced no reading ($DIGEST_ERROR), so this run has one measurement of the digest and not two"
        return 0
    fi
    if [ "$other" != "$expected" ]; then
        fail "$digest_tool and $second_digest_tool disagree about $path: $expected against $other"
    fi
    detail "agree       $second_digest_tool computed the same digest"
}

# -----------------------------------------------------------------------------
# The architecture, from the file's own first eight bytes
# -----------------------------------------------------------------------------
#
# `od` is the POSIX byte dumper and is in a macOS base install. `-An` drops the
# address column, `-tx1` prints one hex byte per field, `-N8` reads eight bytes.
# The output is written to a file and read back, and the whitespace is stripped
# by `tr` reading from that file rather than from the `od` output on a pipe.
MACHO_BYTES=''
MACHO_DESCRIPTION=''
FILE_READING=''

read_macho_header() {
    local path="$1"
    local out="$LOGS/macho-header.txt"
    local err="$LOGS/macho-header.err.txt"
    if [ ! -f "$path" ]; then
        fail "there is no file at $path to read a Mach-O header from"
    fi
    local status=0
    run_captured od "$out" "$err" -An -tx1 -N8 "$path" || status=$?
    if [ $status -ne 0 ]; then
        fail "od exited $status reading $path; its stderr is $err"
    fi
    MACHO_BYTES="$(tr -d ' \t\r\n' <"$out")"
    if [ -z "$MACHO_BYTES" ]; then
        fail "$path is empty, so it has no Mach-O header and is not an executable"
    fi

    # The first eight characters are the magic; the second eight are the CPU
    # type. Both are read as bytes on disk rather than through any tool's prose.
    local magic=''
    local cpu=''
    magic="$(printf '%s' "$MACHO_BYTES" | cut -c1-8)"
    cpu="$(printf '%s' "$MACHO_BYTES" | cut -c9-16)"

    case "$magic" in
        cffaedfe)
            case "$cpu" in
                0c000001) MACHO_DESCRIPTION='Mach-O 64-bit, arm64' ;;
                07000001) MACHO_DESCRIPTION='Mach-O 64-bit, x86_64' ;;
                *) MACHO_DESCRIPTION="Mach-O 64-bit, cputype 0x$cpu" ;;
            esac
            ;;
        cefaedfe) MACHO_DESCRIPTION='Mach-O 32-bit, which no macOS release artifact is' ;;
        cafebabe) MACHO_DESCRIPTION='a universal (fat) binary, or a Java class file' ;;
        bebafeca) MACHO_DESCRIPTION='a universal (fat) binary, little-endian magic' ;;
        4d5a*) MACHO_DESCRIPTION='a PE image, which is the Windows artifact shape' ;;
        7f454c46) MACHO_DESCRIPTION='an ELF image, which is the Linux artifact shape' ;;
        *) MACHO_DESCRIPTION="no Mach-O magic at all (first bytes: $MACHO_BYTES)" ;;
    esac
}

# `require_macho_arch <path>` - the whole check, in one place, so that both
# phases reach it and neither can be run without it.
require_macho_arch() {
    local path="$1"
    read_macho_header "$path"
    detail "header      $MACHO_BYTES"
    detail "reads as    $MACHO_DESCRIPTION"
    if [ "$MACHO_BYTES" != "$EXPECT_MACHO_BYTES" ]; then
        printf '\n' >&2
        printf 'FAILED: the bytes in %s are not %s.\n' "$path" "$EXPECT_MACHO_DESCRIPTION" >&2
        printf '\n' >&2
        printf '  found     %s (%s)\n' "$MACHO_DESCRIPTION" "$MACHO_BYTES" >&2
        printf '  required  %s (%s)\n' "$EXPECT_MACHO_DESCRIPTION" "$EXPECT_MACHO_BYTES" >&2
        printf '\n' >&2
        printf 'The name of the archive says %s and the header says something else, so the\n' "$TARGET" >&2
        printf 'name would be a claim about bytes that do not carry it. Nothing was run.\n' >&2
        printf 'A universal (fat) binary is refused even when one of its slices is %s: the\n' "$EXPECT_FILE_ARCH" >&2
        printf 'build asks for one thin architecture, so a fat image here would mean the\n' >&2
        printf 'invocation did not do what it says rather than that the artifact is better.\n' >&2
        printf 'The first eight bytes are read directly rather than through a tool, so this\n' >&2
        printf 'reading does not depend on how any tool words its answer.\n' >&2
        exit 1
    fi

    # The second reader, where one exists. Its absence is reported rather than
    # passed over: a machine with no `file` has one measurement of this file and
    # a reader should know that from the log rather than infer it.
    if have file; then
        local out="$LOGS/file-$2.txt"
        local err="$LOGS/file-$2.err.txt"
        local status=0
        run_captured file "$out" "$err" -b "$path" || status=$?
        if [ $status -ne 0 ]; then
            fail "file exited $status on $path; its stderr is $err"
        fi
        FILE_READING="$(head -n 1 "$out")"
        detail "file says   $FILE_READING"
        # `$EXPECT_FILE_ARCH` and not `$EXPECT_ARCH`: `file` words a cputype
        # differently from the triple — it says `arm64` where the triple says
        # `aarch64` — so the second reader is held to the word it actually uses.
        case "$FILE_READING" in
            *"$EXPECT_FILE_ARCH"*) ;;
            *)
                fail "file reads $path as '$FILE_READING', which does not name $EXPECT_FILE_ARCH, and the header read above says $MACHO_DESCRIPTION. Two readers disagree and neither is overridden: $out"
                ;;
        esac
    else
        detail 'file says   (no file on PATH on this runner; the header read above is the measurement)'
    fi
}

# =============================================================================
# Start
# =============================================================================

printf 'SURE release artifact (%s)\n' "$ARTIFACT_LABEL"
printf '  repository  %s\n' "$ROOT"
printf '  target      %s\n' "$TARGET"
printf '  output      %s\n' "$OUTPUT_DIR"
printf '  phase       %s\n' "$PHASE"
printf '  this host   %s %s\n' "$(uname -s)" "$(uname -m)"

# `logs/` is created here, where both phases reach it, and not inside the `all`
# branch. `Build-Release.ps1`'s Mutation 3 is exactly this line moved into the
# branch it was once in: a `verify` into a directory holding only an archive and
# its checksum then died inside the redirection *after* the checksum had matched
# and the archive had extracted, so a good artifact in a fresh directory was
# reported as FAILED. `target/tmp/release` passed only because an earlier `all`
# had left a `logs/` behind in it.
mkdir -p "$LOGS"

choose_digest_tools
if [ -n "$second_digest_tool" ]; then
    detail "digest      $digest_tool, cross-checked with $second_digest_tool"
else
    detail "digest      $digest_tool, and it is the only digest tool on PATH"
fi

# -----------------------------------------------------------------------------
# 1. The release gate. Read first, and before anything is built, because a
#    refusal here must cost nothing and must be the first thing a reader sees.
# -----------------------------------------------------------------------------
step 'Release gate'

if [ ! -f "$GATE_PATH" ]; then
    fail "no release gate at $GATE_PATH, so this checkout has no answer to the question
GATE_CONTRACT makes a precondition of packaging. Nothing was built.

  Write it with:  cargo test -p sure-core --test acceptance_report_runner

A missing gate is \"cannot confirm\", and \"cannot confirm\" is not \"permitted\"."
fi

# The one field this script reads. The document is pretty-printed JSON and
# `decision` is a top-level key, so the line is anchored at both ends: the
# pattern requires a line that *begins* with the key, which is what keeps the
# word out of the contract's own prose — that sentence names `decision` several
# times and not one of those lines starts with the key.
DECISION_LINES="$(sed -n 's/^[[:space:]]*"decision"[[:space:]]*:[[:space:]]*"\([^"]*\)".*$/\1/p' "$GATE_PATH" || true)"
DECISION_COUNT="$(printf '%s\n' "$DECISION_LINES" | grep -c . || true)"
if [ "$DECISION_COUNT" -ne 1 ]; then
    fail "the release gate at $GATE_PATH has $DECISION_COUNT top-level \"decision\" fields and this script reads exactly one. Expected one string field; the document may have changed shape, and a gate this script cannot read is not a gate that passed."
fi
DECISION="$DECISION_LINES"
if [ "$DECISION" != 'permitted' ]; then
    fail "the release gate says this release is $DECISION, and this script packages only a permitted one. The document is $GATE_PATH."
fi
detail 'decision    permitted'
detail "gate file   $GATE_PATH"

# The gate is a reading of `evaluation/acceptance-manifest.json`, and this is a
# *timestamp* comparison and not a content one. `corpus.manifest_digest` cannot
# be checked here: it is a domain-separated digest built by
# `sure_core::fingerprint::digest`, so `sha256sum` over the manifest file is a
# different value, and reimplementing that construction in shell would be a
# hand-rolled copy of a crypto format. So this check can say "the manifest was
# written after the gate" and cannot say "the manifest is the one the gate read".
# The gap is named rather than papered over.
if [ -f "$MANIFEST_PATH" ] && [ "$MANIFEST_PATH" -nt "$GATE_PATH" ]; then
    fail "the acceptance manifest was written after the release gate:

  $MANIFEST_PATH
  $GATE_PATH

The gate is a decision about that manifest's release-blocking cases, so a
manifest newer than the gate means the gate is a reading of a different corpus.
Re-run it:  cargo test -p sure-core --test acceptance_report_runner"
fi
detail 'freshness   the manifest is not newer than the gate (timestamp, not content)'

# -----------------------------------------------------------------------------
# 2. Version, build and stage. `verify` reaches none of this.
# -----------------------------------------------------------------------------

VERSION=''
ARTIFACT_NAME=''
ARCHIVE=''
SHA_PATH=''
BUILT_BINARY=''

if [ "$PHASE" = 'all' ]; then
    step 'Version'

    # See the header: a host build, so that the version can be read on a machine
    # that could never execute the artifact it is naming.
    #
    # `--manifest-path` is required rather than tidy. `cargo` finds a workspace
    # by walking up from the **current working directory**, so a `cargo run`
    # without it reads whatever project the caller happened to be standing in —
    # and the run's own claim is that the artifact is named after the version
    # *this* repository reports. The `--target` build below passes it for the
    # same reason.
    version_out="$LOGS/version.txt"
    version_err="$LOGS/version.err.txt"
    version_status=0
    run_captured cargo "$version_out" "$version_err" run --quiet --bin sure --manifest-path "$ROOT/Cargo.toml" -- --version || version_status=$?
    if [ $version_status -ne 0 ]; then
        printf '   %s\n' "stderr      $(head -n 1 "$version_err" 2>/dev/null || true)"
        fail "cargo run --bin sure -- --version exited $version_status, so this script cannot name the artifact after the version sure reports; stderr is $version_err"
    fi
    version_line="$(head -n 1 "$version_out")"
    case "$version_line" in
        'sure '*)
            VERSION="${version_line#sure }"
            ;;
        *)
            fail "sure --version printed '$version_line', which is not the 'sure <number>' shape this script reads the version out of. Nothing was packaged, because a name built from a line this script does not understand is a name nobody checked."
            ;;
    esac
    case "$VERSION" in
        '' | *[!0-9A-Za-z.+-]*)
            fail "the version read from sure --version is '$VERSION', which is not a version this script will put in a file name"
            ;;
    esac
    detail "version     $VERSION (sure --version on this host)"

    step 'Build'
    detail "cargo build --workspace --release --locked --target $TARGET"
    build_out="$LOGS/cargo-build.txt"
    build_err="$LOGS/cargo-build.err.txt"
    build_status=0
    run_captured cargo "$build_out" "$build_err" build --workspace --release --locked --target "$TARGET" --manifest-path "$ROOT/Cargo.toml" || build_status=$?
    # Both streams are echoed, because `GITHUB_WORKFLOW.md`'s rule is that the
    # log is the diagnosis: a build failure whose text went to a file the reader
    # has to go and find is a failure the run's log does not explain.
    cat "$build_out" "$build_err" >"$LOGS/cargo-build.both.txt" 2>/dev/null || true
    while IFS= read -r line; do
        if [ -n "$line" ]; then
            detail "$line"
        fi
    done <"$LOGS/cargo-build.both.txt"
    if [ $build_status -ne 0 ]; then
        fail "cargo build exited $build_status; the full log is $build_out and $build_err"
    fi
    detail 'exit        0'

    BUILT_BINARY="$ROOT/target/$TARGET/release/sure"
    if [ ! -f "$BUILT_BINARY" ]; then
        fail "cargo build succeeded but $BUILT_BINARY is not there"
    fi

    step 'Stage'
    ARTIFACT_NAME="sure-$VERSION-$TARGET"
    stage_dir="$STAGE_ROOT/$ARTIFACT_NAME"
    rm -rf "$STAGE_ROOT"
    mkdir -p "$stage_dir"

    staged_binary="$stage_dir/sure"
    cp "$BUILT_BINARY" "$staged_binary"
    # `tar` records a mode, and an archive whose extracted binary is not
    # executable is an artifact a user cannot run for a reason that is not the
    # build's. Set it here rather than inherit it: cargo's output mode is the
    # builder's, and the archive should not depend on it.
    chmod 755 "$staged_binary"
    cp "$ROOT/LICENSE" "$stage_dir/LICENSE"

    commit='unknown'
    if have git; then
        commit_out="$LOGS/git-commit.txt"
        commit_err="$LOGS/git-commit.err.txt"
        commit_status=0
        run_captured git "$commit_out" "$commit_err" -C "$ROOT" rev-parse HEAD || commit_status=$?
        if [ $commit_status -eq 0 ]; then
            commit="$(head -n 1 "$commit_out")"
        else
            commit="unknown (git rev-parse exited $commit_status)"
        fi
    fi

    # A commit that does not describe the bytes is worse than no commit at all,
    # so the worktree's state at build time is recorded and the three outcomes
    # are kept apart. In particular a `git status` that *failed* is not "clean":
    # an empty stdout from a git that could not run is the shape a clean tree
    # also has, and reading one as the other is how a dirty build acquires a
    # clean provenance line.
    tree_out="$LOGS/git-status.txt"
    tree_err="$LOGS/git-status.err.txt"
    tree_state='unknown (git is not on PATH)'
    if have git; then
        tree_status=0
        run_captured git "$tree_out" "$tree_err" -C "$ROOT" status --porcelain || tree_status=$?
        if [ $tree_status -ne 0 ]; then
            tree_state="unknown (git status exited $tree_status)"
        elif [ -s "$tree_out" ]; then
            tree_state='dirty - uncommitted changes were present when this was built'
        else
            tree_state='clean'
        fi
    fi

    rustc_out="$LOGS/rustc-vV.txt"
    rustc_err="$LOGS/rustc-vV.err.txt"
    rustc_version='unknown'
    if have rustc; then
        if run_captured rustc "$rustc_out" "$rustc_err" -vV; then
            rustc_version="$(head -n 1 "$rustc_out")"
        fi
    fi

    binary_digest=''
    digest_of "$staged_binary" 'sure-binary'
    binary_digest="$DIGEST"
    binary_bytes="$(wc -c <"$staged_binary" | tr -d ' ')"
    detail "sure        $binary_bytes bytes, SHA-256 $binary_digest"

    # The provenance travels inside the archive rather than beside it: an
    # archive that travels without its RELEASE.txt is the one whose origin
    # nobody can state. The worktree marker is here because a recorded commit
    # that does not describe the bytes is worse than no commit at all.
    cat >"$stage_dir/RELEASE.txt" <<EOF
SURE $VERSION - $ARTIFACT_LABEL release artifact

  target        $TARGET
  built from    $commit
  worktree      $tree_state
  built at      $(date -u '+%Y-%m-%dT%H:%M:%SZ')
  built on      $(uname -s) $(uname -m)
  built with    $rustc_version
  release gate  permitted
                read from $GATE_PATH

  sure          SHA-256 $binary_digest
                $binary_bytes bytes

WHAT THIS IS
The SURE command-line program for $ARTIFACT_MACHINES, built for the
$TARGET target. "sure doctor" reports the build it is running
from and where it is running from; "sure --help" lists the commands.

THIS BUILD CARRIES NO APPLE DEVELOPER ID SIGNATURE AND NOTHING NOTARIZES IT
Neither is an oversight: docs/development/RELEASE_PROCESS.md records Developer
ID signing and notarization as an external credential this project does not
have, and says "Do not fake signing". No step of this build applies one. What
that means is a fact about the build procedure and not a claim about the bytes,
and the bytes are read: scripts/Build-Release.sh runs "codesign -d" on the
extracted binary and prints everything it says. A signing *chain* there - an
Authority= line, which is what a certificate produces - fails the run, because
this file says there is none.

An $EXPECT_FILE_ARCH binary can carry an ad-hoc signature, $SIGNATURE_LOAD_NOTE. That is not a certificate, it names no
developer, and it does not contradict the paragraph above; if the reading says
\`Signature=adhoc\` that is what it is.

What macOS does with a program that carries no Developer ID signature is
Apple's behaviour and not SURE's, and this project has not observed it on a Mac
- the general shape is that Gatekeeper refuses or warns about one whose
com.apple.quarantine attribute is set, which is what a browser download sets
and what a command-line download may not. Nothing in SURE asks you to turn
Gatekeeper off. If a prompt appears, treat it as the unsigned state above and
decide for yourself; do not read this file as saying the build is trusted.

THIS ARCHIVE IS NOT BYTE-FOR-BYTE REPRODUCIBLE
A gzip stream records a modification time in its header and a tar records one
per entry, so building the same commit twice produces two archives with two
different digests. The .sha256 beside this archive is an integrity check over
the bytes that were shipped - that they arrived unchanged - and it is not a
claim that a rebuild would produce them again.

HOW THIS IS CHECKED
This file is written before the checks run, so it describes the procedure
rather than certifying its own archive. scripts/Build-Release.sh writes the
.sha256 beside this archive, re-reads it, verifies the archive against it,
lists the archive's entries and requires them to be the promised set, extracts
the archive to a fresh directory, reads the first eight bytes of the extracted
binary and requires them to be the Mach-O magic and the $EXPECT_FILE_ARCH CPU type, reads
the binary's signature with codesign, and runs the extracted "sure" from there,
comparing the running_from it reports with that directory. It prints OK and
exits 0 only when every one of those steps agreed, so a run that failed is a
run that said which step failed.
See docs/development/RELEASE_PROCESS.md for the artifact contract.
EOF

    step 'Package'
    mkdir -p "$OUTPUT_DIR"
    ARCHIVE="$OUTPUT_DIR/$ARTIFACT_NAME.tar.gz"
    SHA_PATH="$ARCHIVE.sha256"
    rm -f "$ARCHIVE" "$SHA_PATH"
    tar_status=0
    tar_out="$LOGS/tar-create.txt"
    tar_err="$LOGS/tar-create.err.txt"
    run_captured tar "$tar_out" "$tar_err" -czf "$ARCHIVE" -C "$STAGE_ROOT" "$ARTIFACT_NAME" || tar_status=$?
    if [ $tar_status -ne 0 ]; then
        printf '   %s\n' "stderr      $(head -n 1 "$tar_err" 2>/dev/null || true)"
        fail "tar exited $tar_status; its stderr is $tar_err"
    fi
    if [ -s "$tar_err" ]; then
        while IFS= read -r line; do detail "tar warns   $line"; done <"$tar_err"
    fi
    detail "archive     $ARCHIVE"
    detail "size        $(wc -c <"$ARCHIVE" | tr -d ' ') bytes"

    # -------------------------------------------------------------------------
    # The checksum. Computed here, over the archive that was just written, and
    # then checked against its own bytes rather than trusted.
    # -------------------------------------------------------------------------
    step 'Write the checksum'

    sha_name="$(basename "$ARCHIVE")"
    digest_of "$ARCHIVE" 'archive'
    actual="$DIGEST"
    # `printf` with an explicit `\n` rather than `echo`, because `echo` is not
    # required to be any particular thing about backslashes and one `echo -e`
    # would put a carriage return in a file whose whole job is to be read by a
    # tool on another platform. The name is the archive's own basename with no
    # directory, because `sha256sum -c` is run from the directory the file was
    # downloaded into and a path inside the file would not resolve there.
    printf '%s  %s\n' "$actual" "$sha_name" >"$SHA_PATH"

    # The bytes are asserted, not assumed. `RELEASE_PROCESS.md` writes this
    # format down — ASCII, no BOM, one LF, two spaces — and a checksum file that
    # a verification tool refuses for a reason that is not the checksum is a
    # file that fails the one job it has.
    #
    # The three counts together pin the layout exactly: one LF, no CR, and a
    # total length of 64 hex + 2 spaces + the name + 1 LF. A BOM is caught
    # without being looked for, because `grep`'s `^[0-9a-f]` above cannot match
    # a line that begins with three non-ASCII bytes, and the total length would
    # be three too many.
    expected_bytes=$((64 + 2 + ${#sha_name} + 1))
    written_bytes="$(wc -c <"$SHA_PATH" | tr -d ' ')"
    written_lf="$(tr -dc '\n' <"$SHA_PATH" | wc -c | tr -d ' ')"
    written_cr="$(tr -dc '\r' <"$SHA_PATH" | wc -c | tr -d ' ')"
    detail "file        $SHA_PATH"
    detail "bytes       $written_bytes (expected $expected_bytes), $written_lf LF, $written_cr CR"
    if [ "$written_cr" -ne 0 ]; then
        fail "the checksum file holds $written_cr carriage returns. A CRLF line ending makes the file a different file from the one this repository documents, and a \`sha256sum -c\` that fails on it fails for a reason that has nothing to do with the artifact."
    fi
    if [ "$written_lf" -ne 1 ]; then
        fail "the checksum file holds $written_lf newlines; this format is one line ending in one LF"
    fi
    if [ "$written_bytes" -ne "$expected_bytes" ]; then
        fail "the checksum file is $written_bytes bytes and this format is $expected_bytes: 64 digest characters, two spaces, '$sha_name', one LF"
    fi
    detail "digest      $actual"
else
    step 'Existing artifact'
    # Exactly one, or a refusal. `Build-Release.ps1` refuses an ambiguous match
    # for the same reason: checking one of two archives and reporting on "the
    # artifact" is a sentence about a file nobody chose.
    found=''
    for candidate in "$OUTPUT_DIR"/sure-*-"$TARGET".tar.gz; do
        [ -f "$candidate" ] || continue
        if [ -n "$found" ]; then
            fail "$OUTPUT_DIR holds more than one sure-*-$TARGET.tar.gz; this phase checks one and will not guess which"
        fi
        found="$candidate"
    done
    if [ -z "$found" ]; then
        fail "no sure-*-$TARGET.tar.gz in $OUTPUT_DIR to verify; run this script with --phase all first"
    fi
    ARCHIVE="$found"
    SHA_PATH="$ARCHIVE.sha256"
    ARTIFACT_NAME="$(basename "$ARCHIVE" .tar.gz)"
    detail "archive     $ARCHIVE"
    detail "size        $(wc -c <"$ARCHIVE" | tr -d ' ') bytes"
fi

# -----------------------------------------------------------------------------
# 3. The archive's entry list, stated rather than discovered.
# -----------------------------------------------------------------------------
step 'Layout'

entries_out="$LOGS/archive-entries.txt"
entries_err="$LOGS/archive-entries.err.txt"
entries_status=0
run_captured tar "$entries_out" "$entries_err" -tzf "$ARCHIVE" || entries_status=$?
if [ $entries_status -ne 0 ]; then
    fail "tar -tzf exited $entries_status on $ARCHIVE; its stderr is $entries_err"
fi
# A trailing slash on a directory entry is the writer's convention, not a
# difference in what is in the archive, so it is normalised away before the
# comparison. The list is then compared by *set* rather than by order, because
# the order of a tar's entries is the writer's and not something this script
# asked for.
normalised="$LOGS/archive-entries.normalised.txt"
sed 's:/$::' "$entries_out" | LC_ALL=C sort >"$normalised"
expected="$LOGS/archive-entries.expected.txt"
cat >"$expected" <<EOF
$ARTIFACT_NAME
$ARTIFACT_NAME/LICENSE
$ARTIFACT_NAME/RELEASE.txt
$ARTIFACT_NAME/sure
EOF
LC_ALL=C sort -o "$expected" "$expected"
if ! cmp -s "$normalised" "$expected"; then
    printf '\n' >&2
    printf 'FAILED: the archive does not hold the entries this artifact promises.\n\n' >&2
    printf '  expected\n' >&2
    printf '    %s\n' "$(tr '\n' ' ' <"$expected")" >&2
    printf '  found\n' >&2
    printf '    %s\n' "$(tr '\n' ' ' <"$normalised")" >&2
    printf '\nA tar is not obliged to be identical across implementations, so this is\n' >&2
    printf 'asserted rather than assumed: an entry added by a different tar is a\n' >&2
    printf 'difference in what a user receives. The full listing is %s.\n' "$entries_out" >&2
    exit 1
fi
detail "entries     $(wc -l <"$normalised" | tr -d ' ') entries, exactly the promised set"

# -----------------------------------------------------------------------------
# 4. Verify the archive against the digest file, read back from disk. This is
#    the step that would catch a truncated download or an archive altered after
#    it was published, so it runs *before* extraction: the property in the
#    header is about the order.
# -----------------------------------------------------------------------------
step 'Verify the checksum file'

if [ ! -f "$SHA_PATH" ]; then
    fail "no checksum file at $SHA_PATH, so nothing verifies $ARCHIVE"
fi
sha_lines="$(grep -c . "$SHA_PATH" || true)"
if [ "$sha_lines" -ne 1 ]; then
    fail "$SHA_PATH holds $sha_lines non-empty lines; this format is one digest line"
fi
sha_text="$(head -n 1 "$SHA_PATH")"
printf '%s\n' "$sha_text" >"$LOGS/checksum-line.txt"
# The `sha256sum` format, which is what `sha256sum -c` reads and what
# `RELEASE_PROCESS.md` writes down for the Windows artifact: 64 lowercase hex
# characters, **two spaces**, then the file's name and nothing else. Two spaces
# rather than the `*` binary-mode marker the tools on Git for Windows emit, for
# the reason recorded at `digest_of`: the separator is the platform's and this
# file is written by `printf` so that it is not. `sha256sum -c` reads both.
#
# The name is matched against the charset the artifact name can use rather than
# against `.*`: a pattern that accepted anything would accept a path, and a
# checksum file naming `/somewhere/else/sure.tar.gz` is a checksum over a file
# this run has not seen. The exact comparison against the archive below is the
# real check; this is the shape.
if ! grep -Eq '^[0-9a-f]{64}  [0-9A-Za-z._+-]+$' "$LOGS/checksum-line.txt"; then
    fail "the checksum file is not one sha256sum-format line of 64 lowercase hex characters, two spaces and a name:

  $sha_text

  $SHA_PATH"
fi
sha_digest="${sha_text%%  *}"
sha_named="${sha_text##*  }"
if [ "$sha_named" != "$(basename "$ARCHIVE")" ]; then
    fail "the checksum file names '$sha_named' but the archive is '$(basename "$ARCHIVE")'"
fi

digest_of "$ARCHIVE" 'archive'
actual="$DIGEST"
detail "expected    $sha_digest"
detail "actual      $actual"
if [ "$sha_digest" != "$actual" ]; then
    fail "the archive does not match its checksum file: $ARCHIVE"
fi
cross_check_digest "$ARCHIVE" 'archive-cross' "$actual"
detail 'matches     the archive on disk is the archive the checksum file names'

# -----------------------------------------------------------------------------
# 5. Extract to a fresh directory and read the bytes that are in there.
# -----------------------------------------------------------------------------
step 'Extract'

rm -rf "$EXTRACT_ROOT"
mkdir -p "$EXTRACT_ROOT"
extract_err="$LOGS/tar-extract.err.txt"
extract_status=0
set +e
tar -xzf "$ARCHIVE" -C "$EXTRACT_ROOT" >/dev/null 2>"$extract_err"
extract_status=$?
set -e
if [ $extract_status -ne 0 ]; then
    fail "tar -xzf exited $extract_status; its stderr is $extract_err"
fi
# The extraction directory is deleted before it is written, so a run that does
# not extract cannot find a binary left over from an earlier one.
top_level_count=0
top_level=''
for entry in "$EXTRACT_ROOT"/*; do
    [ -e "$entry" ] || continue
    if [ -d "$entry" ]; then
        top_level_count=$((top_level_count + 1))
        top_level="$entry"
    fi
done
if [ "$top_level_count" -ne 1 ]; then
    fail "$EXTRACT_ROOT holds $top_level_count top-level directories; the layout this artifact promises is exactly one"
fi
extract_dir="$(cd -P "$top_level" && pwd -P)"
if [ "$(basename "$extract_dir")" != "$ARTIFACT_NAME" ]; then
    fail "the archive's top-level directory is '$(basename "$extract_dir")' and the archive is named '$ARTIFACT_NAME.tar.gz'"
fi
detail "into        $extract_dir"

extracted_binary="$extract_dir/sure"
for required in sure LICENSE RELEASE.txt; do
    if [ ! -f "$extract_dir/$required" ]; then
        fail "the archive extracted but $required is not in it"
    fi
done
# The **mode** is checked after the architecture below, and it used to be here.
# It was moved because a run on a host whose extraction cannot set modes showed
# what being first costs: the mode assertion fires, and the architecture of the
# artifact — the fact this whole script exists to read off the bytes — is never
# read at all. A check that cannot be reached is a check that did not run, and
# this repository's whole subject is the run that reads as though it did. The
# bytes are the artifact and the mode is a property of the extraction, so the
# bytes are asked first.

digest_of "$extracted_binary" 'sure-extracted'
extracted_digest="$DIGEST"
if [ "$PHASE" = 'all' ]; then
    # The archive's digest covers the archive; this line ties the archive to the
    # bytes that get run: the executable inside it is the executable cargo
    # produced.
    if [ "$extracted_digest" != "$binary_digest" ]; then
        fail "the sure inside the archive is not the one that was built:

  built     $binary_digest  $BUILT_BINARY
  packaged  $extracted_digest  $extracted_binary"
    fi
    detail 'same as     the sure cargo built'
fi

# -----------------------------------------------------------------------------
# 6. The architecture, from the artifact's own bytes.
# -----------------------------------------------------------------------------
step 'Architecture'

require_macho_arch "$extracted_binary" 'extracted'

# The mode, now that the bytes have been read. An extracted binary without an
# execute bit cannot be run at all, and the failure a user would meet is their
# shell's rather than this one's — which is the reason the artifact is a tar and
# not a zip, so it is checked rather than mentioned.
if [ ! -x "$extracted_binary" ]; then
    # The whole listing rather than a `cut` of its mode column: it is one line,
    # and a reader who doubts the reading can see the file it is about.
    fail "the extracted 'sure' is not executable, so this artifact cannot be run by the person who downloads it. \`ls -ld\` says:

  $(ls -ld "$extracted_binary")

tar records the mode as a field of the format, so this is a packaging or extraction fault and not a build one."
fi

# -----------------------------------------------------------------------------
# 6b. The signature, read from the artifact rather than asserted in prose.
# -----------------------------------------------------------------------------
#
# The RELEASE.txt inside this archive says the build carries no Apple Developer
# ID signature, and a sentence about a file is not evidence about that file. This
# is the reading behind the sentence: `codesign -d` is macOS's own question, its
# answer is printed in full whatever it is, and the failure is on the one
# direction that matters — a signature **chain**, which is what a certificate
# produces, arriving in an archive whose own text says there is none.
#
# **An arm64 binary is expected to carry an ad-hoc signature**, and that is not
# a certificate: it is what the toolchain applies so the kernel will load the
# image, it names no developer, and its presence does not contradict the
# sentence. So the failure is keyed on an `Authority=` line, which an ad-hoc
# signature does not produce and a Developer ID chain does, and **not** on the
# exit status — `codesign -d` exits 0 for an ad-hoc signature and 1 for none at
# all. Keying it on the exit status would have failed every correct artifact.
#
# A `codesign` that is missing, or that cannot read the file, leaves the reading
# unknown; unknown is reported as unknown rather than turned into "unsigned".
# **Notarization is not read here at all**: nothing in this build notarizes, so
# the sentence rests on there being no such step, not on this measurement.
step 'Signature'

if have codesign; then
    sign_out="$LOGS/codesign.txt"
    sign_err="$LOGS/codesign.err.txt"
    sign_status=0
    run_captured codesign "$sign_out" "$sign_err" -d --verbose=2 "$extracted_binary" || sign_status=$?
    # `codesign -d` writes its answer to **stderr**, including when it succeeds,
    # so both streams are read into one file — stderr first — and that file is
    # what is printed and what is searched. The exit status is carried beside it
    # rather than instead of it, because it is not the thing being asserted on.
    sign_log="$LOGS/codesign.reading.txt"
    cat "$sign_err" "$sign_out" >"$sign_log" 2>/dev/null || true
    if [ -s "$sign_log" ]; then
        while IFS= read -r line; do
            if [ -n "$line" ]; then detail "codesign    $line"; fi
        done <"$sign_log"
    else
        detail 'codesign    (it printed nothing on either stream)'
    fi
    detail "exit        $sign_status (0 with 'Signature=adhoc' is a linker signature, not a certificate)"
    if grep -q '^Authority=' "$sign_log"; then
        fail "codesign read a signing authority on $extracted_binary, and the RELEASE.txt this script writes into the archive says the build carries no Apple Developer ID signature. One of the two is wrong, and an archive whose own text contradicts its own bytes is the shape this repository exists to refuse. The full reading is $sign_err"
    fi
    detail 'authority   none, so the unsigned sentence in RELEASE.txt is what the bytes say'
else
    detail 'codesign    (no codesign on PATH on this runner, so this run says nothing about whether the binary is signed)'
fi

# -----------------------------------------------------------------------------
# 7. Run the extracted binary.
# -----------------------------------------------------------------------------
step 'Run the extracted binary'

run_out="$LOGS/extracted-doctor.txt"
run_err="$LOGS/extracted-doctor.err.txt"
run_status=0
# By absolute path, and `--store-dir` is deliberately not passed: a user runs
# `sure doctor` against their own store, so that is what is run here. On a CI
# runner there is no store to find, and `sure doctor` creates nothing.
run_captured "$extracted_binary" "$run_out" "$run_err" doctor || run_status=$?
detail "invoked     $extracted_binary doctor"
detail "exit        $run_status"
if [ -s "$run_err" ]; then
    while IFS= read -r line; do detail "stderr      $line"; done <"$run_err"
fi
if [ $run_status -ne 0 ]; then
    fail "the extracted sure exited $run_status on this host ($(uname -s) $(uname -m)). If this host's processor is not $EXPECT_FILE_ARCH, a $EXPECT_FILE_ARCH Mach-O cannot be executed here at all, and that is a fact about the runner rather than about the artifact — the header read above already said what the bytes are. stderr is $run_err"
fi
if [ ! -s "$run_out" ]; then
    fail "the extracted sure wrote nothing to stdout; the report should be in $run_out"
fi

first_line="$(head -n 1 "$run_out")"
case "$first_line" in
    *"built for $EXPECT_OS $EXPECT_ARCH"*)
        detail "reports     $first_line"
        ;;
    *)
        fail "the extracted binary's first line is '$first_line', which does not contain 'built for $EXPECT_OS $EXPECT_ARCH'. The build's own statement about what it is disagrees with the target this script built for."
        ;;
esac

# `sure doctor` prints the path it is running from on the line after that one.
# It is what the binary says about itself, and it has to be the executable the
# archive was just extracted into: a run that picked up some other `sure` — a
# copy on PATH, one left by an earlier release — would still exit 0 and still
# print a well-formed report.
reported_from="$(sed -n 's/^running from //p' "$run_out" | head -n 1)"
if [ -z "$reported_from" ]; then
    fail "the extracted binary reported no 'running from' line, so which binary answered cannot be checked: $run_out"
fi
# What the field *is* decides what it may be compared with, so the reading comes
# from the product rather than from this script's expectation: `running from`
# prints `build.running_from`, `sure_core::doctor` fills it from
# `std::env::current_exe()`, and that field is documented there as "Where the
# running executable is." It is the path of the **file** on every platform. An
# earlier version of this check compared it with `$extract_dir`, the directory,
# which is one path component away from it — so the comparison could only ever
# fail, and it reddened the macOS job over a correct artifact.
#
# `current_exe()` is not required to return a canonical path, and on macOS it
# may not: a runner's `$TMPDIR` is under `/var`, which is a symlink to
# `/private/var`, so two spellings of one file are expected. Both sides are
# therefore folded the same way before they are compared — the directory
# component through `cd -P ... && pwd -P`, exactly as the extraction directory
# was already resolved above, and the basename as it is. Comparing the
# spellings, or comparing a file with a directory, would read as a mismatch on
# a machine where nothing is wrong.
#
# The check keeps its teeth: a `sure` from PATH, or one left by an earlier
# release, reports a path in some other directory and still fails here.
fold_binary_path() {
    local dir
    local base
    dir="$(dirname "$1")"
    base="$(basename "$1")"
    if [ -d "$dir" ]; then
        dir="$(cd -P "$dir" && pwd -P)"
    fi
    printf '%s/%s\n' "$dir" "$base"
}
required_binary="$(fold_binary_path "$extracted_binary")"
reported_binary="$(fold_binary_path "$reported_from")"
if [ "$reported_binary" != "$required_binary" ]; then
    fail "the binary that answered 'sure doctor' is not the one this script extracted:

  required  $required_binary
  reported  $reported_binary

The archive was extracted to $extract_dir, so a running_from that is not the
sure inside it means the run measured some other installation."
fi
detail "from        $required_binary"

# -----------------------------------------------------------------------------
# 8. The result.
# -----------------------------------------------------------------------------
printf '\nRESULT\n'
printf '  artifact    %s\n' "$ARCHIVE"
printf '  size        %s bytes\n' "$(wc -c <"$ARCHIVE" | tr -d ' ')"
printf '  sha256      %s\n' "$actual"
printf '  computed by %s\n' "$digest_tool"
printf '  checksum    %s\n' "$SHA_PATH"
printf '  verified    the archive matches the checksum file, re-read from disk\n'
printf '  layout      %s entries, exactly the promised set\n' "$(wc -l <"$normalised" | tr -d ' ')"
printf '  runs        %s\n' "$extracted_binary"
printf '  arch        %s, read from the header bytes %s\n' "$MACHO_DESCRIPTION" "$MACHO_BYTES"
printf '  gate        permitted; read from %s\n' "$GATE_PATH"
printf '  OK          the bytes that are checksummed are the bytes that were run\n'
exit 0

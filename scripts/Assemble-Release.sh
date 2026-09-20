#!/bin/sh
# SURE - verify the four release archives against their checksums, and write the
# one file a person who downloaded them needs.
#
#   sh scripts/Assemble-Release.sh --version 0.0.0-bootstrap
#   sh scripts/Assemble-Release.sh --version 0.0.0-bootstrap --output-dir DIR
#
# Run from anywhere; every path is derived from this file's own location.
#
# =============================================================================
# What this is, and what it deliberately does not do
# =============================================================================
#
# It reads the four archives `scripts/Build-Release.*` produce, requires each to
# be exactly what the `.sha256` written beside it on the machine that built it
# says, and writes `sure-<version>-release-checksums.txt`: one `sha256sum`-
# format line per archive, in the order the four artifacts are declared in
# `.github/workflows/release.yml`.
#
# It creates nothing else, deletes nothing, moves nothing and talks to no
# network. In particular **it does not create a GitHub Release, does not tag
# anything and does not push anything**: creating the draft release is the
# workflow's step and stays there, where a reader auditing an outward-facing act
# is already looking.
#
# **Every digest it writes is recomputed from the bytes on disk.** Reading the
# four `.sha256` files and concatenating them would produce a file that *looks*
# like a checksum file and is a copy of four claims; the release job that runs
# this happens after an upload and a download, and the whole point of that round
# trip is that what arrived is not assumed to be what left. So each archive is
# hashed here, and the shipped `.sha256` is what that reading is held to.
#
# =============================================================================
# The checksum file is NOT SHA256SUMS.txt
# =============================================================================
#
# `SHA256SUMS.txt` in the repository root is a curated integrity manifest over a
# subset of the **source tree**: 195 entries of the form
# `<64-hex>  <repo-relative path>`, no `target/` path, no `sure.exe`, and
# nothing in this repository generates or verifies it. Its ownership is
# `P15-T020`'s question and not this script's.
#
# This script never reads it, never writes it and never names it in a file it
# writes. The file it writes is `sure-<version>-release-checksums.txt`, which is
# a different object under a different name on purpose: a release asset called
# `SHA256SUMS.txt` would be indistinguishable, to a person reading a release
# page, from the source-tree manifest of the same name, and the difference
# between "these are the bytes we shipped" and "these are some source files" is
# the whole content of a checksum file. `scripts/Build-Release.ps1`'s header
# calls this the trap in this task; this name is how it is not fallen into.
#
# The aggregate covers the four **archives** and not their `.sha256` files. A
# digest of a digest file is a statement about the checksum file and not about
# the artifact, and a downloader who wants to know what they got needs the four
# lines above it.
#
# =============================================================================
# Why this is a script, and why it is this script
# =============================================================================
#
# It is a file rather than a step of `.github/workflows/release.yml` for two
# reasons and both are about being readable by something other than its author:
# `ci.yml`'s `shellcheck-secondary` job lints every `scripts/*.sh` on every
# push, so this file has an automated reader that does not depend on a release
# ever being dispatched; and it runs on a workstation, so "the four archives
# verify and the aggregate re-reads" is a claim that can be measured before the
# release job that calls it has ever run.
#
# It is not part of `scripts/Build-Release.sh` because that script builds one
# archive for one target and this one is about the set of four: it runs on a
# machine where all four are already on disk, it takes no target, and it must
# not be able to build anything. Two scripts holding one archive contract would
# be two places for the naming, the layout and the `.sha256` format to drift,
# and that is why the four *producers* are one script; this is a reader of their
# output rather than a fifth producer, which is why it is a second one.
#
# =============================================================================
# What makes this red
# =============================================================================
#
# A check that cannot fail is the defect, and the checks here are the kind that
# pass because nothing was read. Every line below is a run recorded on
# 2026-09-21 against this file, on `target/tmp/p15t011/`: four synthetic archives
# named `sure-0.0.0-bootstrap-<target>.<ext>` with their `.sha256` files written
# the way `Build-Release.ps1` writes them. Each mutation is a copy of that
# directory with exactly one thing changed, and the exit status is the shell's.
#
# * **an archive is missing** - `--version 9.9.9` names four files that are not
#   there: `FAILED: there is no file at .../sure-9.9.9-x86_64-pc-windows-msvc.zip
#   ... A version that names no archive is a version that does not match the
#   bytes on disk`, exit 1. **Three archives and not four** (`rm` of the Intel
#   tarball and of its `.sha256`) reaches the same message for the same reason,
#   exit 1 - the count is not a separate check because a missing member is
#   already fatal;
# * **a byte is appended to an archive after its `.sha256` was written** - the
#   shipped and recomputed digests are both printed and the run stops:
#   `shipped 04d543257e...  recomputed 0648fb30b4...`, exit 1;
# * **the `.sha256` is written with CRLF** - `is 115 bytes and the only shape
#   this script accepts is 114`, exit 1. The message names the CR as the
#   likeliest reason, because that is the failure `Build-Release.ps1` records
#   happening for real and every Windows tool reading the file happily;
# * **the `.sha256` is absent** - `A missing checksum is 'cannot confirm' and
#   not a pass`, exit 1;
# * **the `.sha256` names a different file** - the line is printed back and the
#   run stops: `It reads '...  sure-0.0.0-bootstrap-x86_64-apple-darwin.tar.gz'`,
#   exit 1;
# * **the digest is uppercase** (`56D64D68...`) and **the digest holds a
#   non-hex character** (`z6d64d68...`) - both reach `which is not lowercase
#   hex`, exit 1 each;
# * **the version holds a character a file name cannot carry**
#   (`--version '0.0.0 bootstrap'`) - `the set a version may use here is 0-9,
#   A-Z, a-z, dot, plus and minus`, exit 1;
# * **`--version` is not given at all** - the usage text is printed and then
#   `FAILED: --version is required. ... A script that guessed which release it
#   was assembling would be checking four file names nobody asked about`, **exit
#   2**, which is the usage-error status and not the check-failed one;
# * **`--output-dir` is relative** (`--output-dir ok` from inside the directory)
#   - `--output-dir must be an absolute path: ok`, exit 1; and when the path is
#   absolute and simply not there, `there is no directory at ... so there is
#   nothing to assemble`, exit 1;
# * **the aggregate is written and then a copy of one archive is modified** -
#   the control at the end requires the platform's own `sha256sum -c` to *refuse*
#   the modified copy, so a verification that could not fail is caught by a
#   verification that is required to. Measured: `sha256sum -c` on the four
#   copies prints four `OK` lines (the last, `...x86_64-unknown-linux-gnu.tar.gz:
#   OK`, is the one shown), the first copy then gets one `x` appended, and the
#   second reading of the same file - whose output is discarded, because the
#   only thing wanted from it is the status - **fails**, which is the outcome the
#   control requires.
#
# The last one was also run *without* this script, because a control exercised
# only by the program it is meant to catch is a control that agrees with it by
# construction. Four archives and the aggregate above, in a directory of their
# own: `sha256sum -c` prints four `OK` lines and exits **0**; the first line's
# first hex character changed from `5` to `0` and nothing else touched, and the
# same command prints `sure-...windows-msvc.zip: FAILED` with
# `WARNING: 1 computed checksum did NOT match` and exits **1**. So the mechanism
# the control leans on is one that distinguishes the two files by its exit
# status alone, which is what the control reads.
#
# One recorded result is **against** the shape above and is left visible rather
# than tidied: an aggregate left over from an earlier run
# (`sure-0.0.0-bootstrap-release-checksums.txt` holding `stale`) is **silently
# overwritten**, exit 0, and the written file is the 462-byte, four-line file
# with the four recomputed digests. That is deliberate - the file is an output
# and not an artifact to be preserved - but it is a run where this script
# changed a file that was already there, and a reader deciding whether an
# assembly was fresh cannot tell from the exit status alone. Nothing else on
# disk is written, moved or removed.
#
# The control is the one that matters most, because every other check here is
# this script reading its own work. `sha256sum -c` is a different program
# reading a file this script wrote, from the directory a downloader would be
# standing in.
#
# =============================================================================
# POSIX, and why `sh` rather than bash
# =============================================================================
#
# `release.yml` invokes this as `sh scripts/Assemble-Release.sh`, the same way
# all three artifact jobs invoke `Build-Release.sh`, because Git on Windows does
# not carry the executable bit into a checkout and a script invoked by path is a
# script that works on the machine it was written on. The `sh` here is bash 3.2
# in POSIX mode on a macOS runner and dash on a Linux one, so this file uses no
# bash extension at all: no `local`, no arrays, no `[[ ]]`, no `${x//y}`, no
# process substitution, and **no `set -o pipefail`** - that option is the one
# that reddened the Linux release job while every other machine ran the same
# file green (`P15-T007`, run `35524124026`, `sh: 530: set: Illegal option -o
# pipefail`), and nothing here needs it because no value this script decides on
# is read out of a pipeline.

set -eu

usage() {
    cat <<'EOF'
SURE - verify the four release archives and write the release checksum file.

  sh scripts/Assemble-Release.sh --version VERSION [options]

  --version VERSION   the version the archives are named after, which is the
                      version `sure` itself reports. Required: a run without it
                      cannot know which four files it is being asked about, and
                      a default would be a guess about which release is being
                      assembled.
  --output-dir DIR    where the four archives and their .sha256 files are, and
                      where the release checksum file is written. Default:
                      target/tmp/release.
  --help              this text.
EOF
}

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

have() {
    command -v "$1" >/dev/null 2>&1
}

# The four artifacts, as `<target>:<extension>`, in the order the jobs that
# build them are declared in `.github/workflows/release.yml`. One list, in the
# one place that has to know all four at once.
#
# The extensions differ on purpose and are not a compromise: a Windows user
# expects a ZIP and `[System.IO.Compression.ZipFile]` writes one, while a tar
# records a file's mode as a field of the format, which is what makes the
# extracted `sure` executable. Two macOS archives and one Linux archive are
# tarballs, and the Windows one is the only ZIP.
ARTIFACTS='x86_64-pc-windows-msvc:zip
aarch64-apple-darwin:tar.gz
x86_64-apple-darwin:tar.gz
x86_64-unknown-linux-gnu:tar.gz'

VERSION=''
OUTPUT_DIR=''

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            [ $# -ge 2 ] || { usage >&2; echo "FAILED: --version needs a value" >&2; exit 2; }
            VERSION="$2"
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

if [ -z "$VERSION" ]; then
    usage >&2
    echo "FAILED: --version is required. The archives are named after the version" >&2
    echo "sure reports, and a script that guessed which release it was assembling" >&2
    echo "would be checking four file names nobody asked about." >&2
    exit 2
fi

# The same shape `Build-Release.sh` requires before it will put a version in a
# file name, and the reason is the same: this string is about to become part of
# four paths and of the name of the file this script writes. It is also what
# lets every length below be counted in bytes and characters alike, because a
# version this accepts contains no character that is more than one byte.
case "$VERSION" in
    '' | *[!0-9A-Za-z.+-]*)
        fail "the version '$VERSION' is not a version this script will put in a file name; the set a version may use here is 0-9, A-Z, a-z, dot, plus and minus"
        ;;
esac

SCRIPT_DIR="$(cd -P "$(dirname "$0")" && pwd -P)"
ROOT="$(cd -P "$SCRIPT_DIR/.." && pwd -P)"

if [ -z "$OUTPUT_DIR" ]; then
    OUTPUT_DIR="$ROOT/target/tmp/release"
fi
case "$OUTPUT_DIR" in
    /*) ;;
    *) fail "--output-dir must be an absolute path: $OUTPUT_DIR" ;;
esac
if [ ! -d "$OUTPUT_DIR" ]; then
    fail "there is no directory at $OUTPUT_DIR, so there is nothing to assemble. This script reads the archives a build wrote; it does not build them."
fi

CHECKSUMS_NAME="sure-$VERSION-release-checksums.txt"
CHECKSUMS_PATH="$OUTPUT_DIR/$CHECKSUMS_NAME"

step 'The digest tools this run will use'
if ! have sha256sum; then
    fail "there is no sha256sum on PATH, and this script reads and writes sha256sum-format files. Installing one is not this script's business."
fi
detail "sha256sum   $(command -v sha256sum)"
# The second reader, where one exists. Its absence is reported rather than
# passed over, and its disagreement is a failure: `Build-Release.sh` cross-checks
# the same way, and a machine where two independent programs agree about a
# digest is a different reading from one where only one program was asked.
if have shasum; then
    detail "shasum      $(command -v shasum)"
else
    detail 'shasum      (absent on this machine; the recomputation below has one reader here)'
fi

# -----------------------------------------------------------------------------
# Part 1. Each archive, against the checksum written beside it.
# -----------------------------------------------------------------------------
#
# The four `.sha256` files are read, byte for byte, and held to the only shape
# `docs/development/RELEASE_PROCESS.md` promises:
#
#   <64 lowercase hex><two spaces><the archive's own file name>\n
#
# in ASCII, with no BOM and no CR. The byte count is the check that carries the
# most: it is `67 + the length of the name`, so a CR, a BOM, a second line and a
# missing final newline are all one number away from passing and all caught by
# it. The `case` pattern below is the same statement spelled out, so that a file
# of the right length is still refused if its bytes are not that shape.

step 'Each archive, against the checksum written beside it'

RELEASE_LINES=''
COUNT=0
FIRST_ARCHIVE=''
for entry in $ARTIFACTS; do
    target="${entry%%:*}"
    extension="${entry#*:}"
    archive_name="sure-$VERSION-$target.$extension"
    archive_path="$OUTPUT_DIR/$archive_name"
    sha_path="$archive_path.sha256"
    if [ -z "$FIRST_ARCHIVE" ]; then
        FIRST_ARCHIVE="$archive_name"
    fi

    printf '\n   %s\n' "$archive_name"

    if [ ! -f "$archive_path" ]; then
        fail "there is no file at $archive_path. The four archives are named after the version sure reported when they were built, so a version that names no archive is a version that does not match the bytes on disk - or a run that has not been given the artifacts it is meant to be checking. Nothing was written."
    fi
    if [ ! -f "$sha_path" ]; then
        fail "there is no file at $sha_path, so nothing here says what $archive_name is supposed to be. A missing checksum is 'cannot confirm' and not a pass. Nothing was written."
    fi

    archive_bytes="$(wc -c <"$archive_path")"
    detail "archive     $((archive_bytes + 0)) bytes"

    sha_bytes="$(wc -c <"$sha_path")"
    sha_bytes="$((sha_bytes + 0))"
    name_length=${#archive_name}
    # 64 hex, two spaces, the name, one LF.
    expected_sha_bytes=$((67 + name_length))
    if [ "$sha_bytes" -ne "$expected_sha_bytes" ]; then
        printf '\n' >&2
        printf 'FAILED: %s is %s bytes and the only shape this script accepts is %s.\n' \
            "$sha_path" "$sha_bytes" "$expected_sha_bytes" >&2
        printf '\n' >&2
        printf '  %s bytes of digest and two spaces, %s bytes of file name, one LF.\n' \
            '64' "$name_length" >&2
        printf 'One byte more is the commonest failure this repository has seen: a CR\n' >&2
        printf 'before the LF, written by a tool that ends lines the Windows way. Every\n' >&2
        printf 'Windows program reads that file happily and `sha256sum` does not, because\n' >&2
        printf 'the CR becomes part of the file name it looks for. One byte less is a\n' >&2
        printf 'final newline that is not there. Nothing was written.\n' >&2
        exit 1
    fi

    sha_line=''
    read -r sha_line <"$sha_path" || fail "$sha_path could not be read"
    case "$sha_line" in
        ????????????????????????????????????????????????????????????????\ \ "$archive_name") ;;
        *)
            fail "$sha_path does not read '<64 hex><two spaces>$archive_name'. It reads '$sha_line'. A checksum line that does not name the file beside it is a pairing nobody checked. Nothing was written."
            ;;
    esac
    shipped_digest="${sha_line%%  *}"
    case "$shipped_digest" in
        *[!0-9a-f]*)
            fail "the digest in $sha_path is '$shipped_digest', which is not lowercase hex. sha256sum writes lowercase hex, and a digest in another case still compares unequal on platforms where a byte is a byte. Nothing was written."
            ;;
    esac
    detail "shipped     $shipped_digest"

    # The reading this whole script exists for: the digest of the bytes that are
    # on disk now, next to the digest of the bytes that were on disk when the
    # build wrote the file.
    #
    # The digest is taken as *the leading run of lowercase hex*, rather than by
    # splitting on the two spaces above. `sha256sum` writes `<digest>  <name>`
    # on Linux and macOS and `<digest> *<name>` where the C library makes the
    # distinction between text and binary reads - measured 2026-09-21, on
    # Cygwin's coreutils, where `sha256sum file` printed
    # `56d64d68...6fc5 *sure-....zip`. The separator is not this script's
    # business and the digest is: every implementation of `sha256sum` begins its
    # line with the digest, so the first character that is not lowercase hex is
    # where it ends on all of them. A tool that printed no digest at all leaves
    # this empty, and an empty value is compared and fails rather than being
    # skipped.
    actual_line=''
    actual_line="$(sha256sum "$archive_path")" ||
        fail "sha256sum exited non-zero on $archive_path"
    actual_digest="${actual_line%%[!0-9a-f]*}"
    detail "recomputed  $actual_digest"

    if have shasum; then
        second_line=''
        second_line="$(shasum -a 256 "$archive_path")" ||
            fail "shasum exited non-zero on $archive_path"
        second_digest="${second_line%%[!0-9a-f]*}"
        if [ "$second_digest" != "$actual_digest" ]; then
            fail "two programs disagree about $archive_name: sha256sum says $actual_digest and shasum says $second_digest. Neither is overridden. Nothing was written."
        fi
        detail "second      $second_digest (shasum -a 256, and it agrees)"
    fi

    if [ "$shipped_digest" != "$actual_digest" ]; then
        printf '\n' >&2
        printf 'FAILED: %s does not match the bytes of %s.\n' "$sha_path" "$archive_name" >&2
        printf '\n' >&2
        printf '  shipped     %s\n' "$shipped_digest" >&2
        printf '  recomputed  %s\n' "$actual_digest" >&2
        printf '\n' >&2
        printf 'The archive on disk is not the archive this checksum was written for.\n' >&2
        printf 'The commonest reason is the one this script exists to catch: the file went\n' >&2
        printf 'somewhere and came back, and what came back is not what left. Nothing was\n' >&2
        printf 'written, and no line of a release checksum file was built from either value.\n' >&2
        exit 1
    fi
    detail 'agrees'

    RELEASE_LINES="$RELEASE_LINES$actual_digest  $archive_name
"
    COUNT=$((COUNT + 1))
done

if [ "$COUNT" -ne 4 ]; then
    fail "this script read $COUNT of the four artifacts, which cannot happen without one of the checks above firing; it is stated rather than assumed because a loop that ran zero times would otherwise write an empty checksum file and exit 0."
fi

# -----------------------------------------------------------------------------
# Part 2. The release checksum file, written and then re-read.
# -----------------------------------------------------------------------------
#
# The bytes are assembled in a variable and written once, so the file this
# script writes is a value it states rather than a consequence of four `echo`es
# to a stream. `printf` writes no BOM of its own on any platform.

step 'The release checksum file'

printf '%s' "$RELEASE_LINES" >"$CHECKSUMS_PATH"

written_bytes="$(wc -c <"$CHECKSUMS_PATH")"
written_bytes="$((written_bytes + 0))"
expected_bytes=${#RELEASE_LINES}
if [ "$written_bytes" -ne "$expected_bytes" ]; then
    fail "$CHECKSUMS_PATH is $written_bytes bytes and the value written was $expected_bytes. A file that is not what was put in it is not checked by reading it back."
fi
written_lines="$(wc -l <"$CHECKSUMS_PATH")"
written_lines="$((written_lines + 0))"
if [ "$written_lines" -ne 4 ]; then
    fail "$CHECKSUMS_PATH has $written_lines line terminators and must have one per artifact, which is 4."
fi
case "$RELEASE_LINES" in
    *"$(printf '\r')"*)
        fail "$CHECKSUMS_PATH would contain a CR. sha256sum reads a CR as part of the file name. Nothing else was done."
        ;;
esac
detail "file        $CHECKSUMS_PATH"
detail "bytes       $written_bytes"
detail "lines       $written_lines"
detail "not         SHA256SUMS.txt, which is a manifest over the source tree"
printf '\n'
printf '%s' "$RELEASE_LINES" | sed 's/^/   /'

# The re-read, by a different program than the one that wrote it, from the
# directory a person who downloaded these files would be standing in.
step "The file, read back by the platform's own tool"
checksum_out=''
checksum_status=0
checksum_out="$(cd "$OUTPUT_DIR" && sha256sum -c "$CHECKSUMS_NAME" 2>&1)" || checksum_status=$?
printf '%s\n' "$checksum_out" | sed 's/^/   /'
if [ "$checksum_status" -ne 0 ]; then
    fail "sha256sum -c refused $CHECKSUMS_PATH with exit $checksum_status, and the file it refused is the file this run just wrote. Nothing was published and nothing was created."
fi

# The control. Everything above is this script reading its own work; this asks
# the same command to *fail* on a copy that was modified after the checksum file
# was written, which is what makes the OK lines above a reading rather than a
# step that cannot go red. The artifacts themselves are not touched: only copies
# are.
step 'The control, because a check that cannot fail is not a check'
control_dir="${TMPDIR:-/tmp}/sure-release-checksums-control"
rm -rf "$control_dir"
mkdir -p "$control_dir"
for entry in $ARTIFACTS; do
    target="${entry%%:*}"
    extension="${entry#*:}"
    cp "$OUTPUT_DIR/sure-$VERSION-$target.$extension" "$control_dir/"
done
cp "$CHECKSUMS_PATH" "$control_dir/"
detail "control dir $control_dir (copies; the artifacts are not touched)"
control_ok=''
control_ok="$(cd "$control_dir" && sha256sum -c "$CHECKSUMS_NAME" 2>&1)" ||
    fail "the copies in $control_dir did not verify, and they are byte-for-byte the artifacts that did. Two readings of the same bytes disagreeing is a defect in this check rather than in the artifacts."
printf '%s\n' "$control_ok" | sed 's/^/   /' | tail -n 1

printf 'x' >>"$control_dir/$FIRST_ARCHIVE"
detail "modified    $FIRST_ARCHIVE (the copy, not the artifact)"
if (cd "$control_dir" && sha256sum -c "$CHECKSUMS_NAME" >/dev/null 2>&1); then
    fail "sha256sum -c passed over $FIRST_ARCHIVE after a byte was appended to it, so the check above proves nothing. This is a defect in this script."
fi
detail 'the modified copy was refused, as it must be'
rm -rf "$control_dir"

printf '\nOK  four archives verified against the checksums written beside them, and %s names them\n' "$CHECKSUMS_NAME"
printf '    with the digests recomputed here from the bytes on disk rather than copied from\n'
printf '    the four .sha256 files. This says nothing about whether the archives run or where\n'
printf '    they came from; that is what RELEASE.txt inside each one and the build log are for.\n'

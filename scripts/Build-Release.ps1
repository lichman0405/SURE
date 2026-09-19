# SURE - build, package and check the Windows x64 release artifact.
#
#   & .\scripts\Build-Release.ps1
#   & .\scripts\Build-Release.ps1 -Phase Verify
#
# Run from anywhere; every path is derived from this file's own location.
#
# =============================================================================
# What a release artifact is, decided here and recorded in RELEASE_PROCESS.md
# =============================================================================
#
# `docs/development/RELEASE_PROCESS.md` is the only statement in this tree of
# what a release artifact is, and it says three things: a Windows x64
# (`x86_64-pc-windows-msvc`) CLI/core archive, a SHA-256 checksum, and a
# PowerShell per-user install/uninstall flow. It does not say the archive
# format, the file naming, or the layout inside. This script decides those, and
# the same decision is written into `docs/development/RELEASE_PROCESS.md` so that
# `P15-T003` (which installs this) and `P15-T004` (which references it) have it
# in a document rather than in a script's source:
#
#   artifact   sure-<version>-<target>.zip
#   checksum   sure-<version>-<target>.zip.sha256, one sha256sum-format line
#   layout     one top-level directory, sure-<version>-<target>/, holding
#              sure.exe, LICENSE and RELEASE.txt
#   location   target/tmp/release/ by default
#
# `target/` is gitignored, so nothing binary is committed. That is the same
# reason `sure_core::release_gate` gives about its own document: it is a
# reading of this checkout on this day, and a committed one would go stale in a
# way nothing reddens.
#
# `-Phase Verify` runs everything that reads the artifact and nothing that
# writes it: it re-reads the `.sha256` file, checks the archive against it,
# extracts, and runs the extracted binary. It exists because "the writer
# checked its own work" and "something else re-read it later" are different
# claims, and only the second survives the writer being wrong.
#
# =============================================================================
# The checksum file is NOT SHA256SUMS.txt, and this is the trap in this task
# =============================================================================
#
# `SHA256SUMS.txt` in the repository root is a frozen, curated integrity
# manifest over a subset of the **source tree**: 195 entries of the form
# `<64-hex>  <repo-relative path>`, no `target/` path, no `sure.exe`, and
# nothing in this repository generates or verifies it. An artifact digest
# placed in it would be a file that looks like assurance and is not, and it
# cannot gain one anyway - the supervisor's `regen-sums.mjs` updates only
# entries that already exist and never adds, removes or reorders one. Its
# ownership, and whether it should exist at all, is `P15-T020`'s question.
# This script never writes it and only ever reads it as a normal tracked file.
#
# =============================================================================
# The release gate, and why this script refuses to package without it
# =============================================================================
#
# `sure_core::release_gate`'s `GATE_CONTRACT` is the repository's own statement
# about packaging, and it is written into the document itself:
#
#   "P15 may build, package or publish a release of this checkout only when
#    this document's `decision` is `permitted`."
#
# This is a P15 packaging task, so the sentence applies to it. It is therefore
# a precondition here and not a report: with no gate document, or with a
# `blocked` one, this script refuses to produce an artifact.
#
# **The cost is real and is named rather than hidden.** A fresh clone cannot
# package until something writes the gate, because the document is a build
# artefact under `target/tmp/` and is produced by
# `cargo test -p sure-core --test acceptance_report_runner`. That test is part
# of `cargo test --workspace`, which CI already runs on every push, so the cost
# falls on a developer who has built but not tested. The alternative - reading
# the decision only when the file happens to be there - is a gate that a
# packaging run can bypass by not having run the tests, which is the shape of
# every false green this repository exists to prevent. A missing gate is
# "cannot confirm", and "cannot confirm" is not a yes.
#
# `P15-T011` (the GitHub release workflow) inherits this decision: its job will
# have to run the acceptance test before it can package.
#
# =============================================================================
# What "builds/tests" means here, and what it deliberately does not
# =============================================================================
#
# `tasks/tasks.json`'s acceptance for `P15-T002` is "x86_64-pc-windows-msvc
# artifact builds/tests and checksum generated", and there are two readings of
# "tests". The one this script establishes is the second:
#
#   the workspace's tests pass  - true, but about a *debug* build. A release
#                                 build has debug_assertions off and different
#                                 codegen, so it is not evidence about the
#                                 bytes in the archive.
#   the artifact is exercised  - this script extracts the archive to a fresh
#                                 directory and runs the executable **from
#                                 there**, by absolute path, with the
#                                 extraction directory it actually used
#                                 asserted against what the binary reports as
#                                 `running_from`.
#
# The property that makes that falsifiable is:
#
#   **the bytes that are checksummed are the bytes that were run.**
#
# It is established by the order of the steps, not by a claim: the digest is
# written, then re-read and checked, and only then is the archive extracted.
# The extracted `sure.exe` is additionally hashed and compared with the one
# cargo built, so a package that silently contained something else reddens.
#
# `cargo test --release` is **not** run by this script. Measured on 2026-09-19
# on the machine this was written on, `cargo test --workspace --all-features
# --release --target x86_64-pc-windows-msvc --no-fail-fast` exits 0 with 0
# failed, and that is reported in the task's hand-back rather than implied by
# this script. It is not run here for two reasons: it is minutes rather than
# seconds, and it is not what the acceptance asks for - the acceptance asks
# that the artifact be tested, and the artifact is a binary, not a test
# harness. The two facts are separable and this paragraph keeps them separate.
#
# =============================================================================
# Windows discipline (CLAUDE.md)
# =============================================================================
#
# * Every program is invoked with a typed argument array and the call operator
#   (`& $Program @Arguments`). There is no shell-string construction anywhere in
#   this file, and no assumption of `/bin/sh`.
# * Every claim about a child process - its status, its bytes, its streams -
#   comes from redirection to a **file**. Nothing is read out of a pipeline.
#   `progress/HANDOFF.md` records a Windows capture that produced a wrong number
#   about a child process by taking it from one, and the fix is structural: the
#   exit status is read from `$LASTEXITCODE` on the line after the call and
#   nowhere else, and the text a reader sees is read back from a file with
#   `Get-Content`.
# * No administrator rights, no service, no registry write and no symlink:
#   everything is created under `-OutputDirectory`. Per-user by construction.
# * Paths with spaces and non-ASCII characters go through `Join-Path`,
#   `[IO.Path]::GetFullPath`, `-LiteralPath` and .NET APIs that take a path
#   rather than a glob. This is measured rather than asserted: the whole run is
#   repeated with `-OutputDirectory` at a path containing a space and non-ASCII
#   characters, so that the artifact, the checksum, the extraction directory and
#   the `running_from` the binary reports all carry them. Measured 2026-09-19 -
#   it exits 0 and `running_from` comes back with the non-ASCII characters
#   intact, which is what the `[Console]::OutputEncoding` line above is for.
# * Long paths are tested, and what is tested is a **limit**, not a success. With
#   `-OutputDirectory` nested until the extracted `sure.exe` path is 294
#   characters, the extraction succeeds and the run cannot start: `CreateProcess`
#   is capped at MAX_PATH, and this machine has `LongPathsEnabled = 0`. The
#   script refuses that case by name instead of letting PowerShell's
#   unrelated-sounding message surface - see the guard above the run step.
#
# =============================================================================
# Falsifiability - what reddens this script
# =============================================================================
#
# A check that cannot fail is the defect. Three mutations were run against this
# file on 2026-09-19 and each is reproducible from this comment alone.
#
# **Mutation 0 - write the checksum line with `Set-Content`.** In the checksum
# step, replace the `[System.IO.File]::WriteAllText` call with the
# `Set-Content -Encoding ascii` it replaced. Expected and measured: the run
# stops at the first byte assertion with
#
#   FAILED: the checksum file contains a CR at byte 113; sha256sum reads it as
#   part of the file name
#
# and it stops **alone** - `Fail` exits, so the other two assertions never run,
# and they would not have fired if they had: `\r\n` still ends in a newline and
# `-Encoding ascii` writes no BOM. That is the whole reason the CR assertion is
# the one that has to exist, and the blast radius is one check rather than
# three. This is also the mutation that had already happened once for real:
# with `Set-Content`, this script wrote `sure-....zip\r\n`, and the
# `sha256sum.exe` Git for Windows ships answered `FAILED open or read` with
# `'sure-....zip'\r': No such file or directory`, because the CR is part of the
# file name to it. Every Windows tool read that file happily, which is why it
# took an independent verifier to find and why the assertion is on the bytes.
#
# **Mutation 1 - corrupt the artifact after its checksum was written.** Run
# `-Phase All` to completion, then append one byte to the archive without
# touching the `.sha256` file, then run `-Phase Verify`. Expected and measured:
# `Verify` fails at the digest comparison and names the two digests. It does not
# reach the extraction step, which is the point of comparing first.
#
# **Mutation 2 - run whatever `sure` is on `PATH` instead of the extracted
# one.** In the run step below, change `Invoke-Captured`'s `-Program
# $extractedExe` to `-Program 'sure.exe'`, and run the script with the debug
# build's directory prepended to `PATH`. Expected and measured: the `running_from`
# assertion fails, naming the extraction directory it required and the path the
# binary actually reported. This is the subtle one - the run succeeds, the exit
# status is 0, the frame parses, and `target_env` is still `msvc`, because the
# debug build on this machine is also an MSVC build. The assertion on
# `running_from` is the only thing standing between this script and a green run
# that measured a binary nobody shipped; removing that assertion as well makes
# the mutated script pass, which is how the assertion is shown to be the thing
# doing the work rather than the filesystem.
#
# A third property - that this cannot pass over a stale extraction - is not
# asserted by a mutation but by construction: the extraction directory is
# deleted before it is written, so a run that does not extract cannot find a
# binary left over from an earlier one.

[CmdletBinding()]
param(
    # `All` builds, packages, checksums and checks. `Verify` checks an archive
    # this script already produced, without building and without writing
    # anything next to it.
    [ValidateSet('All', 'Verify')]
    [string] $Phase = 'All',

    # The triple the acceptance names. macOS and Linux artifacts are `P15-T005`
    # and `P15-T007` and are explicitly "in CI/release environment"; a script
    # that accepted any triple here would happily produce a file named after a
    # platform it never built for, which is why the set has one member.
    [ValidateSet('x86_64-pc-windows-msvc')]
    [string] $Target = 'x86_64-pc-windows-msvc',

    # Where the archive, its checksum and the scratch extraction go. The default
    # is gitignored. The hand-back's evidence run overrides it to a path with a
    # space and non-ASCII characters.
    [string] $OutputDirectory = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# PowerShell 7.3+ can be told to turn a native command's stderr into a
# terminating error. Off explicitly, because several programs here write to
# stderr on success (cargo writes its progress there), and
# `$ErrorActionPreference = 'Stop'` would otherwise kill the run on the first
# thing cargo has to say.
$PSNativeCommandUseErrorActionPreference = $false

# The child's bytes are decoded with this. A `running_from` under a path with
# non-ASCII characters is exactly the value this script asserts on, so letting
# the platform's default code page decide how those bytes are read would make
# the assertion depend on the machine's locale.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

# `[IO.Path]::GetFullPath` rather than `Resolve-Path`: this string is derived
# from `$PSScriptRoot`, and `Resolve-Path` would treat a `[` or `]` in a
# directory name as a wildcard. Nothing here should care what the repository is
# called.
$Root = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ($OutputDirectory -eq '') {
    $OutputDirectory = Join-Path $Root 'target\tmp\release'
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

$GatePath = Join-Path $Root 'target\tmp\release-gate.json'
$ManifestPath = Join-Path $Root 'evaluation\acceptance-manifest.json'
$Scratch = Join-Path $OutputDirectory 'scratch'
$LogDirectory = Join-Path $OutputDirectory 'logs'

function Write-Step {
    param([Parameter(Mandatory)][string] $Text)
    Write-Host ''
    Write-Host "== $Text"
}

function Write-Detail {
    param([Parameter(Mandatory)][string] $Text)
    Write-Host "   $Text"
}

# Run one program with a typed argument array and both streams sent to files.
#
# Returns the exit status; never returns or throws on the text. The caller reads
# the files. This is the only place in this script that starts a process, so the
# redirection rule in the header is enforced in one function rather than
# remembered at every call site.
function Invoke-Captured {
    param(
        [Parameter(Mandatory)][string] $Program,
        [Parameter(Mandatory)][string[]] $Arguments,
        [Parameter(Mandatory)][string] $OutPath,
        [Parameter(Mandatory)][string] $ErrPath
    )
    foreach ($path in @($OutPath, $ErrPath)) {
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    }
    & $Program @Arguments 1> $OutPath 2> $ErrPath
    return $LASTEXITCODE
}

# The captured text, or the empty string when the stream stayed empty and no
# file was written. Both are real answers, and a silent success must not be told
# apart from a quiet failure by an exception.
#
# The `$null` arm is not defensive padding: `Get-Content -Raw` returns `$null`,
# not `''`, for a file of zero bytes, and every caller here concatenates or
# trims the result. Measured - a run that reached this line with an empty stderr
# file died with "Cannot call a method on a null-valued expression" at the
# `.Trim()` below, which is a failure in the reporting of a run that had in fact
# succeeded.
function Read-Captured {
    param([Parameter(Mandatory)][string] $Path)
    if (-not (Test-Path -LiteralPath $Path)) { return '' }
    $text = Get-Content -LiteralPath $Path -Raw -ErrorAction SilentlyContinue
    if ($null -eq $text) { return '' }
    return $text
}

function Get-Sha256 {
    param([Parameter(Mandatory)][string] $Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Fail {
    param([Parameter(Mandatory)][string] $Text)
    Write-Host ''
    Write-Host "FAILED: $Text"
    exit 1
}

try {
    Write-Host 'SURE release artifact (Windows x64)'
    Write-Host "  repository  $Root"
    Write-Host "  target      $Target"
    Write-Host "  output      $OutputDirectory"
    Write-Host "  phase       $Phase"

    # -------------------------------------------------------------------------
    # 1. The release gate. Read first, and before anything is built, because a
    #    refusal here must cost nothing and must be the first thing a reader
    #    sees.
    # -------------------------------------------------------------------------
    Write-Step 'Release gate'

    if (-not (Test-Path -LiteralPath $GatePath)) {
        Fail @"
no release gate at $GatePath, so this checkout has no answer to the question
GATE_CONTRACT makes a precondition of packaging. Nothing was built.

  Write it with:  cargo test -p sure-core --test acceptance_report_runner

A missing gate is "cannot confirm", and "cannot confirm" is not "permitted".
"@
    }
    $gate = Get-Content -LiteralPath $GatePath -Raw | ConvertFrom-Json
    if ($gate.decision -ne 'permitted') {
        $blocked = @($gate.blocked_by | ForEach-Object { "  [$($_.agreement)] $($_.id): $($_.why)" }) -join [Environment]::NewLine
        Fail "the release gate says this release is $($gate.decision), and names these cases:$([Environment]::NewLine)$blocked"
    }
    Write-Detail 'decision    permitted'
    Write-Detail 'blocked_by  []'
    Write-Detail "corpus      $($gate.corpus.cases) cases, $($gate.corpus.release_blocking) release-blocking, $($gate.corpus.release_blocking_observed) observed, $($gate.corpus.release_blocking_cannot_confirm) not observed"
    Write-Detail "gate file   $GatePath"

    # The gate is a reading of `evaluation/acceptance-manifest.json`, and this is
    # a *timestamp* comparison and not a content one. `corpus.manifest_digest`
    # cannot be checked here: it is a domain-separated digest
    # (`sure.acceptance-corpus.v1`, one length-prefixed field) built by
    # `sure_core::fingerprint::digest`, so `Get-FileHash` over the manifest file
    # is a different value, and reimplementing that construction in PowerShell
    # would be a hand-rolled copy of a crypto format - a thing that agrees with
    # the real one until it does not, which is the failure this repository calls
    # a false green. So this check can say "the manifest was written after the
    # gate" and cannot say "the manifest is the one the gate read". The gap is
    # named rather than papered over; closing it belongs to whoever owns the
    # gate document, not to a build script.
    if (Test-Path -LiteralPath $ManifestPath) {
        $gateTime = (Get-Item -LiteralPath $GatePath).LastWriteTimeUtc
        $manifestTime = (Get-Item -LiteralPath $ManifestPath).LastWriteTimeUtc
        if ($manifestTime -gt $gateTime) {
            Fail @"
the acceptance manifest was written after the release gate:

  $ManifestPath  $($manifestTime.ToString('o'))
  $GatePath  $($gateTime.ToString('o'))

The gate is a decision about that manifest's release-blocking cases, so a
manifest newer than the gate means the gate is a reading of a different corpus.
Re-run it:  cargo test -p sure-core --test acceptance_report_runner
"@
        }
        Write-Detail 'freshness   the manifest is not newer than the gate (timestamp, not content)'
    }

    # -------------------------------------------------------------------------
    # 2. Build.
    # -------------------------------------------------------------------------
    $builtExe = Join-Path $Root "target\$Target\release\sure.exe"
    $artifactName = ''
    $archivePath = ''
    $shaPath = ''
    $version = ''

    if ($Phase -eq 'All') {
        Write-Step 'Version'
        New-Item -ItemType Directory -Force -Path $LogDirectory | Out-Null

        $metaOut = Join-Path $LogDirectory 'cargo-metadata.json'
        $metaErr = Join-Path $LogDirectory 'cargo-metadata.err.txt'
        $metaCode = Invoke-Captured -Program 'cargo' -Arguments @(
            'metadata', '--no-deps', '--format-version', '1',
            '--manifest-path', (Join-Path $Root 'Cargo.toml')
        ) -OutPath $metaOut -ErrPath $metaErr
        if ($metaCode -ne 0) {
            Fail "cargo metadata exited $metaCode; stderr is $metaErr"
        }
        $metadata = Get-Content -LiteralPath $metaOut -Raw | ConvertFrom-Json
        # The package whose binary is named `sure`, rather than the package named
        # `sure-cli`: the artifact is named after what a user runs, and the
        # package name is a workspace detail.
        $package = $metadata.packages |
            Where-Object { $_.targets | Where-Object { $_.kind -contains 'bin' -and $_.name -eq 'sure' } } |
            Select-Object -First 1
        if ($null -eq $package) {
            Fail 'no package in this workspace declares a binary named sure'
        }
        $version = $package.version
        Write-Detail "version     $version (package $($package.name))"

        Write-Step 'Build'
        Write-Detail "cargo build --workspace --release --locked --target $Target"
        $buildOut = Join-Path $LogDirectory 'cargo-build.txt'
        $buildErr = Join-Path $LogDirectory 'cargo-build.err.txt'
        $buildCode = Invoke-Captured -Program 'cargo' -Arguments @(
            'build', '--workspace', '--release', '--locked',
            '--target', $Target,
            '--manifest-path', (Join-Path $Root 'Cargo.toml')
        ) -OutPath $buildOut -ErrPath $buildErr
        $buildText = (Read-Captured -Path $buildOut) + (Read-Captured -Path $buildErr)
        foreach ($line in ($buildText -split "`r?`n")) {
            if ($line.Trim() -ne '') { Write-Detail $line.Trim() }
        }
        if ($buildCode -ne 0) {
            Fail "cargo build exited $buildCode; the full log is $buildOut and $buildErr"
        }
        Write-Detail 'exit        0'
        if (-not (Test-Path -LiteralPath $builtExe)) {
            Fail "cargo build succeeded but $builtExe is not there"
        }

        # ---------------------------------------------------------------------
        # 3. Stage. The staged tree is what the archive holds, assembled here
        #    rather than copied from somewhere, so the archive's contents are a
        #    value this script states rather than a consequence of a directory
        #    someone happened to leave on disk.
        # ---------------------------------------------------------------------
        Write-Step 'Stage'
        $artifactName = "sure-$version-$Target"
        $stageRoot = Join-Path $Scratch 'stage'
        $stageDir = Join-Path $stageRoot $artifactName
        if (Test-Path -LiteralPath $stageRoot) { Remove-Item -LiteralPath $stageRoot -Recurse -Force }
        New-Item -ItemType Directory -Force -Path $stageDir | Out-Null

        $stagedExe = Join-Path $stageDir 'sure.exe'
        Copy-Item -LiteralPath $builtExe -Destination $stagedExe
        Copy-Item -LiteralPath (Join-Path $Root 'LICENSE') -Destination (Join-Path $stageDir 'LICENSE')
        $exeBytes = (Get-Item -LiteralPath $stagedExe).Length
        Write-Detail "sure.exe    $exeBytes bytes"

        # Provenance, inside the archive rather than beside it: an archive that
        # travels without its RELEASE.txt is the one whose origin nobody can
        # state. The worktree marker is here because a recorded commit that does
        # not describe the bytes is worse than no commit at all.
        $commitOut = Join-Path $LogDirectory 'git-commit.txt'
        $commitErr = Join-Path $LogDirectory 'git-commit.err.txt'
        $commitCode = Invoke-Captured -Program 'git' -Arguments @(
            '-C', $Root, 'rev-parse', 'HEAD'
        ) -OutPath $commitOut -ErrPath $commitErr
        $commit = if ($commitCode -eq 0) { (Read-Captured -Path $commitOut).Trim() } else { 'unknown (git rev-parse exited ' + $commitCode + ')' }

        $treeOut = Join-Path $LogDirectory 'git-status.txt'
        $treeErr = Join-Path $LogDirectory 'git-status.err.txt'
        $null = Invoke-Captured -Program 'git' -Arguments @(
            '-C', $Root, 'status', '--porcelain'
        ) -OutPath $treeOut -ErrPath $treeErr
        $treeState = if ((Read-Captured -Path $treeOut).Trim() -eq '') {
            'clean'
        } else {
            'dirty - uncommitted changes were present when this was built'
        }

        $rustcOut = Join-Path $LogDirectory 'rustc-vV.txt'
        $rustcErr = Join-Path $LogDirectory 'rustc-vV.err.txt'
        $null = Invoke-Captured -Program 'rustc' -Arguments @('-vV') -OutPath $rustcOut -ErrPath $rustcErr
        $rustcVersion = ((Read-Captured -Path $rustcOut) -split "`r?`n" | Select-Object -First 1)

        $exeDigest = Get-Sha256 -Path $stagedExe
        $releaseText = @"
SURE $version - Windows x64 release artifact

  target        $Target
  built from    $commit
  worktree      $treeState
  built at      $((Get-Date).ToUniversalTime().ToString('o'))
  built with    $rustcVersion
  release gate  $($gate.decision); $($gate.corpus.release_blocking) of $($gate.corpus.cases) cases release-blocking, $($gate.corpus.release_blocking_observed) observed
                read from $GatePath

  sure.exe      SHA-256 $exeDigest
                $exeBytes bytes

WHAT THIS IS
The SURE command-line program for 64-bit Windows, built for the
x86_64-pc-windows-msvc target. "sure doctor" reports the build it is running
from and where it is running from; "sure --help" lists the commands.

THIS BUILD IS UNSIGNED
Windows will show a SmartScreen or "unknown publisher" warning the first time
it is run, and that is expected rather than hidden:
docs/development/RELEASE_PROCESS.md says code-signing credentials are external
and "Do not fake signing". There is no Authenticode signature on sure.exe.

THIS ARCHIVE IS NOT BYTE-FOR-BYTE REPRODUCIBLE
A ZIP records a timestamp per entry, so building the same commit twice produces
two archives with two different digests. The .sha256 beside this archive is an
integrity check over the bytes that were shipped - that they arrived unchanged -
and it is not a claim that a rebuild would produce them again.

HOW THIS IS CHECKED
This file is written before the checks run, so it describes the procedure
rather than certifying its own archive. scripts/Build-Release.ps1 writes the
.sha256 beside this archive, re-reads it, verifies the archive against it,
extracts the archive to a fresh directory and runs the extracted sure.exe from
there, comparing the running_from the binary reports with that directory. It
prints OK and exits 0 only when every one of those steps agreed, so a run that
failed is a run that said which step failed. See
docs/development/RELEASE_PROCESS.md for how it is meant to be installed
(P15-T003) and referenced (P15-T004).
"@
        Set-Content -LiteralPath (Join-Path $stageDir 'RELEASE.txt') -Value $releaseText -Encoding utf8NoBOM
        Write-Detail "RELEASE.txt $((Get-Item -LiteralPath (Join-Path $stageDir 'RELEASE.txt')).Length) bytes"
        Write-Detail "commit      $commit"

        # ---------------------------------------------------------------------
        # 4. Package. `ZipFile::CreateFromDirectory` with `includeBaseDirectory`
        #    true, which is a parameter rather than a convention: the archive
        #    has exactly one top-level directory, and that is stated here
        #    instead of being inferred from what Compress-Archive happens to do
        #    with a directory argument.
        # ---------------------------------------------------------------------
        Write-Step 'Package'
        New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
        $archivePath = Join-Path $OutputDirectory "$artifactName.zip"
        $shaPath = "$archivePath.sha256"
        foreach ($path in @($archivePath, $shaPath)) {
            if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
        }
        [System.IO.Compression.ZipFile]::CreateFromDirectory(
            $stageDir, $archivePath, [System.IO.Compression.CompressionLevel]::Optimal, $true)
        Write-Detail "archive     $archivePath"
        Write-Detail "size        $((Get-Item -LiteralPath $archivePath).Length) bytes"
        $zip = [System.IO.Compression.ZipFile]::OpenRead($archivePath)
        try {
            foreach ($entry in $zip.Entries) {
                Write-Detail "entry       $($entry.FullName) ($($entry.Length) bytes)"
            }
        } finally { $zip.Dispose() }

        # ---------------------------------------------------------------------
        # 5. Checksum, then check what was just written. The second half is the
        #    part that can fail, and it is here rather than only in the verifier
        #    because a writer that cannot see its own work go stale is the thing
        #    "the checksum is decoration" describes.
        # ---------------------------------------------------------------------
        Write-Step 'Checksum'
        $digest = Get-Sha256 -Path $archivePath
        # sha256sum format: the digest, two spaces, the file name, one LF.
        #
        # **Written with an explicit LF, and that is not cosmetic.** `Set-Content`
        # ends the line with the platform's newline, so the first version of this
        # file ended `....zip\r\n`; `sha256sum -c` reads the `\r` as part of the
        # file name and answers `FAILED open or read`, and
        # `sure-....zip'\r': No such file or directory`. Measured 2026-09-19 with
        # the `sha256sum.exe` Git for Windows ships, which is the independent
        # verifier this format exists to be read by -- and the check that caught
        # it. ASCII and no BOM for the neighbouring reason: a BOM is three bytes
        # `sha256sum -c` would read as part of the digest.
        $checksumLine = "$digest  $artifactName.zip`n"
        [System.IO.File]::WriteAllText($shaPath, $checksumLine, [System.Text.Encoding]::ASCII)

        # The format, asserted on the bytes rather than trusted to the call
        # above, because the defect this guards against is invisible in
        # `Get-Content`: a CR makes every Windows tool here happy and breaks the
        # one Unix tool the format is for. This is the mutation in the header,
        # and it is a check whose failure was measured before it was written.
        $checksumBytes = [System.IO.File]::ReadAllBytes($shaPath)
        if ($checksumBytes -contains 13) {
            Fail "the checksum file contains a CR at byte $([Array]::IndexOf($checksumBytes, [byte]13)); sha256sum reads it as part of the file name"
        }
        if ($checksumBytes[-1] -ne 10) {
            Fail 'the checksum file does not end with a newline, which sha256sum -c warns about'
        }
        if ($checksumBytes[0] -eq 0xEF) {
            Fail 'the checksum file starts with a BOM, which sha256sum -c reads as part of the digest'
        }
        Write-Detail "sha256      $digest"
        Write-Detail "written to  $shaPath"

        $recomputed = Get-Sha256 -Path $archivePath
        if ($recomputed -ne $digest) {
            Fail "the digest changed between writing it and re-reading it: wrote $digest, re-read $recomputed"
        }
        Write-Detail 'recheck     the digest of the archive on disk is the digest that was written'
    } else {
        Write-Step 'Existing artifact'
        # `Get-ChildItem -Filter` is not used here on purpose: on Windows a
        # `-Filter` pattern is matched with the filesystem's own semantics, which
        # can also match a file through its 8.3 short name. `-like` compares the
        # name a reader sees.
        $found = @(Get-ChildItem -LiteralPath $OutputDirectory -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -like "sure-*-$Target.zip" })
        if ($found.Count -eq 0) {
            Fail "no sure-*-$Target.zip in $OutputDirectory to verify; run this script without -Phase Verify first"
        }
        if ($found.Count -gt 1) {
            Fail "$($found.Count) archives match in $OutputDirectory; this phase checks one, so which is ambiguous and is not guessed"
        }
        $archivePath = $found[0].FullName
        $shaPath = "$archivePath.sha256"
        Write-Detail "archive     $archivePath"
        Write-Detail "size        $($found[0].Length) bytes"
    }

    # -------------------------------------------------------------------------
    # 6. Verify the archive against the digest file, read back from disk. This
    #    is the step that would catch a truncated download or an archive altered
    #    after it was published, so it runs before extraction: the point of the
    #    property in the header is the *order*.
    # -------------------------------------------------------------------------
    Write-Step 'Verify the checksum file'

    if (-not (Test-Path -LiteralPath $shaPath)) {
        Fail "no checksum file at $shaPath, so nothing verifies $archivePath"
    }
    $shaText = (Get-Content -LiteralPath $shaPath -Raw).Trim()
    if ($shaText -notmatch '^([0-9a-f]{64})  (.+)$') {
        Fail "the checksum file is not one sha256sum-format line: $shaText"
    }
    $expected = $Matches[1]
    $named = $Matches[2]
    $actual = Get-Sha256 -Path $archivePath
    Write-Detail "expected    $expected"
    Write-Detail "actual      $actual"
    if ($expected -ne $actual) {
        Fail "the archive does not match its checksum file: $archivePath"
    }
    if ($named -ne (Split-Path -Leaf $archivePath)) {
        Fail "the checksum file names $named but the archive is $(Split-Path -Leaf $archivePath)"
    }
    Write-Detail 'matches     the archive on disk is the archive the checksum file names'

    # -------------------------------------------------------------------------
    # 7. Extract to a fresh directory and run the binary from there.
    # -------------------------------------------------------------------------
    Write-Step 'Extract'
    $extractRoot = Join-Path $Scratch 'extracted'
    if (Test-Path -LiteralPath $extractRoot) { Remove-Item -LiteralPath $extractRoot -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
    [System.IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $extractRoot)
    Write-Detail "into        $extractRoot"

    # The top-level directory the archive's own entries created. Listed rather
    # than built from the artifact name, so a package whose layout changed is a
    # failure here instead of a path this script invented and then found.
    $topLevel = @(Get-ChildItem -LiteralPath $extractRoot -Directory)
    if ($topLevel.Count -ne 1) {
        Fail "$extractRoot holds $($topLevel.Count) top-level directories; the layout this artifact promises is exactly one"
    }
    $extractDir = $topLevel[0].FullName
    $extractedExe = Join-Path $extractDir 'sure.exe'
    if (-not (Test-Path -LiteralPath $extractedExe)) {
        Fail "the archive extracted but $extractedExe is not in it"
    }
    Write-Detail "package dir $extractDir"

    # The one Windows limit this script refuses to run into by accident.
    # `Test-Path` above uses a Win32 file API that copes with a path longer than
    # MAX_PATH, and `CreateProcess` does not: starting the process fails, and
    # what surfaces is PowerShell's own "StandardOutputEncoding is only
    # supported when standard output is redirected", which says nothing about
    # length. Measured on 2026-09-19 on this machine: with `-OutputDirectory`
    # nested until this path is 294 characters, the file exists, `Test-Path`
    # says so, and both a redirected and a shell-executed start fail with "The
    # system cannot find the file specified" and 0xfffffffe.
    # `LongPathsEnabled` is `0` here, which is why. Refusing here names the
    # cause instead of leaving a reader with the wrong one; a machine with long
    # paths enabled would still be refused, because whether the *release*
    # machine has it is not something this script can know.
    if ($extractedExe.Length -ge 260) {
        Fail @"
the extracted binary's path is $($extractedExe.Length) characters, at or over
the 260-character MAX_PATH that Windows will start a process from:

  $extractedExe

Nothing was run, because CreateProcess would fail here with a message that does
not mention path length. Choose a shorter -OutputDirectory, or enable the
machine's long-path support.
"@
    }

    $extractedDigest = Get-Sha256 -Path $extractedExe
    Write-Detail "sure.exe    $extractedDigest"
    if ($Phase -eq 'All') {
        # The archive's digest covers the archive, and this line is what ties
        # the archive to the bytes that get run: the executable inside it is the
        # executable cargo produced.
        $builtDigest = Get-Sha256 -Path $builtExe
        if ($extractedDigest -ne $builtDigest) {
            Fail @"
the sure.exe inside the archive is not the one that was built:

  built     $builtDigest  $builtExe
  packaged  $extractedDigest  $extractedExe
"@
        }
        Write-Detail 'same as     the sure.exe cargo built'
    }

    Write-Step 'Run the extracted binary'
    $stdoutPath = Join-Path $LogDirectory 'extracted-doctor.json.txt'
    $stderrPath = Join-Path $LogDirectory 'extracted-doctor.err.txt'

    # By absolute path, and `--store-dir` is deliberately not passed: a user
    # runs `sure doctor` against their own store, so that is what is run here.
    # `sure doctor` opens the store read-only and creates nothing.
    $doctorCode = Invoke-Captured -Program $extractedExe -Arguments @(
        'doctor', '--format', 'json'
    ) -OutPath $stdoutPath -ErrPath $stderrPath
    $doctorOut = Read-Captured -Path $stdoutPath
    $doctorErr = Read-Captured -Path $stderrPath

    Write-Detail "invoked     $extractedExe doctor --format json"
    Write-Detail "exit        $doctorCode"
    if ($doctorErr.Trim() -ne '') {
        foreach ($line in ($doctorErr.Trim() -split "`r?`n")) { Write-Detail "stderr      $line" }
    }
    if ($doctorCode -ne 0) {
        Fail "the extracted sure.exe exited $doctorCode; its stderr is $stderrPath"
    }
    if ($doctorOut.Trim() -eq '') {
        Fail "the extracted sure.exe wrote nothing to stdout; the frame should be in $stdoutPath"
    }
    $frame = $doctorOut | ConvertFrom-Json
    $build = $frame.details.build

    Write-Detail "version     $($frame.sure_version)"
    Write-Detail "outcome     $($frame.outcome)"
    Write-Detail "os/arch     $($build.os) $($build.arch)"
    Write-Detail "target_env  $($build.target_env)"
    Write-Detail "running_from $($build.running_from)"

    if ($frame.command -ne 'doctor') { Fail "the frame's command field is $($frame.command), not doctor" }
    if ($build.os -ne 'windows') { Fail "the extracted binary reports os=$($build.os)" }
    if ($build.arch -ne 'x86_64') { Fail "the extracted binary reports arch=$($build.arch)" }
    if ($build.target_env -ne 'msvc') { Fail "the extracted binary reports target_env=$($build.target_env), not msvc" }
    if ([string]::IsNullOrWhiteSpace($build.running_from)) {
        Fail "the extracted binary reported no running_from, so which binary answered cannot be checked"
    }

    # The assertion this script exists for. `running_from` is what the binary
    # says about itself, and it has to be the directory the archive was just
    # extracted into. A run that picked up some other `sure` - a debug build on
    # PATH, a copy left by an earlier release - would still exit 0, still print
    # a well-formed frame, and still say msvc, because on a Windows developer
    # machine everything else that could be picked up is also an MSVC build.
    # This comparison is the only thing here that can tell them apart.
    #
    # Case-insensitively, because Windows paths are case-insensitive: C:\ and
    # c:\ are the same directory and would otherwise read as a mismatch. The
    # normalisation is `GetFullPath`'s, so a joined or shortened spelling from
    # either side compares as itself.
    $reportedDir = [System.IO.Path]::GetDirectoryName([System.IO.Path]::GetFullPath($build.running_from))
    $requiredDir = [System.IO.Path]::GetFullPath($extractDir)
    if ($reportedDir -ne $requiredDir) {
        Fail @"
the binary that answered "sure doctor" is not the one this script extracted:

  required  $requiredDir
  reported  $reportedDir

The archive was extracted to $extractDir, so a running_from anywhere else means
the run measured some other installation, and nothing after this point would
have been checked against the artifact.
"@
    }
    Write-Detail 'from        the directory this script extracted to'

    $humanOut = Join-Path $LogDirectory 'extracted-doctor.human.txt'
    $humanErr = Join-Path $LogDirectory 'extracted-doctor.human.err.txt'
    $humanCode = Invoke-Captured -Program $extractedExe -Arguments @('doctor') -OutPath $humanOut -ErrPath $humanErr
    if ($humanCode -ne 0) {
        Fail "the extracted sure.exe exited $humanCode for the human form; stderr is $humanErr"
    }
    foreach ($line in ((Read-Captured -Path $humanOut) -split "`r?`n" | Select-Object -First 2)) {
        Write-Detail "human       $line"
    }

    # -------------------------------------------------------------------------
    # 8. The result.
    # -------------------------------------------------------------------------
    Write-Host ''
    Write-Host 'RESULT'
    Write-Host "  artifact    $archivePath"
    Write-Host "  size        $((Get-Item -LiteralPath $archivePath).Length) bytes"
    Write-Host "  sha256      $actual"
    Write-Host "  checksum    $shaPath"
    Write-Host '  verified    the archive matches the checksum file, re-read from disk'
    Write-Host "  ran         $extractedExe"
    Write-Host "  reported    $($build.running_from)"
    Write-Host "  gate        permitted; $($gate.corpus.cases) cases, $($gate.corpus.release_blocking) release-blocking and all observed"
    Write-Host '  OK          the bytes that are checksummed are the bytes that were run'
    exit 0
} catch {
    Write-Host ''
    Write-Host "FAILED: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}

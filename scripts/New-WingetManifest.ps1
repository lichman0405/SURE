# SURE - render the WinGet manifest from a release archive, and check the result.
#
#   & .\scripts\New-WingetManifest.ps1 -Archive <sure-<version>-x86_64-pc-windows-msvc.zip>
#   & .\scripts\New-WingetManifest.ps1 -Phase Verify -ManifestDirectory <dir> -Archive <the same archive>
#
# Run from anywhere; every path is derived from this file's own location.
#
# =============================================================================
# What this script is for, and what it refuses to be
# =============================================================================
#
# A WinGet manifest is a set of confident claims about a file that is not in
# this repository: a version, a URL and a SHA-256, all about bytes nobody in the
# checkout can see. Every one of them is filled in here from something that
# exists at the moment it is filled in, and checked before it is written:
#
#   PackageVersion        the version the `sure.exe` **inside the archive**
#                         reports about itself through `sure version --format
#                         json`. It is required to equal the version in the
#                         archive's file name, so a manifest cannot describe a
#                         binary other than the one it packages. If they
#                         disagree the script stops and prints both strings.
#   InstallerSha256       the SHA-256 of the archive's bytes, recomputed here
#                         rather than copied, and required to match the
#                         `.sha256` file beside the archive. A digest that is
#                         not over these bytes is the one thing this script must
#                         never write, and the recomputation is the reason it
#                         cannot.
#   InstallerUrl          derived from the repository URL in `Cargo.toml`, the
#                         version, and the archive's own file name.
#   RelativeFilePath      the archive's real layout, asserted by extracting it
#                         and looking. A path that is not in the archive fails
#                         on a user's machine *after* the download, which is why
#                         it is checked here.
#   PackageIdentifier,    the repository's own metadata (`Cargo.toml`'s
#   PublisherUrl,         `repository` and `license`), checked field by field
#   PackageUrl, License   against `Cargo.toml` before anything is written. A
#                         fork that changes the repository is stopped here
#                         rather than publishing under an identity it does not
#                         own.
#
# There is no digest written down in `packaging/winget/template/`, and this
# script would not use one if there were: it renders `<sha256-of-the-archive>`
# from the bytes it just read. A 64-hex constant in a committed file that no
# build produced is the artefact SURE exists to refuse.
#
# =============================================================================
# What this script does NOT check, named rather than left to be assumed
# =============================================================================
#
# * **That the release exists.** `InstallerUrl` points at a GitHub release asset
#   under the tag `v<version>`. Nothing in this repository creates that tag or
#   that release yet (`P15-T011` is the workflow that will), and this script has
#   no network access by design. What it checks is that the URL is the one the
#   repository URL, the version and the archive's name imply; whether a `GET` of
#   it returns the archive is checked by whoever publishes, and by the first
#   user who runs `winget install`, as a 404.
# * **That WinGet installs it correctly.** That would need `winget install`,
#   which this repository's rules do not allow a script or a test to run on the
#   machine it is developed on. `winget validate` reads a manifest and does not
#   download, install or change anything; it is the strongest check available
#   here, and it is weaker than an install.
# * **That the build is signed.** It is not signed. See `## Signing` in
#   `docs/development/RELEASE_PROCESS.md`.
#
# `docs/development/INSTALL_WINGET.md` is where all of that is written down for a
# reader rather than for whoever changes this script: what the package would
# install, where the files land, which of the launchers' three resolution steps
# finds it, every value's provenance, what a fork has to change, what an
# uninstall removes and leaves, and what none of it covers.
#
# =============================================================================
# `winget` is an optional tool, and a check that did not run says so
# =============================================================================
#
# `winget validate` is the last step of both phases. When `winget` is not on the
# machine, this script prints a `--- NOT CHECKED ---` block, exits **3** rather
# than 0, and does not print OK. A skipped dynamic check is not a pass: the
# manifest it produced in that case is worth exactly the checks that ran, and
# the caller is told which one did not.
#
# =============================================================================
# The two phases
# =============================================================================
#
# `All` (the default) renders `packaging/winget/template/` into a directory of
# manifests and checks them. `Verify` writes nothing: it reads a directory of
# already-rendered manifests and re-derives every value from the archive, so
# that "the writer checked its own work" and "something else re-read it later"
# are different claims and the second one is available. `Verify` is what a
# person runs against a manifest this script did not write.
#
# Both phases need the archive: a manifest is a claim about those bytes, and a
# check that does not read them is not a check of that claim.
#
# =============================================================================
# Windows discipline (CLAUDE.md)
# =============================================================================
#
# Every process is started through `Invoke-Captured` with a typed argument array
# and file redirection - no shell string is built from a path, which matters
# because the paths here carry spaces and non-ASCII characters
# (`crates/sure-cli/tests/winget_manifest.rs` runs this script under exactly
# that kind of path). The SHA-256 is computed with .NET rather than
# `Get-FileHash`, for the reason `scripts/Install-Sure.ps1`'s `Get-Sha256`
# records: a PowerShell 7 session's `PSModulePath` reaching Windows PowerShell
# 5.1 leaves `Get-FileHash` undefined there while everything else this script
# uses still resolves. The extraction path is refused when it reaches MAX_PATH,
# because `CreateProcess` fails there with a message that does not mention path
# length (`Build-Release.ps1` measured it, and both scripts refuse rather than
# leave a reader with the wrong reason).

[CmdletBinding()]
param(
    # The release archive this manifest is about. Named
    # `sure-<version>-<target>.zip`; anything else is refused rather than
    # guessed at, because the version in this name is the version the whole
    # manifest is written for.
    [Parameter(Mandatory)][string] $Archive,

    # The `sha256sum`-format file beside the archive. Defaults to
    # `<archive>.sha256`, which is where `scripts/Build-Release.ps1` writes it.
    [string] $Checksum = '',

    # `All` renders and checks. `Verify` checks a directory that was already
    # rendered, writes nothing, and needs `-ManifestDirectory`.
    [ValidateSet('All', 'Verify')]
    [string] $Phase = 'All',

    # Where `All` writes the rendered manifests. Default: under the git-ignored
    # `target/tmp`, in the `<identifier>\<version>` shape WinGet publishes in.
    [string] $OutputDirectory = '',

    # The directory of rendered manifests `Verify` reads. Required by `Verify`
    # and refused by `All`, rather than ignored.
    [string] $ManifestDirectory = '',

    # Where the archive is extracted and the checks' output is captured.
    # Default: a directory of this run's own under `<output>\scratch`, named with
    # the process id and a random token. One run's directory is nobody else's, so
    # two runs at once cannot read or clear each other's files; the same property
    # is what stops a run that does not extract from reading a binary an earlier
    # run left behind. Passed explicitly, it is used as given.
    [string] $ScratchDirectory = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# PowerShell 7.3+ can turn a native command's stderr into a terminating error.
# Off explicitly: programs here write to stderr on success.
$PSNativeCommandUseErrorActionPreference = $false

# The manifest schema version this script writes and checks against. Measured
# against winget 1.29.290 on 2026-09-20, with the rendered files and only this
# value changed: 1.12.0 is answered with "Manifest validation succeeded." and
# exit 0, and so is 1.10.0; 1.11.0 and 1.13.0 are answered with "Manifest
# validation succeeded with warnings", the warning being "The schema header URL
# does not match the expected pattern" on each file's
# `# yaml-language-server: $schema=...` line, and exit -1978335192. A warning is
# not a pass here, so the value that is one on this machine is the one written.
# Which versions another `winget` expects is that machine's answer, not this
# script's; what this script can state is that it writes one value and checks
# the files against it (`$fileManifestVersion -ne $SchemaVersion` below), and
# `crates/sure-cli/tests/winget_manifest.rs` compares it with the templates.
$SchemaVersion = '1.12.0'

# The one target this repository produces, and the architecture WinGet calls it.
# A target that is not in this map is refused rather than mapped by a guess: the
# `Architecture` field is what a user's machine is matched against, so getting it
# wrong installs an x64 build on a machine that asked for something else.
$TargetArchitectures = @{ 'x86_64-pc-windows-msvc' = 'x64' }

# The three files a multi-file manifest is, by their `ManifestType`.
$ManifestTypes = @('version', 'installer', 'defaultLocale')

# The fence around the notes a template carries for a reader of this repository.
# Everything between the two lines, inclusive, is removed when the file is
# rendered: a published manifest should not open with "THIS IS A TEMPLATE", and
# a comment that says "there is no digest written down here" is not something to
# ship next to a digest. The notes stay in the template, where they are read.
# Both lines have to be present exactly once, so a template that lost its fence
# is refused rather than rendered with its notes still in it.
$NotesBegin = '# >>> template notes: removed when this file is rendered'
$NotesEnd = '# <<< end of template notes'

# The two placeholders the templates are allowed to carry, and the only strings
# this script substitutes. Anything else in angle brackets that survives
# rendering is a refusal: a manifest published with a placeholder in a field is
# worse than one that was never written.
$VersionToken = '<version>'
$DigestToken = '<sha256-of-the-archive>'

$Root = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$TemplateDirectory = Join-Path $Root 'packaging\winget\template'
$CargoManifest = Join-Path $Root 'Cargo.toml'

# `[IO.Path]::GetFullPath` rather than `Resolve-Path` throughout: `Resolve-Path`
# treats a `[` or `]` in a directory name as a wildcard, and nothing here should
# care what the caller's directories are called.
$Archive = [System.IO.Path]::GetFullPath($Archive)
if ($Checksum -eq '') { $Checksum = "$Archive.sha256" }
$Checksum = [System.IO.Path]::GetFullPath($Checksum)
if ($OutputDirectory -eq '') { $OutputDirectory = Join-Path $Root 'target\tmp\winget' }
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
# Per run, not per directory. A `-Phase Verify` call takes no `-OutputDirectory`
# — it writes nothing — so a scratch directory derived only from the output
# directory is one name that every `Verify` on the machine shares, and two runs
# at once (a test binary running its cases in parallel, a build matrix) collide
# on the files inside it. The failure reads "The process cannot access the file
# 'sure-version.json.txt' because it is being used by another process" and names
# neither of the two runs. `crates/sure-cli/tests/winget_manifest.rs` found this
# by running its cases in parallel, which is how `cargo test` runs them.
#
# Unique per run also strengthens the reason the directory exists at all: a run
# that does not extract cannot read a binary an earlier run left behind, because
# there is no earlier run's directory under this name to read.
if ($ScratchDirectory -eq '') {
    $ScratchDirectory = Join-Path (Join-Path $OutputDirectory 'scratch') ("run-{0}-{1}" -f $PID, [System.IO.Path]::GetRandomFileName())
}
$ScratchDirectory = [System.IO.Path]::GetFullPath($ScratchDirectory)
if ($ManifestDirectory -ne '') { $ManifestDirectory = [System.IO.Path]::GetFullPath($ManifestDirectory) }

function Write-Step {
    param([Parameter(Mandatory)][string] $Text)
    Write-Host ''
    Write-Host "== $Text"
}

function Write-Detail {
    param([Parameter(Mandatory)][string] $Text)
    Write-Host "   $Text"
}

function Fail {
    param([Parameter(Mandatory)][string] $Text)
    Write-Host ''
    Write-Host "FAILED: $Text"
    exit 1
}

# The check that did not run. Exit 3, which is neither 0 nor a refusal, so a
# caller can tell "the manifests are rendered and validated" from "the manifests
# are rendered and one of the checks never happened".
function NotChecked {
    param([Parameter(Mandatory)][string] $Text)
    Write-Host ''
    Write-Host '--- NOT CHECKED ---'
    Write-Host ''
    Write-Host $Text
    exit 3
}

# Run one program with a typed argument array and both streams sent to files.
# Returns the exit status; never returns or throws on the text. The caller reads
# the files. This is the only place in this script that starts a process, so the
# redirection rule in the header is enforced in one function.
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
# file was written. Both are real answers; `Get-Content -Raw` returns `$null`,
# not `''`, for a file of zero bytes.
function Read-Captured {
    param([Parameter(Mandatory)][string] $Path)
    if (-not (Test-Path -LiteralPath $Path)) { return '' }
    $text = Get-Content -LiteralPath $Path -Raw -ErrorAction SilentlyContinue
    if ($null -eq $text) { return '' }
    return $text
}

function Get-FileText {
    param([Parameter(Mandatory)][string] $Path)
    $text = Get-Content -LiteralPath $Path -Raw -ErrorAction SilentlyContinue
    if ($null -eq $text) { return '' }
    return $text
}

function Get-Sha256 {
    param([Parameter(Mandatory)][string] $Path)
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $hasher = [System.Security.Cryptography.SHA256]::Create()
        try {
            $digest = $hasher.ComputeHash($stream)
        } finally {
            $hasher.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
    # `BitConverter` spells it uppercase with dashes; `sha256sum` and the
    # `.sha256` file this repository writes both want lowercase with none.
    return ([System.BitConverter]::ToString($digest) -replace '-', '').ToLowerInvariant()
}

# One field out of a manifest, read as text.
#
# Text rather than a parsed YAML object, and that is a limit worth naming:
# neither Windows PowerShell nor PowerShell 7 parses YAML, and `ConvertFrom-Yaml`
# is not built in. These files are written by this repository and read by
# WinGet, and what this script needs from them is a handful of scalar values on
# lines of their own. The optional `- ` covers the list form (`  - Architecture:
# x64`); a comment line cannot match, because a `#` before the name leaves it
# outside the pattern. A field that appears zero times, or more than once, is a
# refusal rather than a pick: which one was meant would be a guess, and guessing
# is what this script exists not to do.
function Get-ManifestField {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Name
    )
    $text = Get-FileText -Path $Path
    $found = [regex]::Matches($text, "(?m)^[ \t]*(?:-[ \t]+)?$([regex]::Escape($Name)):[ \t]*(?<value>\S.*?)[ \t]*$")
    if ($found.Count -ne 1) {
        Fail @"
$Path carries $($found.Count) '$Name' lines, and every value this script reads
has to appear exactly once in the file.
"@
    }
    $value = $found[0].Groups['value'].Value
    if ($value.Contains('#')) {
        Fail "$Path's '$Name' is followed by something that looks like a comment, and this script reads these files as text: $value"
    }
    return $value
}

# WinGet's rule for what the three files are called, as a function of the
# package identifier: `<id>.yaml`, `<id>.installer.yaml` and
# `<id>.locale.<locale>.yaml`. The name a rendered file gets is decided here and
# not taken from the template's own file name, so that a fork which changes
# `PackageIdentifier` gets correctly-named files rather than files named after
# whoever the template was written for.
function Get-ManifestFileName {
    param(
        [Parameter(Mandatory)][string] $Identifier,
        [Parameter(Mandatory)][string] $ManifestType,
        [Parameter(Mandatory)][string] $PackageLocale
    )
    switch ($ManifestType) {
        'version' { return "$Identifier.yaml" }
        'installer' { return "$Identifier.installer.yaml" }
        'defaultLocale' { return "$Identifier.locale.$PackageLocale.yaml" }
        default { Fail "no file name is recorded for ManifestType $ManifestType" }
    }
}

# A directory of manifests, as a table from `ManifestType` to file path.
#
# The type is read out of each file rather than inferred from its name, so this
# works the same way on the template directory (where the names are the
# template's) and on a rendered directory (where they are the identifier's). A
# missing type, an unknown type, or two files claiming the same type are all
# refusals: `winget validate` reads a directory, and a directory that is not
# exactly these three files is not the thing this script is checking.
function Get-ManifestsByType {
    param([Parameter(Mandatory)][string] $Directory)
    if (-not (Test-Path -LiteralPath $Directory -PathType Container)) {
        Fail "no directory of manifests at $Directory"
    }
    # `Where-Object`/`-like` rather than `-Filter`, because on Windows a `-Filter`
    # pattern is matched with the filesystem's own semantics, which can match a
    # file through its 8.3 short name as well (`Build-Release.ps1` records the
    # same rule).
    $files = @(Get-ChildItem -LiteralPath $Directory -File | Where-Object { $_.Name -like '*.yaml' })
    if ($files.Count -eq 0) { Fail "no '*.yaml' file in $Directory" }
    $byType = @{}
    foreach ($file in $files) {
        $type = Get-ManifestField -Path $file.FullName -Name 'ManifestType'
        if ($ManifestTypes -notcontains $type) {
            Fail "$($file.Name) says ManifestType $type; a manifest here is one of $($ManifestTypes -join ', ')"
        }
        if ($byType.ContainsKey($type)) {
            Fail "two files say ManifestType ${type}: $($byType[$type]) and $($file.FullName)"
        }
        $byType[$type] = $file.FullName
    }
    foreach ($type in $ManifestTypes) {
        if (-not $byType.ContainsKey($type)) { Fail "no file in $Directory says ManifestType $type" }
    }
    return $byType
}

# A template's text, without its fence and the notes inside it, ending with one
# newline. The fence is required: a template that does not carry both lines
# exactly once is refused, because the alternative is rendering a file whose
# first line tells a user it is not a manifest.
function Remove-TemplateNotes {
    param(
        [Parameter(Mandatory)][string] $Text,
        [Parameter(Mandatory)][string] $Name
    )
    if (([regex]::Matches($Text, [regex]::Escape($NotesBegin))).Count -ne 1 -or
        ([regex]::Matches($Text, [regex]::Escape($NotesEnd))).Count -ne 1) {
        Fail "$Name does not carry the template-notes fence exactly once; the two lines are $NotesBegin and $NotesEnd, and the renderer depends on them"
    }
    $kept = New-Object System.Collections.Generic.List[string]
    $inside = $false
    foreach ($line in ($Text -split "`r?`n")) {
        if ($line -eq $NotesBegin) { $inside = $true; continue }
        if ($line -eq $NotesEnd) { $inside = $false; continue }
        if (-not $inside) { [void] $kept.Add($line.TrimEnd()) }
    }
    $body = ($kept -join "`n").TrimStart("`n").TrimEnd("`n")
    if (-not $body.StartsWith('# yaml-language-server: $schema=https://aka.ms/winget-manifest.')) {
        Fail "$Name's first line after the fence is not the yaml-language-server schema comment, so a rendered manifest would not tell an editor which schema it is"
    }
    return $body
}

# The pieces of a repository URL, or a refusal.
function Split-RepositoryUrl {
    param([Parameter(Mandatory)][string] $Url)
    $uri = $null
    if (-not [System.Uri]::TryCreate($Url, [System.UriKind]::Absolute, [ref] $uri)) {
        Fail "the repository URL in Cargo.toml is not an absolute URL: $Url"
    }
    $segments = @($uri.AbsolutePath.Trim('/') -split '/')
    if ($segments.Count -lt 2 -or $segments[0] -eq '' -or $segments[1] -eq '') {
        Fail "the repository URL in Cargo.toml does not name an owner and a project: $Url"
    }
    return @{
        Owner  = $segments[0]
        Origin = "$($uri.Scheme)://$($uri.Host)"
    }
}

try {
    Write-Host 'SURE WinGet manifest'
    Write-Host "  repository  $Root"
    Write-Host "  archive     $Archive"
    Write-Host "  phase       $Phase"

    if ($Phase -eq 'All' -and $ManifestDirectory -ne '') {
        Fail "-ManifestDirectory is for -Phase Verify; -Phase All writes to -OutputDirectory, and a second destination it silently ignored would be a typo nobody was told about"
    }
    if ($Phase -eq 'Verify' -and $ManifestDirectory -eq '') {
        Fail "-Phase Verify reads a directory of rendered manifests and -ManifestDirectory is empty"
    }

    # -------------------------------------------------------------------------
    # 1. The archive, and the version in its name.
    # -------------------------------------------------------------------------
    Write-Step 'The archive'

    if (-not (Test-Path -LiteralPath $Archive -PathType Leaf)) {
        Fail "no archive at $Archive; this script renders a manifest about an artifact that exists, so with no artifact it has nothing to do"
    }
    $archiveName = Split-Path -Leaf $Archive
    Write-Detail "file        $archiveName"
    Write-Detail "size        $((Get-Item -LiteralPath $Archive).Length) bytes"

    $nameMatch = [regex]::Match($archiveName, '^sure-(?<version>.+)-(?<target>x86_64-pc-windows-msvc)\.zip$')
    if (-not $nameMatch.Success) {
        Fail @"
the archive is not named `sure-<version>-<target>.zip`:

  $archiveName

`scripts/Build-Release.ps1` decides that name and
`docs/development/RELEASE_PROCESS.md` records it. The version in it is the
version this manifest is written for, so a name this script cannot read is not
something it can work around.
"@
    }
    $Version = $nameMatch.Groups['version'].Value
    $Target = $nameMatch.Groups['target'].Value
    if (-not $TargetArchitectures.ContainsKey($Target)) {
        Fail "no architecture is recorded for target $Target in this script; add it deliberately rather than guessing which machine it installs on"
    }
    $Architecture = $TargetArchitectures[$Target]
    Write-Detail "version     $Version"
    Write-Detail "target      $Target ($Architecture)"

    # -------------------------------------------------------------------------
    # 2. The checksum file: the format it has to be in, and the digest of the
    #    bytes on disk. The digest in the manifest comes from here and nowhere
    #    else.
    # -------------------------------------------------------------------------
    Write-Step 'The checksum'

    if (-not (Test-Path -LiteralPath $Checksum -PathType Leaf)) {
        Fail "no checksum file at $Checksum, so nothing ties the manifest to these bytes"
    }

    # The format, read off the bytes rather than trusted to `Get-Content`, which
    # silently hides two of the three defects here: a BOM is stripped, and a CR
    # is a line ending to PowerShell while being part of the file name to
    # `sha256sum`. `Build-Release.ps1` asserts the same properties when it writes
    # the file; this is the reader's half of that rule.
    $checksumBytes = [System.IO.File]::ReadAllBytes($Checksum)
    if ($checksumBytes.Length -eq 0) { Fail "the checksum file $Checksum is empty" }
    if ($checksumBytes[0] -eq 0xEF) {
        Fail "the checksum file $Checksum starts with a BOM, which sha256sum reads as part of the digest"
    }
    if (@($checksumBytes | Where-Object { $_ -gt 0x7F }).Count -ne 0) {
        Fail "the checksum file $Checksum is not ASCII"
    }
    if ($checksumBytes -contains 13) {
        Fail "the checksum file $Checksum contains a CR at byte $([Array]::IndexOf($checksumBytes, [byte]13)); sha256sum reads it as part of the file name"
    }
    if ($checksumBytes[-1] -ne 10) {
        Fail "the checksum file $Checksum does not end with a newline"
    }
    $checksumText = [System.Text.Encoding]::ASCII.GetString($checksumBytes)
    $checksumLines = @($checksumText -split "`n" | Where-Object { $_ -ne '' })
    if ($checksumLines.Count -ne 1) {
        Fail "the checksum file $Checksum holds $($checksumLines.Count) lines; the format is one sha256sum line"
    }
    $checksumMatch = [regex]::Match($checksumLines[0], '^(?<digest>[0-9a-f]{64})  (?<name>.+)$')
    if (-not $checksumMatch.Success) {
        Fail "the checksum file $Checksum is not one sha256sum-format line (64 lowercase hex, two spaces, the file name): $($checksumLines[0])"
    }
    $ClaimedDigest = $checksumMatch.Groups['digest'].Value
    $NamedFile = $checksumMatch.Groups['name'].Value
    if ($NamedFile -ne $archiveName) {
        Fail "the checksum file $Checksum names $NamedFile but the archive is $archiveName, so it is a checksum of a different file"
    }

    $ActualDigest = Get-Sha256 -Path $Archive
    Write-Detail "in file     $ClaimedDigest  ($(Split-Path -Leaf $Checksum))"
    Write-Detail "recomputed  $ActualDigest  ($archiveName)"
    if ($ClaimedDigest -ne $ActualDigest) {
        Fail @"
the archive does not match its checksum file, so there is no digest of these
bytes for the manifest to carry:

  $(Split-Path -Leaf $Checksum)  says  $ClaimedDigest
  $archiveName  is    $ActualDigest
"@
    }
    Write-Detail 'matches     the digest the manifest carries is the digest of the bytes on disk'

    # -------------------------------------------------------------------------
    # 3. Extract, and find the executable the manifest names. This is the check
    #    that would otherwise happen on a user's machine, after the download.
    # -------------------------------------------------------------------------
    Write-Step 'Extract'

    Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue
    $extractRoot = Join-Path $ScratchDirectory 'extracted'
    if (Test-Path -LiteralPath $extractRoot) { Remove-Item -LiteralPath $extractRoot -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
    [System.IO.Compression.ZipFile]::ExtractToDirectory($Archive, $extractRoot)

    # The top-level directory the archive's own entries created, listed rather
    # than built from the archive's name, so that a layout which changed is a
    # failure here instead of a path this script invented and then found.
    $topLevel = @(Get-ChildItem -LiteralPath $extractRoot -Directory)
    if ($topLevel.Count -ne 1) {
        Fail "$extractRoot holds $($topLevel.Count) top-level directories; the layout this artifact promises is exactly one"
    }
    $packageDirectory = $topLevel[0].Name
    $extractedExe = Join-Path $topLevel[0].FullName 'sure.exe'
    if (-not (Test-Path -LiteralPath $extractedExe -PathType Leaf)) {
        Fail "the archive extracted but $extractedExe is not in it, so 'NestedInstallerFiles' would name a path that does not exist"
    }
    Write-Detail "package dir $packageDirectory"
    Write-Detail "sure.exe    $extractedExe"

    # The archive's documented layout also carries these two. They are reported
    # rather than asserted: the manifest does not name them, and a manifest is
    # what this script is checking.
    foreach ($extra in @('LICENSE', 'RELEASE.txt')) {
        Write-Detail "also        $extra present: $(Test-Path -LiteralPath (Join-Path $topLevel[0].FullName $extra) -PathType Leaf)"
    }

    # The one Windows limit this script refuses to run into by accident:
    # `Test-Path` uses a Win32 file API that copes with a path longer than
    # MAX_PATH and `CreateProcess` does not, and what surfaces when it fails is
    # PowerShell's own message about standard output redirection, which says
    # nothing about length.
    if ($extractedExe.Length -ge 260) {
        Fail @"
the extracted binary's path is $($extractedExe.Length) characters, at or over
the 260-character MAX_PATH that Windows will start a process from:

  $extractedExe

Nothing was run, because CreateProcess would fail here with a message that does
not mention path length. Choose a shorter -OutputDirectory or -ScratchDirectory.
"@
    }

    # -------------------------------------------------------------------------
    # 4. Run the binary that is inside the archive and take the version from it.
    #    `PackageVersion` and `sure --version` have to be the same string, or the
    #    manifest describes a different program.
    # -------------------------------------------------------------------------
    Write-Step 'The version the binary reports'

    $versionOut = Join-Path $ScratchDirectory 'sure-version.json.txt'
    $versionErr = Join-Path $ScratchDirectory 'sure-version.err.txt'
    $versionCode = Invoke-Captured -Program $extractedExe -Arguments @(
        'version', '--format', 'json'
    ) -OutPath $versionOut -ErrPath $versionErr
    $versionText = (Read-Captured -Path $versionOut).Trim()
    if ($versionCode -ne 0) {
        Fail "the binary inside the archive answered 'sure version --format json' with status ${versionCode}:$([Environment]::NewLine)$(Read-Captured -Path $versionErr)"
    }
    $versionFrame = $null
    try {
        $versionFrame = $versionText | ConvertFrom-Json
    } catch {
        Fail "the binary inside the archive did not answer 'sure version --format json' with JSON: $versionText"
    }
    if ($null -eq $versionFrame -or -not ($versionFrame.PSObject.Properties.Name -contains 'sure_version')) {
        Fail "the version frame has no 'sure_version': $versionText"
    }
    $ReportedVersion = [string] $versionFrame.sure_version
    Write-Detail "reports     $ReportedVersion"
    Write-Detail "archive     $Version"
    if ($ReportedVersion -ne $Version) {
        Fail @"
the archive is named for one version and holds another:

  file name   sure-$Version-$Target.zip
  binary      $ReportedVersion

`PackageVersion` would be a claim about a program that is not the one in the
archive, and a user who installed $Version would have $ReportedVersion. Nothing
was written.
"@
    }
    Write-Detail 'agrees      PackageVersion is the version this binary reports about itself'

    # -------------------------------------------------------------------------
    # 5. The identity, taken from the repository's own metadata.
    # -------------------------------------------------------------------------
    Write-Step 'Identity'

    $cargoText = Get-FileText -Path $CargoManifest
    if ($cargoText -eq '') { Fail "no readable Cargo.toml at $CargoManifest, so the manifest's identity has no source" }
    $repositoryMatch = [regex]::Match($cargoText, '(?m)^[ \t]*repository[ \t]*=[ \t]*"(?<value>[^"]+)"[ \t]*$')
    if (-not $repositoryMatch.Success) {
        Fail "Cargo.toml has no 'repository = ...' line, so the manifest's URLs have no source"
    }
    $licenseMatch = [regex]::Match($cargoText, '(?m)^[ \t]*license[ \t]*=[ \t]*"(?<value>[^"]+)"[ \t]*$')
    if (-not $licenseMatch.Success) {
        Fail "Cargo.toml has no 'license = ...' line, so the manifest's License has no source"
    }
    $RepositoryUrl = $repositoryMatch.Groups['value'].Value
    $License = $licenseMatch.Groups['value'].Value
    $repository = Split-RepositoryUrl -Url $RepositoryUrl
    $Owner = $repository.Owner
    $PublisherUrl = "$($repository.Origin)/$Owner"
    Write-Detail "repository  $RepositoryUrl"
    Write-Detail "license     $License"

    # Read through the same discovery the rendered directory gets, so that a
    # template set with a file missing, an unknown `ManifestType`, or two files
    # claiming one type is refused here rather than producing a directory that
    # `winget validate` would have to make sense of.
    $templates = Get-ManifestsByType -Directory $TemplateDirectory
    $Identifier = Get-ManifestField -Path $templates['version'] -Name 'PackageIdentifier'
    $PackageLocale = Get-ManifestField -Path $templates['defaultLocale'] -Name 'PackageLocale'
    $manifests = @{}
    foreach ($type in $ManifestTypes) {
        $manifests[$type] = Join-Path 'packaging/winget/template' (Split-Path -Leaf $templates[$type])
    }

    if ($Phase -eq 'All') {
        # The template is the committed statement of who publishes this. A fork
        # that changes `repository` and leaves the template alone is stopped
        # here, because a `PackageIdentifier` belongs to whoever owns the name in
        # it and the prefix is the part that has to be the owner's.
        $templateOwner = ($Identifier -split '\.')[0]
        if ($templateOwner -ine $Owner) {
            Fail @"
the package identifier claims a publisher this repository's metadata does not
name:

  $($manifests['version'])  PackageIdentifier  $Identifier
  Cargo.toml  repository  $RepositoryUrl

A `PackageIdentifier` belongs to whoever owns the name in it. A fork has to
change `PackageIdentifier`, `Publisher`, `PublisherUrl` and `PackageUrl` in all
three template files - the identifiers in them must agree with each other - and
this script will then be satisfied. Nothing was written.
"@
        }
        foreach ($type in $ManifestTypes) {
            $fileIdentifier = Get-ManifestField -Path $templates[$type] -Name 'PackageIdentifier'
            if ($fileIdentifier -ne $Identifier) {
                Fail "$($manifests[$type]) says PackageIdentifier $fileIdentifier and $($manifests['version']) says $Identifier"
            }
        }
        $localeTemplate = $templates['defaultLocale']
        $templatePackageUrl = Get-ManifestField -Path $localeTemplate -Name 'PackageUrl'
        if ($templatePackageUrl -ne $RepositoryUrl) {
            Fail "$($manifests['defaultLocale']) says PackageUrl $templatePackageUrl and Cargo.toml says $RepositoryUrl"
        }
        $templatePublisherUrl = Get-ManifestField -Path $localeTemplate -Name 'PublisherUrl'
        if ($templatePublisherUrl -ne $PublisherUrl) {
            Fail "$($manifests['defaultLocale']) says PublisherUrl $templatePublisherUrl and Cargo.toml's repository URL implies $PublisherUrl"
        }
        $templateLicense = Get-ManifestField -Path $localeTemplate -Name 'License'
        if ($templateLicense -ne $License) {
            Fail "$($manifests['defaultLocale']) says License $templateLicense and Cargo.toml says $License"
        }
        Write-Detail 'template    its identifier, its URLs and its license agree with Cargo.toml'
    }
    Write-Detail "identifier  $Identifier"
    Write-Detail "locale      $PackageLocale"

    # The URL the manifest points at, and the path inside the archive, derived
    # from three things that exist: the repository URL, the version, and what the
    # archive actually extracted to. The tag naming (`v<version>`) is
    # `docs/development/RELEASE_PROCESS.md:5-8`.
    $InstallerUrl = "$RepositoryUrl/releases/download/v$Version/$archiveName"
    $RelativeFilePath = "$packageDirectory/sure.exe"

    # -------------------------------------------------------------------------
    # 6. Render, or read what was rendered.
    # -------------------------------------------------------------------------
    if ($Phase -eq 'All') {
        Write-Step 'Render'

        $destination = Join-Path $OutputDirectory "$Identifier\$Version"
        if (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Recurse -Force }
        New-Item -ItemType Directory -Force -Path $destination | Out-Null

        $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
        foreach ($type in $ManifestTypes) {
            $source = $templates[$type]
            $rendered = Remove-TemplateNotes -Text (Get-FileText -Path $source) -Name (Split-Path -Leaf $source)
            $rendered = $rendered.Replace($VersionToken, $Version)
            $rendered = $rendered.Replace($DigestToken, $ActualDigest)

            # What a reader of the published manifest needs and the template's
            # notes cannot say: that these values were derived from that archive
            # and that hand-editing one makes it disagree with the others.
            $schemaLine, $rest = $rendered -split "`n", 2
            $rendered = @(
                $schemaLine,
                '',
                "# Rendered from $archiveName by scripts/New-WingetManifest.ps1.",
                '# Every value below is checked against that archive; re-render rather than edit.',
                '',
                $rest.TrimStart("`n")
            ) -join "`n"

            # The two tokens are the contract; anything else in angle brackets is
            # a placeholder nobody filled in, and a manifest published with one
            # is worse than no manifest.
            $leftover = [regex]::Matches($rendered, '<[a-zA-Z0-9_-]+>')
            if ($leftover.Count -ne 0) {
                $named = ($leftover | ForEach-Object { $_.Value } | Sort-Object -Unique) -join ', '
                Fail "$(Split-Path -Leaf $source) still carries $named after rendering; this script substitutes exactly two tokens, $VersionToken and $DigestToken"
            }
            $target = Join-Path $destination (Get-ManifestFileName -Identifier $Identifier -ManifestType $type -PackageLocale $PackageLocale)
            # One LF at the end, like every other text file this repository
            # writes for another program to read.
            [System.IO.File]::WriteAllText($target, ($rendered + "`n"), $utf8NoBom)
            Write-Detail "wrote       $target"
        }
        $ManifestDirectory = $destination
    } else {
        Write-Step 'Read the rendered manifests'
        if (-not (Test-Path -LiteralPath $ManifestDirectory -PathType Container)) {
            Fail "no manifest directory at $ManifestDirectory"
        }
        Write-Detail "reading     $ManifestDirectory"
    }

    # -------------------------------------------------------------------------
    # 7. Every claim in the manifests, re-derived from the archive. In `Verify`
    #    this is the whole point: the directory may have been written by someone
    #    else, or by an older run, and these are the claims that decide whether
    #    an install fetches the right bytes.
    # -------------------------------------------------------------------------
    Write-Step 'Every value, against the archive'

    $written = Get-ManifestsByType -Directory $ManifestDirectory
    $identifier = Get-ManifestField -Path $written['version'] -Name 'PackageIdentifier'
    if ($identifier -ne $Identifier) {
        Fail "the manifests in $ManifestDirectory are for $identifier; this run's template, archive and Cargo.toml imply $Identifier"
    }
    $locale = Get-ManifestField -Path $written['defaultLocale'] -Name 'PackageLocale'
    $expectedNames = @($ManifestTypes | ForEach-Object {
        Get-ManifestFileName -Identifier $identifier -ManifestType $_ -PackageLocale $locale
    })
    $actualNames = @(Get-ChildItem -LiteralPath $ManifestDirectory -File | Select-Object -ExpandProperty Name)
    $missing = @($expectedNames | Where-Object { $actualNames -notcontains $_ })
    $extra = @($actualNames | Where-Object { $expectedNames -notcontains $_ })
    if ($missing.Count -ne 0 -or $extra.Count -ne 0) {
        Fail "$ManifestDirectory holds $($actualNames -join ', '); a multi-file manifest for $identifier is exactly $($expectedNames -join ', ')"
    }

    foreach ($type in $ManifestTypes) {
        $path = $written[$type]
        $name = Split-Path -Leaf $path
        $fileVersion = Get-ManifestField -Path $path -Name 'PackageVersion'
        if ($fileVersion -ne $Version) {
            Fail "$name says PackageVersion $fileVersion and the archive is $archiveName"
        }
        $fileManifestVersion = Get-ManifestField -Path $path -Name 'ManifestVersion'
        if ($fileManifestVersion -ne $SchemaVersion) {
            Fail "$name says ManifestVersion $fileManifestVersion; this script writes $SchemaVersion, which is the schema version these checks were written against"
        }
        # The tokens, by name, so that a placeholder that survived is reported as
        # a placeholder rather than only as a mismatch against something else.
        $text = Get-FileText -Path $path
        foreach ($token in @($VersionToken, $DigestToken)) {
            if ($text.Contains($token)) { Fail "$name still carries $token" }
        }
    }
    Write-Detail "version     $Version (in all three files, in the archive's name, and in the binary)"

    $installerManifest = $written['installer']
    $installerType = Get-ManifestField -Path $installerManifest -Name 'InstallerType'
    if ($installerType -ne 'zip') {
        Fail "the installer manifest says InstallerType $installerType; the release artifact is a ZIP"
    }
    $nestedType = Get-ManifestField -Path $installerManifest -Name 'NestedInstallerType'
    if ($nestedType -ne 'portable') {
        Fail "the installer manifest says NestedInstallerType $nestedType; the archive holds one self-contained executable and no installer"
    }
    $relativePath = Get-ManifestField -Path $installerManifest -Name 'RelativeFilePath'
    if ($relativePath -ne $RelativeFilePath) {
        Fail "the installer manifest names $relativePath inside the archive; the archive extracts to $packageDirectory and holds sure.exe at $RelativeFilePath"
    }
    $alias = Get-ManifestField -Path $installerManifest -Name 'PortableCommandAlias'
    if ($alias -ne 'sure') {
        Fail "the installer manifest aliases the command as $alias; the program is named sure"
    }
    $architecture = Get-ManifestField -Path $installerManifest -Name 'Architecture'
    if ($architecture -ne $Architecture) {
        Fail "the installer manifest says Architecture $architecture and the archive is $Target, which is $Architecture"
    }
    $url = Get-ManifestField -Path $installerManifest -Name 'InstallerUrl'
    if ($url -ne $InstallerUrl) {
        Fail @"
the installer manifest points at a URL this repository's metadata does not imply:

  manifest    $url
  expected    $InstallerUrl

Whether a release exists at that URL is not something this script can check (see
the header); what it can check is that the URL is the one the repository URL, the
version and the archive's name imply, and that is what this says.
"@
    }
    $digest = Get-ManifestField -Path $installerManifest -Name 'InstallerSha256'
    if ($digest -ne $ActualDigest) {
        Fail @"
the installer manifest carries a digest that is not the digest of the archive:

  manifest    $digest
  archive     $ActualDigest  $archiveName

Nothing may install from a manifest whose checksum does not describe the bytes
it downloads.
"@
    }
    Write-Detail "sha256      $ActualDigest (the archive on disk, re-read)"
    Write-Detail "url         $url"
    Write-Detail "installer   $installerType -> $nestedType, the command alias is $alias, on $architecture"

    # -------------------------------------------------------------------------
    # 8. `winget validate`, or a loud statement that it did not run.
    # -------------------------------------------------------------------------
    Write-Step 'winget validate'

    # Found the way every other script in this repository looks for it
    # (`Install-DevDeps-Windows.ps1:24`, `Test-SureEnvironment.ps1:51`):
    # `Get-Command`, which is `PATH`. Deliberately *not* a hard-coded fallback to
    # `%LOCALAPPDATA%\Microsoft\WindowsApps\winget.exe`: that path is where the
    # alias usually lives, and reaching for it directly would mean this script
    # could never take the not-checked branch on a machine that has winget
    # installed but not on `PATH` — which is the branch that has to be
    # observable, since it is the one that is honest about a check not running.
    $wingetPath = ''
    $found = Get-Command winget -ErrorAction SilentlyContinue
    if ($found -and $found.Source) {
        $wingetPath = $found.Source
    }

    if ($wingetPath -eq '') {
        $what = 'written and checked by this script'
        if ($Phase -eq 'Verify') { $what = 'read and checked by this script' }
        NotChecked @"
winget was not found, so `winget validate --manifest` did not run against

  $ManifestDirectory

The manifests were $what, and were **not validated** against WinGet's own
schema. That is not a pass: a manifest this script is satisfied with is a
manifest this script is satisfied with.

  Install App Installer (which carries winget) from https://aka.ms/getwinget,
  or run the validation yourself:

  winget validate --manifest "$ManifestDirectory"
"@
    }

    $validateOut = Join-Path $ScratchDirectory 'winget-validate.txt'
    $validateErr = Join-Path $ScratchDirectory 'winget-validate.err.txt'
    $validateCode = Invoke-Captured -Program $wingetPath -Arguments @(
        'validate', '--manifest', $ManifestDirectory
    ) -OutPath $validateOut -ErrPath $validateErr
    $validateText = ((Read-Captured -Path $validateOut) + (Read-Captured -Path $validateErr)).Trim()
    Write-Detail "winget      $wingetPath"
    Write-Detail "directory   $ManifestDirectory"
    if ($validateCode -ne 0) {
        # `winget validate` answers a manifest it can read but not accept with
        # "Manifest validation succeeded with warnings" and a non-zero status. A
        # warning is not a pass here, so the status is the contract, and the text
        # is shown so the reason does not have to be looked up.
        Fail @"
winget validate exited ${validateCode} for ${ManifestDirectory}

$validateText
"@
    }
    Write-Detail $validateText

    Write-Host ''
    if ($Phase -eq 'All') {
        Write-Host "OK: manifests rendered to $ManifestDirectory and accepted by winget validate"
    } else {
        Write-Host "OK: every value in $ManifestDirectory matches the archive, and winget validate accepted it"
    }
    exit 0
} catch {
    Write-Host ''
    Write-Host "FAILED: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}

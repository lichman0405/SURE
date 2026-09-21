# SURE -- the seven gates, run from a clone, with the environment they ran under
# written down beside the numbers.
#
#   pwsh -NoProfile -File scripts/gates.ps1 -Label p16t001-worker
#
# Run from anywhere. The repository root is found from this file's own path and
# never from the working directory, because the version this replaces resolved
# the log directory with `Join-Path (Get-Location) 'target\tmp'` and therefore
# only worked in the one directory it was written in. That file was
# `target\tmp\gates.ps1`: untracked, inside `/target/`, and the producer of the
# `exits: fmt=0 clippy=0 test=0 bootstrap=0 taskctl=0 nonwindows=0` line that
# well over a hundred readings in `progress/` quote. No clone could run it, so
# no clone could reproduce any reading that quotes it. This file is that
# harness, in `scripts/`, where `git ls-files` finds it.
#
# WHY THE OUTPUT SHAPE IS KEPT EXACTLY AS IT WAS. The summary lines below are
# the ones the record quotes, digit for digit: `exits: ...`, `result-lines=...
# passed=... failed=... ignored=... not-ok=...`, `red: ...`, `headers (case-sensitive):
# ...`, `bootstrap: ...`, `taskctl: ...`, `store identical: ...`. A runner that changed
# what one of those means would make every reading in the record unreadable, so
# this one adds lines and changes none. Every added line carries a prefix no
# quoted line carries, so a reader can tell them apart by the first token alone:
# `env:`, `tools:`, `gate-set:`, `policy` (`policy:` and `policy-override:`),
# `suppression` (the census line and each finding), `counts:`, `preflight-only:`
# and the closing `halt:`.
#
# THE ONE FIELD APPENDED TO A QUOTED LINE, AND WHY THAT IS NOT A BREAK OF THE
# RULE ABOVE. `exits: ...` now ends with `productevals=...`, appended *after*
# the six fields the record quotes rather than inserted among them, so those six
# keep their names, their values and their positions, and a reading taken before
# the seventh gate still lines up with one taken after it field for field. The
# rule above protects what a quoted field *means*, and no field's meaning
# changed. The alternative was a summary line that did not mention the seventh
# gate's exit at all — the one line in a whole run that could leave a red gate
# unmentioned in prose. `gate-set:` above it prints the true execution order, in
# which `productevals` runs fourth, and this line is keyed by name rather than
# ordered, so the two do not disagree.
#
# WHAT `passed=` IS, BECAUSE IT WAS PUBLISHED WRONGLY TWICE. `passed=` is the RAW
# SUM over every `test result:` line, and it is NOT the number of tests. On
# `gates-p15t035-accept-test.txt` it is 2754 against a parent figure of 2744, and
# on `gates-p15-batch-accept-test.txt` it is 2772 against 2762; the difference is
# ten children that re-run a parent's test in a second process, each printing its
# own `test result:` line and no `test <name> ... ok` line. Both figures were
# published as a test count in this record before the difference was understood.
# So `passed=` keeps its arithmetic exactly and the runner CALLS
# `scripts/measure-tests.mjs` -- the tracked instrument `P15-T021` put in the tree
# for this -- and prints `counts: parent=N` beside it, refusing by name when that
# instrument refuses. Neither figure is reimplemented here: a second
# implementation of a number that decides an acceptance is two instruments
# disagreeing later, not redundancy now.
#
# TWO INSTRUMENT DEFECTS FROM `P14-T004` ARE STILL FIXED HERE, and neither
# changed an exit code, which is why they survived:
#
#   1. `Select-String` is CASE-INSENSITIVE by default, so a pattern meant to
#      count test-binary headers also matched prose. `-CaseSensitive` is
#      required; without it the header count read 140 against a true 65.
#   2. The ignored total must be SUMMED over the `^test result:` lines. Reading
#      it from a `measure-run.mjs` "ignored max" line gives the last suite's own
#      count -- 8 against a true 12.
#
# A THIRD, FOUND LATER IN THE FILE'S OWN SUMMARY: libtest writes `running 13
# tests` in lowercase and PowerShell's `-match` is case-insensitive, so a
# case-blind pattern reads libtest's count line as if it were a header. Every
# pattern that reads a line libtest wrote is `-cmatch`/`-cnotmatch` here, and the
# difference is not theoretical -- measured on three logs in `target/tmp/`, a
# `^\s*(Running|Doc-tests)` count reads:
#     gates-p15t035-accept-test.txt   case-sensitive  76   case-blind 162
#     gates-p15-batch-accept-test.txt case-sensitive  78   case-blind 166
#     gates-p15t037-clone-test.txt    case-sensitive  71   case-blind 152
# The excess is one `running N tests` line per target binary, which a case-blind
# pattern cannot tell from `Running tests\x.rs`. The count printed below is the
# case-sensitive one, so the figures in the record are unaffected; what a
# case-blind pattern changes is the TARGET NAME attached to a failure, because
# the last line it matches before `---- <test> stdout ----` is the count.
#
# WHAT IT REFUSES, AND WHY REFUSING IS THE ANSWER RATHER THAN RUNNING AND
# REPORTING A RED THAT IS NOT THE DIFF. Six workspace test targets start Windows
# PowerShell with a `.ps1`, and Windows PowerShell's own default when no scope
# sets anything is `Restricted` -- measured 2026-09-21, `docs/development/
# WINDOWS.md` `## Which shell the tests are run from`. A session that starts
# `cargo test` without a process-scoped policy that permits scripts gets 29
# `UnauthorizedAccess` failures across those six targets, and every one of them
# is the shell refusing to load a file, not the tree. On this machine the tool
# harness that launches `pwsh` injects `-ExecutionPolicy Bypass`, which is a
# `Process`-scoped value the grandchildren inherit; an ordinary PowerShell window
# is NOT reproducing these conditions, and the operator has to be told that
# rather than handed a red they cannot read.
#
# So this runner MEASURES whether the shell a test would start can load a script
# (one child process, one probe file in the temp directory, removed afterwards),
# writes what it measured into the log, and refuses to run the gates when the
# answer is no. It sets no execution policy anywhere, at any scope, and writes
# no registry key: clause 10 of `CLAUDE.md`'s discipline and `P15-T036`'s
# measurement both say a machine- or user-scoped change is not the fix. What it
# does instead is name the requirement and the one command that satisfies it for
# a single session. `-AllowRefusedChildPolicy` takes the reading anyway, and the
# log then says out loud that the test gate is expected red for a reason that is
# not the tree.
#
# WHAT IT DOES NOT DO. It does not weaken anything to make a reading come out
# green: no gate command carries an extra flag, no failing target is named in a
# list of known failures, nothing is skipped, and the suppression census below
# reads THIS FILE as well as every file it calls. `-ErrorAction <the-silent-one>`
# appears nowhere in it, and the census is what would say so.

param(
    [Parameter(Mandatory = $true)][string]$Label,
    [switch]$PreflightOnly,
    [switch]$AllowRefusedChildPolicy
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Continue'
# Set here so that reading it before the first native command is a number rather
# than an error: StrictMode makes an unset automatic variable a terminating one.
$LASTEXITCODE = 0

# ---------------------------------------------------------------------------
# The seven commands, written once and executed from the same table.
#
# `Command` is the line as a reader types it and as `git grep` finds it; `Argv`
# is what is run, as an argument array and never as a shell string. The two are
# checked against each other in the preflight, before any gate runs, so a change
# to one that does not reach the other refuses by name instead of moving on to
# produce a reading under a command line that is not the one written down.
# ---------------------------------------------------------------------------
$GateSet = @(
    [pscustomobject]@{
        Name    = 'fmt'
        Command = 'cargo fmt --all -- --check'
        Program = 'cargo'
        Argv    = @('fmt', '--all', '--', '--check')
    }
    [pscustomobject]@{
        Name    = 'clippy'
        Command = 'cargo clippy --workspace --all-targets --all-features -- -D warnings'
        Program = 'cargo'
        Argv    = @('clippy', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings')
    }
    [pscustomobject]@{
        Name    = 'test'
        Command = 'cargo test --workspace --all-features --no-fail-fast'
        Program = 'cargo'
        Argv    = @('test', '--workspace', '--all-features', '--no-fail-fast')
    }
    # The seventh gate, and the only one whose place in this table is not free:
    # its input is `target/tmp/release-gate.json`, the artifact the `test` row
    # above writes, so a run that moved it anywhere before `test` would read a
    # gate from an earlier run or find none at all. The three node gates below
    # carry no such constraint. It starts no process, installs nothing and reads
    # three files, which is why it sits here rather than beside them.
    [pscustomobject]@{
        Name    = 'productevals'
        Command = 'node scripts/product-evals.mjs'
        Program = 'node'
        Argv    = @('scripts/product-evals.mjs')
    }
    [pscustomobject]@{
        Name    = 'bootstrap'
        Command = 'node scripts/validate-bootstrap.mjs'
        Program = 'node'
        Argv    = @('scripts/validate-bootstrap.mjs')
    }
    [pscustomobject]@{
        Name    = 'taskctl'
        Command = 'node scripts/taskctl.mjs validate'
        Program = 'node'
        Argv    = @('scripts/taskctl.mjs', 'validate')
    }
    [pscustomobject]@{
        Name    = 'nonwindows'
        Command = 'node scripts/check-non-windows.mjs'
        Program = 'node'
        Argv    = @('scripts/check-non-windows.mjs')
    }
)

# The files this harness is made of: this one, the node commands above that live
# in the tree (`validate-bootstrap.mjs`, `taskctl.mjs`, `check-non-windows.mjs`
# and `product-evals.mjs`), and the instrument it calls for the parent figure
# (`measure-tests.mjs`). This comment names no total on purpose. It named one
# until `P16` and the total was wrong twice over: it said the census "reads all
# six" over an array of five, and called the three node commands among the
# commands above "four". The count is printed by the census line below, which
# reads `$CensusRelative.Count` rather than a numeral, so no reader has to
# re-count it the next time this array changes. This file is one of the entries
# -- a census that exempted its own subject would be describing the tree rather
# than testing it.
$CensusRelative = @(
    'scripts/gates.ps1'
    'scripts/validate-bootstrap.mjs'
    'scripts/taskctl.mjs'
    'scripts/check-non-windows.mjs'
    'scripts/measure-tests.mjs'
    'scripts/product-evals.mjs'
)

# The two tokens a reading can be suppressed with, spelled in two pieces here so
# that the census can read this file without matching its own pattern. A file
# that spells them whole cannot honestly count them in itself; that is the whole
# of the trick and it is why the comment above cannot spell them either.
$SuppressionTokens = @(('Silently' + 'Continue'), ('continue-on-' + 'error'))

# Exit codes. Named rather than numbered, so the log says which one it is.
$ExitRed = 1        # at least one gate exited non-zero
$ExitCannotRun = 2  # a precondition is missing; no gate was run
$ExitPolicy = 3     # the shell a test would start cannot load a script; no gate was run
$ExitCensus = 4     # a suppression token was found in the harness
$ExitCounts = 5     # the test log exists but no count can be attributed from it

# ---------------------------------------------------------------------------
# Where the repository is, and where the logs go.
# ---------------------------------------------------------------------------
$ScriptDir = $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($ScriptDir)) {
    Write-Output ('gates: refused [no-script-path]: $PSScriptRoot is empty, so this file cannot say where it is. ' +
        'Run it as a file -- pwsh -NoProfile -File scripts/gates.ps1 -Label <label> -- rather than by pasting its text.')
    exit $ExitCannotRun
}
$RepoRoot = Split-Path -Parent $ScriptDir
$TmpDir = Join-Path $RepoRoot 'target\tmp'

# A label reaches a filename, so it is spelled the way a filename is. This also
# keeps a reading inside `target\tmp`, which is where every path in the record
# says it is.
if ($Label -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') {
    Write-Output ("gates: refused [bad-label]: -Label '$Label' is not a reading name. Use letters, digits, dot, dash " +
        'and underscore, starting with a letter or a digit -- the label becomes part of gates-<label>.txt, and a ' +
        'label that reaches outside target\tmp is a reading written over something else.')
    exit $ExitCannotRun
}

if (-not (Test-Path -LiteralPath $TmpDir -PathType Container)) {
    New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null
}
$LogName = if ($PreflightOnly) { "gates-$Label-preflight.txt" } else { "gates-$Label.txt" }
$Log = Join-Path $TmpDir $LogName

function Add-GateLine {
    param([string]$Text)
    Add-Content -Path $script:Log -Value $Text
}

# The first line TRUNCATES the log. `Add-Content` alone appends, so a second
# reading under the same label would sit underneath the first one and
# `Get-Content` at the end would print both -- two readings in one file with
# nothing saying which is which.
function Set-GateLogFirstLine {
    param([string]$Text)
    Set-Content -Path $script:Log -Value $Text -Encoding utf8
}

function Resolve-Tool {
    param([string]$Name)
    try {
        $found = @(Get-Command $Name -CommandType Application -ErrorAction Stop)
        # The first application, not the whole array: `git` resolves to two paths
        # on a Git for Windows machine (`mingw64\bin\git.exe` and `cmd\git.exe`),
        # and a caller handed both passes an array to `&` and gets "not
        # recognized as the name of a cmdlet" from a tool that is right there.
        if ($found.Count -eq 0) { return $null }
        return $found[0].Source
    } catch {
        return $null
    }
}

# The refusal path: named subject, the detail a reader needs, the exit code
# written into the log, the whole log printed. Every refusal in this file goes
# through here so that no refusal can be quieter than any other.
function Stop-WithRefusal {
    param([string]$Code, [int]$ExitCode, [string]$Subject, [string[]]$Detail)
    Add-GateLine ("refused [{0}]: {1}" -f $Code, $Subject)
    foreach ($line in $Detail) { Add-GateLine ('  ' + $line) }
    Add-GateLine ("halt: exit {0} ({1}) -- no gate was run" -f $ExitCode, $Code)
    Get-Content -Path $script:Log
    exit $ExitCode
}

# ---------------------------------------------------------------------------
# The header the record reads first: which tree, which worktree, which store.
# ---------------------------------------------------------------------------
$Git = Resolve-Tool 'git'
# `-C <root>` rather than the working directory: this runner is meant to be
# usable from anywhere, and a reading whose `at <sha>` came from whatever
# repository happened to be under the caller's shell is worse than no reading.
$Head = if ($Git) {
    $rev = & $Git -C $RepoRoot rev-parse HEAD 2>&1
    if ($LASTEXITCODE -eq 0) { ($rev | Select-Object -First 1).ToString().Trim() } else { 'git rev-parse failed' }
} else {
    'git is not on PATH'
}
$WorktreeStart = if ($Git) {
    $status = & $Git -C $RepoRoot status --porcelain 2>&1
    if ($LASTEXITCODE -eq 0) { (@($status) | ForEach-Object { $_.ToString() }) -join '; ' } else { 'git status failed' }
} else {
    'git is not on PATH'
}

$StorePath = Join-Path $env:LOCALAPPDATA 'SURE\sure.db'
$StorePresent = Test-Path -LiteralPath $StorePath -PathType Leaf
$StoreBefore = if ($StorePresent) { (Get-FileHash -LiteralPath $StorePath).Hash } else { 'not present' }

Set-GateLogFirstLine ("gates [{0}] at {1}" -f $Label, $Head)
Add-GateLine ('worktree at start: [' + $WorktreeStart + ']')
Add-GateLine ('store before: ' + $StoreBefore)

# ---------------------------------------------------------------------------
# Preflight. Everything that would make a reading uninterpretable is decided
# here, before the first gate command, and each one refuses by name.
# ---------------------------------------------------------------------------
$PolicyLines = New-Object System.Collections.Generic.List[string]

# 1. The PowerShell edition. The record's readings were taken under PowerShell 7;
#    under Windows PowerShell 5.1 the same commands run, but `Set-Content
#    -Encoding utf8` writes a byte-order mark, `Get-ChildItem` reports different
#    properties, and nothing about that is worth discovering in a summary a
#    reader is about to quote. Named here rather than left to a `#Requires`
#    line, which fails at load time with PowerShell's own message and not this
#    file's.
$Edition = $PSVersionTable.PSVersion
Add-GateLine ("env: pwsh {0} on {1} [{2}]; `$ErrorActionPreference=Continue; Set-StrictMode -Version Latest" -f
    $Edition.ToString(), [System.Runtime.InteropServices.RuntimeInformation]::OSDescription.Trim(),
    [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)

$Scopes = (Get-ExecutionPolicy -List | ForEach-Object { '{0}={1}' -f $_.Scope, $_.ExecutionPolicy }) -join ' '
$Preference = if ([string]::IsNullOrEmpty($env:PSExecutionPolicyPreference)) {
    'not set'
} else {
    $env:PSExecutionPolicyPreference
}
Add-GateLine ("env: execution policy scopes: {0}; PSExecutionPolicyPreference={1}" -f $Scopes, $Preference)

if ($Edition.Major -lt 7) {
    Stop-WithRefusal -Code 'ps-edition' -ExitCode $ExitCannotRun -Subject (
        "this runner requires PowerShell 7 or later and is running under $Edition") -Detail @(
        'The gate commands themselves do not care, but every reading in the record was taken under PowerShell 7 and',
        'the log format is written for it. Run the file under pwsh:',
        '    pwsh -NoProfile -File scripts/gates.ps1 -Label <label>'
    )
}

# 2. The tree this file is part of. A runner that reached outside the repository
#    for one of its own commands would work in this checkout and nowhere else,
#    which is the defect this file exists to close.
$Missing = New-Object System.Collections.Generic.List[string]
foreach ($relative in @('Cargo.toml', 'Cargo.lock') + $CensusRelative) {
    $full = Join-Path $RepoRoot $relative
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { $Missing.Add($relative) }
}
if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot 'tasks\tasks.json') -PathType Leaf)) {
    $Missing.Add('tasks/tasks.json')
}
if ($Missing.Count -gt 0) {
    Stop-WithRefusal -Code 'not-a-checkout' -ExitCode $ExitCannotRun -Subject (
        "this file is not inside a SURE checkout: $($Missing.Count) path(s) it needs are not there") -Detail @(
        "the file is $ScriptDir, so the repository root was read as $RepoRoot",
        ('missing: ' + (($Missing | ForEach-Object { $_ }) -join ', ')),
        'A clone of the repository has all of them; run the copy that is in the tree you mean to measure.'
    )
}

# 3. The programs the seven commands are. `&` on a name that does not resolve
#    leaves `$LASTEXITCODE` at whatever it was, which is how a missing tool turns
#    into a gate that "passed".
$Programs = @{}
$Absent = New-Object System.Collections.Generic.List[string]
foreach ($name in @('cargo', 'node', 'git')) {
    $resolved = Resolve-Tool $name
    if ($null -eq $resolved) { $Absent.Add($name) } else { $Programs[$name] = $resolved }
}
if ($Absent.Count -gt 0) {
    Stop-WithRefusal -Code 'tools-missing' -ExitCode $ExitCannotRun -Subject (
        "the seven commands need cargo, node and git on PATH; not found: $($Absent -join ', ')") -Detail @(
        'cargo runs gates fmt, clippy and test; node runs gates bootstrap, taskctl, nonwindows and',
        'productevals, and the measure-tests instrument this runner calls for the parent figure; git is',
        'what names the tree the reading was taken from.',
        'docs/development/WINDOWS.md lists the versions the record was taken under.'
    )
}
Add-GateLine ("tools: cargo={0}; node={1}; git={2}" -f $Programs['cargo'], $Programs['node'], $Programs['git'])

# 4. The written command line and the argument array must be the same command.
foreach ($gate in $GateSet) {
    $asRun = ($gate.Program + ' ' + ($gate.Argv -join ' '))
    if ($asRun -cne $gate.Command) {
        Stop-WithRefusal -Code 'gate-table' -ExitCode $ExitCannotRun -Subject (
            "gate '$($gate.Name)': the command written down is not the command that would run") -Detail @(
            "written down: $($gate.Command)",
            "would run:    $asRun",
            'One of the two moved without the other. The line a reader greps for and the line that produces the',
            'reading have to be the same command, or the log documents a command nobody ran.'
        )
    }
}
Add-GateLine ("gate-set: " + (($GateSet | ForEach-Object { $_.Command }) -join ' | '))

# 5. Can the shell that the six `.ps1`-spawning test targets start actually load
#    a script? Measured, not inferred from a policy name: one child process, one
#    probe file, both in this process tree. Nothing here sets a policy at any
#    scope and nothing writes a registry key.
$WindowsPowerShell = Resolve-Tool 'powershell.exe'
$PolicyUsable = $null
if ($null -eq $WindowsPowerShell) {
    Add-GateLine ('policy: cannot confirm -- powershell.exe is not on PATH, so the probe was not made. The six ' +
        '.ps1-spawning targets are a Windows concern; on a platform without that host, no execution policy applies.')
} else {
    $ProbePath = Join-Path ([System.IO.Path]::GetTempPath()) ("sure-gates-policy-probe-$PID.ps1")
    try {
        Set-Content -LiteralPath $ProbePath -Encoding utf8 -Value @(
            '# Written by scripts/gates.ps1, run once, removed afterwards. If this file runs, a process-scoped'
            '# policy that permits scripts is in this process tree and every child of it inherits that.'
            'exit 0'
        )
        $probeOutput = & $WindowsPowerShell -NoProfile -NonInteractive -File $ProbePath 2>&1
        $probeCode = $LASTEXITCODE
        $PolicyUsable = ($probeCode -eq 0)
        if ($PolicyUsable) {
            Add-GateLine (("policy: measured -- the shell the tests start ({0}) loaded a local .ps1 (exit 0), so the " +
                    'six .ps1-spawning targets can load theirs') -f $WindowsPowerShell)
        } else {
            $childScopes = @(@(& $WindowsPowerShell -NoProfile -NonInteractive -Command 'Get-ExecutionPolicy -List' 2>&1) |
                ForEach-Object { $_.ToString().Trim() } | Where-Object { $_.Length -gt 0 })
            # The first two lines are the child's own column headings, which are
            # localised on a machine whose display language is not English. What
            # a reader needs is the five scope rows underneath them.
            if ($childScopes.Count -gt 2) { $childScopes = @($childScopes[2..($childScopes.Count - 1)]) }
            $refusalText = (@($probeOutput) | ForEach-Object { $_.ToString() } | Where-Object { $_.Trim().Length -gt 0 })
            $PolicyLines.Add(("policy: REFUSED -- {0} -NoProfile -File <probe> exited {1} instead of 0" -f
                    $WindowsPowerShell, $probeCode))
            foreach ($line in $refusalText) { $PolicyLines.Add('  ' + $line) }
            $PolicyLines.Add('  Get-ExecutionPolicy -List in that child, its two heading lines dropped: ' +
                ($childScopes -join ' / '))
            $PolicyLines.Add('  Six workspace test targets start that shell with a .ps1, and Windows PowerShell')
            $PolicyLines.Add('  answers Restricted when no scope sets one: 29 UnauthorizedAccess failures across')
            $PolicyLines.Add('  those six targets, every one of them the shell refusing to load a file.')
            $PolicyLines.Add('  What it requires: a PROCESS-scoped policy that permits scripts, carried by the')
            $PolicyLines.Add('  session that starts cargo test, inherited by everything under it:')
            $PolicyLines.Add('      pwsh -ExecutionPolicy Bypass -File scripts/gates.ps1 -Label <label>')
            $PolicyLines.Add('  or, in the session you already have:')
            $PolicyLines.Add('      Set-ExecutionPolicy -Scope Process Bypass')
            $PolicyLines.Add('  A machine- or user-scoped change is NOT the fix and must not be proposed as one: it')
            $PolicyLines.Add('  would hide this refusal on every machine it was applied to.')
            $PolicyLines.Add('  Nothing in this file sets an execution policy at any scope, or writes a registry key.')
        }
    } catch {
        Add-GateLine ("policy: cannot confirm -- the probe could not be made: $($_.Exception.Message)")
    } finally {
        # Removed rather than left behind: `target/tmp` and the temp directory are
        # both walked by tests, and a probe file a reader did not put there is
        # somebody else's red.
        try { Remove-Item -LiteralPath $ProbePath -Force -ErrorAction Stop }
        catch { Add-GateLine ("policy: the probe file could not be removed: $ProbePath -- $($_.Exception.Message)") }
    }
}

if ($PolicyUsable -eq $false) {
    foreach ($line in $PolicyLines) { Add-GateLine $line }
}
if ($PolicyUsable -eq $false -and -not $AllowRefusedChildPolicy) {
    Stop-WithRefusal -Code 'child-policy' -ExitCode $ExitPolicy -Subject (
        'the shell a test target starts cannot load a .ps1, so the test gate would be red for a reason that is not the tree') -Detail @(
        'Nothing was run. Take the reading from a session that carries a process-scoped policy, or pass',
        '-AllowRefusedChildPolicy to take it anyway -- the log then records that the test gate is expected red,',
        'and a reading taken that way is not comparable with one that is not.'
    )
}
if ($PolicyUsable -eq $false) {
    Add-GateLine ('policy-override: -AllowRefusedChildPolicy was passed, so the gates run. The test gate is ' +
        'expected red for the reason above, that red is not the diff, and this reading is not comparable with ' +
        'one taken from a session that permits scripts.')
}

# 6. Nothing that suppresses. The census reads this file and every file it calls
#    and counts the two tokens a failure can be hidden behind. The counts are
#    printed whether or not they are zero, and a non-zero count is a named line
#    rather than a quiet one.
$CensusCounts = @{}
$CensusFindings = New-Object System.Collections.Generic.List[string]
foreach ($token in $SuppressionTokens) {
    $total = 0
    foreach ($relative in $CensusRelative) {
        $path = Join-Path $RepoRoot $relative
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
        $found = @(Select-String -LiteralPath $path -Pattern $token -SimpleMatch -CaseSensitive)
        if ($found.Count -gt 0) {
            $total += $found.Count
            $CensusFindings.Add(("{0} x{1} in {2}" -f $token, $found.Count, $relative))
        }
    }
    $CensusCounts[$token] = $total
}
Add-GateLine ("suppression census over {0} files: {1}" -f $CensusRelative.Count, (($SuppressionTokens |
            ForEach-Object { '{0}={1}' -f $_, $CensusCounts[$_] }) -join ' '))
foreach ($finding in $CensusFindings) {
    Add-GateLine ('suppression: ' + $finding)
}

if ($PreflightOnly) {
    $preflightCode = if ($CensusFindings.Count -gt 0) { $ExitCensus }
    elseif ($PolicyUsable -eq $false) { $ExitPolicy }
    else { 0 }
    Add-GateLine ("preflight-only: the seven gates were NOT run, so nothing in this file is a reading. " +
        '(summary -> {0})' -f $LogName)
    Add-GateLine ("halt: exit {0} (preflight-only)" -f $preflightCode)
    Get-Content -Path $Log
    exit $preflightCode
}

# ---------------------------------------------------------------------------
# The seven gates.
# ---------------------------------------------------------------------------
$GateExits = @{}
foreach ($gate in $GateSet) {
    $file = Join-Path $TmpDir ("gates-" + $Label + "-" + $gate.Name + ".txt")
    $program = $Programs[$gate.Program]
    $argv = @($gate.Argv)
    Push-Location -LiteralPath $RepoRoot
    try {
        & $program @argv 1> $file 2>&1
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    $GateExits[$gate.Name] = $code
    $size = if (Test-Path -LiteralPath $file -PathType Leaf) { (Get-Item -LiteralPath $file).Length } else { 'no log written' }
    Add-GateLine ("{0,-12} exit {1}  -> {2} ({3} bytes)" -f $gate.Name, $code, (Split-Path $file -Leaf), $size)
}

Add-GateLine ("exits: fmt={0} clippy={1} test={2} bootstrap={3} taskctl={4} nonwindows={5} productevals={6}" -f
    $GateExits['fmt'], $GateExits['clippy'], $GateExits['test'], $GateExits['bootstrap'], $GateExits['taskctl'], $GateExits['nonwindows'], $GateExits['productevals'])

# ---------------------------------------------------------------------------
# The counts, both of them, each labelled with what it counts.
# ---------------------------------------------------------------------------
$TestFile = Join-Path $TmpDir ("gates-" + $Label + "-test.txt")
$CountsAttributable = $true
if (-not (Test-Path -LiteralPath $TestFile -PathType Leaf)) {
    Add-GateLine ("result-lines=no test log -- {0} was not written, so there is nothing to count" -f
        (Split-Path $TestFile -Leaf))
    $CountsAttributable = $false
} else {
    # -CaseSensitive, so that the selector obeys the same rule as every other
    # pattern in this file that reads a line libtest wrote. It changes no figure:
    # measured on four logs (`gates-p15t035-accept-test.txt`,
    # `gates-p15-batch-accept-test.txt` and two from the clone) the case-blind
    # and case-sensitive counts of `^test result:` are equal -- 86, 88, 88, 88
    # against 86, 88, 88, 88. What it removes is the ability of a line of test
    # OUTPUT that happens to begin `Test result:` to be added into `passed=`.
    $results = @(Select-String -LiteralPath $TestFile -Pattern '^test result:' -CaseSensitive)
    $passed = 0; $failed = 0; $ignored = 0; $bad = 0
    foreach ($r in $results) {
        $line = $r.Line
        # -cnotmatch, not -notmatch: see the header, third instrument defect.
        if ($line -cnotmatch '^test result: ok\.') { $bad++ }
        if ($line -cmatch '(\d+) passed') { $passed += [int]$Matches[1] }
        if ($line -cmatch '(\d+) failed') { $failed += [int]$Matches[1] }
        if ($line -cmatch '(\d+) ignored') { $ignored += [int]$Matches[1] }
    }
    Add-GateLine ("result-lines={0} passed={1} failed={2} ignored={3} not-ok={4}" -f $results.Count, $passed, $failed, $ignored, $bad)
    Add-GateLine ('counts: passed= above is the RAW SUM over every `test result:` line. It is NOT the number of tests.')

    # The parent figure, from the tracked instrument, called rather than
    # reimplemented. Its refusal is surfaced by name and is not a pass.
    $MeasureScript = Join-Path $RepoRoot 'scripts\measure-tests.mjs'
    $Node = $Programs['node']
    $measureOutput = & $Node $MeasureScript $TestFile 2>&1
    $measureCode = $LASTEXITCODE
    $parentLine = @($measureOutput | ForEach-Object { $_.ToString() } | Where-Object { $_ -cmatch '^\s+parent figure\s+(\d+)' })
    if ($measureCode -eq 0 -and $parentLine.Count -gt 0) {
        $parent = [regex]::Match($parentLine[0], 'parent figure\s+(\d+)').Groups[1].Value
        # Single-quoted on purpose: this line spells `test <name> ... ok` with
        # backticks, and a double-quoted string would read `t as a tab and print
        # "	est <name> ... ok". Measured on the first clone run, which is why
        # this comment is here.
        Add-GateLine ('counts: parent={0} -- `test <name> ... ok` lines, one per test a parent ran (scripts/measure-tests.mjs exit 0)' -f $parent)
        Add-GateLine ('counts: quote parent= as the number of tests; the difference is children that re-ran a parent''s test.')
    } else {
        $reason = @($measureOutput | ForEach-Object { $_.ToString() } | Where-Object { $_ -cmatch '^\s+reason: ' }) | Select-Object -First 1
        Add-GateLine ("counts: parent=REFUSED -- scripts/measure-tests.mjs exit {0}; no figure can be attributed from this log" -f $measureCode)
        if ($null -ne $reason) { Add-GateLine ('counts: ' + $reason.Trim()) }
        Add-GateLine ('counts: passed= is a raw sum over `test result:` lines; with the parent figure refused it is not a count of tests either.')
        $CountsAttributable = $false
    }
}

# ---------------------------------------------------------------------------
# Which target and which test failed, not only how many. -cmatch, not -match:
# libtest prints `running 13 tests` in lowercase and a case-blind pattern reads
# the count as the target's name. There is deliberately no list of known-failing
# names anywhere in this file -- a suppressed name is a failure nobody sees
# again, which is worse than a count. Whether a named failure is new is not a
# property of one run; the reader compares the name against the families in
# `P15-T034`'s hand-back and treats a name that is not there as a first
# occurrence.
# ---------------------------------------------------------------------------
function Get-TestFailures {
    param([string]$File)
    $target = 'unknown'
    $named = New-Object System.Collections.Generic.List[string]
    foreach ($line in (Get-Content -LiteralPath $File)) {
        if ($line -cmatch '^\s*Running\s+(.*)$') {
            $target = $Matches[1].Trim()
        } elseif ($line -cmatch '^\s*Doc-tests\s+(.*)$') {
            $target = ($Matches[1].Trim() + ' (doc-tests)')
        } elseif ($line -cmatch '^---- (.*) stdout ----$') {
            $named.Add(($target + ' :: ' + $Matches[1].Trim()))
        }
    }
    return , $named
}

if (Test-Path -LiteralPath $TestFile -PathType Leaf) {
    $red = Get-TestFailures $TestFile
    if ($red.Count -eq 0) {
        Add-GateLine 'red: none'
    } else {
        foreach ($one in ($red | Sort-Object -Unique)) {
            Add-GateLine ('red: ' + $one)
        }
    }
    Add-GateLine ('headers (case-sensitive): ' + (@(Select-String -LiteralPath $TestFile -Pattern '^\s*(Running|Doc-tests)' -CaseSensitive).Count))
} else {
    Add-GateLine 'red: unknown -- there is no test log to read'
}

foreach ($name in @('bootstrap', 'taskctl')) {
    $file = Join-Path $TmpDir ("gates-" + $Label + "-" + $name + ".txt")
    $last = if (Test-Path -LiteralPath $file -PathType Leaf) {
        (Get-Content -LiteralPath $file | Select-Object -Last 1)
    } else {
        'no log written'
    }
    Add-GateLine ('{0}:{1}{2}' -f $name, (' ' * [Math]::Max(1, 11 - $name.Length)), $last)
}

# ---------------------------------------------------------------------------
# The store, byte for byte, before and after. SURE's own evidence store is the
# one thing a reading must not disturb, and this is the check that says it was
# not disturbed -- not a claim about it.
# ---------------------------------------------------------------------------
if ($StorePresent) {
    $StoreAfter = (Get-FileHash -LiteralPath $StorePath).Hash
    $store = Get-Item -LiteralPath $StorePath
    Add-GateLine ('store after:  ' + $StoreAfter)
    Add-GateLine ('store size: {0} bytes, mtime {1}' -f $store.Length, $store.LastWriteTimeUtc.ToString('o'))
    Add-GateLine ('store identical: ' + ($StoreBefore -eq $StoreAfter))
} else {
    Add-GateLine ('store after:  not present at ' + $StorePath)
    Add-GateLine 'store identical: n/a -- this machine has no SURE store, so there was nothing to disturb'
}

$WorktreeEnd = if ($Git) {
    # `-C $RepoRoot`, like the two calls above: this runner is meant to be used
    # from any directory, and the first clone run printed `worktree: [git status
    # failed]` because this call inherited the caller's working directory
    # (`C:\Windows\Temp`) while the one at the top of the run did not.
    $status = & $Git -C $RepoRoot status --porcelain 2>&1
    if ($LASTEXITCODE -eq 0) { (@($status) | ForEach-Object { $_.ToString() }) -join '; ' } else { 'git status failed' }
} else {
    'git is not on PATH'
}
Add-GateLine ('worktree:   [' + $WorktreeEnd + ']')

# ---------------------------------------------------------------------------
# The halt line and the exit code. Every failure above is in the log by name;
# this is the one line a script reads.
# ---------------------------------------------------------------------------
$Red = @($GateExits.Values | Where-Object { $_ -ne 0 })
$Halt = if ($Red.Count -gt 0) {
    @{ Code = $ExitRed; Reason = "{0} of {1} gates exited non-zero" -f $Red.Count, $GateSet.Count }
} elseif ($CensusFindings.Count -gt 0) {
    @{ Code = $ExitCensus; Reason = 'a suppression token is in the harness' }
} elseif (-not $CountsAttributable) {
    @{ Code = $ExitCounts; Reason = 'the test log exists but no count can be attributed from it' }
} elseif ($PolicyUsable -eq $false) {
    @{ Code = $ExitPolicy; Reason = 'taken with -AllowRefusedChildPolicy; the test gate is expected red' }
} else {
    @{ Code = 0; Reason = 'clean' }
}
Add-GateLine ("halt: exit {0} ({1})" -f $Halt.Code, $Halt.Reason)

Get-Content -Path $Log
exit $Halt.Code

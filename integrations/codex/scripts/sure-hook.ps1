$ErrorActionPreference='SilentlyContinue'

# Redirected stdin uses the console code page unless this says so; inert under our own -File (P16-T011).
[Console]::InputEncoding=New-Object System.Text.UTF8Encoding($false)
# Read the JSON event Codex sends to this hook. It is forwarded unchanged: the
# payload is Codex's own shape and carries `hook_event_name` itself, so no event
# name is added here. There is no `param` block on purpose: with one, PowerShell
# 5.1 tries to bind every line of the event as a parameter and the event never
# arrives. `$input` is read first because that is where a redirected payload has
# been measured to arrive in both hosts, with the console read as the fallback.
$inputJson=($input -join "`n")
if([string]::IsNullOrWhiteSpace($inputJson)){$inputJson=[Console]::In.ReadToEnd()}

# Encoding of the pipe into sure.exe below: 5.1's default is ASCII, which turns
# non-ASCII text in the payload into `?`, so UTF-8 is used instead. 5.1 puts a
# byte-order mark in front of the text whatever this is set to (measured
# 2026-09-18: EF BB BF, both with stdin as a pipe and as a file, and with
# pwsh 7 sending none); SURE drops a leading mark, so either host delivers the
# same event. Without that, 5.1 events were refused as invalid JSON.
$OutputEncoding=New-Object System.Text.UTF8Encoding($false)

# Resolve the local SURE binary: explicit override, then PATH, then the
# per-user install location the installer uses.
$bin=$env:SURE_BIN
if(-not $bin){$cmd=Get-Command sure -ErrorAction SilentlyContinue;if($cmd){$bin=$cmd.Source}}
if(-not $bin){$candidate=Join-Path $env:LOCALAPPDATA 'SURE\bin\sure.exe';if(Test-Path $candidate){$bin=$candidate}}

# Fail safely: if SURE is missing, say so on stderr and let the session
# continue. There is no decision in this note because SURE never ran, and a
# decision SURE did not make is not one this script may invent.
if (-not $bin) {
    [Console]::Error.WriteLine((@{acknowledged=$false;reason='SURE binary not found';source='codex'} | ConvertTo-Json -Compress))
    exit 0
}

# Forward the event and return SURE's exit code; stdout and stderr pass through
# unchanged so Codex sees exactly what SURE produced.
$inputJson | & $bin --format json hook ingest --source codex
exit $LASTEXITCODE

$ErrorActionPreference='SilentlyContinue'

# A redirected stdin is otherwise decoded with the console's own code page (936
# on a Simplified-Chinese Windows), which turns a non-ASCII `project_root` into a
# path that is not there: the event records nothing and says nothing.
[Console]::InputEncoding=New-Object System.Text.UTF8Encoding($false)

# Read the JSON payload Copilot sends to this hook process. There is no `param`
# block and `$input` is read before the console, which is the shape
# `integrations/codex/scripts/sure-hook.ps1:5-12` records a measurement for:
# PowerShell 5.1 binds a `param` block from the redirected event and the event
# never arrives. The event name the manifest passes arrives in `$args` instead.
$inputJson=($input -join "`n")
if([string]::IsNullOrWhiteSpace($inputJson)){$inputJson=[Console]::In.ReadToEnd()}

# 5.1's default encoding for the pipe below is ASCII, which turns non-ASCII in
# the payload into `?`; that is the same file's line 20, measured there.
$OutputEncoding=New-Object System.Text.UTF8Encoding($false)

# Resolve the local SURE binary. Order:
# 1. Explicit override (useful for development and custom installs).
# 2. PATH lookup (works once SURE is on the user's PATH).
# 3. Per-user install location under %LOCALAPPDATA%.
$bin=$env:SURE_BIN
if(-not $bin){$cmd=Get-Command sure -ErrorAction SilentlyContinue;if($cmd){$bin=$cmd.Source}}
if(-not $bin){$candidate=Join-Path $env:LOCALAPPDATA 'SURE\bin\sure.exe';if(Test-Path $candidate){$bin=$candidate}}

# Fail safely: do not block the agent or fabricate evidence if SURE is missing.
if (-not $bin) {
    [Console]::Error.WriteLine(
        (@{acknowledged=$false;reason='SURE binary not found';decision='allow';source='copilot'} | ConvertTo-Json -Compress)
    )
    exit 0
}

# Forward the event to SURE and return its exit code. stdout/stderr pass through
# unchanged so Copilot sees any decision/response SURE produces.
$inputJson | & $bin --format json hook ingest --source copilot @args
exit $LASTEXITCODE

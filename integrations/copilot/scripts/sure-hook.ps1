param([Parameter(ValueFromRemainingArguments=$true)][string[]]$HookArgs)
$ErrorActionPreference='SilentlyContinue'

# Read the JSON payload Copilot sends to this hook process.
$inputJson=[Console]::In.ReadToEnd()

# Resolve the local SURE binary. Order:
# 1. Explicit override (useful for development and custom installs).
# 2. PATH lookup (works once SURE is on the user's PATH).
# 3. Per-user install location under %LOCALAPPDATA% (works when Copilot is
#    launched from the Windows Start menu and the installer placed SURE there).
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

# Forward the event to SURE and return its exit code. stdout/stderr are passed
# through unchanged so Copilot sees any decision/response SURE produces.
$inputJson | & $bin --format json hook ingest --source copilot @HookArgs
exit $LASTEXITCODE

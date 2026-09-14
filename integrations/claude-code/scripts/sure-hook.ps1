param([Parameter(ValueFromRemainingArguments=$true)][string[]]$HookArgs)
$ErrorActionPreference='SilentlyContinue'
$inputJson=[Console]::In.ReadToEnd()
$bin=$env:SURE_BIN
if(-not $bin){$cmd=Get-Command sure -ErrorAction SilentlyContinue;if($cmd){$bin=$cmd.Source}}
if(-not $bin){$candidate=Join-Path $env:LOCALAPPDATA 'SURE\bin\sure.exe';if(Test-Path $candidate){$bin=$candidate}}
if(-not $bin){exit 0}
$inputJson | & $bin hook ingest --source claude-code @HookArgs
exit $LASTEXITCODE

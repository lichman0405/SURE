# SURE portable Agent Plugin per-user uninstall. Complements install.ps1 which resolves sure.exe.
$ErrorActionPreference='Stop'
$dst=$env:AGENT_PLUGIN_DIR
if(-not $dst){$dst=Join-Path $env:LOCALAPPDATA 'agent-plugins'}
$install=Join-Path $dst 'sure'
if(Test-Path $install){
    Remove-Item -Recurse -Force $install
    Write-Host "Uninstalled SURE Agent Plugin from $install"
}else{
    Write-Host "SURE Agent Plugin was not installed at $install"
}

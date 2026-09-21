# SURE Cursor plugin per-user uninstall. Complements install.ps1 which resolves sure.exe.
$ErrorActionPreference='Stop'
$dst=$env:CURSOR_PLUGIN_DIR
if(-not $dst){$dst=Join-Path $env:APPDATA 'Cursor\plugins'}
$install=Join-Path $dst 'sure'
if(Test-Path $install){
    Remove-Item -Recurse -Force $install
    Write-Host "Uninstalled SURE Cursor plugin from $install"
}else{
    Write-Host "SURE Cursor plugin was not installed at $install"
}

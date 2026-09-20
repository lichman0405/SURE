# SURE Claude Code plugin per-user uninstall. Complements install.ps1, which
# resolves sure.exe and places the package under CLAUDE_PLUGIN_DIR.
$ErrorActionPreference='Stop'
$dst=$env:CLAUDE_PLUGIN_DIR
if(-not $dst){$dst=Join-Path $env:LOCALAPPDATA 'claude-plugins'}
$install=Join-Path $dst 'sure'
if(Test-Path $install){
    # The removal is recursive, and an install may have made $install a symlink
    # (`-ForceCopy` is what rules that out). Measured on 2026-09-21 on this
    # machine, where the symlink privilege itself is refused: a reparse point
    # under this same path is removed as a link and its target is left whole —
    # `Remove-Item -Recurse -Force` on a junction reported `link exists=False ;
    # keep.txt exists=True` under PowerShell 7.6.6 and Windows PowerShell
    # 5.1.26100.9444 alike. What is not measured is a real symlink, which no
    # host here will create.
    Remove-Item -Recurse -Force $install
    Write-Host "Uninstalled SURE Claude Code plugin from $install"
}else{
    Write-Host "SURE Claude Code plugin was not installed at $install"
}

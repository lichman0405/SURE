# SURE Claude Code plugin per-user install. No admin required.
param([switch]$ForceCopy)
$ErrorActionPreference='Stop'
# Resolve SURE binary (same order as the hook launchers): SURE_BIN -> PATH -> LOCALAPPDATA
$bin=$env:SURE_BIN
if(-not $bin){$c=Get-Command sure -EA SilentlyContinue;if($c){$bin=$c.Source}}
if(-not $bin){$c=Join-Path $env:LOCALAPPDATA 'SURE\bin\sure.exe';if(Test-Path $c){$bin=$c}}
if(-not $bin){Write-Error 'SURE not found. Install SURE or set SURE_BIN.';exit 1}
# Plugin source is the parent directory of this script
$src=Resolve-Path (Join-Path (Split-Path $MyInvocation.MyCommand.Path) '..')|Select-Object -ExpandProperty Path
# Claude Code plugin dir: CLAUDE_PLUGIN_DIR -> %LOCALAPPDATA%\claude-plugins
$dst=$env:CLAUDE_PLUGIN_DIR
if(-not $dst){$dst=Join-Path $env:LOCALAPPDATA 'claude-plugins'}
if(-not (Test-Path $dst)){New-Item -ItemType Directory -Path $dst -Force|Out-Null}
$install=Join-Path $dst 'sure'
# Detect symlink capability unless ForceCopy. Windows without Developer Mode
# refuses it, so the privilege is a convenience and never a requirement: the
# copy below is the path that always works, and it is what runs here.
$link=$false
if(-not $ForceCopy){
    $t=Join-Path $env:TEMP "sure-claude-symlink-test-$([Guid]::NewGuid())"
    try{New-Item -ItemType SymbolicLink -Path $t -Target $src -EA Stop|Out-Null;$link=$true;Remove-Item $t}catch{}
}
if(Test-Path $install){Remove-Item -Recurse -Force $install}
if($link){New-Item -ItemType SymbolicLink -Path $install -Target $src|Out-Null;Write-Host "Installed (symlink): $install"}
else{
    Copy-Item -Recurse -Path $src -Destination $install
    $map=@{'{{SURE_BIN}}'=$bin;'{{PLUGIN_ROOT}}'=$install}
    Get-ChildItem -Recurse -File $install|Where-Object{$_.Extension -in '.json','.md','.ps1','.sh','.txt','.yaml','.yml'}|ForEach-Object{
        $txt=Get-Content -Raw $_.FullName;$orig=$txt
        foreach($k in $map.Keys){$txt=$txt -replace [regex]::Escape($k),$map[$k]}
        if($txt -ne $orig){Set-Content -Path $_.FullName -Value $txt -NoNewline}
    }
    Write-Host "Installed (copy): $install"
}
Write-Host "Package installed at $install. Loading a plugin is Claude Code's own workflow, not this script's; the commands and the limit are in README.md."

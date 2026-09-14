# SURE Windows environment checker v0.3.1
# Fixes v0.3 native-command argument splatting and the reserved $Host variable collision.
[CmdletBinding()]
param(
  [switch]$OnlineClaudeProbe,
  [switch]$SkipRustBuildSmoke,
  [switch]$SkipGitSmoke
)

$ErrorActionPreference='Continue'
$ProgressPreference='SilentlyContinue'
$Root=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Diag=Join-Path $Root 'diagnostics'
New-Item -ItemType Directory -Force -Path $Diag | Out-Null
$checks=[System.Collections.Generic.List[object]]::new()

function Add-Check($Name,$Status,$RequiredFor,$Detail,$Fix=''){
  $checks.Add([pscustomobject]@{name=$Name;status=$Status;required_for=$RequiredFor;detail=$Detail;fix=$Fix})
  Write-Host "[$Status] $Name - $Detail"
}
function Invoke-Native([string]$FilePath,[string[]]$Arguments=@()){
  try{
    $raw = & $FilePath @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($raw | Out-String).Trim()
    return [pscustomobject]@{ok=($exitCode -eq 0);text=$text;code=$exitCode}
  }
  catch{
    return [pscustomobject]@{ok=$false;text=$_.Exception.Message;code=-1}
  }
}
function MajorVersion($Text){ if($Text -match 'v?(\d+)\.'){return [int]$Matches[1]}; return 0 }

Write-Host '=== SURE Windows Environment Check ==='
$isWin=($IsWindows -or $env:OS -eq 'Windows_NT')
if($isWin){Add-Check 'Windows' 'PASS' 'core' "Native Windows $([Environment]::OSVersion.Version)"}
else{Add-Check 'Windows' 'FAIL' 'core' 'Run this checker on native Windows' 'Use Windows 11 x64 for primary development'}

if($PSVersionTable.PSVersion.Major -ge 7){Add-Check 'PowerShell' 'PASS' 'core' "$($PSVersionTable.PSVersion)"}
else{Add-Check 'PowerShell' 'FAIL' 'core' "$($PSVersionTable.PSVersion)" 'Install PowerShell 7+'}

$git=Invoke-Native 'git' @('--version')
if($git.ok){Add-Check 'Git' 'PASS' 'core' $git.text}else{Add-Check 'Git' 'FAIL' 'core' $git.text 'Install Git for Windows'}

if($git.ok){
  $long=Invoke-Native 'git' @('config','--global','--get','core.longpaths')
  if($long.ok -and $long.text -eq 'true'){Add-Check 'Git long paths' 'PASS' 'recommended' 'core.longpaths=true'}
  else{Add-Check 'Git long paths' 'WARN' 'recommended' 'Not enabled globally' 'Consider: git config --global core.longpaths true'}
}

$winget=Get-Command winget -ErrorAction SilentlyContinue
if($winget){Add-Check 'winget' 'PASS' 'recommended' $winget.Source}else{Add-Check 'winget' 'WARN' 'recommended' 'Not found' 'Optional: install/update App Installer'}

$rustup=Invoke-Native 'rustup' @('--version'); if($rustup.ok){Add-Check 'rustup' 'PASS' 'core' $rustup.text}else{Add-Check 'rustup' 'FAIL' 'core' $rustup.text 'Install rustup'}
$rustc=Invoke-Native 'rustc' @('--version'); if($rustc.ok){Add-Check 'rustc' 'PASS' 'core' $rustc.text}else{Add-Check 'rustc' 'FAIL' 'core' $rustc.text 'Install Rust 1.98.1 MSVC'}
$cargo=Invoke-Native 'cargo' @('--version'); if($cargo.ok){Add-Check 'cargo' 'PASS' 'core' $cargo.text}else{Add-Check 'cargo' 'FAIL' 'core' $cargo.text}

$rustHostInfo=Invoke-Native 'rustc' @('-vV')
if($rustHostInfo.ok -and $rustHostInfo.text -match 'host: x86_64-pc-windows-msvc'){Add-Check 'Rust MSVC host' 'PASS' 'core' 'x86_64-pc-windows-msvc'}
else{Add-Check 'Rust MSVC host' 'FAIL' 'core' $rustHostInfo.text 'Use rustup toolchain 1.98.1-x86_64-pc-windows-msvc'}

if($rustc.ok -and $rustc.text -match 'rustc 1\.98\.1'){Add-Check 'Pinned Rust' 'PASS' 'core' 'Rust 1.98.1'}
elseif($rustc.ok){Add-Check 'Pinned Rust' 'WARN' 'core' $rustc.text 'Run rustup toolchain install 1.98.1 and use repository rust-toolchain.toml'}

$comps=Invoke-Native 'rustup' @('component','list','--installed')
foreach($c in @('rustfmt','clippy')){
  if($comps.ok -and $comps.text -match "(?m)^$c"){Add-Check $c 'PASS' 'core' 'installed'}else{Add-Check $c 'FAIL' 'core' 'not installed' "rustup component add $c"}
}

if($SkipRustBuildSmoke){Add-Check 'Rust compile/link smoke' 'SKIP' 'core' 'Skipped'}
elseif($cargo.ok){
  $tmp=Join-Path ([IO.Path]::GetTempPath()) ('sure-rust-'+[guid]::NewGuid().ToString('N'))
  $pushed=0
  try{
    New-Item -ItemType Directory -Force $tmp | Out-Null; Push-Location $tmp; $pushed++
    & cargo new --quiet --bin smoke; if($LASTEXITCODE -ne 0){throw 'cargo new failed'}
    Push-Location (Join-Path $tmp 'smoke'); $pushed++
    $out=& cargo build --quiet 2>&1 | Out-String; $code=$LASTEXITCODE
    if($code -eq 0){Add-Check 'Rust compile/link smoke' 'PASS' 'core' 'Native MSVC executable built'}
    else{Add-Check 'Rust compile/link smoke' 'FAIL' 'core' $out.Trim() 'Install/repair Visual Studio Build Tools Desktop development with C++ + Windows SDK'}
  } catch {Add-Check 'Rust compile/link smoke' 'FAIL' 'core' $_.Exception.Message 'Repair MSVC Build Tools + Windows SDK'}
  finally{while($pushed -gt 0){Pop-Location;$pushed--}; Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue}
}

if($SkipGitSmoke){Add-Check 'Git path/worktree smoke' 'SKIP' 'core' 'Skipped'}
elseif($git.ok){
  $tmp=Join-Path ([IO.Path]::GetTempPath()) ('sure git '+[guid]::NewGuid().ToString('N'))
  $pushed=0
  try{
    $repo=Join-Path $tmp 'repo with spaces Ω'; $wt=Join-Path $tmp 'worktree with spaces Ω'
    New-Item -ItemType Directory -Force $repo | Out-Null; Push-Location $repo; $pushed++
    git init -q; git config user.email 'sure-smoke@example.invalid'; git config user.name 'sure-smoke'
    'x' | Set-Content -Encoding UTF8 README.md; git add README.md; git commit -q -m init
    git worktree add -q -b sure-smoke $wt; $ok=($LASTEXITCODE -eq 0)
    if($ok){git worktree remove -f $wt 2>&1 | Out-Null; Add-Check 'Git path/worktree smoke' 'PASS' 'core' 'Passed with spaces + Unicode'}
    else{Add-Check 'Git path/worktree smoke' 'FAIL' 'core' 'worktree failed' 'Update/repair Git for Windows'}
  } catch {Add-Check 'Git path/worktree smoke' 'FAIL' 'core' $_.Exception.Message}
  finally{while($pushed -gt 0){Pop-Location;$pushed--}; Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue}
}

$node=Invoke-Native 'node' @('--version')
if($node.ok -and (MajorVersion $node.text) -ge 24){Add-Check 'Node.js' 'PASS' 'dev' $node.text}
elseif($node.ok){Add-Check 'Node.js' 'FAIL' 'dev' $node.text 'Install Node.js 24 LTS'}
else{Add-Check 'Node.js' 'FAIL' 'dev' $node.text 'Install Node.js 24 LTS'}
$npm=Invoke-Native 'npm' @('--version'); if($npm.ok){Add-Check 'npm' 'PASS' 'dev' $npm.text}else{Add-Check 'npm' 'FAIL' 'dev' $npm.text 'Install npm with Node.js'}

$claude=Invoke-Native 'claude' @('--version')
if($claude.ok){
  Add-Check 'Claude Code' 'PASS' 'core' $claude.text
  $doctor=Invoke-Native 'claude' @('doctor')
  if($doctor.ok){Add-Check 'Claude doctor' 'PASS' 'core' (($doctor.text -replace "`r?`n",' | ').Substring(0,[Math]::Min(500,($doctor.text -replace "`r?`n",' | ').Length)))}
  else{Add-Check 'Claude doctor' 'WARN' 'core' $doctor.text 'Run claude doctor manually'}
}else{Add-Check 'Claude Code' 'FAIL' 'core' $claude.text 'Install/authenticate Claude Code'}

if($OnlineClaudeProbe -and $claude.ok){
  $probe=Invoke-Native 'claude' @('-p','Reply exactly SURE_OK and nothing else.','--output-format','text','--max-turns','1')
  if($probe.ok -and $probe.text -match 'SURE_OK'){Add-Check 'Claude online probe' 'PASS' 'core' 'Authenticated call succeeded'}
  else{Add-Check 'Claude online probe' 'FAIL' 'core' $probe.text 'Check Claude authentication/network'}
}else{Add-Check 'Claude online probe' 'SKIP' 'core' 'Use -OnlineClaudeProbe for a real authenticated call'}

$gh=Invoke-Native 'gh' @('--version'); if($gh.ok){Add-Check 'GitHub CLI' 'PASS' 'recommended' ($gh.text -split "`n")[0]}else{Add-Check 'GitHub CLI' 'WARN' 'recommended' 'not found' 'Optional: winget install GitHub.cli'}
$rg=Invoke-Native 'rg' @('--version'); if($rg.ok){Add-Check 'ripgrep' 'PASS' 'recommended' ($rg.text -split "`n")[0]}else{Add-Check 'ripgrep' 'WARN' 'recommended' 'not found' 'Optional but useful'}

$cursor=Get-Command cursor -ErrorAction SilentlyContinue
if($cursor){Add-Check 'Cursor CLI' 'PASS' 'integration' $cursor.Source}else{
  $cursorExe=Join-Path $env:LOCALAPPDATA 'Programs\Cursor\Cursor.exe'
  if(Test-Path $cursorExe){Add-Check 'Cursor' 'PASS' 'integration' $cursorExe}else{Add-Check 'Cursor' 'WARN' 'integration' 'not detected' 'Install Cursor for integration smoke tests'}
}
$codex=Invoke-Native 'codex' @('--version'); if($codex.ok){Add-Check 'Codex CLI' 'PASS' 'optional' $codex.text}else{Add-Check 'Codex CLI' 'WARN' 'optional' 'not found'}
$docker=Invoke-Native 'docker' @('--version'); if($docker.ok){Add-Check 'Docker' 'PASS' 'optional' $docker.text}else{Add-Check 'Docker' 'WARN' 'optional' 'not found; container execution tests will be limited'}

if($env:OneDrive -and $Root.StartsWith($env:OneDrive,[StringComparison]::OrdinalIgnoreCase)){Add-Check 'Repository location' 'WARN' 'recommended' 'Repository is under OneDrive' 'If file locks/watchers become flaky, move repo to a normal local development directory'}
else{Add-Check 'Repository location' 'PASS' 'recommended' $Root}

try{$drive=Get-PSDrive ([IO.Path]::GetPathRoot($Root).Substring(0,1));$gb=[math]::Round($drive.Free/1GB,1);if($gb -ge 20){Add-Check 'Free disk' 'PASS' 'core' "$gb GB free"}elseif($gb -ge 8){Add-Check 'Free disk' 'WARN' 'core' "$gb GB free" '20 GB+ recommended'}else{Add-Check 'Free disk' 'FAIL' 'core' "$gb GB free" 'Free disk space'}}catch{Add-Check 'Free disk' 'WARN' 'core' 'Could not determine'}

$coreFails=@($checks|Where-Object {$_.required_for -eq 'core' -and $_.status -eq 'FAIL'}).Count
$devFails=@($checks|Where-Object {$_.required_for -eq 'dev' -and $_.status -eq 'FAIL'}).Count
$coreReady=($coreFails -eq 0); $fullDevReady=($coreReady -and $devFails -eq 0)
$result=[pscustomobject]@{generated_at=(Get-Date).ToUniversalTime().ToString('o');core_ready=$coreReady;full_dev_ready=$fullDevReady;checks=$checks}
$json=Join-Path $Diag 'sure-env-report.json';$md=Join-Path $Diag 'sure-env-report.md';$result|ConvertTo-Json -Depth 8|Set-Content -Encoding UTF8 $json
$lines=[System.Collections.Generic.List[string]]::new();$lines.Add('# SURE Windows Environment Report');$lines.Add('');$lines.Add("- CORE_READY: **$coreReady**");$lines.Add("- FULL_DEV_READY: **$fullDevReady**");$lines.Add('');$lines.Add('| Status | Check | Required for | Detail | Fix |');$lines.Add('|---|---|---|---|---|')
foreach($c in $checks){$d=(("$($c.detail)" -replace '\|','\|') -replace "`r?`n",' ');$f=(("$($c.fix)" -replace '\|','\|') -replace "`r?`n",' ');$lines.Add("| $($c.status) | $($c.name) | $($c.required_for) | $d | $f |")}
$lines|Set-Content -Encoding UTF8 $md
Write-Host '';Write-Host "CORE_READY: $coreReady";Write-Host "FULL_DEV_READY: $fullDevReady";Write-Host "Report: $md"
if(-not $coreReady){exit 2}

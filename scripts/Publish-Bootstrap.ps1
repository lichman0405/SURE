[CmdletBinding()]
param(
  [string]$Remote='https://github.com/lichman0405/SURE.git',
  [string]$WorkBranch='claude/v0.1-autonomous'
)
$ErrorActionPreference='Stop'
$Root=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $Root
try {
  node scripts/validate-bootstrap.mjs
  if($LASTEXITCODE -ne 0){throw 'Bootstrap validation failed'}

  if(-not (Test-Path '.git')) { git init -b main }

  $origin=(git remote get-url origin 2>$null)
  if($LASTEXITCODE -ne 0 -or -not $origin){ git remote add origin $Remote; $origin=$Remote }
  if($origin -ne $Remote){ throw "origin is '$origin', expected '$Remote'. Refusing to rewrite remote." }

  git fetch origin --prune 2>$null | Out-Null
  $remoteMain=(git ls-remote --heads origin main)
  if($remoteMain){
    $localMain=(git rev-parse --verify main 2>$null)
    if(-not $localMain){ throw 'Remote main is non-empty but local main does not exist. Refusing to overwrite history.' }
    git merge-base --is-ancestor origin/main main 2>$null
    if($LASTEXITCODE -ne 0){ throw 'Remote main has incompatible history. Refusing to overwrite/force-push.' }
  }

  git add -A
  $dirty=git status --porcelain
  if($dirty){
    git commit -m 'chore: bootstrap SURE autonomous development v0.3'
    if($LASTEXITCODE -ne 0){ throw 'Could not create bootstrap commit. Configure git user.name/user.email.' }
  }

  git branch -M main
  git push -u origin main
  if($LASTEXITCODE -ne 0){ throw 'Failed to push main.' }

  $exists=(git branch --list $WorkBranch)
  if(-not $exists){ git switch -c $WorkBranch } else { git switch $WorkBranch }
  git push -u origin $WorkBranch
  if($LASTEXITCODE -ne 0){ throw 'Failed to push autonomous work branch.' }

  Write-Host "Published bootstrap. Current branch: $WorkBranch"
  Write-Host 'Next: claude  ->  /build-sure'
} finally { Pop-Location }

[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$Root=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $Root
try {
  node scripts/validate-bootstrap.mjs
  if($LASTEXITCODE -ne 0){throw 'bootstrap validation failed'}
  node scripts/taskctl.mjs validate
  if($LASTEXITCODE -ne 0){throw 'task state validation failed'}
  cargo fmt --all -- --check
  if($LASTEXITCODE -ne 0){throw 'cargo fmt failed'}
  cargo check --workspace --all-targets
  if($LASTEXITCODE -ne 0){throw 'cargo check failed'}
  cargo clippy --workspace --all-targets -- -D warnings
  if($LASTEXITCODE -ne 0){throw 'cargo clippy failed'}
  cargo test --workspace
  if($LASTEXITCODE -ne 0){throw 'cargo test failed'}
  Write-Host 'SURE Windows preflight passed.'
} finally { Pop-Location }

[CmdletBinding()]
param([switch]$Apply)

$ErrorActionPreference='Stop'

if (-not ($IsWindows -or $env:OS -eq 'Windows_NT')) { throw 'This helper is for Windows.' }

Write-Host 'SURE Windows development prerequisites:'
Write-Host '- PowerShell 7+'
Write-Host '- Git for Windows'
Write-Host '- Rustup / Rust 1.98.1 MSVC'
Write-Host '- Visual Studio 2022 Build Tools: Desktop development with C++ + Windows SDK'
Write-Host '- Node.js 24 LTS'
Write-Host '- Claude Code'
Write-Host '- Optional: gh, ripgrep, Cursor, Codex, Docker Desktop'
Write-Host ''

if (-not $Apply) {
  Write-Host 'This script intentionally does not install toolchains silently.'
  Write-Host 'Use -Apply only for safe package-manager items; Visual Studio workloads/rustup/Claude may still require their official installers.'
  exit 0
}

$winget=Get-Command winget -ErrorAction SilentlyContinue
if (-not $winget) { throw 'winget not found. Install App Installer or install dependencies manually.' }

$packages=@(
  'Git.Git',
  'OpenJS.NodeJS.LTS',
  'GitHub.cli',
  'BurntSushi.ripgrep.MSVC'
)
foreach($id in $packages){
  Write-Host "Installing/upgrading $id ..."
  winget install --id $id --exact --accept-package-agreements --accept-source-agreements --silent
}

Write-Host ''
Write-Host 'Now install/verify separately:'
Write-Host '1. Visual Studio 2022 Build Tools: Desktop development with C++ + Windows SDK'
Write-Host '2. rustup with x86_64-pc-windows-msvc and Rust 1.98.1'
Write-Host '3. Claude Code authentication'
Write-Host 'Then run .\scripts\Test-SureEnvironment.ps1'

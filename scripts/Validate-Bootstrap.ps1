param([string]$Root=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path)
$ErrorActionPreference='Stop'
$required=@(
 'MASTER_PROMPT.md','CLAUDE.md','ASSUMPTIONS.md','START_HERE.md','Cargo.toml','rust-toolchain.toml',
 'docs\product\PRODUCT_THESIS.md','docs\product\MVP_SPEC.md','docs\product\DEFINITION_OF_DONE.md',
 'docs\architecture\ARCHITECTURE.md','docs\architecture\PROJECT_INTENT.md','docs\architecture\EXECUTION_SAFETY.md',
 'docs\security\THREAT_MODEL.md','docs\testing\TEST_STRATEGY.md','docs\development\WINDOWS.md',
 'tasks\phases.json','tasks\tasks.json','progress\state.json','progress\HANDOFF.md'
)
foreach($rel in $required){if(-not(Test-Path(Join-Path $Root $rel))){throw "Missing $rel"}}
node (Join-Path $Root 'scripts\validate-bootstrap.mjs')
if($LASTEXITCODE -ne 0){throw 'Node bootstrap validator failed'}
Write-Host 'PowerShell bootstrap validation OK.'

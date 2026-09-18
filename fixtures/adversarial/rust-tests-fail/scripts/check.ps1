# This project's own check, run on a copy of the project.
#
# Any Rust project's own check is `cargo test`, and that is what runs below. The
# copy is the one thing this script adds, and it is there for a reason worth
# stating rather than leaving in the code: `cargo` writes `Cargo.lock` and build
# output next to the manifest it reads, and this project ships inside SURE's own
# checkout, where either would show up in `git status` and could be committed by
# mistake. A scratch directory outside the checkout leaves the repository alone.
#
# The copy is the same project, and that is asserted rather than promised in
# prose: crates/sure-core/tests/rust_fixture_apps.rs copies this directory the
# same way and compares the content fingerprints of the original and the copy.
#
# Run it with:
#
#     powershell -File scripts/check.ps1
#
# It exits with the code `cargo test` gave it. This half of the fixture pair
# fails, so that code is non-zero.

$source = Split-Path -Parent $PSScriptRoot
$scratch = Join-Path ([System.IO.Path]::GetTempPath()) (
    'sure-rust-tests-fail-' + [guid]::NewGuid().ToString('N')
)

New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    Copy-Item -Path (Join-Path $source '*') -Destination $scratch -Recurse -Force

    Push-Location $scratch
    try {
        & cargo test
        $code = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }

    $verdict = if ($code -eq 0) { 'PASS' } else { 'FAIL' }

    Write-Output ''
    Write-Output 'fixture                            : rust-tests-fail'
    Write-Output "project this check ran on          : a copy of $source"
    Write-Output "copy made at                       : $scratch"
    Write-Output 'the project''s own check            : cargo test'
    Write-Output "what cargo test said               : $verdict (exit code $code)"
    Write-Output "verdict of this project's own check: $verdict (this is the point)"
}
finally {
    Remove-Item -Recurse -Force $scratch -ErrorAction SilentlyContinue
}

exit $code

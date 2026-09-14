#!/usr/bin/env bash
# Secondary macOS/Linux preflight. Windows canonical path is scripts/Preflight-Windows.ps1.
#
# The shebang is the first line and the comment follows it. `shellcheck` reports
# a comment above the shebang as SC1128, an error, and this script is one of the
# files its own last line checks — so the old order made this script fail its own
# gate. Nothing ran it: the Windows preflight is the one used here, and CI's
# shellcheck job was red from the bootstrap commit until this was fixed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
node scripts/validate-bootstrap.mjs
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
if command -v shellcheck >/dev/null 2>&1; then shellcheck scripts/*.sh integrations/*/scripts/*.sh; fi
printf 'SURE preflight passed.\n'

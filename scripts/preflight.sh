# Secondary macOS/Linux preflight. Windows canonical path is scripts/Preflight-Windows.ps1.
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
node scripts/validate-bootstrap.mjs
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
if command -v shellcheck >/dev/null 2>&1; then shellcheck scripts/*.sh integrations/*/scripts/*.sh; fi
printf 'SURE preflight passed.\n'

#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
chmod +x scripts/*.sh scripts/*.mjs integrations/*/scripts/*.sh 2>/dev/null || true
node scripts/validate-bootstrap.mjs
node scripts/taskctl.mjs validate
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run validate
printf '\nSURE bootstrap validation passed.\nSecondary Unix bootstrap passed. On Windows use Bootstrap-Sure.ps1 / Publish-Bootstrap.ps1. Then run claude and /build-sure.\n'

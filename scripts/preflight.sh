#!/usr/bin/env bash
# Secondary macOS/Linux preflight. Windows canonical path is scripts/Preflight-Windows.ps1.
#
# The shebang is the first line and the comment follows it. The linter reports a
# comment above the shebang as SC1128, an error, and this script is one of the
# files its own last line checks — so the old order made this script fail its own
# gate. Nothing ran it: the Windows preflight is the one used here, and the
# linter job in CI was red from the bootstrap commit until this was fixed.
#
# No comment in this repository may begin with that linter's name: a comment
# whose first word is the tool is read as a directive, and the directive parser
# then rejects the prose that follows it. This paragraph is the note that made
# that mistake once.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
node scripts/validate-bootstrap.mjs
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
if command -v shellcheck >/dev/null 2>&1; then shellcheck scripts/*.sh integrations/*/scripts/*.sh; fi
printf 'SURE preflight passed.\n'

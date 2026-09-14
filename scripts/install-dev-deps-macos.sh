#!/usr/bin/env bash
set -euo pipefail
if [ "$(uname -s)" != Darwin ]; then echo "This helper is for macOS." >&2; exit 2; fi
if [ "${1:-}" != "--apply" ]; then
  cat <<'EOF'
This helper intentionally does not modify your Mac without an explicit flag.
It can run:
  xcode-select --install   (interactive, if needed)
  brew bundle
It does NOT install rustup automatically.

Review Brewfile, then run:
  ./scripts/install-dev-deps-macos.sh --apply
Install Rust separately from https://rustup.rs/ if rustup is missing.
EOF
  exit 0
fi
if ! xcode-select -p >/dev/null 2>&1; then xcode-select --install; echo "Complete the Xcode CLT installer, then re-run this script."; exit 2; fi
if ! command -v brew >/dev/null 2>&1; then echo "Homebrew is not installed. Install it from https://brew.sh/ or install dependencies manually." >&2; exit 2; fi
brew bundle
printf 'Homebrew dependencies installed. If rustup is missing, install it from https://rustup.rs/.\n'

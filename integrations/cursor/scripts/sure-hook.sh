#!/usr/bin/env bash
set -u
SOURCE="cursor"
find_sure() {
  if [ -n "${SURE_BIN:-}" ] && [ -x "$SURE_BIN" ]; then printf '%s\n' "$SURE_BIN"; return 0; fi
  if command -v sure >/dev/null 2>&1; then command -v sure; return 0; fi
  for p in "$HOME/.cargo/bin/sure" /opt/homebrew/bin/sure /usr/local/bin/sure; do [ -x "$p" ] && { printf '%s\n' "$p"; return 0; }; done
  return 1
}
BIN="$(find_sure || true)"
if [ -z "$BIN" ]; then
  # During installation/bootstrap, absence of SURE must not emit malformed hook JSON or block unrelated agent work.
  cat >/dev/null
  exit 0
fi
exec "$BIN" hook ingest --source "$SOURCE" "$@"

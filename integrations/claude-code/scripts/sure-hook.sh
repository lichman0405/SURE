#!/usr/bin/env bash
set -u
SOURCE="claude-code"

# Resolve the local SURE binary. Order:
# 1. Explicit override (useful for development and custom installs).
# 2. PATH lookup (works once SURE is on the user's PATH).
# 3. Common per-user install locations (Cargo, Homebrew, /usr/local).
find_sure() {
  if [ -n "${SURE_BIN:-}" ] && [ -x "$SURE_BIN" ]; then printf '%s\n' "$SURE_BIN"; return 0; fi
  if command -v sure >/dev/null 2>&1; then command -v sure; return 0; fi
  for p in "$HOME/.cargo/bin/sure" /opt/homebrew/bin/sure /usr/local/bin/sure; do [ -x "$p" ] && { printf '%s\n' "$p"; return 0; }; done
  return 1
}

BIN="$(find_sure || true)"
if [ -z "$BIN" ]; then
  # Drain the event before exiting. Exiting with it unread kills the harness's
  # write (SIGPIPE on Unix, a broken pipe on Windows) under an exit 0 the harness
  # reads as success. Same shape as the cursor and codex launchers; argued and
  # measured in docs/integrations/HOOK_FAILURE_SEMANTICS.md §2.2.
  cat >/dev/null
  # Fail safely: do not block the agent or fabricate evidence if SURE is missing.
  printf '%s\n' '{"acknowledged":false,"reason":"SURE binary not found","decision":"allow","source":"claude-code"}' >&2
  exit 0
fi
exec "$BIN" hook ingest --source "$SOURCE" "$@"

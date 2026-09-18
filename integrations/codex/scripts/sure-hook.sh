#!/usr/bin/env bash
# Codex sends the event as one JSON object on stdin; it is forwarded unchanged
# and SURE reads `hook_event_name` out of it, so no event name is passed here.
set -u
SOURCE="codex"
find_sure() {
  if [ -n "${SURE_BIN:-}" ] && [ -x "$SURE_BIN" ]; then printf '%s\n' "$SURE_BIN"; return 0; fi
  if command -v sure >/dev/null 2>&1; then command -v sure; return 0; fi
  for p in "$HOME/.cargo/bin/sure" /opt/homebrew/bin/sure /usr/local/bin/sure; do [ -x "$p" ] && { printf '%s\n' "$p"; return 0; }; done
  return 1
}
BIN="$(find_sure || true)"
if [ -z "$BIN" ]; then
  # Absence of SURE must not block unrelated agent work. The note goes to
  # stderr, which is where Codex reads a blocking reason from, and there is no
  # decision in it because SURE never ran.
  cat >/dev/null
  printf '%s\n' '{"acknowledged":false,"reason":"SURE binary not found","source":"codex"}' >&2
  exit 0
fi
exec "$BIN" --format json hook ingest --source "$SOURCE" "$@"

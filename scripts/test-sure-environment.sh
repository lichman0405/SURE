#!/usr/bin/env bash
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIAG="$ROOT/diagnostics"
mkdir -p "$DIAG"
REPORT_MD="$DIAG/sure-env-report.md"
REPORT_JSON="$DIAG/sure-env-report.json"
TMP_ROWS="$(mktemp)"
trap 'rm -f "$TMP_ROWS"' EXIT

core_fail=0
full_fail=0

sanitize_field() {
  printf '%s' "$1" | tr '\t\r\n' '   '
}

add_check() {
  name="$1"; status="$2"; required="$3"; detail="$4"; fix="${5:-}"
  if [ "$required" = "core" ] && [ "$status" = "FAIL" ]; then core_fail=$((core_fail+1)); fi
  if [ "$required" = "full_dev" ] && [ "$status" = "FAIL" ]; then full_fail=$((full_fail+1)); fi
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$(sanitize_field "$name")" "$(sanitize_field "$status")" "$(sanitize_field "$required")" \
    "$(sanitize_field "$detail")" "$(sanitize_field "$fix")" >> "$TMP_ROWS"
  printf '[%s] %s - %s\n' "$status" "$name" "$detail"
}

printf '=== SURE macOS Environment Check ===\n'

if [ "$(uname -s 2>/dev/null)" = "Darwin" ]; then
  add_check "macOS" "PASS" "core" "$(sw_vers -productVersion 2>/dev/null || echo unknown)"
else
  add_check "macOS" "FAIL" "core" "This checker is intended for native macOS" "Run on the Mac used for SURE development"
fi

ARCH="$(uname -m 2>/dev/null || echo unknown)"
case "$ARCH" in
  # PASS is about this machine as a *development host*, and it is deserved on
  # both architectures: `cargo build` here is a native build and the workspace
  # compiles. It is deliberately not a statement that a release artifact exists
  # for the machine.
  #
  # SURE produces a macOS archive for each of arm64 and x86_64, and
  # `.github/workflows/release-dry-run.yml` builds, checksums and runs each of
  # them -- the x86_64 one on a native Intel runner, run 35514769749 job
  # 106088732392. Neither is published: this repository has no GitHub Releases,
  # and both archives are workflow artifacts retained on the run page. So the
  # two architectures are in the same position, and neither is a reason to grade
  # a development host differently. What is true of both is the narrow thing:
  # nothing is published as a release for either. The boundary is written down
  # in docs/development/RELEASE_PROCESS.md, "What the macOS Intel archive is,
  # concretely".
  arm64) add_check "CPU architecture" "PASS" "core" "Apple Silicon (arm64) - builds SURE from source; the arm64 release archive is not published" ;;
  x86_64) add_check "CPU architecture" "PASS" "core" "Intel (x86_64) - builds SURE from source; the x86_64 release archive is not published" ;;
  *) add_check "CPU architecture" "WARN" "core" "$ARCH" "SURE builds from source on arm64 and x86_64 macOS; the release archive for each is not published" ;;
esac

if xcode-select -p >/dev/null 2>&1; then
  add_check "Xcode Command Line Tools" "PASS" "core" "$(xcode-select -p)"
else
  add_check "Xcode Command Line Tools" "FAIL" "core" "Not configured" "Run: xcode-select --install"
fi

if command -v clang >/dev/null 2>&1; then
  add_check "clang" "PASS" "core" "$(clang --version 2>/dev/null | head -1)"
else
  add_check "clang" "FAIL" "core" "clang not found" "Install Xcode Command Line Tools"
fi

if command -v git >/dev/null 2>&1; then
  add_check "Git" "PASS" "core" "$(git --version)"
else
  add_check "Git" "FAIL" "core" "git not found" "Install Git (Homebrew recommended)"
fi

if command -v brew >/dev/null 2>&1; then
  add_check "Homebrew" "PASS" "recommended" "$(brew --prefix)"
else
  add_check "Homebrew" "WARN" "recommended" "Not found" "Install Homebrew or manage dependencies manually"
fi

if command -v rustup >/dev/null 2>&1; then
  add_check "rustup" "PASS" "core" "$(rustup --version 2>/dev/null | head -1)"
else
  add_check "rustup" "FAIL" "core" "not found" "Install rustup from https://rustup.rs/"
fi

if command -v rustc >/dev/null 2>&1; then
  RUSTC="$(rustc --version 2>/dev/null)"
  add_check "rustc" "PASS" "core" "$RUSTC"
  HOST="$(rustc -vV 2>/dev/null | awk '/^host:/{print $2}')"
  case "$HOST" in
    aarch64-apple-darwin|x86_64-apple-darwin) add_check "Rust host target" "PASS" "core" "$HOST" ;;
    *) add_check "Rust host target" "FAIL" "core" "${HOST:-unknown}" "Use a native apple-darwin rustup toolchain" ;;
  esac
else
  add_check "rustc" "FAIL" "core" "not found" "Install Rust 1.98.1 via rustup"
fi

if command -v cargo >/dev/null 2>&1; then
  add_check "cargo" "PASS" "core" "$(cargo --version 2>/dev/null)"
else
  add_check "cargo" "FAIL" "core" "not found" "Install Rust via rustup"
fi

for comp in rustfmt clippy; do
  if command -v "cargo-${comp}" >/dev/null 2>&1; then
    add_check "$comp" "PASS" "core" "installed"
  elif command -v rustup >/dev/null 2>&1 && rustup component list --installed 2>/dev/null | grep -q "^${comp}"; then
    add_check "$comp" "PASS" "core" "installed"
  else
    add_check "$comp" "FAIL" "core" "not installed" "rustup component add $comp"
  fi
done

if command -v cargo >/dev/null 2>&1; then
  TMP_RUST="$(mktemp -d "${TMPDIR:-/tmp}/sure-rust.XXXXXX")"
  if (cd "$TMP_RUST" && cargo new --quiet --bin smoke && cd smoke && cargo build --quiet) >/tmp/sure-rust-smoke.log 2>&1; then
    add_check "Rust compile/link smoke" "PASS" "core" "Native Rust executable built successfully"
  else
    add_check "Rust compile/link smoke" "FAIL" "core" "$(tail -20 /tmp/sure-rust-smoke.log 2>/dev/null | tr '\n' ' ')" "Repair rustup/Xcode Command Line Tools"
  fi
  rm -rf "$TMP_RUST" /tmp/sure-rust-smoke.log
fi

if command -v git >/dev/null 2>&1; then
  TMP_GIT="$(mktemp -d "${TMPDIR:-/tmp}/sure-git.XXXXXX")"
  REPO="$TMP_GIT/repo with spaces"
  WT="$TMP_GIT/worktree with spaces"
  mkdir -p "$REPO"
  if (cd "$REPO" && git init -q && git config user.email sure-smoke@example.invalid && git config user.name sure-smoke && printf x > README.md && git add README.md && git commit -q -m init && git worktree add -q -b sure-smoke "$WT") >/tmp/sure-git-smoke.log 2>&1; then
    add_check "Git worktree/path smoke" "PASS" "core" "Passed with spaces in paths"
  else
    add_check "Git worktree/path smoke" "FAIL" "core" "$(tail -20 /tmp/sure-git-smoke.log 2>/dev/null | tr '\n' ' ')" "Repair/update Git"
  fi
  rm -rf "$TMP_GIT" /tmp/sure-git-smoke.log
fi

if command -v node >/dev/null 2>&1; then
  NODE_VER="$(node --version)"
  NODE_MAJOR="$(printf '%s' "$NODE_VER" | sed 's/^v//' | cut -d. -f1)"
  if [ "$NODE_MAJOR" -ge 24 ] 2>/dev/null; then
    add_check "Node.js" "PASS" "core" "$NODE_VER"
  else
    add_check "Node.js" "FAIL" "core" "$NODE_VER" "Install Node.js 24 LTS or newer supported version"
  fi
else
  add_check "Node.js" "FAIL" "core" "not found" "Install Node.js 24 LTS"
fi

if command -v npm >/dev/null 2>&1; then
  add_check "npm" "PASS" "core" "$(npm --version)"
else
  add_check "npm" "FAIL" "core" "not found" "Install npm with Node.js"
fi

if command -v claude >/dev/null 2>&1; then
  add_check "Claude Code" "PASS" "core" "$(claude --version 2>/dev/null | head -1)"
  DOCTOR="$(claude doctor 2>&1 || true)"
  if [ -n "$DOCTOR" ]; then
    add_check "Claude doctor" "PASS" "recommended" "$(printf '%s' "$DOCTOR" | tr '\n' ' ' | cut -c1-500)"
  fi
else
  add_check "Claude Code" "FAIL" "core" "not found" "Install/authenticate Claude Code"
fi

if [ "${1:-}" = "--online-claude" ] && command -v claude >/dev/null 2>&1; then
  PROBE="$(claude -p 'Reply exactly SURE_OK and nothing else.' --output-format text --max-turns 1 2>&1 || true)"
  if printf '%s' "$PROBE" | grep -q 'SURE_OK'; then
    add_check "Claude authenticated probe" "PASS" "core" "Live Claude Code call succeeded"
  else
    add_check "Claude authenticated probe" "FAIL" "core" "$(printf '%s' "$PROBE" | cut -c1-500)" "Check Claude authentication/network"
  fi
else
  add_check "Claude authenticated probe" "SKIP" "recommended" "Run with --online-claude to make a real call"
fi

if command -v gh >/dev/null 2>&1; then
  if gh auth status >/dev/null 2>&1; then
    add_check "GitHub CLI" "PASS" "recommended" "gh installed and authenticated"
  else
    add_check "GitHub CLI" "WARN" "recommended" "gh installed but not authenticated" "Run: gh auth login"
  fi
else
  add_check "GitHub CLI" "WARN" "recommended" "not found" "brew install gh"
fi

if command -v cursor >/dev/null 2>&1; then
  add_check "Cursor CLI" "PASS" "integration_smoke" "$(command -v cursor)"
elif [ -d "/Applications/Cursor.app" ]; then
  add_check "Cursor app" "PASS" "integration_smoke" "/Applications/Cursor.app (CLI not on PATH)"
else
  add_check "Cursor" "WARN" "integration_smoke" "not found" "Install Cursor for integration smoke testing"
fi

if command -v codex >/dev/null 2>&1; then
  add_check "Codex CLI" "PASS" "integration_smoke" "$(codex --version 2>/dev/null | head -1)"
else
  add_check "Codex CLI" "WARN" "integration_smoke" "not found" "Optional for Codex integration smoke tests"
fi

if command -v docker >/dev/null 2>&1; then
  add_check "Docker" "PASS" "optional" "$(docker --version 2>/dev/null)"
elif command -v podman >/dev/null 2>&1; then
  add_check "Podman" "PASS" "optional" "$(podman --version 2>/dev/null)"
else
  add_check "Container runtime" "WARN" "optional" "not found" "Optional; useful for container execution-mode development"
fi

if command -v shellcheck >/dev/null 2>&1; then add_check "shellcheck" "PASS" "recommended" "installed"; else add_check "shellcheck" "WARN" "recommended" "not found" "brew install shellcheck"; fi
if command -v jq >/dev/null 2>&1; then add_check "jq" "PASS" "recommended" "$(jq --version)"; else add_check "jq" "WARN" "recommended" "not found" "brew install jq"; fi

FREE_KB="$(df -k "$ROOT" | tail -1 | awk '{print $4}')"
FREE_GB=$((FREE_KB/1024/1024))
if [ "$FREE_GB" -ge 20 ]; then
  add_check "Free disk" "PASS" "core" "${FREE_GB} GB free"
elif [ "$FREE_GB" -ge 8 ]; then
  add_check "Free disk" "WARN" "core" "${FREE_GB} GB free" "20 GB+ recommended"
else
  add_check "Free disk" "FAIL" "core" "${FREE_GB} GB free" "Free disk space"
fi

if [ "$core_fail" -eq 0 ]; then CORE_READY=true; else CORE_READY=false; fi
if [ "$core_fail" -eq 0 ] && [ "$full_fail" -eq 0 ]; then FULL_DEV_READY=true; else FULL_DEV_READY=false; fi

if command -v node >/dev/null 2>&1; then
  node - "$TMP_ROWS" "$REPORT_JSON" "$CORE_READY" "$FULL_DEV_READY" <<'NODE'
const fs=require('fs');
const [rows,out,core,full]=process.argv.slice(2);
const checks=fs.readFileSync(rows,'utf8').trim().split('\n').filter(Boolean).map(line=>{
  const [name,status,required_for,detail,fix]=line.split('\t');
  return {name,status,required_for,detail,fix};
});
fs.writeFileSync(out,JSON.stringify({generated_at:new Date().toISOString(),core_ready:core==='true',full_dev_ready:full==='true',checks},null,2)+'\n');
NODE
else
  printf '{"generated_at":null,"core_ready":false,"full_dev_ready":false,"checks":[],"note":"Node.js missing; see Markdown report"}\n' > "$REPORT_JSON"
fi

{
  echo "# SURE macOS Environment Report"
  echo
  echo "- CORE_READY: **$CORE_READY**"
  echo "- FULL_DEV_READY: **$FULL_DEV_READY**"
  echo "- Architecture: **$ARCH**"
  echo
  echo '| Status | Check | Required for | Detail | Fix |'
  echo '|---|---|---|---|---|'
  awk -F '\t' '{gsub(/\|/,"\\|"); printf "| %s | %s | %s | %s | %s |\n", $2,$1,$3,$4,$5}' "$TMP_ROWS"
} > "$REPORT_MD"

printf '\nCORE_READY: %s\nFULL_DEV_READY: %s\nReport: %s\n' "$CORE_READY" "$FULL_DEV_READY" "$REPORT_MD"

if [ "$CORE_READY" != true ]; then exit 2; fi

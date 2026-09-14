#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
REMOTE_URL="https://github.com/lichman0405/SURE.git"
WORK_BRANCH="claude/v0.1-autonomous"

if [ ! -d .git ]; then git init -b main; fi
if git remote get-url origin >/dev/null 2>&1; then
  EXISTING="$(git remote get-url origin)"
  if [ "$EXISTING" != "$REMOTE_URL" ] && [ "$EXISTING" != "git@github.com:lichman0405/SURE.git" ]; then
    echo "Refusing: origin is $EXISTING, expected $REMOTE_URL" >&2; exit 2
  fi
else
  git remote add origin "$REMOTE_URL"
fi

# Refuse to overwrite an already-populated incompatible remote.
git fetch origin --prune >/dev/null 2>&1 || true
if git show-ref --verify --quiet refs/remotes/origin/main; then
  if ! git rev-parse HEAD >/dev/null 2>&1; then
    echo "Remote main already exists but local repository has no commit. Clone/pull the repository instead of overwriting it." >&2; exit 2
  fi
  if ! git merge-base --is-ancestor origin/main HEAD 2>/dev/null && ! git merge-base --is-ancestor HEAD origin/main 2>/dev/null; then
    echo "Remote main has incompatible history. Refusing to overwrite." >&2; exit 2
  fi
fi

if ! git config user.email >/dev/null || ! git config user.name >/dev/null; then
  echo "Configure git user.name and user.email first." >&2; exit 2
fi

if ! git rev-parse HEAD >/dev/null 2>&1; then
  git add .
  git commit -m "chore: bootstrap SURE autonomous development"
elif [ -n "$(git status --porcelain)" ]; then
  git add .
  git commit -m "chore: update SURE autonomous development bootstrap"
fi

# Ensure local main points at the bootstrap commit before first push.
CURRENT="$(git branch --show-current)"
if [ "$CURRENT" != "main" ]; then git branch -f main HEAD; fi

git push -u origin main
if git show-ref --verify --quiet "refs/heads/$WORK_BRANCH"; then git switch "$WORK_BRANCH"; else git switch -c "$WORK_BRANCH"; fi
git push -u origin "$WORK_BRANCH"
printf 'Bootstrap published. Autonomous branch: %s\n' "$WORK_BRANCH"

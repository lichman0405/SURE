#!/usr/bin/env node
// SURE Git safety guard.
//
// Enforces the two Git policies that autonomous development must never break:
//   1. the canonical origin is lichman0405/SURE;
//   2. no force push, and no autonomous merge into `main`.
//
// Usage:
//   node scripts/git-guard.mjs check     # verify remote + branch invariants (exit 1 on violation)
//   node scripts/git-guard.mjs assert-push   # refuse a push that would not be a fast-forward update
//   node scripts/git-guard.mjs assert-branch <branch>  # refuse an autonomous branch other than the allowed one
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CANONICAL_REMOTE = 'https://github.com/lichman0405/SURE.git';
const CANONICAL_SLUG = 'lichman0405/SURE';
const AUTONOMOUS_BRANCH = 'claude/v0.1-autonomous';
const PROTECTED_BRANCHES = ['main'];

/** Run git with argument array (no shell string building). */
function git(args) {
  return execFileSync('git', args, { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}

function tryGit(args) {
  try {
    return { ok: true, out: git(args) };
  } catch (err) {
    return { ok: false, out: `${err.stderr ?? ''}${err.stdout ?? ''}`.trim() || String(err.message) };
  }
}

function normalizeRemote(url) {
  return String(url).trim().replace(/\.git$/, '').replace(/\/+$/, '');
}

const problems = [];

function check() {
  const remotes = tryGit(['remote', '-v']);
  if (!remotes.ok) {
    problems.push(`cannot read git remotes: ${remotes.out}`);
  } else {
    const originLines = remotes.out.split('\n').filter((l) => l.startsWith('origin\t'));
    if (originLines.length === 0) {
      problems.push('origin remote is not configured');
    }
    const fetchLine = originLines.find((l) => l.includes('(fetch)'));
    if (!fetchLine) {
      problems.push('origin has no fetch URL');
    } else {
      const url = fetchLine.split(/\s+/)[1];
      if (normalizeRemote(url) !== normalizeRemote(CANONICAL_REMOTE)) {
        problems.push(`origin fetch URL is ${url}, expected ${CANONICAL_REMOTE}`);
      }
    }
    for (const line of originLines) {
      const m = /^origin\s+(\S+)\s+\(push\)/.exec(line);
      if (m && normalizeRemote(m[1]) !== normalizeRemote(CANONICAL_REMOTE)) {
        problems.push(`origin push URL is ${m[1]}, expected ${CANONICAL_REMOTE}`);
      }
    }
  }

  // A repository-level config that rewrites the push target would defeat the URL check above.
  const pushInsteadOf = tryGit(['config', '--get-regexp', '^remote\\..*\\.pushurl$']);
  if (pushInsteadOf.ok && pushInsteadOf.out) {
    for (const line of pushInsteadOf.out.split('\n')) {
      const url = line.split(/\s+/).slice(1).join(' ');
      if (url && normalizeRemote(url) !== normalizeRemote(CANONICAL_REMOTE)) {
        problems.push(`push URL override detected: ${line}`);
      }
    }
  }

  const branch = tryGit(['rev-parse', '--abbrev-ref', 'HEAD']);
  if (!branch.ok) {
    problems.push(`cannot determine current branch: ${branch.out}`);
  } else {
    const name = branch.out;
    if (PROTECTED_BRANCHES.includes(name)) {
      problems.push(`HEAD is on protected branch ${name}; autonomous work must use ${AUTONOMOUS_BRANCH}`);
    }
  }

  // `git config` entries that would weaken history safety.
  for (const key of ['push.default']) {
    const v = tryGit(['config', '--get', key]);
    if (v.ok && v.out && v.out === 'matching') {
      problems.push(`${key}=matching would push every matching branch; use simple/current`);
    }
  }
  return problems;
}

function assertPush() {
  const found = check();
  if (found.length) return found;

  const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
  if (PROTECTED_BRANCHES.includes(branch)) {
    found.push(`refusing to push protected branch ${branch}`);
    return found;
  }
  if (branch !== AUTONOMOUS_BRANCH) {
    found.push(`refusing to push ${branch}; autonomous checkpoints push ${AUTONOMOUS_BRANCH}`);
    return found;
  }

  // A fast-forward check requires the remote-tracking ref. If it is absent the push is
  // a first publish and is allowed, but we never claim to have verified fast-forward.
  const upstreamRef = `origin/${branch}`;
  const upstream = tryGit(['rev-parse', '--verify', '--quiet', upstreamRef]);
  if (upstream.ok) {
    const ff = tryGit(['merge-base', '--is-ancestor', upstreamRef, 'HEAD']);
    if (!ff.ok) {
      found.push(`push would not fast-forward: ${upstreamRef} is not an ancestor of HEAD`);
    }
  }
  return found;
}

function assertBranch(name) {
  if (PROTECTED_BRANCHES.includes(name)) {
    return [`refusing to check out or commit onto protected branch ${name}`];
  }
  if (name !== AUTONOMOUS_BRANCH) {
    return [`branch ${name} is not the autonomous branch ${AUTONOMOUS_BRANCH}`];
  }
  return [];
}

const [cmd, arg] = process.argv.slice(2);
let result;
if (cmd === 'check') result = check();
else if (cmd === 'assert-push') result = assertPush();
else if (cmd === 'assert-branch') result = assertBranch(arg ?? '');
else {
  console.error('usage: git-guard.mjs check|assert-push|assert-branch <branch>');
  process.exit(2);
}

if (result.length) {
  for (const p of result) console.error(`git-guard: ${p}`);
  process.exit(1);
}
console.log(`git-guard OK: ${CANONICAL_SLUG} / ${git(['rev-parse', '--abbrev-ref', 'HEAD'])}`);

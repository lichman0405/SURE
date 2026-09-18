#!/usr/bin/env node
// SURE — check the non-Windows configuration from Windows.
//
// Usage (from the repository root, in native PowerShell):
//
//   node scripts/check-non-windows.mjs
//
// Why this exists. `CLAUDE.md` requires the Rust core to stay portable to macOS
// and Linux, and the machine it is developed on is Windows. `cargo clippy` with
// no `--target` compiles the Windows `cfg` set, in which `#[cfg(windows)]` items
// are used and `#[cfg(not(windows))]` items are not, so a break in the
// non-Windows arms is invisible here and appears only on a CI runner. That is
// not hypothetical: it is the class that left `ci` red on every platform for 76
// consecutive runs, because on Linux and macOS `cargo clippy --all-targets -- -D
// warnings` failed before the test step ran, so every Unix test behind it was
// unrun rather than failing. The census is item 99 of `progress/HANDOFF.md`.
//
// What it runs. For each Unix target, the same lints and the same `-D warnings`
// as the native gate in `CLAUDE.md`, asked for the other platform:
//
//   cargo clippy --workspace --all-targets --all-features --target <triple> -- -D warnings
//
// `--all-targets` is not decoration: the defect that caused the streak was three
// helper functions in `crates/sure-testkit/tests/integration_thinness.rs` that
// only Windows-gated tests used, and integration tests are a target
// `--all-targets` reaches and a plain `cargo clippy` does not.
//
// What it reaches, and what it does not. Measured 2026-09-19 with
// `grep -rn 'cfg(windows)\|cfg(not(windows))\|target_os\|target_family\|cfg(unix)' crates --include=*.rs`,
// which finds 116 platform-conditional sites in the workspace:
//
//   sure-domain     0 sites   reached
//   sure-protocol   0 sites   reached
//   sure-testkit   12 sites   reached
//   sure-core     100 sites   NOT reached
//   sure-cli        4 sites   NOT reached
//
// The two it does not reach are the two that need a C compiler for the target:
// `sure-core` takes `rusqlite` with `bundled`, so `libsqlite3-sys` compiles
// `sqlite3.c` for the target, and it takes `ureq`, which brings `ring` and its
// own C. Both fail on a machine with no cross C compiler — `cc-rs: failed to
// find tool "x86_64-linux-gnu-gcc"` — and that happens in a build script, so
// `cargo check` fails there exactly as `cargo clippy` does, and `--no-deps` does
// not avoid it. `sure-cli` depends on `sure-core` and inherits it. A crate that
// cannot be checked is not silently treated as checked: the run names it, and
// says what would have to be installed to close the gap.
//
// The gap closes by itself. The command below is attempted for the whole
// workspace first. On a machine that has a cross C compiler — any compiler cc-rs
// can drive through `CC_x86_64_unknown_linux_gnu` and `CC_x86_64_apple_darwin`,
// such as zig's `cc` — the whole workspace is checked and nothing is reported as
// unchecked. The section naming the unchecked crates appears only when the
// whole-workspace run actually failed for that reason, so it is a report of a
// failed attempt and not a claim kept in a source file.
//
// This command does not catch everything, and cannot. Two of the three defects
// in that census are assertions that fail when Unix code *runs* (a Windows path
// classified by Unix path rules, and a Unix branch asserting the wrong
// `kept_open`) — no compile step and no lint on any platform catches those, only
// executing the tests on Unix does, which is what CI is for. What this catches
// is the class that made them invisible: the lint or compile break that aborts
// the Unix job before its tests run.
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

// The two targets `CLAUDE.md`'s portability requirement is checked against.
// Linux and macOS are both `target_family = "unix"` and both false for
// `cfg(windows)`, so Linux alone catches the `windows`/`not(windows)` class —
// the second target is here for the `target_os = "macos"` arms a Linux build
// never sees. Measured 2026-09-19: all 22 `target_os`-qualified sites in the
// workspace are in `sure-core`, which this check cannot compile, so as things
// stand both targets reach the same 12 sites. The second target is kept because
// the day one of those arms moves into a crate this can compile, Linux would
// not see it and macOS would.
const TARGETS = ['x86_64-unknown-linux-gnu', 'x86_64-apple-darwin'];

// Left out of the check set only while the whole-workspace attempt is failing
// because no C compiler for the target is installed. That exclusion is what
// this script does when that attempt fails; it is not a standing claim about
// these crates, and it does not survive a machine that can compile them.
const NEEDS_C_COMPILER = ['sure-core', 'sure-cli'];

// The one failure this script answers for itself rather than passing through:
// cc-rs saying the C compiler is not there. Anything else is the developer's
// own output, printed whole, with the exit status preserved.
const C_COMPILER_MISSING = /failed to find tool "[^"]+": program not found/;

const CLIPPY_TAIL = ['--', '-D', 'warnings'];

/** The cargo invocation for one target, optionally leaving crates out. */
function clippyArgs(target, excluding) {
  return [
    'clippy',
    '--workspace',
    ...excluding.flatMap((crate) => ['--exclude', crate]),
    '--all-targets',
    '--all-features',
    '--target',
    target,
    ...CLIPPY_TAIL,
  ];
}

/** The same invocation as a developer would have to type it. */
function commandLine(args) {
  return ['cargo', ...args].join(' ');
}

/** Run a program with an argument array — never a shell string. */
function spawn(command, args) {
  const result = spawnSync(command, args, {
    cwd: ROOT,
    encoding: 'utf8',
    maxBuffer: 128 * 1024 * 1024,
    windowsHide: true,
  });
  if (result.error) {
    return { status: null, found: false, output: String(result.error.message ?? result.error) };
  }
  return { status: result.status, found: true, output: `${result.stdout ?? ''}${result.stderr ?? ''}` };
}

function print(text = '') {
  process.stdout.write(`${text}\n`);
}

function printIndented(text, indent = '    ') {
  for (const line of text.split(/\r?\n/)) {
    if (line.trim().length > 0) print(`${indent}${line}`);
  }
}

/**
 * The targets rustup has for the active toolchain, or why it cannot say. Both
 * unix targets are installed on the machine this was written on; a machine that
 * lacks one is told which command installs it rather than being left to read a
 * rustc error about `std`.
 */
function installedTargets() {
  const result = spawn('rustup', ['target', 'list', '--installed']);
  if (!result.found) return { available: false, reason: 'rustup was not found on PATH' };
  if (result.status !== 0) {
    return { available: false, reason: `rustup target list --installed exited ${result.status}` };
  }
  return {
    available: true,
    targets: result.output
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean),
  };
}

function main() {
  print('SURE: the non-Windows configuration');
  print();
  print('  Asking cargo clippy for the Windows cfg set is what the local gate does.');
  print('  This asks it for the Unix ones, on a machine that compiles the other.');
  print();

  const installed = installedTargets();
  if (!installed.available) {
    print(`FAILED: ${installed.reason}.`);
    print();
    print('  This check confirms the targets it compiles for through rustup, and will');
    print('  not report a pass it cannot stand behind.');
    return 1;
  }

  const missing = TARGETS.filter((target) => !installed.targets.includes(target));
  if (missing.length > 0) {
    print(`FAILED: this toolchain is missing the target(s) ${missing.join(', ')}.`);
    print();
    print('  Nothing was checked. Install them with:');
    print();
    print(`    rustup target add ${missing.join(' ')}`);
    print();
    print(`  Installed: ${installed.targets.join(', ')}`);
    return 1;
  }

  print(`  targets: ${TARGETS.join(', ')}`);
  print();

  let failed = false;
  const unchecked = new Set();
  const uncheckedFor = new Set();

  for (const target of TARGETS) {
    print(`--- ${target} ---`);
    const wholeArgs = clippyArgs(target, []);
    const whole = spawn('cargo', wholeArgs);
    if (whole.status === 0) {
      print(`  ok  ${commandLine(wholeArgs)}`);
      print();
      continue;
    }

    if (!C_COMPILER_MISSING.test(whole.output)) {
      print(`  FAILED  ${commandLine(wholeArgs)}`);
      print();
      printIndented(whole.output);
      print();
      failed = true;
      continue;
    }

    // The failure this script exists to explain: a build script could not find a
    // C compiler for the target. The Rust crates that do not go through C are
    // still checkable, so ask again without the ones that do, and say plainly
    // which they are.
    for (const crate of NEEDS_C_COMPILER) unchecked.add(crate);
    uncheckedFor.add(target);
    const scopedArgs = clippyArgs(target, NEEDS_C_COMPILER);
    const scoped = spawn('cargo', scopedArgs);
    if (scoped.status === 0) {
      print(`  ok  ${commandLine(scopedArgs)}`);
      print(`      (${NEEDS_C_COMPILER.join(', ')} left out: no C compiler for this target)`);
      print();
      continue;
    }

    print(`  FAILED  ${commandLine(scopedArgs)}`);
    print();
    if (C_COMPILER_MISSING.test(scoped.output)) {
      print('  A crate inside the check set also needs a C compiler for this target.');
      print('  Nothing was installed or skipped for it; the cargo output follows.');
      print();
    }
    printIndented(scoped.output);
    print();
    failed = true;
  }

  if (unchecked.size > 0) {
    print('--- NOT CHECKED ---');
    print();
    print(`  ${[...unchecked].join(', ')} were NOT checked for ${[...uncheckedFor].join(', ')}.`);
    print();
    print('  They need a C compiler for the target: sure-core builds the bundled');
    print('  sqlite3.c through libsqlite3-sys and the C in ring through ureq, and');
    print('  sure-cli depends on sure-core. No such compiler is installed here, and');
    print('  nothing was skipped to work around that: the whole-workspace attempt was');
    print('  made first, and this is what it did.');
    print();
    print('  To close the gap, install a C compiler that emits ELF for');
    print('  x86_64-unknown-linux-gnu and Mach-O for x86_64-apple-darwin, and point');
    print('  cc-rs at it with CC_x86_64_unknown_linux_gnu and CC_x86_64_apple_darwin.');
    print('  Zig ships one as `zig cc` and installs without administrator rights');
    print('  (https://ziglang.org/download/); this repository has not installed or');
    print('  verified it. Re-run this command afterwards: the whole-workspace attempt');
    print('  is made first, so the day it succeeds this section stops appearing.');
    print();
  }

  if (failed) {
    print('FAILED: the crates that could be checked did not pass.');
    return 1;
  }

  if (unchecked.size > 0) {
    print('RESULT: every workspace crate except the ones named above is clean for both');
    print('Unix targets. The ones named above were not checked, so this run says');
    print('nothing about their Unix configuration.');
    return 0;
  }

  print('RESULT: the whole workspace is clean for both Unix targets.');
  return 0;
}

process.exitCode = main();

#!/usr/bin/env node
// SURE — measure a `cargo test` log with two instruments, and refuse to print a
// count when the two disagree.
//
// Usage (from the repository root, in native PowerShell — no Git Bash, no Unix
// shell, no administrator rights, no symlink privilege; the only thing it needs
// is the log it measures):
//
//   node scripts/measure-tests.mjs target/tmp/gates-<label>-test.txt
//   node scripts/measure-tests.mjs <log> [<log> ...]
//   Get-Content <log> | node scripts/measure-tests.mjs -
//
// Exit 0 when every log's two instruments agree and a parent figure was printed.
// Exit 1 when any log is refused: no parent figure is printed for it, so no
// reader of this output can quote a count that could not be attributed.
// Exit 2 when the command itself is wrong (no log named, or one that cannot be
// read) — a refusal to guess, not a verdict about a log.
//
// WHAT THE TWO INSTRUMENTS ARE, because a count without its instrument is the
// defect this file exists to end.
//
//   The parent figure is one `test <name> ... ok` line per test a parent ran.
//   A parent is a suite `cargo test` started: every test it runs that passes
//   prints exactly one such line, so counting the lines counts the tests. This
//   is the instrument that answers "how many tests are there".
//
//   The raw sum is every `test result:` line added up — the number each suite
//   prints in `test result: ok. N passed; ...`. This is the instrument that
//   answers "how many passing test results did this run report", and it is the
//   one a person reaches for first, because the number is already there in the
//   log, printed by cargo, in the same place on every line.
//
//   The difference between them is produced by children: a test that starts a
//   second process to re-run a parent's test. That child process prints its own
//   `test result:` line (so the raw sum counts that test twice) and prints no
//   `test <name> ... ok` line (so the parent figure counts it once). In this
//   repository that is `crates/sure-core/tests/store_concurrency.rs`, whose
//   `store_concurrency` suite spawns ten children, each of which runs one test
//   again with `--exact`; that is why this repository's logs show a difference
//   of exactly 10 and why the same tree reports 2508 or 2518 depending on which
//   instrument was used.
//
//   The second instrument is therefore NOT a cross-check of the first: raw sum
//   minus parent figure is a property of the test tree (how many children re-run
//   a parent's test), not of the code under test. Counting `test result:` lines
//   over-counts by exactly that many, and comparing two runs by raw sum is a
//   comparison that is still valid only because both sides move by the same
//   number. Quoting a raw sum as "the number of tests" is wrong by 10 today and
//   by whatever the children become tomorrow.
//
// WHAT IT REFUSES, AND WHY THE REFUSAL IS THE POINT. The two instruments are
// reconciled from the log alone, without ever using the position of a line in
// the log — attribution by "the last header seen" is not a rule, and item 103 of
// `progress/HANDOFF.md` records the instrument that got it wrong that way:
//
//   * `test result:` lines that no suite header accounts for are child re-runs;
//     a suite header is `     Running ...` or `   Doc-tests ...`, one per suite
//     `cargo test` started, and a child process prints none.
//   * those same lines are cross-checked by content, not by position: a child
//     re-runs a subset, so it is a `test result:` line with a non-zero
//     `filtered out` count.
//   * each child line must report exactly one passed test, because a child that
//     re-runs one parent test contributes one to the raw sum.
//
// When any of those three does not hold, or when the `test ... ok` lines are not
// equal to the raw sum less the child contribution, this prints NO figure and
// exits 1. A number that cannot be attributed must not be quotable, because the
// figure that gets quoted ends up in `progress/state.json` and
// `progress/HANDOFF.md` and then in a release note.
//
// HOW THE NEXT ACCEPTANCE PRODUCES ITS OWN COUNT. This is clause 4 of P15-T021:
// after the six gates have run and their test output is a file, the figure to
// write into `progress/state.json`'s evidence and `progress/HANDOFF.md`'s
// validation section is the one this prints —
//
//   & .\target\tmp\gates.ps1 -Label pXXtYYY
//   node scripts/measure-tests.mjs target/tmp/gates-pXXtYYY-test.txt
//
// — and the `passed=` line in the gate summary `target/tmp/gates-pXXtYYY.txt`
// is NOT that figure: that line adds up every `test result:` line, so it is the
// raw sum and it is 10 higher than the parent figure on every run measured so
// far. Quoting `passed=` from the summary as the number of tests is the mistake
// this task was minted for.
//
// WHY IT IS NOT WIRED INTO CI, in writing, as the sixth gate's own header does
// for the choice it made (`scripts/check-non-windows.mjs`, `docs/testing/TEST_STRATEGY.md`).
//
//   1. Gate membership is the supervisor's decision and this is a measuring
//      instrument, not a check of the product: it decides nothing about whether
//      SURE is correct. Wiring it in is a change to the gate set, which a worker
//      does not make.
//   2. It measures a log that another command produced; it cannot produce one.
//      A CI job would have to capture the test step's output as a file and then
//      measure the file, which changes the test step's command rather than
//      adding a check beside it.
//   3. That change is not free on either shell, and this repository has already
//      paid for it once: on PowerShell a native command inside a pipeline leaves
//      its exit code only in `$LASTEXITCODE`, and on the Unix jobs `| tee` needs
//      `set -o pipefail` — the class of defect recorded in
//      `target/tmp/gates-p15t007-pipefail-fix.txt`. A gate command has to be
//      measured on all three platforms before it becomes a gate, and its cost
//      measured rather than assumed.
//   4. Its cost, measured so that a decision to wire it in starts from a number
//      and not an assumption: 0.068 s for a 236 KB gate log on this machine
//      (`Measure-Command { node scripts/measure-tests.mjs target/tmp/gates-p15t016-record-test.txt }`),
//      which is the cost of a file scan, not of a build.
//
// WHY THIS FILE EXISTS AT ALL, so the next reader does not have to reconstruct
// it from `progress/HANDOFF.md`. An instrument that did this work existed once
// as `target/tmp/measure-run.mjs` — untracked, in `target/`, written the day
// item 103's falsifier fired. It was used by the acceptances from `P15-T017` on,
// and it is now GONE: re-measured 2026-09-21, `test -f target/tmp/measure-run.mjs`
// fails, `git ls-files | grep -c measure-run` is 0, and the only tracked file
// naming it is the prose in `progress/HANDOFF.md`. The same defect is behind
// `P15-T020`'s `target/tmp/regen-sums.mjs`: a tool that decides a published
// figure, living where no clone can reach it and where a `cargo clean` deletes
// it. This file is that instrument, committed, so the next session runs it
// instead of re-deriving it.
//
// Measured 2026-09-21 on the logs in `target/tmp/`: every gate log from
// `gates-base-p14t007-test.txt` to `gates-p15t019-reopen-test.txt` reports a
// raw sum exactly 10 above its `test ... ok` lines, which is the ten children
// above and not a coincidence. Of the 122 `gates-*-test.txt` logs in that
// directory, 120 agree and 2 are refused — `gates-p15t005-post-test.txt` and
// `gates-p15t014-acceptance2-test.txt`, both of which end in the middle of a
// suite (`running 5 tests` and three `... ok` lines, and no `test result:`
// line), so they are interrupted runs and a count from them would be a count of
// part of a run. That refusal is the instrument working.

import fs from 'node:fs';
import process from 'node:process';

// A GitHub Actions job log (what `gh run view --log` writes, and what the
// repository keeps as `target/tmp/ci-*.txt`) prefixes every line with
// `<job>\t<step>\t<ISO timestamp>Z `.
const JOB_LOG_PREFIX = /^(?:[^\t\n]*\t){2}\d{4}-\d{2}-\d{2}T[0-9:.]+Z /;
// Cargo colours the `Running` line when it believes it is on a terminal, and a
// job log preserves the escape. Measured: without this, the 58 `Running` headers
// in `target/tmp/p15t021-ci-win-35392425989.log` match nothing and every child
// line looks like a suite.
const ANSI = /\[[0-9;]*[A-Za-z]/g;

const TEST_OK = /^test (.*) \.\.\. ok(?: <[^>]*>)?$/;
const TEST_RESULT =
  /^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out/;
const SUITE_HEADER = /^ +(Running|Doc-tests) /;

function normalise(line) {
  let text = line.replace(/\r$/, '');
  const prefix = JOB_LOG_PREFIX.exec(text);
  if (prefix) text = text.slice(prefix[0].length);
  return text.replace(ANSI, '');
}

export function measure(text) {
  const seen = {
    okLines: 0,
    resultLines: 0,
    runningHeaders: 0,
    docTestHeaders: 0,
    rawSum: 0,
    failed: 0,
    ignored: 0,
    suitesNotOk: 0,
    childLinesWithFilter: 0,
    childPassedWithFilter: 0,
  };
  const names = new Map();
  for (const raw of text.split('\n')) {
    const line = normalise(raw);
    const result = TEST_RESULT.exec(line);
    if (result) {
      seen.resultLines += 1;
      seen.rawSum += Number(result[2]);
      seen.failed += Number(result[3]);
      seen.ignored += Number(result[4]);
      if (result[1] !== 'ok') seen.suitesNotOk += 1;
      if (Number(result[6]) > 0) {
        seen.childLinesWithFilter += 1;
        seen.childPassedWithFilter += Number(result[2]);
      }
      continue;
    }
    const header = SUITE_HEADER.exec(line);
    if (header) {
      if (header[1] === 'Doc-tests') seen.docTestHeaders += 1;
      else seen.runningHeaders += 1;
      continue;
    }
    const ok = TEST_OK.exec(line);
    if (ok) {
      seen.okLines += 1;
      names.set(ok[1], (names.get(ok[1]) ?? 0) + 1);
    }
  }
  seen.headers = seen.runningHeaders + seen.docTestHeaders;
  seen.childLinesByHeader = seen.resultLines - seen.headers;
  seen.distinctNames = names.size;
  seen.repeatedNames = [...names.values()].filter((n) => n > 1).length;
  return seen;
}

export function verdict(m) {
  const reasons = [];
  if (m.resultLines === 0) {
    reasons.push(
      'no `test result:` line in this file: it is not a `cargo test` log, or the run never reached a suite, and neither instrument can be read from it',
    );
  }
  if (m.headers > m.resultLines) {
    reasons.push(
      `this log has ${m.headers} suite header(s) and only ${m.resultLines} \`test result:\` line(s): it is truncated or was interrupted, a suite started and never reported, and a partial run has no count`,
    );
  } else if (m.resultLines > 0 && m.childLinesByHeader !== m.childLinesWithFilter) {
    reasons.push(
      `the ${m.childLinesByHeader} \`test result:\` line(s) with no suite header are not the ${m.childLinesWithFilter} line(s) that ran a subset, so which lines are children cannot be decided from this log`,
    );
  }
  if (m.childPassedWithFilter !== m.childLinesWithFilter) {
    reasons.push(
      `the ${m.childLinesWithFilter} line(s) that ran a subset report ${m.childPassedWithFilter} passed test(s) between them, and a child that re-runs one parent test reports one: the raw sum cannot be corrected by the number of lines`,
    );
  }
  if (m.resultLines > 0 && m.okLines !== m.rawSum - m.childLinesWithFilter) {
    reasons.push(
      'the `test ... ok` lines and the raw sum less the child contribution are not the same number, so no count of the tests that exist can be attributed',
    );
  }
  return reasons;
}

function agreeingBlock(name, m) {
  const out = [];
  out.push(name);
  out.push(
    `  suite headers        ${m.headers}   (${m.runningHeaders} \`Running\`, ${m.docTestHeaders} \`Doc-tests\`)`,
  );
  out.push(
    `  \`test result:\` lines ${m.resultLines}   (${m.childLinesByHeader} with no header: a child re-ran a parent's test)`,
  );
  out.push(`  raw sum              ${m.rawSum}   every \`test result:\` line, children included`);
  out.push(`  parent figure        ${m.okLines}   \`test <name> ... ok\` lines, one per test a parent ran`);
  out.push(`  difference           ${m.rawSum - m.okLines}   passed tests the children ran again`);
  out.push(
    `  failed ${m.failed}, ignored ${m.ignored}, suites not ok ${m.suitesNotOk}; ${m.distinctNames} distinct parent test names, ${m.repeatedNames} names in more than one suite`,
  );
  if (m.suitesNotOk > 0) {
    out.push(
      '  the parent figure is the tests that PASSED: a suite that is not ok reports its failing tests as failed, not as passed, so this figure is below the number of tests that exist',
    );
  }
  return out.join('\n');
}

function refusingBlock(name, m, reasons) {
  const out = [];
  out.push(name);
  out.push('  REFUSED -- no count is printed for this log, and this exits 1.');
  for (const reason of reasons) out.push(`  reason: ${reason}`);
  out.push('  what the log contains, none of it a count of tests:');
  out.push(`    \`test result:\` lines                              ${m.resultLines}`);
  if (m.childLinesByHeader >= 0) {
    out.push(`    of those, with no suite header                      ${m.childLinesByHeader}`);
  }
  out.push(`    of those, that ran a subset (\`filtered out\` > 0)     ${m.childLinesWithFilter}`);
  out.push(`    passed reported by the lines that ran a subset      ${m.childPassedWithFilter}`);
  out.push(`    suite headers                                      ${m.headers}`);
  return out.join('\n');
}

function readSource(argument) {
  if (argument === '-') return { name: '<stdin>', text: fs.readFileSync(0, 'utf8') };
  return { name: argument, text: fs.readFileSync(argument, 'utf8') };
}

function main(argv) {
  const arguments_ = argv.filter((a) => a !== '--');
  if (arguments_.length === 0 || arguments_.includes('--help') || arguments_.includes('-h')) {
    process.stderr.write(
      'usage: node scripts/measure-tests.mjs <log> [<log> ...]\n' +
        '       Get-Content <log> | node scripts/measure-tests.mjs -\n',
    );
    return 2;
  }
  let refused = 0;
  for (const argument of arguments_) {
    let source;
    try {
      source = readSource(argument);
    } catch (error) {
      process.stderr.write(`cannot read ${argument}: ${error.message}\n`);
      return 2;
    }
    const m = measure(source.text);
    const reasons = verdict(m);
    if (reasons.length > 0) {
      refused += 1;
      process.stdout.write(refusingBlock(source.name, m, reasons) + '\n');
    } else {
      process.stdout.write(agreeingBlock(source.name, m) + '\n');
    }
  }
  return refused === 0 ? 0 : 1;
}

process.exitCode = main(process.argv.slice(2));

'use strict';

/**
 * Runs one package's own cases and exits non-zero if any of them is wrong.
 *
 * One file rather than two, because the two packages' checks have to report the
 * same way: a run that failed in one package and printed nothing in the other
 * would be two accounts of one result.
 */
function check(cases) {
  const wrong = [];
  for (const [name, actual, expected] of cases) {
    if (actual !== expected) {
      wrong.push(`${name}: expected ${expected}, got ${actual}`);
    }
  }
  if (wrong.length > 0) {
    for (const line of wrong) {
      process.stderr.write(`${line}\n`);
    }
    process.stdout.write(`FAILED ${wrong.length} of ${cases.length}\n`);
    process.exit(1);
  }
  process.stdout.write(`ok ${cases.length}\n`);
}

module.exports = { check };

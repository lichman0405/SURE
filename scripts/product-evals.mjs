#!/usr/bin/env node
// The numbers `docs/product/PRODUCT_EVALS.md` states, measured, in one command.
//
// # Why this exists
//
// `P15-T026`'s subject is a document that stated numbers no instrument computed.
// Three of the file's five numeric claims are now compared with a measurement by
// `crates/sure-core/tests/release_gate_runner.rs`, which runs inside
// `cargo test --workspace` — but that test reads the report
// `sure_core::acceptance_report::acceptance_report` produces on its own, and that
// report cannot observe the `repair-regression` case (observing it means starting
// the fixture's two node checks, which no `src` module may do). So the metric for
// that line reads `unmeasured` there and the test only checks that it is *named*
// as unmeasured: **a false percentage on that line passes green.** Measured:
// changing `docs/product/PRODUCT_EVALS.md`'s repair-regression line from `100%`
// to `99%` leaves `cargo test -p sure-core --test release_gate_runner` at
// `16 passed; 0 failed`.
//
// The shipped document is the other reading. `cargo test -p sure-core --test
// acceptance_report_runner` supplies the missing measurement and writes the gate
// to `target/tmp/release-gate.json`, where that metric reads `1 of 1`. This
// script compares the document against *that* document, so every line of the
// prose file is checked against the gate a packaging task actually reads.
//
// # What it refuses to do
//
// It never prints a number it did not read. If the gate is absent, or was taken
// against a corpus that has since been edited, this exits non-zero and says which
// command produces the reading — it does not fall back to counting declarations
// in `evaluation/acceptance-manifest.json`, because a count of declarations is
// not a measurement of results and `sure_core::release_gate`'s own comment says
// that is the trap the false-green rate was built to avoid.
//
// # The two rules, and why they are not the same rule
//
// - A metric the gate measured as a rate, and a line that states a number: the
//   number must be the one the gate wrote (its numerator, or `numerator * 100 /
//   denominator` when the line says `%`). A rate line that states no number is
//   left alone — the file is allowed not to state a figure, and the table below
//   prints the measurement anyway.
// - A metric the gate could not measure: the line must state **no** number. That
//   is the whole of this task in one sentence, and it is why the secret-redaction
//   line carries no `100%` any more. A number no instrument computes is what the
//   file may not keep.
//
// # What it does not check
//
// - `fixtures/adversarial/` is not digested. The gate carries a digest of
//   `evaluation/acceptance-manifest.json` and of nothing else, so editing a
//   fixture's `scenario.json` without re-running the runner leaves this script
//   comparing against a stale reading. Editing the manifest is caught; editing a
//   fixture is not, and that bound is stated rather than papered over.
// - It does not run any test. Producing the gate takes minutes and starts
//   processes; this reads what that run wrote.
// - It does not check the second instance of the same defect. `tasks/SUMMARY.md`
//   used to state a task count nothing computed; the counts are printed by
//   `node scripts/validate-bootstrap.mjs` and `node scripts/taskctl.mjs validate`,
//   which are gates already, and this script does not read that file at all.
//
// # Running it
//
//     node scripts/product-evals.mjs
//
// Node and the repository's own checkout are the whole of the requirement: no
// package install, no administrator rights, no shell beyond the one that starts
// `node`. Exits 0 when every line agrees with the gate, 1 when one does not or
// when a measurement could not be taken.

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/** The document this script is about. */
const DOC = 'docs/product/PRODUCT_EVALS.md';

/** The gate document, written by the runner named below. */
const GATE = 'target/tmp/release-gate.json';

/** The corpus `manifest_digest` is taken over. */
const MANIFEST = 'evaluation/acceptance-manifest.json';

/** The command that writes the gate. Named in every remedy this script prints. */
const RUNNER = 'cargo test -p sure-core --test acceptance_report_runner';

/**
 * The domain tag `sure_core::fingerprint::digest::Digest` is started with for
 * this corpus, read from `crates/sure-core/src/acceptance_report.rs`.
 */
const DIGEST_DOMAIN = 'sure.acceptance-corpus.v1';

/** The line the metric bullets follow, and the phrase the test reader keys on. */
const HEADING = 'Required release metrics';

function stop(reason, remedy) {
  console.error(`product-evals: cannot measure. ${reason}`);
  if (remedy) console.error(`  ${remedy}`);
  console.error('  (no number is printed, because none was read)');
  process.exit(1);
}

function read(file) {
  const full = path.join(ROOT, file);
  try {
    return fs.readFileSync(full);
  } catch (error) {
    stop(`cannot read ${file} from ${ROOT}: ${error.message}`);
  }
}

/**
 * The corpus digest, recomputed exactly as the report computes it: SHA-256 over
 * two length-prefixed fields — the domain tag, then the manifest's own bytes.
 * Reproduced rather than approximated, because a digest that does not agree with
 * the one the gate carries would make every reading look stale.
 */
function corpusDigest(bytes) {
  const hasher = crypto.createHash('sha256');
  const field = (fieldBytes) => {
    const length = Buffer.alloc(8);
    length.writeBigUInt64BE(BigInt(fieldBytes.length));
    hasher.update(length);
    hasher.update(fieldBytes);
  };
  field(Buffer.from(DIGEST_DOMAIN, 'utf8'));
  field(bytes);
  return hasher.digest('hex');
}

/**
 * The bullets under the heading, without their `- ` prefix.
 *
 * The same reading `crates/sure-core/tests/release_gate_runner.rs` makes: blank
 * lines before the first bullet are skipped, and the first line that is not a
 * bullet ends the list. Kept identical on purpose, so the two readers cannot
 * disagree about which lines are the metrics.
 */
function metricLines(text) {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim().startsWith(HEADING));
  if (start < 0) stop(`${DOC} has no line beginning "${HEADING}"`);
  const bullets = [];
  for (const line of lines.slice(start + 1)) {
    const trimmed = line.trim();
    if (trimmed === '' && bullets.length === 0) continue;
    if (!trimmed.startsWith('- ')) break;
    bullets.push(trimmed.slice(2));
  }
  return bullets;
}

/**
 * The first number a line states, as the test reader reads it: the first ASCII
 * digit and the run of digits that follows. `null` when the line states none.
 */
function firstNumber(text) {
  const found = /[0-9]/.exec(text);
  if (!found) return null;
  return Number.parseInt(text.slice(found.index), 10);
}

// --- the reading ----------------------------------------------------------

const text = read(DOC).toString('utf8');
const lines = metricLines(text);

const gateBytes = read(GATE);
let gate;
try {
  gate = JSON.parse(gateBytes.toString('utf8'));
} catch (error) {
  stop(`${GATE} is not JSON: ${error.message}`, `re-run: ${RUNNER}`);
}

if (!Array.isArray(gate.metrics) || gate.metrics.length === 0) {
  stop(`${GATE} carries no metrics`, `re-run: ${RUNNER}`);
}

const manifest = read(MANIFEST);
const measured = corpusDigest(manifest);
if (gate.corpus?.manifest_digest !== measured) {
  stop(
    `${GATE} was taken against a different ${MANIFEST}: the gate names ` +
      `${gate.corpus?.manifest_digest ?? '(no digest)'} and the file now hashes ` +
      `${measured}. The corpus has been edited since the gate was written, so ` +
      'every number in it describes a corpus that has moved.',
    `re-run: ${RUNNER}`,
  );
}

if (lines.length !== gate.metrics.length) {
  stop(
    `${DOC} states ${lines.length} metric line(s) and the gate carries ` +
      `${gate.metrics.length}`,
    `one bullet per metric, in the gate's order, under "${HEADING}"`,
  );
}

// --- the comparison -------------------------------------------------------

const rows = [];
const disagreements = [];

for (const [index, metric] of gate.metrics.entries()) {
  const claim = metric.claim;
  const line = lines[index];
  const stated = firstNumber(line);

  if (!line.includes(claim)) {
    disagreements.push(
      `line ${index + 1} of the bullets is not the metric the gate carries there.\n` +
        `      the document states: ${line}\n` +
        `      the gate claims:     ${claim}`,
    );
    rows.push([claim, '(misaligned)', stated ?? '(none)', 'MISALIGNED']);
    continue;
  }

  const value = metric.value ?? {};

  if (value.state === 'rate') {
    const wanted = line.includes('%')
      ? (value.numerator * 100) / value.denominator
      : value.numerator;
    const shown = `${value.numerator} of ${value.denominator}`;
    if (stated === null) {
      rows.push([claim, shown, '(none stated)', 'ok']);
    } else if (line.includes('%') && wanted % 1 !== 0) {
      disagreements.push(
        `"${claim}" is measured ${shown}, which is not a whole percentage, and the ` +
          `line states ${stated}%`,
      );
      rows.push([claim, shown, `${stated}%`, 'NOT WHOLE']);
    } else if (stated === wanted) {
      rows.push([claim, shown, line.includes('%') ? `${stated}%` : `${stated}`, 'ok']);
    } else {
      disagreements.push(
        `"${claim}" is measured ${shown}, and ${DOC} states ${stated}` +
          `${line.includes('%') ? '%' : ''}`,
      );
      rows.push([claim, shown, line.includes('%') ? `${stated}%` : `${stated}`, 'WRONG']);
    }
    continue;
  }

  // Unmeasured: the line may not state a number at all.
  if (stated === null) {
    rows.push([claim, 'unmeasured', '(none stated)', 'ok']);
  } else {
    disagreements.push(
      `"${claim}" reads unmeasured on this corpus (${value.reason ?? 'no reason given'}\n` +
        `      ...) and ${DOC} states ${stated}. A number no instrument computes is ` +
        'what this file may not keep.',
    );
    rows.push([claim, 'unmeasured', `${stated}`, 'UNMEASURED']);
  }
}

// --- the report -----------------------------------------------------------

const width = Math.max(...rows.map((row) => row[0].length), 'metric'.length);
console.log(`${DOC} against ${GATE}`);
console.log(`gate contract: ${gate.corpus.manifest} @ ${gate.corpus.manifest_digest.slice(0, 12)}…`);
console.log(`gate decision: ${gate.decision}`);
console.log();
console.log(
  `${'metric'.padEnd(width)}  ${'measured'.padEnd(11)}  ${'document'.padEnd(13)}  verdict`,
);
for (const [claim, shown, printed, verdict] of rows) {
  console.log(
    `${claim.padEnd(width)}  ${shown.padEnd(11)}  ${printed.padEnd(13)}  ${verdict}`,
  );
}
console.log();

if (gate.unmeasured_metrics?.length) {
  console.log(`unmeasured on this corpus: ${gate.unmeasured_metrics.join('; ')}`);
  console.log('  a line for one of these states no number, and is checked to state none.');
  console.log();
}

if (disagreements.length) {
  console.error(`product-evals: ${disagreements.length} disagreement(s) with ${DOC}`);
  for (const disagreement of disagreements) console.error(`  - ${disagreement}`);
  console.error(
    `\n  The numbers belong to the gate, not to the prose. Change the gate by ` +
      `changing the corpus (${MANIFEST} and the projects under fixtures/adversarial/), ` +
      `then re-run: ${RUNNER}`,
  );
  process.exit(1);
}

console.log(
  `every one of the ${rows.length} metric lines agrees with the gate. ` +
    `The gate is produced by: ${RUNNER}`,
);
process.exit(0);

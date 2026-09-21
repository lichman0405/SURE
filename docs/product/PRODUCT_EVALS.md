# Product evaluation metrics

Traditional unit-test coverage is not enough. SURE needs product-level evals.

Two corpora carry these metrics, and a reader can open both. The curated acceptance
corpus is `evaluation/acceptance-manifest.json`, whose cases are the fixture projects
under `fixtures/adversarial/`; the secret and protection corpus is
`fixtures/privacy/manifest.json`, driven by `crates/sure-cli/tests/privacy_suite.rs`.
Each line below names the cases it is taken over — or says plainly that no number is
stated for it, and why. A line that states no number states none because nothing in
this tree computes one, and a number nobody measures is worse than a sentence that
says so.

Required release metrics, each line naming the cases it is taken over:

- false green rate on mandatory blocker fixtures: **0**, over the release-blocking cases of the curated acceptance corpus, each row's observed severity and outcome against the `expected_severity` the manifest requires of that case;
- fabricated execution claims: **0**, over the corpus's `tests-not-run` case, whose recording holds no event SURE can use at all and whose rule is that the claim checker must refuse to confirm the claim;
- insufficient evidence fixtures that incorrectly become confirmed: **0**, over the corpus's `stale-test-evidence` and `unknown-evidence` cases, whose recordings hold evidence that exists and cannot be used, and whose rule is that the checker must refuse to confirm the claim;
- mandatory repair-regression fixture caught: **100%**, over the corpus's `repair-regression` case, which is run twice and driven through the repair lifecycle so that the careless repair leaves the finding open and the complete repair closes it;
- secret redaction mandatory fixtures: no number is stated, because none is computed here — the eval is that every case in `fixtures/privacy/manifest.json`, each of them release-blocking, holds, and `cargo test -p sure-cli --test privacy_suite` drives them and fails if any one of them does not;
- benign-mock false-positive corpus tracked explicitly: the corpus's `benign-test-mocks` case, over which the observed severity is compared with the `note` the manifest requires, so that a test double, a worked example or a documentation snippet must not become a `must_fix` production finding;
- default user report passes jargon golden tests: no number is stated, because none is computed here — `cargo test -p sure-cli --test golden_reports` renders the reports under the default settings and asserts the phrases each must contain, and the jargon assertions that do exist are per-string, in the modules that produce the strings, under `cargo test --workspace`, so nothing yet asserts a jargon word list over a whole rendered report.

The numbers above are not typed into this file. They are the release gate's own
values, written into `target/tmp/release-gate.json` by `cargo test -p sure-core
--test acceptance_report_runner`, and `node scripts/product-evals.mjs` reads that
document and this file together and fails if a line disagrees with the gate, or if
the gate was taken against a corpus that has since changed. `cargo test -p sure-core
--test release_gate_runner` compares this file with a measurement of its own during
every `cargo test --workspace`: it compares the bare counts directly, and checks the
repair-regression percentage only to be named as a metric its report cannot observe —
that report runs no fixture, so it has no row that case could be measured in. A line
whose metric the gate reads `unmeasured` keeps no number here.

Do not optimize a single numeric score at the expense of honest unknowns.

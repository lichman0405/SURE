//! P14-T012's acceptance, measured over the real corpus.
//!
//! > 1. *Release-blocking corpus has zero false green.*
//! > 2. *Any unmet case blocks P15 release packaging.*
//!
//! # What this file is
//!
//! It drives `sure_core::release_gate` — the seven metrics
//! `docs/product/PRODUCT_EVALS.md` states as values, and the release decision
//! that reads them — over the report
//! [`sure_core::acceptance_report::acceptance_report`] produces, and asserts the
//! properties a release decision has to rest on.
//!
//! # Where the document is written, and why not here
//!
//! `target/tmp/release-gate.json` is written by
//! `crates/sure-core/tests/acceptance_report_runner.rs`, in the same binary that
//! produces the report the gate is computed from — because the gate must be
//! taken from the report the release is judged on, and that report carries one
//! measurement (`repair-regression`) no `src` module can make. A second report
//! built here would be a second reading of the corpus, which is the thing §3 of
//! the brief forbids. So this file measures the gate's *properties* on the
//! corpus the module can read on its own, and on control corpora written under
//! `target/tmp`; the shipped document is the one the other file writes.
//!
//! # The reading this file asserts most of all
//!
//! The module's own report — what a caller with no process runner gets — has
//! `repair-regression` as a release-blocking case nothing observed. That is
//! where the two clauses meet: the case is not `met`, so the gate is `blocked`,
//! and the reason it gives is that nothing measured it rather than that it
//! failed. `a_release_blocking_case_nothing_observed_blocks` is that assertion.
//!
//! # What this file does not do
//!
//! It opens no store (`store: None`), starts no process, writes nothing except
//! the control corpora under `target/tmp` that it removes and rebuilds, and
//! edits neither `evaluation/` nor `fixtures/`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use sure_core::acceptance_report::{AcceptanceReport, Agreement, acceptance_report};
use sure_core::release_gate::{
    Decision, GATE_CONTRACT, Metric, MetricValue, PRODUCT_EVALS_PATH, ReleaseGate, release_gate,
    release_gate_json,
};

/// The contract this gate is taken against, and the corpus it is produced from.
const MANIFEST_FILE: &str = "evaluation/acceptance-manifest.json";
const FIXTURES: &str = "fixtures/adversarial";

/// The five corpus directories that answer to no case in the manifest.
///
/// Duplicated from `acceptance_report_runner.rs` because a control corpus has to
/// have them: `fixtures_without_a_case` is computed from the disk in both
/// directions, and a corpus that shipped one directory with a reason written for
/// four it does not have is refused by the module. A directory added to the real
/// corpus without a case reddens both files, which is the point of the list.
const OUT_OF_CONTRACT: &[&str] = &[
    "container-unavailable",
    "intent-mismatch",
    "missing-config",
    "rust-tests-fail",
    "rust-tests-pass",
];

/// The release-blocking case the module cannot observe, and the row that makes
/// the module's own gate `blocked`.
const REPAIR_REGRESSION: &str = "repair-regression";

/// The case the runner supplies the measurement for, and the case nothing
/// observes at all.
const BENIGN_MOCK: &str = "benign-test-mocks";

/// The schema the document is validated against, embedded at compile time the
/// way `crates/sure-cli/src/json_report.rs` embeds its own.
const SCHEMA: &str = include_str!("../../../schemas/release-gate.schema.json");

/// One control corpus per test, each with its own directory.
///
/// Named per test rather than shared, and prefixed so they do not collide with
/// `acceptance_report_runner.rs`'s: `cargo test` runs the tests of one binary on
/// several threads, and two controls sharing a directory would delete and
/// rebuild each other's corpus while the other is reading it. A test that had to
/// wait for another is a test whose result depends on the order it ran in.
const CONTROL_MISS: &str = "release-gate miss";
const CONTROL_DECLARATIONS: &str = "release-gate declarations";
const CONTROL_SCHEMA: &str = "release-gate schema";
const CONTROL_PATHS: &str = "release-gate paths";
const CONTROL_ESCALATION: &str = "release-gate escalation";
const CONTROL_PERMITTED: &str = "release-gate permitted";
const CONTROL_EMPTY_RATE: &str = "release-gate empty-rate";

// --- the corpus, driven ---------------------------------------------------

/// The repository root, as `sure_testkit` resolves it.
fn repository_root() -> PathBuf {
    sure_testkit::repository_root()
}

/// The report the module produces on its own over the real corpus.
///
/// Built once and shared: the five scanners run over twenty fixture projects,
/// and a report rebuilt per test would repeat that for each of the tests in this
/// file. The value is the same either way — the control below is what shows the
/// metrics moving — so it is built once and cloned.
fn the_module_report() -> AcceptanceReport {
    static REPORT: std::sync::OnceLock<AcceptanceReport> = std::sync::OnceLock::new();
    REPORT
        .get_or_init(|| {
            acceptance_report(&repository_root())
                .unwrap_or_else(|error| panic!("the corpus did not produce a report: {error}"))
        })
        .clone()
}

/// The gate over the module's own report.
fn the_gate() -> ReleaseGate {
    release_gate(&the_module_report())
}

/// The gate over one control corpus.
fn the_gate_over(control: &Path) -> (AcceptanceReport, ReleaseGate) {
    let report = acceptance_report(control)
        .unwrap_or_else(|error| panic!("the control corpus produced no report: {error}"));
    let gate = release_gate(&report);
    (report, gate)
}

/// A metric by its claim, with a message naming the claims when it is missing.
fn metric<'a>(gate: &'a ReleaseGate, claim: &str) -> &'a Metric {
    gate.metrics
        .iter()
        .find(|metric| metric.claim.contains(claim))
        .unwrap_or_else(|| {
            panic!(
                "the gate has no metric whose claim contains `{claim}`; it has {:?}",
                gate.metrics
                    .iter()
                    .map(|metric| &metric.claim)
                    .collect::<Vec<_>>()
            )
        })
}

/// A metric's value as a rate, or a message saying what it read instead.
fn rate(metric: &Metric) -> (usize, usize) {
    match &metric.value {
        MetricValue::Rate {
            numerator,
            denominator,
        } => (*numerator, *denominator),
        MetricValue::Unmeasured { reason, .. } => panic!(
            "`{}` read `unmeasured` here and this assertion is about a rate: {reason}",
            metric.claim
        ),
    }
}

/// Everything a metric says about itself: what it is computed from, what it
/// covers and what it does not.
fn everything_a_metric_says(metric: &Metric) -> String {
    std::iter::once(&metric.computed_from)
        .chain(&metric.covers)
        .chain(&metric.does_not_cover)
        .cloned()
        .collect::<Vec<String>>()
        .join("\n")
}

/// The metric lines of `docs/product/PRODUCT_EVALS.md`, without their bullets.
///
/// Read rather than duplicated: the module's claims are checked against this
/// file by `the_seven_metrics_are_the_lines_the_document_states`, so a line
/// rewritten there without the module moving is a red test rather than a drift
/// nobody sees.
fn stated_metric_lines() -> Vec<String> {
    let path = repository_root().join(PRODUCT_EVALS_PATH);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let mut bullets = Vec::new();
    let after_header = text
        .lines()
        .skip_while(|line| !line.trim().starts_with("Required release metrics"))
        .skip(1);
    for line in after_header {
        let trimmed = line.trim();
        if trimmed.is_empty() && bullets.is_empty() {
            continue;
        }
        match trimmed.strip_prefix("- ") {
            Some(bullet) => bullets.push(bullet.to_owned()),
            None => break,
        }
    }
    bullets
}

/// The first number a line states, if it states one.
fn first_number(text: &str) -> Option<u64> {
    let start = text.find(|character: char| character.is_ascii_digit())?;
    text[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

// --- the seven metrics ----------------------------------------------------

#[test]
fn the_seven_metrics_are_the_lines_the_document_states() {
    let gate = the_gate();
    let stated = stated_metric_lines();
    assert_eq!(
        stated.len(),
        7,
        "docs/product/PRODUCT_EVALS.md states the release metrics as bullets under `Required \
         release metrics`, and this test reads exactly those: {stated:#?}"
    );
    assert_eq!(
        gate.metrics.len(),
        stated.len(),
        "the gate carries one metric per line the document states"
    );
    for (line, metric) in stated.iter().zip(&gate.metrics) {
        assert!(
            line.contains(&metric.claim),
            "this line of the document is not the metric the gate carries at the same position.\n\
             the document states: {line}\n\
             the gate claims:     {}",
            metric.claim
        );
    }
    let claims: std::collections::BTreeSet<&str> =
        gate.metrics.iter().map(|m| m.claim.as_str()).collect();
    assert_eq!(
        claims.len(),
        gate.metrics.len(),
        "two metrics share a claim, so a reader cannot name one"
    );
}

#[test]
fn the_numbers_the_document_states_are_the_numbers_this_corpus_measured() {
    // Both directions of the same drift check: every number the document states
    // is compared with what this corpus measured, and a metric that states a
    // number and could not be measured here has to be named as unmeasured rather
    // than left to disagree quietly.
    let gate = the_gate();
    let stated = stated_metric_lines();
    let mut compared = 0;
    for (line, metric) in stated.iter().zip(&gate.metrics) {
        let Some(number) = first_number(line) else {
            continue;
        };
        match &metric.value {
            MetricValue::Rate {
                numerator,
                denominator,
            } => {
                assert!(
                    *denominator > 0,
                    "`{}` is a rate over no rows, which is not a measurement: {line}",
                    metric.claim
                );
                let wanted = if line.contains('%') {
                    numerator * 100 / denominator
                } else {
                    *numerator
                };
                assert_eq!(
                    number, wanted as u64,
                    "docs/product/PRODUCT_EVALS.md states `{number}` for `{}` and this corpus \
                     measured {numerator} of {denominator}",
                    metric.claim
                );
                compared += 1;
            }
            MetricValue::Unmeasured { reason, .. } => assert!(
                gate.unmeasured_metrics.contains(&metric.claim),
                "the document states `{number}` for `{}` and this corpus cannot measure it, so it \
                 must be named in `unmeasured_metrics`: {reason}",
                metric.claim
            ),
        }
    }
    assert!(
        compared >= 3,
        "at least the three metrics that state a bare count were compared with the measurement, \
         and only {compared} were"
    );
}

#[test]
fn every_metric_says_what_it_is_computed_from_and_what_it_cannot_see() {
    let gate = the_gate();
    let report = the_module_report();
    let release_blocking: std::collections::BTreeSet<&str> = report
        .cases
        .iter()
        .filter(|row| row.release_blocking)
        .map(|row| row.id.as_str())
        .collect();
    for metric in &gate.metrics {
        assert!(
            !metric.computed_from.trim().is_empty(),
            "`{}` does not say what it is computed from",
            metric.claim
        );
        assert!(
            !metric.covers.is_empty(),
            "`{}` does not say what it sees",
            metric.claim
        );
        assert!(
            !metric.does_not_cover.is_empty(),
            "`{}` does not say what it does not see, so an exclusion in it is a hole rather than a \
             decision",
            metric.claim
        );
        let says = everything_a_metric_says(metric);
        // Either the metric is anchored to rows it names, or it is one of the two
        // that has no rows at all — and then it must read `unmeasured`, because a
        // metric with no denominator is a metric nothing measured.
        let names_a_case = release_blocking.iter().any(|id| says.contains(id))
            || says.contains("`tests-not-run`")
            || says.contains("`stale-test-evidence`")
            || says.contains("`unknown-evidence`")
            || says.contains("`lying-readme`");
        assert!(
            names_a_case || matches!(metric.value, MetricValue::Unmeasured { .. }),
            "`{}` names no corpus case and reads a value anyway, so nothing ties the number to a \
             row a reader can check",
            metric.claim
        );
    }
}

#[test]
fn every_metric_says_what_it_does_with_the_unmeasured_rows() {
    // §4.2 of the brief, as an assertion rather than as prose: the one
    // `cannot_confirm` row and the rows the report carries no severity for have
    // to be accounted for metric by metric. Each of the seven says what it does
    // with both; a metric that stopped saying so reddens here.
    let gate = the_gate();
    for metric in &gate.metrics {
        let says = everything_a_metric_says(metric);
        assert!(
            says.contains("cannot_confirm"),
            "`{}` does not say what it does with the corpus's `cannot_confirm` row, so its silence \
             would be read as a pass",
            metric.claim
        );
        assert!(
            says.contains("null"),
            "`{}` does not say what it does with a row that carries no severity, and \"it did not \
             count against us\" has to be a decision with a reason rather than a default",
            metric.claim
        );
    }
    // The two rows the module's own report cannot observe are the ones that make
    // the two clauses meet, and they are named in the gate rather than folded
    // into a rate.
    let module_only = the_gate();
    assert_eq!(
        module_only
            .blocked_by
            .iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>(),
        vec![REPAIR_REGRESSION],
        "the module's own report observes neither `lying-readme` nor `repair-regression`, and only \
         the second is release-blocking"
    );
}

#[test]
fn a_rate_is_never_taken_over_no_rows() {
    // The honest-unknowns rule as a property of the shape: a metric with no rows
    // to count reads `unmeasured` with its reason, never `0 of 0` and never `0`,
    // because a rate over nothing is not a rate of zero.
    for gate in [
        the_gate(),
        the_gate_over(&control_corpus(
            CONTROL_EMPTY_RATE,
            BENIGN_MOCK,
            "note",
            "the control: a contract the machinery meets, so the gate permits",
        ))
        .1,
    ] {
        for metric in &gate.metrics {
            if let MetricValue::Rate {
                numerator,
                denominator,
            } = &metric.value
            {
                assert!(
                    *denominator > 0,
                    "`{}` reads {numerator} of {denominator}",
                    metric.claim
                );
                assert!(
                    numerator <= denominator,
                    "`{}` reads {numerator} of {denominator}, which is not a rate",
                    metric.claim
                );
            }
        }
    }
}

#[test]
fn the_metrics_that_this_corpus_cannot_measure_read_unmeasured() {
    let gate = the_gate();
    // The secret-redaction corpus and the jargon goldens are other suites and
    // contribute no row to this report. PRODUCT_EVALS.md's own constraint is not
    // to trade honest unknowns for a score, so both read `unmeasured` with a
    // reason, what would have to change, and where the number is measured
    // instead — never `0` and never `100%`.
    for claim in [
        "secret redaction mandatory fixtures",
        "default user report passes jargon golden tests",
    ] {
        let found = metric(&gate, claim);
        let MetricValue::Unmeasured {
            reason,
            would_require,
            measured_elsewhere,
        } = &found.value
        else {
            panic!(
                "`{claim}` reads {:?} on a corpus that holds no row for it",
                found.value
            );
        };
        assert!(!reason.trim().is_empty(), "`{claim}` gives no reason");
        assert!(
            !would_require.trim().is_empty(),
            "`{claim}` does not say what would have to change"
        );
        assert!(
            measured_elsewhere.is_some(),
            "`{claim}` does not name where the number is measured instead, which would leave a \
             reader to conclude nobody has it"
        );
    }
    assert_eq!(
        gate.unmeasured_metrics,
        vec![
            String::from("mandatory repair-regression fixture caught"),
            String::from("secret redaction mandatory fixtures"),
            String::from("default user report passes jargon golden tests"),
        ],
        "the metrics this reading cannot measure are named, and a metric that stopped being measured \
         appears here rather than reading as a zero"
    );
    // The repair-regression metric is a third unmeasured reading, and it is the
    // one that moves: the module's own report cannot observe the case, so the
    // metric says so rather than printing `0 of 0`.
    let repair = metric(&gate, "mandatory repair-regression fixture caught");
    let MetricValue::Unmeasured {
        reason,
        measured_elsewhere,
        ..
    } = &repair.value
    else {
        panic!(
            "`repair-regression` is not observed by the module, so its metric cannot read a rate"
        );
    };
    assert!(
        reason.contains("0` over `0") || reason.contains("no row"),
        "the reason does not say that a rate over no rows is not a rate of zero: {reason}"
    );
    assert!(
        measured_elsewhere
            .as_deref()
            .is_some_and(|where_| where_.contains("acceptance_report_runner")),
        "the metric does not name the runner that does measure it"
    );
}

#[test]
fn the_false_green_rate_is_measured_rather_than_constructed() {
    // Clause 1 of the acceptance. The denominator is the release-blocking rows
    // that carry an observation — not the corpus, and not the corpus's
    // declarations — and the numerator is what the machinery produced on them.
    let report = the_module_report();
    let gate = release_gate(&report);
    let (numerator, denominator) = rate(metric(&gate, "false green rate"));
    assert_eq!(
        denominator, report.totals.release_blocking_observed,
        "the rate is taken over the release-blocking rows that carry an observation"
    );
    assert_eq!(
        numerator, report.totals.release_blocking_unmet,
        "with no escalation on the release-blocking corpus, the rows that are not met are the false \
         greens; the control below is where the two readings part company"
    );
    assert_eq!(
        numerator, 0,
        "the criterion: the release-blocking corpus has zero false green on this run"
    );
    assert_eq!(
        report.totals.release_blocking_unmet, 0,
        "the report agrees: no release-blocking row is unmet"
    );
    assert_eq!(
        denominator + report.totals.release_blocking_cannot_confirm,
        report.corpus.release_blocking,
        "every release-blocking case is either observed or named as unobserved, and none is dropped \
         into a denominator it does not belong in"
    );
}

// --- the gate -------------------------------------------------------------

#[test]
fn a_release_blocking_case_nothing_observed_blocks() {
    // Clause 2, on the reading this build can make on its own: the module's
    // report observes twelve of the corpus's thirteen release-blocking cases and
    // says so about the thirteenth. `cannot_confirm` is not a pass — a gate that
    // needed an `unmet` row to block would permit a release over a case nobody
    // measured, which is the false green this repository exists to catch.
    let report = the_module_report();
    let gate = release_gate(&report);
    assert_eq!(
        gate.decision,
        Decision::Blocked,
        "the module's own report leaves a release-blocking case unobserved, so the release is not \
         permitted from it"
    );
    assert_eq!(gate.blocked_by.len(), 1, "{:#?}", gate.blocked_by);
    let blocked = &gate.blocked_by[0];
    assert_eq!(blocked.id, REPAIR_REGRESSION);
    assert_eq!(blocked.agreement, Agreement::CannotConfirm);
    assert!(
        blocked.why.contains("nothing in this build observed it"),
        "the gate does not say why an unobserved case blocks: {}",
        blocked.why
    );
    assert!(!blocked.comparison.trim().is_empty());
    assert!(!blocked.requirement.trim().is_empty());
    // The decision is tied to the contract it was taken against.
    assert_eq!(gate.corpus.manifest_digest, report.corpus.manifest_digest);
    assert_eq!(gate.corpus.manifest, MANIFEST_FILE);
    assert_eq!(gate.corpus.cases, report.corpus.cases);
    assert_eq!(gate.corpus.release_blocking, report.corpus.release_blocking);
}

#[test]
fn the_gate_permits_when_every_release_blocking_case_is_met() {
    // The other direction, so that `permitted` is a value this gate can reach
    // and not a decision nothing produces. `benign-test-mocks` is the corpus's
    // case for a benign double not becoming a `must_fix` finding, and the five
    // scanners reach `note` on it — the contract here asks for `note`.
    let (report, gate) = the_gate_over(&control_corpus(
        CONTROL_PERMITTED,
        BENIGN_MOCK,
        "note",
        "the control: a contract the machinery meets, so the gate permits",
    ));
    assert_eq!(report.totals.release_blocking, 1);
    assert_eq!(report.totals.release_blocking_met, 1);
    assert_eq!(gate.decision, Decision::Permitted);
    assert!(
        gate.blocked_by.is_empty(),
        "a permitted release names no blocking case: {:#?}",
        gate.blocked_by
    );
    // This control's manifest holds one case, so the metric whose case it does
    // not hold says so — it reads `unmeasured` with the no-row reason rather than
    // a rate, which is the property `a_rate_is_never_taken_over_no_rows` asserts
    // of every metric here.
    let repair = metric(&gate, "mandatory repair-regression fixture caught");
    let MetricValue::Unmeasured { reason, .. } = &repair.value else {
        panic!("a corpus that does not hold the case cannot have a rate over it");
    };
    assert!(
        reason.contains("no row"),
        "the metric does not say the case is absent from this corpus: {reason}"
    );
    assert!(
        gate.unmeasured_metrics
            .iter()
            .any(|claim| claim == &repair.claim)
    );
}

#[test]
fn an_unmet_case_blocks_the_release_and_moves_the_false_green_rate() {
    // The anti-vacuity control, and the one the acceptance turns on. The same
    // fixture, whose `scenario.json` is byte-identical to the shipped one (the
    // next test asserts that), under a contract that requires `must_fix` for a
    // case the machinery reaches `note` on.
    //
    // Reproducible mutation, and the one this test is for: in
    // `sure_core::release_gate::blocking_cases`, replace
    //
    //     let why = match row.agreement {
    //         Agreement::Met => continue,
    //
    // with a `continue` for every arm — i.e. `blocked` is never pushed — and the
    // first assertion below goes red alone while every metric value in the
    // document stays what it is.
    let (report, gate) = the_gate_over(&control_corpus(
        CONTROL_MISS,
        BENIGN_MOCK,
        "must_fix",
        "the control: a contract that requires more than the machinery reaches",
    ));
    assert_eq!(
        gate.decision,
        Decision::Blocked,
        "a release-blocking case the machinery did not meet must block the release: {:#?}",
        gate.blocked_by
    );
    assert_eq!(
        gate.blocked_by
            .iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>(),
        vec![BENIGN_MOCK]
    );
    assert_eq!(gate.blocked_by[0].agreement, Agreement::Unmet);
    assert_eq!(report.totals.release_blocking_unmet, 1);

    // The metric moves with it: the same corpus that reads `0 of 12` on the real
    // contract reads `1 of 1` here, with no declaration changed and no fixture
    // edited. This is what makes the rate a measurement rather than a restatement
    // of the corpus's own `forbidden_outcomes`.
    let (numerator, denominator) = rate(metric(&gate, "false green rate"));
    assert_eq!((numerator, denominator), (1, 1));

    // And the same fixture under the real contract is still `met`, so the
    // control's redness is about the requirement it changed rather than about a
    // recipe that stopped working.
    let real = the_gate();
    let same = metric(&real, "false green rate");
    assert_eq!(rate(same), (0, 12), "the real corpus is unchanged by this");
}

#[test]
fn the_control_changes_no_declaration() {
    // §5.1 of the brief, closed off as evidence: the rate above moved because an
    // observation was re-read against a different requirement, not because a
    // fixture declared anything different. The control copies the fixture's
    // `scenario.json` byte for byte, and this is where that is checked rather
    // than asserted in a sentence.
    let control = control_corpus(
        CONTROL_DECLARATIONS,
        BENIGN_MOCK,
        "must_fix",
        "the control: a contract that requires more than the machinery reaches",
    );
    let shipped = std::fs::read(
        repository_root()
            .join(FIXTURES)
            .join(BENIGN_MOCK)
            .join("scenario.json"),
    )
    .expect("the shipped fixture is readable");
    let copied = std::fs::read(
        control
            .join(FIXTURES)
            .join(BENIGN_MOCK)
            .join("scenario.json"),
    )
    .expect("the control's copy is readable");
    assert_eq!(
        shipped, copied,
        "the control's declaration is not byte-identical to the shipped fixture's, so the difference \
         the control measures could be a difference in what the corpus declared"
    );
    // The thing the control does change is the contract, and only that.
    let control_manifest = std::fs::read_to_string(control.join(MANIFEST_FILE))
        .expect("the control manifest is readable");
    assert!(control_manifest.contains("must_fix"));
    let shipped_manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repository_root().join(MANIFEST_FILE))
            .expect("the manifest is readable"),
    )
    .expect("the manifest is JSON");
    let case = shipped_manifest["cases"]
        .as_array()
        .expect("the manifest declares cases")
        .iter()
        .find(|case| case["id"] == BENIGN_MOCK)
        .expect("the manifest has a case for the fixture this control copies");
    assert_eq!(
        case["expected_severity"], "note",
        "this is a drift check on the control's own premise: the real manifest asks for `note` on \
         this case, which is why the same fixture reads `met` under the real contract and `unmet` \
         under the control's"
    );
}

#[test]
fn the_false_green_rate_is_not_the_reports_unmet_total() {
    // §5.3 of the brief: a metric that reports the report's own totals back is
    // evaluating nothing. An escalation — a case the machinery weighed *heavier*
    // than the contract asks — is `unmet` and is not a false green: it is a false
    // alarm, the other direction. The two readings agree on the real corpus
    // today, where no release-blocking case can escalate because each already
    // requires the heaviest severity; this control is where they part company,
    // and it is the reason the rate is computed from the observed severity rather
    // than from the agreement.
    let (report, gate) = the_gate_over(&control_corpus(
        CONTROL_ESCALATION,
        "demo-analytics",
        "note",
        "the control: a contract that requires less than the machinery reaches",
    ));
    assert_eq!(
        report.totals.release_blocking_unmet, 1,
        "the row is not met, and the control is built so that it is not"
    );
    assert_eq!(gate.decision, Decision::Blocked);
    assert_eq!(
        gate.blocked_by
            .iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>(),
        vec!["demo-analytics"],
        "an escalation is unmet, and any unmet case blocks"
    );
    let (numerator, denominator) = rate(metric(&gate, "false green rate"));
    assert_eq!(
        (numerator, denominator),
        (0, 1),
        "the row is unmet and is not a false green, so the rate and the report's unmet total are \
         two different readings"
    );
}

#[test]
fn the_gate_matches_the_schema_it_ships_with() {
    for (what, gate) in [
        ("the module's own report", the_gate()),
        (
            "a control corpus",
            the_gate_over(&control_corpus(
                CONTROL_SCHEMA,
                BENIGN_MOCK,
                "must_fix",
                "the control: a contract that requires more than the machinery reaches",
            ))
            .1,
        ),
    ] {
        let text = release_gate_json(&gate).expect("the gate serialises");
        let value: serde_json::Value = serde_json::from_str(&text).expect("the gate is JSON");
        let schema = sure_protocol::schema::Schema::parse("release-gate", SCHEMA)
            .unwrap_or_else(|error| panic!("schemas/release-gate.schema.json: {error}"));
        let violations = schema.validate(&value);
        assert!(
            violations.is_empty(),
            "the gate over {what} does not match schemas/release-gate.schema.json: {violations:#?}"
        );
    }
}

#[test]
fn two_runs_are_byte_identical() {
    let first = release_gate_json(&the_gate()).expect("the gate serialises");
    let second = release_gate_json(&the_gate()).expect("the gate serialises");
    assert_eq!(
        first, second,
        "the same report twice is the same document; a difference here is a wall-clock value, a \
         generated identifier or a hash-map iteration order"
    );
}

#[test]
fn no_machine_path_appears_in_the_document() {
    // Over a control corpus, whose root carries a space and a non-ASCII character
    // and lives under `target/tmp`: the strongest case this file can build.
    let control = control_corpus(
        CONTROL_PATHS,
        BENIGN_MOCK,
        "must_fix",
        "the control: a contract that requires more than the machinery reaches",
    );
    let text = release_gate_json(&the_gate_over(&control).1).expect("the gate serialises");
    let root = repository_root().display().to_string();
    let control_text = control.display().to_string();
    for absolute in [
        root.as_str(),
        control_text.as_str(),
        "C:/",
        "C:\\\\",
        "\\\\?\\",
    ] {
        assert!(
            !text.contains(absolute),
            "the document carries {absolute}, so it is a reading of this machine rather than of the \
             corpus"
        );
    }
}

#[test]
fn the_contract_and_the_limitations_are_in_the_document() {
    let gate = the_gate();
    assert_eq!(gate.contract, GATE_CONTRACT);
    for wanted in [
        "only when this document's `decision` is `permitted`",
        "release_blocking",
        "cannot_confirm",
        "unmeasured_metrics",
    ] {
        assert!(
            gate.contract.contains(wanted),
            "the contract does not state `{wanted}`, so a packaging task reading only the document \
             cannot know it: {}",
            gate.contract
        );
    }
    let limitations = gate.limitations.join("\n");
    for wanted in [
        "unmeasured_metrics",
        "Deterministic",
        "forbidden_outcomes",
        "nothing observed",
    ] {
        assert!(
            limitations.contains(wanted),
            "the limitations do not say `{wanted}`: {limitations}"
        );
    }
}

// --- the control corpus ---------------------------------------------------

/// A one-case corpus under `target/tmp` whose contract disagrees with what the
/// fixture's machinery produces.
///
/// The same experiment `acceptance_report_runner.rs` builds, with its own
/// directory names: the two files' test binaries run at the same time under
/// `cargo test`, and two controls sharing a directory would delete and rebuild
/// each other's corpus while the other is reading it.
///
/// Written rather than checked in: `evaluation/` and `fixtures/` are the real
/// corpus and this task does not edit them. The directory is left in place for a
/// reader — it is a build artefact under `target/tmp`, and `cargo clean` removes
/// it.
fn control_corpus(what: &str, fixture: &str, required: &str, expectation: &str) -> PathBuf {
    let root = repository_root()
        .join("target")
        .join("tmp")
        .join(format!("sure 指纹 {what}"));
    let shipped = root.join(FIXTURES).join(fixture);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("evaluation")).expect("the control corpus is creatable");
    copy_tree(&repository_root().join(FIXTURES).join(fixture), &shipped);
    // The five directories the module has a written reason for, created empty.
    for id in OUT_OF_CONTRACT {
        std::fs::create_dir_all(root.join(FIXTURES).join(id))
            .expect("the control corpus is creatable");
    }
    let manifest = serde_json::json!({
        "schema_version": 1,
        "cases": [{
            "id": fixture,
            "release_blocking": true,
            "expected_severity": required,
            "expectation": expectation,
        }],
    });
    std::fs::write(root.join(MANIFEST_FILE), format!("{manifest:#}\n"))
        .expect("the control manifest is writable");
    root
}

/// Copy a directory tree.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", to.display()));
    for entry in std::fs::read_dir(from)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", from.display()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let destination = to.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &destination);
        } else {
            std::fs::copy(&path, &destination).unwrap_or_else(|error| {
                panic!(
                    "cannot copy {} to {}: {error}",
                    path.display(),
                    destination.display()
                )
            });
        }
    }
}

//! The release gate: `docs/product/PRODUCT_EVALS.md` as values, and the block
//! every packaging task has to read.
//!
//! # The two clauses this module exists for
//!
//! `tasks/tasks.json` states `P14-T012`'s acceptance in two sentences, and they
//! are not the same kind of claim:
//!
//! > *Release-blocking corpus has zero false green.*
//!
//! > *Any unmet case blocks P15 release packaging.*
//!
//! The first is a claim about **measured rows**, and [`release_gate`] answers it
//! by reading [`AcceptanceReport`]'s observations and nothing else. The second is
//! a claim about a **mechanism**, and a mechanism that has never been shown to
//! block is the same shape of unearned green as a check that never ran. So the
//! second half of this module is a value — [`ReleaseGate`] — whose `decision` is
//! `blocked` the moment a release-blocking case is not `met`, whose `blocked_by`
//! names every such case with the row's own sentence, and which is written to
//! [`GATE_PATH`] by `crates/sure-core/tests/acceptance_report_runner.rs`, the
//! same binary that writes the report it is computed from. The gate's own
//! properties are measured in `crates/sure-core/tests/release_gate_runner.rs`.
//!
//! `docs/testing/ADVERSARIAL_FIXTURES.md` states the rule in one line — *"A
//! mandatory false green blocks release."* — and until this module existed,
//! nothing in the tree turned that sentence into something a packaging task could
//! read. [`GATE_CONTRACT`] is that sentence, as the document's own field.
//!
//! # The trap, and the measurement that avoids it
//!
//! `crates/sure-testkit/tests/fixture_apps.rs`'s `false_green_violations` reads a
//! fixture's `forbidden_outcomes` and reports *structural* violations: that the
//! list is non-empty, that some outcome has `kind: "false_green"`, and for a
//! release-blocking case that some outcome mentions "green" and that some
//! mentions "passing", "verified" or "working". It is driven over the whole
//! corpus by `every_fixture_forbids_the_false_green_shape`, and it is green
//! today.
//!
//! That makes a false-green rate derived from those declarations **0 by
//! construction and permanently so**: the number could only move if a test that
//! is already green went red, and a number that cannot fail is worse than no
//! number. Every metric in this module is therefore computed from what the
//! machinery that grades a case *produced* — the severity it reached, whether
//! the outcome its rule names happened — against what the manifest *requires*. A
//! reader can see the difference move: `crates/sure-core/tests/release_gate_runner.rs`
//! runs this module over a control corpus whose `scenario.json` is byte-identical
//! to the shipped fixture's and whose manifest asks for more than the machinery
//! reaches, and the false-green rate goes from `0` to `1` without one
//! declaration changing.
//!
//! # What this module does not do
//!
//! It opens no store (`store: None` is the whole corpus's rule), starts no
//! process, writes nothing and knows nothing about a CLI. [`release_gate`] takes
//! an [`AcceptanceReport`] and returns a value, the same shape
//! [`crate::acceptance_report::acceptance_report`] has, so it runs on macOS and
//! Linux CI as it does here.
//!
//! Two of the seven metrics cannot be computed from this corpus at all — the
//! secret-redaction corpus and the jargon golden tests are other suites, and
//! neither of them contributes a row to this report. `docs/product/PRODUCT_EVALS.md`'s
//! own constraint is *"Do not optimize a single numeric score at the expense of
//! honest unknowns"*, so those two read [`MetricValue::Unmeasured`] with the
//! reason, the suite that does measure them, and what would have to change —
//! never as `0`, and never as `100%`.

use serde::{Deserialize, Serialize};

use crate::acceptance_report::{AcceptanceReport, Agreement, Axis, CaseRow, Observation};
use crate::severity::Severity;

/// The version of this document's shape.
///
/// Bumped when the shape changes in a way an older reader would get wrong.
pub const RELEASE_GATE_SCHEMA_VERSION: u32 = 1;

/// Where the seven metrics are written down, and where each [`Metric::claim`]
/// is checked to still be a line of.
pub const PRODUCT_EVALS_PATH: &str = "docs/product/PRODUCT_EVALS.md";

/// Where the runner writes the document, and where a packaging task reads it.
///
/// A build artefact under `target/tmp`, for the reason
/// `crates/sure-core/tests/acceptance_report_runner.rs` gives about the report it
/// writes beside it: it is a reading of this checkout on this day, and a
/// committed one would go stale in a way nothing reddens.
pub const GATE_PATH: &str = "target/tmp/release-gate.json";

/// The contract, in the document rather than only in a commit message.
///
/// This is the sentence a packaging task is meant to be able to read out of the
/// artefact itself: what `permitted` authorises, what `blocked` forbids, and
/// what the decision is *not* about.
pub const GATE_CONTRACT: &str = "P15 may build, package or publish a release of this checkout only \
     when this document's `decision` is `permitted`. The decision rests on \
     `evaluation/acceptance-manifest.json`: every case the manifest marks `release_blocking` must \
     be `met` by an observation, and a release-blocking case that is `unmet` or that nothing \
     observed (`cannot_confirm`) blocks the release and is named in `blocked_by` with the row's own \
     sentence. This is the sentence `docs/testing/ADVERSARIAL_FIXTURES.md` states as \"A mandatory \
     false green blocks release.\" The decision is about the acceptance corpus's release-blocking \
     cases and not about the metrics this document also carries: the two metrics it could not \
     measure are named in `unmeasured_metrics`, and a `permitted` decision does not certify them.";

/// Where a release stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Every release-blocking case met its contract, as measured.
    Permitted,
    /// At least one release-blocking case did not, and [`ReleaseGate::blocked_by`]
    /// names it.
    Blocked,
}

/// What one metric is worth on this corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MetricValue {
    /// A count over named rows, with the denominator it was taken over. Written
    /// as both numbers rather than as a percentage so that a reader can see what
    /// the rate was taken over: a rate of `0` over `0` rows is not a rate of
    /// zero, and [`MetricValue::Unmeasured`] is what this document says instead
    /// of printing one.
    Rate {
        /// The rows the metric counts.
        numerator: usize,
        /// The rows it was taken over.
        denominator: usize,
    },
    /// Nothing this build can compute over this corpus answers this line, and
    /// the metric says why rather than printing a zero.
    Unmeasured {
        /// Why nothing here measures it.
        reason: String,
        /// What would have to change for this document to carry a value.
        would_require: String,
        /// The suite that does measure it today, where one does — named, so a
        /// reader who wants the number can go and get it instead of concluding
        /// that nobody has it.
        measured_elsewhere: Option<String>,
    },
}

/// One line of `docs/product/PRODUCT_EVALS.md`, and what this corpus makes of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metric {
    /// The metric's name as the document names it, checked against that file's
    /// bullets by `crates/sure-core/tests/release_gate_runner.rs`, so a line
    /// rewritten there without this module moving is a red test rather than a
    /// drift nobody sees. The document's *stated value* is deliberately not
    /// parsed: it is markdown, and `P15-T026` owns it.
    pub claim: String,
    /// What this corpus measured.
    pub value: MetricValue,
    /// The rows and values the number was computed from, named so that a reader
    /// can recompute it by hand.
    pub computed_from: String,
    /// What the metric sees, one sentence per decision — including what it does
    /// with a row that carries no severity and with the row nothing observed.
    pub covers: Vec<String>,
    /// What it does not see, with the reason. An exclusion in here is a decision
    /// somebody made and wrote down; an exclusion that is not here is a hole.
    pub does_not_cover: Vec<String>,
}

/// A release-blocking case that stops the release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockingCase {
    /// The manifest's id for the case.
    pub id: String,
    /// Whether the case was graded unmet, or nothing observed it.
    pub agreement: Agreement,
    /// What the contract asks of it, in the manifest's own words.
    pub requirement: String,
    /// The row's own comparison sentence, which carries the measured value — the
    /// gate points at the report's evidence rather than paraphrasing it.
    pub comparison: String,
    /// Why a release is not permitted while this case stands as it does.
    pub why: String,
}

/// Which contract the decision was taken against.
///
/// The digest is the one the report carries over the manifest's bytes, so a
/// packaging task can tell a decision taken against today's contract from one
/// taken against a contract somebody has since edited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateCorpus {
    /// The manifest, relative to the repository root.
    pub manifest: String,
    /// The digest of the manifest's bytes, as the report computed it.
    pub manifest_digest: String,
    /// How many cases the manifest declares.
    pub cases: usize,
    /// How many of them are release-blocking.
    pub release_blocking: usize,
    /// How many release-blocking cases carry an observation.
    pub release_blocking_observed: usize,
    /// How many release-blocking cases nothing observed.
    pub release_blocking_cannot_confirm: usize,
}

/// The seven `docs/product/PRODUCT_EVALS.md` metrics and the release decision,
/// measured against one acceptance report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseGate {
    /// This document's shape version.
    pub schema_version: u32,
    /// What a consumer of this document is entitled to do with it.
    pub contract: String,
    /// Whether a release may proceed.
    pub decision: Decision,
    /// Which contract, and which corpus, the decision is against.
    pub corpus: GateCorpus,
    /// Every release-blocking case that blocks the release, sorted by id.
    pub blocked_by: Vec<BlockingCase>,
    /// The seven metrics, in the order the document states them.
    pub metrics: Vec<Metric>,
    /// The claims of the metrics this document could not measure, so that a
    /// reader of a `permitted` decision is not left to discover a gap by reading
    /// every entry.
    pub unmeasured_metrics: Vec<String>,
    /// What this document does not know.
    pub limitations: Vec<String>,
}

// --- the entry points -----------------------------------------------------

/// The gate, over one acceptance report.
///
/// Pure: it reads the report and returns a value. Nothing is opened, started or
/// written, and the same report twice is the same document — there is no
/// wall-clock value, no generated identifier and no path of the machine that
/// produced it anywhere in the result.
#[must_use]
pub fn release_gate(report: &AcceptanceReport) -> ReleaseGate {
    let blocked_by = blocking_cases(report);
    let metrics = metrics(report);
    let unmeasured_metrics = metrics
        .iter()
        .filter(|metric| matches!(&metric.value, MetricValue::Unmeasured { .. }))
        .map(|metric| metric.claim.clone())
        .collect();
    ReleaseGate {
        schema_version: RELEASE_GATE_SCHEMA_VERSION,
        contract: GATE_CONTRACT.to_owned(),
        decision: if blocked_by.is_empty() {
            Decision::Permitted
        } else {
            Decision::Blocked
        },
        corpus: GateCorpus {
            manifest: report.corpus.manifest.clone(),
            manifest_digest: report.corpus.manifest_digest.clone(),
            cases: report.corpus.cases,
            release_blocking: report.corpus.release_blocking,
            release_blocking_observed: report.totals.release_blocking_observed,
            release_blocking_cannot_confirm: report.totals.release_blocking_cannot_confirm,
        },
        blocked_by,
        metrics,
        unmeasured_metrics,
        limitations: limitations(report),
    }
}

/// The document, as the JSON the schema describes.
///
/// Separated from [`release_gate`] so the runner can write bytes and compare
/// bytes without a second serialisation of its own.
///
/// # Errors
///
/// Returns the serialiser's error when the document cannot be serialised, which
/// is a defect in this module rather than in a corpus.
pub fn release_gate_json(gate: &ReleaseGate) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(gate)?;
    text.push('\n');
    Ok(text)
}

/// Every release-blocking case that stops the release, sorted by id.
///
/// The report's rows are already sorted by id, so this is a filter and not a
/// sort, and the order is the report's rather than this module's.
fn blocking_cases(report: &AcceptanceReport) -> Vec<BlockingCase> {
    let mut blocked: Vec<BlockingCase> = Vec::new();
    for row in &report.cases {
        if !row.release_blocking {
            continue;
        }
        let why = match row.agreement {
            Agreement::Met => continue,
            Agreement::Unmet => format!(
                "`{}` is release-blocking and its requirement did not hold, as measured: {}",
                row.id, row.comparison
            ),
            Agreement::CannotConfirm => format!(
                "`{}` is release-blocking and nothing in this build observed it, so its requirement \
                 has not been shown to hold and permitting a release would be a green over a case \
                 nobody measured: {}",
                row.id, row.comparison
            ),
        };
        blocked.push(BlockingCase {
            id: row.id.clone(),
            agreement: row.agreement,
            requirement: format!(
                "`{}` (expected severity `{}`)",
                row.required.expectation,
                row.required.severity.as_str()
            ),
            comparison: row.comparison.clone(),
            why,
        });
    }
    blocked
}

/// What this document does not know.
fn limitations(report: &AcceptanceReport) -> Vec<String> {
    let mut limitations = vec![
        String::from(
            "The decision is about `evaluation/acceptance-manifest.json`'s release-blocking cases and \
             about nothing else. Two of the seven metrics below cannot be computed over that corpus at \
             all — `secret redaction mandatory fixtures` and `default user report passes jargon golden \
             tests` — and they are named in `unmeasured_metrics`. A `permitted` decision does not \
             certify them, and this sentence is here so that a packaging task reading only `decision` \
             cannot take it to.",
        ),
        String::from(
            "An observation is what the machinery that grades a case produced, and nothing else. Every \
             row of the report behind this document was produced by driving that machinery — no \
             value was read out of the manifest's `expected_severity` or out of a fixture's own \
             `scenario.json` — so a case that reads `met` here is a case the product was measured \
             against, not a case it was asked about.",
        ),
        String::from(
            "Nothing here ran project code or touched a store. The report behind this document was \
             produced under `ExecutionSettings::inspect_only()` with `store: None` everywhere, and the \
             one case whose checks really run runs them from \
             `crates/sure-core/tests/acceptance_report_runner.rs`, which is also the caller that \
             supplies that row's measurement.",
        ),
        String::from(
            "Deterministic. Two runs over the same report produce byte-identical output: no \
             wall-clock timestamp, no generated identifier and no path of the machine that produced \
             it appears anywhere in this document, and every collection is ordered before it is \
             written.",
        ),
    ];
    limitations.push(format!(
        "The corpus is {} case(s), {} of them release-blocking, and this document covers {} of those \
         with an observation and {} without. A release-blocking case nothing observed blocks the \
         release — that is a decision this document states rather than a default — because a case \
         nobody measured is not a case that passed.",
        report.corpus.cases,
        report.corpus.release_blocking,
        report.totals.release_blocking_observed,
        report.totals.release_blocking_cannot_confirm
    ));
    limitations.push(String::from(
        "`fixture_apps.rs`'s `every_fixture_forbids_the_false_green_shape` is a structural check of the \
         corpus's own declarations and is green by construction. Nothing here reads a \
         `forbidden_outcomes` list: every number below is a function of a row's observed value and the \
         manifest's requirement, which is what makes it able to be non-zero. The reason this matters is \
         the one the task was built around — a metric that cannot fail is worse than no metric — and \
         the `false green rate` below is the metric it applies to. The evidence that the number can \
         move is `crates/sure-core/tests/release_gate_runner.rs`: it drives a control corpus under \
         `target/tmp` whose `scenario.json` is byte-identical to the shipped fixture's and whose \
         manifest asks for more than the machinery reaches, and the rate goes from `0 of 12` to `1 of \
         1` without one declaration changing.",
    ));
    limitations
}

// --- the seven metrics ----------------------------------------------------

/// The seven metrics, in the order `docs/product/PRODUCT_EVALS.md` states them.
fn metrics(report: &AcceptanceReport) -> Vec<Metric> {
    vec![
        false_green_rate(report),
        fabricated_execution_claims(report),
        insufficient_evidence_confirmed(report),
        repair_regression_caught(report),
        secret_redaction(),
        benign_mock_tracked(report),
        jargon_goldens(),
    ]
}

/// The rows of the corpus that grade a claim the AI made about its own work.
///
/// The report's recipe table grades five cases through `claim_checker`. Three of
/// them are the corpus's insufficient-evidence family and they are split
/// differently by the next two metrics; the two here are named per metric rather
/// than gathered, because the split *is* the decision being written down.
const NO_USABLE_EVENT: &[&str] = &["tests-not-run"];

/// The two claim cases whose recording holds evidence that exists and cannot be
/// used.
///
/// `stale-test-evidence` holds a `test.finished` event that predates the last
/// write to the file under test; `unknown-evidence` holds an event matched to the
/// claim that cannot be placed in time. Both are the corpus's own words for the
/// difference this split is made on: an event SURE *found* and cannot use, rather
/// than no event at all.
const EVIDENCE_THAT_DOES_NOT_SUFFICE: &[&str] = &["stale-test-evidence", "unknown-evidence"];

/// The corpus's benign-mock case, whose whole subject is the other direction:
/// an escalation.
const BENIGN_MOCK: &[&str] = &["benign-test-mocks"];

/// The corpus's repair-regression case.
const REPAIR_REGRESSION: &[&str] = &["repair-regression"];

/// The rows a metric is computed over, named by id, with the ids that are not in
/// the report's rows returned separately.
///
/// A metric whose case has left the corpus would otherwise read `0/0` and look
/// like a measurement of nothing rather than a corpus that has moved, so the
/// missing ids are carried out and named in the metric's `does_not_cover`.
fn rows<'a>(
    report: &'a AcceptanceReport,
    ids: &[&'static str],
) -> (Vec<&'a CaseRow>, Vec<&'static str>) {
    let mut found: Vec<&'a CaseRow> = Vec::new();
    let mut missing: Vec<&'static str> = Vec::new();
    for id in ids {
        match report.cases.iter().find(|row| row.id == *id) {
            Some(row) => found.push(row),
            None => missing.push(*id),
        }
    }
    (found, missing)
}

/// The rows of a set that carry an observation.
fn observed<'a>(found: &[&'a CaseRow]) -> Vec<&'a CaseRow> {
    found
        .iter()
        .copied()
        .filter(|row| matches!(row.observed, Observation::Observed { .. }))
        .collect()
}

/// A sentence naming the rows a metric could not find, for `does_not_cover`.
fn missing_sentence(missing: &[&'static str]) -> Option<String> {
    if missing.is_empty() {
        return None;
    }
    Some(format!(
        "The report has no row for {}, so this metric is computed over fewer cases than the corpus \
         names — a case that has left the corpus is a change to the corpus and is said here rather \
         than absorbed into a denominator.",
        names(missing)
    ))
}

/// The ids a metric is about, as a list a sentence can name.
fn names(ids: &[&'static str]) -> String {
    ids.iter()
        .map(|id| format!("`{id}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What a metric whose corpus holds its case and nothing observed it would need.
const WOULD_HAVE_TO_BE_OBSERVED: &str = "a report from a run that observes this metric's \
     case(s) — the machinery that grades them is the machinery of this product, and the real \
     corpus's report is produced by `crates/sure-core/tests/acceptance_report_runner.rs`. Nothing \
     here can be supplied by hand: a measurement typed into a document is the false green this gate \
     exists to catch.";

/// Where a metric that read `unmeasured` can be read instead.
const MEASURED_ELSEWHERE: &str = "over the real corpus, through \
     `cargo test -p sure-core --test acceptance_report_runner`: that runner drives \
     `sure_core::acceptance_report` over `evaluation/acceptance-manifest.json` and \
     `fixtures/adversarial/`, and it is the caller that supplies the one measurement no `src` module \
     can make. This document is computed from the report that caller produces when it is the one \
     reading the corpus.";

/// The value of a metric over the rows it measured, or `unmeasured` when it
/// measured none.
///
/// A metric with no denominator is a metric nothing measured, and `0 of 0` is
/// not a rate of zero — it would read as measured and failed when nothing
/// measured it. The reason differs by why the denominator is empty: the case can
/// be absent from the corpus the gate was taken against, or present in it and
/// observed by nothing, and those are different facts about a corpus that a
/// reader has to be able to tell apart.
fn value_of(
    numerator: usize,
    measured: &[&CaseRow],
    missing: &[&'static str],
    would_require: &str,
) -> MetricValue {
    if !measured.is_empty() {
        return MetricValue::Rate {
            numerator,
            denominator: measured.len(),
        };
    }
    let reason = if missing.is_empty() {
        String::from(
            "the report this gate was computed from carries no observation of the case(s) this metric \
             is about: they are in the corpus, and nothing in the run that produced the report observed \
             them, so there is no row to take a rate over. A rate of `0` over `0` rows is not a rate of \
             zero — it would read as measured and failed when the truth is that nothing measured it.",
        )
    } else {
        format!(
            "the report this gate was computed from has no row for {} at all: the corpus it was taken \
             against does not hold the case(s), so there is nothing here to measure and nothing that \
             could honestly be written as zero.",
            names(missing)
        )
    };
    MetricValue::Unmeasured {
        reason,
        would_require: would_require.to_owned(),
        measured_elsewhere: Some(MEASURED_ELSEWHERE.to_owned()),
    }
}

/// The false-green rate over the release-blocking corpus.
///
/// Clause 1 of the acceptance, and the metric the whole task is built around.
fn false_green_rate(report: &AcceptanceReport) -> Metric {
    let blocking: Vec<&CaseRow> = report
        .cases
        .iter()
        .filter(|row| row.release_blocking)
        .collect();
    let measured = observed(&blocking);
    let false_greens: Vec<&CaseRow> = measured
        .iter()
        .copied()
        .filter(|row| is_a_false_green(row))
        .collect();
    let named = |rows: &[&CaseRow]| {
        if rows.is_empty() {
            String::from("none")
        } else {
            rows.iter()
                .map(|row| row.id.clone())
                .collect::<Vec<_>>()
                .join(", ")
        }
    };

    let mut does_not_cover = vec![
        String::from(
            "A release-blocking case that nothing observed — a row the report reads `cannot_confirm` \
             for — is in neither the numerator nor the denominator. A rate over rows nothing measured \
             is a rate over nothing rather than a rate of zero, and the exclusion is a decision with a \
             reason: nothing in this build observes the case, which the report states in the row's own \
             `missing` sentence. Such a case does not become invisible by being excluded — it blocks \
             the release, which is the gate's rule and not this metric's.",
        ),
        String::from(
            "Whether the corpus declares enough forbidden outcomes is not this metric's subject. \
             `crates/sure-testkit/tests/fixture_apps.rs`'s \
             `every_fixture_forbids_the_false_green_shape` is the check that asks that question, it is \
             structural, and it is green by construction; this metric asks what the machinery \
             produced, which is the question that can be answered `no`. Reading `forbidden_outcomes` \
             to compute this number would have made it `0` for as long as that test stayed green — a \
             metric that cannot fail, which is worse than no metric.",
        ),
        String::from(
            "This is not the report's `release_blocking_unmet` under another name. A release-blocking \
             case graded on its severity that the machinery weighed *heavier* than the contract asks \
             is `unmet` and is **not** a false green: it is a false alarm, which is the other \
             direction. The two readings agree on today's corpus, where no release-blocking case can \
             escalate because every one of them already requires the heaviest severity in the \
             vocabulary, and they are different rules that would part company on a corpus where one \
             could.",
        ),
    ];
    does_not_cover.push(format!(
        "The whole of this rate rests on the corpus's release-blocking rows: {} of them carry an \
         observation and are the denominator. The cases it counts today are {}.",
        measured.len(),
        named(&false_greens)
    ));

    Metric {
        claim: String::from("false green rate on mandatory blocker fixtures"),
        value: if blocking.is_empty() {
            MetricValue::Unmeasured {
                reason: String::from(
                    "the report this gate was computed from holds no release-blocking case at all, so \
                     there is nothing to take a rate over. A rate of `0` over `0` rows is not a rate of \
                     zero, and a corpus with nothing in it is not a corpus that passed: this is the \
                     one reading in which the criterion's first clause would be true for a reason \
                     nobody would accept.",
                ),
                would_require: String::from(
                    "a manifest with at least one case marked `release_blocking` — the real corpus \
                     holds thirteen.",
                ),
                measured_elsewhere: Some(MEASURED_ELSEWHERE.to_owned()),
            }
        } else {
            value_of(
                false_greens.len(),
                &measured,
                &[],
                WOULD_HAVE_TO_BE_OBSERVED,
            )
        },
        computed_from: String::from(
            "the report's rows whose `release_blocking` is true, the observed value in each of them — \
             the severity the grading machinery reached on the severity axis, and whether the outcome \
             the case's rule names happened on the outcome axis — against the `expected_severity` the \
             manifest requires of that case",
        ),
        covers: vec![
            String::from(
                "On the severity axis a row is a false green when the machinery reached a severity \
                 lighter than the contract asks, or reached nothing at all, or carried a name the \
                 manifest's vocabulary does not have. Those are the states in which a run can read \
                 green over a defect the contract calls release-blocking, and each one is measured \
                 rather than inferred.",
            ),
            String::from(
                "On the outcome axis a row is a false green when the outcome the case's rule names did \
                 not happen. For these cases the forbidden outcome and the unmet requirement are the \
                 same event — `dangerous-delete`'s rule is that a request declared held is held, and \
                 the corpus's forbidden outcome is that it is answered `allow` — so a row that is not \
                 `met` here is a row where the act the case exists to stop went ahead.",
            ),
            String::from(
                "Nine observed rows carry `severity: null`, and the two axes read it differently on \
                 purpose. On the severity axis it means nothing fired at all and is counted as a false \
                 green. On the outcome axis it means the machinery that graded the case answers with a \
                 decision, an assessment or a run aggregate and computes no severity in the manifest's \
                 vocabulary; it is not counted as a false green, because inventing a severity for it \
                 would be this document making a measurement up rather than reading one.",
            ),
            String::from(
                "A row whose observed severity is not a name the manifest's vocabulary has is counted \
                 as a false green rather than dropped: this document cannot show such a row to be a \
                 blocker, and the safe direction is to count what cannot be shown. The report's own \
                 runner asserts that never happens.",
            ),
        ],
        does_not_cover,
    }
}

/// Whether one observed row is the state the case forbids.
fn is_a_false_green(row: &CaseRow) -> bool {
    let Observation::Observed { axis, severity, .. } = &row.observed else {
        return false;
    };
    match axis {
        Axis::Severity => weighs_below(severity.as_deref(), row.required.severity),
        // The case is graded on whether the outcome happened, and the outcome it
        // names is the negation of what the corpus forbids.
        Axis::Outcome => row.agreement == Agreement::Unmet,
    }
}

/// Whether an observed severity is below what a case requires.
///
/// `None` is "nothing fired at all" and is below every requirement. A name this
/// vocabulary does not have is below every requirement too, deliberately: see
/// the metric's `covers`.
fn weighs_below(reached: Option<&str>, required: Severity) -> bool {
    match reached {
        None => true,
        Some(name) => match severity_of(name) {
            Some(reached) => reached.rank() < required.rank(),
            None => true,
        },
    }
}

/// Whether an observed severity is above what a case requires — the false-alarm
/// direction, which is the benign-mock case's whole subject.
fn weighs_above(reached: Option<&str>, required: Severity) -> bool {
    match reached {
        None => false,
        Some(name) => severity_of(name).is_some_and(|reached| reached.rank() > required.rank()),
    }
}

/// A severity by the wire name a row carries, or `None` when the name is not one
/// this vocabulary has.
fn severity_of(name: &str) -> Option<Severity> {
    [
        Severity::MustFix,
        Severity::ShouldFixFirst,
        Severity::CanFixLater,
        Severity::Note,
    ]
    .into_iter()
    .find(|severity| severity.as_str() == name)
}

/// Counts the rows of a named set that are not `met`, over the rows observed.
///
/// The three claim-graded metrics are the same shape: the row's rule is that the
/// case is met when the checker refuses to confirm the claim, so a row that is
/// not `met` is a claim that became confirmed. Returns the count of those, the
/// rows the rate is over, and the ids the report has no row for.
fn claims_not_refused<'a>(
    report: &'a AcceptanceReport,
    ids: &[&'static str],
) -> (usize, Vec<&'a CaseRow>, Vec<&'static str>) {
    let (found, missing) = rows(report, ids);
    let measured = observed(&found);
    let confirmed = measured
        .iter()
        .filter(|row| row.agreement != Agreement::Met)
        .count();
    (confirmed, measured, missing)
}

/// A claim of execution that nothing in the recording supports.
fn fabricated_execution_claims(report: &AcceptanceReport) -> Metric {
    let (confirmed, measured, missing) = claims_not_refused(report, NO_USABLE_EVENT);
    let mut does_not_cover = vec![
        String::from(
            "This metric reads the report's own grading rule for the rows it names, and the rule for \
             a claim case *is* \"the case is met when the checker refuses to confirm the claim\". It \
             therefore cannot disagree with the report on those rows, and that is said here rather \
             than left for a reader to find out: what it adds is the corpus's cases named with a \
             denominator, which the report's totals do not give them.",
        ),
        String::from(
            "`stale-test-evidence` and `unknown-evidence` are the corpus's other two claim-graded \
             cases and they are **not** in this metric's denominator. A run happened in both \
             recordings — SURE has the event and cannot use it — which is insufficient evidence rather \
             than a fabricated claim, and it is the next metric's subject. The split follows the \
             corpus's own sentences for the two families; it is not two readings of the same rows.",
        ),
        String::from(
            "Six further fixtures forbid this shape in their own words without this report measuring \
             it: `fake-payment`, `fake-auth`, `missing-migration`, `missing-config` and \
             `external-unverified` each forbid, in their own sentences, their own check being taken as \
             evidence — four of them naming the `npm test` or `python scripts/check.py` exit code, and \
             `missing-migration` naming a check that builds a fresh database on every run — and \
             `fake-email` forbids console output that looks like an email being taken as evidence that \
             one was sent. The report's recipes for those cases drive the detection machinery and not \
             the claim checker, so no row of this report measures that shape at all. It is a real gap \
             between what the corpus's declarations forbid and what the report observes, and naming \
             it is the honest alternative to computing a number that would not have seen it.",
        ),
        String::from(
            "The corpus's one `cannot_confirm` row is not one of this metric's cases. `lying-readme` \
             is a `scenario.json` with no project behind it, so no call in this build observes it; it \
             is also not release-blocking, which is why it neither appears in this denominator nor \
             blocks the gate. The exclusion is the report's and not this metric's — the row is \
             `cannot_confirm` there — and the gate's own rule for a release-blocking case nothing \
             observed is stated on `repair-regression`'s entry below.",
        ),
    ];
    if let Some(missing) = missing_sentence(&missing) {
        does_not_cover.push(missing);
    }
    Metric {
        claim: String::from("fabricated execution claims"),
        value: value_of(confirmed, &measured, &missing, WOULD_HAVE_TO_BE_OBSERVED),
        computed_from: String::from(
            "the report's row for the corpus's `tests-not-run` case — the one \
             release-blocking case whose recording holds no event SURE can use at all — and that \
             row's agreement, whose rule is that the claim checker must refuse to confirm the claim",
        ),
        covers: vec![
            String::from(
                "The claim the AI made about its own tests is fabricated in the sense that nothing in \
                 the recording supports it: the fixture's declared recording carries an event the \
                 harness named `mytest.finished`, which is not a family SURE recognises, and its \
                 `counts.passed: 12` is a number a harness wrote about itself. A row that is not `met` \
                 is a build that took that event as evidence.",
            ),
            String::from(
                "The row's observed severity is `null`, and that is the outcome axis's meaning of it: \
                 `claim_checker` answers with a `ClaimAssessment` and computes no severity in the \
                 manifest's vocabulary. It is not read as \"nothing fired\" and it is not counted \
                 against this metric.",
            ),
        ],
        does_not_cover,
    }
}

/// Evidence that exists, cannot be used, and was counted anyway.
fn insufficient_evidence_confirmed(report: &AcceptanceReport) -> Metric {
    let (confirmed, measured, missing) = claims_not_refused(report, EVIDENCE_THAT_DOES_NOT_SUFFICE);
    let mut does_not_cover = vec![
        String::from(
            "This metric reads the report's own grading rule for the two rows it names, exactly as the \
             metric above does, and cannot disagree with the report on them. What it adds is the two \
             cases named with a denominator.",
        ),
        String::from(
            "`tests-not-run` is not in this denominator: there SURE has no event it can use at all, \
             which is a fabricated claim rather than insufficient evidence, and it is the previous \
             metric's.",
        ),
        String::from(
            "Neither row carries a severity in the manifest's vocabulary — `claim_checker` answers \
             with an assessment — so `severity: null` on these rows is the outcome axis's meaning and \
             not a measurement of nothing. There is no severity axis reading to make here.",
        ),
        String::from(
            "The corpus's one `cannot_confirm` row, `lying-readme`, is not one of this metric's cases: \
             the case records no test claim at all, and nothing in this build observes it. It is not \
             release-blocking, so it does not block the gate either — the gate's rule for a \
             release-blocking case nothing observed is stated on `repair-regression`'s entry below.",
        ),
    ];
    if let Some(missing) = missing_sentence(&missing) {
        does_not_cover.push(missing);
    }
    Metric {
        claim: String::from("insufficient evidence fixtures that incorrectly become confirmed"),
        value: value_of(confirmed, &measured, &missing, WOULD_HAVE_TO_BE_OBSERVED),
        computed_from: String::from(
            "the report's rows for `stale-test-evidence` and `unknown-evidence` — the two \
             cases whose recordings hold evidence that exists and cannot be used — and each row's \
             agreement, whose rule is that the checker must refuse to confirm the claim",
        ),
        covers: vec![
            String::from(
                "`stale-test-evidence`: a `test.finished` event with `success: true` exists and \
                 predates the last write to the file under test, so the suite that passed and the code \
                 being shipped are not the same code. `unknown-evidence`: the event exists and cannot \
                 be placed in time. Both are evidence SURE found and will not use; a row that is not \
                 `met` is a build that used it.",
            ),
            String::from(
                "The corpus forbids the other direction on both cases as well — reporting the test run \
                 as failed, or the event as malformed — and this metric counts neither. A build that \
                 contradicted the recording would read `unmet` here for the same reason a false \
                 confirmation does; the row's own comparison sentence is where the difference is \
                 visible, and the assessments it names are not the same value.",
            ),
        ],
        does_not_cover,
    }
}

/// The corpus's mandatory repair-regression case.
fn repair_regression_caught(report: &AcceptanceReport) -> Metric {
    let (found, missing) = rows(report, REPAIR_REGRESSION);
    let measured = observed(&found);
    let caught = measured
        .iter()
        .filter(|row| row.agreement == Agreement::Met)
        .count();
    let mut does_not_cover = vec![
        String::from(
            "**A report the runner did not supply this measurement to has no denominator here, and \
             this metric then reads `unmeasured` rather than `0%`.** Observing this case means \
             starting the fixture's two node checks, `crates/sure-core/tests/spawn_sites.rs`'s rule \
             two forbids a `src` module from naming the type that starts one, and \
             `sure_core::support`'s level-C ceiling rests on no product path running project code. \
             `crates/sure-core/tests/acceptance_report_runner.rs` is the caller that supplies it. A \
             release decision taken from a module-only report therefore cannot claim this metric — and \
             because the case is release-blocking, that report blocks the release instead, which is \
             what this document does with a case nobody measured.",
        ),
        String::from(
            "The case is graded on its outcome and its row carries a measured severity (`must_fix`, \
             what SURE's own node proposer gives a declared test script) as well as the outcome, so an \
             escalation would be visible in the row even though it is not graded on it. That severity \
             is measured rather than `null`, which is the difference between an outcome-axis row the \
             product weighed and one where the machinery that graded it computes no severity in this \
             vocabulary at all.",
        ),
        String::from(
            "The corpus's one `cannot_confirm` row, `lying-readme`, is not this metric's case, and it \
             is not release-blocking. A release-blocking `cannot_confirm` looks like this metric's own \
             case in a report produced without a process runner — see the first entry above — and it \
             blocks the gate rather than being folded into a rate.",
        ),
    ];
    if let Some(missing) = missing_sentence(&missing) {
        does_not_cover.push(missing);
    }
    Metric {
        claim: String::from("mandatory repair-regression fixture caught"),
        value: value_of(
            caught,
            &measured,
            &missing,
            "a report produced by `crates/sure-core/tests/acceptance_report_runner.rs`, which copies \
             the fixture under `target/tmp`, runs both members' declared checks with \
             `sure_core::process`, drives the repair lifecycle over the results and supplies the \
             measurement — the drive that file makes in full.",
        ),
        computed_from: String::from(
            "the report's row for `repair-regression`, which the runner supplies after \
             running the fixture's two node checks twice and driving the repair lifecycle over the \
             results: the careless repair must leave the finding open with the regressed member \
             blocking a run that is not green, and the complete repair must close it with every \
             selected check passing",
        ),
        covers: vec![
            String::from(
                "The numerator counts the row as caught when its agreement is `met`, which is the \
                 case's rule holding as measured: the careless half blocked and the complete half \
                 closed. Both halves are measured against each other, so the rule is not a predicate \
                 nothing could satisfy.",
            ),
            String::from(
                "The metric is a rate over the rows the report observed, not over the corpus: with the \
                 runner's measurement the denominator is 1, and without it the denominator is 0 and \
                 the value is `unmeasured`.",
            ),
        ],
        does_not_cover,
    }
}

/// Secret redaction on the mandatory privacy corpus.
///
/// Unmeasurable here, and said so rather than printed as a zero.
fn secret_redaction() -> Metric {
    Metric {
        claim: String::from("secret redaction mandatory fixtures"),
        value: MetricValue::Unmeasured {
            reason: String::from(
                "The corpus this line is about is `fixtures/privacy/manifest.json`, and this document \
                 is produced against `evaluation/acceptance-manifest.json` and \
                 `fixtures/adversarial/`. The two corpora share no case, so this report has no row a \
                 redaction case could be measured in and there is no denominator here to take a rate \
                 over. Computing `25 of 25` from the privacy manifest's entries would be counting \
                 declarations rather than measuring a result — the same trap the false-green rate \
                 above was built to avoid — and this document does not do it.",
            ),
            would_require: String::from(
                "this document reading a second corpus and a second runner: the privacy suite's \
                 per-case outcomes would have to reach the gate as values, which is a decision about \
                 what the release gate covers rather than a line to add quietly. Until that decision \
                 is made, the honest form of this line here is \"unmeasured\", not `0` and not `100%`.",
            ),
            measured_elsewhere: Some(String::from(
                "`cargo test -p sure-cli --test privacy_suite`, whose \
                 `every_case_in_the_mandatory_corpus_passes` fails if any case fails — so the `100%` \
                 this line states is a fact about a run of that suite today rather than a number this \
                 document carries.",
            )),
        },
        computed_from: String::from(
            "nothing: this report has no row for any case in \
             `fixtures/privacy/manifest.json`",
        ),
        covers: vec![String::from(
            "Nothing. This entry exists so that the line is not silently absent from a document a \
             packaging task reads, and so that a reader can see the reason rather than infer a zero.",
        )],
        does_not_cover: vec![
            String::from(
                "The privacy corpus itself, entirely: `fixtures/privacy/manifest.json` holds 25 cases, \
                 eight driven end to end and seventeen pointers at tests that must still exist, all of \
                 them release-blocking, and not one of them appears in the report this gate is \
                 computed from.",
            ),
            String::from(
                "It is not part of this document's decision either. The decision is about the \
                 acceptance corpus's release-blocking cases, and this entry is named in \
                 `unmeasured_metrics` so that a `permitted` decision cannot be read as covering it. \
                 The acceptance corpus's `cannot_confirm` row and its null-severity rows are outside \
                 this metric entirely, because it has no rows at all: it is not that they were \
                 excluded, it is that nothing here could have seen them.",
            ),
        ],
    }
}

/// The benign-mock false-positive corpus.
fn benign_mock_tracked(report: &AcceptanceReport) -> Metric {
    let (found, missing) = rows(report, BENIGN_MOCK);
    let measured = observed(&found);
    let not_escalated = measured
        .iter()
        .copied()
        .filter(|row| !weighs_above(observed_severity(row), row.required.severity))
        .count();
    let mut does_not_cover = vec![
        String::from(
            "The corpus's four `forbidden_outcomes` for this case are not what this number reads. Two \
             of them are `kind: \"false_green\"` and two are `kind: \"false_positive\"`, and \
             `crates/sure-core/tests/benign_fixture_e2e.rs` says of the second kind that \"nothing in \
             the tree reads it as a rule\": `fixture_apps.rs` reads the `false_green` entries and \
             requires one of them, and the other is written down for a reader. The evidence here is \
             the report's row, so this metric is the case's declared false-positive subject measured \
             rather than restated.",
        ),
        String::from(
            "The escalation direction is held over this fixture by the equality rule the report grades \
             every severity-axis row by, and over the corpus by \
             `crates/sure-core/tests/acceptance_report_runner.rs`'s \
             `an_escalation_does_not_read_met`, which uses `demo-analytics` rather than this case. A \
             `benign-test-mocks` that escalated would move the number here; the corpus has one such \
             case and this metric covers exactly it.",
        ),
        String::from(
            "`severity: null` on this row would mean nothing fired at all, which is a miss and not a \
             false positive. The numerator counts it as not escalated, which is true and is not the \
             same statement as \"the case passed\": the row's own agreement is where the miss shows.",
        ),
        String::from(
            "The corpus's one `cannot_confirm` row, `lying-readme`, is not this metric's case, and it \
             is not release-blocking, so it neither enters this denominator nor blocks the gate. A \
             release-blocking case that nothing observed blocks — that rule is stated on \
             `repair-regression`'s entry — and it is a rule about the gate rather than about this \
             count of false positives.",
        ),
    ];
    if let Some(missing) = missing_sentence(&missing) {
        does_not_cover.push(missing);
    }
    Metric {
        claim: String::from("benign-mock false-positive corpus tracked explicitly"),
        value: value_of(
            not_escalated,
            &measured,
            &missing,
            WOULD_HAVE_TO_BE_OBSERVED,
        ),
        computed_from: String::from(
            "the report's row for `benign-test-mocks` — the corpus's case whose whole \
             subject is that a test double, a worked example or a documentation snippet must not \
             become a `must_fix` production finding — and the observed severity in it against the \
             `note` the manifest requires",
        ),
        covers: vec![
            String::from(
                "\"Tracked explicitly\" as a measurement rather than as a claim: the case is in the \
                 manifest with its own `expected_severity`, the report carries a row for it, the row \
                 carries an observed severity measured by the five scanners, and the numerator is \
                 that observation — not the presence of a sentence in a document.",
            ),
            String::from(
                "The direction this metric is about is the false positive, so the numerator counts a \
                 row whose observed severity is not heavier than the contract asks. The corpus's other \
                 benign-pattern fixtures are outside this corpus's manifest and outside this number.",
            ),
        ],
        does_not_cover,
    }
}

/// The default user report's jargon golden tests.
///
/// Unmeasurable here, and said so rather than printed as a zero.
fn jargon_goldens() -> Metric {
    Metric {
        claim: String::from("default user report passes jargon golden tests"),
        value: MetricValue::Unmeasured {
            reason: String::from(
                "No jargon golden test exists. `docs/testing/GOLDEN_REPORTS.md` asks for golden tests \
                 of eight reports and `crates/sure-cli/tests/golden_reports.rs` holds seven of them, \
                 every one rendered with `HumanReportSettings::default()` — the default user report — \
                 but they assert the phrases a report must contain, and no test in this tree asserts \
                 a jargon word list over a whole rendered report. What exists instead is per-string \
                 assertions in the modules that produce the strings: \
                 `sure_domain::status`'s `agg_severities_have_plain_language_headlines_free_of_jargon`, \
                 `sure_domain::capability`'s `blind_spot_explanations_avoid_technical_jargon`, \
                 `sure_domain::evidence`'s `stale_caveats_avoid_leading_jargon`, \
                 `sure_core::protection_history`, `sure_core::hook_protection`, \
                 `sure_core::config`'s settings sentences and `sure_core::aggregation`'s labels. This \
                 document observes none of them: it reads a corpus of fixtures and never renders a \
                 user report.",
            ),
            would_require: String::from(
                "a golden test that renders the default user report for a fixture project — the whole \
                 document, not a list of phrases it should contain — and asserts it against a \
                 checked-in expectation that a reviewer updates deliberately, which is the shape \
                 `docs/testing/GOLDEN_REPORTS.md` describes; and a way for this document to see that \
                 test's result rather than inferring it from the test's existence.",
            ),
            measured_elsewhere: Some(String::from(
                "partly, and not under this name: `cargo test -p sure-cli --test golden_reports` runs \
                 the seven report goldens (all rendered with the default settings), and the per-string \
                 jargon assertions named above run inside `cargo test --workspace`. Neither is a \
                 golden test of the default user report against a jargon word list, which is what this \
                 line states.",
            )),
        },
        computed_from: String::from(
            "nothing: this document renders no user report and holds no golden expectation",
        ),
        covers: vec![String::from(
            "Nothing. This entry exists so that the line is not silently absent from a document a \
             packaging task reads, and so that a reader can see what does exist in its place.",
        )],
        does_not_cover: vec![
            String::from(
                "Every rendered report, in every form. A jargon regression in the terminal report, the \
                 Markdown report, the HTML report or the JSON report is caught by whatever test was \
                 written for the sentence that changed, and by the phrase assertions in \
                 `golden_reports.rs`, and by nothing this document can see.",
            ),
            String::from(
                "It is not part of this document's decision either: the decision is about the \
                 acceptance corpus's release-blocking cases, and this entry is named in \
                 `unmeasured_metrics` so that a `permitted` decision cannot be read as covering it. \
                 The acceptance corpus's `cannot_confirm` row and its null-severity rows are outside \
                 this metric entirely, because it has no rows at all.",
            ),
        ],
    }
}

/// The severity a row carries, whatever it is graded on, or `None` where it
/// carries none.
fn observed_severity(row: &CaseRow) -> Option<&str> {
    match &row.observed {
        Observation::Observed { severity, .. } => severity.as_deref(),
        Observation::CannotConfirm { .. } => None,
    }
}

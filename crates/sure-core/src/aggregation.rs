//! What a run's checks add up to, with every way a critical check can fail to
//! pass visible as itself.
//!
//! `P4-T009`'s acceptance is two sentences: *"Critical skipped/error/unknown is
//! visible."* and *"False-green unit tests exist."* The first is a claim about
//! what a report built from a run can say. The second is a claim about what this
//! module's own tests have to prove, and it is here rather than in a note
//! because it is the reason several of them are shaped the way they are.
//!
//! # The rule was frozen before this module existed, and none of it is restated
//!
//! [`aggregate`](sure_domain::status::aggregate) is the only aggregation entry
//! point — `docs/adr/0010-frozen-domain-semantics-in-code.md` says so, and
//! `docs/architecture/FROZEN_SEMANTICS.md` holds its five rules — and **this
//! module calls it and reimplements no part of it**. There is no question about
//! a failure, a skip or a warning asked anywhere below: the severity, the
//! blocking list and the coverage counts are the frozen function's own answers,
//! carried out unchanged. `the_aggregation_does_not_restate_the_frozen_rule` and
//! `nothing_but_the_frozen_rule_builds_a_verdict` in `tests/aggregation.rs` are
//! the rules that keep that true of the shipped source rather than of the prose
//! above it — the second over the whole workspace, because the first is only as
//! good as there being nowhere else for a verdict to come from.
//!
//! # Two things the frozen rule cannot see, and both of them are false greens
//!
//! `aggregate` answers one question about the slice it is handed: given these
//! results, what may be claimed. What it cannot see is anything about **the
//! set** rather than about a member of it, and both gaps are of that shape.
//!
//! **A check the plan named for which no result came back.** The frozen function
//! sees a shorter list and has no way to know a row is missing — and the same
//! list without its failing check is not an incomplete run, it is a different
//! and greener one. This is the false green a composition layer exists to close,
//! and it cannot be closed inside a function whose input *is* the results.
//!
//! **Which kind of not-checked.** [`CoverageSummary`] carries
//! `critical_not_checked`, and that one list holds two states a reader acts on
//! differently: [`CheckStatus::Skipped`](sure_domain::status::CheckStatus::Skipped),
//! a check that was deliberately not run, and
//! [`CheckStatus::Unknown`](sure_domain::status::CheckStatus::Unknown), a check
//! that ran and established nothing.
//! *"I was not allowed to look"* and *"I looked and could not tell"* are
//! different promises to somebody deciding whether to hand a project over, and a
//! report built from that list cannot tell them apart.
//!
//! The vocabulary for the distinction already exists — [`CriticalState`] is a
//! five-way classification the domain froze on the wire — and until this module
//! it had **no consumer in this crate at all**. [`CriticalState::from_status`]
//! classifies every row here, so the three states the acceptance names are the
//! domain's own three and not a second set invented beside them.
//!
//! # The plan decides the run, and it arrives in one piece
//!
//! [`CheckSchedule`] is the authority on what the run consists of. It decides
//! the order rows appear in — the order `P4-T001` built the plan in, which is the
//! order every other report in this crate shows — and it is where a result for a
//! check that did not run comes from: [`ScheduledCheck::not_run`], which by its
//! own documentation can produce a skipped result for a check and *cannot*
//! produce any other status for one. `P4-T001` wrote that function for exactly
//! this caller. This is the caller.
//!
//! # The checks the plan cannot hold travel with it
//!
//! A plan cannot hold every check a project declares. `"test": ["jest"]` names
//! something SURE cannot run, so the node checks produce a
//! [`MissingCommand`] instead of a proposal and the runner is never handed it.
//! **That is not a check with no answer.** It has one, and the declaration is
//! the only thing that can produce it: no runner will ever report on a check it
//! was not given, so a run that reads only its schedule is silent about it, and
//! *silence* is precisely the outcome
//! [`MissingCommand`]'s own documentation says the type exists to prevent.
//!
//! So the declarations arrive as their own argument and become rows of the run.
//! **The argument is typed, which is the whole of why this is safe**: a
//! [`MissingCommand`] can only produce [`MissingCommand::not_checked`], a
//! `Skipped` result against the state being aggregated, so there is no way for a
//! caller to declare a check and have it counted as a pass — the property holds
//! by construction rather than by a rule this function has to get right.
//!
//! **And it is what lets the declaration's own reason decide the verdict.** The
//! kind of a declaration is the whole of whether a critical one holds a run out
//! of green — [`MissingKind::NotDeclared`](crate::checks::MissingKind::NotDeclared)
//! is a scope limit and
//! [`MissingKind::NotACommand`](crate::checks::MissingKind::NotACommand) is a
//! defect the project can fix — and that question is asked by
//! [`blocks_green`](CheckResult::blocks_green), inside the frozen function, over
//! the rows it is handed. A row kept out of `complete` is a check whose answer
//! was computed and then dropped, and the sentence that decided it was
//! *"a critical test check that cannot run because of a broken manifest should
//! keep the run out of green"*: a promise no reader of the verdict could then
//! find in it.
//!
//! Two rules follow, and they are the whole of what this module decides:
//!
//! 1. **The aggregate is over the checks the run was made of, and nothing
//!    else.** One entry per check: the plan's checks in plan order, then the
//!    declared checks the plan could not hold, in the order of their ids. A
//!    result for a check that neither of those names is reported by
//!    [`RunReport::unscheduled`] and is not aggregated: it can neither help nor
//!    hurt the verdict, because the verdict is about the run and the run is what
//!    the plan and its declarations said it would be. This is
//!    [`Enforcement`](crate::enforce::Enforcement)'s own rule one level up — a
//!    command for a check that is not in the plan is reported by
//!    [`unscheduled`](crate::enforce::Enforcement::unscheduled) and is not
//!    admitted — and it is also what stops an empty plan plus one stray passing
//!    result from reading green.
//! 2. **The plan's decision is what happened.** A check the plan stopped that
//!    came back with a result claiming it ran is aggregated as the plan's stopped
//!    entry — a skipped result, which cannot be green — and its id is recorded by
//!    [`RunReport::overruled`]. Nothing is repaired and nothing is believed: the
//!    plan is what SURE was allowed to do, and a claim to the contrary is a fact
//!    about the caller rather than about the project.
//!
//! # Deterministic, and that is a property rather than an adjective
//!
//! The report is a function of the plan, the *set* of results and the project
//! state. **The order a caller collected the results in cannot reach the
//! verdict**, which is why the two places it could — the order of the aggregated
//! rows and the order of the reported extras — are both decided here rather than
//! left to a caller's loop. `every_order_of_the_same_results_gives_the_same_
//! report` in `tests/aggregation.rs` feeds a run's results in every order and
//! compares the reports.
//!
//! Nothing here reads a file, a clock, an environment variable or a process.
//! `the_aggregation_reads_nothing_but_its_arguments` is the source rule, and it
//! is there because "deterministic" is easy to write in a doc comment and hard
//! to keep.
//!
//! # Two ways a caller is refused, and neither is repaired
//!
//! [`RunRefused`] holds both.
//!
//! **Two results for one check.** There is no honest rule for choosing between
//! them, and inventing one — first wins, or the least green wins — would be new
//! aggregation semantics living in a composition layer, which is the second place
//! for a rule this repository keeps in one. The check is named and no verdict is
//! produced, which is the safe direction: a refusal is not a green.
//!
//! **A result established against a different project state.** Every
//! [`CheckResult`] names the state it applies to, and a verdict that mixed two
//! states would be reading a pass from an older tree as a pass about this one —
//! the stale pass [`CheckResult`]'s own documentation says both of its required
//! fields exist to prevent.
//!
//! [`CoverageSummary`]: sure_domain::status::CoverageSummary

use std::collections::BTreeMap;
use std::fmt;

use sure_domain::evidence::EvidenceClass;
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::status::{Aggregate, CheckResult, CriticalState, NotCheckedReason, aggregate};

use crate::checks::MissingCommand;
use crate::schedule::CheckSchedule;

/// The detail shown for a check the plan allowed that reported nothing at all.
///
/// This is this module's sentence rather than the vocabulary's, and the reason
/// is that the vocabulary has no state for it: every [`NotCheckedReason`] is an
/// answer to *why was this not run*, and the honest answer here is that SURE
/// does not know what happened. Inventing a variant of a frozen enum to carry
/// it, or reusing [`NotCheckedReason::UnknownReason`] — whose own sentence is
/// "SURE does not know why this was not checked", about a check that *was* meant
/// to run — would both be worse than one sentence in the module that has the
/// fact.
///
/// **Public because the fixture that grades this path holds what SURE produced
/// against what SURE says, by equality, and the sentence is written down in two
/// places rather than three.** `fixtures/adversarial/check-crash` declares this
/// sentence in the answer it expects, and
/// `crates/sure-core/tests/adversarial_fixture_detection.rs` compares the
/// sentence the report carries against *this* constant as well as against the
/// fixture's copy — so a rewrite here leaves the fixture's copy as the one thing
/// that disagrees, and a rewrite there is reported the same way. A copy in the
/// test would be a third place to keep, and it is the one most easily brought
/// back into agreement by pasting this line into it, which is what would leave
/// the test asserting nothing the product says.
/// `NO_TRUSTED_INTENT_LIMITATION` in `sure_domain::status` is public for the same
/// reason and is quoted whole by `missing-user-intent`.
pub const NOTHING_CAME_BACK: &str = "Nothing was reported for this check, so SURE has no basis for \
                                     a verdict on it.";

/// Aggregate a run's results into a verdict, against the plan the run was made
/// from.
///
/// `schedule` is what the run intended to check and `results` is what came back
/// for it; `missing` is the checks the project declared that no plan entry could
/// be built for, and they are rows of the run exactly as the scheduled checks
/// are — see the module comment above. `project_fingerprint` is the state this
/// verdict is about, and it is required because a check the plan stopped has no
/// result of its own and one has to be made for it; a declared check has no
/// result of its own for the same reason and is made one the same way.
///
/// # `missing` is the one argument a caller can pass empty and be wrong about
///
/// **A recorded risk, left standing deliberately** (`P18-T012` measured it and
/// decided against a guard). One product caller passes a non-empty slice —
/// `pipeline.rs`, which passes the declarations the same walk produced — and
/// every other caller in the tree passes `&[]`. An empty slice makes this
/// function's second loop a no-op, so a caller that *had* declarations and passed
/// `&[]` would produce a report that is silent about checks the project declared,
/// which is the false green the module comment above is about.
///
/// What is not done about it, and why: the type cannot tell the two cases apart —
/// `&[]` means *no declarations* and *declarations I forgot* with one value — and
/// a guard would need a second parameter that only a real second caller could
/// justify. **No such caller exists, and inventing one to hang a test on would be
/// a test with no product behind it.** What holds the property instead is the
/// measurement rather than the type: `crates/sure-core/tests/declared_commands.rs`
/// drives the real `sure check` pipeline over a project with a broken manifest and
/// asserts the declaration's row is a `Skipped` that blocks green and is counted —
/// and `P18-T012` proved that test red by deleting this function's second loop,
/// which is the mutation a guard would have had to catch.
///
/// # Errors
/// Returns [`RunRefused`] when the rows cannot be aggregated honestly: two
/// answers for one check — two results, or a result and a declaration, or two
/// declarations — or an answer established against a different project state.
/// Both are described on [`RunRefused`], and neither is repaired.
#[must_use = "a run's verdict is the whole point of aggregating it"]
pub fn aggregate_run(
    schedule: &CheckSchedule,
    results: &[CheckResult],
    missing: &[MissingCommand],
    project_fingerprint: &FingerprintId,
) -> Result<RunReport, RunRefused> {
    let mut reported: BTreeMap<CheckId, &CheckResult> = BTreeMap::new();
    for result in results {
        // Before anything is composed, so that a refusal is a statement about
        // the caller's set and not about a half-built verdict.
        if result.project_fingerprint != *project_fingerprint {
            return Err(RunRefused::ADifferentProjectState {
                id: result.id.clone(),
                found: result.project_fingerprint.clone(),
            });
        }
        if reported.insert(result.id.clone(), result).is_some() {
            return Err(RunRefused::TwoAnswersForOneCheck {
                id: result.id.clone(),
            });
        }
    }

    // A declaration is an answer to the same question a result is, so it is
    // refused on the same terms: a check answered twice has no one answer to
    // aggregate, whichever two of the three lists the two answers came from.
    let mut declared: BTreeMap<CheckId, &MissingCommand> = BTreeMap::new();
    for declaration in missing {
        let id = declaration.id().clone();
        if declared.insert(id.clone(), declaration).is_some() || reported.contains_key(&id) {
            return Err(RunRefused::TwoAnswersForOneCheck { id });
        }
    }

    let mut complete: Vec<CheckResult> =
        Vec::with_capacity(schedule.len().saturating_add(declared.len()));
    let mut unreported: Vec<CheckId> = Vec::new();
    let mut overruled: Vec<CheckId> = Vec::new();

    // Rule one: one entry per scheduled check, in plan order. The schedule is
    // walked rather than the results, so a check with no result is a row here
    // and not an absence — which is the whole of what the frozen function
    // cannot do for itself.
    for scheduled in schedule.checks() {
        let id = scheduled.proposal().id().clone();
        // A check cannot have been both proposed and declared missing, and a
        // caller that says both has supplied two answers for one check rather
        // than a plan this function could read.
        if declared.contains_key(&id) {
            return Err(RunRefused::TwoAnswersForOneCheck { id });
        }
        let theirs = reported.remove(&id);
        let stopped = scheduled.not_run(project_fingerprint);

        let result = match (stopped, theirs) {
            // The plan stopped it and nothing was claimed about it.
            (Some(stopped), None) => stopped,
            // The plan stopped it and something was claimed anyway. Rule two:
            // the plan's own entry is what is aggregated, and a claim that the
            // check *ran* is recorded. A caller that hands back the stopped
            // entry `Enforcement` produced is the ordinary case and is not
            // recorded, which is why the test is `produced_a_result` rather
            // than the mere presence of a result.
            (Some(stopped), Some(theirs)) => {
                if theirs.status.produced_a_result() {
                    overruled.push(id);
                }
                stopped
            }
            // It ran, and this is what it found.
            (None, Some(theirs)) => theirs.clone(),
            // The plan said it would run and nothing came back. SURE does not
            // know what happened, which is `Unknown` and is not `Skipped`: a
            // skip is a decision somebody made.
            (None, None) => {
                unreported.push(id);
                nothing_came_back(scheduled, project_fingerprint)
            }
        };
        complete.push(result);
    }

    // And then the checks no plan entry could be built for. The order is the
    // declarations' ids and not the caller's order, for the reason the
    // `unscheduled` list's is: the report is a function of the set, not of the
    // sequence somebody happened to collect it in.
    for declaration in declared.values() {
        complete.push(declaration.not_checked(project_fingerprint));
    }

    // Whatever is left in `reported` names a check the plan never proposed. It
    // is reported and dropped; `BTreeMap` iteration is what makes the order of
    // that report a function of the ids rather than of the caller.
    let unscheduled: Vec<CheckId> = reported.keys().cloned().collect();

    Ok(RunReport {
        aggregate: aggregate(&complete),
        critical: critical_checks(&complete),
        unreported,
        overruled,
        unscheduled,
        results: complete,
    })
}

/// The critical checks in a set of results, one row each, in the order given.
///
/// This is the pure part of the module and it is public for the reason every
/// pure part is: it is what the unit tests below can reach without a plan, and a
/// caller that already holds results can classify them without one.
///
/// **It classifies what it is handed and says nothing about the set.** A missing
/// row is invisible here exactly as it is in the frozen function, so this is not
/// a verdict and must not be read as one — [`aggregate_run`] is the function
/// that knows the plan, and it is the one that knows a check reported nothing.
#[must_use]
pub fn critical_checks(results: &[CheckResult]) -> Vec<CriticalCheck> {
    results
        .iter()
        .filter(|result| result.critical)
        .map(|result| CriticalCheck {
            id: result.id.clone(),
            title: result.title.clone(),
            state: result.critical_state(),
            blocks: result.blocks_green(),
            reason: result.not_checked_reason,
            detail: detail_of(result),
        })
        .collect()
}

/// What a row says about why the check is not a pass.
///
/// A check that did not run is shown the vocabulary's own sentence for its own
/// reason, because that sentence is the promise the product made and this module
/// has no better one. Everything else — a checker that failed, a check with no
/// basis for a verdict, a failure — is shown the detail the result itself
/// carries, which is the caller's text and the only text there is about it.
fn detail_of(result: &CheckResult) -> String {
    if let Some(reason) = result.not_checked_reason {
        return reason.plain_explanation().to_owned();
    }
    result.reason.clone()
}

/// The result for a check the plan would have run that reported nothing.
fn nothing_came_back(
    scheduled: &crate::schedule::ScheduledCheck,
    project_fingerprint: &FingerprintId,
) -> CheckResult {
    let proposal = scheduled.proposal();
    CheckResult::unknown(
        proposal.id().clone(),
        proposal.title(),
        proposal.severity(),
        proposal.critical(),
        EvidenceClass::Unknown,
        project_fingerprint.clone(),
    )
    .with_reason(NOTHING_CAME_BACK)
}

/// A run's verdict, and every way a critical check contributed to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    aggregate: Aggregate,
    critical: Vec<CriticalCheck>,
    unreported: Vec<CheckId>,
    overruled: Vec<CheckId>,
    unscheduled: Vec<CheckId>,
    /// Every check the run was made of and what became of it: the plan's checks
    /// in plan order, then the declared checks no plan entry could be built for,
    /// in the order of their ids.
    ///
    /// Stored so that downstream summaries can join each scheduled check back to
    /// its result without recomputing the plan's decisions.
    results: Vec<CheckResult>,
}

impl RunReport {
    /// The frozen aggregate, exactly as [`aggregate`](sure_domain::status::aggregate)
    /// produced it.
    ///
    /// Handed back rather than summarised, because it holds four answers this
    /// type does not repeat — the severity, the plain headline, the blocking
    /// list and the coverage counts — and a copy of any of them here would be a
    /// second place for it to be written down wrong.
    #[must_use]
    pub const fn aggregate(&self) -> &Aggregate {
        &self.aggregate
    }

    /// Whether the run ended with nothing blocking.
    ///
    /// The frozen severity's own answer, asked of it rather than recomputed.
    #[must_use]
    pub fn is_green(&self) -> bool {
        self.aggregate.is_green()
    }

    /// Every critical check, one row each, in plan order.
    #[must_use]
    pub fn critical(&self) -> &[CriticalCheck] {
        &self.critical
    }

    /// The critical checks that stop a green verdict.
    pub fn blocking(&self) -> impl Iterator<Item = &CriticalCheck> {
        self.critical.iter().filter(|check| check.blocks)
    }

    /// The critical checks that were deliberately not run.
    ///
    /// One of the three states the acceptance names, and one of the two the
    /// frozen coverage summary merges into a single list.
    pub fn skipped(&self) -> impl Iterator<Item = &CriticalCheck> {
        self.in_state(CriticalState::NotRun)
    }

    /// The critical checks whose checker failed.
    ///
    /// The third state, and the one that means *SURE does not know* rather than
    /// *the project is fine*.
    pub fn errored(&self) -> impl Iterator<Item = &CriticalCheck> {
        self.in_state(CriticalState::CheckerError)
    }

    /// The critical checks SURE has no basis for a verdict on.
    pub fn unknown(&self) -> impl Iterator<Item = &CriticalCheck> {
        self.in_state(CriticalState::Uncertain)
    }

    /// The critical checks in one state.
    ///
    /// **A filter over the one list rather than a list per state**, so that the
    /// rows and the counts cannot disagree: a check that is in
    /// [`Self::critical`] is counted here if its state matches, and there is no
    /// second collection for it to be missing from.
    pub fn in_state(&self, state: CriticalState) -> impl Iterator<Item = &CriticalCheck> {
        self.critical
            .iter()
            .filter(move |check| check.state == state)
    }

    /// Checks the plan would have run that reported nothing at all.
    ///
    /// Each one is aggregated as
    /// [`CheckStatus::Unknown`](sure_domain::status::CheckStatus::Unknown) —
    /// SURE does not know
    /// what happened to it — and this is where a caller finds out that its own
    /// collection loop dropped one. Empty is the ordinary case.
    #[must_use]
    pub fn unreported(&self) -> &[CheckId] {
        &self.unreported
    }

    /// Checks the plan stopped that came back with a result claiming they ran.
    ///
    /// Aggregated as the plan's own stopped entry, never as the claim. Empty is
    /// the ordinary case: the caller that hands back the skipped results
    /// [`Enforcement`](crate::enforce::Enforcement) produced is not recorded
    /// here.
    #[must_use]
    pub fn overruled(&self) -> &[CheckId] {
        &self.overruled
    }

    /// Results for checks the plan never proposed and no declaration accounts
    /// for.
    ///
    /// **Reported and not aggregated**, in both directions: a stray pass cannot
    /// make a run green and a stray failure cannot make it red. The run is what
    /// the plan said it would be.
    ///
    /// A check the project *declared* and no plan entry could be built for is
    /// not stray: its declaration is handed to [`aggregate_run`] and it is a row
    /// of [`Self::results`], so it is neither aggregated as a result nor listed
    /// here. What is listed here is a caller's answer about a check nobody asked
    /// about.
    #[must_use]
    pub fn unscheduled(&self) -> &[CheckId] {
        &self.unscheduled
    }

    /// One line per critical check, in plan order, for a report.
    #[must_use]
    pub fn plain_description(&self) -> Vec<String> {
        self.critical
            .iter()
            .map(CriticalCheck::plain_description)
            .collect()
    }

    /// Every check the run was made of and what became of it.
    ///
    /// This is the same set the aggregate was computed over, so a caller can join
    /// each check back to its result without trusting a second list — and the
    /// declared checks that no plan entry could be built for are in it, because
    /// they are rows the verdict was computed over too. A caller that walks the
    /// schedule alone will not find them: their ids were never proposed.
    #[must_use]
    pub fn results(&self) -> &[CheckResult] {
        &self.results
    }
}

/// One critical check and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriticalCheck {
    id: CheckId,
    title: String,
    state: CriticalState,
    blocks: bool,
    reason: Option<NotCheckedReason>,
    detail: String,
}

impl CriticalCheck {
    /// The check's identity.
    #[must_use]
    pub const fn id(&self) -> &CheckId {
        &self.id
    }

    /// The check's short human title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// How this check contributed, in the domain's own five-way vocabulary.
    ///
    /// Read from [`CriticalState::from_status`] and never matched on
    /// [`CheckStatus`](sure_domain::status::CheckStatus) here, so the
    /// classification has one definition and it is the frozen one.
    #[must_use]
    pub const fn state(&self) -> CriticalState {
        self.state
    }

    /// Whether this check stops a green verdict.
    ///
    /// [`CheckResult::blocks_green`]'s answer, which is not the same question as
    /// [`CriticalState::blocks_green`]: a critical check that was honestly out of
    /// scope for this project is `NotRun` and does not block. Both are true and
    /// they are about different things, so this carries the result's answer and
    /// [`Self::state`] carries the classification.
    #[must_use]
    pub const fn blocks(&self) -> bool {
        self.blocks
    }

    /// Why the check did not run, when it did not run.
    #[must_use]
    pub const fn reason(&self) -> Option<NotCheckedReason> {
        self.reason
    }

    /// The detail line, in plain language.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// The sentence a report shows for this check.
    #[must_use]
    pub fn plain_description(&self) -> String {
        let state = state_label(self.state);
        if self.detail.is_empty() {
            return format!("{} - {state}", self.title);
        }
        format!("{} - {state}: {}", self.title, self.detail)
    }
}

/// The short phrase a report shows for a state.
///
/// Written out here rather than asked of the domain because [`CriticalState`]
/// carries a wire name and not a sentence, and this module owns the sentence.
/// **The match has no wildcard arm**, so a sixth state cannot be added to the
/// frozen enum without this file failing to compile — which is the only way a
/// classification that grows quietly is caught. `every_state_has_its_own_label`
/// below holds the other half, that no two states share a phrase.
const fn state_label(state: CriticalState) -> &'static str {
    match state {
        CriticalState::Passed => "passed",
        CriticalState::Failed => "failed",
        CriticalState::NotRun => "did not run",
        CriticalState::CheckerError => "SURE's own check failed",
        CriticalState::Uncertain => "could not be determined",
    }
}

/// A refusal to aggregate a run.
///
/// Two ways to hand in a set of rows that no verdict can honestly be built
/// from, each refused rather than repaired. **Repairing either would be the
/// false green the rest of this module exists to prevent**: choosing between two
/// answers to one question makes the verdict depend on a rule nobody wrote down,
/// and accepting a result from another project state reads a pass about an older
/// tree as a pass about this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunRefused {
    /// Two or more rows carry the same check's identity.
    ///
    /// A "row" is a result or a declared check, so this is refused for any pair
    /// of them rather than for two results alone: the question is *how many
    /// answers are there to this check*, and two answers have no one answer to
    /// aggregate whatever shape they arrived in.
    TwoAnswersForOneCheck {
        /// The check that was answered more than once.
        id: CheckId,
    },
    /// A result names a project state other than the one being aggregated.
    ADifferentProjectState {
        /// The check whose result is from somewhere else.
        id: CheckId,
        /// The state the result was established against.
        found: FingerprintId,
    },
}

impl fmt::Display for RunRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TwoAnswersForOneCheck { id } => write!(
                f,
                "the check {id} was answered more than once, so there is no one answer to \
                 aggregate"
            ),
            Self::ADifferentProjectState { id, found } => write!(
                f,
                "the result for {id} was established against project state {found}, which is not \
                 the state this run is about"
            ),
        }
    }
}

impl std::error::Error for RunRefused {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::severity::Severity;
    use sure_domain::status::CheckStatus;

    /// The statuses a critical check can carry and still block a green verdict.
    ///
    /// Written out rather than derived, because this list *is* the claim being
    /// tested: a rule computed from the same functions under test would agree
    /// with them by construction and would keep agreeing after either changed.
    const BLOCKING: &[CheckStatus] = &[
        CheckStatus::Fail,
        CheckStatus::Skipped,
        CheckStatus::Error,
        CheckStatus::Unknown,
    ];

    fn a_result(status: CheckStatus, critical: bool, fingerprint: &FingerprintId) -> CheckResult {
        let id = CheckId::generate();
        let title = "a check";
        let severity = Severity::ShouldFixFirst;
        let class = EvidenceClass::DeterministicCheck;
        let fingerprint = fingerprint.clone();
        match status {
            CheckStatus::Pass => {
                CheckResult::pass(id, title, severity, critical, class, fingerprint)
            }
            CheckStatus::Fail => {
                CheckResult::fail(id, title, severity, critical, class, fingerprint)
            }
            CheckStatus::Warning => {
                CheckResult::warning(id, title, severity, critical, class, fingerprint)
            }
            CheckStatus::Skipped => CheckResult::not_run(
                id,
                title,
                severity,
                critical,
                NotCheckedReason::ExecutionNotAuthorized,
                fingerprint,
            ),
            CheckStatus::Error => CheckResult::errored(
                id,
                title,
                severity,
                critical,
                "the check itself could not finish",
                fingerprint,
            ),
            CheckStatus::Unknown => {
                CheckResult::unknown(id, title, severity, critical, class, fingerprint)
            }
        }
    }

    #[test]
    fn every_state_has_its_own_label() {
        let labels: Vec<&str> = CriticalState::ALL
            .iter()
            .copied()
            .map(state_label)
            .collect();
        for (index, state) in CriticalState::ALL.iter().enumerate() {
            let label = labels[index];
            assert!(
                !label.trim().is_empty(),
                "{state:?} has no sentence to show, so a report would carry an empty row"
            );
            assert!(
                !labels[..index].contains(&label),
                "{state:?} is shown as {label:?} and so is an earlier state, so a report \
                 cannot tell the two apart"
            );
        }
        assert_eq!(
            labels.len(),
            CriticalState::ALL.len(),
            "the labels and the vocabulary have drifted apart"
        );
    }

    #[test]
    fn the_state_of_a_critical_check_is_the_domains_own_classification() {
        // The classification is `CriticalState::from_status`'s and is asked of
        // it rather than restated here: a match of this module's own would be a
        // second definition of the rule the domain froze.
        let fingerprint = FingerprintId::generate();
        for &status in CheckStatus::ALL {
            let result = a_result(status, true, &fingerprint);
            let checks = critical_checks(std::slice::from_ref(&result));
            assert_eq!(
                checks[0].state(),
                CriticalState::from_status(status),
                "{} was classified by something other than the frozen rule",
                status.as_str()
            );
            assert_eq!(
                checks[0].state(),
                result.critical_state(),
                "{} was classified differently from the result it came from",
                status.as_str()
            );
        }
    }

    #[test]
    fn only_a_critical_check_becomes_a_row() {
        let fingerprint = FingerprintId::generate();
        let results = vec![
            a_result(CheckStatus::Fail, false, &fingerprint),
            a_result(CheckStatus::Pass, true, &fingerprint),
            a_result(CheckStatus::Pass, false, &fingerprint),
        ];
        let checks = critical_checks(&results);
        assert_eq!(checks.len(), 1, "one critical check, one row");
        assert_eq!(checks[0].state(), CriticalState::Passed);
        assert!(!checks[0].blocks());
    }

    #[test]
    fn a_non_critical_check_never_becomes_a_row_however_it_went() {
        // Read as the complement of the test above, over the whole vocabulary:
        // the word "critical" is doing the work, and a check that is not one is
        // not a check a report weighs.
        let fingerprint = FingerprintId::generate();
        for &status in CheckStatus::ALL {
            let checks = critical_checks(&[a_result(status, false, &fingerprint)]);
            assert!(
                checks.is_empty(),
                "{} on a check that is not critical became a row, so a report would weigh \
                 something the plan did not",
                status.as_str()
            );
        }
    }

    #[test]
    fn a_critical_check_that_did_not_pass_blocks_and_a_warning_does_not() {
        // The false-green rule over the whole vocabulary. `Warning` is the
        // interesting row: the domain classifies it as `Passed` — it cannot
        // block — and it still degrades the run, which is `aggregate`'s answer
        // and not this module's.
        let fingerprint = FingerprintId::generate();
        for &status in CheckStatus::ALL {
            let checks = critical_checks(&[a_result(status, true, &fingerprint)]);
            assert_eq!(
                checks[0].blocks(),
                BLOCKING.contains(&status),
                "{} on a critical check has the wrong blocking answer",
                status.as_str()
            );
            // The three states the acceptance names are exactly the statuses
            // that produced no result about the project, and that is the whole
            // reason a report has to name them separately: `not_checked` is not
            // a shade of green and it is not one thing either.
            let among_the_three = matches!(
                checks[0].state(),
                CriticalState::NotRun | CriticalState::CheckerError | CriticalState::Uncertain
            );
            assert_eq!(
                among_the_three,
                !status.produced_a_result(),
                "{} is treated as checked through a rule that is not `produced_a_result`",
                status.as_str()
            );
        }
    }

    #[test]
    fn a_check_that_did_not_run_is_shown_the_sentence_the_user_was_promised() {
        // Every reason, because the mapping is the vocabulary's and a report
        // that paraphrased one of them would be a promise made in a different
        // wording from the one the product froze.
        let fingerprint = FingerprintId::generate();
        for &reason in NotCheckedReason::ALL {
            let result = CheckResult::not_run(
                CheckId::generate(),
                "run the tests",
                Severity::MustFix,
                true,
                reason,
                fingerprint.clone(),
            );
            let checks = critical_checks(std::slice::from_ref(&result));
            assert_eq!(checks[0].reason(), Some(reason));
            assert_eq!(checks[0].detail(), reason.plain_explanation());
            assert_eq!(checks[0].state(), CriticalState::NotRun);
            assert_eq!(
                checks[0].plain_description(),
                format!(
                    "run the tests - did not run: {}",
                    reason.plain_explanation()
                )
            );
            assert_eq!(
                checks[0].blocks(),
                !reason.is_scope_limit(),
                "{reason:?} is a row in the same state either way, and only the result's own \
                 rule decides whether it stops the verdict"
            );
        }
    }

    #[test]
    fn the_reason_a_result_carries_does_not_replace_the_sentence_for_a_check_that_did_not_run() {
        // `CheckResult::not_run` fills `reason` from the explanation and a caller
        // may overwrite it with `with_reason`, so the two are set independently
        // and nothing keeps them equal. `browser::Probe::verdict` is a shipped
        // caller that overwrites it, and `probe`'s are three more, which is why
        // the row reads the reason rather than the field: the sentence in front
        // of a user for a check that did not run is the vocabulary's, and a
        // caller's own wording is not a second way to say it.
        let fingerprint = FingerprintId::generate();
        let reason = NotCheckedReason::ExecutionNotAuthorized;
        let result = CheckResult::not_run(
            CheckId::generate(),
            "run the tests",
            Severity::MustFix,
            true,
            reason,
            fingerprint,
        )
        .with_reason("`cargo test` exited 101 after 3 seconds");
        assert_ne!(
            result.reason,
            reason.plain_explanation(),
            "the fixture has to disagree with the explanation, or it proves nothing about \
             which of the two the row reads"
        );

        let checks = critical_checks(std::slice::from_ref(&result));
        assert_eq!(checks[0].reason(), Some(reason));
        assert_eq!(
            checks[0].detail(),
            reason.plain_explanation(),
            "the row is showing the words the result carries for a check that did not run"
        );
        assert_eq!(checks[0].state(), CriticalState::NotRun);
    }

    #[test]
    fn a_checkers_failure_is_its_own_state_and_carries_the_checkers_own_words() {
        let fingerprint = FingerprintId::generate();
        let checks = critical_checks(&[a_result(CheckStatus::Error, true, &fingerprint)]);
        assert_eq!(checks[0].state(), CriticalState::CheckerError);
        assert!(checks[0].blocks());
        assert_eq!(
            checks[0].plain_description(),
            "a check - SURE's own check failed: the check itself could not finish"
        );
        assert_eq!(checks[0].reason(), None, "an error is not a skip");
    }

    #[test]
    fn a_check_with_no_verdict_is_not_a_check_that_did_not_run() {
        // The sentence the acceptance turns on: `skipped` and `unknown` are two
        // rows and two sentences, not one list under one name.
        let fingerprint = FingerprintId::generate();
        let skipped = critical_checks(&[a_result(CheckStatus::Skipped, true, &fingerprint)]);
        let unknown = critical_checks(&[a_result(CheckStatus::Unknown, true, &fingerprint)]);
        assert_eq!(skipped[0].state(), CriticalState::NotRun);
        assert_eq!(unknown[0].state(), CriticalState::Uncertain);
        assert_ne!(skipped[0].state(), unknown[0].state());
        assert_ne!(
            skipped[0].plain_description(),
            unknown[0].plain_description()
        );
        assert!(skipped[0].blocks() && unknown[0].blocks());
    }

    #[test]
    fn the_labels_are_plain_language_and_free_of_the_words_of_the_code() {
        // The labels are read by somebody who did not write this code, so the
        // words a programmer would reach for are the words they must not use.
        for &state in CriticalState::ALL {
            let label = state_label(state);
            for word in ["CheckStatus", "CriticalState", "enum", "snake_case", "fn "] {
                assert!(
                    !label.contains(word),
                    "{state:?}'s label {label:?} carries the implementation word {word:?}"
                );
            }
        }
    }

    #[test]
    fn each_refusal_says_which_refusal_it_is() {
        // A refusal reaches a caller as a `Display` and nothing else carries the
        // difference, so the sentence *is* the refusal as far as a reader is
        // concerned. Each one has to name the fact that made it a refusal, and
        // neither may name the other's fact: a state refusal that read as a
        // second answer would send a caller looking for a duplicate that is not
        // there. The duplicate's own phrase is asserted positively as well as
        // absent from the other, because an assertion that only says "not this
        // phrase" stops testing anything the moment a rewording drops the phrase.
        let id = CheckId::generate();
        let found = FingerprintId::generate();
        let answered_twice = RunRefused::TwoAnswersForOneCheck { id: id.clone() }.to_string();
        let from_elsewhere = RunRefused::ADifferentProjectState {
            id: id.clone(),
            found: found.clone(),
        }
        .to_string();

        assert_ne!(
            answered_twice, from_elsewhere,
            "two refusals that read the same are one refusal to whoever has to act on it"
        );
        assert!(
            answered_twice.contains(&id.to_string()) && answered_twice.contains("more than once"),
            "a duplicate is a check that was answered twice, and the sentence has to say which \
             check and say what happened: {answered_twice}"
        );
        assert!(
            from_elsewhere.contains(&id.to_string()) && from_elsewhere.contains(&found.to_string()),
            "a result from another state is refused for naming that state, so the sentence has to \
             name the state and the check it belongs to: {from_elsewhere}"
        );
        assert!(
            !from_elsewhere.contains("more than once"),
            "the state refusal reads as a duplicate answer, so it names a problem the caller \
             does not have: {from_elsewhere}"
        );
    }
}

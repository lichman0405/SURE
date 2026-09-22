//! `P4-T009`'s acceptance, checked rather than asserted.
//!
//! The task's two sentences are:
//!
//! > *Critical skipped/error/unknown is visible.*
//! >
//! > *False-green unit tests exist.*
//!
//! The first is a claim about what a report built from a run can say, and it is
//! checked here from outside the crate — which is also what proves the seam is
//! usable by a caller rather than only by the module's own tests. The second is a
//! claim about what the tests themselves have to prove, so the interesting tests
//! below are not the ones showing that a skipped check produces a non-green
//! verdict. They are the ones that make the claim **unavoidable**: a sweep over
//! the whole [`CheckStatus`] vocabulary rather than over the cases somebody
//! thought of, an equivalence rather than an implication at each point, and a run
//! of nothing but passing critical checks that **is** green — because an
//! aggregator that answered `not_enough_checked` to everything would satisfy
//! every other test in this file.
//!
//! # Six rules
//!
//! **One: the acceptance sentence, over a run that holds one of each.** A plan of
//! four checks — one the plan stops, one whose checker fails, one nothing is
//! reported for, and one that passes — read back by name through
//! [`RunReport::skipped`], [`RunReport::errored`] and [`RunReport::unknown`].
//!
//! **Two: nothing that did not pass comes out green, and something that did
//! can.** The sweep over the vocabulary, in both directions, and the row that
//! went missing.
//!
//! **Three: the plan decides, and a result cannot argue with it.** A result for a
//! check the plan stopped is aggregated as the plan's own stopped entry; a result
//! for a check the plan never proposed is reported and changes nothing, in either
//! direction.
//!
//! **Four: the same results give the same report whatever order they arrive in.**
//! Every permutation of a five-result run, compared.
//!
//! **Five: two ways a caller is refused rather than repaired**, because a chosen
//! answer would be a rule nobody wrote down.
//!
//! **Six: the source rules.** The module reads nothing but its arguments, it does
//! not restate the frozen aggregation, and exactly one file in the workspace
//! builds a verdict.
//!
//! # What is not claimed here
//!
//! **None of this says the run was worth making.** The aggregation faithfully
//! reports that a check passed; whether the check was a good one is not a
//! question this file can reach. What it holds is that the verdict cannot be
//! greener than the results support in the two ways a *set* of results can be
//! wrong rather than any member of it: a row missing from it, and a row in it
//! that the plan never asked for.
//!
//! # A note on the fixture
//!
//! The plan below is built through [`PlanBuilder`], which is `P4-T001`'s, rather
//! than by assembling a schedule by hand — so this file also checks that the two
//! modules fit together, which is what a composition layer's tests are for. The
//! checks are chosen so that one of them needs a permission the inspecting mode
//! does not grant and the other three do not, because a plan in which everything
//! is stopped can only ever show one of the three states the acceptance names.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_core::aggregation::{CriticalCheck, RunRefused, aggregate_run};
use sure_core::planned_work::{CheckOperation, PlannedWork, PrecomputedEvidence};
use sure_core::scan::{ScanOptions, scan};
use sure_core::schedule::{CheckProposal, CheckReason, CheckSchedule, PlanBuilder, ScheduledCheck};
use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::{ActionKind, ExecutionMode, ExecutionPermissions};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{
    AggregateSeverity, CheckResult, CheckStatus, CriticalState, NotCheckedReason,
};

/// The file that owns the frozen aggregation rule.
const THE_FROZEN_RULE: &str = "crates/sure-domain/src/status.rs";

/// The frozen rule's own path as the `crates/` walk spells it.
const THE_FROZEN_RULE_IN_THE_WALK: &str = "src/status.rs";

/// The module under test's path as the same walk spells it.
const THE_AGGREGATION_IN_THE_WALK: &str = "src/aggregation.rs";

/// The field every `Aggregate` has to be given, and so the mark of one being
/// built.
///
/// Four fields, and this is the one that cannot be defaulted, derived or left
/// out: a verdict has to carry the sentence it shows a user, and only the frozen
/// rule and the vocabulary that names the severities have one to hand. A file
/// that writes `headline:` is building a verdict, whatever the function around it
/// is called.
///
/// The field is also the reason this rule is written against the *shipped* part
/// of a file: the vocabulary's own test module assembles an `Aggregate` to test
/// something else, which is what testing an aggregate looks like, and a rule that
/// counted it would be a rule nobody could satisfy.
const THE_MARK_OF_A_VERDICT: &str = "headline:";

/// The words a second aggregation would be written out of.
///
/// The frozen severity's name and a `match` on a status, which is what
/// reimplementing the five rules would look like from the outside.
const A_SECOND_AGGREGATIONS_WORDS: &[&str] = &[
    "AggregateSeverity",
    "match result.status",
    "match check.status",
];

/// The concrete things a module that is meant to be a pure function must not
/// reach for.
const IMPURE: &[&str] = &[
    "std::fs",
    "std::process",
    "std::env",
    "SystemTime",
    "Instant",
    "std::thread",
    "rand::",
];

/// A check identifier built from a fixed body, so a test can name one.
///
/// `CheckId::generate` is random per run, and half the claims below are about
/// *which* check a row is for — so the ids here are written down rather than
/// generated, which is also what lets the permutation test assert an exact
/// equality rather than a resemblance.
fn id(body: &str) -> CheckId {
    CheckId::parse(format!("chk_{body}")).expect("a well-formed check id")
}

/// A builder for a run that may execute the project's code.
fn an_executing_builder() -> PlanBuilder {
    PlanBuilder::new(
        ExecutionMode::HostConfirmed,
        ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        },
    )
}

/// A builder for a run that may not.
///
/// `InspectOnly` and an inspect-only permission set are the pair the product
/// actually uses for the safe mode, so a plan built from this is not a plan that
/// cannot happen.
fn an_inspecting_builder() -> PlanBuilder {
    PlanBuilder::new(
        ExecutionMode::InspectOnly,
        ExecutionPermissions::inspect_only(),
    )
}

/// Hand a check to a builder.
fn propose(
    builder: &mut PlanBuilder,
    id: CheckId,
    title: &str,
    severity: Severity,
    critical: bool,
    action: ActionKind,
) {
    builder
        .propose(work(CheckProposal::new(
            id,
            title,
            severity,
            critical,
            EvidenceClass::DeterministicCheck,
            CheckReason::ProjectWide,
            &[action],
        )))
        .unwrap();
}

/// The plan entry for a proposal, since `P18-T003` a proposal plus its operation.
///
/// **A candidate observation, because that is the whole of what these fixtures
/// establish.** Every check this file builds is built by hand and nothing here
/// starts a process: the value is *nothing has settled this check*, which maps to
/// a warning and never to a pass — the one answer that claims nothing. This file is
/// about what a run's results aggregate to, not about how a check is carried out,
/// so what it supplies beside the proposal is the most conservative thing it can
/// honestly say; `P18-T004` replaces the placeholders on the product's own paths.
fn work(proposal: CheckProposal) -> PlannedWork {
    PlannedWork::new(
        proposal,
        CheckOperation::Precomputed(PrecomputedEvidence::candidate(
            "this fixture builds a plan and observes nothing",
        )),
    )
}

/// The four checks every plan below is built from, in the order they are handed
/// to the builder — which is deliberately not the order the plan comes back in.
///
/// One runs the project's code, so a mode that may not run anything stops it and
/// a mode that may lets it through. The other three read the project, so they run
/// under either mode, and that is what makes it possible for one plan to hold a
/// stopped check beside a check that reported something.
///
/// The one that runs the project's code is a `RunTests` check rather than, say, a
/// network one, because a check the plan stopped is shown the sentence for the
/// reason the schedule records — and that reason is `P4-T001`'s
/// `NotCheckedReason::ExecutionNotAuthorized`, whose words are about running the
/// project's code. This file does not re-word it.
const THE_CHECKS: &[(char, &str, Severity, bool, ActionKind)] = &[
    (
        'c',
        "the declared tests pass",
        Severity::MustFix,
        true,
        ActionKind::RunTests,
    ),
    (
        'g',
        "the declared type check is clean",
        Severity::MustFix,
        true,
        ActionKind::StaticAnalysis,
    ),
    (
        'l',
        "the declared metadata is readable",
        Severity::MustFix,
        true,
        ActionKind::ReadMetadata,
    ),
    (
        's',
        "the lockfile matches the manifest",
        Severity::CanFixLater,
        false,
        ActionKind::ReadFile,
    ),
];

/// The body of the identifier each of [`THE_CHECKS`] is given.
fn body(letter: char) -> String {
    std::iter::repeat_n(letter, 20).collect()
}

/// The identifier of one of [`THE_CHECKS`].
fn the_id(letter: char) -> CheckId {
    id(&body(letter))
}

/// Build a plan out of [`THE_CHECKS`], under whatever a builder allows.
fn a_plan_from(mut builder: PlanBuilder) -> CheckSchedule {
    for &(letter, title, severity, critical, action) in THE_CHECKS {
        propose(
            &mut builder,
            the_id(letter),
            title,
            severity,
            critical,
            action,
        );
    }
    builder.build()
}

/// The plan most of the tests below aggregate against: one check the mode stops,
/// three it does not.
fn a_plan() -> CheckSchedule {
    a_plan_from(an_inspecting_builder())
}

/// The same four checks under a mode that may run all of them.
fn a_plan_that_runs_everything() -> CheckSchedule {
    a_plan_from(an_executing_builder())
}

/// One critical check the plan would run, for the sweep over the vocabulary.
fn a_one_check_plan() -> CheckSchedule {
    let mut builder = an_executing_builder();
    propose(
        &mut builder,
        the_id('c'),
        "the declared tests pass",
        Severity::MustFix,
        true,
        ActionKind::RunTests,
    );
    builder.build()
}

/// A result of `status` for a scheduled check, built the way a run would.
fn a_result_for(
    scheduled: &ScheduledCheck,
    status: CheckStatus,
    fingerprint: &FingerprintId,
) -> CheckResult {
    let proposal = scheduled.proposal();
    a_result_named(
        proposal.id().clone(),
        proposal.title(),
        proposal.severity(),
        proposal.critical(),
        status,
        fingerprint,
    )
}

/// The same, for a check no plan proposed.
fn a_result_named(
    id: CheckId,
    title: &str,
    severity: Severity,
    critical: bool,
    status: CheckStatus,
    fingerprint: &FingerprintId,
) -> CheckResult {
    let class = EvidenceClass::DeterministicCheck;
    let fingerprint = fingerprint.clone();
    match status {
        CheckStatus::Pass => CheckResult::pass(id, title, severity, critical, class, fingerprint),
        CheckStatus::Fail => CheckResult::fail(id, title, severity, critical, class, fingerprint),
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
            "the check could not be completed",
            fingerprint,
        ),
        CheckStatus::Unknown => {
            CheckResult::unknown(id, title, severity, critical, class, fingerprint)
        }
    }
}

/// A passing result for a check the plan never proposed.
///
/// Two of them exist, told apart by their first letter, because the order the
/// extras come back in is a claim as well as their membership: one extra cannot
/// show an order.
fn a_stray_result(letter: char, fingerprint: &FingerprintId) -> CheckResult {
    a_result_named(
        id(&body(letter)),
        "a check this plan never proposed",
        Severity::MustFix,
        true,
        CheckStatus::Pass,
        fingerprint,
    )
}

/// The scheduled check with this identifier.
///
/// A test that lost its subject is a failure and not a skip, so this panics
/// rather than returning an `Option` for the caller to shrug at.
fn entry<'a>(schedule: &'a CheckSchedule, wanted: &CheckId) -> &'a ScheduledCheck {
    schedule
        .get(wanted)
        .unwrap_or_else(|| panic!("{wanted} is not in the plan, so a test lost its subject"))
}

/// The identifiers of a list of rows, in the order the rows came in.
fn ids<'a>(rows: impl IntoIterator<Item = &'a CriticalCheck>) -> Vec<CheckId> {
    rows.into_iter().map(|check| check.id().clone()).collect()
}

/// Every ordering of a set.
///
/// Written out rather than pulled from a crate, because the claim it supports is
/// that the *module* is order-independent, and a dependency that quietly sorted
/// its own input would be the wrong instrument to prove it with.
fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut orderings = Vec::new();
    for index in 0..items.len() {
        let mut rest = items.to_vec();
        let taken = rest.remove(index);
        for mut tail in permutations(&rest) {
            let mut ordering = vec![taken.clone()];
            ordering.append(&mut tail);
            orderings.push(ordering);
        }
    }
    orderings
}

#[test]
fn a_run_whose_critical_checks_all_pass_is_green() {
    // The anti-vacuity rule, and it comes first on purpose. Every other test in
    // this file is about a verdict *not* being green, and an implementation that
    // answered `not_enough_checked` to everything would satisfy all of them.
    let schedule = a_plan_that_runs_everything();
    assert_eq!(
        schedule.may_run().count(),
        THE_CHECKS.len(),
        "this is the plan every check in it is allowed to run, which is what makes it the \
         anti-vacuity case"
    );
    let fingerprint = FingerprintId::generate();
    let results: Vec<CheckResult> = schedule
        .may_run()
        .map(|scheduled| a_result_for(scheduled, CheckStatus::Pass, &fingerprint))
        .collect();

    let report = aggregate_run(&schedule, &results, &fingerprint)
        .expect("one result per scheduled check is aggregable");

    assert_eq!(
        report.aggregate().severity,
        AggregateSeverity::Green,
        "a run of checks that all ran and all passed is what green means; the headline was {:?}",
        report.aggregate().headline
    );
    assert!(report.is_green());
    assert_eq!(report.blocking().count(), 0);
    assert!(report.unreported().is_empty());
    assert!(report.overruled().is_empty());
    assert!(report.unscheduled().is_empty());
    assert_eq!(report.critical().len(), 3, "three of the four are critical");
    assert_eq!(report.skipped().count(), 0);
    assert_eq!(report.errored().count(), 0);
    assert_eq!(report.unknown().count(), 0);
    assert_eq!(
        report.critical().len(),
        report.in_state(CriticalState::Passed).count(),
        "every critical row is in the one state, so the accessors and the rows agree"
    );
}

#[test]
fn the_three_states_the_acceptance_names_are_answered_separately() {
    // Rule one. One run holding one of each, made three different ways: the
    // plan's own answer for a check it stopped, a checker that failed, and
    // nothing reported at all.
    let schedule = a_plan();
    assert_eq!(
        schedule.blocked().count(),
        1,
        "under a mode that runs nothing, exactly the check that runs the project's code is \
         stopped, and that is what makes this a schedule rather than four proposals"
    );
    let fingerprint = FingerprintId::generate();
    let results = vec![
        a_result_for(
            entry(&schedule, &the_id('g')),
            CheckStatus::Error,
            &fingerprint,
        ),
        a_result_for(
            entry(&schedule, &the_id('s')),
            CheckStatus::Pass,
            &fingerprint,
        ),
        // `l` is simply absent: the plan would have run it and nothing came
        // back, which is the row the frozen rule cannot see is missing.
        // `c` is stopped by the plan, so it has a result without anyone
        // reporting one.
    ];
    let report = aggregate_run(&schedule, &results, &fingerprint)
        .expect("nothing here is contradictory, so this aggregates");

    assert!(!report.is_green());
    assert_eq!(
        report.aggregate().severity,
        AggregateSeverity::NotEnoughChecked
    );

    // The ids are asserted and not the counts, so a report that classified the
    // three rows by a rule of its own would fail here rather than pass by having
    // one of each.
    assert_eq!(
        ids(report.skipped()),
        vec![the_id('c')],
        "the check the plan stopped is the one that reads as `did not run`"
    );
    assert_eq!(
        ids(report.errored()),
        vec![the_id('g')],
        "the check whose checker failed is the one that reads as errored"
    );
    assert_eq!(
        ids(report.unknown()),
        vec![the_id('l')],
        "the check nothing came back for is the one that reads as unknown"
    );
    assert_eq!(
        report.unreported(),
        [the_id('l')],
        "the check nothing was reported for is named, so a dropped row is read rather than \
         inferred from a count"
    );
    assert!(
        report.overruled().is_empty(),
        "nothing claimed a stopped check had run"
    );
    assert_eq!(
        report.blocking().count(),
        3,
        "a stopped check, a failed checker and a check with no verdict all stop a green; the \
         non-critical pass does not"
    );

    // Three states, three sentences.
    let sentence = |state: CriticalState| {
        report
            .in_state(state)
            .next()
            .expect("one row in this state")
            .plain_description()
    };
    assert_eq!(
        sentence(CriticalState::NotRun),
        format!(
            "the declared tests pass - did not run: {}",
            NotCheckedReason::ExecutionNotAuthorized.plain_explanation()
        )
    );
    assert_eq!(
        sentence(CriticalState::CheckerError),
        "the declared type check is clean - SURE's own check failed: the check could not be \
         completed"
    );
    // The sentence is written out here rather than read from the module, because
    // the claim is about the words a user sees: a report that showed a different
    // sentence should fail this file, and a file that imported the constant would
    // agree with any wording at all.
    assert_eq!(
        sentence(CriticalState::Uncertain),
        "the declared metadata is readable - could not be determined: Nothing was reported for \
         this check, so SURE has no basis for a verdict on it."
    );

    // And the reading that was impossible before this task: the frozen coverage
    // summary is where skipped and unknown are one list, and both of them are in
    // it under a name that does not say which is which.
    let mut merged = report.aggregate().coverage.critical_not_checked.clone();
    merged.sort();
    let mut expected = vec![the_id('c'), the_id('l')];
    expected.sort();
    assert_eq!(
        merged, expected,
        "the frozen summary merges the two states, which is why a caller needs the report to \
         tell them apart"
    );
    assert_eq!(
        report.aggregate().coverage.critical_errored,
        vec![the_id('g')],
        "an error was the one state the frozen summary did keep separate"
    );
    assert_eq!(
        report.aggregate().coverage.critical_checked,
        0,
        "not one of the three states the acceptance names counts as a critical check that ran, \
         which is exactly why a verdict that only carried the coverage counts could not tell a \
         reader what happened"
    );
}

#[test]
fn the_rows_come_back_in_the_plans_own_order() {
    // The order is `P4-T001`'s decision and not this module's, which is why it is
    // read back from the schedule rather than written down here: a report that
    // sorted by severity, or that kept whatever order the results arrived in,
    // would be a second ordering rule sitting beside the plan's.
    let schedule = a_plan();
    let fingerprint = FingerprintId::generate();
    // Handed over in the reverse of plan order, and with one check missing, so
    // neither the arrival order nor the plan order can be what the rows come
    // back in by accident.
    let results: Vec<CheckResult> = schedule
        .checks()
        .iter()
        .rev()
        .filter(|scheduled| scheduled.may_run())
        .map(|scheduled| a_result_for(scheduled, CheckStatus::Pass, &fingerprint))
        .collect();
    let report = aggregate_run(&schedule, &results, &fingerprint).expect("aggregable");

    let planned: Vec<CheckId> = schedule
        .checks()
        .iter()
        .filter(|scheduled| scheduled.proposal().critical())
        .map(|scheduled| scheduled.proposal().id().clone())
        .collect();
    let backwards: Vec<CheckId> = planned.iter().rev().cloned().collect();
    assert_ne!(
        planned, backwards,
        "the plan's order happens to be its own reverse here, so this test would pass whatever \
         the module did"
    );
    assert_eq!(
        ids(report.critical()),
        planned,
        "the rows are the plan's checks in the plan's order"
    );
    assert_eq!(
        report.plain_description().len(),
        planned.len(),
        "and the one-line-per-check listing has one line per row"
    );
}

#[test]
fn every_way_a_critical_check_can_fail_to_pass_stops_the_run_being_green() {
    // Rule two, as a sweep rather than as cases. The claim is an equivalence —
    // green exactly when the one critical check passed — and it is asserted in
    // both directions, so neither an aggregator that never says green nor one
    // that always does can satisfy it.
    let schedule = a_one_check_plan();
    let fingerprint = FingerprintId::generate();
    let scheduled = entry(&schedule, &the_id('c'));
    assert!(scheduled.may_run(), "the sweep needs a check that runs");

    for &status in CheckStatus::ALL {
        let result = a_result_for(scheduled, status, &fingerprint);
        let report = aggregate_run(&schedule, std::slice::from_ref(&result), &fingerprint)
            .expect("one result for the one scheduled check is aggregable");

        assert_eq!(
            report.is_green(),
            status == CheckStatus::Pass,
            "a run whose only critical check came back {} is {}",
            status.as_str(),
            if report.is_green() {
                "green"
            } else {
                "not green"
            }
        );
        assert_eq!(
            report.blocking().count(),
            usize::from(matches!(
                status,
                CheckStatus::Fail
                    | CheckStatus::Skipped
                    | CheckStatus::Error
                    | CheckStatus::Unknown
            )),
            "{} has the wrong blocking answer",
            status.as_str()
        );
        assert_eq!(
            report.critical().len(),
            1,
            "one critical check is one row, whatever came back for it"
        );
        // A warning is the row worth reading twice: the domain classifies it as
        // `Passed` — the check did produce a result — and the run is still not
        // green, because a warning is something to look at before handing a
        // project over. Both facts are the frozen rule's and neither is this
        // module's, which is why they are asserted together here.
        let produced = report.in_state(CriticalState::Passed).count();
        assert_eq!(
            produced,
            usize::from(matches!(status, CheckStatus::Pass | CheckStatus::Warning)),
            "{} was classified as a pass or not by a rule that is not `produced_a_result`",
            status.as_str()
        );
        let stopped_it = report.skipped().count()
            + report.errored().count()
            + report.unknown().count()
            + report.in_state(CriticalState::Failed).count();
        assert_eq!(
            stopped_it,
            1 - produced,
            "a run that is not green has to name the one row that stopped it, in the vocabulary \
             the acceptance asks for; {} has {stopped_it} such rows",
            status.as_str()
        );
    }
}

#[test]
fn the_plan_stops_a_result_that_claims_a_check_ran() {
    // Rule three, first half. The mode stopped the check that runs the project's
    // code, and a caller reports a pass for it anyway. The plan's answer is what
    // is aggregated — a stopped check did not run, whatever came back — and the
    // claim is recorded rather than absorbed.
    let schedule = a_plan();
    let stopped = the_id('c');
    let fingerprint = FingerprintId::generate();
    let mut results = vec![a_result_for(
        entry(&schedule, &stopped),
        CheckStatus::Pass,
        &fingerprint,
    )];
    for letter in ['g', 'l', 's'] {
        results.push(a_result_for(
            entry(&schedule, &the_id(letter)),
            CheckStatus::Pass,
            &fingerprint,
        ));
    }

    let report = aggregate_run(&schedule, &results, &fingerprint)
        .expect("one result per scheduled check is aggregable");

    assert_eq!(
        report.overruled(),
        std::slice::from_ref(&stopped),
        "the check that was claimed to have run is named"
    );
    assert_eq!(
        ids(report.skipped()),
        vec![stopped],
        "and it is aggregated as the plan's own stopped entry, so the claim did not become a pass"
    );
    assert!(
        !report.is_green(),
        "a caller's claim cannot turn a check the run was not allowed to perform into evidence"
    );
    assert_eq!(
        report.aggregate().coverage.critical_checked,
        2,
        "the two critical checks that really ran are the two the coverage counts, and the \
         claimed one is not among them"
    );
}

#[test]
fn a_result_for_a_check_the_plan_never_proposed_is_reported_and_changes_nothing() {
    // Rule three, second half, in both directions: a stray pass cannot make a
    // run green and a stray fail cannot make it red. The verdict is about the
    // run, and the run is what the plan said it would be.
    let schedule = a_plan();
    let fingerprint = FingerprintId::generate();
    let results: Vec<CheckResult> = schedule
        .may_run()
        .map(|scheduled| a_result_for(scheduled, CheckStatus::Pass, &fingerprint))
        .collect();
    let quiet = aggregate_run(&schedule, &results, &fingerprint).expect("aggregable");

    let stray = id("zzzzzzzzzzzzzzzzzzzz");
    for status in [CheckStatus::Pass, CheckStatus::Fail] {
        let mut with_stray = results.clone();
        with_stray.push(a_result_named(
            stray.clone(),
            "a check this plan never proposed",
            Severity::MustFix,
            true,
            status,
            &fingerprint,
        ));

        let report = aggregate_run(&schedule, &with_stray, &fingerprint).expect("aggregable");
        assert_eq!(
            report.unscheduled(),
            std::slice::from_ref(&stray),
            "the check the plan never proposed is named, and the {} it came back with did not \
             get it into the run",
            status.as_str()
        );
        assert_eq!(
            report.aggregate(),
            quiet.aggregate(),
            "a {} for a check outside the plan moved the verdict",
            status.as_str()
        );
        assert_eq!(
            report.critical(),
            quiet.critical(),
            "and it moved no row either"
        );
    }
}

#[test]
fn a_stray_result_cannot_make_an_empty_plan_green() {
    // The sharpest form of the rule above, and the one that reads as a false
    // green if the rule is not there at all: a plan with nothing in it, and one
    // passing result from somewhere else. `aggregate` on its own sees a single
    // passing critical check and calls the run green.
    let schedule = PlanBuilder::new(
        ExecutionMode::InspectOnly,
        ExecutionPermissions::inspect_only(),
    )
    .build();
    assert!(schedule.is_empty());

    let fingerprint = FingerprintId::generate();
    let report = aggregate_run(
        &schedule,
        &[a_stray_result('z', &fingerprint)],
        &fingerprint,
    )
    .expect("a stray result is reported, not refused");

    assert_eq!(report.unscheduled().len(), 1);
    assert!(!report.is_green());
    assert_eq!(
        report.aggregate().severity,
        AggregateSeverity::NotEnoughChecked,
        "an empty run is not green, however good a result arrived from outside it"
    );
    assert!(report.critical().is_empty());
    assert_eq!(
        report.aggregate().coverage.critical_checked,
        0,
        "the stray result is not counted as a critical check that ran"
    );
}

#[test]
fn a_critical_check_with_no_result_cannot_be_dropped_out_of_the_verdict() {
    // The other false green, and the one the frozen rule cannot close for
    // itself: the same results with and without the failing row. If the set were
    // trusted, dropping a row would be a way to turn a red run green.
    let schedule = a_plan_that_runs_everything();
    let fingerprint = FingerprintId::generate();
    let failing = the_id('g');
    let complete: Vec<CheckResult> = schedule
        .may_run()
        .map(|scheduled| {
            let status = if scheduled.proposal().id() == &failing {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            };
            a_result_for(scheduled, status, &fingerprint)
        })
        .collect();

    let full = aggregate_run(&schedule, &complete, &fingerprint).expect("aggregable");
    assert!(
        !full.is_green(),
        "one critical check failed, so the run is not green"
    );
    assert_eq!(
        full.aggregate().severity,
        AggregateSeverity::NotReady,
        "a failing critical check is a statement about the project, and not about coverage"
    );

    // Now the same run with that one row quietly left out of the results.
    let shorter: Vec<CheckResult> = complete
        .iter()
        .filter(|result| result.id != failing)
        .cloned()
        .collect();
    let dropped = aggregate_run(&schedule, &shorter, &fingerprint).expect("aggregable");

    assert_eq!(
        dropped.unreported(),
        std::slice::from_ref(&failing),
        "the row that went missing is named rather than counted"
    );
    assert!(
        !dropped.is_green(),
        "a result set with a row missing must not be greener than the one it came from"
    );
    assert_eq!(
        ids(dropped.unknown()),
        vec![failing],
        "the missing check is read as one SURE has no verdict on"
    );
    assert_eq!(
        dropped.aggregate().severity,
        AggregateSeverity::NotEnoughChecked,
        "and the severity says the run does not know, which is the whole difference from the \
         run it came from"
    );
}

#[test]
fn every_order_of_the_same_results_gives_the_same_report() {
    // Rule four. Six results, and every one of the 720 orderings of them: a
    // verdict that depended on the order a caller's loop collected results in
    // would be a transcript of that loop rather than a value. The fixture was
    // five results when the claim was first written; the sixth is a second stray
    // and it was added because with one stray the assertion below held whatever
    // order the extras came back in, which made it a claim about membership
    // wearing the words of a claim about order.
    let schedule = a_plan();
    let fingerprint = FingerprintId::generate();
    let mut results: Vec<CheckResult> = schedule
        .may_run()
        .map(|scheduled| a_result_for(scheduled, CheckStatus::Pass, &fingerprint))
        .collect();
    // A pass for the check the plan stopped, so the overruled path is inside the
    // permutation too, and one result for a check no plan proposed.
    results.push(a_result_for(
        entry(&schedule, &the_id('c')),
        CheckStatus::Pass,
        &fingerprint,
    ));
    // Two results for checks no plan proposed, handed over in the opposite of
    // the order they come back in, so the extras' order is asserted rather than
    // their membership.
    results.push(a_stray_result('z', &fingerprint));
    results.push(a_stray_result('y', &fingerprint));
    assert_eq!(results.len(), 6);

    let orderings = permutations(&results);
    assert_eq!(orderings.len(), 720, "six results have 720 orderings");
    let first = aggregate_run(&schedule, &orderings[0], &fingerprint).expect("aggregable");
    for (index, ordering) in orderings.iter().enumerate() {
        let report = aggregate_run(&schedule, ordering, &fingerprint).expect("aggregable");
        assert_eq!(
            report, first,
            "ordering {index} produced a different report from the first one"
        );
    }
    assert_eq!(
        first.overruled(),
        [the_id('c')],
        "the permutation did not exercise the path it was written for, so it would prove less \
         than it says"
    );
    assert_eq!(
        first.unscheduled(),
        [id(&body('y')), id(&body('z'))],
        "the extras are listed in a settled order rather than in the order they arrived, which \
         is a claim of its own: an order taken from the caller would make the report a \
         transcript of the caller's loop"
    );
}

#[test]
fn handing_back_the_plans_own_stopped_entry_is_the_ordinary_case() {
    // The path `enforce` actually produces. For a check the plan stopped, the
    // caller hands back `ScheduledCheck::not_run`'s own result — the schedule
    // wrote that function for this caller — and nothing was claimed about the
    // check, so nothing is recorded. `overruled` is for a caller that said the
    // check *ran*, and a report that flagged the ordinary case would make the
    // list useless.
    let schedule = a_plan();
    let fingerprint = FingerprintId::generate();
    let mut results: Vec<CheckResult> = schedule
        .may_run()
        .map(|scheduled| a_result_for(scheduled, CheckStatus::Pass, &fingerprint))
        .collect();
    let stopped_entry = entry(&schedule, &the_id('c'))
        .not_run(&fingerprint)
        .expect("the plan stopped this check, so it has a stopped entry");
    assert_eq!(
        stopped_entry.status,
        CheckStatus::Skipped,
        "the schedule's own entry for a stopped check is a skip, which is what makes handing it \
         back different from claiming the check ran"
    );

    let without = aggregate_run(&schedule, &results, &fingerprint).expect("aggregable");
    results.push(stopped_entry);
    let with = aggregate_run(&schedule, &results, &fingerprint).expect("aggregable");

    assert!(
        with.overruled().is_empty(),
        "the ordinary case is not a claim and must not be recorded as one"
    );
    assert_eq!(
        with, without,
        "handing back the entry the schedule produced changed the report"
    );
}

#[test]
fn two_answers_for_one_check_are_refused_rather_than_chosen() {
    // Rule five, first refusal. There is no honest rule for choosing between two
    // answers to one question, and inventing one here would be new aggregation
    // semantics living in a composition layer.
    let schedule = a_one_check_plan();
    let fingerprint = FingerprintId::generate();
    let scheduled = entry(&schedule, &the_id('c'));
    let results = vec![
        a_result_for(scheduled, CheckStatus::Pass, &fingerprint),
        a_result_for(scheduled, CheckStatus::Fail, &fingerprint),
    ];

    let refused = aggregate_run(&schedule, &results, &fingerprint)
        .expect_err("two answers to one check cannot be aggregated");
    assert_eq!(
        refused,
        RunRefused::TwoAnswersForOneCheck { id: the_id('c') }
    );
    assert!(
        refused.to_string().contains(&the_id('c').to_string()),
        "the refusal names the check: {refused}"
    );
}

#[test]
fn a_result_from_another_project_state_is_refused() {
    // Rule five, second refusal. A verdict that mixed two project states would
    // read a pass about an older tree as a pass about this one.
    let schedule = a_one_check_plan();
    let fingerprint = FingerprintId::generate();
    let elsewhere = FingerprintId::generate();
    assert_ne!(
        fingerprint, elsewhere,
        "two generated states are two states"
    );
    let scheduled = entry(&schedule, &the_id('c'));
    let results = vec![a_result_for(scheduled, CheckStatus::Pass, &elsewhere)];

    let refused = aggregate_run(&schedule, &results, &fingerprint)
        .expect_err("a result about another state cannot be aggregated into this one");
    assert_eq!(
        refused,
        RunRefused::ADifferentProjectState {
            id: the_id('c'),
            found: elsewhere.clone(),
        }
    );
    assert!(
        refused.to_string().contains(&elsewhere.to_string()),
        "the refusal says which state the result was about: {refused}"
    );
}

#[test]
fn the_aggregation_reads_nothing_but_its_arguments() {
    // Rule six, first half, as a source rule rather than a promise: the report
    // is a function of the plan, the results and the project state, and a module
    // that could read a clock or a file could produce two answers for one input.
    //
    // The whole file is read rather than its shipped part, which is the stricter
    // direction: a name that only a test used would still fail here.
    let source = read_shipped(THE_AGGREGATION_IN_THE_WALK);
    assert!(
        source.contains("pub fn aggregate_run"),
        "the rule is being checked against a file that does not hold the function, which is the \
         false green every source rule in this repository is written against"
    );
    for word in IMPURE {
        assert!(
            !source.contains(word),
            "{THE_AGGREGATION_IN_THE_WALK} names {word}, so its answer is not a function of its \
             arguments alone"
        );
    }
}

#[test]
fn the_aggregation_does_not_restate_the_frozen_rule() {
    // Rule six, second half, about the file this task adds: no severity is chosen
    // here and no status is matched on, because the rule that decides both is
    // `sure-domain/src/status.rs`'s and `docs/adr/0010` names it the only
    // aggregation entry point.
    let source = shipped_code_only(THE_AGGREGATION_IN_THE_WALK);
    assert!(
        source.contains("aggregate(&complete)"),
        "the shipped part of the file does not call the frozen rule, so this rule is checking \
         something other than what it says"
    );
    for word in A_SECOND_AGGREGATIONS_WORDS {
        assert!(
            !source.contains(word),
            "{THE_AGGREGATION_IN_THE_WALK} carries {word:?}, which is how a second aggregation \
             would be written — the frozen one is reached through `sure_domain::status::aggregate` \
             and its answer is carried rather than recomputed"
        );
    }
}

#[test]
fn nothing_but_the_frozen_rule_builds_a_verdict() {
    // The same rule one level up, over the whole workspace: exactly one file
    // builds an `Aggregate`, and it is the file that owns the five rules.
    //
    // This is the rule that makes the two above it more than promises about one
    // file. A module that assembled an `Aggregate` of its own would have to
    // restate the severity rule to do it, and it would not be this file.
    let shipped = shipped_sources();
    let builders: Vec<&str> = shipped
        .iter()
        .filter(|(_, text)| shipped_part(text).contains(THE_MARK_OF_A_VERDICT))
        .map(|(path, _)| path.as_str())
        .collect();

    assert_eq!(
        builders.len(),
        1,
        "exactly one file may build a verdict; {} were found: {builders:?}. A file that needs a \
         verdict calls `sure_domain::status::aggregate` rather than assembling one, because the \
         five rules it would have to restate live in {THE_FROZEN_RULE}",
        builders.len(),
    );
    assert!(
        builders[0].ends_with(THE_FROZEN_RULE_IN_THE_WALK),
        "the one file that builds a verdict is {}, and this rule was written for {THE_FROZEN_RULE}",
        builders[0]
    );
}

/// Every shipped `.rs` file under `crates/`, as `(path, text)`.
///
/// Narrowed to `src/`, because the rule is about code that ships: a test file is
/// *supposed* to name these types — this one does — and a rule that counted them
/// would be a rule nobody could satisfy. The walk is the product's own scanner,
/// so "a file in this crate" means here what it means everywhere else, and it is
/// asserted complete, because a source rule over an unknown subset of the sources
/// is the false green this test exists to prevent.
fn shipped_sources() -> Vec<(String, String)> {
    let crates = sure_testkit::repository_root().join("crates");
    let walked = scan(&crates, ScanOptions::default()).expect("crates/ is a directory");
    assert!(
        walked.is_complete(),
        "the source tree could not be read completely, so these rules would be checking an \
         unknown subset of it"
    );

    let shipped: Vec<(String, String)> = walked
        .files()
        .filter(|entry| {
            entry
                .path
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .map(|entry| {
            let path = crates.join(&entry.path);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            (entry.display_path(), text)
        })
        .filter(|(path, _)| path.contains("src"))
        .collect();

    assert!(
        shipped.len() > 10,
        "the source walk found {} shipped files, which is not this workspace — the filter is \
         matching the wrong thing",
        shipped.len()
    );
    shipped
}

/// The whole text of a shipped file under `crates/sure-core/`.
fn read_shipped(relative: &str) -> String {
    let path = sure_testkit::repository_root()
        .join("crates/sure-core")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The part of a file above its `#[cfg(test)]` module.
///
/// A test is *supposed* to name the things these rules are about — this file
/// does, and so does the vocabulary's — and a rule that counted them would be a
/// rule nobody could satisfy. The cut follows the repository's own convention for
/// where the shipped part of a file ends.
///
/// **The empty string when there is no test module**, which a caller would read
/// as "this file builds no verdict" rather than as "this file is all shipped
/// code". That is the direction that cannot produce a false green: the failure it
/// can produce is a file that builds a verdict and is not counted, and the
/// caller's count is asserted to be exactly one, so a workspace where the real
/// builder had no test module would be caught by the count rather than silently
/// passing.
fn shipped_part(text: &str) -> &str {
    text.split_once("\n#[cfg(test)]")
        .map_or("", |(shipped, _)| shipped)
}

/// The same cut, read from a file.
fn shipped_code_only(relative: &str) -> String {
    shipped_part(&read_shipped(relative)).to_owned()
}

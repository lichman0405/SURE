//! The door between a plan and a process.
//!
//! [`crate::planned_work`] carries a check's executable half as named fields, and
//! [`crate::enforce`] decides which of those may run. Neither of them starts
//! anything, and both say so: `CommandSpec` has deliberately no method that turns
//! it into a runner's request, and `AdmittedCommand::new` is private so that the
//! only value carrying a "yes" is one the enforcement produced. This module is the
//! caller those two were written for. It is the first file in the product that can
//! carry a planned command to [`crate::process`], and `tests/spawn_sites.rs` is
//! the census that makes its arrival a deliberate edit rather than a quiet one.
//!
//! # What runs, and what makes that a fact about a type
//!
//! The runner's input is [`AdmittedRun`], and there is no way to build one except
//! [`AdmittedRun::new`], which demands an [`AdmittedCommand`]. That type has a
//! private constructor and is produced by exactly one function, so a check that
//! enforcement did not admit is not *unlikely* to reach the process runner — it is
//! unrepresentable, because there is no value to hand the runner and no way to
//! make one. This is decision 4 of
//! `docs/adr/0014-planned-check-execution-contract.md` stated as a signature
//! rather than as a rule for a reader.
//!
//! Pairing is not a formality either. An admitted command is two things — a
//! program and an argument vector — and so is a [`CommandSpec`], and
//! [`AdmittedRun::new`] refuses unless the check, the program and the arguments
//! all agree. Everything else a spec carries (directory, environment, limits) is
//! not part of what was decided, so it is not part of what is compared; see
//! [`crate::safety::classify`], which is the decision and takes the same two
//! things.
//!
//! # One seam, and no test in this file starts a process
//!
//! [`CommandRunner`] is the whole of the machine: one method, from an admitted run
//! to a [`CommandRun`]. [`ProcessRunner`] is the implementation that calls
//! [`crate::process::run`], and it is the only one in this file that can start
//! anything. Every proof below — that an unadmitted command never arrives, that
//! one check produces one result, that a check which reported nothing becomes an
//! error — is made with a fake that records what it was asked to run and answers
//! from a value. **A test that starts a real process is a test that measures this
//! machine rather than this code**, and nothing in this module's acceptance needs
//! one: `tests/process_runner.rs` is where starting a process is the subject.
//!
//! # Exactly one result per scheduled check
//!
//! Rule six of the ADR, in three parts, and this module is where two of them land.
//!
//! - **A check the mode stopped keeps the plan's own stopped result.** Asked
//!   first, of the plan, and then of the enforcement: a check the schedule
//!   decided against is not carried out whatever its operation holds, and a check
//!   the schedule allowed but whose command the mode stopped keeps the result that
//!   command's own refusal carries. Both are `Skipped` and neither can be a pass.
//! - **A check the mode admitted that produced no result becomes an `Error`.** Not
//!   a silence, not a row quietly missing from the run's list, and never a pass.
//!   This is the gap `coverage_summary.rs:75`'s `None => continue` leaves open for
//!   any caller that hands it a set that does not cover the plan: a scheduled check
//!   with no result is a check SURE said it would carry out, and the honest report
//!   of having no observation for it is that SURE's own machinery did not produce
//!   one.
//! - **Two results for one check is an internal error that stops the run**, and so
//!   is the same duplication one step earlier — two commands the enforcement
//!   admitted for one check. There is no honest rule for choosing between two
//!   answers to one question, and inventing one (first wins, least green wins)
//!   would be new run semantics living in a runner.
//!
//! [`crate::aggregation::aggregate_run`] states a rule that looks like the second
//! of those and is not the same one: a set of results that does not cover a check
//! is aggregated as `Unknown` with [`crate::aggregation::NOTHING_CAME_BACK`],
//! because a result *set* that is missing a row is not evidence about why. The two
//! are kept apart on purpose. The runner knows what it was asked to carry out and
//! says so; the aggregator knows only what it was handed, and says only that. The
//! effect of this module's rule is that the aggregator's arm is now rare rather
//! than always — which is what the ADR's consequence note says it should be.
//!
//! # What this module does not do
//!
//! **It does not decide what may run.** Every "yes" comes from
//! [`Enforcement::admitted`], and a caller holding every command the plan
//! considered has nothing it can do with them here.
//!
//! **It does not carry out services or browsers.** [`CheckOperation::Service`] and
//! [`CheckOperation::Browser`] are named rather than ignored — a check of either
//! kind is reported as an `Error` saying this build has no runner for it — because
//! a `match` with a wildcard arm would let a fifth kind of work arrive as a silent
//! nothing. `P18-T009` and `P18-T010` are where those two doors are opened.
//!
//! **It builds no command line.** The one translation here is field for field, and
//! an argument holding a space is one argument before it and one argument after it.

use std::collections::BTreeMap;
use std::fmt;

use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::status::CheckResult;

use crate::enforce::{AdmittedCommand, Enforcement};
use crate::planned_work::{CheckOperation, CommandRun, CommandSpec};
use crate::process::{self, Cancellation, ProcessRequest};
use crate::schedule::{CheckSchedule, ScheduledCheck};

/// What SURE says about a check the mode admitted that produced no result.
///
/// **An `Error`, and the reason it is not a silence is the whole of clause two.**
/// A check the mode admitted is a check SURE said it would carry out; a runner
/// that comes back with nothing for it has not observed anything, and the honest
/// report of that is that SURE's own machinery did not produce the observation the
/// plan promised. It is not `Unknown` and it is not a skip: `Unknown` belongs to
/// [`crate::aggregation::NOTHING_CAME_BACK`], where a *set* of results does not
/// cover a check and nothing is known about why, and a skip is a decision somebody
/// made. This is neither — it is an answer SURE owes and does not have.
///
/// A `Warning` would be worse than either: `CriticalState::from_status` maps
/// `Warning` to `Passed`, so a warning on a critical check blocks nothing at all.
const NOTHING_WAS_REPORTED: &str = "SURE admitted this check and the runner reported nothing for \
                                    it, so nothing about it was observed.";

/// The one seam at which an admitted command becomes a process.
///
/// **A trait rather than a function**, for one reason that matters and one that
/// follows from it. The reason that matters: an acceptance about *what may run*
/// has to be provable without running anything, and a seam is what lets the proof
/// be made with a value that records what it was asked for. The one that follows:
/// the interface a fake has to satisfy is one method wide, so a fake cannot agree
/// with the real runner on everything except the thing being tested.
///
/// The method takes [`AdmittedRun`] and not a [`CommandSpec`], which is what makes
/// "an unadmitted command cannot reach a process" a property of the signature
/// rather than a promise in a comment.
///
/// **[`fmt::Debug`] is a supertrait, and the reason is one field rather than this
/// module.** `Pipeline` holds a `&dyn CommandRunner` — a run is built with the
/// runner it will use, so that a caller answering "what may this run start?" is
/// answering it at the place the run is described — and `Pipeline` prints itself
/// when a test fails, so the runner it holds has to be printable. Nothing about
/// running a command needs this; what needs it is a runner being part of a value a
/// person reads.
pub trait CommandRunner: fmt::Debug {
    /// Carry out one admitted command and report what came of it.
    ///
    /// [`CommandRun`] rather than a `Result`, because the runner's two answers —
    /// *the process ran, and this is what it said* and *the process never started,
    /// and this is why* — are one question about one check, and
    /// [`CommandRun::to_result`] is where the answer is read.
    fn run(&self, work: &AdmittedRun<'_>) -> CommandRun;
}

/// The runner that starts a real process.
///
/// The only implementation in the product that can, and it is reached from
/// [`run_scheduled_checks`] by whoever the pipeline hands it to.
#[derive(Debug, Clone, Default)]
pub struct ProcessRunner {
    /// The handle a caller cancels a run through. Held rather than made per call,
    /// so that the caller keeps the other clone and cancelling it reaches every
    /// request the runner makes.
    cancellation: Cancellation,
}

impl ProcessRunner {
    /// A runner whose runs are cancelled through `cancellation`.
    #[must_use]
    pub fn new(cancellation: Cancellation) -> Self {
        Self { cancellation }
    }

    /// The handle this runner's requests carry.
    #[must_use]
    pub const fn cancellation(&self) -> &Cancellation {
        &self.cancellation
    }
}

impl CommandRunner for ProcessRunner {
    fn run(&self, work: &AdmittedRun<'_>) -> CommandRun {
        // One call, and the conversion is `CommandRun`'s own: `process::run`
        // returns exactly the `Result` that `From` is written for, so the step
        // from the machinery to the check's result is not a reshape written here.
        process::run(&work.request(&self.cancellation)).into()
    }
}

/// One scheduled check's command, paired with the admission that lets it run.
///
/// **The only value a [`CommandRunner`] can be handed**, and it cannot be built
/// without an [`AdmittedCommand`] — a type with a private constructor that exactly
/// one function produces. That is what makes *unadmitted work cannot reach a
/// process* a fact about the type system rather than a rule about how callers
/// ought to behave.
///
/// It holds the [`CommandSpec`] as well as the admission because the admission is
/// not the whole of the work: the enforcement decided about a program and an
/// argument vector, and the directory, the environment policy and the deadline
/// live in the spec. Pairing the two is what lets the request be built field for
/// field from the plan's own value, with nothing reconstructed in between.
#[derive(Debug, Clone, Copy)]
pub struct AdmittedRun<'a> {
    admitted: AdmittedCommand<'a>,
    spec: &'a CommandSpec,
}

impl<'a> AdmittedRun<'a> {
    /// Pair a scheduled check's work with the command the enforcement admitted
    /// for it.
    ///
    /// # Errors
    ///
    /// [`AdmissionRefused`] when the two cannot be the same command: when the
    /// check's work is not one command, when the admitted command belongs to a
    /// different check, or when the program or the argument vector differs from
    /// the one the plan holds. **The argument vector is compared element by
    /// element and never compared as text**, because two commands whose rendered
    /// lines are equal are not necessarily the same command and the admission was
    /// a decision about the elements.
    pub fn new(
        scheduled: &'a ScheduledCheck,
        admitted: AdmittedCommand<'a>,
    ) -> Result<Self, AdmissionRefused> {
        let proposal = scheduled.proposal();
        let id = proposal.id();
        let CheckOperation::Command(spec) = scheduled.operation() else {
            return Err(AdmissionRefused::NotOneCommand {
                id: id.clone(),
                work: scheduled.operation().plain_description(),
            });
        };

        let command = admitted.command();
        if command.check().id() != id {
            return Err(AdmissionRefused::ForADifferentCheck {
                id: id.clone(),
                admitted_for: command.check().id().clone(),
            });
        }
        if command.program() != spec.program() || command.arguments() != spec.arguments() {
            return Err(AdmissionRefused::NotTheCommandThatWasAdmitted {
                id: id.clone(),
                planned: display_of(spec),
                admitted: command.display(),
            });
        }

        Ok(Self { admitted, spec })
    }

    /// The check this command belongs to.
    #[must_use]
    pub fn check(&self) -> &'a CheckId {
        self.admitted.command().check().id()
    }

    /// The command as the plan holds it: program, arguments, directory,
    /// environment and limits.
    #[must_use]
    pub const fn command(&self) -> &'a CommandSpec {
        self.spec
    }

    /// The request [`crate::process::run`] is given, built field for field.
    ///
    /// The program, the working directory, the deadline and the output bound are
    /// the spec's own values, handed over unchanged; the argument vector is the
    /// spec's vector, handed over whole. **Nothing here is rendered, joined,
    /// split, quoted or re-parsed** — an argument holding a space is one element
    /// on both sides of this call — and `cancellation` is the only thing this
    /// function adds, because a request requires one and a spec has no business
    /// holding a handle to a caller's decision to stop.
    ///
    /// Public because the translation is the part of this module most likely to be
    /// got subtly wrong, and a test that can hold the request can compare it
    /// against the spec it came from without starting anything.
    #[must_use]
    pub fn request(&self, cancellation: &Cancellation) -> ProcessRequest {
        ProcessRequest::new(
            self.spec.program(),
            self.spec.working_directory(),
            self.spec.limits(),
            cancellation.clone(),
        )
        .with_arguments(self.spec.arguments())
        .with_environment(self.spec.environment().clone())
    }
}

/// Why an admitted command could not be paired with a scheduled check's work.
///
/// Three ways to hand in a pair that is not one command, each refused rather than
/// repaired. Repairing any of them would mean SURE running something the plan did
/// not hold or something nobody admitted, and the second of those is the whole
/// point of the type the pair is built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRefused {
    /// The check's work is not one command, so there is no command for the
    /// admission to be about.
    NotOneCommand {
        /// The check that was paired.
        id: CheckId,
        /// What the check's work is, in the words a report uses for it.
        work: &'static str,
    },
    /// The admitted command belongs to another check.
    ForADifferentCheck {
        /// The check whose work was handed in.
        id: CheckId,
        /// The check the admitted command actually belongs to.
        admitted_for: CheckId,
    },
    /// The admission covers a different program or a different argument vector.
    NotTheCommandThatWasAdmitted {
        /// The check both commands claim.
        id: CheckId,
        /// The command the plan holds for it, as a person reads it.
        planned: String,
        /// The command the enforcement admitted for it, as a person reads it.
        admitted: String,
    },
}

impl fmt::Display for AdmissionRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOneCommand { id, work } => write!(
                f,
                "the check {id} would be carried out as {work}, and an admission can only be \
                 paired with a check whose work is one command"
            ),
            Self::ForADifferentCheck { id, admitted_for } => write!(
                f,
                "the command the enforcement admitted belongs to the check {admitted_for}, and it \
                 was paired with the check {id}"
            ),
            Self::NotTheCommandThatWasAdmitted {
                id,
                planned,
                admitted,
            } => write!(
                f,
                "the admission covers `{admitted}` and the plan holds `{planned}` for the check \
                 {id}, so the command SURE would start is not the command it decided about"
            ),
        }
    }
}

impl std::error::Error for AdmissionRefused {}

/// A refusal that stops the run.
///
/// Each of these is a run SURE cannot report on honestly, and none is repaired:
/// choosing between two commands, or between two results, would make the run
/// depend on a rule nobody wrote down, and dropping a result the schedule does not
/// hold would be the silent loss this repository treats as worse than an error.
/// [`crate::aggregation::RunRefused`] is the same idea one layer up, about the
/// verdict rather than about the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerRefused {
    /// The enforcement admitted more than one command for one check.
    ///
    /// A check is one command — [`CheckOperation::Command`] holds one spec, and
    /// `ScheduledCheck::plain_description` says "one command, once, under a
    /// deadline" — so a second admitted command is the plan and the enforcement
    /// disagreeing about what the check is. **Refused before anything is carried
    /// out**, so no work starts under a plan this runner has just found
    /// unreadable. A future check that is genuinely several commands needs a
    /// vocabulary for that, and this refusal is the thing that will have to move.
    MoreThanOneCommandForACheck {
        /// The check that was given two commands.
        id: CheckId,
    },
    /// Two results were produced for one check.
    TwoResultsForOneCheck {
        /// The check that was answered twice.
        id: CheckId,
    },
    /// A result was handed in for a check the schedule does not hold.
    ///
    /// Reported rather than absorbed. [`crate::aggregation::RunReport`] can carry
    /// such a result to a caller as an anomaly, because a verdict is computed over
    /// a set; a run's own results cannot, because they are the schedule's list and
    /// nothing else.
    ResultForACheckThatWasNotScheduled {
        /// The check the result claims.
        id: CheckId,
    },
}

impl fmt::Display for RunnerRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MoreThanOneCommandForACheck { id } => write!(
                f,
                "the enforcement admitted more than one command for the check {id}, so there is no \
                 one command this check is"
            ),
            Self::TwoResultsForOneCheck { id } => write!(
                f,
                "two results were produced for the check {id}, and a run has one answer for a \
                 check"
            ),
            Self::ResultForACheckThatWasNotScheduled { id } => write!(
                f,
                "a result for the check {id} was handed in, and this schedule does not hold that \
                 check"
            ),
        }
    }
}

impl std::error::Error for RunnerRefused {}

/// What a run produced: one result per scheduled check, in the plan's order.
///
/// **Not a verdict**, and the type says so by being a list of results rather than
/// an aggregate: [`crate::aggregation::RunReport`] is what a verdict is, and it is
/// built from these by [`crate::aggregation::aggregate_run`]. The difference is
/// the one decision 3 of the ADR draws — the plan is metadata and this is the
/// evidence — and a report that could be green while a result cannot is the reason
/// they are two types rather than one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResults {
    /// One entry per scheduled check, in the schedule's own order. The order is
    /// the schedule's and not this module's, so a report cannot number its rows
    /// differently from the plan they came from.
    results: Vec<CheckResult>,
}

impl RunResults {
    /// Turn what was produced into the run's results, one per scheduled check.
    ///
    /// Public because the two rules it enforces are the acceptance: exactly one
    /// result per scheduled check, and a check that produced nothing is an `Error`
    /// rather than a missing row. `produced` is what the runner (or any caller)
    /// collected; the schedule is what decides the shape of the answer.
    ///
    /// # Errors
    ///
    /// [`RunnerRefused::TwoResultsForOneCheck`] — a duplicate is not resolved by
    /// choosing. [`RunnerRefused::ResultForACheckThatWasNotScheduled`] — a result
    /// that cannot be placed is not dropped.
    pub fn assemble(
        schedule: &CheckSchedule,
        produced: &[CheckResult],
        project_fingerprint: &FingerprintId,
    ) -> Result<Self, RunnerRefused> {
        // Counted before anything is assembled, so a refusal is a statement about
        // the caller's set and not about a half-built run.
        let mut by_id: BTreeMap<&CheckId, &CheckResult> = BTreeMap::new();
        for result in produced {
            if by_id.insert(&result.id, result).is_some() {
                return Err(RunnerRefused::TwoResultsForOneCheck {
                    id: result.id.clone(),
                });
            }
        }
        for id in by_id.keys() {
            if schedule.get(id).is_none() {
                return Err(RunnerRefused::ResultForACheckThatWasNotScheduled {
                    id: (*id).clone(),
                });
            }
        }

        let mut results: Vec<CheckResult> = Vec::with_capacity(schedule.len());
        for scheduled in schedule.checks() {
            let result = match by_id.remove(scheduled.proposal().id()) {
                Some(produced) => produced.clone(),
                // Nothing was produced for this check, and what that means is
                // the whole of what this function is for.
                None => unreported(scheduled, project_fingerprint),
            };
            results.push(result);
        }

        Ok(Self { results })
    }

    /// Every scheduled check and what became of it, in plan order.
    #[must_use]
    pub fn results(&self) -> &[CheckResult] {
        &self.results
    }

    /// The result for one check, if it is one of these.
    ///
    /// Looking one up by identity is how a caller joins a result back to the plan
    /// that proposed it, which is the same thing
    /// [`CheckSchedule::get`] does in the other direction.
    #[must_use]
    pub fn get(&self, id: &CheckId) -> Option<&CheckResult> {
        self.results.iter().find(|result| &result.id == id)
    }

    /// How many results there are, which is how many checks were scheduled.
    #[must_use]
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Whether the run has no results at all, which is to say nothing was
    /// scheduled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

/// Carry out every scheduled check that may be carried out, and report one result
/// for each.
///
/// This is the runner's whole job. It stops at the first thing it cannot report on
/// honestly — see [`RunnerRefused`] — and otherwise produces exactly one result per
/// scheduled check, in the schedule's order:
///
/// | what the plan says | what the mode says | what the run reports |
/// | --- | --- | --- |
/// | the check will not run | — | the plan's own `Skipped` result with its reason |
/// | it will run | the command was not admitted | the command's own `Skipped` result |
/// | it will run | it was admitted | what the runner reported, through `CommandRun::to_result` |
/// | it will run | nothing was admitted and nothing was stopped | an `Error` — nothing was observed |
///
/// Static evidence never reaches a runner at all: a [`CheckOperation::Precomputed`]
/// check is its own observation, and its result is read from the evidence by
/// [`crate::planned_work::PrecomputedEvidence::to_result`], which is the same
/// mapping the command path ends in.
///
/// # Errors
///
/// [`RunnerRefused`], for the reasons on that type. Every one of them means the
/// run produced nothing: a refusal is not a partial result set.
#[must_use = "a run's results are the whole point of running it"]
pub fn run_scheduled_checks<R>(
    schedule: &CheckSchedule,
    enforcement: &Enforcement,
    project_fingerprint: &FingerprintId,
    runner: &R,
) -> Result<RunResults, RunnerRefused>
where
    R: CommandRunner + ?Sized,
{
    // The results the mode itself produced, by the check they are about. Keyed
    // rather than searched so that the two loops below read as the two questions
    // they are rather than as a scan inside a loop.
    let stopped: BTreeMap<&CheckId, &CheckResult> = enforcement
        .stopped()
        .iter()
        .map(|result| (&result.id, result))
        .collect();

    // Every command the enforcement admitted, by the check it belongs to — and
    // counted before any work happens, so that a plan this runner cannot read is
    // refused before SURE starts anything at all.
    let mut admitted: BTreeMap<&CheckId, AdmittedCommand<'_>> = BTreeMap::new();
    for command in enforcement.admitted() {
        let id = command.command().check().id();
        if admitted.insert(id, command).is_some() {
            return Err(RunnerRefused::MoreThanOneCommandForACheck { id: id.clone() });
        }
    }

    let mut produced: Vec<CheckResult> = Vec::with_capacity(schedule.len());
    for scheduled in schedule.checks() {
        let id = scheduled.proposal().id();

        // The plan's own answer, and it is asked first: a check the mode stopped
        // keeps the plan's own stopped result and is not carried out, whatever its
        // operation holds. Without this, a precomputed check the plan refused would
        // be reported as one that ran, and `aggregate_run` would record the
        // disagreement.
        if let Some(stopped) = scheduled.not_run(project_fingerprint) {
            produced.push(stopped);
            continue;
        }

        // Then the enforcement's: the plan allowed this check, and the command it
        // holds is one the mode stopped. The reason is the command's own — nothing
        // is made up here — and this branch is what makes *what may run* a question
        // the enforcement answers even when the schedule says yes.
        if let Some(result) = stopped.get(id) {
            produced.push((*result).clone());
            continue;
        }

        if let Some(result) = carry_out(
            scheduled,
            admitted.get(id).copied(),
            project_fingerprint,
            runner,
        ) {
            produced.push(result);
        }
    }

    RunResults::assemble(schedule, &produced, project_fingerprint)
}

/// What carrying out one scheduled check produced, or `None` when it produced
/// nothing.
///
/// **`None` is a real answer and not a failure**: it means SURE has no observation
/// for this check, and what that means for the run is [`RunResults::assemble`]'s
/// rule rather than this function's. It is returned in exactly one situation — the
/// plan holds a command for the check and the enforcement neither admitted nor
/// stopped one — because that is the case where there is nothing to observe and
/// nothing to report *about* the observation.
fn carry_out<R>(
    scheduled: &ScheduledCheck,
    admitted: Option<AdmittedCommand<'_>>,
    project_fingerprint: &FingerprintId,
    runner: &R,
) -> Option<CheckResult>
where
    R: CommandRunner + ?Sized,
{
    match scheduled.operation() {
        // The answer was already observed while the plan was made. There is no
        // process, no admission and no runner between this evidence and the
        // result, which is what `CheckOperation::starts_a_process` means.
        CheckOperation::Precomputed(evidence) => {
            Some(evidence.to_result(scheduled.proposal(), project_fingerprint))
        }
        CheckOperation::Command(_) => {
            // A plan holding a command the enforcement neither admitted nor
            // stopped has nothing to observe, and `None` is that answer; what it
            // means for the run is `RunResults::assemble`'s rule.
            let command = admitted?;
            match AdmittedRun::new(scheduled, command) {
                Ok(work) => {
                    let run = runner.run(&work);
                    Some(run.to_result(scheduled.proposal(), project_fingerprint))
                }
                // The admission does not cover this check's command, so there is
                // nothing SURE may start. An `Error` for this check rather than a
                // refusal of the run: the pairing is a fact about one check, and
                // the run's other checks can still be reported honestly.
                Err(refusal) => Some(refused_result(scheduled, &refusal, project_fingerprint)),
            }
        }
        // Named rather than swallowed by a wildcard, so that a fifth kind of work
        // is a compile error here instead of a check that quietly produces nothing.
        CheckOperation::Service(_) | CheckOperation::Browser(_) => {
            Some(no_runner_result(scheduled, project_fingerprint))
        }
    }
}

/// The result for a scheduled check the runner produced nothing for.
///
/// Two answers, and which one is honest depends on what the plan said. A check the
/// plan refused keeps the plan's own stopped entry, and this is the path that
/// covers a caller that assembled results without asking the schedule what it
/// decided. A check the plan allowed that produced nothing is
/// [`NOTHING_WAS_REPORTED`], which is an `Error`.
///
/// `not_run` returning `None` *is* the question "would this check run?", so this
/// match is the rule rather than a re-derivation of it: a check that may run is
/// one SURE owes a result to, and a check that may not is one whose result the plan
/// already wrote.
fn unreported(scheduled: &ScheduledCheck, project_fingerprint: &FingerprintId) -> CheckResult {
    match scheduled.not_run(project_fingerprint) {
        Some(stopped) => stopped,
        None => {
            let proposal = scheduled.proposal();
            CheckResult::errored(
                proposal.id().clone(),
                proposal.title(),
                proposal.severity(),
                proposal.critical(),
                NOTHING_WAS_REPORTED,
                project_fingerprint.clone(),
            )
        }
    }
}

/// The result for a check whose command the admission does not cover.
///
/// `Error` and never a pass, because nothing was observed: SURE could not start
/// the command the plan holds, so what it has is a reason rather than evidence.
fn refused_result(
    scheduled: &ScheduledCheck,
    refusal: &AdmissionRefused,
    project_fingerprint: &FingerprintId,
) -> CheckResult {
    let proposal = scheduled.proposal();
    CheckResult::errored(
        proposal.id().clone(),
        proposal.title(),
        proposal.severity(),
        proposal.critical(),
        format!("SURE could not run the work this check holds: {refusal}."),
        project_fingerprint.clone(),
    )
}

/// The result for a check whose kind of work this build cannot carry out.
///
/// [`CheckOperation::Service`] and [`CheckOperation::Browser`] arrive here. The
/// status is `Error` rather than `Unknown` for the same reason
/// [`NOTHING_WAS_REPORTED`] is: the plan says the check would run, so the honest
/// report of SURE having no way to run it is a failure of SURE's own check. A
/// `Skipped` would read as a decision somebody made, and nobody decided this.
fn no_runner_result(
    scheduled: &ScheduledCheck,
    project_fingerprint: &FingerprintId,
) -> CheckResult {
    let proposal = scheduled.proposal();
    CheckResult::errored(
        proposal.id().clone(),
        proposal.title(),
        proposal.severity(),
        proposal.critical(),
        format!(
            "This build has no runner for work that is {}, so nothing about this check was \
             observed.",
            scheduled.operation().plain_description()
        ),
        project_fingerprint.clone(),
    )
}

/// A command as a person reads it, for a refusal message and for nothing else.
///
/// **This rendering is never turned back into a program and an argument vector.**
/// Splitting a command line into arguments is the act of starting a shell, and the
/// refusal this feeds is a sentence about two commands that were *not* run.
/// `PlannedCommand::display` is the same rendering for the other half of the pair
/// and carries the same note, so the two sides of the sentence are built the same
/// way and neither is parsed back.
fn display_of(spec: &CommandSpec) -> String {
    let mut line = spec.program().to_string_lossy().into_owned();
    for argument in spec.arguments() {
        line.push(' ');
        line.push_str(&argument.to_string_lossy());
    }
    line
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::ffi::OsString;
    use std::time::{Duration, SystemTime};

    use sure_domain::evidence::EvidenceClass;
    use sure_domain::execution::{ActionKind, ExecutionMode, ExecutionPermissions};
    use sure_domain::severity::Severity;
    use sure_domain::status::{CheckStatus, NotCheckedReason};

    use crate::consent::{PermissionPlan, PlannedCheck};
    use crate::planned_work::{PlannedWork, PrecomputedEvidence, Readiness, ServiceCheckSpec};
    use crate::process::{CapturedOutput, Outcome, ProcessError, Termination};
    use crate::schedule::{CheckProposal, CheckReason, PlanBuilder};

    /// What one call to a fake runner was asked to run.
    ///
    /// Structured rather than a rendered line, so that the assertion about what
    /// reached the runner is an assertion about the program and the argument
    /// vector — the two things the enforcement decided about — and not about how
    /// they happen to print.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Asked {
        id: CheckId,
        program: OsString,
        arguments: Vec<OsString>,
    }

    /// A runner with no process behind it.
    ///
    /// It records what it was asked for before it answers, so a test can assert
    /// both halves of the seam: what arrived, and what the result was made of.
    /// **No test in this file starts a process, and that is not a convenience** —
    /// these tests are about what may reach the seam, and a real run would answer a
    /// question about this machine instead.
    #[derive(Debug)]
    struct FakeRunner {
        asked: RefCell<Vec<Asked>>,
        answer: CommandRun,
    }

    impl FakeRunner {
        /// A runner that reports `answer` for every command.
        fn reporting(answer: CommandRun) -> Self {
            Self {
                asked: RefCell::new(Vec::new()),
                answer,
            }
        }

        /// A runner that reports a clean success for every command.
        fn succeeding() -> Self {
            Self::reporting(CommandRun::Ran(an_outcome(Some(0))))
        }

        /// Everything it was asked to run, in the order it was asked.
        fn asked(&self) -> Vec<Asked> {
            self.asked.borrow().clone()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, work: &AdmittedRun<'_>) -> CommandRun {
            self.asked.borrow_mut().push(Asked {
                id: work.check().clone(),
                program: work.command().program().to_os_string(),
                arguments: work.command().arguments().to_vec(),
            });
            self.answer.clone()
        }
    }

    /// An outcome the process runner could have reported, built directly rather
    /// than observed: `Outcome`'s constructors are crate-internal, which is what
    /// lets a unit test reach the mapping without a process.
    fn an_outcome(code: Option<i32>) -> Outcome {
        Outcome::new(
            OsString::from("a fake program"),
            Termination::Exited { code },
            CapturedOutput::empty(),
            CapturedOutput::empty(),
            SystemTime::now(),
            Duration::from_millis(1),
        )
    }

    fn check_id(body: &str) -> CheckId {
        CheckId::parse(format!("chk_{body}")).expect("a well-formed check id")
    }

    /// The mode and permissions this file's fixtures are built under.
    ///
    /// `HostConfirmed` with the one grant that lets a declared command run, which
    /// is the pair `tests/aggregation.rs` uses for a run that may execute the
    /// project's code. The same pair is given to the plan and to the permission
    /// plan, because a schedule built under one set of rules and enforced under
    /// another would be a fixture about a state the product cannot be in.
    fn execution() -> (ExecutionMode, ExecutionPermissions) {
        (
            ExecutionMode::HostConfirmed,
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        )
    }

    /// A check's proposal, with the operation that would carry it out.
    fn work(body: &str, title: &str, action: ActionKind, operation: CheckOperation) -> PlannedWork {
        PlannedWork::new(
            CheckProposal::new(
                check_id(body),
                title,
                Severity::MustFix,
                true,
                EvidenceClass::DeterministicCheck,
                CheckReason::ProjectWide,
                &[action],
            ),
            operation,
        )
    }

    /// The command this file's fixtures use when they need one the mode admits.
    ///
    /// `python -m pytest` is classified as running code the project controls,
    /// which the grants [`execution`] hands out cover — so a permission plan that
    /// holds it admits it. A fixture that needs a command the mode will **not**
    /// admit uses `frobnicate`, which is a program the classifier does not know:
    /// an unknown program needs a permission nobody can grant, so it is the
    /// shape of a command the mode refuses without SURE having any opinion about
    /// the program itself.
    const ADMITTED_PROGRAM: &str = "python";
    const ADMITTED_ARGUMENTS: [&str; 2] = ["-m", "pytest"];

    /// The operation for a check that runs the admitted command.
    fn runs_the_admitted_command() -> CheckOperation {
        a_command(ADMITTED_PROGRAM, &ADMITTED_ARGUMENTS)
    }

    /// The operation for a check that runs one command.
    fn a_command(program: &str, arguments: &[&str]) -> CheckOperation {
        CheckOperation::Command(
            CommandSpec::new(
                program,
                std::env::temp_dir(),
                crate::process::Environment::inherited(),
                crate::process::Limits::new(Duration::from_secs(30), 64 * 1024, 64 * 1024),
            )
            .with_arguments(arguments.iter().copied()),
        )
    }

    /// The operation for a check whose answer a detector already observed.
    fn observed(detail: &str) -> CheckOperation {
        CheckOperation::Precomputed(PrecomputedEvidence::holds(detail))
    }

    /// The operation for a check that starts a service.
    ///
    /// The command is the admitted one, so that a fixture can hold an admission
    /// for a service check too. [`AdmittedRun::new`] refuses a service by what its
    /// work *is* and refuses a mismatched command by what it *holds*, and a
    /// service fixture whose command was also unadmitted would not tell those two
    /// refusals apart — the mismatch would answer first.
    fn a_service() -> CheckOperation {
        let CheckOperation::Command(spec) = runs_the_admitted_command() else {
            panic!("the fixture builds a command");
        };
        CheckOperation::Service(ServiceCheckSpec::new(
            spec,
            Readiness::StaysUp,
            Duration::from_secs(5),
        ))
    }

    /// A schedule built from these checks, in the plan's own order.
    ///
    /// The order that comes back is the plan's, not this function's — the plan
    /// sorts by what a check would do and how heavily it weighs — so every test
    /// below looks a check up by identity rather than by position.
    fn schedule_of(work: impl IntoIterator<Item = PlannedWork>) -> CheckSchedule {
        let (mode, permissions) = execution();
        let mut builder = PlanBuilder::new(mode, permissions);
        for entry in work {
            builder
                .propose(entry)
                .expect("the fixture's proposals are well formed");
        }
        builder.build()
    }

    /// A permission plan that holds one command per `(check, program, arguments)`,
    /// decided under [`execution`].
    fn permission_plan(
        fingerprint: &FingerprintId,
        commands: &[(&str, &str, &[&str])],
    ) -> PermissionPlan {
        let (mode, permissions) = execution();
        let mut plan = PermissionPlan::new(mode, fingerprint.clone(), permissions);
        for (body, program, arguments) in commands {
            plan.add(
                PlannedCheck::new(check_id(body), "a check", Severity::MustFix, true),
                *program,
                arguments.iter().copied(),
            );
        }
        plan
    }

    /// The enforcement for a schedule and a permission plan.
    fn enforcement_of(schedule: &CheckSchedule, plan: PermissionPlan) -> Enforcement {
        Enforcement::of("P18-T006's fixture", plan, &schedule.planned_checks())
    }

    /// The status of the result for one check.
    fn status_of(results: &RunResults, body: &str) -> CheckStatus {
        results
            .get(&check_id(body))
            .expect("the run reported nothing for this check")
            .status
    }

    // ---- clause one: exactly one result per scheduled check ------------------

    #[test]
    fn every_scheduled_check_comes_back_with_exactly_one_result() {
        let fingerprint = FingerprintId::generate();
        // Four kinds of check in one plan: one already observed, one that runs an
        // admitted command, one the plan stopped, and one whose kind of work this
        // build has no runner for.
        let schedule = schedule_of([
            work(
                "observed",
                "a detector read the project",
                ActionKind::ReadFile,
                observed("it is there"),
            ),
            work(
                "runs",
                "the project's tests",
                ActionKind::RunTests,
                a_command("python", &["-m", "pytest"]),
            ),
            work(
                "stopped",
                "a check the mode stopped",
                ActionKind::RunTests,
                a_command("frobnicate", &["--everything"]),
            ),
            work(
                "served",
                "a service that answers",
                ActionKind::LocalProbe,
                a_service(),
            ),
        ]);
        let plan = permission_plan(
            &fingerprint,
            &[
                ("runs", "python", &["-m", "pytest"]),
                ("stopped", "frobnicate", &["--everything"]),
            ],
        );
        let enforcement = enforcement_of(&schedule, plan);
        let runner = FakeRunner::succeeding();

        let results = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect("nothing in this plan is ambiguous");

        assert_eq!(
            results.len(),
            schedule.len(),
            "every scheduled check has one result"
        );
        let reported: Vec<&CheckId> = results.results().iter().map(|result| &result.id).collect();
        let scheduled: Vec<&CheckId> = schedule
            .checks()
            .iter()
            .map(|scheduled| scheduled.proposal().id())
            .collect();
        assert_eq!(
            reported, scheduled,
            "the results are the schedule's checks, in the schedule's own order"
        );
        let mut seen: Vec<&CheckId> = Vec::new();
        for id in &reported {
            assert!(
                !seen.contains(id),
                "{id} was reported twice, which is the duplication this run refuses"
            );
            seen.push(id);
        }
        assert_eq!(
            runner.asked().len(),
            1,
            "exactly one check in this plan reaches the runner"
        );
    }

    #[test]
    fn two_results_for_one_check_stop_the_run() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "observed",
            "a detector read the project",
            ActionKind::ReadFile,
            observed("it is there"),
        )]);
        // Two results that claim the same check. The first is a perfectly good
        // result, which is the point: the refusal is about the duplication and
        // not about anything being wrong with either answer.
        let first = CheckResult::pass(
            check_id("observed"),
            "a detector read the project",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            fingerprint.clone(),
        );
        let again = CheckResult::errored(
            check_id("observed"),
            "a detector read the project",
            Severity::MustFix,
            true,
            "a second answer to the same question",
            fingerprint.clone(),
        );

        let refused = RunResults::assemble(&schedule, &[first, again], &fingerprint)
            .expect_err("two results for one check cannot be assembled");

        assert_eq!(
            refused,
            RunnerRefused::TwoResultsForOneCheck {
                id: check_id("observed")
            }
        );
    }

    #[test]
    fn two_commands_admitted_for_one_check_stop_the_run_before_anything_runs() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "twice",
            "a check with two commands",
            ActionKind::RunTests,
            a_command("python", &["-m", "pytest"]),
        )]);
        let plan = permission_plan(
            &fingerprint,
            &[
                ("twice", "python", &["-m", "pytest"]),
                ("twice", "python", &["-m", "pytest", "-x"]),
            ],
        );
        let enforcement = enforcement_of(&schedule, plan);
        let runner = FakeRunner::succeeding();

        let refused = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect_err("a check that is two commands cannot be carried out");

        assert_eq!(
            refused,
            RunnerRefused::MoreThanOneCommandForACheck {
                id: check_id("twice")
            }
        );
        assert!(
            runner.asked().is_empty(),
            "the refusal happens before any work, so nothing ran under a plan SURE had just found \
             unreadable"
        );
    }

    #[test]
    fn a_result_for_a_check_the_schedule_does_not_hold_is_refused() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "observed",
            "a detector read the project",
            ActionKind::ReadFile,
            observed("it is there"),
        )]);
        let stray = CheckResult::errored(
            check_id("elsewhere"),
            "a check nobody scheduled",
            Severity::MustFix,
            true,
            "a result that cannot be placed",
            fingerprint.clone(),
        );

        let refused = RunResults::assemble(&schedule, &[stray], &fingerprint)
            .expect_err("a result the schedule does not hold cannot be part of its run");

        assert_eq!(
            refused,
            RunnerRefused::ResultForACheckThatWasNotScheduled {
                id: check_id("elsewhere")
            }
        );
    }

    // ---- clause two: an admitted check that reported nothing is an error -----

    #[test]
    fn a_check_the_mode_admitted_that_reported_nothing_becomes_an_error() {
        let fingerprint = FingerprintId::generate();
        // The plan says this check runs one command. The permission plan holds no
        // command for it at all, so the enforcement neither admits one nor stops
        // one — and the runner has nothing to report.
        let schedule = schedule_of([work(
            "unreported",
            "a command nobody planned to run",
            ActionKind::RunTests,
            a_command("python", &["-m", "pytest"]),
        )]);
        assert!(
            schedule.checks()[0].may_run(),
            "the plan allowed this check, which is what makes the missing result a question SURE \
             owes an answer to"
        );
        let enforcement = enforcement_of(&schedule, permission_plan(&fingerprint, &[]));
        let runner = FakeRunner::succeeding();

        let results = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect("a missing result is reported rather than refused");

        let result = results
            .get(&check_id("unreported"))
            .expect("the check cannot be dropped out of the run");
        assert_eq!(
            result.status,
            CheckStatus::Error,
            "a check the mode admitted that produced nothing is an error, never a silence"
        );
        assert!(
            result.blocks_green(),
            "a critical check in this state must stop a green verdict"
        );
        assert_eq!(result.reason, NOTHING_WAS_REPORTED);
        assert!(
            runner.asked().is_empty(),
            "nothing was admitted, so the runner was asked nothing"
        );
    }

    #[test]
    fn a_scheduled_check_with_no_result_is_an_error_and_not_a_dropped_row() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "admitted",
            "a check the plan allowed",
            ActionKind::RunTests,
            a_command("python", &["-m", "pytest"]),
        )]);

        let results = RunResults::assemble(&schedule, &[], &fingerprint)
            .expect("an empty result set is assembled rather than refused");

        assert_eq!(results.len(), 1, "the check is a row and not an absence");
        let result = &results.results()[0];
        assert_eq!(result.id, check_id("admitted"));
        assert_eq!(result.status, CheckStatus::Error);
        assert!(result.blocks_green());
        assert_eq!(result.reason, NOTHING_WAS_REPORTED);
    }

    #[test]
    fn a_check_the_plan_stopped_keeps_the_plans_own_stopped_result() {
        let fingerprint = FingerprintId::generate();
        let (mode, permissions) = (
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        );
        let mut builder = PlanBuilder::new(mode, permissions);
        builder
            .propose(work(
                "blocked",
                "a check the mode stopped",
                ActionKind::RunTests,
                a_command("python", &["-m", "pytest"]),
            ))
            .expect("the proposal is well formed");
        let schedule = builder.build();
        assert!(
            !schedule.checks()[0].may_run(),
            "the fixture is about a check that will not run"
        );

        let results = RunResults::assemble(&schedule, &[], &fingerprint)
            .expect("a stopped check needs no result from a runner");

        assert_eq!(results.len(), 1);
        let result = &results.results()[0];
        assert_eq!(
            result.status,
            CheckStatus::Skipped,
            "a check the plan stopped keeps the plan's own answer"
        );
        assert_eq!(
            result.not_checked_reason,
            Some(NotCheckedReason::ExecutionNotAuthorized)
        );
    }

    // ---- clause three: an unadmitted command cannot reach the runner ---------

    #[test]
    fn a_command_the_enforcement_did_not_admit_never_reaches_the_runner() {
        let fingerprint = FingerprintId::generate();
        // Both checks may run as far as the *plan* is concerned — their actions
        // need nothing this mode withholds. One of them holds a command the mode
        // will not admit, and that is the whole of the difference.
        let schedule = schedule_of([
            work(
                "admitted",
                "the project's tests",
                ActionKind::RunTests,
                a_command("python", &["-m", "pytest"]),
            ),
            work(
                "unadmitted",
                "a command the mode will not admit",
                ActionKind::ReadFile,
                a_command("frobnicate", &["--everything"]),
            ),
        ]);
        let plan = permission_plan(
            &fingerprint,
            &[
                ("admitted", "python", &["-m", "pytest"]),
                ("unadmitted", "frobnicate", &["--everything"]),
            ],
        );
        let enforcement = enforcement_of(&schedule, plan);
        assert_eq!(
            enforcement.stopped().len(),
            1,
            "the enforcement stopped exactly the command it did not admit"
        );
        let runner = FakeRunner::succeeding();

        let results = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect("a stopped command is a result and not a refusal");

        let asked = runner.asked();
        assert_eq!(
            asked.len(),
            1,
            "only the admitted command reached the runner, found: {asked:?}"
        );
        assert_eq!(asked[0].id, check_id("admitted"));
        assert_eq!(asked[0].program, OsString::from("python"));
        assert_eq!(
            asked[0].arguments,
            vec![OsString::from("-m"), OsString::from("pytest")]
        );
        assert_eq!(
            status_of(&results, "unadmitted"),
            CheckStatus::Skipped,
            "the check whose command was not admitted keeps the command's own stopping"
        );
        assert_eq!(status_of(&results, "admitted"), CheckStatus::Pass);
    }

    #[test]
    fn an_admission_cannot_be_paired_with_another_checks_command() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([
            work(
                "mine",
                "a check that runs a command",
                ActionKind::RunTests,
                runs_the_admitted_command(),
            ),
            work(
                "theirs",
                "another check that runs a command",
                ActionKind::RunTests,
                a_command(ADMITTED_PROGRAM, &["-m", "pytest", "-x"]),
            ),
            work("served", "a service", ActionKind::LocalProbe, a_service()),
        ]);
        let plan = permission_plan(
            &fingerprint,
            &[
                ("mine", ADMITTED_PROGRAM, &ADMITTED_ARGUMENTS),
                ("theirs", ADMITTED_PROGRAM, &ADMITTED_ARGUMENTS),
                ("served", ADMITTED_PROGRAM, &ADMITTED_ARGUMENTS),
            ],
        );
        let enforcement = enforcement_of(&schedule, plan);
        let admitted: BTreeMap<&CheckId, AdmittedCommand<'_>> = enforcement
            .admitted()
            .map(|command| (command.command().check().id(), command))
            .collect();

        // The pair that really is one command.
        let mine = schedule.get(&check_id("mine")).expect("scheduled");
        let work = AdmittedRun::new(mine, admitted[&check_id("mine")])
            .expect("the plan and the enforcement agree about this command");
        assert_eq!(work.check(), &check_id("mine"));
        assert_eq!(work.command().arguments().len(), 2);

        // The same admission, paired with another check's work.
        let theirs = schedule.get(&check_id("theirs")).expect("scheduled");
        let refused = AdmittedRun::new(theirs, admitted[&check_id("mine")])
            .expect_err("an admission is about one check");
        assert_eq!(
            refused,
            AdmissionRefused::ForADifferentCheck {
                id: check_id("theirs"),
                admitted_for: check_id("mine"),
            }
        );

        // An admission paired with work that is not one command at all.
        let served = schedule.get(&check_id("served")).expect("scheduled");
        let refused = AdmittedRun::new(served, admitted[&check_id("served")])
            .expect_err("a service is not one command");
        assert_eq!(
            refused,
            AdmissionRefused::NotOneCommand {
                id: check_id("served"),
                work: "started, asked one question, and stopped",
            }
        );
    }

    #[test]
    fn an_admission_that_covers_a_different_command_is_refused() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "differs",
            "a check whose command and admission disagree",
            ActionKind::RunTests,
            a_command("python", &["-m", "pytest"]),
        )]);
        // The same program, a different argument vector: the case a comparison of
        // rendered command lines would let through.
        let plan = permission_plan(
            &fingerprint,
            &[("differs", "python", &["-m", "pytest", "-x"])],
        );
        let enforcement = enforcement_of(&schedule, plan);
        let admitted = enforcement
            .admitted()
            .next()
            .expect("one command was admitted");
        let scheduled = schedule.get(&check_id("differs")).expect("scheduled");

        let refused = AdmittedRun::new(scheduled, admitted)
            .expect_err("the decision was about a different argument vector");

        match refused {
            AdmissionRefused::NotTheCommandThatWasAdmitted {
                id,
                planned,
                admitted,
            } => {
                assert_eq!(id, check_id("differs"));
                assert_eq!(planned, "python -m pytest");
                assert_eq!(admitted, "python -m pytest -x");
            }
            other => panic!("the refusal names the wrong reason: {other}"),
        }
    }

    // ---- the translation, field for field -----------------------------------

    #[test]
    fn the_request_is_the_spec_field_for_field() {
        let fingerprint = FingerprintId::generate();
        // The program and the arguments are the ones the mode admits, because
        // those are the only ones an admission can be paired with. Everything
        // else the spec holds is free to differ, and this fixture makes them
        // differ so that the assertions below can only pass if each field came
        // from the spec.
        let spec = CommandSpec::new(
            ADMITTED_PROGRAM,
            std::env::temp_dir().join("sure-fixture"),
            crate::process::Environment::inherited()
                .without("CARGO_TARGET_DIR")
                .with("CARGO_TERM_COLOR", "never"),
            crate::process::Limits::new(Duration::from_secs(11), 1024, 2048),
        )
        .with_arguments(ADMITTED_ARGUMENTS);
        let schedule = schedule_of([work(
            "translated",
            "a check whose request is compared against its spec",
            ActionKind::RunTests,
            CheckOperation::Command(spec.clone()),
        )]);
        let plan = permission_plan(
            &fingerprint,
            &[("translated", ADMITTED_PROGRAM, &ADMITTED_ARGUMENTS)],
        );
        let enforcement = enforcement_of(&schedule, plan);
        let admitted = enforcement
            .admitted()
            .next()
            .expect("one command was admitted");
        let scheduled = schedule.get(&check_id("translated")).expect("scheduled");
        let work = AdmittedRun::new(scheduled, admitted).expect("the two agree");

        let request = work.request(&Cancellation::new());

        assert_eq!(request.program(), spec.program());
        assert_eq!(request.arguments(), spec.arguments());
        assert_eq!(request.working_directory(), spec.working_directory());
        assert_eq!(request.limits(), spec.limits());
        assert_eq!(request.environment(), spec.environment());
    }

    #[test]
    fn an_argument_holding_a_space_stays_one_argument() {
        let fingerprint = FingerprintId::generate();
        // Three arguments, two of which no shell-free layer may touch: one holds
        // a space, and one holds a quote. The module flag keeps the classifier
        // reading these as arguments of the operation rather than as the
        // operation, and nothing else about them matters here.
        let spec = CommandSpec::new(
            ADMITTED_PROGRAM,
            std::env::temp_dir(),
            crate::process::Environment::inherited(),
            crate::process::Limits::new(Duration::from_secs(5), 1024, 1024),
        )
        .with_arguments(["tests/a file with spaces", "it's", "a\\b"]);
        let schedule = schedule_of([work(
            "spaces",
            "a check with an argument holding a space",
            ActionKind::RunTests,
            CheckOperation::Command(spec.clone()),
        )]);
        let plan = permission_plan(
            &fingerprint,
            &[(
                "spaces",
                ADMITTED_PROGRAM,
                &["tests/a file with spaces", "it's", "a\\b"],
            )],
        );
        let enforcement = enforcement_of(&schedule, plan);
        let admitted = enforcement
            .admitted()
            .next()
            .expect("one command was admitted");
        let scheduled = schedule.get(&check_id("spaces")).expect("scheduled");
        let work = AdmittedRun::new(scheduled, admitted).expect("the two agree");

        let request = work.request(&Cancellation::new());

        assert_eq!(
            request.arguments().len(),
            3,
            "three arguments went in and three came out: nothing is split and nothing is joined"
        );
        assert_eq!(
            request.arguments(),
            [
                OsString::from("tests/a file with spaces"),
                OsString::from("it's"),
                OsString::from("a\\b"),
            ],
            "each argument arrived as itself, with the space and the quote still inside it"
        );
        assert_eq!(request.arguments(), spec.arguments());
        for argument in request.arguments() {
            let text = argument.to_string_lossy();
            assert!(
                !text.starts_with('"') && !text.starts_with('\''),
                "{text:?} was quoted on the way through, which no layer here may do"
            );
        }
    }

    // ---- what the runner reported, as the check's result ---------------------

    #[test]
    fn a_command_that_ran_and_reported_success_passes_the_check() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "passes",
            "the project's tests",
            ActionKind::RunTests,
            a_command("python", &["-m", "pytest"]),
        )]);
        let plan = permission_plan(&fingerprint, &[("passes", "python", &["-m", "pytest"])]);
        let enforcement = enforcement_of(&schedule, plan);
        let runner = FakeRunner::reporting(CommandRun::Ran(an_outcome(Some(0))));

        let results = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect("nothing here is ambiguous");

        assert_eq!(status_of(&results, "passes"), CheckStatus::Pass);
    }

    #[test]
    fn a_command_that_never_started_makes_the_check_an_error() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "missing",
            "a check whose program is not installed",
            ActionKind::RunTests,
            a_command("python", &["-m", "pytest"]),
        )]);
        let plan = permission_plan(&fingerprint, &[("missing", "python", &["-m", "pytest"])]);
        let enforcement = enforcement_of(&schedule, plan);
        let runner = FakeRunner::reporting(CommandRun::NeverStarted(ProcessError::NotStarted {
            program: OsString::from("python"),
            working_directory: std::env::temp_dir(),
            message: "the system cannot find the file specified".to_owned(),
        }));

        let results = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect("a command that did not start is a result and not a refusal");

        let result = results.get(&check_id("missing")).expect("reported");
        assert_eq!(result.status, CheckStatus::Error);
        assert!(
            result
                .reason
                .contains("the system cannot find the file specified"),
            "the operating system's own words are what a reader needs: {}",
            result.reason
        );
        assert!(result.blocks_green());
    }

    #[test]
    fn a_check_this_build_has_no_runner_for_is_an_error_and_not_a_silence() {
        let fingerprint = FingerprintId::generate();
        let schedule = schedule_of([work(
            "served",
            "a service this build cannot start",
            ActionKind::LocalProbe,
            a_service(),
        )]);
        let enforcement = enforcement_of(&schedule, permission_plan(&fingerprint, &[]));
        let runner = FakeRunner::succeeding();

        let results = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect("a kind of work this build cannot carry out is a result");

        let result = results.get(&check_id("served")).expect("reported");
        assert_eq!(result.status, CheckStatus::Error);
        assert!(result.blocks_green());
        assert!(
            result
                .reason
                .contains("started, asked one question, and stopped"),
            "the reason names the work this build has no runner for: {}",
            result.reason
        );
        assert!(
            runner.asked().is_empty(),
            "a service is not carried out by the command runner"
        );
    }

    #[test]
    fn a_permission_the_mode_did_not_grant_keeps_a_check_from_the_runner() {
        let fingerprint = FingerprintId::generate();
        // The action needs a permission this mode does not grant, so the plan
        // itself refuses the check: it never reaches a decision about a command.
        let (mode, permissions) = (
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        );
        let mut builder = PlanBuilder::new(mode, permissions);
        builder
            .propose(work(
                "networked",
                "a check the mode will not run",
                ActionKind::NetworkAccess,
                a_command("git", &["fetch"]),
            ))
            .expect("the proposal is well formed");
        let schedule = builder.build();
        let plan = permission_plan(&fingerprint, &[("networked", "git", &["fetch"])]);
        let enforcement = enforcement_of(&schedule, plan);
        let runner = FakeRunner::succeeding();

        let results = run_scheduled_checks(&schedule, &enforcement, &fingerprint, &runner)
            .expect("the plan's own refusal is a result and not a failure");

        assert_eq!(status_of(&results, "networked"), CheckStatus::Skipped);
        assert_eq!(
            results
                .get(&check_id("networked"))
                .map(|result| result.not_checked_reason),
            Some(Some(NotCheckedReason::ExecutionNotAuthorized))
        );
        assert!(
            runner.asked().is_empty(),
            "a check the plan refused is not carried out, whatever command it holds"
        );
    }
}

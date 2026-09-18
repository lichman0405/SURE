//! The check pipeline: from a project directory to a verdict.
//!
//! `docs/architecture/CHECK_PIPELINE.md` names twelve stages, and this module is
//! the thing that runs them, in that order, for one project. **It implements no
//! stage.** Every stage is a call into a module an earlier phase built and
//! tested — [`crate::discover`], [`crate::project_intent`] and
//! [`crate::intent_model`], [`crate::fingerprint`], [`crate::schedule`],
//! [`crate::checks`] and [`crate::runtime_probes`], the five completeness
//! scanners, [`crate::analysis_provider`], [`crate::claim_checker`],
//! [`crate::aggregation`], [`crate::coverage_summary`],
//! [`crate::project_verdict`] and [`crate::recheck_lifecycle`] — and composed
//! through [`crate::false_completion_aggregator`]. What is written here is the
//! sequencing, the per-stage record, and the rule that decides what all of it
//! means together.
//!
//! # The rule this module exists to keep
//!
//! **A stage that did not run is never a stage that passed.** Every one of the
//! twelve stages gets a [`StageRecord`], and a stage that had work and did not do
//! it takes one of three forms, none of which is silence:
//!
//! - [`StageOutcome::NotPartOfWork`] — this run does not ask for the stage. A
//!   `sure check` produces no repair contract, and a stage SURE was never asked
//!   to perform is not a stage that failed to run.
//! - [`StageOutcome::NotRun`] — the stage had work to do and did not do it, with
//!   the reason the domain's own vocabulary gives where the vocabulary has a word
//!   for it. Every such stage also contributes a [`CheckResult`] to the
//!   aggregate, so the verdict is degraded by the gap rather than merely
//!   accompanied by a note about it.
//! - [`StageOutcome::Unfinished`] — the stage tried and could not finish. The run
//!   has no verdict at all, and [`PipelineOutcome::run`] is `None`.
//!
//! [`PipelineOutcome::is_green`] asks two questions and requires both: the
//! verdict permits hand-off by [`ProjectVerdict`]'s own rule, **and** no stage in
//! this run's range is anything other than [`StageOutcome::Ran`]. The second is
//! not a restatement of the first — a stage whose gap produced no `CheckResult`,
//! because it had nothing checkable to record a gap against, would be invisible
//! to the aggregate and visible here. A reviewer can see both.
//!
//! # What runs, and what cannot
//!
//! This build plans every check the discovery supports and **runs none of them**.
//! There is no runner for a planned check: the only one in the tree is
//! [`crate::runtime_start::StartSmoke`], and reaching it from a product path is
//! what `tests/spawn_sites.rs`'s census forbids until `sure_core::support`'s
//! ceiling moves, which is a different task's decision. So a check the execution
//! mode refuses is recorded as the plan's own
//! [`ScheduledCheck::not_run`](crate::schedule::ScheduledCheck::not_run), and a
//! check the mode permits is recorded by [`crate::aggregation::aggregate_run`] as
//! `unknown` — a check the plan said would run and that reported nothing. Neither
//! is a pass, and [`StageOutcome::NotRun`] on stages 5 and 6 says so in the run's
//! own words.
//!
//! # The store
//!
//! The pipeline reads a store when it is given one and never opens one itself.
//! That is deliberate: opening a store creates its directory and its file, and a
//! check that created a history in order to report that it had nothing to compare
//! against would be a command changing the thing it is describing. `sure doctor`
//! states the same rule for the same reason.

use std::path::Path;

use sure_domain::capability::CapabilityReport;
use sure_domain::evidence::ClaimAssessment;
use sure_domain::execution::{ExecutionMode, ExecutionPermissions};
use sure_domain::ids::{CheckId, ClaimId, FingerprintId};
use sure_domain::intent::{IntentSource, ProjectIntent};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, NotCheckedReason};
use sure_domain::vocabulary::{Claim, ProjectFingerprint, ProjectSupport, ProjectVerdict};

use crate::aggregation::{RunReport, aggregate_run};
use crate::analysis_provider;
use crate::candidate_scanner::CandidateScanner;
use crate::checks::{self, check_id};
use crate::claim_checker::{self, CheckedClaim};
use crate::components::ComponentGraph;
use crate::config::AnalysisProvider;
use crate::config::Config;
use crate::config::ExecutionSettings;
use crate::coverage_summary::{CoverageNotCheckedSummary, summarize};
use crate::demo_data_heuristics::DemoDataHeuristics;
use crate::discover::{self, DiscoverOptions, Discovery, Ecosystem, Findings};
use crate::false_completion_aggregator;
use crate::fingerprint::{self, project_fingerprint};
use crate::intent_implementation::compare_intent_to_project;
use crate::intent_model;
use crate::noop_heuristics::NoOpHeuristics;
use crate::project_intent;
use crate::project_verdict::build_verdict;
use crate::recheck_lifecycle::{self, LifecycleInputs, LifecycleUpdate};
use crate::route_consistency::RouteConsistency;
use crate::runtime_probes::{PlanRefused, ProbePlan};
use crate::schedule::{CheckProposal, CheckSchedule, PlanBuilder, ProposalRefused, ScheduledCheck};
use crate::store::Store;
use crate::support;
use crate::ui_action_bridge::UiActionBridge;

/// What SURE was asked to do with the pipeline.
///
/// The three commands on this surface that reach the engine, and the reason they
/// are a value rather than three entry points: they differ in **how far down the
/// twelve stages they go** and in nothing else. They share the discovery, the
/// intent, the fingerprint, the plan, the checks, the aggregation and the
/// verdict, so a second entry point would be a second copy of all of that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purpose {
    /// `sure check`: stages 1 to 10.
    Check,
    /// `sure repair`: stages 1 to 11. The same check, plus the repair contract.
    Repair,
    /// `sure recheck`: all twelve. The same check, plus the comparison with what
    /// the last run left open.
    Recheck,
}

impl Purpose {
    /// The stable name, matching the command a user typed.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Repair => "repair",
            Self::Recheck => "recheck",
        }
    }

    /// The last stage this purpose performs.
    ///
    /// Stated as an endpoint rather than as a list, because the range is
    /// contiguous: a run performs stages `1..=this` and no others, and a table of
    /// two ranges is a table somebody has to keep in step with the enum. The
    /// stages after it are [`StageOutcome::NotPartOfWork`] and are recorded rather
    /// than omitted, so a reader of the log sees the whole documented pipeline
    /// whatever the command was.
    #[must_use]
    pub const fn last_stage(self) -> Stage {
        match self {
            Self::Check => Stage::Aggregate,
            Self::Repair => Stage::RepairContract,
            Self::Recheck => Stage::Recheck,
        }
    }

    /// Whether this purpose performs the repair-contract stage.
    #[must_use]
    pub const fn wants_repair_contracts(self) -> bool {
        matches!(self, Self::Repair | Self::Recheck)
    }

    /// Whether this purpose performs the re-check stage.
    #[must_use]
    pub const fn wants_recheck(self) -> bool {
        matches!(self, Self::Recheck)
    }
}

/// One stage of `docs/architecture/CHECK_PIPELINE.md`.
///
/// The variants are in the order that document lists them, and [`Self::ALL`] is
/// that order. A stage's number is its position, so a report can print "3/12"
/// without a second table to keep in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stage {
    /// 1. Discover the project, its stacks, components and declared commands.
    Discover,
    /// 2. Resolve intent from explicit and documented sources.
    ResolveIntent,
    /// 3. Fingerprint the current relevant project state.
    Fingerprint,
    /// 4. Plan static and dynamic checks and their execution trust.
    Plan,
    /// 5. Static deterministic checks that do not execute project code.
    StaticChecks,
    /// 6. Approved dynamic checks, under the selected execution mode.
    DynamicChecks,
    /// 7. Completeness analysis: mocks, stubs, fake success, dead glue.
    Completeness,
    /// 8. Grounded model assessment, if a provider is configured.
    ModelAssessment,
    /// 9. Claim checking against observed evidence and the current state.
    ClaimChecking,
    /// 10. Aggregate into findings, scope and a verdict.
    Aggregate,
    /// 11. Repair contract for the selected findings.
    RepairContract,
    /// 12. Re-check affected and regression checks after a repair.
    Recheck,
}

impl Stage {
    /// Every stage, in the documented order.
    pub const ALL: &'static [Self] = &[
        Self::Discover,
        Self::ResolveIntent,
        Self::Fingerprint,
        Self::Plan,
        Self::StaticChecks,
        Self::DynamicChecks,
        Self::Completeness,
        Self::ModelAssessment,
        Self::ClaimChecking,
        Self::Aggregate,
        Self::RepairContract,
        Self::Recheck,
    ];

    /// The stage's number in the documented order, counting from one.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Discover => 1,
            Self::ResolveIntent => 2,
            Self::Fingerprint => 3,
            Self::Plan => 4,
            Self::StaticChecks => 5,
            Self::DynamicChecks => 6,
            Self::Completeness => 7,
            Self::ModelAssessment => 8,
            Self::ClaimChecking => 9,
            Self::Aggregate => 10,
            Self::RepairContract => 11,
            Self::Recheck => 12,
        }
    }

    /// The stage's name, spelled as the architecture document spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discover => "discover",
            Self::ResolveIntent => "resolve-intent",
            Self::Fingerprint => "fingerprint",
            Self::Plan => "plan",
            Self::StaticChecks => "static-checks",
            Self::DynamicChecks => "dynamic-checks",
            Self::Completeness => "completeness",
            Self::ModelAssessment => "model-assessment",
            Self::ClaimChecking => "claim-checking",
            Self::Aggregate => "aggregate",
            Self::RepairContract => "repair-contract",
            Self::Recheck => "recheck",
        }
    }

    /// The title a report shows for this stage.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Discover => "Find the project's parts",
            Self::ResolveIntent => "Work out what was asked for",
            Self::Fingerprint => "Take the project's fingerprint",
            Self::Plan => "Decide what to check",
            Self::StaticChecks => "Read the project and check it",
            Self::DynamicChecks => "Run the project's own checks",
            Self::Completeness => "Look for unfinished work",
            Self::ModelAssessment => "Ask a model to assess the project",
            Self::ClaimChecking => "Check what was claimed against the evidence",
            Self::Aggregate => "Work out the verdict",
            Self::RepairContract => "Write instructions for a repair",
            Self::Recheck => "Compare with what the last run found",
        }
    }

    /// The identifier SURE uses for a result this stage contributes.
    ///
    /// Deterministic, through [`crate::checks::check_id`], so that the same stage
    /// in the same project is the same check on every run. A generated identifier
    /// would make two runs' results for one stage look like two stages.
    #[must_use]
    pub fn id(self) -> CheckId {
        check_id("sure.pipeline", self.as_str())
    }
}

/// What happened to one stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageOutcome {
    /// The stage did its work for this run.
    Ran {
        /// What it did, in one line a person reads.
        detail: String,
    },
    /// This run does not ask for the stage.
    ///
    /// Not a gap: nothing was asked of the stage, so there is nothing that failed
    /// to happen. Recorded rather than omitted so that the log shows the whole
    /// documented pipeline, and deliberately not turned into a [`CheckResult`]: a
    /// result saying "not checked" about a stage nobody asked for would be a gap
    /// SURE invented.
    NotPartOfWork {
        /// Why this run does not ask for it.
        detail: String,
    },
    /// The stage had work to do and did not run it.
    ///
    /// `reason` is the domain's own vocabulary where the vocabulary has a word for
    /// it, so the sentence a user reads is not invented here; `None` when the
    /// honest answer is in `detail` and no variant of [`NotCheckedReason`] says
    /// it.
    NotRun {
        /// Why, in the vocabulary's terms when it has them.
        reason: Option<NotCheckedReason>,
        /// What did not happen, in plain language.
        detail: String,
    },
    /// The stage tried and could not finish.
    ///
    /// The run has no verdict. This is the only outcome that stops the pipeline: a
    /// stage that cannot finish leaves the stages after it without an input, and
    /// carrying on would produce a report about a project SURE never got to look
    /// at.
    Unfinished {
        /// What stopped it.
        detail: String,
    },
}

impl StageOutcome {
    /// Whether the stage did its work.
    #[must_use]
    pub const fn ran(&self) -> bool {
        matches!(self, Self::Ran { .. })
    }

    /// Whether SURE has to say that this stage did not happen.
    ///
    /// `NotPartOfWork` is not a gap — see [`StageOutcome`] — and `Ran` is not one
    /// either. Everything else is.
    #[must_use]
    pub const fn is_a_gap(&self) -> bool {
        matches!(self, Self::NotRun { .. } | Self::Unfinished { .. })
    }

    /// The sentence a report prints for this outcome.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Ran { detail }
            | Self::NotPartOfWork { detail }
            | Self::NotRun { detail, .. }
            | Self::Unfinished { detail } => detail,
        }
    }
}

/// One stage and what happened to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRecord {
    /// Which stage.
    pub stage: Stage,
    /// What happened.
    pub outcome: StageOutcome,
}

impl StageRecord {
    /// The stage, its number and what happened, as a line a person reads.
    #[must_use]
    pub fn plain_description(&self) -> String {
        let marker = match &self.outcome {
            StageOutcome::Ran { .. } => "",
            StageOutcome::NotPartOfWork { .. } => " (not part of this run)",
            StageOutcome::NotRun { .. } => " (NOT CHECKED)",
            StageOutcome::Unfinished { .. } => " (STOPPED HERE)",
        };
        format!(
            "{}/{}. {}: {}{marker}",
            self.stage.number(),
            Stage::ALL.len(),
            self.stage.title(),
            self.outcome.detail(),
        )
    }
}

/// One completeness candidate, as a report reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateSummary {
    /// The candidate's stable identity.
    pub id: String,
    /// The short human title.
    pub title: String,
    /// How bad it would be if the check this describes did not pass.
    pub severity: Severity,
    /// Whether the project cannot be trusted for hand-off if it does not.
    pub critical: bool,
    /// Why this candidate is proposed, in plain language.
    pub because: String,
}

/// The completeness candidates stage 7 found.
///
/// Carried rather than folded into the findings, because a candidate is not a
/// defect: [`crate::candidate_scanner`]'s own acceptance is that candidates "are
/// not automatically product defects", and turning a `TODO` comment into a finding
/// with a severity would be this module inventing a judgement no phase made. What
/// the run reports is what was seen — which is what a reader needs in order to
/// decide.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Candidates {
    /// Candidates worth acting on, ordered by their identifiers.
    pub material: Vec<CandidateSummary>,
    /// Candidates that are style noise by [`crate::false_completion_aggregator`]'s
    /// own rule.
    pub style_noise: Vec<CandidateSummary>,
    /// Candidates dropped because another proposal at the same anchor was more
    /// serious.
    pub duplicates_dropped: usize,
}

impl Candidates {
    /// Whether stage 7 saw anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.material.is_empty() && self.style_noise.is_empty()
    }

    /// Every candidate, material first.
    #[must_use]
    pub fn total(&self) -> usize {
        self.material.len() + self.style_noise.len()
    }
}

/// Everything a run that finished produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    /// The project, as text, for a record or a report.
    pub project_root: String,
    /// The execution mode every dynamic check was decided under.
    pub mode: ExecutionMode,
    /// The permissions that mode was granted for this run.
    pub permissions: ExecutionPermissions,
    /// The project state everything in this outcome is about.
    pub project_state: ProjectFingerprint,
    /// What SURE could and could not see, from discovery.
    pub support: ProjectSupport,
    /// The intent the run compared the project against.
    pub intent: ProjectIntent,
    /// What the run planned to check, and what became of each entry.
    pub schedule: CheckSchedule,
    /// Every planned check's result, and every way a critical check contributed.
    pub report: RunReport,
    /// The checked and not-checked scope, in plain language.
    pub coverage: CoverageNotCheckedSummary,
    /// The intent, evidence and scope assembled into one value.
    pub verdict: ProjectVerdict,
    /// What the completeness scanners found.
    pub candidates: Candidates,
    /// What the project's agent claimed, and whether it could be confirmed.
    pub claims: Vec<Claim>,
    /// The caveat the report must carry when SURE may not compare the project
    /// against what the user asked for.
    pub intent_caveat: Option<&'static str>,
    /// One repair contract per finding, when the purpose asks for them.
    pub repairs: Vec<sure_domain::vocabulary::RepairContract>,
    /// What the comparison with the previous run found, for a re-check.
    pub lifecycle: Option<LifecycleUpdate>,
}

impl RunOutcome {
    /// Every check that did not run, in plan order.
    #[must_use]
    pub fn not_checked(&self) -> Vec<&CheckResult> {
        self.report
            .results()
            .iter()
            .filter(|result| result.is_not_checked())
            .collect()
    }
}

/// The result of running the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineOutcome {
    /// What SURE was asked to do.
    pub purpose: Purpose,
    /// Every stage, in order, and what happened to each.
    pub stages: Vec<StageRecord>,
    /// What the run found, or `None` when it did not finish.
    pub run: Option<RunOutcome>,
    /// The stage that stopped the run, when one did.
    pub stopped_at: Option<Stage>,
}

impl PipelineOutcome {
    /// What happened to one stage.
    ///
    /// Every stage has exactly one record by construction, so this is total. The
    /// fallback to the first record is unreachable and is there rather than an
    /// index expression because this workspace forbids `panic` in shipped code: a
    /// wrong answer under a test is a better failure than a crash in a terminal.
    #[must_use]
    pub fn stage(&self, stage: Stage) -> &StageRecord {
        let position = stage.number() as usize - 1;
        self.stages.get(position).unwrap_or(&self.stages[0])
    }

    /// Whether every stage this run asked for did its work.
    ///
    /// The second half of [`Self::is_green`], exposed on its own so that a test —
    /// and a reader — can see which of the two questions failed.
    #[must_use]
    pub fn every_stage_ran(&self) -> bool {
        self.stages.iter().all(|record| !record.outcome.is_a_gap())
    }

    /// Every stage SURE has to say did not happen.
    pub fn gaps(&self) -> impl Iterator<Item = &StageRecord> {
        self.stages
            .iter()
            .filter(|record| record.outcome.is_a_gap())
    }

    /// Whether the run finished.
    #[must_use]
    pub const fn finished(&self) -> bool {
        self.run.is_some()
    }

    /// Whether this run may be reported as a clean one.
    ///
    /// Two independent questions, and both have to be yes:
    ///
    /// 1. the verdict permits hand-off, by [`ProjectVerdict`]'s own rule — a green
    ///    aggregate with no open finding that blocks hand-off;
    /// 2. no stage in this run's range failed to do its work.
    ///
    /// The second is not a restatement of the first. A stage whose gap produced no
    /// `CheckResult` — because it had nothing checkable to record a gap against —
    /// would be invisible to the aggregate and visible here.
    #[must_use]
    pub fn is_green(&self) -> bool {
        self.run
            .as_ref()
            .is_some_and(|run| run.verdict.is_ready_for_hand_off() && self.every_stage_ran())
    }
}

/// One run of the pipeline for one project.
///
/// Built rather than called as a function with six arguments, because the inputs
/// are named at the call site and a caller that got the order wrong would
/// otherwise compile. Every field is public and every one is an input: the type
/// holds no state between [`Self::run`] calls, and running it twice is running it
/// twice rather than continuing it.
#[derive(Debug)]
pub struct Pipeline<'a> {
    /// The project directory SURE was pointed at.
    pub project: &'a Path,
    /// What SURE was asked to do.
    pub purpose: Purpose,
    /// The settings in effect, already read from the project.
    pub config: &'a Config,
    /// What SURE may do, and how, already arbitrated between the two files.
    ///
    /// Not read from [`Self::config`] here, and that is the point: this run
    /// plans checks under the mode and the permissions a *hook* would decide
    /// under, and the authority layer is the one place either is worked out
    /// (`Authority::execution`). A stage 4 that read `config.execution` would
    /// plan under whatever the project's own file asked for, including on a
    /// machine whose user never allowed any of it — the defect P13-T009 exists
    /// to close, in the one place `check` can observe it.
    pub execution: ExecutionSettings,
    /// The history to compare against, when there is one.
    ///
    /// Never opened here: see the module comment.
    pub store: Option<&'a Store>,
    /// The goal the user stated on the command line, if they stated one.
    ///
    /// It is here rather than only in the store because stage 2 resolves intent
    /// from it in the same run that records it — a run that wrote a goal and then
    /// checked against a different intent would be recording something it did not
    /// use.
    pub goal: Option<&'a str>,
}

/// A stage that stopped the run.
struct Stopped {
    stage: Stage,
    detail: String,
}

/// What SURE says about a stage it never reached.
const STOPPED_BEFORE: &str = "the run stopped before reaching this stage.";

/// The stage records, kept in order as they are produced.
struct Stages {
    records: Vec<StageRecord>,
}

impl Stages {
    fn new() -> Self {
        Self {
            records: Vec::with_capacity(Stage::ALL.len()),
        }
    }

    fn push(&mut self, stage: Stage, outcome: StageOutcome) {
        debug_assert_eq!(
            self.records.len() + 1,
            stage.number() as usize,
            "stages are recorded once each and in order"
        );
        self.records.push(StageRecord { stage, outcome });
    }
}

/// The proposals stage 4 gathered, and the ones that could not be planned.
#[derive(Default)]
struct Planned {
    /// Declared commands the project does not have, which are checks SURE could
    /// not plan and must record as not-checked rather than drop.
    missing: Vec<checks::MissingCommand>,
    /// How many runtime probes the project's shape ruled out.
    not_planned: usize,
    /// Why the probe planner refused, when it did.
    probe_refusal: Option<PlanRefused>,
}

impl Pipeline<'_> {
    /// Run the twelve stages, in order, as far as `purpose` asks.
    #[must_use]
    pub fn run(&self) -> PipelineOutcome {
        let mut stages = Stages::new();

        // ---- 1. Discover -----------------------------------------------------
        let discovery = match discover::discover(self.project, &DiscoverOptions::default()) {
            Ok(discovery) => discovery,
            Err(error) => {
                return self.stopped_at(
                    stages,
                    Stopped {
                        stage: Stage::Discover,
                        detail: error.to_string(),
                    },
                );
            }
        };
        let support = support::classify(&discovery);
        stages.push(
            Stage::Discover,
            StageOutcome::Ran {
                detail: describe_discovery(&discovery, &support),
            },
        );

        // ---- 2. Resolve intent -----------------------------------------------
        let intent = self.resolve_intent();
        stages.push(
            Stage::ResolveIntent,
            StageOutcome::Ran {
                detail: describe_intent(&intent),
            },
        );

        // ---- 3. Fingerprint --------------------------------------------------
        let state =
            match project_fingerprint(self.project, &fingerprint::FingerprintOptions::default()) {
                Ok(state) => state,
                Err(error) => {
                    return self.stopped_at(
                        stages,
                        Stopped {
                            stage: Stage::Fingerprint,
                            detail: error.to_string(),
                        },
                    );
                }
            };
        let fingerprint = state.id.clone();
        stages.push(
            Stage::Fingerprint,
            StageOutcome::Ran {
                detail: format!("the project is at fingerprint {}.", state.digest),
            },
        );

        // ---- 4. Plan ---------------------------------------------------------
        let graph = ComponentGraph::of(&discovery);
        let mode = self.execution.mode;
        let permissions = self.execution.permissions.clone();
        let mut builder = PlanBuilder::new(mode, permissions.clone());
        let planned = propose_everything(
            &discovery,
            &graph,
            &intent,
            &self.config.checks,
            &mut builder,
        );
        let refused: Vec<ProposalRefused> = builder.refused().to_vec();
        let schedule = builder.build();
        stages.push(
            Stage::Plan,
            StageOutcome::Ran {
                detail: describe_plan(&schedule, &refused, planned.probe_refusal.as_ref()),
            },
        );

        // The results every later stage aggregates. A check the mode stopped is the
        // plan's own answer and nothing here invents one; a check the mode permits
        // is left unreported, and `aggregate_run` records it as a check that
        // reported nothing.
        let mut results: Vec<CheckResult> = Vec::new();
        for scheduled in schedule.checks() {
            if let Some(stopped) = scheduled.not_run(&fingerprint) {
                results.push(stopped);
            }
        }
        for declaration in &planned.missing {
            results.push(declaration.not_checked(&fingerprint));
        }

        // ---- 5. Static deterministic checks ----------------------------------
        let static_checks = schedule
            .checks()
            .iter()
            .filter(|scheduled| !scheduled.proposal().requirements().runs_project_code())
            .count();
        let dynamic_checks = schedule.checks().len() - static_checks;
        stages.push(
            Stage::StaticChecks,
            if static_checks == 0 {
                StageOutcome::NotPartOfWork {
                    detail: "the plan holds no check that reads your project without running it."
                        .to_owned(),
                }
            } else {
                StageOutcome::NotRun {
                    reason: None,
                    detail: format!(
                        "{static_checks} of the planned checks read your project's files and run \
                         nothing. This build has no runner for a planned check, so none of them \
                         reported a result and each is recorded as unknown rather than passed."
                    ),
                }
            },
        );

        // ---- 6. Approved dynamic checks --------------------------------------
        stages.push(
            Stage::DynamicChecks,
            describe_dynamic(&schedule, dynamic_checks),
        );

        // ---- 7. Completeness analysis ----------------------------------------
        let candidates = completeness(&discovery, &intent);
        stages.push(
            Stage::Completeness,
            StageOutcome::Ran {
                detail: describe_candidates(&candidates),
            },
        );

        // ---- 8. Grounded model assessment ------------------------------------
        let (model_outcome, model_results) = self.model_assessment(&fingerprint);
        results.extend(model_results);
        stages.push(Stage::ModelAssessment, model_outcome);

        // ---- 9. Claim checking -----------------------------------------------
        let (claims, claim_outcome, claim_results) = self.check_claims(&fingerprint);
        results.extend(claim_results);
        stages.push(Stage::ClaimChecking, claim_outcome);

        // ---- 10. Aggregate ---------------------------------------------------
        let report = match aggregate_run(&schedule, &results, &fingerprint) {
            Ok(report) => report,
            Err(refusal) => {
                return self.stopped_at(
                    stages,
                    Stopped {
                        stage: Stage::Aggregate,
                        detail: refusal.to_string(),
                    },
                );
            }
        };
        let capability = CapabilityReport::cli();
        let coverage = summarize(&schedule, &report, &capability);
        let not_checked: Vec<CheckResult> = report
            .results()
            .iter()
            .filter(|result| result.is_not_checked())
            .cloned()
            .collect();
        let verdict = build_verdict(
            fingerprint,
            report.aggregate().clone(),
            intent.clone(),
            capability,
            Vec::new(),
            not_checked,
            claims.clone(),
        );
        stages.push(
            Stage::Aggregate,
            StageOutcome::Ran {
                detail: format!(
                    "{} {} check(s) produced a result, {} did not run.",
                    verdict.aggregate.headline,
                    coverage.checked_count,
                    coverage.not_checked.len(),
                ),
            },
        );

        // ---- 11 and 12, as far as this purpose asks --------------------------
        let mut run = RunOutcome {
            project_root: discovery.root.to_string_lossy().into_owned(),
            mode,
            permissions,
            project_state: state,
            support,
            intent,
            schedule,
            report,
            coverage,
            verdict,
            candidates,
            claims,
            intent_caveat: self.resolve_intent().caveat(),
            repairs: Vec::new(),
            lifecycle: None,
        };

        if self.purpose.wants_repair_contracts() {
            let (repairs, outcome) = repair_contracts(&run);
            run.repairs = repairs;
            stages.push(Stage::RepairContract, outcome);
        } else {
            stages.push(
                Stage::RepairContract,
                StageOutcome::NotPartOfWork {
                    detail: format!(
                        "`sure {}` reports what was found; `sure repair` turns it into \
                         instructions an agent can act on.",
                        self.purpose.as_str()
                    ),
                },
            );
        }

        if self.purpose.wants_recheck() {
            let (lifecycle, outcome) = self.recheck(&run);
            run.lifecycle = lifecycle;
            stages.push(Stage::Recheck, outcome);
        } else {
            stages.push(
                Stage::Recheck,
                StageOutcome::NotPartOfWork {
                    detail: "this run is not comparing against an earlier one.".to_owned(),
                },
            );
        }

        PipelineOutcome {
            purpose: self.purpose,
            stages: stages.records,
            run: Some(run),
            stopped_at: None,
        }
    }

    /// The outcome for a run that could not finish.
    fn stopped_at(&self, mut stages: Stages, stopped: Stopped) -> PipelineOutcome {
        let stopped_at = stopped.stage;
        let number = stopped_at.number();
        stages.push(
            stopped_at,
            StageOutcome::Unfinished {
                detail: stopped.detail,
            },
        );
        for stage in Stage::ALL.iter().copied() {
            if stage.number() > number {
                stages.push(
                    stage,
                    StageOutcome::NotPartOfWork {
                        detail: STOPPED_BEFORE.to_owned(),
                    },
                );
            }
        }
        PipelineOutcome {
            purpose: self.purpose,
            stages: stages.records,
            run: None,
            stopped_at: Some(stopped_at),
        }
    }

    /// Stage 2: every trusted source of intent, kept apart by where it came from.
    ///
    /// Two channels can stand in for what the user asked for: the words they typed
    /// on this command line, and — where a permission allows it — what a saved
    /// session showed. The project's own `sure.yaml` is documentation, and
    /// [`intent_model::documented_goal`] labels it as such, so it is carried and
    /// never graded against. Nothing here invents a requirement from a guess:
    /// [`intent_model::inferred`] exists for a guess and is the caller's to add,
    /// not this function's.
    fn resolve_intent(&self) -> ProjectIntent {
        let mut requirements = Vec::new();
        if let Some(goal) = self.goal
            && let Ok(recorded) = project_intent::explicit_goal(goal)
        {
            requirements.extend(recorded.requirements);
        }
        if let Some(documented) = intent_model::documented_goal(self.config) {
            requirements.push(documented);
        }
        ProjectIntent::from_requirements(requirements)
    }

    /// Stage 8: the grounded model assessment, or the state a run is in without
    /// one.
    ///
    /// **This is a scope question and not a failure.** With no provider configured
    /// the stage contributes a non-critical not-checked result carrying
    /// [`NotCheckedReason::AnalysisProviderDisabled`], which is the vocabulary's
    /// own word for it, and [`crate::project_verdict`] already has a sentence for
    /// what that costs. The run is then `needs_attention` rather than green, and it
    /// is never reported as having passed something it did not do.
    fn model_assessment(&self, fingerprint: &FingerprintId) -> (StageOutcome, Vec<CheckResult>) {
        let analysis = &self.config.analysis;
        let not_checked = |reason: NotCheckedReason, detail: &str| {
            CheckResult::not_run(
                Stage::ModelAssessment.id(),
                Stage::ModelAssessment.title(),
                Severity::Note,
                false,
                reason,
                fingerprint.clone(),
            )
            .with_reason(detail.to_owned())
        };

        if analysis.provider == AnalysisProvider::Disabled {
            let detail = "No analysis provider is configured, so SURE assessed nothing with a \
                          model. This is a scope limit and not a failure: the deterministic \
                          checks are unaffected.";
            return (
                StageOutcome::NotRun {
                    reason: Some(NotCheckedReason::AnalysisProviderDisabled),
                    detail: detail.to_owned(),
                },
                vec![not_checked(
                    NotCheckedReason::AnalysisProviderDisabled,
                    detail,
                )],
            );
        }

        match analysis_provider::build(analysis, self.project) {
            // A provider is configured. No check in this build asks for
            // model-backed analysis, so the stage has nothing to do — a different
            // statement from "it could not run", and recorded as one.
            Ok(_analyzer) => (
                StageOutcome::NotPartOfWork {
                    detail: format!(
                        "the {} provider is configured, and no planned check asks for \
                         model-backed analysis in this build.",
                        analysis.provider.as_str()
                    ),
                },
                Vec::new(),
            ),
            // The project asked for a provider SURE cannot build — a local-command
            // provider with no command, say. That is a configuration fault rather
            // than a scope limit, it is visible as one, and it is not a crash.
            Err(error) => {
                let detail = format!("SURE could not build the configured model provider: {error}");
                (
                    StageOutcome::NotRun {
                        reason: Some(NotCheckedReason::AnalysisProviderDisabled),
                        detail: detail.clone(),
                    },
                    vec![not_checked(
                        NotCheckedReason::AnalysisProviderDisabled,
                        &detail,
                    )],
                )
            }
        }
    }

    /// Stage 9: what the project's agent claimed, against what was recorded.
    ///
    /// Without a store there are no claims and nothing to check them against — a
    /// scope limit rather than a gap in the project, and recorded as one. With a
    /// store, [`crate::claim_checker`] decides; a claim SURE cannot confirm is
    /// reported as `Cannot confirm` and never as `Contradicted`, which is that
    /// module's own distinction and not this one's.
    fn check_claims(
        &self,
        fingerprint: &FingerprintId,
    ) -> (Vec<Claim>, StageOutcome, Vec<CheckResult>) {
        let Some(store) = self.store else {
            let detail = "SURE has no recorded history for this machine, so there are no agent \
                          claims to check against evidence.";
            return (
                Vec::new(),
                StageOutcome::NotRun {
                    // A scope limit rather than a gap in the project: nothing was
                    // ever recorded, so there is nothing a claim could be checked
                    // against. `NotApplicable` is the vocabulary's own word for
                    // exactly that, and it is a scope limit, which is why this is
                    // not a critical result.
                    reason: Some(NotCheckedReason::NotApplicable),
                    detail: detail.to_owned(),
                },
                vec![
                    CheckResult::not_run(
                        Stage::ClaimChecking.id(),
                        Stage::ClaimChecking.title(),
                        Severity::Note,
                        false,
                        NotCheckedReason::NotApplicable,
                        fingerprint.clone(),
                    )
                    .with_reason(detail.to_owned()),
                ],
            );
        };

        match claim_checker::check_claims_in_store(store, fingerprint) {
            Ok(checked) => {
                let detail = if checked.is_empty() {
                    "the history holds no claims about this project.".to_owned()
                } else {
                    format!(
                        "{} claim(s) checked against the recorded evidence.",
                        checked.len()
                    )
                };
                let claims = checked.iter().map(claim_of).collect();
                (claims, StageOutcome::Ran { detail }, Vec::new())
            }
            // SURE tried and could not tell, which is `errored` and not
            // `not_run`: a check that could not be read is a different promise
            // from one that was never going to run, and the vocabulary keeps the
            // two apart for a reader deciding whether to try again.
            Err(error) => {
                let detail = format!("SURE could not read the claims it had recorded: {error}");
                (
                    Vec::new(),
                    StageOutcome::NotRun {
                        reason: Some(NotCheckedReason::UnknownReason),
                        detail: detail.clone(),
                    },
                    vec![CheckResult::errored(
                        Stage::ClaimChecking.id(),
                        Stage::ClaimChecking.title(),
                        Severity::Note,
                        false,
                        detail,
                        fingerprint.clone(),
                    )],
                )
            }
        }
    }

    /// Stage 12: what the last run left open, compared with this one.
    ///
    /// The comparison needs a history. Without one the stage has nothing to
    /// compare against, which is a scope limit: a first run on a machine with no
    /// history is not a run that failed to re-check.
    fn recheck(&self, run: &RunOutcome) -> (Option<LifecycleUpdate>, StageOutcome) {
        let Some(store) = self.store else {
            return (
                None,
                StageOutcome::NotRun {
                    reason: Some(NotCheckedReason::NotApplicable),
                    detail:
                        "SURE has no recorded history for this machine, so there is no earlier \
                             run to compare this one with."
                            .to_owned(),
                },
            );
        };

        let previous = match recheck_lifecycle::previous_open_findings(
            store,
            &run.project_root,
            &run.project_state.id,
        ) {
            Ok(previous) => previous,
            Err(error) => {
                return (
                    None,
                    StageOutcome::NotRun {
                        reason: Some(NotCheckedReason::UnknownReason),
                        detail: format!(
                            "SURE could not read what an earlier run left open: {error}"
                        ),
                    },
                );
            }
        };

        // No module in this build derives "the checks that can observe whether this
        // finding is fixed" from a finding — `repair_impact::select_impacted_checks`
        // takes the contract it would be helping to build. `reconcile` documents
        // what an absent entry means, and it is the conservative direction: the
        // finding stays open unless a current finding matches it. SURE takes that
        // reading and says so, rather than inventing a re-check list that would let
        // it close findings on evidence nobody gathered.
        let update = recheck_lifecycle::reconcile(
            LifecycleInputs {
                previous_open: &previous,
                current_findings: &run.verdict.findings,
                check_results: run.report.results(),
                rechecks: &[],
            },
            run.project_state.id.clone(),
        );
        let detail = if previous.is_empty() {
            "no earlier run left anything open for this project.".to_owned()
        } else {
            format!(
                "{} earlier finding(s) stayed open, {} resolved. No re-check list could be derived \
                 from a finding, so an earlier finding stayed open unless this run found it again \
                 — the conservative reading, and the only one the evidence supports.",
                update.kept_open.len(),
                update.resolved.len(),
            )
        };
        (Some(update), StageOutcome::Ran { detail })
    }
}

/// Stage 11: a repair contract per finding, when the purpose asks for one.
///
/// **This stage cannot be performed as the modules stand**, and says so rather
/// than inventing the input it is missing. [`RepairContract::from_finding`] needs
/// the checks that can observe whether a finding is fixed; nothing in this
/// workspace derives that list from a finding, and
/// [`crate::repair_impact::select_impacted_checks`] — the module that owns the
/// rule — takes the contract it would be helping to produce. A contract whose
/// acceptance test was made up would be worse than no contract, so a run with
/// findings records the stage as not-run. A run with no findings has nothing to
/// write instructions for, which is a stage that ran and found nothing to do.
///
/// [`RepairContract::from_finding`]: sure_domain::vocabulary::RepairContract::from_finding
fn repair_contracts(
    run: &RunOutcome,
) -> (Vec<sure_domain::vocabulary::RepairContract>, StageOutcome) {
    if run.verdict.findings.is_empty() {
        return (
            Vec::new(),
            StageOutcome::Ran {
                detail: "no check produced a finding, so there is nothing to write instructions \
                         for."
                    .to_owned(),
            },
        );
    }
    (
        Vec::new(),
        StageOutcome::NotRun {
            reason: None,
            detail: format!(
                "{} finding(s) need instructions. Building one needs the checks that can observe \
                 whether the finding is fixed, and no module in this build derives that list from \
                 a finding, so SURE will not invent an acceptance test for a repair.",
                run.verdict.findings.len()
            ),
        },
    )
}

/// Stage 4's proposals, from every module that produces one.
fn propose_everything(
    discovery: &Discovery,
    graph: &ComponentGraph,
    intent: &ProjectIntent,
    preferences: &crate::config::ChecksConfig,
    builder: &mut PlanBuilder,
) -> Planned {
    let mut planned = Planned::default();

    if let Some(node) = node_of(discovery) {
        let checks = checks::node::NodeChecks::of(&node);
        planned.missing.extend(checks.missing().iter().cloned());
        checks.add_to(builder);

        // The runtime probes need the Node project, because a probe is a command
        // the manifest declares. A project that is not Node-shaped plans none,
        // which is a statement about the project rather than a failure.
        match ProbePlan::of(graph, &node, preferences) {
            Ok(probes) => {
                planned.not_planned = probes.not_planned().len();
                probes.add_to(builder);
            }
            Err(refusal) => planned.probe_refusal = Some(refusal),
        }
    }
    if let Some(python) = python_of(discovery) {
        let checks = checks::python::PythonChecks::of(&python);
        planned.missing.extend(checks.missing().iter().cloned());
        checks.add_to(builder);
    }
    if let Some(rust) = rust_of(discovery) {
        let checks = checks::rust::RustChecks::of(&rust);
        planned.missing.extend(checks.missing().iter().cloned());
        checks.add_to(builder);
    }

    // Stage 7's own proposals are planned too: a candidate SURE can see is a check
    // SURE would run, and leaving them out of the plan would make the plan a
    // shorter and greener thing than the run.
    for proposal in completeness_proposals(discovery, intent) {
        // A refusal is remembered by the builder rather than lost, and the plan
        // stage reports it. Nothing here second-guesses the decision.
        drop(builder.propose(proposal));
    }

    planned
}

/// Stage 7: the completeness scanners, and the aggregator that orders them.
fn completeness(discovery: &Discovery, intent: &ProjectIntent) -> Candidates {
    let aggregated =
        false_completion_aggregator::aggregate(completeness_proposals(discovery, intent));
    let mut candidates = Candidates {
        material: aggregated
            .material()
            .iter()
            .map(summarize_proposal)
            .collect(),
        style_noise: aggregated
            .style_noise()
            .iter()
            .map(summarize_proposal)
            .collect(),
        duplicates_dropped: aggregated.duplicates_dropped(),
    };
    candidates
        .material
        .sort_by(|left, right| left.id.cmp(&right.id));
    candidates
        .style_noise
        .sort_by(|left, right| left.id.cmp(&right.id));
    candidates
}

/// Every proposal the completeness scanners produce, in one list.
///
/// The five scanners and the intent comparison are separate modules and this is
/// the only place they are gathered. Nothing is filtered here:
/// [`crate::false_completion_aggregator`] decides what is material and what is
/// style noise, and a second opinion about that in this function would be the
/// drift that module exists to prevent.
fn completeness_proposals(discovery: &Discovery, intent: &ProjectIntent) -> Vec<CheckProposal> {
    let mut proposals = Vec::new();
    proposals.extend(CandidateScanner::of(discovery).proposed().iter().cloned());
    proposals.extend(NoOpHeuristics::of(discovery).proposed().iter().cloned());
    proposals.extend(DemoDataHeuristics::of(discovery).proposed().iter().cloned());
    proposals.extend(RouteConsistency::of(discovery).proposed().iter().cloned());
    proposals.extend(UiActionBridge::of(discovery).proposals());
    proposals.extend(compare_intent_to_project(intent, discovery).findings);
    proposals
}

fn summarize_proposal(proposal: &CheckProposal) -> CandidateSummary {
    CandidateSummary {
        id: proposal.id().as_str().to_owned(),
        title: proposal.title().to_owned(),
        severity: proposal.severity(),
        critical: proposal.critical(),
        because: proposal.reason().plain_description(),
    }
}

fn node_of(discovery: &Discovery) -> Option<crate::discover::NodeProject> {
    match &discovery.report(Ecosystem::Node)?.findings {
        Findings::Node(node) => Some((**node).clone()),
        _ => None,
    }
}

fn python_of(discovery: &Discovery) -> Option<crate::discover::PythonProject> {
    match &discovery.report(Ecosystem::Python)?.findings {
        Findings::Python(python) => Some((**python).clone()),
        _ => None,
    }
}

fn rust_of(discovery: &Discovery) -> Option<crate::discover::RustProject> {
    match &discovery.report(Ecosystem::Rust)?.findings {
        Findings::Rust(rust) => Some((**rust).clone()),
        _ => None,
    }
}

/// A checked claim in the verdict's own type.
///
/// [`crate::claim_checker`] reports what it established; [`ProjectVerdict`] holds
/// the domain's [`Claim`]. The mapping is here rather than in either module
/// because it is a conversion between two vocabularies — and a session identifier
/// is not recoverable from a stored claim row, so it is `None`, which says "this
/// claim is not attributed to a session" rather than inventing one.
fn claim_of(checked: &CheckedClaim) -> Claim {
    Claim {
        id: ClaimId::parse(checked.id.as_str()).unwrap_or_else(|_| ClaimId::generate()),
        claim_text: checked.claim_text.clone(),
        claim_type: checked.claim_type.clone(),
        assessment: checked.assessment,
        reason: checked.reason.clone(),
        evidence: checked.evidence.clone(),
        session: None,
    }
}

/// Whether a claim assessment falls on the positive side, for a report grouping
/// claims. The vocabulary owns the answer; this names it for a renderer.
#[must_use]
pub const fn is_confirmed(assessment: ClaimAssessment) -> bool {
    matches!(assessment, ClaimAssessment::Confirmed)
}

fn describe_discovery(discovery: &Discovery, support: &ProjectSupport) -> String {
    let stacks: Vec<String> = discovery
        .stacks()
        .iter()
        .map(|stack| format!("{} (level {})", stack.stack, stack.level.letter()))
        .collect();
    let named = if stacks.is_empty() {
        "no stack SURE recognises".to_owned()
    } else {
        stacks.join(", ")
    };
    let complete = if discovery.is_complete() {
        "all of it was read"
    } else {
        "part of it was not read"
    };
    format!(
        "{named}; {complete}. Support reaches level {}.",
        support.level.letter()
    )
}

fn describe_intent(intent: &ProjectIntent) -> String {
    let user = intent.user_requirements().count();
    let documented = intent_model::from_source(intent, IntentSource::ProjectSpec).count();
    if user == 0 {
        format!(
            "nothing you said was available to check against ({documented} statement(s) the \
             project documents about itself)."
        )
    } else {
        format!("{user} statement(s) from you, {documented} the project documented for itself.")
    }
}

fn describe_plan(
    schedule: &CheckSchedule,
    refused: &[ProposalRefused],
    probe_refusal: Option<&PlanRefused>,
) -> String {
    let mut parts = vec![format!(
        // "stopped by the mode" rather than "was stopped": the count is
        // parenthesised out of the grammar, and `crate::enforce` prints the
        // same phrase for the same fact.
        "{} check(s) planned under {}: {} may run, {} stopped by the mode.",
        schedule.len(),
        schedule.mode().as_str(),
        schedule.may_run().count(),
        schedule.blocked().count(),
    )];
    if !refused.is_empty() {
        parts.push(format!(
            "{} candidate(s) could not become a check: {}.",
            refused.len(),
            refused
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    if let Some(refusal) = probe_refusal {
        parts.push(format!("the runtime probes were not planned: {refusal}."));
    }
    parts.join(" ")
}

fn describe_dynamic(schedule: &CheckSchedule, dynamic: usize) -> StageOutcome {
    if dynamic == 0 {
        return StageOutcome::NotPartOfWork {
            detail: "no check in the plan would run your project's code.".to_owned(),
        };
    }
    let blocked: Vec<&str> = schedule
        .blocked()
        .filter(|scheduled: &&ScheduledCheck| {
            scheduled.proposal().requirements().runs_project_code()
        })
        .map(|scheduled| scheduled.proposal().title())
        .collect();
    if blocked.is_empty() {
        return StageOutcome::NotRun {
            reason: None,
            detail: format!(
                "{dynamic} check(s) would run your project's code and the mode allows it. This \
                 build has no runner for a planned check, so none of them ran and each is \
                 recorded as unknown rather than passed."
            ),
        };
    }
    StageOutcome::NotRun {
        reason: Some(NotCheckedReason::ExecutionNotAuthorized),
        detail: format!(
            "{dynamic} check(s) would run your project's code. {} of them were stopped by the \
             execution mode and are recorded as not checked: {}.",
            blocked.len(),
            blocked.join(", ")
        ),
    }
}

fn describe_candidates(candidates: &Candidates) -> String {
    if candidates.is_empty() {
        return "no completeness candidate was found.".to_owned();
    }
    format!(
        "{} candidate(s) worth acting on, {} that are style noise, {} dropped as duplicates. A \
         candidate is something to look at, not a defect SURE has established.",
        candidates.material.len(),
        candidates.style_noise.len(),
        candidates.duplicates_dropped,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_purpose_performs_a_contiguous_run_of_the_documented_stages() {
        for purpose in [Purpose::Check, Purpose::Repair, Purpose::Recheck] {
            let last = purpose.last_stage();
            assert!(
                last.number() >= Stage::Aggregate.number(),
                "{} stops before it has a verdict",
                purpose.as_str()
            );
            assert!(Stage::ALL.contains(&last));
        }
        assert_eq!(Purpose::Check.last_stage(), Stage::Aggregate);
        assert_eq!(Purpose::Repair.last_stage(), Stage::RepairContract);
        assert_eq!(Purpose::Recheck.last_stage(), Stage::Recheck);
        assert!(!Purpose::Check.wants_recheck());
        assert!(Purpose::Recheck.wants_recheck());
        assert!(!Purpose::Check.wants_repair_contracts());
        assert!(Purpose::Repair.wants_repair_contracts());
    }

    #[test]
    fn the_stages_are_numbered_by_their_position_in_the_documented_order() {
        for (position, stage) in Stage::ALL.iter().enumerate() {
            assert_eq!(
                stage.number() as usize,
                position + 1,
                "{} is out of order",
                stage.as_str()
            );
        }
    }

    #[test]
    fn a_stage_identifier_is_the_same_on_every_run_and_differs_between_stages() {
        for stage in Stage::ALL {
            assert_eq!(stage.id(), stage.id());
        }
        for pair in Stage::ALL.windows(2) {
            assert_ne!(pair[0].id(), pair[1].id());
        }
    }

    #[test]
    fn only_an_outcome_that_did_not_run_is_a_gap() {
        assert!(
            !StageOutcome::Ran {
                detail: String::new()
            }
            .is_a_gap()
        );
        assert!(
            !StageOutcome::NotPartOfWork {
                detail: String::new()
            }
            .is_a_gap()
        );
        assert!(
            StageOutcome::NotRun {
                reason: None,
                detail: String::new()
            }
            .is_a_gap()
        );
        assert!(
            StageOutcome::Unfinished {
                detail: String::new()
            }
            .is_a_gap()
        );
    }

    #[test]
    fn a_confirmed_claim_is_the_only_positive_assessment() {
        assert!(is_confirmed(ClaimAssessment::Confirmed));
        assert!(!is_confirmed(ClaimAssessment::CannotConfirm));
        assert!(!is_confirmed(ClaimAssessment::Contradicted));
        assert!(!is_confirmed(ClaimAssessment::NotCheckable));
    }
}

//! The checks a run intends to perform, in the order a report shows them.
//!
//! # Two words that are not the same word
//!
//! A **schedule** is what a run intends to check. A
//! [`CheckPlan`](sure_domain::vocabulary::CheckPlan) is what the execution mode
//! then allowed: it is the frozen domain record, and
//! [`Enforcement::of`](crate::enforce::Enforcement::of) is the only thing that
//! produces one. This module builds the *first* and feeds the second —
//! [`CheckSchedule::planned_checks`] is the bridge, and it exists so that the
//! order a report shows is decided in one place rather than re-derived by
//! whoever calls `Enforcement`.
//!
//! The distinction is not decoration. A schedule entry can carry a reason, an
//! evidence class and what the check needs, because it is SURE's own account of
//! what it is about to do. A `CheckPlan` holds identifiers and nothing else,
//! because it is a frozen record and adding a field to it is an ADR-level
//! decision. **P4-T001's acceptance is about the schedule**: *"Ordered plan
//! includes reason, evidence class and execution requirements per check."*
//!
//! # What "ordered" has to mean
//!
//! Not "in the order the caller thought of them", which would make the plan a
//! transcript of somebody's loop rather than a value. **The order is a function
//! of the checks themselves**, and the test
//! `a_plan_is_the_same_whatever_order_it_was_built_in` holds it over a sweep of
//! all 120 permutations of a five-check set. Three rules, applied in order, the
//! last of which makes the order *total*:
//!
//! 1. **Checks that run nothing come first.** They are the ones SURE can always
//!    do, they are the ones a user can act on without being asked anything, and
//!    putting them first means the part of the report that always exists appears
//!    before the part that may not.
//! 2. **Then by severity, worst first.** This sorts on [`Severity::rank`], whose
//!    documentation says a larger number is more serious, rather than on
//!    [`Ord`] — which would also be correct today, because `Severity`'s ordering
//!    is written by hand for exactly this reason, but which is correct because
//!    somebody wrote it that way rather than because this module said so. The two
//!    are asserted to agree rather than assumed to.
//! 3. **Then by identifier, smallest first.** This rule fires only on a tie, and
//!    its only job is to make the order total: two `must_fix` checks that run
//!    nothing have no other property that distinguishes them, and without a last
//!    rule the plan's order would depend on the order the proposals arrived in.
//!
//! **The mode is deliberately not one of the rules**, even though it would be
//! easy to put the runnable checks first. `ExecutionDecision` is not a property
//! of a check; it is the answer to a question asked *about* a check under a
//! particular mode and permission set. Folding it into the ordering would mean
//! the same checks came back in two different orders depending on how they were
//! about to be run, so a caller comparing an inspect-only plan with a
//! host-confirmed one could no longer tell a changed decision from a reshuffled
//! plan. The decisions are on the entries, and
//! `the_order_does_not_move_when_the_mode_does_but_the_decisions_do` is the test
//! that keeps them there.
//!
//! # A check that cannot run is still in the plan
//!
//! Every proposal produces exactly one entry, whether or not the mode and the
//! permissions would let it run. Dropping a blocked check would produce a plan
//! that reads as complete while something in it never happened, which is the one
//! outcome this repository is built to avoid. So a blocked entry stays, carrying
//! [`ExecutionDecision`] and the permission that stopped it, and
//! [`CheckSchedule::blocked`] is how a caller finds them.
//!
//! **A schedule entry is not a result.** Nothing here says a check passed, and
//! the only status this module can produce is a *skipped* one, through
//! [`ScheduledCheck::not_run`] — which a caller reaches only for a check that
//! would not run, because the function returns `None` for one that would.
//!
//! **The evidence class on the entry does not travel into that result**, and that
//! is the domain's decision rather than this module's: `CheckResult::not_run` sets
//! [`EvidenceClass::Unknown`] and documents why — a check that did not run
//! established nothing, so offering a choice would only offer a way to write that
//! down wrongly. The class on a proposal is what the check's *result* would be
//! worth if it ran; the class on a result is what was actually established. They
//! are two different claims and the product keeps them apart.
//!
//! # What this does not do
//!
//! **It does not decide which checks exist.** There is no catalogue here and no
//! rule that derives a check from a project: proposals arrive from a caller, and
//! this module orders and gates them. Since `P4-T002` there *is* a caller —
//! [`crate::checks`] turns a discovered project's own declarations into
//! proposals — and this module still knows nothing about it. `P4-T003` (Python)
//! and `P4-T004` (Rust) add siblings there, and none of them changes anything
//! here, which is the property the seam was built for.
//!
//! The source rule in `tests/check_schedule.rs` names the files that may
//! construct a [`CheckProposal`]; it was written to fail on the day the first
//! proposer landed, it did, and the list is where that arrival was recorded.
//!
//! **It does not run anything.** Nothing here builds a
//! [`Command`](std::process::Command), and the checks a schedule admits are
//! admitted by [`Enforcement`](crate::enforce::Enforcement) afterwards — this
//! module says what a check *would* need, and [`ExecutionDecision`] is [`decide`]'s
//! answer rather than a second opinion about it. **It does not ask the user
//! anything**: `NeedsConsent` is reported as `NeedsConsent`, and turning that into
//! a prompt is a caller's job, which is what `crate::consent` is for.
//!
//! **It says nothing about whether a check is any good.** A proposal names its
//! own evidence class and its own reason, and a caller that labels a guess
//! `ObservedFact` has lied in a way this module cannot detect — the same limit
//! `crate::browser` states about its drivers.

use std::fmt;

use sure_domain::evidence::{AnchorSubject, EvidenceAnchor, EvidenceClass};
use sure_domain::execution::{
    ActionKind, ExecutionDecision, ExecutionMode, ExecutionPermissions, Permission, decide,
};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, NotCheckedReason};

use crate::consent::PlannedCheck;

/// Why a check is in the schedule.
///
/// **A reason is a claim about the project, and each variant says where it came
/// from.** The alternative — a free-text `String` — was not taken because it
/// cannot be checked: `ProjectWide` and `DeclaredCommand` are different claims,
/// and a caller that has the second and writes the first has made the report less
/// specific in a way nothing would notice. The detail each variant carries is the
/// detail the discovery already holds, so a reason cannot describe a component
/// the scan did not find.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckReason {
    /// The project declares a command this check would run, and where.
    ///
    /// `declared_in` is the path of the file the command was read from, so a
    /// reader can go and look.
    ///
    /// **`command` is the line SURE would run, and it is a rendering of what the
    /// project declared rather than a copy of it.** A `package.json` with
    /// `"build": "tsc -b"` is a project whose build check runs `npm run build` —
    /// the manifest declares a script, and what a person is asked to allow is the
    /// command that starts it. The two are different strings and this field holds
    /// the second; the first draft of this variant's documentation said it held
    /// the first, which would have had SURE print a sentence about a command line
    /// that is nowhere in the project. `P4-T002`'s proposer is what fills it, and
    /// `the_reason_names_the_command_sure_would_run_and_not_the_script_text` in
    /// `tests/node_checks.rs` is what holds the two apart.
    DeclaredCommand {
        /// The manifest or task file the declared command was read from.
        declared_in: String,
        /// The command SURE would run, as the project's package manager spells it.
        command: String,
    },
    /// A component is on the stack this check is for.
    StackPresent {
        /// The component's path, as the scan reports it.
        component: String,
        /// The stack's own name, from the component's reading.
        stack: String,
    },
    /// A file or directory this check is about is present.
    ///
    /// For the checks that are about the project rather than about a component: a
    /// lockfile, a schema directory, a sample environment file.
    FilePresent {
        /// The path, relative to the project root, as the scan reports it.
        path: String,
    },
    /// The check applies to the project as a whole.
    ProjectWide,
}

impl CheckReason {
    /// The sentence a report shows under the check's title.
    ///
    /// Written for someone who is not a programmer, like every other
    /// `plain_description` in this repository: it answers *why is SURE doing this*,
    /// not *what is this called*.
    #[must_use]
    pub fn plain_description(&self) -> String {
        match self {
            Self::DeclaredCommand {
                declared_in,
                command,
            } => format!("SURE would run `{command}`, from {declared_in}."),
            Self::StackPresent { component, stack } => {
                format!("{component} is a {stack} project.")
            }
            Self::FilePresent { path } => format!("Your project has {path}."),
            Self::ProjectWide => "This is about the project as a whole.".to_owned(),
        }
    }

    /// Whether the detail this reason carries is present rather than empty.
    ///
    /// A `DeclaredCommand` with no file, a `StackPresent` with no component: each
    /// reads as a reason and names nothing, which is worse than a missing reason
    /// because it is a claim about a file or a directory that does not exist and a
    /// reader cannot tell. [`PlanBuilder::propose`] refuses these, and this is the
    /// predicate it refuses them with.
    #[must_use]
    pub fn names_something(&self) -> bool {
        match self {
            Self::DeclaredCommand {
                declared_in,
                command,
            } => !declared_in.trim().is_empty() && !command.trim().is_empty(),
            Self::StackPresent { component, stack } => {
                !component.trim().is_empty() && !stack.trim().is_empty()
            }
            Self::FilePresent { path } => !path.trim().is_empty(),
            Self::ProjectWide => true,
        }
    }

    /// Where a reader goes to check this reason for themselves.
    ///
    /// **A reason is a claim about the project, and a claim without a place to
    /// look is one a reader has to take on trust.** The reason already carries
    /// the detail; this is that detail in the shape
    /// [`EvidenceAnchor`](sure_domain::evidence::EvidenceAnchor) asks for, so
    /// that the evidence a check produces points at the same file the check's
    /// own sentence named rather than at a second copy of it that could drift.
    ///
    /// **`None` is [`Self::ProjectWide`] and nothing else, and it is a real
    /// limit rather than a placeholder.** Every other variant here was built
    /// from a path or a component the discovery found, so it has somewhere to
    /// point. `ProjectWide` says the check applies to the project as a whole,
    /// which is a true thing to say and is not a location: a reader asking
    /// *where do I look* has no answer, and
    /// [`EvidenceAnchor::is_checkable`](sure_domain::evidence::EvidenceAnchor::is_checkable)
    /// would answer `false` for an anchor filled in with the project's own name.
    /// Returning `None` says so where a caller has to deal with it; filling in a
    /// plausible path would be the lookup that fails into a wrong anchor. No
    /// proposer in the product uses `ProjectWide` today, and when one does, this
    /// is the function that will say so.
    #[must_use]
    pub fn anchor(&self) -> Option<EvidenceAnchor> {
        match self {
            Self::DeclaredCommand {
                declared_in,
                command,
            } => Some(EvidenceAnchor::new(
                AnchorSubject::Command,
                declared_in.clone(),
                command.clone(),
            )),
            Self::StackPresent { component, stack } => Some(EvidenceAnchor::new(
                AnchorSubject::File,
                component.clone(),
                stack.clone(),
            )),
            Self::FilePresent { path } => Some(EvidenceAnchor::new(
                AnchorSubject::File,
                path.clone(),
                "the file this check is about",
            )),
            Self::ProjectWide => None,
        }
    }
}

/// What a check would need in order to run.
///
/// Built from the actions the check's commands would perform, and nothing else:
/// **the permissions are derived, not declared**, because a proposal that named
/// its own permissions could ask for less than its actions need and the schedule
/// would report it as runnable. [`ActionKind::required_permission`] is the one
/// table that maps an action to what it costs, and this module reads it rather
/// than restating it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRequirements {
    /// The actions, deduplicated, in [`ActionKind::ALL`] order.
    ///
    /// Ordered by the enum rather than by the proposal so that two checks that
    /// would do the same things compare equal, and so that
    /// [`Self::permissions_needed`] is a function of the set rather than of the
    /// order somebody listed it in.
    actions: Vec<ActionKind>,
}

impl ExecutionRequirements {
    /// The requirements of a check that would perform `actions`.
    ///
    /// Deduplicates, so `[ReadFile, ReadFile]` is one action and not two — a check
    /// that reads a file twice does not need the permission twice, and a list that
    /// said so would make the two checks compare unequal.
    ///
    /// The sort is the enum's own [`Ord`], which is the derive over the variant
    /// declaration, and [`ActionKind::ALL`] is that same declaration — so the
    /// order here is `ALL` order. That is a fact about two separate lists in
    /// `execution.rs` agreeing, so the test `the_enum_order_and_all_agree` reads
    /// it out of the domain rather than trusting this sentence.
    #[must_use]
    pub fn of(actions: &[ActionKind]) -> Self {
        let mut actions = actions.to_vec();
        actions.sort();
        actions.dedup();
        Self { actions }
    }

    /// A check that runs nothing and reads nothing.
    ///
    /// Not the default and not a shortcut: a check with no actions is a check SURE
    /// has no idea how to perform, so it is built only where that is the honest
    /// answer, and [`Self::is_empty`] is how a caller notices. The builder refuses
    /// such a check rather than scheduling it.
    #[must_use]
    pub fn none() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// Whether the check would perform no action at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// The actions the check would perform.
    #[must_use]
    pub fn actions(&self) -> &[ActionKind] {
        &self.actions
    }

    /// Every permission the check needs, in [`Permission::ALL`] order.
    ///
    /// **Derived one action at a time from [`ActionKind::required_permission`]**,
    /// so a check that reads a file and runs a test needs `Inspect` *and*
    /// `RunProjectCode` and a schedule entry showing only the second would
    /// understate what it costs. The mapping is not "everything needs `Inspect`":
    /// a browser probe needs only `ConnectService`, and a check that installs a
    /// package needs only `InstallDependencies`.
    #[must_use]
    pub fn permissions_needed(&self) -> Vec<Permission> {
        let mut needed: Vec<Permission> = self
            .actions
            .iter()
            .map(|action| action.required_permission())
            .collect();
        needed.sort();
        needed.dedup();
        needed
    }

    /// Whether any action would execute the project's own code.
    #[must_use]
    pub fn runs_project_code(&self) -> bool {
        self.actions
            .iter()
            .any(|action| action.executes_project_code())
    }

    /// Whether any action could reach outside this machine.
    #[must_use]
    pub fn can_touch_network(&self) -> bool {
        self.actions.iter().any(|action| action.can_touch_network())
    }

    /// Whether any action could change something on disk.
    #[must_use]
    pub fn can_modify_disk(&self) -> bool {
        self.actions.iter().any(|action| action.can_modify_disk())
    }

    /// Whether every action is read-only analysis.
    #[must_use]
    pub fn is_inspection_only(&self) -> bool {
        !self.runs_project_code() && !self.can_modify_disk() && !self.can_touch_network()
    }

    /// Whether the check runs no project code, which is the first ordering rule.
    ///
    /// **This is not [`Self::is_inspection_only`]**, and the difference is the
    /// actions that reach out without executing anything: a `BrowserProbe` drives
    /// a real browser, so it is not read-only by the domain's own predicates, and
    /// it still runs none of the project's code. The two agree for a `ReadFile`
    /// and for a `RunTests` and come apart in between, which is where the ordering
    /// rule has to be about the narrower question — what a report can show without
    /// asking for anything that runs code.
    #[must_use]
    pub fn runs_nothing(&self) -> bool {
        !self.runs_project_code()
    }

    /// The decision for the check as a whole, under a mode and a permission set.
    ///
    /// **The worst of the individual decisions**, and the order is written out
    /// here rather than derived: [`ExecutionDecision`] has no `Ord` and
    /// deliberately so — `Denied` and `NeedsConsent` are not two points on a
    /// scale, they are two different answers, and a derived order would make which
    /// one is "greater" a fact about the variant list. Here `Denied` beats
    /// `NeedsConsent` and `NeedsConsent` beats `Allowed`, because a check with one
    /// action that can never run is not more runnable than a check with one action
    /// that could at least be asked about.
    ///
    /// A check with no actions is `Allowed`, which is the same answer `decide`
    /// gives for an empty set — and is unreachable through
    /// [`PlanBuilder`](crate::schedule::PlanBuilder), which refuses such a check.
    #[must_use]
    pub fn decision(
        &self,
        mode: ExecutionMode,
        permissions: &ExecutionPermissions,
    ) -> ExecutionDecision {
        self.actions
            .iter()
            .map(|action| decide(*action, mode, permissions))
            .max_by_key(|decision| decision_rank(*decision))
            .unwrap_or(ExecutionDecision::Allowed)
    }

    /// The first permission the check needs and has not been granted, in
    /// [`Permission::ALL`] order.
    ///
    /// `None` when nothing is missing. **The first rather than all of them**: a
    /// prompt that listed four missing permissions for one check would ask the
    /// user to agree to something they cannot evaluate, and the product's rule is
    /// that a refusal names the one thing that would change the answer.
    /// [`Self::permissions_missing`] gives the whole list where a caller wants it.
    ///
    /// **A granted permission is never returned, even when the check still cannot
    /// run.** A check the *mode* stops — project code under
    /// [`InspectOnly`](ExecutionMode::InspectOnly) — has every permission it needs,
    /// so this returns `None` and the entry says the mode is what stopped it.
    /// Naming a permission the user has already granted would be a sentence that
    /// asks them to do something they have done; the first draft of this function
    /// did exactly that, and the test
    /// `a_check_missing_several_permissions_names_the_first_of_them` is what found
    /// it. Read [`Self::decision`] to tell the two apart.
    #[must_use]
    pub fn blocked_by(&self, permissions: &ExecutionPermissions) -> Option<Permission> {
        self.permissions_missing(permissions).into_iter().next()
    }

    /// Every permission the check needs and has not been granted.
    ///
    /// Deduplicated and in [`Permission::ALL`] order, so a check that would read
    /// two files and install a package reports `InstallDependencies` once.
    ///
    /// **This is a fact about the permission set and not about the mode**, so it
    /// takes no mode: it is the same list under every one. That is the whole of
    /// what "denied" means in [`decide`] — `Denied` is returned exactly when the
    /// permission is not granted — and a check that has its permissions but is
    /// stopped by the mode comes back empty here and non-`Allowed` from
    /// [`Self::decision`].
    #[must_use]
    pub fn permissions_missing(&self, permissions: &ExecutionPermissions) -> Vec<Permission> {
        let mut missing: Vec<Permission> = self
            .actions
            .iter()
            .filter(|action| !permissions.allows(action.required_permission()))
            .map(|action| action.required_permission())
            .collect();
        missing.sort();
        missing.dedup();
        missing
    }
}

/// Where a decision sits, worst last.
///
/// See [`ExecutionRequirements::decision`] for why this is written out rather than
/// derived. It is the only place [`ExecutionDecision`] is given an order, and it
/// is not exported: the crate's callers compare decisions with `is_allowed`, and a
/// public ranking of them would be an invitation to sort by something the domain
/// says is not a scale.
const fn decision_rank(decision: ExecutionDecision) -> u8 {
    match decision {
        ExecutionDecision::Allowed => 0,
        ExecutionDecision::NeedsConsent => 1,
        ExecutionDecision::Denied => 2,
    }
}

/// A check SURE proposes to perform.
///
/// The four things the acceptance names are all here as fields, and the type does
/// not let a caller omit one: there is no `Default` and no setter, so the only way
/// to build a proposal is [`Self::new`] with all of them. A check that cannot say
/// why it is here, how strong its answer would be, or what it needs is not a check
/// this product can put in front of a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckProposal {
    id: CheckId,
    title: String,
    severity: Severity,
    critical: bool,
    evidence_class: EvidenceClass,
    reason: CheckReason,
    requirements: ExecutionRequirements,
}

impl CheckProposal {
    /// Propose a check.
    ///
    /// `evidence_class` is the weight the check's *result* would carry, and it is
    /// a parameter rather than a constant because a check that reads a lockfile
    /// and a check that runs a test suite do not produce the same kind of evidence
    /// — the first is [`EvidenceClass::ObservedFact`] and the second is
    /// [`EvidenceClass::DeterministicCheck`]. Making it a field is what stops the
    /// distinction being lost between the proposal and the finding.
    #[must_use]
    pub fn new(
        id: CheckId,
        title: impl Into<String>,
        severity: Severity,
        critical: bool,
        evidence_class: EvidenceClass,
        reason: CheckReason,
        actions: &[ActionKind],
    ) -> Self {
        Self {
            id,
            title: title.into(),
            severity,
            critical,
            evidence_class,
            reason,
            requirements: ExecutionRequirements::of(actions),
        }
    }

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

    /// How bad it is if this check is not satisfied.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Whether the project cannot be trusted for hand-off when this fails.
    #[must_use]
    pub const fn critical(&self) -> bool {
        self.critical
    }

    /// How much weight the check's result would carry.
    #[must_use]
    pub const fn evidence_class(&self) -> EvidenceClass {
        self.evidence_class
    }

    /// Why the check is proposed.
    #[must_use]
    pub const fn reason(&self) -> &CheckReason {
        &self.reason
    }

    /// What the check would need in order to run.
    #[must_use]
    pub const fn requirements(&self) -> &ExecutionRequirements {
        &self.requirements
    }
}

/// One check placed in the schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledCheck {
    proposal: CheckProposal,
    position: usize,
    decision: ExecutionDecision,
    blocked_by: Option<Permission>,
}

impl ScheduledCheck {
    /// The check, with its reason, evidence class and requirements.
    #[must_use]
    pub const fn proposal(&self) -> &CheckProposal {
        &self.proposal
    }

    /// Where the check sits in the schedule, counting from zero.
    ///
    /// Stored rather than left to the caller's loop counter, because a report that
    /// numbered its own rows could number them in a different order from the one
    /// the plan was built in and nothing would notice.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Whether the mode and the permissions would let this check run.
    #[must_use]
    pub const fn decision(&self) -> ExecutionDecision {
        self.decision
    }

    /// The permission that stops the check, if one does.
    #[must_use]
    pub const fn blocked_by(&self) -> Option<Permission> {
        self.blocked_by
    }

    /// Whether the check would run.
    #[must_use]
    pub const fn may_run(&self) -> bool {
        self.decision.is_allowed()
    }

    /// This check as the result of *not* running it, or `None` if it would run.
    ///
    /// The `None` is the point: this module can produce a skipped result for a
    /// check and cannot produce any other status for one, so a caller cannot get a
    /// *pass* out of a schedule entry. The reason is the vocabulary's own —
    /// [`NotCheckedReason::ExecutionNotAuthorized`], which is what an execution
    /// mode that did not authorize this check means — and never a word invented
    /// here.
    #[must_use]
    pub fn not_run(&self, project_fingerprint: &FingerprintId) -> Option<CheckResult> {
        if self.may_run() {
            return None;
        }
        Some(CheckResult::not_run(
            self.proposal.id.clone(),
            self.proposal.title.clone(),
            self.proposal.severity,
            self.proposal.critical,
            NotCheckedReason::ExecutionNotAuthorized,
            project_fingerprint.clone(),
        ))
    }

    /// The sentence a report shows for this entry.
    #[must_use]
    pub fn plain_description(&self) -> String {
        let mut line = format!(
            "{} - {} ({})",
            self.proposal.title,
            self.proposal.reason.plain_description(),
            self.proposal.evidence_class.as_str()
        );
        // Three outcomes rather than two, because "will not run" covers two
        // situations a reader acts on differently: one where granting a permission
        // changes the answer, and one where the mode does. The wording follows
        // `decision` and not `blocked_by`, so a check the mode stopped is never
        // described as one a permission stopped.
        let outcome = match self.decision {
            ExecutionDecision::Allowed => ", will run".to_owned(),
            ExecutionDecision::NeedsConsent => ", will run only if you agree".to_owned(),
            ExecutionDecision::Denied => match self.blocked_by {
                Some(permission) => format!(", will not run: {}", permission.as_str()),
                None => ", will not run".to_owned(),
            },
        };
        line.push_str(&outcome);
        line
    }
}

/// What a run intends to check, in the order a report shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckSchedule {
    mode: ExecutionMode,
    checks: Vec<ScheduledCheck>,
    duplicates: Vec<CheckId>,
}

impl CheckSchedule {
    /// The checks, in the schedule's order.
    #[must_use]
    pub fn checks(&self) -> &[ScheduledCheck] {
        &self.checks
    }

    /// The mode the schedule was built under.
    ///
    /// Recorded because the decisions on the entries are answers about this mode,
    /// and a schedule read without it would show a set of refusals with nothing
    /// saying what refused them.
    #[must_use]
    pub const fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// How many checks are scheduled.
    #[must_use]
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Whether nothing is scheduled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// The check with this identifier, if it is scheduled.
    ///
    /// Looking one up by identity is how a caller joins a result back to the plan
    /// that proposed it, which is what the repair contract in
    /// `docs/architecture/CHECK_PIPELINE.md` does at step 11.
    #[must_use]
    pub fn get(&self, id: &CheckId) -> Option<&ScheduledCheck> {
        self.checks
            .iter()
            .find(|scheduled| scheduled.proposal.id == *id)
    }

    /// The checks that would run, in plan order.
    pub fn may_run(&self) -> impl Iterator<Item = &ScheduledCheck> {
        self.checks.iter().filter(|scheduled| scheduled.may_run())
    }

    /// The checks that would not, with their reasons still attached.
    ///
    /// **A separate iterator rather than a filter the caller writes**, so that the
    /// two halves are always complements of one another and a caller cannot count
    /// only the runnable ones and call the plan finished.
    pub fn blocked(&self) -> impl Iterator<Item = &ScheduledCheck> {
        self.checks.iter().filter(|scheduled| !scheduled.may_run())
    }

    /// Identifiers that were proposed more than once.
    ///
    /// **Reported rather than absorbed silently.** A check proposed twice is
    /// scheduled once, because two entries with one identifier would produce two
    /// results for one check — the same reasoning
    /// [`Enforcement::of`](crate::enforce::Enforcement::of) applies to its own
    /// list. The caller that did it has a bug, and this is where it shows, which is
    /// what `CheckPlan::exclude`'s return value and
    /// [`Enforcement::unscheduled`](crate::enforce::Enforcement::unscheduled) do
    /// one level down.
    #[must_use]
    pub fn duplicates(&self) -> &[CheckId] {
        &self.duplicates
    }

    /// The scheduled checks as the values [`Enforcement`](crate::enforce::Enforcement)
    /// consumes, in this schedule's order.
    ///
    /// This is the bridge, and it is one function so that the order a report shows
    /// cannot be decided twice. **It carries what `Enforcement` needs and nothing
    /// more**: a `PlannedCheck` is an identity and a weight, so the reason and the
    /// evidence class stop here — they are the schedule's, and the frozen plan does
    /// not grow a field for them.
    #[must_use]
    pub fn planned_checks(&self) -> Vec<PlannedCheck> {
        self.checks
            .iter()
            .map(|scheduled| {
                PlannedCheck::new(
                    scheduled.proposal.id.clone(),
                    scheduled.proposal.title.clone(),
                    scheduled.proposal.severity,
                    scheduled.proposal.critical,
                )
            })
            .collect()
    }

    /// One line per check, in order, for a report or a consent prompt.
    #[must_use]
    pub fn plain_description(&self) -> Vec<String> {
        self.checks
            .iter()
            .map(ScheduledCheck::plain_description)
            .collect()
    }
}

/// A refusal to build a schedule.
///
/// Three ways to hand in a proposal that cannot be part of a plan, each refused
/// rather than repaired. **Repairing one would be the false green**: a check with
/// an empty title, a reason that names nothing, or no actions at all would appear
/// in the plan as a row a user is meant to read, and none of the three can be
/// invented by this module without making something up. An empty title has no text
/// to show, an empty reason has no file to point at, and a check with no actions is
/// one SURE has no idea how to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalRefused {
    /// The check has no title to show.
    NoTitle {
        /// The check that was refused.
        id: CheckId,
    },
    /// The reason names nothing — an empty path, component, stack or command.
    EmptyReason {
        /// The check that was refused.
        id: CheckId,
    },
    /// The check declares no action, so it is not a check SURE knows how to do.
    NoActions {
        /// The check that was refused.
        id: CheckId,
    },
}

impl fmt::Display for ProposalRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (id, why) = match self {
            Self::NoTitle { id } => (id, "has no title to show a user"),
            Self::EmptyReason { id } => (
                id,
                "gives a reason that names no file, component or command",
            ),
            Self::NoActions { id } => (id, "declares no action, so nothing would be done"),
        };
        write!(f, "the check {id} {why}")
    }
}

impl std::error::Error for ProposalRefused {}

/// Builds a [`CheckSchedule`] out of proposals.
///
/// See the module documentation for the ordering rules. Held mutably rather than
/// chained, because a builder that returns `Self` makes the *order the proposals
/// were added in* look like it might matter, and rule three exists precisely
/// because it does not.
#[derive(Debug, Clone)]
pub struct PlanBuilder {
    mode: ExecutionMode,
    permissions: ExecutionPermissions,
    proposals: Vec<CheckProposal>,
    refused: Vec<ProposalRefused>,
}

impl PlanBuilder {
    /// A builder for a run under `mode` and `permissions`.
    #[must_use]
    pub fn new(mode: ExecutionMode, permissions: ExecutionPermissions) -> Self {
        Self {
            mode,
            permissions,
            proposals: Vec::new(),
            refused: Vec::new(),
        }
    }

    /// Hand in a check.
    ///
    /// Returns `Err` with the reason when the proposal cannot be part of a plan;
    /// see [`ProposalRefused`]. **A refused proposal is remembered rather than
    /// discarded**, so a caller that ignores the return value still finds out:
    /// [`Self::refused`] lists them, and that is where a caller checks that what it
    /// proposed is what the plan holds.
    pub fn propose(&mut self, proposal: CheckProposal) -> Result<(), ProposalRefused> {
        let refusal = if proposal.title.trim().is_empty() {
            Some(ProposalRefused::NoTitle {
                id: proposal.id.clone(),
            })
        } else if !proposal.reason.names_something() {
            Some(ProposalRefused::EmptyReason {
                id: proposal.id.clone(),
            })
        } else if proposal.requirements.is_empty() {
            Some(ProposalRefused::NoActions {
                id: proposal.id.clone(),
            })
        } else {
            None
        };

        match refusal {
            Some(refusal) => {
                self.refused.push(refusal.clone());
                Err(refusal)
            }
            None => {
                self.proposals.push(proposal);
                Ok(())
            }
        }
    }

    /// The proposals that were refused, in the order they were handed in.
    ///
    /// **Ordered by arrival rather than by the plan's ordering rules**, because
    /// these are not checks and have no plan position: they are a record of what a
    /// caller tried to do, and the order it tried them in is the useful thing about
    /// them.
    #[must_use]
    pub fn refused(&self) -> &[ProposalRefused] {
        &self.refused
    }

    /// The schedule, ordered and with every decision made.
    #[must_use]
    pub fn build(self) -> CheckSchedule {
        let mut proposals = self.proposals;
        // Rule three has to be inside the comparison rather than a second pass, so
        // that the identifier is the *last* word: a `sort_by_key` on the first two
        // rules would leave equal checks in submission order, which is exactly the
        // order this module promises not to depend on.
        proposals.sort_by(|left, right| {
            ordering_key(left)
                .cmp(&ordering_key(right))
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });

        let mut seen: Vec<CheckId> = Vec::new();
        let mut duplicates: Vec<CheckId> = Vec::new();
        let mut checks: Vec<ScheduledCheck> = Vec::new();

        for proposal in proposals {
            if seen.contains(&proposal.id) {
                // The first occurrence keeps its place and the second is reported.
                // Keeping the second would move the check to wherever the duplicate
                // happened to sort, and keeping both would produce two results for
                // one check.
                if !duplicates.contains(&proposal.id) {
                    duplicates.push(proposal.id.clone());
                }
                continue;
            }
            seen.push(proposal.id.clone());
            let decision = proposal.requirements.decision(self.mode, &self.permissions);
            let blocked_by = proposal.requirements.blocked_by(&self.permissions);
            checks.push(ScheduledCheck {
                position: checks.len(),
                decision,
                blocked_by,
                proposal,
            });
        }

        CheckSchedule {
            mode: self.mode,
            checks,
            duplicates,
        }
    }
}

/// The first two ordering rules, as one comparable key.
///
/// A tuple rather than a closure body because the rules have to be readable as
/// rules: `(runs_code, Reverse(rank))` sorts checks that run nothing first and,
/// within each half, worst first. The severity is reversed rather than the whole
/// tuple, because reversing the tuple would also reverse the half that must keep
/// the other direction.
fn ordering_key(proposal: &CheckProposal) -> (bool, std::cmp::Reverse<u8>) {
    (
        !proposal.requirements.runs_nothing(),
        std::cmp::Reverse(proposal.severity.rank()),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::status::CheckStatus;

    fn a_proposal(
        title: &str,
        severity: Severity,
        class: EvidenceClass,
        actions: &[ActionKind],
    ) -> CheckProposal {
        CheckProposal::new(
            CheckId::generate(),
            title,
            severity,
            false,
            class,
            CheckReason::ProjectWide,
            actions,
        )
    }

    fn a_builder() -> PlanBuilder {
        PlanBuilder::new(
            ExecutionMode::HostConfirmed,
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        )
    }

    fn titles(schedule: &CheckSchedule) -> Vec<&str> {
        schedule
            .checks()
            .iter()
            .map(|scheduled| scheduled.proposal().title())
            .collect()
    }

    /// Every permutation of `0..n`, in lexicographic order.
    ///
    /// Written out rather than pulled in, and the caller asserts the count so that
    /// a generator which produced fewer — or the same one twice — fails rather than
    /// silently sweeping a smaller set. **A permutation sweep is exactly the kind
    /// of loop whose failure is a smaller, plausible answer.**
    fn permutations_of(n: usize) -> Vec<Vec<usize>> {
        fn walk(
            n: usize,
            current: &mut Vec<usize>,
            used: &mut Vec<bool>,
            out: &mut Vec<Vec<usize>>,
        ) {
            if current.len() == n {
                out.push(current.clone());
                return;
            }
            for candidate in 0..n {
                if used[candidate] {
                    continue;
                }
                used[candidate] = true;
                current.push(candidate);
                walk(n, current, used, out);
                current.pop();
                used[candidate] = false;
            }
        }

        let mut out = Vec::new();
        walk(n, &mut Vec::new(), &mut vec![false; n], &mut out);
        out
    }

    #[test]
    fn the_enum_order_and_all_agree() {
        // `ExecutionRequirements::of` sorts with the derive and claims that is
        // `ActionKind::ALL`'s order. `ALL` is written out by hand in `execution.rs`
        // and the derive is over the declaration, so the two are separate lists
        // that have to agree -- this is the test that says they do, rather than a
        // comment asserting it.
        for pair in ActionKind::ALL.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{:?} sorts before {:?}, so the derived order is not ALL order",
                pair[0],
                pair[1]
            );
        }
        let mut by_ord = ActionKind::ALL.to_vec();
        by_ord.sort();
        assert_eq!(by_ord, ActionKind::ALL);

        for pair in Permission::ALL.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{:?} sorts before {:?}, so the derived order is not ALL order",
                pair[0],
                pair[1]
            );
        }
        let mut permissions = Permission::ALL.to_vec();
        permissions.sort();
        assert_eq!(permissions, Permission::ALL);

        assert_eq!(ActionKind::ALL.len(), 17, "an action was added or removed");
        assert_eq!(
            Permission::ALL.len(),
            6,
            "a permission was added or removed"
        );
    }

    #[test]
    fn a_check_that_runs_nothing_comes_before_one_that_does() {
        let mut builder = a_builder();
        builder
            .propose(a_proposal(
                "runs the tests",
                Severity::MustFix,
                EvidenceClass::DeterministicCheck,
                &[ActionKind::RunTests],
            ))
            .unwrap();
        builder
            .propose(a_proposal(
                "reads the manifest",
                Severity::Note,
                EvidenceClass::ObservedFact,
                &[ActionKind::ReadMetadata],
            ))
            .unwrap();

        let schedule = builder.build();
        // The more serious check runs project code and comes second anyway: rule
        // one is about what a report can show without asking anything.
        assert_eq!(titles(&schedule), ["reads the manifest", "runs the tests"]);
        assert!(
            schedule.checks()[0]
                .proposal()
                .requirements()
                .runs_nothing()
        );
        assert!(
            !schedule.checks()[1]
                .proposal()
                .requirements()
                .runs_nothing()
        );
    }

    #[test]
    fn within_a_half_the_worst_comes_first() {
        let mut builder = a_builder();
        for (title, severity) in [
            ("a note", Severity::Note),
            ("must fix", Severity::MustFix),
            ("can fix later", Severity::CanFixLater),
            ("should fix first", Severity::ShouldFixFirst),
        ] {
            builder
                .propose(a_proposal(
                    title,
                    severity,
                    EvidenceClass::ObservedFact,
                    &[ActionKind::ReadFile],
                ))
                .unwrap();
        }

        assert_eq!(
            titles(&builder.build()),
            ["must fix", "should fix first", "can fix later", "a note"],
            "the schedule sorts on Severity::rank and not on the variant order"
        );

        // `Severity` carries two orderings -- the hand-written `Ord` and `rank` --
        // and this module sorts on `rank` while `severity.rs` documents `Ord` as
        // meaning the same thing. Two orderings that are supposed to agree are
        // exactly where an inversion hides, so they are compared rather than
        // trusted: over every level, greater `Ord` must mean greater `rank`.
        for pair in Severity::ALL.windows(2) {
            assert!(
                pair[0] > pair[1] && pair[0].rank() > pair[1].rank(),
                "{:?} and {:?} disagree between Ord and rank",
                pair[0],
                pair[1]
            );
        }
        assert_eq!(Severity::ALL.len(), 4);
        assert!(Severity::MustFix > Severity::Note);
        assert_eq!(Severity::MustFix.rank(), 3);
        assert_eq!(Severity::Note.rank(), 0);
    }

    #[test]
    fn a_plan_is_the_same_whatever_order_it_was_built_in() {
        // Every permutation of five checks, compared as a whole rather than by its
        // first entry: a rule that only ordered the head would pass a check on the
        // first title and fail here.
        let ids: Vec<CheckId> = (0..5).map(|_| CheckId::generate()).collect();
        let kinds = [
            ("reads", Severity::Note, ActionKind::ReadFile),
            ("runs", Severity::MustFix, ActionKind::RunTests),
            ("probes", Severity::MustFix, ActionKind::LocalProbe),
            ("lists", Severity::CanFixLater, ActionKind::ListDirectory),
            ("builds", Severity::ShouldFixFirst, ActionKind::Build),
        ];

        let proposals: Vec<CheckProposal> = ids
            .iter()
            .zip(kinds)
            .map(|(id, (title, severity, action))| {
                CheckProposal::new(
                    id.clone(),
                    title,
                    severity,
                    false,
                    EvidenceClass::DeterministicCheck,
                    CheckReason::ProjectWide,
                    &[action],
                )
            })
            .collect();

        let mut permutations = 0_usize;
        let mut reference: Option<Vec<CheckId>> = None;
        for order in permutations_of(5) {
            let mut builder = a_builder();
            for index in &order {
                builder.propose(proposals[*index].clone()).unwrap();
            }
            let schedule = builder.build();
            let seen: Vec<CheckId> = schedule
                .checks()
                .iter()
                .map(|scheduled| scheduled.proposal().id().clone())
                .collect();
            assert_eq!(seen.len(), 5, "a permutation lost a check");
            match &reference {
                None => reference = Some(seen),
                Some(first) => assert_eq!(
                    &seen, first,
                    "the plan built from permutation {order:?} differs from the \
                     first one, so the order depends on how the checks arrived"
                ),
            }
            permutations += 1;
        }
        assert_eq!(permutations, 120, "5! permutations were not all visited");

        // The sweep is only worth anything if the order it settled on is the one
        // the rules describe, so that is asserted too rather than the plan merely
        // being stably wrong. Nothing-running first, worst first inside each half.
        let reference = reference.expect("the sweep ran at least once");
        let actual: Vec<&str> = reference
            .iter()
            .map(|id| {
                proposals
                    .iter()
                    .find(|proposal| proposal.id() == id)
                    .expect("every scheduled id came from the proposals")
                    .title()
            })
            .collect();
        assert_eq!(actual, ["probes", "lists", "reads", "runs", "builds"]);
    }

    #[test]
    fn two_checks_that_are_alike_in_every_ordering_rule_are_ordered_by_identity() {
        // Rule three is the only thing that makes the order total, so it needs a
        // test of its own: without it these two are interchangeable and the plan
        // would depend on arrival order while every other test still passed.
        let first = CheckId::generate();
        let second = CheckId::generate();
        let (low, high) = if first.as_str() < second.as_str() {
            (first, second)
        } else {
            (second, first)
        };

        let proposal_for = |id: CheckId, title: &str| {
            CheckProposal::new(
                id,
                title,
                Severity::MustFix,
                true,
                EvidenceClass::ObservedFact,
                CheckReason::ProjectWide,
                &[ActionKind::ReadFile],
            )
        };

        let ids_of = |schedule: &CheckSchedule| -> Vec<CheckId> {
            schedule
                .checks()
                .iter()
                .map(|scheduled| scheduled.proposal().id().clone())
                .collect()
        };

        let mut one = a_builder();
        one.propose(proposal_for(low.clone(), "low")).unwrap();
        one.propose(proposal_for(high.clone(), "high")).unwrap();

        let mut other = a_builder();
        other.propose(proposal_for(high.clone(), "high")).unwrap();
        other.propose(proposal_for(low.clone(), "low")).unwrap();

        // The two proposals differ only in their identifiers, so the titles are
        // the wrong thing to compare -- the order of the ids is the property.
        let low_first = [low.clone(), high.clone()];
        assert_eq!(ids_of(&one.build()), low_first);
        assert_eq!(ids_of(&other.build()), low_first);
    }

    #[test]
    fn the_order_does_not_move_when_the_mode_does_but_the_decisions_do() {
        // The plan's order is a function of the checks, so the same proposals under
        // two modes differ in exactly one thing. If the decision ever entered the
        // ordering key this fails, and it fails by the two plans disagreeing about
        // sequence rather than about allowability.
        // The proposals themselves, not a round trip through `planned_checks`:
        // that bridge carries an identity and a weight and deliberately drops the
        // actions, so rebuilding from it would give every check the same
        // requirements and this test would be comparing three identical checks
        // under a name that says otherwise.
        let proposals: Vec<CheckProposal> = [
            ("reads", Severity::Note, ActionKind::ReadFile),
            ("tests", Severity::MustFix, ActionKind::RunTests),
            ("lockfile", Severity::MustFix, ActionKind::ReadMetadata),
        ]
        .into_iter()
        .map(|(title, severity, action)| {
            a_proposal(
                title,
                severity,
                EvidenceClass::DeterministicCheck,
                &[action],
            )
        })
        .collect();

        let schedule_under = |mode: ExecutionMode, permissions: ExecutionPermissions| {
            let mut builder = PlanBuilder::new(mode, permissions);
            for proposal in &proposals {
                builder.propose(proposal.clone()).unwrap();
            }
            builder.build()
        };

        let host = schedule_under(
            ExecutionMode::HostConfirmed,
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        let inspect = schedule_under(
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        );

        assert_eq!(
            titles(&host),
            titles(&inspect),
            "the mode changed the order, so the order is not a function of the checks"
        );
        assert_eq!(titles(&host), ["lockfile", "reads", "tests"]);

        // And the decisions did change, which is what makes the equality above a
        // claim about ordering rather than about two identical plans.
        let decision_of = |schedule: &CheckSchedule, title: &str| {
            schedule
                .checks()
                .iter()
                .find(|scheduled| scheduled.proposal().title() == title)
                .expect("the check is in the plan")
                .decision()
        };
        assert_eq!(
            decision_of(&host, "tests"),
            ExecutionDecision::Allowed,
            "host-confirmed with run-project-code granted can run the tests"
        );
        assert_eq!(
            decision_of(&inspect, "tests"),
            ExecutionDecision::Denied,
            "inspect-only cannot, and says so rather than dropping the check"
        );
        assert_eq!(decision_of(&host, "reads"), ExecutionDecision::Allowed);
        assert_eq!(decision_of(&inspect, "reads"), ExecutionDecision::Allowed);
    }

    #[test]
    fn every_check_in_the_plan_carries_a_reason_an_evidence_class_and_its_requirements() {
        // The acceptance sentence, checked over a plan rather than over one
        // proposal: every entry, not the first one.
        let mut builder = a_builder();
        builder
            .propose(CheckProposal::new(
                CheckId::generate(),
                "the lockfile matches the manifest",
                Severity::MustFix,
                true,
                EvidenceClass::ObservedFact,
                CheckReason::FilePresent {
                    path: "package-lock.json".to_owned(),
                },
                &[ActionKind::ReadFile, ActionKind::ReadMetadata],
            ))
            .unwrap();
        builder
            .propose(CheckProposal::new(
                CheckId::generate(),
                "the declared tests pass",
                Severity::MustFix,
                true,
                EvidenceClass::DeterministicCheck,
                CheckReason::DeclaredCommand {
                    declared_in: "package.json".to_owned(),
                    command: "npm test".to_owned(),
                },
                &[ActionKind::RunTests],
            ))
            .unwrap();

        let schedule = builder.build();
        assert!(!schedule.is_empty());
        for scheduled in schedule.checks() {
            let proposal = scheduled.proposal();
            assert!(proposal.reason().names_something());
            assert!(!proposal.reason().plain_description().is_empty());
            assert!(!proposal.evidence_class().as_str().is_empty());
            assert!(!proposal.requirements().is_empty());
            assert!(!proposal.requirements().actions().is_empty());
            assert!(!proposal.requirements().permissions_needed().is_empty());
            assert!(!scheduled.plain_description().is_empty());
        }

        // And the evidence classes are not all the same one, which is what makes
        // carrying the field worth anything: a proposal that could only ever be
        // `unknown` would satisfy the loop above while saying nothing.
        let classes: Vec<&str> = schedule
            .checks()
            .iter()
            .map(|scheduled| scheduled.proposal().evidence_class().as_str())
            .collect();
        assert!(classes.contains(&"observed_fact"));
        assert!(classes.contains(&"deterministic_check"));
    }

    #[test]
    fn a_check_that_cannot_run_is_in_the_plan_with_its_reason_and_its_permission() {
        // Inspect-only, so the check that runs tests cannot run. It is still
        // scheduled, still ordered, and still carries what it needs.
        let mut builder = PlanBuilder::new(
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        );
        builder
            .propose(CheckProposal::new(
                CheckId::generate(),
                "the declared tests pass",
                Severity::MustFix,
                // Critical, so the *skipped* result below can be checked against
                // `blocks_green`: a non-critical check that never ran does not
                // block, which is a different fact and is asserted further down.
                true,
                EvidenceClass::DeterministicCheck,
                CheckReason::ProjectWide,
                &[ActionKind::RunTests],
            ))
            .unwrap();

        let schedule = builder.build();
        assert_eq!(
            schedule.len(),
            1,
            "a blocked check was dropped from the plan"
        );
        let blocked = schedule.blocked().next().expect("one blocked check");
        assert_eq!(blocked.decision(), ExecutionDecision::Denied);
        assert_eq!(blocked.blocked_by(), Some(Permission::RunProjectCode));
        assert!(!blocked.may_run());
        assert!(schedule.may_run().next().is_none());
        assert_eq!(schedule.may_run().count() + schedule.blocked().count(), 1);

        // The only status this module can produce for it is `skipped`, it is never
        // a pass, and the evidence class collapses to `unknown` because a check
        // that did not run established nothing.
        let fingerprint = FingerprintId::generate();
        let result = blocked
            .not_run(&fingerprint)
            .expect("a check that will not run has a result saying so");
        assert_eq!(result.status, CheckStatus::Skipped);
        assert!(!result.status.is_green());
        assert!(!result.status.produced_a_result());
        assert!(
            result.blocks_green(),
            "a critical check that never ran blocks"
        );
        assert_eq!(result.evidence_class, EvidenceClass::Unknown);
        assert_eq!(
            result.not_checked_reason,
            Some(NotCheckedReason::ExecutionNotAuthorized)
        );
        assert_eq!(result.id, *blocked.proposal().id());
        assert_eq!(result.project_fingerprint, fingerprint);
        assert_eq!(result.title, "the declared tests pass");

        // And the same shape for a check that is not critical: still skipped, still
        // not a pass, but it does not on its own forbid a green verdict. The two
        // results differ in exactly that one field.
        let ordinary = ScheduledCheck {
            proposal: CheckProposal::new(
                CheckId::generate(),
                "a nice-to-have",
                Severity::Note,
                false,
                EvidenceClass::DeterministicCheck,
                CheckReason::ProjectWide,
                &[ActionKind::RunTests],
            ),
            position: 0,
            decision: ExecutionDecision::Denied,
            blocked_by: Some(Permission::RunProjectCode),
        };
        let ordinary_result = ordinary
            .not_run(&fingerprint)
            .expect("it is blocked too, so it has a result");
        assert_eq!(ordinary_result.status, CheckStatus::Skipped);
        assert!(!ordinary_result.status.is_green());
        assert!(!ordinary_result.blocks_green());
        assert_ne!(
            result.blocks_green(),
            ordinary_result.blocks_green(),
            "criticality is the only field that differs, and it has to show"
        );
    }

    #[test]
    fn a_check_that_would_run_has_no_not_run_result_at_all() {
        let mut builder = a_builder();
        builder
            .propose(a_proposal(
                "reads the manifest",
                Severity::Note,
                EvidenceClass::ObservedFact,
                &[ActionKind::ReadMetadata],
            ))
            .unwrap();
        let schedule = builder.build();
        let scheduled = &schedule.checks()[0];
        assert!(scheduled.may_run());
        assert!(
            scheduled.not_run(&FingerprintId::generate()).is_none(),
            "a check that may run was given a result saying it did not"
        );
    }

    #[test]
    fn a_check_the_mode_stops_still_gets_a_result_saying_it_did_not_run() {
        // The counterpart of the test above, and written because a mutation found
        // nothing holding it: answering `may_run` from `blocked_by` instead of from
        // the decision survived the entire suite. The two agree whenever a check is
        // denied a permission and come apart exactly here — every permission
        // granted, the mode still refusing to run project code. A caller asking
        // `blocked_by` would be told the check runs and would produce no result for
        // it, so the check would *disappear* from the report instead of appearing
        // as one that did not happen, which is the failure this repository is built
        // to prevent and the one a green suite would not have shown.
        let mut builder = PlanBuilder::new(
            ExecutionMode::InspectOnly,
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        builder
            .propose(a_proposal(
                "runs the declared tests",
                Severity::MustFix,
                EvidenceClass::ObservedFact,
                &[ActionKind::RunTests],
            ))
            .unwrap();
        let schedule = builder.build();
        let scheduled = &schedule.checks()[0];

        assert_eq!(scheduled.decision(), ExecutionDecision::NeedsConsent);
        assert_eq!(
            scheduled.blocked_by(),
            None,
            "the user granted everything the check needs, so no permission stopped it"
        );
        assert!(
            !scheduled.may_run(),
            "the mode is what stopped this check, and it stops it without naming a permission"
        );

        let result = scheduled
            .not_run(&FingerprintId::generate())
            .expect("a check that will not run still owes the report a result saying so");
        assert_eq!(result.status, CheckStatus::Skipped);
        assert!(!result.status.is_green());
        assert!(
            !result.status.produced_a_result(),
            "a check that did not happen produced nothing"
        );

        // And the two halves of the schedule add up, which is the property the
        // caller actually relies on when it reports what was not done.
        assert_eq!(schedule.may_run().count(), 0);
        assert_eq!(schedule.blocked().count(), 1);
    }

    #[test]
    fn the_worst_of_a_checks_decisions_is_the_checks_decision() {
        // One action allowed and one denied: the check is denied, because a check
        // is a unit and half of it running is not the check running.
        let requirements =
            ExecutionRequirements::of(&[ActionKind::ReadFile, ActionKind::InstallDependencies]);
        let run_only = ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        };
        assert_eq!(
            requirements.decision(ExecutionMode::HostConfirmed, &run_only),
            ExecutionDecision::Denied
        );
        assert_eq!(
            requirements.blocked_by(&run_only),
            Some(Permission::InstallDependencies)
        );
        assert_eq!(
            requirements.permissions_missing(&run_only),
            [Permission::InstallDependencies]
        );

        // And the other direction: everything needed is granted.
        let granted = ExecutionPermissions {
            install_dependencies: true,
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        };
        assert_eq!(
            requirements.decision(ExecutionMode::HostConfirmed, &granted),
            ExecutionDecision::Allowed
        );
        assert_eq!(requirements.blocked_by(&granted), None);
        assert!(requirements.permissions_missing(&granted).is_empty());
    }

    #[test]
    fn a_check_missing_several_permissions_names_the_first_of_them() {
        // Written because nothing above reaches a check with more than one missing
        // permission, and `blocked_by` taking the *last* of them is the plausible
        // wrong answer that a single-missing case cannot tell apart from the right
        // one. The three actions are chosen so that their permissions are not
        // adjacent in `Permission::ALL`, which is what makes "first" a claim
        // rather than the same answer twice.
        let requirements = ExecutionRequirements::of(&[
            ActionKind::NetworkAccess,
            ActionKind::RunTests,
            ActionKind::InstallDependencies,
        ]);
        let missing = requirements.permissions_missing(&ExecutionPermissions::inspect_only());
        assert_eq!(
            missing,
            [
                Permission::RunProjectCode,
                Permission::InstallDependencies,
                Permission::Network
            ],
            "the missing permissions are reported in the product's own order"
        );
        assert_eq!(
            requirements.blocked_by(&ExecutionPermissions::inspect_only()),
            Some(Permission::RunProjectCode),
            "a refusal names the first permission that would change the answer, \
             and not the last one that happens to be missing"
        );

        // Granting the first one moves the answer to the next, one at a time, so
        // the order above is the order the refusals would be asked in.
        let run_granted = ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        };
        assert_eq!(
            requirements.blocked_by(&run_granted),
            Some(Permission::InstallDependencies)
        );
        let install_granted = ExecutionPermissions {
            run_project_code: true,
            install_dependencies: true,
            ..ExecutionPermissions::inspect_only()
        };
        assert_eq!(
            requirements.blocked_by(&install_granted),
            Some(Permission::Network)
        );

        // And the case that separates the two questions. With every permission
        // granted, no permission is named -- and the check still does not run,
        // because the mode is what is stopping it. A function that listed the
        // permission of every action that is not `Allowed` would name
        // `RunProjectCode` here, which the user has already granted.
        let all_granted = ExecutionPermissions {
            run_project_code: true,
            install_dependencies: true,
            network: true,
            ..ExecutionPermissions::inspect_only()
        };
        assert_eq!(requirements.blocked_by(&all_granted), None);
        assert!(requirements.permissions_missing(&all_granted).is_empty());
        assert_eq!(
            requirements.decision(ExecutionMode::InspectOnly, &all_granted),
            ExecutionDecision::NeedsConsent,
            "project code under inspect-only is a question, not a refusal"
        );
        assert_eq!(
            requirements.decision(ExecutionMode::HostConfirmed, &all_granted),
            ExecutionDecision::Allowed,
            "and the same permission set is enough once the mode allows the code"
        );
    }

    #[test]
    fn an_entry_says_which_of_the_three_things_happened_to_it() {
        // `plain_description` is what a report and a consent prompt show, and the
        // three outcomes are what a reader acts on differently: nothing to do, a
        // permission to grant, or a question to answer. The wording is asserted
        // rather than merely being non-empty, because "non-empty" is satisfied by
        // all three being the same sentence.
        let proposal = |title: &str, actions: &[ActionKind]| {
            CheckProposal::new(
                CheckId::generate(),
                title,
                Severity::MustFix,
                true,
                EvidenceClass::DeterministicCheck,
                CheckReason::ProjectWide,
                actions,
            )
        };
        let under = |mode: ExecutionMode, permissions: ExecutionPermissions| {
            let mut builder = PlanBuilder::new(mode, permissions);
            builder
                .propose(proposal("reads", &[ActionKind::ReadFile]))
                .unwrap();
            builder
                .propose(proposal("tests", &[ActionKind::RunTests]))
                .unwrap();
            builder
                .propose(proposal("installs", &[ActionKind::InstallDependencies]))
                .unwrap();
            builder.build()
        };
        let line_for = |schedule: &CheckSchedule, title: &str| {
            schedule
                .checks()
                .iter()
                .find(|scheduled| scheduled.proposal().title() == title)
                .expect("the check is in the plan")
                .plain_description()
        };

        // Inspect only: reading happens, running and installing are refused and
        // name a permission, and the two refusals name different ones.
        let inspect = under(
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        );
        assert!(line_for(&inspect, "reads").ends_with(", will run"));
        assert!(
            line_for(&inspect, "tests").ends_with(", will not run: run_project_code"),
            "{}",
            line_for(&inspect, "tests")
        );
        assert!(
            line_for(&inspect, "installs").ends_with(", will not run: install_dependencies"),
            "{}",
            line_for(&inspect, "installs")
        );

        // Every permission granted but the mode still inspect-only: nothing is
        // named, and the sentence must not pretend a permission is the problem.
        let granted = under(
            ExecutionMode::InspectOnly,
            ExecutionPermissions {
                run_project_code: true,
                install_dependencies: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        assert!(
            line_for(&granted, "tests").ends_with(", will run only if you agree"),
            "{}",
            line_for(&granted, "tests")
        );

        // And a mode that runs project code lets two of them run. The third still
        // does not, and still names the permission -- so the three outcomes are
        // reachable in one plan rather than being three modes' worth of cases.
        let host = under(
            ExecutionMode::HostConfirmed,
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        assert!(line_for(&host, "reads").ends_with(", will run"));
        assert!(line_for(&host, "tests").ends_with(", will run"));
        assert!(
            line_for(&host, "installs").ends_with(", will not run: install_dependencies"),
            "{}",
            line_for(&host, "installs")
        );

        // Every line still leads with the title and the reason, which is the part
        // the ordering rules and the acceptance are about.
        for line in host.plain_description() {
            assert!(
                line.starts_with("reads - ")
                    || line.starts_with("tests - ")
                    || line.starts_with("installs - "),
                "{line}"
            );
            assert!(
                line.contains("This is about the project as a whole."),
                "{line}"
            );
            assert!(line.contains("(deterministic_check)"), "{line}");
        }
    }

    #[test]
    fn a_denied_permission_is_worse_than_one_that_could_be_asked_about() {
        // `decide` gives `NeedsConsent` for an unrecognised command even when the
        // permission is granted, and `Denied` when it is not. A check holding one
        // of each is `Denied`: there is no prompt that would make it run.
        let requirements = ExecutionRequirements::of(&[ActionKind::ArbitraryCommand]);
        let granted = ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        };
        assert_eq!(
            requirements.decision(ExecutionMode::HostConfirmed, &granted),
            ExecutionDecision::NeedsConsent,
            "an unrecognised command is never covered by a blanket grant"
        );
        assert_eq!(
            requirements.decision(
                ExecutionMode::HostConfirmed,
                &ExecutionPermissions::inspect_only()
            ),
            ExecutionDecision::Denied
        );

        let mixed =
            ExecutionRequirements::of(&[ActionKind::ArbitraryCommand, ActionKind::RunTests]);
        assert_eq!(
            mixed.decision(
                ExecutionMode::HostConfirmed,
                &ExecutionPermissions::inspect_only()
            ),
            ExecutionDecision::Denied
        );

        // A project-code action in a mode that runs no project code needs asking
        // rather than being refused, which is the third branch of `decide`.
        let tests = ExecutionRequirements::of(&[ActionKind::RunTests]);
        assert_eq!(
            tests.decision(
                ExecutionMode::InspectOnly,
                &ExecutionPermissions {
                    run_project_code: true,
                    ..ExecutionPermissions::inspect_only()
                }
            ),
            ExecutionDecision::NeedsConsent
        );
    }

    #[test]
    fn the_permissions_a_check_needs_are_derived_from_its_actions() {
        let requirements = ExecutionRequirements::of(&[ActionKind::RunTests, ActionKind::ReadFile]);
        assert_eq!(
            requirements.permissions_needed(),
            [Permission::Inspect, Permission::RunProjectCode],
            "reading a file needs the inspect permission, so a check that reads \
             and runs needs both"
        );
        assert!(requirements.runs_project_code());
        assert!(!requirements.can_touch_network());
        assert!(!requirements.can_modify_disk());
        assert!(!requirements.is_inspection_only());
        assert!(!requirements.runs_nothing());

        // Deduplication is by the enum, so neither the order a caller listed the
        // actions in nor a repeat can change the requirements.
        let reordered = ExecutionRequirements::of(&[ActionKind::ReadFile, ActionKind::RunTests]);
        assert_eq!(requirements, reordered);
        let repeated = ExecutionRequirements::of(&[
            ActionKind::RunTests,
            ActionKind::ReadFile,
            ActionKind::RunTests,
        ]);
        assert_eq!(requirements, repeated);
        assert_eq!(repeated.actions().len(), 2);

        // A browser probe does not run the project's code and does reach outside
        // the machine, which is where `runs_nothing` and `is_inspection_only` come
        // apart -- and they are the same answer for a `LocalProbe`, which opens a
        // socket to this machine and is still read-only by every predicate the
        // domain has. The first draft of this test named `LocalProbe` and asserted
        // the two disagreed; they do not, and the fix is the action rather than the
        // assertion.
        let browser = ExecutionRequirements::of(&[ActionKind::BrowserProbe]);
        assert!(browser.runs_nothing());
        assert!(!browser.is_inspection_only());
        assert!(browser.can_touch_network());
        assert!(!browser.runs_project_code());
        assert_eq!(browser.permissions_needed(), [Permission::ConnectService]);

        let local = ExecutionRequirements::of(&[ActionKind::LocalProbe]);
        assert!(local.runs_nothing());
        assert!(local.is_inspection_only());
        assert_eq!(local.permissions_needed(), [Permission::Inspect]);

        // And the empty case is the one the builder refuses rather than schedules.
        assert!(ExecutionRequirements::none().is_empty());
        assert!(ExecutionRequirements::of(&[]).is_empty());
        assert_eq!(
            ExecutionRequirements::none(),
            ExecutionRequirements::of(&[])
        );
    }

    #[test]
    fn a_proposal_that_names_nothing_is_refused_rather_than_repaired() {
        let mut builder = a_builder();

        let no_title = CheckProposal::new(
            CheckId::generate(),
            "   ",
            Severity::Note,
            false,
            EvidenceClass::ObservedFact,
            CheckReason::ProjectWide,
            &[ActionKind::ReadFile],
        );
        assert!(matches!(
            builder.propose(no_title),
            Err(ProposalRefused::NoTitle { .. })
        ));

        let empty_reason = CheckProposal::new(
            CheckId::generate(),
            "a check",
            Severity::Note,
            false,
            EvidenceClass::ObservedFact,
            CheckReason::FilePresent {
                path: String::new(),
            },
            &[ActionKind::ReadFile],
        );
        assert!(matches!(
            builder.propose(empty_reason),
            Err(ProposalRefused::EmptyReason { .. })
        ));

        let no_actions = CheckProposal::new(
            CheckId::generate(),
            "a check",
            Severity::Note,
            false,
            EvidenceClass::ObservedFact,
            CheckReason::ProjectWide,
            &[],
        );
        assert!(matches!(
            builder.propose(no_actions),
            Err(ProposalRefused::NoActions { .. })
        ));

        // All three are remembered, so a caller that dropped the return value still
        // has somewhere to look, and the plan holds none of them.
        assert_eq!(builder.refused().len(), 3);
        let schedule = builder.build();
        assert!(schedule.is_empty());

        // Each refusal names the check it is about and says what was wrong, in
        // words a caller can print. The identifier is not spelled out here beyond
        // its prefix -- `chk_` is `ids.rs`'s to choose and this test should not be
        // a second place that has to be edited if it changes.
        let refused_id = CheckId::generate();
        for (refusal, expected) in [
            (
                ProposalRefused::NoTitle {
                    id: refused_id.clone(),
                },
                "has no title",
            ),
            (
                ProposalRefused::EmptyReason {
                    id: refused_id.clone(),
                },
                "names no file",
            ),
            (
                ProposalRefused::NoActions {
                    id: refused_id.clone(),
                },
                "declares no action",
            ),
        ] {
            let rendered = refusal.to_string();
            assert!(rendered.contains(refused_id.as_str()), "{rendered}");
            assert!(rendered.contains(expected), "{rendered}");
            assert!(refused_id.as_str().starts_with("chk_"), "{refused_id}");
        }
    }

    #[test]
    fn the_same_check_proposed_twice_is_scheduled_once_and_the_second_is_reported() {
        let id = CheckId::generate();
        let proposal = CheckProposal::new(
            id.clone(),
            "reads the manifest",
            Severity::Note,
            false,
            EvidenceClass::ObservedFact,
            CheckReason::ProjectWide,
            &[ActionKind::ReadMetadata],
        );

        let mut builder = a_builder();
        builder.propose(proposal.clone()).unwrap();
        // The same identifier with a different title, which is the case that
        // matters: deduplicating by value would keep both and produce two results
        // for one check.
        builder
            .propose(CheckProposal::new(
                id.clone(),
                "a different title entirely",
                Severity::MustFix,
                true,
                EvidenceClass::DeterministicCheck,
                CheckReason::ProjectWide,
                &[ActionKind::RunTests],
            ))
            .unwrap();

        let schedule = builder.build();
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.duplicates(), std::slice::from_ref(&id));
        assert_eq!(
            schedule.checks()[0].proposal().title(),
            "reads the manifest",
            "the first proposal keeps its place"
        );
        assert_eq!(schedule.get(&id).map(ScheduledCheck::position), Some(0));

        // A third copy is still one duplicate entry and not two, because the list
        // is per check rather than per extra proposal.
        let mut builder = a_builder();
        for _ in 0..3 {
            builder.propose(proposal.clone()).unwrap();
        }
        let schedule = builder.build();
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.duplicates(), std::slice::from_ref(&id));
    }

    #[test]
    fn the_schedule_hands_enforcement_its_checks_in_the_same_order() {
        // The bridge, checked rather than asserted: `planned_checks` and `checks`
        // must not be able to disagree about the order, because the order a report
        // shows is decided here and read there.
        let mut builder = a_builder();
        for (title, severity, action) in [
            ("c", Severity::Note, ActionKind::RunTests),
            ("a", Severity::MustFix, ActionKind::ReadFile),
            ("b", Severity::MustFix, ActionKind::Build),
        ] {
            builder
                .propose(a_proposal(
                    title,
                    severity,
                    EvidenceClass::DeterministicCheck,
                    &[action],
                ))
                .unwrap();
        }

        let schedule = builder.build();
        let planned = schedule.planned_checks();
        assert_eq!(planned.len(), schedule.len());
        for (index, (scheduled, planned)) in schedule.checks().iter().zip(&planned).enumerate() {
            assert_eq!(scheduled.position(), index);
            assert_eq!(scheduled.proposal().id(), planned.id());
            assert_eq!(scheduled.proposal().title(), planned.title());
            assert_eq!(scheduled.proposal().severity(), planned.severity());
            assert_eq!(scheduled.proposal().critical(), planned.critical());
        }

        // And `Enforcement` accepts them: the two types meet at this call, and the
        // frozen plan holds exactly the checks this schedule says would run.
        let permissions = crate::consent::PermissionPlan::new(
            ExecutionMode::HostConfirmed,
            FingerprintId::generate(),
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        let enforcement =
            crate::enforce::Enforcement::of("plan-1", permissions, &schedule.planned_checks());
        assert_eq!(
            enforcement.check_plan().all_checks().len(),
            schedule.may_run().count()
        );
        assert!(enforcement.unscheduled().is_empty());
    }

    #[test]
    fn an_empty_schedule_is_a_real_answer() {
        let schedule = PlanBuilder::new(
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        )
        .build();
        assert!(schedule.is_empty());
        assert_eq!(schedule.len(), 0);
        assert!(schedule.plain_description().is_empty());
        assert!(schedule.duplicates().is_empty());
        assert!(schedule.planned_checks().is_empty());
        assert_eq!(schedule.mode(), ExecutionMode::InspectOnly);
        assert!(schedule.may_run().next().is_none());
        assert!(schedule.blocked().next().is_none());
        assert!(schedule.get(&CheckId::generate()).is_none());
    }

    #[test]
    fn every_reason_says_what_it_is_about_and_which_ones_name_nothing() {
        let reasons = [
            CheckReason::DeclaredCommand {
                declared_in: "package.json".to_owned(),
                command: "npm test".to_owned(),
            },
            CheckReason::StackPresent {
                component: "web".to_owned(),
                stack: "node".to_owned(),
            },
            CheckReason::FilePresent {
                path: "Cargo.toml".to_owned(),
            },
            CheckReason::ProjectWide,
        ];
        let mut sentences: Vec<String> = Vec::new();
        for reason in &reasons {
            let sentence = reason.plain_description();
            assert!(!sentence.trim().is_empty(), "{reason:?} says nothing");
            assert!(
                reason.names_something(),
                "{reason:?} was built naming nothing"
            );
            // Each reason says something the others do not, so a report showing
            // one where another belongs is a visible difference rather than four
            // ways of saying "a check".
            assert!(
                !sentences.contains(&sentence),
                "two reasons render identically: {sentence}"
            );
            sentences.push(sentence);
        }

        // The detail travels into the sentence rather than being described
        // generically, which is the whole point of the variants.
        assert!(reasons[0].plain_description().contains("npm test"));
        assert!(reasons[0].plain_description().contains("package.json"));
        assert!(reasons[1].plain_description().contains("web"));
        assert!(reasons[2].plain_description().contains("Cargo.toml"));

        // And the empty spellings are the ones `names_something` refuses.
        for reason in [
            CheckReason::DeclaredCommand {
                declared_in: String::new(),
                command: "npm test".to_owned(),
            },
            CheckReason::DeclaredCommand {
                declared_in: "package.json".to_owned(),
                command: "   ".to_owned(),
            },
            CheckReason::StackPresent {
                component: "web".to_owned(),
                stack: "  ".to_owned(),
            },
            CheckReason::FilePresent {
                path: String::new(),
            },
        ] {
            assert!(
                !reason.names_something(),
                "{reason:?} should not count as naming something"
            );
        }
    }
}

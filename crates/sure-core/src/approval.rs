//! Host-confirmed execution: what the user approved, what may run because of it,
//! and where that answer is kept.
//!
//! `P3-T006` acceptance: *"Only approved command categories execute."* and
//! *"Approval state is locally auditable."*
//!
//! # What was missing, and what this fills
//!
//! [`crate::consent`] decides, per command, whether the mode and the granted
//! permissions allow it — and one of its three answers is
//! [`NeedsConsent`](sure_domain::execution::ExecutionDecision::NeedsConsent), a
//! question nothing asks. The domain declares the record that answers it,
//! [`HostConsent`], and **nothing constructs one**. It declares
//! [`ConsentGrantor::can_grant`], and **nothing calls it**. And it has a word for
//! a refusal, [`NotCheckedReason::UserDeclined`], that **no code produces**.
//!
//! This module is the missing half, and it is three pieces that only work
//! together:
//!
//! 1. [`ConsentRequest`] is what the user is shown — the plan's askable commands,
//!    rendered **once**, so that what is displayed and what is recorded are the
//!    same values rather than two readings of one plan.
//! 2. [`ConsentRequest::approve`] builds the [`HostConsent`], and refuses to build
//!    one from a grantor that cannot grant.
//! 3. [`authorise`] is the gate. It decides, per command, which of the plan's
//!    commands may run, and refuses the rest with a reason that is never a pass.
//!
//! # The rules, and one of them is the acceptance sentence
//!
//! **A command may run if the permissions cover it, or if the consent names it and
//! the categories the user was shown cover the categories it now falls into.** That
//! is *"only approved command categories execute"*, and the second half is the half
//! worth reading. A command whose argument vector differs from the approved one is
//! refused; a command approved under another working directory is refused; a
//! consent whose grantor cannot grant is refused outright; and a command that has
//! come to fall into a category the user was not shown is refused even though the
//! command line is identical.
//!
//! **Why the category comparison is not vacuous, since SURE's classifier is
//! deterministic.** [`crate::safety::classify`] is a pure function of the program
//! and the argument vector, so within one build the recorded reading and the live
//! one always agree — and a check that compared them would be comparing a value
//! with itself. What makes it a check is that **an approval is a record that
//! outlives the build that wrote it**. It is written to disk, it is read back by a
//! later SURE, and the question *may this run* is then put to a classifier that may
//! since have learned something. `git clean` approved as destruction and later read
//! as destruction is the same answer; the same command line approved as *static*
//! and now read as destruction is not, and it is exactly the case a record without
//! categories would sail past.
//!
//! **A refusal here gives a producer to a word the vocabulary was holding.** A
//! command that was shown and not approved is [`Refusal::Declined`], and
//! `NotCheckedReason::UserDeclined` is what it becomes. `crate::consent` says why it
//! did not use that word — *"nothing has been declined, and a report that said so
//! would be inventing an event — the prompt that can be declined is `P3-T006`'s"*.
//! This is that prompt. A command that was never shown, because SURE could not
//! render it, is a different refusal ([`Refusal::CannotBeShown`]) and deliberately
//! not the same word.
//!
//! # A refused command is never a pass, and that is structural
//!
//! [`Standing::Refused`] carries a [`Refusal`], and the only way this module turns
//! one into a [`CheckResult`] is [`Authorisation::refusals`] — every value of which
//! is [`CheckResult::not_run`]: `Skipped`, with its reason and its weight kept.
//! There is no constructor here that produces a result for a command that did not
//! run, and [`Authorisation::admitted`] is the only door to the commands that may.
//!
//! # Local auditability
//!
//! The second acceptance sentence. A consent that lives only as long as the process
//! that built it is not auditable, so [`recorded`] reads back what
//! [`Store::append_approval`] wrote, under a [`RecordKind::Approval`] of its own — a
//! schema-less kind, in history rather than out of it, naming the project state it
//! was given for. [`RecordedApproval::command_for`] is the question an auditor
//! actually asks: *was this exact command approved, and when.*
//!
//! # What this does not establish
//!
//! **Nothing has been run.** This module builds the gate and the record; it does not
//! open it. There is still no caller of `sure_core::process::run`, and
//! `sure_core::support`'s ceiling of *inspect only* is still true. The census that
//! ceiling rests on gained a site in `P5-T004` — the browser launcher — and gained
//! nothing that reaches it: the list is a list of ways SURE *could* run something,
//! and what keeps the ceiling true is that no product path takes any of them. The
//! day something runs a command, that changes, and it changes in the same commit as
//! this paragraph: they are the two places that would otherwise go on claiming
//! nothing executes.
//!
//! **Nothing about what an approved command will do.** Approving `cargo test`
//! approves a command line, not the code behind it. [`crate::safety`] says the same
//! thing from its side.
//!
//! **Nothing about whether the user understood the question.** This records what was
//! shown and what was answered. The two are the same values by construction, which
//! is as much as a program can establish about a person.
//!
//! **Nothing about a prompt having been displayed.** [`ConsentRequest`] is by
//! definition what SURE shows; a caller that builds one and never displays it has a
//! bug this module cannot see, and [`Refusal::Declined`] would then be reporting a
//! decline that never happened.

use std::fmt;

use sure_domain::execution::{
    ApprovedCommand, CommandClass, CommandEffects, ConsentGrantor, ExecutionMode, HostConsent,
};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::status::{CheckResult, NotCheckedReason};

use crate::consent::{PermissionPlan, PlannedCheck, PlannedCommand};
use crate::store::{HistoryFilter, RecordKind, Store, StoreError, StoredRecord};

/// Why a command is a question rather than a decision.
///
/// The two ways a command reaches a prompt, and the distinction is not decoration:
/// under one of them a request can be non-empty and under the other it never is, so
/// a caller that had the wrong one would be asking a user to grant a permission for
/// something no permission covers.
///
/// **The two cannot both be true of one plan.** `crate::consent::decide_for` asks
/// about the ungrantable category *before* the mode, so a destructive command under
/// a mode that runs nothing is `Denied` rather than `NeedsConsent` — it is not a
/// question at all, and [`ConsentRequest::of`] leaves it out. The two reasons
/// therefore never appear in the same request, which is a fact about the plan rather
/// than a rule of this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhyAsked {
    /// No permission SURE can ask for covers one of its categories.
    ///
    /// The category is [`CommandClass::Destructive`] and it is not always
    /// destruction: SURE answers *destructive* for a command whose reading is wider
    /// than anything it can bound, which includes a command it could not read at
    /// all. Either way there is nothing to grant, so the answer has to name this
    /// command.
    NoPermissionCovers(CommandClass),
    /// The mode does not run project code, and this command would.
    ///
    /// A question a permission could have answered; the mode is what makes it a
    /// question instead.
    ModeDoesNotRunProjectCode,
}

impl WhyAsked {
    /// The line a prompt adds for this reason, where the plan does not already
    /// carry one.
    ///
    /// [`NoPermissionCovers`](Self::NoPermissionCovers) answers `None`, and that is
    /// the point rather than an omission: [`PlannedCommand::explain`] already emits
    /// that sentence word for word, because `P3-T005` owns it. A second copy here
    /// would be a second sentence that has to stay in step with the first, which is
    /// how two sentences that must agree stop agreeing.
    /// [`ModeDoesNotRunProjectCode`](Self::ModeDoesNotRunProjectCode) has no other
    /// producer — the plan explains what a command needs, not what the mode refuses
    /// — so this is where it is said.
    #[must_use]
    pub fn line(self) -> Option<String> {
        match self {
            Self::NoPermissionCovers(_) => None,
            Self::ModeDoesNotRunProjectCode => Some(format!(
                "{} You can answer for this command.",
                ExecutionMode::InspectOnly.plain_description()
            )),
        }
    }
}

/// One command SURE is asking about, rendered once.
///
/// The program and the arguments are stored as text rather than re-derived from
/// [`Self::command`] at each use, because a second reading is a second chance to
/// differ. What the prompt displays, what the record stores and what the gate
/// compares are this one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedCommand {
    index: usize,
    command: PlannedCommand,
    program: String,
    arguments: Vec<String>,
}

impl RequestedCommand {
    /// Where this command sits in the plan it came from.
    ///
    /// An index rather than a search by check identity, because one check can plan
    /// two commands — `npm ci` and then `npm test` are one check — and two approvals
    /// under one check are then two different commands rather than a duplicate.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// The planned command this is the question about.
    #[must_use]
    pub const fn command(&self) -> &PlannedCommand {
        &self.command
    }

    /// The check the command belongs to.
    #[must_use]
    pub const fn check(&self) -> &PlannedCheck {
        self.command.check()
    }

    /// The program, as the user is shown it and as the record keeps it.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The arguments, one element each, as the user is shown them.
    #[must_use]
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    /// The categories SURE read the command as.
    #[must_use]
    pub fn effects(&self) -> &CommandEffects {
        self.command.effects()
    }

    /// Why this command is a question.
    #[must_use]
    pub fn why(&self) -> WhyAsked {
        match self.command.ungrantable() {
            Some(class) => WhyAsked::NoPermissionCovers(class),
            None => WhyAsked::ModeDoesNotRunProjectCode,
        }
    }

    /// The command line as a person should read it.
    #[must_use]
    pub fn display(&self) -> String {
        self.command.display()
    }

    /// What the prompt says about this command.
    ///
    /// [`PlannedCommand::explain`]'s own lines, plus [`WhyAsked::line`] where the
    /// plan has not already said it — so that the prompt and the plan cannot explain
    /// the same command two ways, and the one sentence they share is written once.
    #[must_use]
    pub fn explain(&self) -> Vec<String> {
        let mut lines = self.command.explain();
        lines.extend(self.why().line());
        lines
    }
}

/// What SURE is about to ask the user, and what it will record if they agree.
///
/// Built from a [`PermissionPlan`] by [`ConsentRequest::of`], which takes the plan's
/// commands that came back
/// [`NeedsConsent`](sure_domain::execution::ExecutionDecision::NeedsConsent) and no
/// others: a command the permissions already cover is not a question, and a command
/// the plan refused is not one either — it was refused *because* no answer could
/// help.
///
/// It **owns** the plan rather than borrowing it. The gate needs the plan and the
/// request together — one to know what may run at all, the other to know what was
/// asked — and taking the plan by value means a caller cannot hold the plan it asked
/// about and a differently-built plan side by side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentRequest {
    plan: PermissionPlan,
    working_directory: String,
    asked: Vec<RequestedCommand>,
    unrenderable: Vec<(usize, CheckId)>,
}

impl ConsentRequest {
    /// Everything in the plan that SURE has to ask about.
    ///
    /// The working directory is one value for the whole request, because that is
    /// what it is: a plan is a set of commands for one project at one state, and
    /// [`ApprovedCommand::working_directory`] records where they run the same way
    /// [`PermissionPlan`] holds one fingerprint.
    ///
    /// A command whose program or any argument is not text SURE can read cannot be
    /// shown, so it cannot be approved, and it is reported by [`Self::unrenderable`]
    /// rather than dropped: a question that quietly loses a command is a prompt the
    /// user answered without being asked about all of it.
    #[must_use]
    pub fn of(plan: PermissionPlan, working_directory: impl Into<String>) -> Self {
        let working_directory = working_directory.into();
        let mut asked = Vec::new();
        let mut unrenderable = Vec::new();

        for (index, command) in plan.commands().iter().enumerate() {
            if !command.needs_consent() {
                continue;
            }
            match render(command) {
                Some((program, arguments)) => asked.push(RequestedCommand {
                    index,
                    command: command.clone(),
                    program,
                    arguments,
                }),
                None => unrenderable.push((index, command.check().id().clone())),
            }
        }

        Self {
            plan,
            working_directory,
            asked,
            unrenderable,
        }
    }

    /// The plan these questions are about.
    #[must_use]
    pub const fn plan(&self) -> &PermissionPlan {
        &self.plan
    }

    /// The mode the plan was built under.
    #[must_use]
    pub const fn mode(&self) -> ExecutionMode {
        self.plan.mode()
    }

    /// The project state the plan was built for.
    #[must_use]
    pub const fn fingerprint(&self) -> &FingerprintId {
        self.plan.fingerprint()
    }

    /// The directory the approved commands would run in.
    #[must_use]
    pub fn working_directory(&self) -> &str {
        &self.working_directory
    }

    /// Every command SURE is asking about, in the order the plan holds them.
    #[must_use]
    pub fn commands(&self) -> &[RequestedCommand] {
        &self.asked
    }

    /// Whether there is nothing to ask.
    ///
    /// True for a request whose commands were all renderable and none of which
    /// needed an answer — which is not the same as a plan that will run, and is not
    /// reported as one.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.asked.is_empty() && self.unrenderable.is_empty()
    }

    /// Commands that need an answer and could not be shown, with their places.
    ///
    /// Each is `(index into the plan's commands, the check it belongs to)`.
    #[must_use]
    pub fn unrenderable(&self) -> &[(usize, CheckId)] {
        &self.unrenderable
    }

    /// The question this request is, in the words a person reads.
    ///
    /// `docs/architecture/EXECUTION_SAFETY.md`: *"SURE must show what it intends to
    /// run when approval is required."* The commands' own lines are
    /// [`PlannedCommand::explain`]'s, so this is the plan's explanation with the
    /// askable commands gathered rather than the plan's explanation restated.
    #[must_use]
    pub fn explain(&self) -> Vec<String> {
        if self.is_empty() {
            return vec![
                "Nothing in this plan needs your approval: the permissions you have granted \
                 already cover every command it runs."
                    .to_owned(),
            ];
        }

        let mut lines = vec![format!(
            "SURE is asking about {} command(s) in {}, before any of them runs.",
            self.asked.len(),
            self.working_directory
        )];
        for command in &self.asked {
            lines.extend(command.explain());
        }
        for (_, check) in &self.unrenderable {
            lines.push(format!(
                "SURE could not show you one command in this plan, so it cannot be approved and \
                 will not run (check {check})."
            ));
        }
        lines
    }

    /// Build the record, for a user who said yes to everything they were shown.
    ///
    /// **Everything asked about is approved, or nothing is.** What a user who said
    /// no to part of it produces is an approval list that omits those commands,
    /// which is [`Self::approve_some`]'s job — a separate door, so that the common
    /// case is the one a caller has to type.
    ///
    /// # Errors
    ///
    /// [`ApprovalError::GrantorCannotGrant`] if `granted_by` is not a grantor that
    /// may grant execution authority. [`ConsentGrantor::can_grant`] is false for
    /// `ProjectRequestEscalated`, and a project file asking for authority is not the
    /// same as authority being given. `ADR 0009` is the rule; this is the first
    /// caller of the method that states it.
    pub fn approve(
        &self,
        granted_by: ConsentGrantor,
        granted_at: impl Into<String>,
    ) -> Result<HostConsent, ApprovalError> {
        self.approve_some(&self.asked, granted_by, granted_at)
    }

    /// Build the record for a user who approved part of what they were shown.
    ///
    /// The commands not in `approved` are the ones that were declined, and the gate
    /// reports them as such.
    ///
    /// # Errors
    ///
    /// As [`Self::approve`].
    pub fn approve_some<'a>(
        &self,
        approved: impl IntoIterator<Item = &'a RequestedCommand>,
        granted_by: ConsentGrantor,
        granted_at: impl Into<String>,
    ) -> Result<HostConsent, ApprovalError> {
        if !granted_by.can_grant() {
            return Err(ApprovalError::GrantorCannotGrant(granted_by));
        }
        Ok(HostConsent {
            approved_commands: approved
                .into_iter()
                .map(|command| ApprovedCommand {
                    program: command.program.clone(),
                    args: command.arguments.clone(),
                    working_directory: self.working_directory.clone(),
                    check: command.check().id().clone(),
                    effects: command.effects().clone(),
                })
                .collect(),
            granted_at: granted_at.into(),
            granted_by,
        })
    }

    /// The question asked about the command at `index`, if there was one.
    fn asked_at(&self, index: usize) -> Option<&RequestedCommand> {
        self.asked.iter().find(|asked| asked.index == index)
    }
}

/// Why a consent record could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalError {
    /// The grantor is not one that may grant execution authority.
    GrantorCannotGrant(ConsentGrantor),
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GrantorCannotGrant(grantor) => write!(
                formatter,
                "{grantor:?} cannot grant execution authority: a project file may ask for a \
                 privileged behaviour, and asking is not granting"
            ),
        }
    }
}

impl std::error::Error for ApprovalError {}

/// Why a command did not run.
///
/// Every value here becomes a `Skipped` result with its weight kept, and none of
/// them is a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The consent was not given by anyone who can give one.
    ///
    /// Refuses **every** command in the plan, including the ones the consent names
    /// exactly. A record is a value that can be read off a disk, and the check is on
    /// the record rather than only on the code that builds one for that reason:
    /// [`ConsentRequest::approve`] will not make one of these, and a stored one can
    /// still arrive.
    GrantorCannotGrant { grantor: ConsentGrantor },
    /// The plan itself refuses: a permission is missing and no prompt can help.
    ///
    /// Carried rather than recomputed, so that the reason a user reads is the reason
    /// `crate::consent` produced.
    NotPermitted { reason: NotCheckedReason },
    /// The command needs its own approval and SURE could not show it to anyone.
    CannotBeShown { check: CheckId },
    /// The user was shown the command and did not approve it.
    Declined { check: CheckId },
    /// The consent approves a different command under this check.
    NotTheApprovedCommand { approved: String, planned: String },
    /// The consent approves the command in a different directory.
    DifferentWorkingDirectory { approved: String, planned: String },
    /// The command falls into a category the user was not shown.
    ///
    /// The acceptance sentence, as a refusal: the command line is exactly the one
    /// that was approved, and the reading is not the one that was approved.
    CategoryNotApproved {
        approved: CommandEffects,
        planned: CommandEffects,
    },
}

impl Refusal {
    /// The frozen reason this refusal is reported under.
    ///
    /// [`NotCheckedReason::UserDeclined`] appears here and nowhere else in the
    /// product, and it is the reason the vocabulary was holding: a command that was
    /// shown and not approved is a prompt that was declined, which is the event the
    /// word describes. Every other refusal is
    /// [`NotCheckedReason::ExecutionNotAuthorized`] — SURE is not authorized to run
    /// it, which is true whether the obstacle is the grantor, the argument vector or
    /// a category.
    ///
    /// [`Refusal::NotPermitted`] does not decide: it carries the reason
    /// `crate::consent` already chose, which is the permission's own word rather
    /// than this module's.
    #[must_use]
    pub const fn reason(&self) -> NotCheckedReason {
        match self {
            Self::NotPermitted { reason } => *reason,
            Self::Declined { .. } => NotCheckedReason::UserDeclined,
            Self::GrantorCannotGrant { .. }
            | Self::CannotBeShown { .. }
            | Self::NotTheApprovedCommand { .. }
            | Self::DifferentWorkingDirectory { .. }
            | Self::CategoryNotApproved { .. } => NotCheckedReason::ExecutionNotAuthorized,
        }
    }

    /// The refusal in the words a report shows.
    #[must_use]
    pub fn explain(&self) -> String {
        match self {
            Self::GrantorCannotGrant { grantor } => format!(
                "The approval this command would rest on came from {grantor:?}, which is not a \
                 source that can grant execution authority."
            ),
            Self::NotPermitted { reason } => reason.plain_explanation().to_owned(),
            Self::CannotBeShown { check } => format!(
                "SURE could not show you this command, so it cannot be approved and will not run \
                 (check {check})."
            ),
            Self::Declined { check } => {
                format!("You were shown this command and did not approve it (check {check}).")
            }
            Self::NotTheApprovedCommand { approved, planned } => format!(
                "You approved a different command for this check: {approved:?} was approved and \
                 {planned:?} is planned. Nothing is re-parsed and nothing runs under an approval \
                 for something else."
            ),
            Self::DifferentWorkingDirectory { approved, planned } => format!(
                "You approved this command in {approved:?}, and it would run in {planned:?}."
            ),
            Self::CategoryNotApproved { approved, planned } => format!(
                "You approved this command as {approved}. It now reads as {planned}, and a \
                 category you were not shown is not covered by what you agreed to."
            ),
        }
    }
}

/// What may happen to one command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// The granted permissions cover it; no approval was needed.
    Permitted,
    /// The user approved this exact command, with these categories.
    Approved,
    /// It may not run.
    Refused(Refusal),
}

impl Standing {
    /// Whether this command may run.
    #[must_use]
    pub const fn is_admitted(&self) -> bool {
        matches!(self, Self::Permitted | Self::Approved)
    }

    /// Why it may not, when it may not.
    #[must_use]
    pub const fn refusal(&self) -> Option<&Refusal> {
        match self {
            Self::Refused(refusal) => Some(refusal),
            Self::Permitted | Self::Approved => None,
        }
    }

    /// One clause a report can print.
    #[must_use]
    pub const fn plain_description(&self) -> &'static str {
        match self {
            Self::Permitted => "the permissions you granted cover it",
            Self::Approved => "you approved this exact command",
            Self::Refused(_) => "it will not run",
        }
    }
}

/// Every command in a plan and what may happen to it.
///
/// Produced by [`authorise`], and the only way to reach a command that may run.
#[derive(Debug, Clone)]
pub struct Authorisation<'a> {
    request: &'a ConsentRequest,
    consent: &'a HostConsent,
    standings: Vec<Standing>,
}

impl<'a> Authorisation<'a> {
    /// The request this was decided against.
    #[must_use]
    pub const fn request(&self) -> &'a ConsentRequest {
        self.request
    }

    /// The consent it was decided with.
    #[must_use]
    pub const fn consent(&self) -> &'a HostConsent {
        self.consent
    }

    /// Every planned command with its standing, in the plan's own order.
    pub fn commands(&self) -> impl Iterator<Item = (&'a PlannedCommand, &Standing)> {
        self.request
            .plan()
            .commands()
            .iter()
            .zip(self.standings.iter())
    }

    /// The commands that may run.
    ///
    /// The only door to a command a caller could start. It is not a claim that
    /// anything has run, and `sure_core::process` is still called by nothing.
    pub fn admitted(&self) -> impl Iterator<Item = &'a PlannedCommand> {
        self.commands()
            .filter(|(_, standing)| standing.is_admitted())
            .map(|(command, _)| command)
    }

    /// The commands that may not, with why.
    pub fn refused(&self) -> impl Iterator<Item = (&'a PlannedCommand, &Refusal)> {
        self.commands()
            .filter_map(|(command, standing)| standing.refusal().map(|refusal| (command, refusal)))
    }

    /// A result for every command that will not run.
    ///
    /// Each one is `Skipped` with a reason and its weight kept, in the order the
    /// plan holds them. A plan whose commands all run returns an empty list — which
    /// is not a green result and is not treated as one: this says nothing about
    /// whether any check passed.
    #[must_use]
    pub fn refusals(&self) -> Vec<CheckResult> {
        self.refused()
            .map(|(command, refusal)| {
                CheckResult::not_run(
                    command.check().id().clone(),
                    command.check().title().to_owned(),
                    command.check().severity(),
                    command.check().critical(),
                    refusal.reason(),
                    self.request.fingerprint().clone(),
                )
            })
            .collect()
    }

    /// Every category the commands that may run could fall into.
    ///
    /// **The first acceptance sentence, as a value.** A caller about to run
    /// something can read this set and compare it against what the user agreed to,
    /// without re-deriving it from the plan and the consent and getting a second
    /// answer. It is the set of *possible* effects of the commands that were
    /// admitted — not a prediction of what they will do, which is what
    /// [`crate::safety`] keeps saying it cannot give.
    #[must_use]
    pub fn admitted_categories(&self) -> Vec<CommandClass> {
        let admitted: Vec<&PlannedCommand> = self.admitted().collect();
        CommandClass::ALL
            .iter()
            .copied()
            .filter(|class| {
                admitted
                    .iter()
                    .any(|command| command.effects().contains(*class))
            })
            .collect()
    }

    /// The whole decision, in the words a report prints.
    #[must_use]
    pub fn explain(&self) -> Vec<String> {
        let total = self.standings.len();
        let admitted = self
            .standings
            .iter()
            .filter(|standing| standing.is_admitted())
            .count();
        let mut lines = vec![format!(
            "SURE may run {admitted} of the {total} command(s) this plan holds, in mode {}.",
            self.request.mode().as_str()
        )];
        for (command, standing) in self.commands() {
            match standing.refusal() {
                Some(refusal) => lines.push(format!(
                    "{} — will not run. {}",
                    command.display(),
                    refusal.explain()
                )),
                None => lines.push(format!(
                    "{} — {}. Read as {}.",
                    command.display(),
                    standing.plain_description(),
                    command.effects()
                )),
            }
        }
        lines
    }
}

/// Decide, for every command in a request's plan, whether it may run.
///
/// The gate. See the module documentation for the rules, and
/// [`Authorisation::refusals`] for what a refusal becomes.
#[must_use]
pub fn authorise<'a>(request: &'a ConsentRequest, consent: &'a HostConsent) -> Authorisation<'a> {
    let standings = request
        .plan()
        .commands()
        .iter()
        .enumerate()
        .map(|(index, command)| standing_for(request, consent, index, command))
        .collect();
    Authorisation {
        request,
        consent,
        standings,
    }
}

/// What may happen to one command, by the module's rules.
fn standing_for(
    request: &ConsentRequest,
    consent: &HostConsent,
    index: usize,
    command: &PlannedCommand,
) -> Standing {
    if command.is_allowed() {
        return Standing::Permitted;
    }
    // The grantor first, and before anything is matched. A record whose grantor
    // cannot grant is not a consent with a defect in it; it is not a consent. It
    // refuses every command, including the ones it names exactly.
    if !consent.granted_by.can_grant() {
        return Standing::Refused(Refusal::GrantorCannotGrant {
            grantor: consent.granted_by,
        });
    }
    // The plan's own refusal, which no answer can change: a missing permission is
    // not a question. `Denied` is the only non-allowed decision that is not
    // `NeedsConsent`, so this is exhaustive without a wildcard.
    if !command.needs_consent() {
        return Standing::Refused(Refusal::NotPermitted {
            reason: command
                .reason()
                .unwrap_or(NotCheckedReason::ExecutionNotAuthorized),
        });
    }
    let Some(asked) = request.asked_at(index) else {
        return Standing::Refused(Refusal::CannotBeShown {
            check: command.check().id().clone(),
        });
    };

    let under_this_check: Vec<&ApprovedCommand> = consent
        .approved_commands
        .iter()
        .filter(|approved| approved.check == *command.check().id())
        .collect();
    if under_this_check.is_empty() {
        return Standing::Refused(Refusal::Declined {
            check: command.check().id().clone(),
        });
    }
    // Matched on the command line rather than taken as the first approval for the
    // check, because one check can plan two commands and the second of them is not
    // approved by the first one's answer.
    let Some(approved) = under_this_check
        .iter()
        .find(|approved| approved.program == asked.program() && approved.args == asked.arguments())
    else {
        return Standing::Refused(Refusal::NotTheApprovedCommand {
            approved: under_this_check[0].display(),
            planned: asked.display(),
        });
    };

    if approved.working_directory != request.working_directory() {
        return Standing::Refused(Refusal::DifferentWorkingDirectory {
            approved: approved.working_directory.clone(),
            planned: request.working_directory().to_owned(),
        });
    }
    if !approved.effects.covers(command.effects()) {
        return Standing::Refused(Refusal::CategoryNotApproved {
            approved: approved.effects.clone(),
            planned: command.effects().clone(),
        });
    }
    Standing::Approved
}

/// The program and arguments as text, or `None` where one of them is not text.
///
/// The one place a planned command becomes the strings a record keeps. A caller
/// that built its own strings and a record that kept them would be two readings of
/// one command line.
fn render(command: &PlannedCommand) -> Option<(String, Vec<String>)> {
    let program = command.program().to_str()?.to_owned();
    let mut arguments = Vec::with_capacity(command.arguments().len());
    for argument in command.arguments() {
        arguments.push(argument.to_str()?.to_owned());
    }
    Some((program, arguments))
}

/// One recorded approval, read back out of the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedApproval {
    id: i64,
    written_at_ms: i64,
    consent: HostConsent,
}

impl RecordedApproval {
    /// The row's identity, stable for the life of the file.
    #[must_use]
    pub const fn id(&self) -> i64 {
        self.id
    }

    /// When the row was written, as milliseconds since the epoch.
    ///
    /// **This is when SURE wrote it down, which is not the same instant as
    /// [`HostConsent::granted_at`].** `granted_at` is what the user was told the
    /// time was, in their own terms; this is the store's clock. An audit that found
    /// them disagreeing has found something worth looking at, which is why the two
    /// are kept apart rather than reconciled.
    #[must_use]
    pub const fn written_at_ms(&self) -> i64 {
        self.written_at_ms
    }

    /// What the user agreed to.
    #[must_use]
    pub const fn consent(&self) -> &HostConsent {
        &self.consent
    }

    /// The command this approval covers for a check, if it covers one.
    ///
    /// The question an auditor asks, and the reason the record is a list rather than
    /// a flag: *was this exact command approved* is answered by the command line and
    /// the check, not by whether any approval exists.
    #[must_use]
    pub fn command_for(&self, check: &CheckId) -> Option<&ApprovedCommand> {
        self.consent
            .approved_commands
            .iter()
            .find(|approved| approved.check == *check)
    }
}

/// Write an approval down where it can be read after the process that made it.
///
/// The second acceptance sentence's write half. The row carries the project state
/// it was given for, so that a consent read against a later state can be told apart
/// from one read against the state it was asked about.
///
/// # Errors
///
/// [`StoreError`] from the write, including [`StoreError::Busy`] if another process
/// held the lock for longer than the store's timeout.
pub fn record(
    store: &Store,
    consent: &HostConsent,
    project_root: &str,
    fingerprint: &FingerprintId,
) -> Result<i64, StoreError> {
    store.append_approval(consent, project_root, fingerprint)
}

/// Every approval recorded for one project state, newest first.
///
/// The read half. A rejected row stops the read rather than being skipped —
/// [`Store::history`]'s rule, and the one that matters here: an audit trail with a
/// hole in it that looks complete is worse than one that says it could not be read.
///
/// # Errors
///
/// [`StoreError::Decode`] if a row is not a consent this build can read, plus
/// everything [`Store::history`] reports.
pub fn recorded(
    store: &Store,
    fingerprint: &FingerprintId,
    limit: usize,
) -> Result<Vec<RecordedApproval>, StoreError> {
    store
        .history(&HistoryFilter::approvals(fingerprint), limit)?
        .into_iter()
        .map(read_back)
        .collect()
}

/// One stored row, as an approval.
fn read_back(row: StoredRecord) -> Result<RecordedApproval, StoreError> {
    debug_assert!(
        row.kind == RecordKind::Approval,
        "the query asked for one kind and got another"
    );
    Ok(RecordedApproval {
        id: row.id,
        written_at_ms: row.written_at_ms,
        consent: row.decode()?,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::paths::Paths;
    use sure_domain::execution::{ExecutionPermissions, Permission};
    use sure_domain::severity::Severity;
    use sure_domain::status::{CheckStatus, aggregate};

    fn fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    fn check(title: &str) -> PlannedCheck {
        PlannedCheck::new(CheckId::generate(), title, Severity::MustFix, true)
    }

    /// Every permission granted. Each test varies the mode instead, because under
    /// host-confirmed the mode is what decides whether a command is a question.
    fn everything() -> ExecutionPermissions {
        let mut permissions = ExecutionPermissions::inspect_only();
        for permission in Permission::ALL {
            permissions.set(*permission, true);
        }
        permissions
    }

    /// A plan of `(program, arguments)` pairs.
    ///
    /// Generic over the argument type so that a test can hand it an argument SURE
    /// cannot read as text, which is [`OsString`] rather than `&str`. A helper that
    /// only took `&str` could not build the one command this module most needs to
    /// be tried on.
    fn plan_of<S>(mode: ExecutionMode, commands: &[(&str, &[S])]) -> PermissionPlan
    where
        S: Into<std::ffi::OsString> + Clone,
    {
        let mut plan = PermissionPlan::new(mode, fingerprint(), everything());
        for (program, arguments) in commands {
            plan.add(check(program), *program, arguments.to_vec());
        }
        plan
    }

    fn request_of<S>(commands: &[(&str, &[S])]) -> ConsentRequest
    where
        S: Into<std::ffi::OsString> + Clone,
    {
        ConsentRequest::of(plan_of(ExecutionMode::HostConfirmed, commands), ".")
    }

    fn approved_for(request: &ConsentRequest) -> HostConsent {
        request
            .approve(ConsentGrantor::InteractiveUser, "2026-09-15T00:00:00Z")
            .expect("a person at the keyboard can grant")
    }

    // ---- what the user is asked -----------------------------------------

    #[test]
    fn a_request_asks_about_exactly_the_commands_that_need_an_answer() {
        // `git status` only reads, so the `inspect` permission covers it. `npm
        // test` runs project code, and under host-confirmed the mode allows that and
        // `run_project_code` is granted. `git clean` can destroy data, and there is
        // no permission for that, so it is the one question.
        let request = request_of(&[
            ("git", &["status"]),
            ("npm", &["test"]),
            ("git", &["clean"]),
        ]);

        assert_eq!(request.commands().len(), 1);
        assert_eq!(request.commands()[0].display(), "git clean");
        assert_eq!(
            request.commands()[0].why(),
            WhyAsked::NoPermissionCovers(CommandClass::Destructive)
        );
        assert!(request.unrenderable().is_empty());
        assert!(!request.is_empty());
        assert_eq!(request.mode(), ExecutionMode::HostConfirmed);
    }

    #[test]
    fn a_command_the_mode_would_not_run_is_a_question_rather_than_a_refusal() {
        // Inspect-only with every permission granted. The permissions are not the
        // obstacle — `Permission::consent_prompt` is exactly the question this
        // becomes — and the reason it is a question has to be the mode, because a
        // user told "no permission covers this" would go looking for one that does.
        let mut plan = plan_of(ExecutionMode::InspectOnly, &[("npm", &["test"])]);
        // The mode grants nothing by itself; grant the permission explicitly so that
        // the only obstacle left is the mode.
        plan = PermissionPlan::new(
            ExecutionMode::InspectOnly,
            plan.fingerprint().clone(),
            everything(),
        );
        plan.add(check("tests"), "npm", ["test"]);
        let request = ConsentRequest::of(plan, ".");

        assert_eq!(request.commands().len(), 1);
        assert_eq!(
            request.commands()[0].why(),
            WhyAsked::ModeDoesNotRunProjectCode
        );
        let lines = request.commands()[0].explain();
        assert!(
            lines
                .iter()
                .any(|line| line.contains("You can answer for this command")),
            "the mode's half of the reason has no other producer: {lines:?}"
        );
    }

    #[test]
    fn a_command_no_permission_covers_is_denied_outright_when_the_mode_runs_nothing() {
        // `consent::decide_for` asks about the ungrantable category before the mode,
        // so an inspect-only run does not turn `git clean` into a prompt. It is
        // refused, the request does not carry it, and the gate reports the plan's own
        // reason rather than one of this module's.
        let request = ConsentRequest::of(
            plan_of(ExecutionMode::InspectOnly, &[("git", &["clean"])]),
            ".",
        );
        assert!(request.is_empty(), "there is nothing to ask");

        let consent = approved_for(&request);
        let authorisation = authorise(&request, &consent);
        assert_eq!(authorisation.refused().count(), 1);
        assert_eq!(
            authorisation.refusals()[0].not_checked_reason,
            Some(NotCheckedReason::ExecutionNotAuthorized),
            "not `UserDeclined`: the user was never asked, so nothing was declined"
        );
    }

    #[test]
    fn a_command_sure_cannot_show_is_reported_rather_than_dropped_from_the_question() {
        // A command whose argument is not text cannot be displayed, so it cannot be
        // approved — and a request that quietly left it out would be a prompt the
        // user answered without being asked about all of it. It is a question under
        // host-confirmed, because a command SURE could not read may do anything and
        // `Destructive` is one of the things it may do.
        let request = request_of(&[("frobnicate", &[not_text()])]);

        assert!(request.commands().is_empty(), "it was not shown");
        assert_eq!(request.unrenderable().len(), 1);
        assert_eq!(request.unrenderable()[0].0, 0, "its place in the plan");
        assert!(!request.is_empty(), "and it is not nothing to ask about");
        assert!(
            request
                .explain()
                .iter()
                .any(|line| line.contains("could not show you")),
            "{:?}",
            request.explain()
        );

        // The gate agrees with the request rather than re-deriving the answer.
        let consent = approved_for(&request);
        assert!(consent.approved_commands.is_empty());
        let authorisation = authorise(&request, &consent);
        match authorisation.refused().next().map(|(_, refusal)| refusal) {
            Some(Refusal::CannotBeShown { .. }) => {}
            other => panic!("expected a command that could not be shown, got {other:?}"),
        }
    }

    #[test]
    fn a_request_with_nothing_to_ask_says_so_rather_than_printing_nothing() {
        let request = request_of(&[("git", &["status"])]);
        assert!(request.is_empty());
        let lines = request.explain();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Nothing in this plan needs your approval"));
    }

    #[test]
    fn the_plan_explains_a_command_once_and_the_prompt_does_not_say_it_again() {
        // The sentence "no permission SURE can ask for covers this" belongs to
        // `PlannedCommand::explain`, and `WhyAsked::line` deliberately adds nothing
        // for that reason. A second copy would be a second sentence that has to stay
        // in step with the first.
        let request = request_of(&[("git", &["clean"])]);
        let lines = request.commands()[0].explain();
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("No permission SURE can ask for covers this"))
                .count(),
            1,
            "{lines:?}"
        );
        assert_eq!(
            request.commands()[0].why().line(),
            None,
            "the plan already said it"
        );
        assert!(
            WhyAsked::ModeDoesNotRunProjectCode.line().is_some(),
            "and the mode's reason has no other producer"
        );
    }

    // ---- the record -----------------------------------------------------

    #[test]
    fn the_record_is_built_from_what_was_shown() {
        let request = request_of(&[("git", &["clean", "-fdx"]), ("frobnicate", &["--all"])]);
        let consent = approved_for(&request);

        assert_eq!(consent.approved_commands.len(), request.commands().len());
        for (approved, asked) in consent.approved_commands.iter().zip(request.commands()) {
            assert_eq!(approved.program, asked.program());
            assert_eq!(approved.args, asked.arguments());
            assert_eq!(approved.working_directory, request.working_directory());
            assert_eq!(approved.check, *asked.check().id());
            assert_eq!(approved.effects, *asked.effects());
        }
        assert_eq!(consent.granted_by, ConsentGrantor::InteractiveUser);
        assert_eq!(consent.granted_at, "2026-09-15T00:00:00Z");
    }

    #[test]
    fn a_project_file_cannot_build_a_consent_for_itself() {
        // `ADR 0009`: a project may request authority and can never grant it. The
        // record cannot even be constructed here, which is why the gate checks the
        // same thing again — a record can arrive off a disk.
        let request = request_of(&[("git", &["clean"])]);
        assert_eq!(
            request.approve(ConsentGrantor::ProjectRequestEscalated, "now"),
            Err(ApprovalError::GrantorCannotGrant(
                ConsentGrantor::ProjectRequestEscalated
            ))
        );
        let error = ApprovalError::GrantorCannotGrant(ConsentGrantor::ProjectRequestEscalated);
        assert!(error.to_string().contains("asking is not granting"));

        for grantor in [
            ConsentGrantor::InteractiveUser,
            ConsentGrantor::UserConfiguration,
            ConsentGrantor::OrganizationPolicy,
        ] {
            assert!(request.approve(grantor, "now").is_ok(), "{grantor:?}");
        }
        assert!(!ConsentGrantor::ProjectRequestEscalated.can_grant());
    }

    // ---- the gate --------------------------------------------------------

    #[test]
    fn a_command_the_user_approved_may_run_and_one_they_declined_may_not() {
        let mut plan =
            PermissionPlan::new(ExecutionMode::HostConfirmed, fingerprint(), everything());
        let approved_check = check("clean");
        let declined_check = check("untidy");
        plan.add(approved_check.clone(), "git", ["clean"]);
        plan.add(declined_check.clone(), "git", ["frobnicate"]);
        let request = ConsentRequest::of(plan, ".");
        assert_eq!(request.commands().len(), 2);

        // The user said yes to the first and no to the second.
        let consent = request
            .approve_some(
                request
                    .commands()
                    .iter()
                    .filter(|asked| asked.check().id() == approved_check.id()),
                ConsentGrantor::InteractiveUser,
                "2026-09-15T00:00:00Z",
            )
            .expect("a person at the keyboard can grant");

        let authorisation = authorise(&request, &consent);
        let standings: Vec<&Standing> = authorisation
            .commands()
            .map(|(_, standing)| standing)
            .collect();
        assert_eq!(standings[0], &Standing::Approved);
        assert_eq!(
            standings[1],
            &Standing::Refused(Refusal::Declined {
                check: declined_check.id().clone()
            })
        );

        let refusals = authorisation.refusals();
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].status, CheckStatus::Skipped);
        assert_eq!(
            refusals[0].not_checked_reason,
            Some(NotCheckedReason::UserDeclined),
            "a command that was shown and not approved is the event the vocabulary was holding"
        );
        assert!(refusals[0].critical, "the weight is kept");
        assert!(!refusals[0].reason.is_empty(), "a skipped check says why");
        assert!(!aggregate(&refusals).is_green());
    }

    #[test]
    fn a_command_the_permissions_cover_needs_no_approval_at_all() {
        let request = request_of(&[("npm", &["test"])]);
        let consent = approved_for(&request);
        assert!(consent.approved_commands.is_empty(), "nothing was asked");

        let authorisation = authorise(&request, &consent);
        assert_eq!(
            authorisation.commands().next().map(|(_, s)| s.clone()),
            Some(Standing::Permitted)
        );
        assert_eq!(authorisation.refusals().len(), 0);
        assert_eq!(authorisation.admitted().count(), 1);
    }

    #[test]
    fn a_grantor_that_cannot_grant_refuses_every_command_it_names_exactly() {
        // The reason this check is on the record rather than only on the
        // constructor: a `HostConsent` is a value that can be read off a disk. Every
        // command line here matches the approval exactly, and none of them runs.
        let request = request_of(&[("git", &["clean"]), ("frobnicate", &["--all"])]);
        let mut consent = approved_for(&request);
        assert_eq!(
            authorise(&request, &consent).admitted().count(),
            request.commands().len(),
            "the records match before the grantor is changed"
        );

        consent.granted_by = ConsentGrantor::ProjectRequestEscalated;
        let authorisation = authorise(&request, &consent);
        assert_eq!(authorisation.admitted().count(), 0);
        for (_, refusal) in authorisation.refused() {
            assert_eq!(
                refusal,
                &Refusal::GrantorCannotGrant {
                    grantor: ConsentGrantor::ProjectRequestEscalated
                }
            );
        }
        assert!(authorisation.refusals().iter().all(|result| {
            result.status == CheckStatus::Skipped
                && result.not_checked_reason == Some(NotCheckedReason::ExecutionNotAuthorized)
        }));
    }

    #[test]
    fn an_approval_for_a_different_command_line_does_not_cover_this_one() {
        // Same check, same program, one more argument. The whole reason the record
        // keeps a vector instead of a string is that this comparison is exact, and
        // `-fd` is a different command from no `-fd`.
        let request = request_of(&[("git", &["clean", "-fd"])]);
        let mut consent = approved_for(&request);
        consent.approved_commands[0].args = vec!["clean".to_owned()];

        let authorisation = authorise(&request, &consent);
        match authorisation.refused().next().map(|(_, refusal)| refusal) {
            Some(Refusal::NotTheApprovedCommand { approved, planned }) => {
                assert_eq!(approved, "git clean");
                assert_eq!(planned, "git clean -fd");
            }
            other => panic!("expected a mismatch, got {other:?}"),
        }
    }

    #[test]
    fn an_approval_for_one_of_a_checks_commands_does_not_cover_the_other() {
        // One check can plan two commands — `npm ci` and then `npm test` are one
        // check — and the second is not approved by the first one's answer. The gate
        // matches on the command line rather than taking the first approval it finds
        // for the check.
        let mut plan =
            PermissionPlan::new(ExecutionMode::HostConfirmed, fingerprint(), everything());
        let shared = check("checks");
        plan.add(shared.clone(), "git", ["clean"]);
        plan.add(shared.clone(), "git", ["clean", "-fdx"]);
        let request = ConsentRequest::of(plan, ".");
        assert_eq!(request.commands().len(), 2, "two commands under one check");

        let consent = request
            .approve_some(
                request.commands().iter().take(1),
                ConsentGrantor::InteractiveUser,
                "now",
            )
            .expect("a person at the keyboard can grant");

        let authorisation = authorise(&request, &consent);
        assert_eq!(authorisation.admitted().count(), 1);
        assert_eq!(
            authorisation.admitted().next().map(PlannedCommand::display),
            Some("git clean".to_owned())
        );
        match authorisation.refused().next().map(|(_, refusal)| refusal) {
            Some(Refusal::NotTheApprovedCommand { approved, planned }) => {
                assert_eq!(approved, "git clean");
                assert_eq!(planned, "git clean -fdx");
            }
            other => panic!("expected the second command to be refused, got {other:?}"),
        }
    }

    #[test]
    fn an_approval_in_another_directory_does_not_cover_this_one() {
        // The same command in a subdirectory is not the same command, and the
        // consent names where it was approved to run.
        let request = request_of(&[("git", &["clean"])]);
        let mut consent = approved_for(&request);
        consent.approved_commands[0].working_directory = "packages/web".to_owned();

        let authorisation = authorise(&request, &consent);
        match authorisation.refused().next().map(|(_, refusal)| refusal) {
            Some(Refusal::DifferentWorkingDirectory { approved, planned }) => {
                assert_eq!(approved, "packages/web");
                assert_eq!(planned, ".");
            }
            other => panic!("expected a directory mismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_command_that_came_to_fall_into_an_unapproved_category_is_refused() {
        // **The acceptance sentence, as a test.** The command line is exactly the
        // one that was approved and nothing about it changed — what changed is what
        // SURE reads it as, which is what happens when a record written by one build
        // is read by a later one. `git push --force` reaches the network *and* can
        // destroy work that re-running does not bring back; an approval that named
        // only the first of those is not an approval for the second.
        let request = request_of(&[("git", &["push", "--force", "origin", "main"])]);
        let planned = request.commands()[0].effects().clone();
        assert!(
            planned.classes().len() >= 2,
            "this test needs a command that falls into more than one category, and it reads as \
             {planned}"
        );

        let mut consent = approved_for(&request);
        let dropped = *planned.classes().last().expect("at least two");
        let narrowed: Vec<CommandClass> = planned
            .classes()
            .iter()
            .copied()
            .filter(|class| *class != dropped)
            .collect();
        consent.approved_commands[0].effects = CommandEffects::of(&narrowed);

        let authorisation = authorise(&request, &consent);
        match authorisation.refused().next().map(|(_, refusal)| refusal) {
            Some(Refusal::CategoryNotApproved { .. }) => {}
            other => panic!("expected a category refusal, got {other:?}"),
        }
        assert_eq!(authorisation.admitted().count(), 0);
        assert!(
            !authorisation.admitted_categories().contains(&dropped),
            "{dropped:?} was never approved and is not among the admitted categories"
        );
    }

    #[test]
    fn a_wider_approval_than_the_reading_is_still_covered() {
        // `covers` is one way on purpose. A command that has *gained* a category
        // nobody agreed to is refused; one that still falls inside what the user was
        // shown is covered, and refusing it would make an approval expire for a
        // reason the user could not act on.
        let request = request_of(&[("git", &["push", "--force", "origin", "main"])]);
        let planned = request.commands()[0].effects().clone();
        assert!(planned.classes().len() >= 2);

        let mut consent = approved_for(&request);
        consent.approved_commands[0].effects = CommandEffects::anything();
        assert!(
            CommandEffects::anything().covers(&planned),
            "everything covers a subset of everything"
        );
        let authorisation = authorise(&request, &consent);
        assert_eq!(
            authorisation.commands().next().map(|(_, s)| s.clone()),
            Some(Standing::Approved)
        );
    }

    #[test]
    fn only_the_categories_of_admitted_commands_are_reported_as_admitted() {
        // The acceptance sentence as a set rather than as a case: whatever a caller
        // about to run something reads must be a subset of what the user agreed to,
        // and the refused commands' categories must be absent.
        let request = request_of(&[
            ("npm", &["test"]),
            ("git", &["clean"]),
            ("frobnicate", &["--all"]),
        ]);

        // Approve the destruction and decline the command SURE could not read.
        let consent = request
            .approve_some(
                request
                    .commands()
                    .iter()
                    .filter(|asked| asked.display() == "git clean"),
                ConsentGrantor::InteractiveUser,
                "2026-09-15T00:00:00Z",
            )
            .expect("a person at the keyboard can grant");

        let authorisation = authorise(&request, &consent);
        assert_eq!(
            authorisation.admitted_categories(),
            vec![CommandClass::DynamicHost, CommandClass::Destructive],
            "`npm test` by permission and `git clean` by approval, and nothing else"
        );
        assert_eq!(authorisation.admitted().count(), 2);
        assert_eq!(authorisation.refused().count(), 1);

        // Every admitted command's every category is covered by a permission or by
        // an approval — which is the sentence, checked rather than asserted.
        for command in authorisation.admitted() {
            for class in command.effects().classes() {
                let by_permission = class
                    .required_permission()
                    .is_some_and(|permission| request.plan().permissions().allows(permission));
                let by_approval = consent.approved_commands.iter().any(|approved| {
                    approved.check == *command.check().id() && approved.effects.contains(*class)
                });
                assert!(
                    by_permission || by_approval,
                    "{class:?} on {} was admitted by neither",
                    command.display()
                );
            }
        }
    }

    #[test]
    fn a_refused_command_never_produces_a_passing_result() {
        // Two ways to be refused in one plan, and one rule: a command that did not
        // run is `Skipped` with its reason and its weight, which is never a pass. The
        // network permission is withheld, so `cargo add` is denied by the plan itself
        // rather than by any answer.
        let mut permissions = everything();
        permissions.set(Permission::Network, false);
        let mut plan =
            PermissionPlan::new(ExecutionMode::HostConfirmed, fingerprint(), permissions);
        plan.add(check("clean"), "git", ["clean"]);
        plan.add(
            PlannedCheck::new(CheckId::generate(), "install", Severity::CanFixLater, false),
            "cargo",
            ["add", "serde"],
        );
        let request = ConsentRequest::of(plan, ".");
        assert_eq!(
            request.commands().len(),
            1,
            "only `git clean` is a question"
        );

        // The user approved none of it.
        let consent = request
            .approve_some(
                std::iter::empty::<&RequestedCommand>(),
                ConsentGrantor::InteractiveUser,
                "now",
            )
            .expect("a person at the keyboard can grant");
        let authorisation = authorise(&request, &consent);

        let refusals = authorisation.refusals();
        assert_eq!(refusals.len(), 2);
        let reasons: Vec<Option<NotCheckedReason>> = refusals
            .iter()
            .map(|result| result.not_checked_reason)
            .collect();
        assert!(
            reasons.contains(&Some(NotCheckedReason::UserDeclined)),
            "{reasons:?}"
        );
        assert!(
            reasons.contains(&Some(NotCheckedReason::NetworkNotPermitted)),
            "the plan's own reason is carried rather than replaced: {reasons:?}"
        );
        for result in &refusals {
            assert_eq!(result.status, CheckStatus::Skipped);
            assert_ne!(result.status, CheckStatus::Pass);
            assert!(!result.reason.is_empty(), "a skipped check explains itself");
        }
        assert!(!aggregate(&refusals).is_green());
        assert_eq!(authorisation.admitted().count(), 0);
    }

    #[test]
    fn an_authorisation_explains_every_command_it_holds() {
        let request = request_of(&[("npm", &["test"]), ("git", &["clean"])]);
        let consent = approved_for(&request);
        let authorisation = authorise(&request, &consent);

        let lines = authorisation.explain();
        assert!(lines[0].contains("SURE may run 2 of the 2"), "{lines:?}");
        assert!(lines[0].contains("host_confirmed"), "{lines:?}");
        for command in authorisation.request().plan().commands() {
            assert!(
                lines.iter().any(|line| line.contains(&command.display())),
                "{} is missing from {lines:?}",
                command.display()
            );
        }
    }

    // ---- local auditability ---------------------------------------------

    /// A store under `target/tmp`, never the user's own.
    struct Scratch {
        paths: Paths,
        root: std::path::PathBuf,
    }

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(std::path::Path::parent)
                .expect("the workspace root")
                .join("target")
                .join("tmp")
                .join(format!("approval-{name}"));
            if root.exists() {
                std::fs::remove_dir_all(&root).expect("the previous run's directory");
            }
            let data = root.join("data");
            std::fs::create_dir_all(&data).expect("a scratch data directory");
            std::fs::create_dir_all(root.join("config")).expect("a scratch config directory");
            Self {
                paths: Paths::from_roots(data, root.join("config")).expect("absolute roots"),
                root,
            }
        }

        /// `Store::open_at`, never `Store::open`: the user-level store is the
        /// user's, and a test that wrote an approval into it would be recording a
        /// consent nobody gave.
        fn store(&self) -> Store {
            Store::open_at(&self.paths.data_dir().join("sure.db")).expect("a store")
        }

        /// The project root as the record keeps it: forward slashes, so that a row
        /// written here and a row written on a Unix CI runner read the same way.
        fn project_root(&self) -> String {
            self.root.to_string_lossy().replace('\\', "/")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn an_approval_survives_the_process_that_made_it() {
        // The second acceptance sentence, both halves. What comes back is the record
        // as it was given, and the gate decides the same way on the version that came
        // off the disk as on the version in hand — which is the only form of
        // "auditable" worth anything, because the question is always asked later.
        let scratch = Scratch::new("survives");
        let store = scratch.store();
        let request = request_of(&[("git", &["clean", "-fdx"]), ("frobnicate", &["--all"])]);
        let consent = approved_for(&request);

        record(
            &store,
            &consent,
            &scratch.project_root(),
            request.fingerprint(),
        )
        .expect("the write");

        let rows = recorded(&store, request.fingerprint(), 10).expect("the read");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].consent(),
            &consent,
            "the record read back is the record that was written"
        );
        assert!(rows[0].written_at_ms() > 0, "and it says when");
        assert!(rows[0].id() > 0);

        let in_hand = authorise(&request, &consent);
        let off_disk = authorise(&request, rows[0].consent());
        assert_eq!(in_hand.admitted().count(), off_disk.admitted().count());
        assert_eq!(
            in_hand.admitted_categories(),
            off_disk.admitted_categories()
        );
        assert_eq!(in_hand.refusals().len(), off_disk.refusals().len());
        for (before, after) in in_hand.refusals().iter().zip(off_disk.refusals()) {
            assert_eq!(before.not_checked_reason, after.not_checked_reason);
            assert_eq!(before.status, after.status);
        }
    }

    #[test]
    fn the_store_answers_whether_this_exact_command_was_approved_and_when() {
        // The question an auditor asks, and the reason the record is a list of
        // commands rather than a flag.
        let scratch = Scratch::new("audit");
        let store = scratch.store();
        let request = request_of(&[("git", &["clean", "-fdx"])]);
        let consent = approved_for(&request);
        let written = record(
            &store,
            &consent,
            &scratch.project_root(),
            request.fingerprint(),
        )
        .expect("the write");

        let rows = recorded(&store, request.fingerprint(), 10).expect("the read");
        assert_eq!(rows[0].id(), written);
        let covered = request.commands()[0].check().id();
        let other = CheckId::generate();

        let found = rows[0]
            .command_for(covered)
            .expect("the check it was recorded against");
        assert_eq!(found.program, "git");
        assert_eq!(found.args, vec!["clean".to_owned(), "-fdx".to_owned()]);
        assert_eq!(found.effects, *request.commands()[0].effects());
        assert!(rows[0].command_for(&other).is_none());
    }

    #[test]
    fn an_approval_for_one_project_state_is_not_read_against_another() {
        // `docs/architecture/EVIDENCE_MODEL.md`: a record that does not name its
        // project state can be read against a later one, which is how a stale answer
        // becomes a false one. The filter is the fingerprint, and a consent for
        // another state is simply not found.
        let scratch = Scratch::new("states");
        let store = scratch.store();
        let request = request_of(&[("git", &["clean"])]);
        let consent = approved_for(&request);
        record(
            &store,
            &consent,
            &scratch.project_root(),
            request.fingerprint(),
        )
        .expect("the write");

        assert_eq!(
            recorded(&store, request.fingerprint(), 10)
                .expect("the read")
                .len(),
            1
        );
        assert!(
            recorded(&store, &fingerprint(), 10)
                .expect("the read")
                .is_empty(),
            "an approval given for one project state is not an approval for another"
        );
    }

    #[test]
    fn a_recorded_approval_is_in_history_rather_than_hidden_from_it() {
        // Unlike a recording, which the privacy rule excludes by default. A user
        // asking what SURE has been allowed to run is asking through the same door as
        // everything else — and the row can still be deleted, because
        // `docs/security/PRIVACY.md` gives the user the whole local history and not a
        // part of it.
        let scratch = Scratch::new("history");
        let store = scratch.store();
        let request = request_of(&[("git", &["clean"])]);
        let root = scratch.project_root();
        record(
            &store,
            &approved_for(&request),
            &root,
            request.fingerprint(),
        )
        .expect("the write");

        let default = store
            .history(&HistoryFilter::default(), 10)
            .expect("the default filter");
        assert_eq!(default.len(), 1);
        assert_eq!(default[0].kind, RecordKind::Approval);
        assert!(!default[0].kind.is_recording());
        assert!(default[0].kind.is_approval());
        assert_eq!(default[0].project_root.as_deref(), Some(root.as_str()));

        assert_eq!(
            store
                .delete(&HistoryFilter::approvals(request.fingerprint()))
                .expect("the delete"),
            1
        );
        assert!(
            recorded(&store, request.fingerprint(), 10)
                .expect("the read")
                .is_empty()
        );
    }

    #[test]
    fn the_gate_does_not_need_the_store_and_the_store_does_not_need_the_gate() {
        // Two halves of one sentence, and they fail apart: a caller with no store
        // open can still authorise, and a caller that only wants the audit trail
        // never builds a request.
        let request = request_of(&[("git", &["clean"])]);
        let consent = approved_for(&request);
        assert_eq!(authorise(&request, &consent).admitted().count(), 1);

        let scratch = Scratch::new("apart");
        let store = scratch.store();
        record(
            &store,
            &consent,
            &scratch.project_root(),
            request.fingerprint(),
        )
        .expect("the write");
        assert_eq!(
            recorded(&store, request.fingerprint(), 10)
                .expect("the read")
                .len(),
            1
        );
    }

    /// An argument that is a legal OS string and not text.
    ///
    /// Built without `unsafe`, because the workspace forbids it: an unpaired
    /// surrogate is a legal Windows wide string, and a byte that is not valid UTF-8
    /// is a legal Unix one.
    #[cfg(windows)]
    fn not_text() -> std::ffi::OsString {
        use std::os::windows::ffi::OsStringExt;
        std::ffi::OsString::from_wide(&[0xD800, u16::from(b'x')])
    }

    #[cfg(unix)]
    fn not_text() -> std::ffi::OsString {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(vec![0xff, b'x'])
    }
}

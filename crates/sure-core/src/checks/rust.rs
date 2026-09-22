//! The checks a Rust project's own declarations turn into.
//!
//! `P4-T004`'s acceptance, and it is one sentence:
//!
//! > *fmt/check/clippy/test evidence binds to current fingerprint.*
//!
//! # The four words are four [`CommandRole`]s, and the first one is the hard one
//!
//! Unlike `P4-T003`'s — where `import` named something that is not a role and the
//! sentence had to be read twice to find out what it was about — these four are
//! `cargo` subcommands and [`CommandRole`] has a variant for each:
//! [`Format`](CommandRole::Format), [`Check`](CommandRole::Check),
//! [`Lint`](CommandRole::Lint) (which is `clippy`) and
//! [`Test`](CommandRole::Test). [`CHECKS`] is those four, in that order.
//!
//! **`fmt` is a check and `cargo fmt` is not, and the difference is the whole of
//! the second half of the acceptance sentence.** `cargo fmt` rewrites the source
//! tree, so a check built from it would change the state it was about: the result
//! would describe the project *before* the run, the fingerprint would be taken
//! before it, and the two would agree with each other and with nothing that is on
//! disk afterwards. Evidence from that run cannot be bound to the current state,
//! because by the time the run ends there is no state it is still true of. The
//! read-only form is the same tool's own `--check` flag, and it is what
//! [`check_invocation`] adds; the reason it is added here rather than read out of
//! the discovery is that function's own.
//!
//! That is where this module parts company with [`super::node`] and
//! [`super::python`], which both *drop* their format role. They are right to:
//! `prettier --write` and `ruff format` are what those ecosystems' conventional
//! format commands are, and neither proposer will invent a flag because a check
//! would be convenient. Rust is the case where the flag is the conventional form —
//! `.github/workflows/ci.yml` in this repository runs `cargo fmt --all -- --check`,
//! which is what a Rust CI job runs — and dropping the role would leave the first
//! word of this acceptance with no check under it.
//!
//! **And the title moves with it.** [`titled`] does not give the format check
//! [`CommandRole::plain_name`]'s sentence, because that sentence is *"rewrite the
//! source to a style"* and a check that runs `--check` does not rewrite anything. A
//! title is what a person reads in the consent prompt and in the report, and one
//! that described the writing form while the reason beside it named the reading form
//! would be the product telling a user SURE will change their files when it will
//! not.
//!
//! # Why there is no install question here, and why that is not a gap
//!
//! `P4-T003`'s acceptance was half about *without silent package installation*, and
//! its module carries an [`InstallStep`](super::python::InstallStep) that is
//! deliberately not a check. **Rust has no equivalent and needs none**:
//! [`CommandRole::tool`] answers `cargo` for build, test, check, document, run,
//! bench and clean, and a `Cargo.toml` is `cargo`'s own file — there is no second
//! package manager to choose between, no interpreter whose environment might not be
//! the project's, and no case where SURE could plan to install anything. `cargo
//! test` fetches what it needs when it runs, and that is the runner's business in
//! exactly the way [`super::python`] states about `uv run`. So this module proposes
//! nothing that installs and has no step that does; the property holds here by there
//! being nothing to hold.
//!
//! **The tools that are not `cargo` are a different question, and it is the one this
//! ecosystem actually asks.** `clippy` and `rustfmt` are separate installs, so a
//! project that has not asked for them gets no check — see [`gap_kind`]. The
//! discovery already reads the three ways a project asks (a toolchain component,
//! `[lints.clippy]` levels, a configuration file) into
//! [`ToolEvidence`](crate::discover::rust::ToolEvidence), and nothing here
//! re-derives that: the answer arrives as
//! [`ConventionalCommand::command`](crate::discover::rust::ConventionalCommand::command)
//! being `Some`, and the evidence for it arrives in the same row's `because`.
//!
//! # One component, and it is the workspace root
//!
//! [`super::node`] walks workspace members and gives each one its own component.
//! Rust does not, and the reason is in the command rather than in the reading:
//! `cargo test` with no `-p` runs the whole workspace, so a root manifest that
//! declares `[workspace] members = ["crates/a", "crates/b"]` gives **one** test check
//! covering both, not three. Splitting it per member would need `-p` arguments the
//! discovery does not plan, and inventing them here would be SURE choosing a command
//! the project did not write.
//!
//! # What it does not do
//!
//! **It does not look at the installed toolchain.** Nothing here can tell whether
//! `clippy` is the one `rustup` has for this project's channel, or whether the pinned
//! channel is installed at all. That is a fact about this machine, it is discovered
//! by running the command, and the run is the check — the same limit [`super::node`]
//! states about `node_modules`.
//!
//! **It does not read `[lints]` to decide what a lint failure means.** A project with
//! `#![deny(warnings)]` and one with `clippy::all` at `allow` both get the same
//! check, because *how much the project cares* is not something a weight in
//! [`CHECKS`] can be derived from. The four weights there are SURE's judgement and are
//! argued where they are written.
//!
//! **It is not the plan.** No check here has been ordered, deduplicated or gated by a
//! mode; that is [`crate::schedule`]'s work and `tests/rust_checks.rs` is where the
//! two are put together.
//!
//! # A check is planned as work, and not only as a line
//!
//! Since `P18-T003` every check here is a proposal **and the operation that would
//! carry it out** ([`PlannedWork`](crate::planned_work::PlannedWork)): the program is
//! `cargo`, the arguments are the discovery's own subcommand, and the directory is the
//! project root.
//!
//! **The rendered line stays a rendering.** [`CheckReason::DeclaredCommand`] still
//! carries `cargo test`, because that is what a report prints and what a person is
//! being asked to allow — and nothing reads it back into a program. The line and the
//! vector are one value seen twice: both come from the discovery's
//! [`invocation_for`](crate::discover::rust::invocation_for), which is also what
//! [`command_for`](crate::discover::rust::RustProject::conventional_commands) renders,
//! so a check cannot be shown one command and started with another.
//!
//! **The program's name is not completed for the platform.** `cargo` is a name and not
//! a path, and nothing here appends `.exe` to it or wraps it in `cmd.exe /c`: whether a
//! name is startable is
//! [`ProgramPath`](crate::planned_work::ProgramPath)'s answer, taken where the work
//! runs, and constructing an interpreter is the thing ADR 0014 says SURE must not do.
//!
//! **`--check` is an argument here and not in the discovery**, which is the same split
//! the paragraph above the table describes, seen from the operation's side: the
//! discovery answers *what command formats this project* and this module answers *is
//! that a question or an edit*, so the flag is added to the discovery's own argument
//! vector — [`Invocation::with_argument`](crate::discover::Invocation::with_argument) —
//! and never to a string cut apart to find the program again.

use std::path::Path;

use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::ActionKind;
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;
use sure_domain::status::CheckResult;

use crate::discover::Invocation;
use crate::discover::rust::{CommandRole, MANIFEST, RustProject, ToolchainState, invocation_for};
use crate::planned_work::PlannedWork;
use crate::schedule::{CheckProposal, CheckReason, PlanBuilder};

use super::{MissingCommand, MissingKind, check_id, command_operation};

/// What SURE proposes for one role.
///
/// A table rather than a `match` per question, for the reason [`super::node`]'s and
/// [`super::python`]'s are: four roles have to agree about four things, and four
/// matches that agree by hand are four places for a role to be added to only one of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoleCheck {
    /// What the check would do.
    ///
    /// All four of these execute project code, so
    /// [`decide`](sure_domain::execution::decide) refuses every one of them under
    /// [`InspectOnly`](sure_domain::execution::ExecutionMode::InspectOnly). **None of
    /// them is `InstallDependencies`**, and unlike [`super::python`]'s table that is
    /// not keeping anything out: there is no install in this ecosystem to keep out.
    action: ActionKind,
    /// How bad it is if the check does not pass.
    severity: Severity,
    /// Whether the project cannot be trusted for hand-off when it does not pass.
    ///
    /// **Nothing in a `Cargo.toml` says how much any of this matters**, so the four
    /// weights are SURE's judgement, written where they can be argued with and pinned
    /// as a value by `the_table_carries_the_four_weights_this_module_argues_for` — the
    /// same lesson `P4-T002`'s first mutation run taught about prose with no test
    /// under it.
    critical: bool,
}

/// The roles SURE checks, and what each one's check is.
///
/// **The order is the acceptance's**, fmt then check then clippy then test. Two of
/// the four weights are worth arguing for:
///
/// - **`Check` is [`CanFixLater`](Severity::CanFixLater) and not critical.** It
///   overlaps `Test` and `Lint` almost completely — `cargo check --all-targets`
///   compiles everything without running it, `cargo test` compiles and runs,
///   `cargo clippy --all-targets` compiles and lints — so a failure here is nearly
///   always a failure of one of the other two, which are weighted on their own.
///   Weighing it *must fix* would make one broken line produce three findings at the
///   top of the report, and the report would rank a duplicate above the thing it
///   duplicates.
/// - **`Format` is [`Note`](Severity::Note) and not critical.** Formatting is a taste
///   the project has; `cargo fmt --check` failing says the tree differs from what
///   `cargo fmt` would write, which is a fact about style and not about whether the
///   project works. A project SURE refuses to hand over because its indentation is
///   off is a project SURE has mis-ranked.
///
/// The other two are [`super::node`]'s, and that argument does not depend on the
/// language: a failing test suite and a project that will not compile are both reasons
/// not to hand it over.
///
/// [`Build`](CommandRole::Build), [`Document`](CommandRole::Document),
/// [`Run`](CommandRole::Run), [`Bench`](CommandRole::Bench) and
/// [`Clean`](CommandRole::Clean) are absent. `Clean` **deletes build output**, which is
/// [`super::node`]'s `clean` argument and does not need repeating; `Run` and `Bench` do
/// not finish in the way a check has to — a server and a measurement respectively — and
/// `cargo bench` with no bench target succeeds and does nothing, which is the shape of
/// answer this product exists not to give. `Build` is the one worth a sentence: `cargo
/// check --all-targets` already compiles everything without producing binaries and
/// `cargo test` compiles everything and runs it, so a third check that compiles is a
/// third finding for one broken line. `Document` builds the docs, which is a real
/// question and is not one this acceptance names.
const CHECKS: &[(CommandRole, RoleCheck)] = &[
    (
        CommandRole::Format,
        RoleCheck {
            action: ActionKind::Lint,
            severity: Severity::Note,
            critical: false,
        },
    ),
    (
        CommandRole::Check,
        RoleCheck {
            action: ActionKind::TypeCheck,
            severity: Severity::CanFixLater,
            critical: false,
        },
    ),
    (
        CommandRole::Lint,
        RoleCheck {
            action: ActionKind::Lint,
            severity: Severity::ShouldFixFirst,
            critical: false,
        },
    ),
    (
        CommandRole::Test,
        RoleCheck {
            action: ActionKind::RunTests,
            severity: Severity::MustFix,
            critical: true,
        },
    ),
];

/// The flag that turns the formatting role from an edit into a question.
///
/// **One constant, because it is one decision and it is the acceptance's.** `cargo
/// fmt` rewrites every file it disagrees with; `cargo fmt --check` writes nothing and
/// exits non-zero when it disagrees. A check is a question, so the second is what SURE
/// runs, and the reason it matters here rather than in the discovery is
/// [`check_invocation`]'s.
const FORMAT_CHECK_FLAG: &str = "--check";

/// What SURE would check in a Rust project, and what it could not.
///
/// Built from a discovery result and nothing else — see the module documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustChecks {
    component: Option<String>,
    proposed: Vec<PlannedWork>,
    missing: Vec<MissingCommand>,
}

impl RustChecks {
    /// Everything SURE would check in this project, and everything it could not.
    ///
    /// **A project with nothing readable is not a project that declares nothing.**
    /// [`Self::component`] is `None` exactly when SURE did not read a root
    /// `Cargo.toml` — absent, or there and unparseable — and the result is a
    /// `RustChecks` with no proposals and no gaps, for the reason [`super::node`] gives
    /// about a manifest that could not be read: four "you declare nothing" rows for a
    /// file SURE never opened would be four false statements about the project, and the
    /// unread manifest is already a value in the discovery result.
    ///
    /// **`root` is the project root, and it is a parameter rather than a field of the
    /// discovery result** because every check here is planned as typed work and
    /// [`CommandSpec`](crate::planned_work::CommandSpec) requires an absolute working
    /// directory, while a discovery result carries paths relative to whatever it was
    /// read from. The scan refuses a root that is not absolute (`scan::open_root`), so
    /// the directory handed to a [`CommandSpec`](crate::planned_work::CommandSpec) is
    /// absolute whenever the discovery is a value at all — which is what makes it a
    /// value a runner could be given rather than a path that happens to look right.
    #[must_use]
    pub fn of(project: &RustProject, root: &Path) -> Self {
        let mut checks = Self {
            component: component(project),
            proposed: Vec::new(),
            missing: Vec::new(),
        };

        let Some(component) = checks.component.clone() else {
            return checks;
        };

        for &(role, check) in CHECKS {
            let id = check_id(&component, &format!("rust{}", role.as_str()));
            let title = titled(role);

            // **The plan, asked for once**, and both the line the reason carries and
            // the operation beside it are derived from it: `invocation_for` is what
            // the discovery's `command_for` renders, so the row a report prints and
            // the program a runner would start cannot come from two decisions. The
            // role is the key rather than the discovery's rendered string, for the
            // reason [`super::node`] gives: cutting a command line at its spaces is
            // parsing a display into a program, and a space in a path is enough to
            // make that the wrong program.
            match check_invocation(role, invocation_for(project, role)) {
                // **No command, and why is [`gap_kind`]'s question.** The two roles
                // that can reach here are the two whose tools are separate installs.
                None => checks.missing.push(MissingCommand::new(
                    id,
                    title,
                    component.clone(),
                    check.severity,
                    check.critical,
                    gap_kind(project),
                )),
                Some(invocation) => checks.proposed.push(PlannedWork::new(
                    CheckProposal::new(
                        id,
                        title,
                        check.severity,
                        check.critical,
                        // The same answer the other two proposers give, and for the same
                        // reason: a declared command watched for what it does is
                        // deterministic, and the same project state gives the same answer.
                        EvidenceClass::DeterministicCheck,
                        CheckReason::DeclaredCommand {
                            declared_in: component.clone(),
                            // The rendered line, and only the rendered line: it is what
                            // a report prints, and the program that would run is the
                            // `CommandSpec` beside it. Nothing reads this string back
                            // into a program -- ADR 0014 rejected exactly that, and the
                            // pairing is what makes it unnecessary.
                            command: invocation.rendered(),
                        },
                        &[check.action],
                    ),
                    command_operation(
                        invocation.program(),
                        root,
                        &invocation
                            .arguments()
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>(),
                    ),
                )),
            }
        }

        checks
    }

    /// The file SURE read this project from, or `None` when it read no manifest.
    ///
    /// **`Cargo.toml` and nothing else, which is the whole of the answer here.**
    /// [`super::python`] has three candidates and has to rank them; a Rust project SURE
    /// can say anything about is one whose root `Cargo.toml` was read, and there is no
    /// second manifest a check could be planned from. A workspace's members have their
    /// own `Cargo.toml` files and this does not name one: the commands are the root's,
    /// and a component is where a check's evidence points.
    ///
    /// **The file that declares a tool may be a member's manifest**, and SURE does not
    /// claim otherwise. `clippy` declared only in `crates/parser/Cargo.toml` produces a
    /// lint check whose component is the root — because the *command* is the root's,
    /// `cargo clippy` covering the workspace — and
    /// [`ConventionalCommand::because`](crate::discover::rust::ConventionalCommand::because)
    /// is where a reader who does not find the name in the root goes next. That list is
    /// carried by the discovery per file and is not reproduced here.
    #[must_use]
    pub fn component(&self) -> Option<&str> {
        self.component.as_deref()
    }

    /// The checks SURE would run, in no particular order.
    ///
    /// **No particular order is the honest description**: [`PlanBuilder`] sorts them,
    /// and a caller that read an order out of this list would be depending on the order
    /// of [`CHECKS`], which is the acceptance's rather than the project's.
    ///
    /// Each value is a proposal **with the work that would carry it out** — the
    /// program, its arguments and the directory, as typed fields. A caller that wants a
    /// proposal alone asks a value in this list for one; there is no second list,
    /// because a pair kept in two places can be put together wrong.
    #[must_use]
    pub fn planned(&self) -> &[PlannedWork] {
        &self.proposed
    }

    /// The checks SURE could not propose, one per checked role.
    #[must_use]
    pub fn missing(&self) -> &[MissingCommand] {
        &self.missing
    }

    /// Whether there is nothing to check and nothing missing.
    ///
    /// **This is true exactly when SURE read no root `Cargo.toml`**, and for this
    /// ecosystem that is a fact rather than a coincidence:
    /// [`CommandRole::Check`] and [`CommandRole::Test`] name `cargo`, which
    /// `Cargo.toml`'s own presence declares, so a project whose manifest was read
    /// always gets at least those two checks and `proposed` is never empty while
    /// `missing` is not. The other direction is [`Self::of`]'s early return.
    /// `the_layer_is_empty_exactly_when_sure_read_no_manifest` holds the equivalence
    /// over every project shape below.
    ///
    /// **The conjunction is written out even though the two halves agree here, and it
    /// is a finding rather than tidiness that this paragraph says so.** The first
    /// version of this comment claimed the second half was *"the one a mutation can
    /// drop"* and named a project that asked for neither tool as the case that proved
    /// it — and the mutation run showed the claim was false: dropping
    /// `&& self.missing.is_empty()` changed nothing for any value this type can hold,
    /// because the project the comment named proposes two checks and so answers
    /// `false` either way. [`super::python`] is where the conjunction is
    /// load-bearing — all four of its roles can be unplanned, so a project that
    /// declares nothing there really is a project with gaps and no checks — and this
    /// module keeps the same shape the sibling modules do because the predicate a
    /// caller reads should not depend on which ecosystem produced it.
    ///
    /// So the second half is **not reachable through [`Self::of`] today**, and saying
    /// that here is worth more than a sentence claiming a test covers it. What *is*
    /// held is the equivalence above, which fails the day a role table change makes
    /// the halves disagree — and that failure is the prompt to revisit this comment
    /// rather than a surprise.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.proposed.is_empty() && self.missing.is_empty()
    }

    /// Hands every proposal to a plan builder.
    ///
    /// **The builder's refusals are the record, which is why nothing is returned
    /// here.** [`PlanBuilder::propose`] remembers a refusal even when its `Err` is
    /// dropped, and [`PlanBuilder::refused`] is where a caller finds them — so a
    /// proposal this module got wrong cannot disappear by being ignored. Nothing this
    /// module builds can be refused in the first place: every proposal has a title, a
    /// reason naming a file and a command, and exactly one action, and
    /// `nothing_this_module_builds_is_refused` holds that rather than this sentence.
    ///
    /// **The work is handed over with the proposal, not beside it.** A check enters the
    /// plan as one value carrying both, so a schedule cannot hold a proposal whose
    /// operation stayed behind in this module — see [`PlanBuilder::propose`].
    pub fn add_to(&self, builder: &mut PlanBuilder) {
        for work in &self.proposed {
            // The `Err` is the refusal, and it is not dropped: `propose` has already
            // pushed it onto the builder's own list by the time this returns it, which
            // is the contract that function documents.
            if let Err(refusal) = builder.propose(work.clone()) {
                debug_assert!(
                    builder.refused().contains(&refusal),
                    "the builder returned a refusal it did not record"
                );
            }
        }
    }

    /// One skipped result per missing command.
    ///
    /// **This is the half of the layer that is about absences**, and it is the only
    /// thing this module produces that is a result. A caller that renders the checks
    /// SURE ran and forgets this list has a report that is silent about both tools the
    /// project did not ask for; a caller that renders both cannot show a missing command
    /// as anything but something that was not checked, because
    /// [`MissingCommand::not_checked`] has one constructor to reach for and it is
    /// `CheckResult::not_run`.
    ///
    /// **And a skipped result has no evidence to give** — see [`super::evidence_of`],
    /// which answers `None` for it because the status established nothing. The two
    /// halves fit together: `a_missing_command_produces_a_result_that_produces_no_evidence`
    /// is what says a gap cannot reach a report as a green.
    #[must_use]
    pub fn not_checked(&self, project_fingerprint: &FingerprintId) -> Vec<CheckResult> {
        self.missing
            .iter()
            .map(|missing| missing.not_checked(project_fingerprint))
            .collect()
    }
}

/// The file SURE read this project from, or `None` when it read no manifest.
fn component(project: &RustProject) -> Option<String> {
    project.manifest.manifest().map(|_| MANIFEST.to_owned())
}

/// The plan for a role as a *check*, which is not always as the discovery planned
/// it.
///
/// # Why this exists for one role out of four
///
/// For `check`, `clippy` and `test` this is the discovery's value unchanged, and the
/// function is an identity on three of its four inputs. The fourth is
/// [`Format`](CommandRole::Format), and this is the whole reason it is a function
/// rather than a field read.
///
/// **The discovery answers a different question and its answer is not wrong.**
/// [`conventional_commands`](crate::discover::rust::RustProject::conventional_commands)
/// answers *what command runs this role* — a plan — and for the formatting role that is
/// `cargo fmt`, which is what a person types to format their code. This module answers
/// *is that a question or an edit*, and for that role the answer is that it is an edit.
/// Both are true, and the second is not the discovery's to decide: a plan is allowed to
/// contain a command that writes, and a check is not.
///
/// **One argument appended to the discovery's own plan rather than a command composed
/// here.** SURE does not choose the program (`cargo`) or the verb (`fmt`) — those come
/// from the discovery's [`Invocation`] — and
/// [`with_argument`](Invocation::with_argument) adds `--check` to the end of the vector
/// that plan already is.
/// `a_format_check_is_the_discoverys_command_with_one_flag` holds the two together, so
/// that a change to the discovery's answer moves this one with it instead of leaving a
/// stale copy behind. Composing a whole command here would be the second answer this
/// module refuses to have.
///
/// **This is also where the flag cannot become a parsing problem.** It is pushed onto a
/// vector, so the discovery's program stays the first element and its arguments stay in
/// their order; an earlier shape appended it to the rendered line, which was only ever
/// safe because nothing read the line back — and nothing does, which is why the reason
/// still carries the rendered form and the operation carries this one.
fn check_invocation(role: CommandRole, planned: Option<Invocation>) -> Option<Invocation> {
    let invocation = planned?;
    match role {
        // `cargo fmt` rewrites the source, and a check that rewrites the source
        // invalidates its own evidence: the fingerprint it would be bound to is the
        // state before the run, and after the run that state is gone. So the check is
        // the read-only form, which is the same tool's own flag and is what a Rust CI
        // job runs.
        CommandRole::Format => Some(invocation.with_argument(FORMAT_CHECK_FLAG)),
        _ => Some(invocation),
    }
}

/// Why there is no command for a role.
///
/// **The question this answers is what the project said, and the two answers are two
/// different facts.** A project whose toolchain file SURE read and which names neither
/// `clippy` nor `rustfmt` has said nothing about either tool, and that is
/// [`MissingKind::NotDeclared`] — a scope limit, so a critical check that hits it does
/// not hold the run out of green. That is the right answer twice over here, because no
/// role in [`CHECKS`] is critical anyway.
///
/// **A toolchain file SURE could not read is the other answer, and getting this wrong
/// is the false statement this layer exists to refuse.** `rust-toolchain.toml` is one
/// of the three ways a project asks for `clippy` or `rustfmt` — the other two are lint
/// levels and a configuration file, and both of those are read by *presence* — so a
/// project with an unparseable toolchain file is a project SURE cannot say asked for
/// nothing. Reporting [`NotDeclared`](MissingKind::NotDeclared) there would print *"Your
/// project does not declare a way to do this"* about a file that may well declare
/// exactly that, which is a claim about the project made from a failure to read it.
/// [`MissingKind::NotReadable`] is the variant whose sentence is true instead.
///
/// **The role is not a parameter**, because the answer does not depend on one: every
/// role that can reach here wants a tool that is not `cargo`, and one unread toolchain
/// file is the same absence of information about all of them. The two roles that cannot
/// reach here are `check` and `test`, whose tool is `cargo` and whose commands the
/// discovery plans whenever a root manifest was read — and [`RustChecks::of`] returning
/// before the loop when one was not is what keeps them out.
///
/// [`MissingKind::NotACommand`] is `package.json`'s shape and no Rust file has it. The
/// Rust equivalent of poetry's unreadable table form would be a `Cargo.toml` that did
/// not parse, which the discovery records as the *whole manifest* being unread rather
/// than as one declaration being unreadable, so it takes [`RustChecks::of`]'s early
/// return instead of this function. Both are held by
/// `an_unread_manifest_is_not_a_project_that_declares_nothing` and
/// `an_unread_toolchain_is_not_a_project_that_asked_for_nothing` rather than by a
/// sentence here claiming they cannot happen.
fn gap_kind(project: &RustProject) -> MissingKind {
    match project.toolchain {
        ToolchainState::Unread(_) => MissingKind::NotReadable,
        ToolchainState::Read(_) | ToolchainState::Absent => MissingKind::NotDeclared,
    }
}

/// The title a check would have, which is also what a missing command is titled.
///
/// The role's own sentence, for [`super::python`]'s reason: one component means the
/// directory would be noise. Rust needs no suffix for a second reason as well — the
/// commands are workspace-wide, so there is genuinely one of each.
///
/// **`Format` is the exception, and it is [`check_invocation`]'s consequence.**
/// [`CommandRole::plain_name`] answers *"rewrite the source to a style"*, which is what
/// `cargo fmt` does and is the opposite of what this check does: the title is what a
/// person reads before allowing the check to run, and a title that said SURE will
/// rewrite their files while the reason beside it named `cargo fmt --check` would be
/// the product contradicting itself in the one place it is asking for consent.
/// `the_format_check_is_not_titled_as_a_rewrite` holds that.
fn titled(role: CommandRole) -> String {
    match role {
        CommandRole::Format => "check that the source is formatted".to_owned(),
        _ => role.plain_name().to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::discover::UnreadReason;
    use crate::discover::rust::{Manifest, ManifestState, PackageSection, Toolchain, Workspaces};
    use crate::planned_work::CheckOperation;
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;
    use sure_domain::evidence::{
        AnchorSubject, EvidenceAnchor, Freshness, StalenessReason, freshness,
    };
    use sure_domain::execution::{ExecutionMode, ExecutionPermissions};
    use sure_domain::status::{CheckStatus, NotCheckedReason};

    /// The root these fixtures are read from.
    ///
    /// **A directory and not a manifest path**, because that is what a check's
    /// working directory is: the project root, for a project whose manifest is at
    /// the top of it. Written with a space so that a path assembled by pasting
    /// strings together has somewhere to go wrong — the Windows discipline this
    /// repository holds to — and absolute, because
    /// [`CommandSpec`](crate::planned_work::CommandSpec) refuses anything else.
    fn root() -> PathBuf {
        PathBuf::from(r"C:\projects\my crate")
    }

    /// The checks SURE would plan for this project, read from [`root`].
    fn checks_of(project: &RustProject) -> RustChecks {
        RustChecks::of(project, &root())
    }

    /// The proposals alone, for the assertions that are about a proposal.
    fn proposals(checks: &RustChecks) -> Vec<&CheckProposal> {
        checks.planned().iter().map(PlannedWork::proposal).collect()
    }

    /// The work behind one role's check, by the title a person reads.
    fn work_for(checks: &RustChecks, role: CommandRole) -> &PlannedWork {
        checks
            .planned()
            .iter()
            .find(|work| work.proposal().title() == titled(role))
            .unwrap_or_else(|| panic!("{role:?} was not proposed: {:?}", checks.missing()))
    }

    /// A project whose root `Cargo.toml` SURE read, with these declarations.
    ///
    /// **Built by hand rather than through TOML**, for [`super::python`]'s reason: the
    /// TOML reading and the crate table are the layer below, and a private copy of
    /// either here would be testing code nothing runs. `RustProject` is a *discovery
    /// result*, this layer receives one, and a fixture standing in for one has to carry
    /// the fields a discovery would have filled.
    ///
    /// **`tooling` is empty and that is exact rather than convenient.** Every fixture
    /// here declares no dependencies, and the only reader of `tooling` in this module
    /// would be a benchmark role — which [`CHECKS`] does not hold. A fixture that
    /// invented tool rows would be describing a project no manifest here declares. The
    /// real mapping from a manifest's dependency names to tool rows is exercised end to
    /// end, over `Cargo.toml` files on disk, by `tests/rust_checks.rs`.
    ///
    /// The three arguments are the three ways a Rust project can ask for `clippy` or
    /// `rustfmt`, and each call site says which shape it is building rather than leaving
    /// three bare `Option`s for a reader to count.
    fn project(
        linter_config: Option<&'static str>,
        formatter_config: Option<&'static str>,
        toolchain_components: &[&str],
    ) -> RustProject {
        let toolchain = if toolchain_components.is_empty() {
            ToolchainState::Absent
        } else {
            ToolchainState::Read(Box::new(Toolchain {
                file: "rust-toolchain.toml",
                channel: Some("stable".to_owned()),
                components: toolchain_components
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
                targets: Vec::new(),
                profile: None,
            }))
        };

        RustProject {
            manifest: ManifestState::Read(Box::new(Manifest {
                package: Some(PackageSection {
                    name: Some("demo".to_owned()),
                    ..PackageSection::default()
                }),
                ..Manifest::default()
            })),
            workspaces: Workspaces::default(),
            lockfile: true,
            toolchain,
            formatter_config,
            linter_config,
            tooling: Vec::new(),
            conventional_targets: Vec::new(),
        }
    }

    /// A project that asks for both tools, by their configuration files.
    fn complete_project() -> RustProject {
        project(Some("clippy.toml"), Some("rustfmt.toml"), &[])
    }

    /// A project that names both tools in its toolchain file instead.
    ///
    /// The other tier, so that a module which had hard-coded "the config file" would
    /// fail here rather than passing on the one shape somebody thought of.
    fn pinned_project() -> RustProject {
        project(None, None, &["clippy", "rustfmt"])
    }

    /// A project that has said nothing about either tool.
    fn bare_project() -> RustProject {
        project(None, None, &[])
    }

    /// A project whose `Cargo.toml` SURE could not read.
    fn unread_manifest() -> RustProject {
        RustProject {
            manifest: ManifestState::Unread(UnreadReason::WrongShape { found: "a list" }),
            ..bare_project()
        }
    }

    /// A project whose toolchain file SURE could not read.
    fn unread_toolchain() -> RustProject {
        RustProject {
            toolchain: ToolchainState::Unread(UnreadReason::NotParsed {
                detail: "expected `=`".to_owned(),
            }),
            ..bare_project()
        }
    }

    /// The command a role's proposal carries, or a panic naming the gap.
    fn command_of(checks: &RustChecks, role: CommandRole) -> String {
        match work_for(checks, role).proposal().reason() {
            CheckReason::DeclaredCommand { command, .. } => command.clone(),
            other => panic!("the {role:?} check's reason is {other:?}"),
        }
    }

    /// The gap for one role, by the title a missing command carries.
    fn gap_for(checks: &RustChecks, role: CommandRole) -> &MissingKind {
        checks
            .missing()
            .iter()
            .find(|missing| missing.title() == titled(role))
            .unwrap_or_else(|| panic!("{role:?} has a command, so there is no gap to read"))
            .kind()
    }

    /// A result for a check that ran, in the shape a finished check produces.
    fn ran(status: CheckStatus, class: EvidenceClass, fingerprint: &FingerprintId) -> CheckResult {
        CheckResult {
            id: check_id(MANIFEST, "rusttest"),
            title: titled(CommandRole::Test),
            status,
            severity: Severity::MustFix,
            evidence_class: class,
            project_fingerprint: fingerprint.clone(),
            not_checked_reason: None,
            reason: String::new(),
            critical: true,
        }
    }

    /// The anchor a test check's evidence would carry.
    fn anchor() -> EvidenceAnchor {
        CheckReason::DeclaredCommand {
            declared_in: MANIFEST.to_owned(),
            command: "cargo test".to_owned(),
        }
        .anchor()
        .expect("a declared command names a file")
    }

    #[test]
    fn the_table_carries_the_four_weights_this_module_argues_for() {
        // The whole table as a value, because `P4-T002`'s first mutation run found that
        // four flips of a table like this one passed the entire suite while the prose
        // above it was the only thing holding them.
        let shape: Vec<(&str, ActionKind, Severity, bool)> = CHECKS
            .iter()
            .map(|(role, check)| (role.as_str(), check.action, check.severity, check.critical))
            .collect();
        assert_eq!(
            shape,
            vec![
                ("format", ActionKind::Lint, Severity::Note, false),
                ("check", ActionKind::TypeCheck, Severity::CanFixLater, false),
                ("lint", ActionKind::Lint, Severity::ShouldFixFirst, false),
                ("test", ActionKind::RunTests, Severity::MustFix, true),
            ],
            "the four roles, the four actions and the four weights are one decision and \
             not four"
        );

        // Exactly one critical, and it is the test check — the same shape as the other
        // two proposers. A second critical added without a thought about what it means
        // for a hand-off fails here.
        assert_eq!(CHECKS.iter().filter(|(_, check)| check.critical).count(), 1);

        for (role, check) in CHECKS {
            // Nothing in the table installs, which is the property `P4-T003`'s
            // acceptance needed a whole type to hold and which this one gets by there
            // being nothing in this ecosystem to install.
            assert_ne!(
                check.action,
                ActionKind::InstallDependencies,
                "{role:?} would install packages"
            );
            assert!(
                !check.action.can_modify_disk(),
                "{role:?} can change the disk, so it is not a check"
            );
            // And every one of them runs project code, which is what makes the mode
            // gate this layer leans on apply to all four.
            assert!(
                check.action.executes_project_code(),
                "{role:?} would not be gated by the execution mode at all"
            );
        }
    }

    #[test]
    fn the_four_roles_the_acceptance_names_are_the_four_this_table_checks() {
        // Read off the acceptance sentence itself rather than agreed with a comment.
        // `fmt`, `check`, `clippy` and `test` are `cargo`'s own verbs, and
        // `CommandRole::as_str` is where the third one's name is `lint`.
        let named = ["format", "check", "lint", "test"];
        let checked: Vec<&str> = CHECKS.iter().map(|(role, _)| role.as_str()).collect();
        assert_eq!(checked, named, "the table is not in the acceptance's order");

        // And the roles that are absent are absent rather than forgotten, so that a
        // later change adding one is a failing test rather than a surprise.
        for role in CommandRole::ALL {
            let in_the_table = CHECKS.iter().any(|(checked, _)| checked == role);
            assert_eq!(
                in_the_table,
                named.contains(&role.as_str()),
                "{role:?} is on the wrong side"
            );
        }
    }

    #[test]
    fn a_format_check_is_the_discoverys_command_with_one_flag() {
        // **The rewrite, held as a relation rather than as a string.** The point of
        // appending `--check` here rather than writing the whole command out is that the
        // program and the verb stay the discovery's; a test that compared against the
        // literal `cargo fmt --check` would pass just as well if this module had started
        // inventing its own commands. Since `P18-T003` the relation is held over the
        // typed plans — program, arguments — rather than over rendered lines, which is
        // strictly more: the flag is the last *argument* rather than a suffix.
        for planned in [
            Invocation::of("cargo", &["fmt"]),
            Invocation::of("cargo", &["+nightly", "fmt"]),
        ] {
            let composed = check_invocation(CommandRole::Format, Some(planned.clone()))
                .unwrap_or_else(|| panic!("{planned:?} is a plan"));

            // The program is the discovery's and the arguments are the discovery's,
            // unchanged and in order, with one more on the end.
            assert_eq!(composed.program(), planned.program(), "{planned:?}");
            assert_eq!(
                composed.arguments().split_at(planned.arguments().len()).0,
                planned.arguments(),
                "{composed:?} is not {planned:?} with something added"
            );
            assert_eq!(
                composed.arguments().len(),
                planned.arguments().len() + 1,
                "{composed:?} is not {planned:?} with one argument added"
            );
            assert_eq!(
                composed.arguments().last().map(String::as_str),
                Some(FORMAT_CHECK_FLAG),
                "the formatting check writes the source tree: {composed:?}"
            );
            assert_ne!(
                composed, planned,
                "the check is the form that rewrites every file it disagrees with"
            );

            // And the line a report prints is that vector rendered, so the flag is in
            // the sentence a person consents to as well as in the work.
            assert!(
                composed.rendered().ends_with(FORMAT_CHECK_FLAG),
                "{composed:?}"
            );
        }

        // The other three are the discovery's answer unchanged, written out so that
        // format's special case cannot quietly spread to its neighbours.
        let something = Invocation::of("cargo", &["something"]);
        for role in [CommandRole::Check, CommandRole::Lint, CommandRole::Test] {
            assert_eq!(
                check_invocation(role, Some(something.clone())),
                Some(something.clone()),
                "{role:?} must run what the discovery planned"
            );
        }

        // And a role with no command has no command, whatever the flag would be.
        assert_eq!(check_invocation(CommandRole::Format, None), None);
    }

    #[test]
    fn the_format_check_is_not_titled_as_a_rewrite() {
        // **The title and the command have to agree, and they are built in two places.**
        // `CommandRole::plain_name` answers "rewrite the source to a style" — true of
        // `cargo fmt` and false of `cargo fmt --check` — and the title is what a person
        // reads in the consent prompt. A module that took the discovery's sentence for
        // the title and its own flag for the command would ask permission to rewrite the
        // user's files and then not do it.
        assert_ne!(
            titled(CommandRole::Format),
            CommandRole::Format.plain_name(),
            "the format check is titled with the sentence for an edit"
        );
        assert!(
            !titled(CommandRole::Format).contains("rewrite"),
            "{}",
            titled(CommandRole::Format)
        );

        // And the other three are the discovery's own sentences, so this cannot spread:
        // `cargo test` really does run the tests.
        for role in [CommandRole::Check, CommandRole::Lint, CommandRole::Test] {
            assert_eq!(titled(role), role.plain_name());
        }

        // The whole thing over a real proposal, because the two halves are only
        // together once `of` has run.
        let checks = checks_of(&complete_project());
        let proposal = proposals(&checks)
            .into_iter()
            .find(|proposal| *proposal.id() == check_id(MANIFEST, "rustformat"))
            .expect("a project with a rustfmt config gets a format check");
        assert_eq!(proposal.title(), titled(CommandRole::Format));
        match proposal.reason() {
            CheckReason::DeclaredCommand { command, .. } => {
                assert_eq!(command, "cargo fmt --check");
                assert!(
                    !proposal.title().contains("rewrite"),
                    "the title says {} and the command is {command}",
                    proposal.title()
                );
            }
            other => panic!("the format check's reason is {other:?}"),
        }
    }

    #[test]
    fn a_project_that_asks_for_both_tools_gets_four_checks_and_no_gaps() {
        // The acceptance's four words, over a project that declares all four roles. Each
        // command is written out because each one is a decision: `cargo fmt` is the
        // discovery's and gets a flag, `--all-targets` on clippy and check is the
        // discovery's own and must survive, and `cargo test` is unchanged.
        let checks = checks_of(&complete_project());
        assert!(checks.missing().is_empty(), "{:?}", checks.missing());
        assert_eq!(checks.planned().len(), 4);
        assert!(!checks.is_empty());
        assert_eq!(checks.component(), Some(MANIFEST));

        assert_eq!(
            command_of(&checks, CommandRole::Format),
            "cargo fmt --check"
        );
        assert_eq!(
            command_of(&checks, CommandRole::Check),
            "cargo check --all-targets"
        );
        assert_eq!(
            command_of(&checks, CommandRole::Lint),
            "cargo clippy --all-targets"
        );
        assert_eq!(command_of(&checks, CommandRole::Test), "cargo test");

        // Every proposal carries the file SURE read, the deterministic class, one
        // action, and an identity that is the same on the next run — the last because a
        // result is compared against the run before it, and an identity that moved with
        // the *plan* rather than the project would make every comparison a difference.
        let mut ids = Vec::new();
        for proposal in proposals(&checks) {
            assert_eq!(proposal.evidence_class(), EvidenceClass::DeterministicCheck);
            assert_eq!(proposal.requirements().actions().len(), 1);
            match proposal.reason() {
                CheckReason::DeclaredCommand {
                    declared_in,
                    command,
                } => {
                    assert_eq!(declared_in, MANIFEST);
                    assert!(command.starts_with("cargo "), "{command}");
                }
                other => panic!("{} has the reason {other:?}", proposal.title()),
            }
            ids.push(proposal.id().as_str().to_owned());
        }
        let again: Vec<String> = proposals(&checks_of(&complete_project()))
            .iter()
            .map(|proposal| proposal.id().as_str().to_owned())
            .collect();
        assert_eq!(
            ids, again,
            "the identities moved between two reads of one project"
        );
        assert_eq!(ids.len(), 4);
        for (index, id) in ids.iter().enumerate() {
            assert!(
                !ids[index + 1..].contains(id),
                "two roles share an identity, so one check would be dropped as a \
                 duplicate of the other: {ids:?}"
            );
        }
    }

    #[test]
    fn the_toolchain_file_is_evidence_about_a_tool_as_well_as_a_configuration_file_is() {
        // The middle tier, which a module written against one shape would miss: a
        // project that names `clippy` and `rustfmt` as toolchain components and has no
        // configuration file for either is a project that asked for both, and the
        // discovery is where that was decided.
        let checks = checks_of(&pinned_project());
        assert!(checks.missing().is_empty(), "{:?}", checks.missing());
        assert_eq!(checks.planned().len(), 4);
        assert_eq!(
            command_of(&checks, CommandRole::Lint),
            "cargo clippy --all-targets"
        );
        assert_eq!(
            command_of(&checks, CommandRole::Format),
            "cargo fmt --check"
        );
    }

    #[test]
    fn a_project_that_has_said_nothing_about_a_tool_gets_two_checks_and_two_gaps() {
        // **The half of the layer that is about absences, and the reason `is_empty` is a
        // conjunction.** `cargo check` and `cargo test` are `cargo`'s own, so a project
        // with a readable manifest has them; `clippy` and `rustfmt` are separate
        // installs, so a project that never named one gets a sentence rather than a
        // check.
        let checks = checks_of(&bare_project());
        assert_eq!(checks.planned().len(), 2);
        assert_eq!(checks.missing().len(), 2);
        assert_eq!(
            command_of(&checks, CommandRole::Check),
            "cargo check --all-targets"
        );
        assert_eq!(command_of(&checks, CommandRole::Test), "cargo test");

        for role in [CommandRole::Format, CommandRole::Lint] {
            assert_eq!(
                gap_for(&checks, role),
                &MissingKind::NotDeclared,
                "{role:?}"
            );
        }

        // And it is not empty, which is the whole of the accessor's second half: a
        // caller reading "empty" as "nothing was proposed" would print a report silent
        // about both of these gaps.
        assert!(checks.planned().len() < CHECKS.len());
        assert!(!checks.is_empty());

        // Neither of these is a defect either, which is the other side of the split: a
        // project that names no linter is not a project SURE should refuse to call
        // green.
        for missing in checks.missing() {
            assert_eq!(missing.kind(), &MissingKind::NotDeclared);
            assert!(
                missing.kind().not_checked_reason().is_scope_limit(),
                "{} is treated as a gap in the project",
                missing.title()
            );
            assert!(!missing.critical());
        }
    }

    #[test]
    fn an_unread_toolchain_is_not_a_project_that_asked_for_nothing() {
        // **The false statement this distinction exists to refuse.** A
        // `rust-toolchain.toml` is one of the three ways a project asks for `clippy`, so
        // a project whose toolchain file SURE could not parse is a project SURE cannot
        // say asked for nothing. Reporting it as "you declare no linter" would be a
        // claim about a file made from a failure to read it.
        let checks = checks_of(&unread_toolchain());
        for role in [CommandRole::Format, CommandRole::Lint] {
            assert_eq!(
                gap_for(&checks, role),
                &MissingKind::NotReadable,
                "{role:?}"
            );
        }
        // **And it does not spread.** The two roles whose tool is `cargo` are not made
        // *more* missing by a toolchain file SURE could not read: they have their
        // commands, and a version of this that reported every role as unreadable would be
        // the same defect in the other direction — a project SURE can check, described as
        // one it cannot. `cargo test` is written out because it is the whole claim.
        assert_eq!(checks.missing().len(), 2);
        assert_eq!(command_of(&checks, CommandRole::Test), "cargo test");
        assert_eq!(
            command_of(&checks, CommandRole::Check),
            "cargo check --all-targets"
        );

        // Two different facts, two different sentences, and a report that rendered them
        // the same way would have turned one into the other.
        assert_ne!(
            MissingKind::NotReadable.plain_explanation(),
            MissingKind::NotDeclared.plain_explanation()
        );

        // The frozen reasons differ too, so this is not decoration: `NotReadable` is not
        // a scope limit and would hold a critical check out of green, and `NotDeclared`
        // is and would not. Neither role here is critical, so what the difference buys
        // today is the sentence — and the split is what makes the sentence and the
        // classification tell the same story rather than two.
        assert_eq!(
            MissingKind::NotReadable.not_checked_reason(),
            NotCheckedReason::UnknownReason
        );
        assert_eq!(
            MissingKind::NotDeclared.not_checked_reason(),
            NotCheckedReason::NotApplicable
        );
    }

    #[test]
    fn an_unread_manifest_is_not_a_project_that_declares_nothing() {
        // The other half of the same rule, and the worse half: four "you declare nothing"
        // rows here would be four false statements about a file SURE never got a value
        // out of — and two of them would be about `cargo check` and `cargo test`, which
        // would run perfectly well. A project SURE read no manifest of is neither a check
        // nor a gap.
        for project in [
            unread_manifest(),
            RustProject {
                manifest: ManifestState::Absent,
                ..bare_project()
            },
        ] {
            let checks = checks_of(&project);
            assert!(checks.is_empty(), "{checks:?}");
            assert!(checks.component().is_none());
            assert!(checks.planned().is_empty());
            assert!(checks.missing().is_empty());
            assert!(checks.not_checked(&FingerprintId::generate()).is_empty());
        }
    }

    #[test]
    fn the_layer_is_empty_exactly_when_sure_read_no_manifest() {
        // **The test `is_empty`'s first documentation claimed and could not have.** The
        // mutation run for this task dropped the conjunction's second half
        // (`&& self.missing.is_empty()`) and no test in the crate noticed — because for
        // Rust there is no value of this type where the two halves disagree. What is
        // worth holding is therefore not the conjunction, which the type cannot tell
        // apart, but *why* it cannot be told apart: [`CommandRole::Check`] and
        // [`CommandRole::Test`] name `cargo`, a root `Cargo.toml` is what declares
        // `cargo`, and so a project whose manifest SURE read always gets at least those
        // two checks. `proposed` is never empty while `missing` is not.
        //
        // Held as an equivalence rather than as a sentence, because the day a role table
        // change makes the halves disagree is exactly the day this paragraph needs
        // rewriting — and a failing test is a better prompt than a stale one.
        let mut empty = 0;
        let mut has_something_to_say = 0;
        let mut gaps_with_something_to_say = 0;
        for (what, project) in [
            ("asks for both tools", complete_project()),
            ("names both in its toolchain", pinned_project()),
            ("has said nothing", bare_project()),
            ("has an unread toolchain", unread_toolchain()),
            ("has an unread manifest", unread_manifest()),
            (
                "has no manifest at all",
                RustProject {
                    manifest: ManifestState::Absent,
                    ..bare_project()
                },
            ),
        ] {
            let checks = checks_of(&project);
            assert_eq!(checks.is_empty(), checks.component().is_none(), "{what}");
            if checks.is_empty() {
                empty += 1;
            } else {
                has_something_to_say += 1;
            }
            if checks.component().is_some() && !checks.missing().is_empty() {
                gaps_with_something_to_say += 1;
            }

            // The reason, over the shapes that have one: whenever this layer has anything
            // to say at all, the two roles whose tool is `cargo` are among the things it
            // proposes — which is what makes the second half of the conjunction
            // unreachable here rather than merely untested.
            if checks.component().is_some() {
                for role in [CommandRole::Check, CommandRole::Test] {
                    assert!(
                        checks
                            .planned()
                            .iter()
                            .any(|work| work.proposal().title() == titled(role)),
                        "{what}: {role:?} is not proposed, so `proposed` could be empty \
                         while `missing` is not, and the two halves of `is_empty` would \
                         part company — revisit that method's documentation"
                    );
                }
            }
        }
        // The sweep has to have visited both answers, or it holds nothing about either.
        assert_eq!(
            (empty, has_something_to_say),
            (2, 4),
            "the sweep did not visit both answers: two shapes are projects SURE read no \
             manifest of, and four are projects it has something to say about"
        );
        // **And it has to have visited the shape the halves could have disagreed on.** A
        // project that has gaps *and* a component is the only place the conjunction's two
        // halves are both doing work; two fixtures are that shape, and the assertion above
        // is the reason they still answer the same thing.
        assert_eq!(
            gaps_with_something_to_say, 2,
            "the shapes worth counting: a project that said nothing about either tool, and \
             one whose toolchain file SURE could not read"
        );
    }

    #[test]
    fn a_missing_command_produces_a_result_that_produces_no_evidence() {
        // **The two halves of this layer, joined.** A gap becomes a skipped result, and a
        // skipped result has no evidence to give — so a tool the project never asked for
        // cannot reach a report as anything but a sentence saying it was not checked.
        // Held over `MissingKind::ALL` rather than over the two kinds this module
        // produces, because the property has to survive a fourth kind arriving here from
        // another ecosystem's shape.
        let fingerprint = FingerprintId::generate();
        for kind in MissingKind::ALL {
            let missing = MissingCommand::new(
                check_id(MANIFEST, "rusttest"),
                titled(CommandRole::Test),
                MANIFEST,
                Severity::MustFix,
                true,
                kind.clone(),
            );
            let result = missing.not_checked(&fingerprint);
            assert_eq!(result.status, CheckStatus::Skipped, "{kind:?}");
            assert_eq!(
                super::super::evidence_of(&result, anchor()),
                None,
                "{kind:?} produced evidence, so a gap can reach a report as a green"
            );
        }
    }

    #[test]
    fn a_check_that_ran_produces_evidence_bound_to_the_state_it_ran_against() {
        // The acceptance sentence, as a value. A check's result is evidence, and the
        // evidence is fresh for exactly the state the result names and for no other —
        // which is what makes a green from one state unusable against the next.
        let ran_at = FingerprintId::generate();
        let moved_on = FingerprintId::generate();

        for status in CheckStatus::ALL {
            let result = ran(*status, EvidenceClass::DeterministicCheck, &ran_at);
            let evidence = super::super::evidence_of(&result, anchor())
                .unwrap_or_else(|| panic!("{status:?} produced no evidence"));

            assert_eq!(
                freshness(&evidence, &ran_at),
                Freshness::Fresh,
                "{status:?} is not bound to the state it ran against"
            );
            assert_eq!(
                freshness(&evidence, &moved_on),
                Freshness::Stale(StalenessReason::FingerprintChanged),
                "{status:?} stayed usable after the project moved on, which is the green \
                 that outlived its evidence"
            );

            // And it is not evidence with no provenance, which would be stale against
            // *everything* — the other way to be bound wrongly, and the one that reads as
            // caution rather than as a defect.
            assert!(
                evidence.fingerprint.is_some(),
                "{status:?} produced evidence that applies to nothing"
            );
            assert_eq!(evidence.class, EvidenceClass::DeterministicCheck);
            assert_eq!(evidence.severity, result.severity);
            assert!(
                evidence.summary.contains(&result.title),
                "{} does not say what it is about",
                evidence.summary
            );
        }

        // The class is what decides, and it decides for every class rather than for a
        // list of the ones somebody thought of: `Unknown` is the class that means nothing
        // was established, and nothing with it produces evidence.
        for class in EvidenceClass::ALL {
            let result = ran(CheckStatus::Pass, *class, &ran_at);
            assert_eq!(
                super::super::evidence_of(&result, anchor()).is_none(),
                *class == EvidenceClass::Unknown,
                "{class:?} is on the wrong side of the one case that answers None"
            );
        }
    }

    /// A reason and the anchor it should build: subject, location, locator.
    ///
    /// A named alias rather than the tuple spelled out, because the table below is a
    /// list of pairs and a reader should be able to see the pairs without first
    /// unpacking a four-deep type.
    type AnchorCase = (
        CheckReason,
        Option<(AnchorSubject, &'static str, &'static str)>,
    );

    #[test]
    fn every_reason_that_names_something_points_the_anchor_at_it() {
        // `CheckReason::anchor` is the bridge between a proposal and the evidence its
        // result is worth, and an anchor that pointed somewhere plausible rather than
        // somewhere true is the failure `docs/architecture/EVIDENCE_MODEL.md` is about.
        let cases: &[AnchorCase] = &[
            (
                CheckReason::DeclaredCommand {
                    declared_in: MANIFEST.to_owned(),
                    command: "cargo test".to_owned(),
                },
                Some((AnchorSubject::Command, MANIFEST, "cargo test")),
            ),
            (
                CheckReason::FilePresent {
                    path: "Cargo.lock".to_owned(),
                },
                Some((
                    AnchorSubject::File,
                    "Cargo.lock",
                    "the file this check is about",
                )),
            ),
            (
                CheckReason::StackPresent {
                    component: "apps/web".to_owned(),
                    stack: "next".to_owned(),
                },
                Some((AnchorSubject::File, "apps/web", "next")),
            ),
            // The arm that is not a location: a project-wide check has no file to point
            // at, and filling in a plausible one would be the lookup that fails into a
            // wrong anchor.
            (CheckReason::ProjectWide, None),
        ];

        for (reason, expected) in cases {
            match (reason.anchor(), expected) {
                (None, None) => {}
                (Some(anchor), Some((subject, location, locator))) => {
                    assert_eq!(anchor.subject, *subject, "{reason:?}");
                    assert_eq!(anchor.location, *location, "{reason:?}");
                    assert_eq!(anchor.locator, *locator, "{reason:?}");
                    assert!(
                        anchor.is_checkable(),
                        "{reason:?} built an anchor nobody can check"
                    );
                }
                (anchor, expected) => {
                    panic!("{reason:?}: built {anchor:?}, expected {expected:?}")
                }
            }
        }

        // And every one of those reasons really does name something — the predicate the
        // builder refuses on. Without this the test above could be comparing two empty
        // strings and calling it a match.
        for (reason, _) in cases {
            assert!(
                reason.names_something(),
                "{reason:?} is in the table above and names nothing"
            );
        }
    }

    #[test]
    fn the_work_behind_a_check_is_the_typed_form_of_the_line_a_report_prints() {
        // **The pairing, held rather than asserted about.** A report prints
        // `cargo test` and a runner is handed a program and an argument vector; the
        // two are one value seen twice, so rendering the vector has to give back the
        // line a person was shown — over every role, including the one whose
        // arguments this module adds to (`Format`'s `--check`).
        let checks = checks_of(&complete_project());
        let expected: &[(CommandRole, &str, &[&str])] = &[
            (CommandRole::Format, "cargo", &["fmt", "--check"]),
            (CommandRole::Check, "cargo", &["check", "--all-targets"]),
            (CommandRole::Lint, "cargo", &["clippy", "--all-targets"]),
            (CommandRole::Test, "cargo", &["test"]),
        ];
        assert_eq!(
            checks.planned().len(),
            expected.len(),
            "{:?}",
            checks.missing()
        );

        for (role, program, arguments) in expected {
            let work = work_for(&checks, *role);
            let CheckReason::DeclaredCommand { command, .. } = work.proposal().reason() else {
                panic!("{role:?} is not a declared command");
            };
            let CheckOperation::Command(spec) = work.operation() else {
                panic!(
                    "{role:?} is not a command operation: {:?}",
                    work.operation()
                );
            };

            assert_eq!(spec.program(), OsStr::new(*program), "{role:?}");
            // **A name, and not one this module completed.** On Windows a name
            // Windows cannot complete is not a program, and appending `.exe` to make
            // it one would be SURE inventing a program — see the module documentation.
            assert!(
                !spec.program().to_string_lossy().contains('.'),
                "{role:?} names a program with an extension: {:?}",
                spec.program()
            );
            let planned: Vec<OsString> = arguments.iter().map(OsString::from).collect();
            assert_eq!(spec.arguments(), planned.as_slice(), "{role:?}");
            assert_eq!(spec.working_directory(), root(), "{role:?}");

            // The line and the vector cannot describe different commands: the line
            // *is* this vector rendered, character for character.
            let rendered = std::iter::once(spec.program().to_owned())
                .chain(spec.arguments().iter().cloned())
                .map(|word| word.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(&rendered, command, "{role:?}");
        }

        // And the format check's flag is an argument rather than a suffix: the
        // discovery's own plan is the first two, and the vector is longer than it.
        let work = work_for(&checks, CommandRole::Format);
        let CheckOperation::Command(spec) = work.operation() else {
            panic!("the format check is not a command operation");
        };
        assert_eq!(
            spec.arguments().len(),
            2,
            "one argument was added, not one word"
        );
        assert_eq!(spec.arguments()[1], OsString::from(FORMAT_CHECK_FLAG));
    }

    #[test]
    fn nothing_this_module_builds_is_refused() {
        // The builder refuses a proposal with no title, a reason naming nothing, or no
        // action — three things this module claims never to get wrong. Claimed here
        // rather than asserted in prose above `add_to`, and over every fixture shape so
        // that a proposal added later is covered without anybody remembering to add it.
        let mut proposals = 0;
        for (what, project) in [
            ("asks for both tools", complete_project()),
            ("names both in its toolchain", pinned_project()),
            ("has said nothing", bare_project()),
            ("has an unread toolchain", unread_toolchain()),
            ("has an unread manifest", unread_manifest()),
        ] {
            let checks = checks_of(&project);
            proposals += checks.planned().len();

            let mut builder = PlanBuilder::new(
                ExecutionMode::HostConfirmed,
                ExecutionPermissions {
                    run_project_code: true,
                    ..ExecutionPermissions::inspect_only()
                },
            );
            checks.add_to(&mut builder);
            assert!(
                builder.refused().is_empty(),
                "{what}: a proposal was refused: {:?}",
                builder.refused()
            );

            // And what went in is what the plan holds, with no identity used twice.
            let schedule = builder.build();
            assert_eq!(schedule.len(), checks.planned().len(), "{what}");
            assert!(schedule.duplicates().is_empty(), "{what}");
            assert_eq!(schedule.may_run().count(), schedule.len(), "{what}");
        }
        assert_eq!(
            proposals, 12,
            "the sweep did not visit the shapes it claims: 4 + 4 + 2 + 2 + 0"
        );
    }

    #[test]
    fn every_gap_becomes_a_skipped_result_and_never_a_pass() {
        // From the module that has to satisfy the clause, over the project shapes that
        // produce each kind, rather than over the kinds somebody thought of.
        let fingerprint = FingerprintId::generate();
        for (what, project, expected) in [
            ("has said nothing", bare_project(), 2),
            ("has an unread toolchain", unread_toolchain(), 2),
            ("asks for both tools", complete_project(), 0),
            ("has an unread manifest", unread_manifest(), 0),
        ] {
            let checks = checks_of(&project);
            let results = checks.not_checked(&fingerprint);
            assert_eq!(results.len(), expected, "{what}");
            assert_eq!(results.len(), checks.missing().len(), "{what}");
            for result in &results {
                assert_eq!(
                    result.status,
                    CheckStatus::Skipped,
                    "{what}: {}",
                    result.title
                );
                assert!(!result.status.is_green(), "{what}: {}", result.title);
                assert!(
                    !result.status.produced_a_result(),
                    "{what}: {}",
                    result.title
                );
                assert!(result.is_not_checked(), "{what}: {}", result.title);
                assert!(!result.reason.trim().is_empty(), "{what}: {}", result.title);
                assert_eq!(result.project_fingerprint, fingerprint, "{what}");
                // A skipped result established nothing, so there is no evidence it could
                // be worth — the other half of the join above, over the results this
                // module actually produces rather than over a table of kinds.
                assert_eq!(
                    super::super::evidence_of(result, anchor()),
                    None,
                    "{what}: {} reached a report as evidence",
                    result.title
                );
            }
        }
    }
}

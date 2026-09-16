//! The state the packages a project declares are in — read as a fact about the
//! run, never as a fact about the project.
//!
//! `P4-T008`'s acceptance has two sentences and this module is arranged around
//! them: *"Missing dependencies are distinguishable from failing project code."*
//! and *"Install remains separate approved action."* The first is a claim about
//! how a **failure** is read. The second is a claim about what SURE must never
//! do while reading it.
//!
//! # This is not a discovery fact, and the difference is the whole design
//!
//! `docs/architecture/ECOSYSTEM_DISCOVERY.md` lists what discovery does not do,
//! and one entry is this:
//!
//! > **It does not look at `node_modules`, `.venv`, `target`, or what is
//! > installed anywhere else**, and not only because the scan leaves those out.
//! > What is installed is the package manager's answer, and a directory that is
//! > usually absent, usually stale and never committed is not a fact about the
//! > project a person can act on.
//!
//! That is right, and nothing here contradicts it. [`crate::discover::node`]
//! still has no field for `node_modules` and still does not look. What discovery
//! refuses is to let *installed-ness* influence **what the project is** — which
//! ecosystems it uses, which manager resolves it, what support level it is at. A
//! stale `node_modules` must not make a project a different project, and a fresh
//! clone must not be a project SURE cannot classify.
//!
//! What this module asks is a different question with a different subject. Not
//! *what is this project* but *what can SURE do with it right now*. The answer
//! to the second is not a fact about the project: it changes between two runs
//! over one unchanged commit, and it is different for two people on one branch.
//! That is exactly why it does not belong in a discovery result, and exactly why
//! it belongs here, beside the check results it is used to read.
//!
//! # The rule is one-directional, and that is what makes it safe
//!
//! **A sentinel SURE met means nothing. A sentinel SURE did not meet, over a
//! walk that finished, means one thing.** The module never says the packages are
//! installed, not even when it finds the directory an install fills, because
//! *what is installed is the package manager's answer* — the directory can be
//! stale, partial, or for a different lockfile than the one in the tree, and
//! SURE reading a directory name cannot tell any of that apart from the good
//! case.
//!
//! The consequence is worth stating plainly, because it is the reason this
//! reading is safe to make at all: **every way of being wrong in the presence
//! direction is silent.** A false sentinel suppresses a finding; only a true
//! absence can produce one. The direction of the error is chosen, not hoped for.
//!
//! # Why the table has one row, and why that is not an unfinished table
//!
//! [`ECOSYSTEMS`] holds Node and nothing else, and the two omissions are
//! different from each other — the same shape [`crate::db_migrations::DETECTORS`]
//! documents its own omissions in.
//!
//! **Rust is out because there is no gap to find.** `cargo test` fetches what it
//! needs when it runs, so an absent `target` is not a state anything has to be
//! fixed before; it is where the next `cargo` command is going to write.
//! [`crate::checks::rust`] argues this at length and declines to carry an install
//! step for the same reason, and a state check that reported an absent `target`
//! would be telling a person to do a thing `cargo` is about to do anyway.
//!
//! **Python is out because absence settles nothing.** This is the less obvious
//! omission and the more interesting one. Where a Python check runs is decided
//! by [`command_for`](crate::discover::python::command_for) and its `run` helper:
//! a project with an installer gets `uv run pytest` or `poetry run pytest`, and a
//! project without one gets `python -m pytest` — and that last one is, in the
//! discovery's own words, *"a command that depends on which interpreter is first
//! on the path"* (`crates/sure-core/src/discover/python.rs:1596`). An interpreter
//! first on the path is not in the project, so a walk of the project cannot see
//! it. A project with no `.venv` may have every one of its packages installed and
//! importable; SURE saying otherwise would be asserting another tool's
//! convention about where environments live, which is not something a walk of
//! this project establishes. So there is no Python row, and the honest reason is
//! that SURE cannot tell — not that Python projects do not need installing.
//!
//! `the_table_holds_only_the_ecosystems_absence_settles_for` holds the content of
//! this table by name, so that adding a row is a decision somebody makes rather
//! than a diff nobody reads.
//!
//! # The severity, argued from the frozen text rather than chosen
//!
//! The claim this module makes is [`Severity::Note`], and the two levels above
//! it are wrong in ways worth writing down.
//!
//! [`Severity::MustFix`] is *"Do not recommend publishing or handing this off."*
//! A project whose packages are not installed is not unfit to hand off — a fresh
//! clone of a healthy project is in exactly this state, which is why the state
//! exists at all. Blocking a hand-off on it would make SURE refuse to pass
//! judgement on a project it has not yet been able to check.
//!
//! [`Severity::ShouldFixFirst`] is *"A material reliability or quality risk."*
//! That is the wrong subject as well as the wrong weight: the risk here is not
//! to the project's quality, it is to **SURE's ability to measure** that quality.
//! Calling an environment fact a quality risk of the code is the precise
//! confusion this task exists to prevent, and it would be a strange module that
//! set out to prevent it and then committed it in the severity field.
//!
//! What is left is [`Severity::Note`] — *"Informational."* — and that is what
//! this is: not a defect, not a risk, but the sentence a reader needs in order to
//! read every other line of the report correctly. Two properties follow from the
//! choice and neither is bolted on: a `Note` cannot
//! [`blocks_hand_off`](Severity::blocks_hand_off), and
//! [`can_alone_support_must_fix`](sure_domain::evidence::EvidenceClass::can_alone_support_must_fix)
//! is about a class rather than a level, so no missing package can become a
//! `must_fix` about somebody's code.
//!
//! # The install, and the one thing this module may not do
//!
//! **Nothing here installs, and nothing here can be made to.**
//! `docs/architecture/EXECUTION_SAFETY.md` states the rule as *"Never silently
//! run `npm install` … merely to make a check pass"*, and
//! `docs/architecture/FROZEN_SEMANTICS.md` makes `install_dependencies` one of
//! six permissions that never imply each other.
//!
//! This module's answer is [`Assessed::action`], which is
//! [`ActionKind::InstallDependencies`] and is a function rather than a field for
//! the reason [`InstallStep::action`](crate::checks::python::InstallStep::action)
//! gives: there is exactly one answer and a field would be a second place for it
//! to be written down wrong. It is the requirement
//! [`ExecutionRequirements::of`](crate::schedule::ExecutionRequirements::of)
//! turns into a decision and [`crate::consent`] turns into a question, so a
//! caller that does anything with it goes through the same door every other
//! action in the product goes through.
//!
//! **What this module does not do is compose the command.** The line SURE would
//! run is `crate::checks`' business and lives there — `uv sync` in
//! [`checks::python`](crate::checks::python), from the same
//! [`ConventionalCommand`](crate::discover::python::ConventionalCommand) the
//! checks are built from, so that a project whose install row has no command
//! cannot have an install step. Composing a second one here would be a second
//! place for a command to be written down wrong, which is the failure that
//! argument exists to prevent.
//!
//! # What it does not do
//!
//! **It reads no file and runs nothing.** Every fact here is a field of a
//! discovery result — the walk's own lists and the manifest the reading already
//! parsed. `the_module_opens_no_file_and_runs_nothing` holds that as a source
//! rule rather than as a promise, in the shape
//! [`crate::db_migrations`] uses for the same reason.
//!
//! **It says nothing about a workspace member.** Only the project root is read.
//! npm, yarn and pnpm hoist a workspace's packages into the **root**
//! `node_modules`, so a member with no `node_modules` of its own is the normal
//! shape of an installed monorepo rather than a missing install — reading
//! members would produce a finding on every correctly-installed workspace in
//! existence. The root is the only directory where absence is about the install
//! rather than about the layout.
//!
//! **It does not decide what a check does.** No result is built here and no
//! status is set. [`Reading`] is what a caller consults when a check *fails*,
//! and the failure is still the check's own; what this adds is the sentence that
//! stops it being read as a verdict on code that may never have run.

use std::path::Path;

use sure_domain::evidence::{
    AnchorSubject, ClaimAssessment, Evidence, EvidenceAnchor, EvidenceClass,
};
use sure_domain::execution::ActionKind;
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

use crate::discover::{Discovery, Ecosystem, Findings, lookup_key};
use crate::paths::CaseSensitivity;

/// Why SURE cannot say whether the packages are here, when the walk did not
/// finish.
///
/// A constant rather than a `format!` of the losses, because this is printed
/// beside a report that already carries the losses themselves — see
/// [`Discovery::losses`] — and naming them twice would be two places for the
/// same list to be rendered differently.
const A_WALK_THAT_DID_NOT_FINISH: &str = "SURE did not finish looking at this project, so whether the packages it \
     declares are here is something it cannot say.";

/// What the walk met where an install would have written.
///
/// Three values rather than a boolean, because the third is the one that keeps
/// the other two honest: a walk that did not finish cannot support either
/// answer, and a two-valued type would have to pick one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallState {
    /// The walk met something only an install puts there.
    ///
    /// **This is not a statement that the packages are correct.** It is the
    /// module's own rule that a sentinel being present settles nothing; the
    /// value exists so that a reader can see what was met, and so that
    /// [`Self::reading`] has an answer that is not a guess. Nothing downstream
    /// may render it as "your dependencies are installed".
    InPlace {
        /// Which sentinel was met, named as the table names it.
        met: &'static str,
    },
    /// The walk finished and met nothing an install would have put there.
    NotInPlace,
    /// The walk did not finish, so SURE cannot say.
    CannotTell(&'static str),
}

impl InstallState {
    /// Whether the packages this project declares are in the tree SURE walked.
    ///
    /// The reading is over **both** of the walk's lists, and that is not
    /// defensive: `node_modules` is in the scanner's ignore table and reaches
    /// this as a [`Skipped`](crate::scan::Skipped), while `.pnp.cjs` is an
    /// ordinary file that reaches it as an [`Entry`](crate::scan::Entry). A rule
    /// that read only one list would be right about one package manager and
    /// silently wrong about another.
    ///
    /// It is also, and deliberately, independent of *why* the scanner declined
    /// to look: a sentinel met for any reason at all was still met. What the
    /// reason changes is whether the walk finished, and that is asked separately.
    #[must_use]
    pub fn of(row: &Dependencies, discovery: &Discovery) -> Self {
        if let Some(met) = row.met(discovery) {
            return Self::InPlace { met };
        }
        if discovery.is_complete() {
            Self::NotInPlace
        } else {
            Self::CannotTell(A_WALK_THAT_DID_NOT_FINISH)
        }
    }

    /// How a check that did not pass over this project is to be read.
    ///
    /// The answer to the first half of the acceptance. It is asked of the state
    /// rather than of a result because it is a property of the project state and
    /// not of any one check: two checks over one tree are read the same way.
    #[must_use]
    pub const fn reading(self) -> Reading {
        match self {
            Self::NotInPlace => Reading::TheDependencies,
            // `InPlace` reads as the code because nothing SURE observed stands
            // between the failure and the code — not because SURE confirmed the
            // packages are good; see `Self::InPlace`.
            //
            // `CannotTell` reads as the code too, and that is a decision rather
            // than an oversight. SURE has observed nothing at all that bears on
            // the failure, and a caveat with no observation behind it is noise:
            // printed on every check of every project SURE could not finish
            // reading, it would teach a reader to skip the sentence that matters
            // where it does. Where SURE's ignorance belongs is
            // `Discovery::is_complete`, which the report carries in its own
            // right.
            Self::InPlace { .. } | Self::CannotTell(_) => Reading::TheCode,
        }
    }

    /// One sentence for a person, saying what was or was not met.
    #[must_use]
    pub fn plain_description(self) -> String {
        match self {
            Self::InPlace { met } => {
                format!(
                    "SURE found `{met}`, which is where an install would have put the packages."
                )
            }
            Self::NotInPlace => {
                "SURE found none of the places an install would have put the packages.".to_owned()
            }
            Self::CannotTell(why) => why.to_owned(),
        }
    }
}

/// How a check that did not pass is to be read, given the state above.
///
/// The vocabulary the first half of the acceptance needs, and it is deliberately
/// two values rather than three. It is not a severity and not a status: it does
/// not change what the check reported, it changes whether a person may read that
/// as a statement about their code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// A failure here is a statement about the code, and SURE adds nothing.
    TheCode,
    /// A failure here may be about what is not there rather than about what is,
    /// and SURE must say so rather than let it stand as a verdict.
    ///
    /// **Only reachable from an observed absence.** Not from an unfinished walk,
    /// not from a guess about a stack SURE does not read.
    TheDependencies,
}

impl Reading {
    /// Whether a failure read this way is a verdict on the project's code.
    #[must_use]
    pub const fn names_the_code(self) -> bool {
        matches!(self, Self::TheCode)
    }

    /// The sentence that goes beside the failure, when there is one.
    ///
    /// `None` for [`Self::TheCode`] — the product adds no line to a failure it is
    /// reading straight, and a sentence saying "this is about your code" printed
    /// on every failure would be noise dressed as evidence.
    #[must_use]
    pub const fn plain_explanation(self) -> Option<&'static str> {
        match self {
            Self::TheCode => None,
            Self::TheDependencies => Some(
                "This may be about the packages this project declares rather than about its \
                 code: SURE found no installed dependency tree here. SURE has not installed \
                 anything, and putting them there is a separate action you would have to \
                 approve.",
            ),
        }
    }
}

/// One ecosystem's dependencies, and where an install would put them.
///
/// A row is a claim about another project's conventions, in the sense
/// [`crate::db_migrations::Detector`] uses — and here the direction the claim can
/// be wrong in matters more than usual, so the list is generous on purpose. See
/// [`Self::resolved_into`].
///
/// **Not `PartialEq`.** One field is a function pointer, and comparing those is
/// not a meaningful question — the compiler has a lint for it, because two
/// different functions can share an address. A caller wanting to know whether two
/// rows are the same ecosystem compares [`Self::ecosystem`], which is the name a
/// person would use.
#[derive(Debug, Clone, Copy)]
pub struct Dependencies {
    /// Which of [`crate::discover`]'s ecosystems this row reads.
    pub ecosystem: Ecosystem,
    /// What SURE calls this ecosystem in a sentence.
    pub name: &'static str,
    /// The manifest whose declarations this row counts.
    pub declared_in: &'static str,
    /// Everything an install could leave in the project root.
    ///
    /// **Generous on purpose, and the generosity is the safety argument.** This
    /// module's one productive finding is an absence, so a sentinel that is
    /// missing from this list is a project SURE wrongly tells to install packages
    /// it already has. A sentinel that should not have been here costs a
    /// *suppressed* finding, which is the quiet direction and the one this
    /// product would rather be wrong in. So the list names everything a package
    /// manager could resolve from rather than the one directory most of them use.
    ///
    /// **The first is the one a claim names**, because a sentence has to say
    /// where SURE looked and where it looked first is the directory every Node
    /// package manager but one uses. The order is load-bearing and the test holds
    /// it.
    ///
    /// What is *not* here is a cache, and that is the distinction
    /// [`SkipReason`](crate::scan::SkipReason) already draws: it separates
    /// `Vendored` from `Cache` because code installed from somewhere else is not
    /// the same thing as a download a package manager is holding for later. A
    /// populated cache resolves nothing on its own, so counting one as an install
    /// would suppress the finding for exactly the project that needs it.
    pub resolved_into: &'static [&'static str],
    /// How many packages this ecosystem declares, or `None` when there is
    /// nothing that would need installing.
    ///
    /// A function rather than a flag because the answer lives in an ecosystem's
    /// own reading — Node's is a `package.json` SURE either read or did not — and
    /// a second reading of it here would be the place the two disagreed.
    pub declarations: fn(&Discovery) -> Option<usize>,
}

impl Dependencies {
    /// The first sentinel the walk met anywhere in the project root, if any.
    #[must_use]
    pub fn met(&self, discovery: &Discovery) -> Option<&'static str> {
        let case = CaseSensitivity::platform();
        self.resolved_into
            .iter()
            .copied()
            .find(|name| was_met(discovery, name, case))
    }

    /// The sentinels as a person reads them: `a`, `b` or `c`.
    #[must_use]
    pub fn sentinels(&self) -> String {
        match self.resolved_into {
            [] => String::new(),
            [only] => format!("`{only}`"),
            [rest @ .., last] => {
                let named: Vec<String> = rest.iter().map(|name| format!("`{name}`")).collect();
                format!("{} or `{last}`", named.join(", "))
            }
        }
    }
}

/// Whether the walk recorded `name` in the project root.
///
/// **Equality with the root is the test, not a search under it.** A workspace
/// member's `packages/web/node_modules` folds to a longer path and does not equal
/// `node_modules`, so this answers about the root without a second rule saying
/// so — see the module documentation on why only the root is read.
///
/// The case rule is [`CaseSensitivity::platform`] rather than whatever case the
/// walk was asked for, for the reason `crate::db_migrations` states about the
/// same question: what is being decided is whether two names are the same thing
/// on this filesystem, and that is a fact about the platform rather than a
/// preference of the caller.
fn was_met(discovery: &Discovery, name: &str, case: CaseSensitivity) -> bool {
    let wanted = lookup_key(Path::new(name), case);
    let skipped = discovery
        .scan
        .skipped()
        .iter()
        .any(|skipped| lookup_key(&skipped.path, case) == wanted);
    let found = discovery
        .scan
        .entries()
        .iter()
        .any(|entry| lookup_key(&entry.path, case) == wanted);
    skipped || found
}

/// How many packages a Node project declares.
///
/// `None` covers three different situations and the module wants them to
/// collapse: the project declares no packages, so there is nothing an install
/// would put here; and the manifest is absent or was not readable, so SURE has no
/// declaration to count. In all three there is no gap between what the project
/// asked for and what the tree holds, and a claim would be SURE reporting a
/// number it does not have.
#[must_use]
pub fn node_declarations(discovery: &Discovery) -> Option<usize> {
    let report = discovery.report(Ecosystem::Node)?;
    let Findings::Node(project) = &report.findings else {
        return None;
    };
    let package = project.package()?;
    (!package.dependencies.is_empty()).then_some(package.dependencies.len())
}

/// Every ecosystem SURE reads the dependency state of.
///
/// One row. The module documentation argues both omissions — Rust because there
/// is no gap for a state check to find, Python because an absent environment
/// there settles nothing — and the shortness is the same kind of statement
/// [`crate::db_migrations::DETECTORS`] makes with its own four.
pub const ECOSYSTEMS: &[Dependencies] = &[Dependencies {
    ecosystem: Ecosystem::Node,
    name: "Node",
    declared_in: "package.json",
    // `node_modules` is what npm, yarn's node-modules linker, pnpm's symlink
    // farm and bun all resolve from, and it is the one a claim names.
    //
    // The two `.pnp` files are Yarn's plug'n'play linker, which resolves
    // packages **without a `node_modules` at all**. Leaving them out is the
    // mistake this list exists to avoid: every PnP project in the world would be
    // told to install packages it already has.
    resolved_into: &["node_modules", ".pnp.cjs", ".pnp.js"],
    declarations: node_declarations,
}];

/// One thing SURE concluded about the state a project's packages are in.
///
/// Built by [`DependencyReport::of`] and read through the accessors, in the shape
/// every completeness analyser in this crate uses — see
/// [`crate::db_migrations::AssessedGap`] and
/// [`crate::env_completeness::AssessedKey`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessed {
    ecosystem: &'static str,
    declared: usize,
    declared_in: &'static str,
    state: InstallState,
    assessment: ClaimAssessment,
    severity: Severity,
    reason: String,
    evidence: Vec<Evidence>,
}

impl Assessed {
    /// What SURE calls the ecosystem in a sentence.
    #[must_use]
    pub const fn ecosystem(&self) -> &'static str {
        self.ecosystem
    }

    /// How many packages the manifest declares.
    #[must_use]
    pub const fn declared(&self) -> usize {
        self.declared
    }

    /// The manifest those declarations were read from.
    #[must_use]
    pub const fn declared_in(&self) -> &'static str {
        self.declared_in
    }

    /// What the walk found, which is what the reading rests on.
    #[must_use]
    pub const fn state(&self) -> InstallState {
        self.state
    }

    /// How the claim was established.
    #[must_use]
    pub const fn assessment(&self) -> ClaimAssessment {
        self.assessment
    }

    /// How much it matters, which for an environment fact is
    /// [`Severity::Note`] — the module documentation argues why.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// What SURE concluded, as a sentence.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// What the claim rests on.
    #[must_use]
    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    /// What putting the packages there would need allowed.
    ///
    /// Always [`ActionKind::InstallDependencies`], and a function rather than a
    /// field for the reason
    /// [`InstallStep::action`](crate::checks::python::InstallStep::action) gives:
    /// there is exactly one answer and a field would be a second place for it to
    /// be written down wrong.
    ///
    /// **It is a requirement and not a plan.** This module composes no command —
    /// see the module documentation — so what a caller has here is what an
    /// install would cost, not a line to run. It is the value
    /// [`ExecutionRequirements::of`](crate::schedule::ExecutionRequirements::of)
    /// turns into a decision, and under
    /// [`ExecutionPermissions::inspect_only`](sure_domain::execution::ExecutionPermissions::inspect_only)
    /// that decision is a refusal, which is the second half of the acceptance
    /// holding at the type level rather than by convention.
    #[must_use]
    pub const fn action(&self) -> ActionKind {
        ActionKind::InstallDependencies
    }

    /// One line for a report.
    #[must_use]
    pub fn plain_description(&self) -> String {
        match self.state {
            InstallState::NotInPlace => format!(
                "this {} project declares {} packages in `{}` and none of {} is here - SURE has \
                 not installed anything, and putting them there is a separate action you would \
                 have to approve.",
                self.ecosystem,
                self.declared,
                self.declared_in,
                // The row is found again rather than stored, because the sentence
                // is the only place the list is rendered and a stored copy would
                // be a second one to keep in step.
                sentinels_of(self.ecosystem),
            ),
            InstallState::CannotTell(why) => format!("this {} project: {why}", self.ecosystem),
            InstallState::InPlace { met } => format!(
                "this {} project has `{met}` - which is all SURE can see, and is not a statement \
                 that the packages in it are the ones the project asks for.",
                self.ecosystem,
            ),
        }
    }
}

/// The sentinel list of the row for an ecosystem, read back out of
/// [`ECOSYSTEMS`].
///
/// The claim stores the ecosystem's *name* rather than the row, because a claim
/// that held a `&'static Dependencies` would be a claim that could not be built
/// for a row a caller passed in. Looking the row up again by name keeps the table
/// the one place the list is written.
fn sentinels_of(ecosystem: &str) -> String {
    ECOSYSTEMS
        .iter()
        .find(|row| row.name == ecosystem)
        .map_or_else(String::new, Dependencies::sentinels)
}

/// What SURE read of the state every ecosystem's packages are in.
///
/// Built from a discovery result and nothing else, in the shape the other
/// completeness analysers use. **Silent where there is no gap**: a project whose
/// sentinel was met, and a project that declares nothing to install, produce no
/// claim at all rather than a reassuring one, because *"your packages are
/// installed"* is a sentence this module is not entitled to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyReport {
    claims: Vec<Assessed>,
    complete: bool,
}

impl DependencyReport {
    /// Read the state of every ecosystem [`ECOSYSTEMS`] names.
    #[must_use]
    pub fn of(discovery: &Discovery, project_fingerprint: &FingerprintId) -> Self {
        let claims = ECOSYSTEMS
            .iter()
            .filter_map(|row| assess(row, discovery, project_fingerprint))
            .collect();
        Self {
            claims,
            complete: discovery.is_complete(),
        }
    }

    /// What SURE concluded, one entry per ecosystem with something to say.
    #[must_use]
    pub fn claims(&self) -> &[Assessed] {
        &self.claims
    }

    /// Whether the walk this rests on saw everything.
    ///
    /// False means SURE is not in a position to say a sentinel was absent
    /// anywhere, which is why every claim in such a report is
    /// [`ClaimAssessment::CannotConfirm`].
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }

    /// Whether there is anything to report.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }

    /// How a check that did not pass over this project is to be read.
    ///
    /// The report-level form of the first half of the acceptance, and the one a
    /// caller with a failing result actually wants: any ecosystem whose packages
    /// are observed to be absent makes the reading
    /// [`Reading::TheDependencies`], and a report with nothing to say reads as
    /// [`Reading::TheCode`].
    #[must_use]
    pub fn reading(&self) -> Reading {
        self.claims
            .iter()
            .map(|claim| claim.state().reading())
            .find(|reading| !reading.names_the_code())
            .unwrap_or(Reading::TheCode)
    }

    /// One line per claim, for a report.
    #[must_use]
    pub fn plain_description(&self) -> Vec<String> {
        self.claims
            .iter()
            .map(Assessed::plain_description)
            .collect()
    }
}

/// The claim for one ecosystem, or `None` where there is nothing to claim.
fn assess(
    row: &Dependencies,
    discovery: &Discovery,
    project_fingerprint: &FingerprintId,
) -> Option<Assessed> {
    // Asked first, and the order matters: a project that declares nothing has no
    // gap even if the walk met nothing, and reporting one would be SURE inventing
    // an install for a project that does not need one.
    let declared = (row.declarations)(discovery)?;
    let state = InstallState::of(row, discovery);

    // A sentinel being there is the end of the question, and it is the one arm
    // that produces nothing. See the module documentation on why this module
    // never says the packages are installed.
    if let InstallState::InPlace { .. } = state {
        return None;
    }

    // The same rule `crate::db_migrations` and `crate::env_completeness` hold: a
    // reading that did not finish cannot support a claim, and the honest form of
    // "cannot confirm" carries no evidence, because every piece of evidence here
    // is a statement about what the walk saw.
    let (assessment, evidence) = match state {
        InstallState::NotInPlace => (
            ClaimAssessment::Confirmed,
            evidence_of(row, declared, project_fingerprint),
        ),
        _ => (ClaimAssessment::CannotConfirm, Vec::new()),
    };

    Some(Assessed {
        ecosystem: row.name,
        declared,
        declared_in: row.declared_in,
        state,
        assessment,
        severity: Severity::Note,
        reason: reason_for(row, declared, state),
        evidence,
    })
}

/// The sentence a claim carries.
///
/// Built from the row's constants and the count the reading produced, so no
/// project text reaches it — the same rule every sentence in this product
/// follows, and the reason [`Dependencies::name`] is a `&'static str`.
fn reason_for(row: &Dependencies, declared: usize, state: InstallState) -> String {
    match state {
        InstallState::NotInPlace => format!(
            "`{}` declares {declared} packages and the walk met none of {} in the project root. \
             What is installed is the package manager's answer, so this is not a statement about \
             whether they are installed elsewhere - it is that SURE cannot see them here, and a \
             check that runs the project's code would be measuring this rather than the code.",
            row.declared_in,
            row.sentinels(),
        ),
        InstallState::CannotTell(why) => why.to_owned(),
        InstallState::InPlace { met } => format!("the walk met `{met}`"),
    }
}

/// What an observed absence rests on.
///
/// Two observations and both are needed: the manifest that says there is
/// something to install, and the walk that says it is not there. One without the
/// other is either a project with nothing to install or a directory SURE could
/// not have looked for without a reason to.
///
/// **The absence is anchored at the sentinel that was not found**, which is the
/// shape `crate::db_migrations` uses for its own absence — a location a reader
/// can check by looking, with a locator saying what would have been there. An
/// anchor points at where evidence is, and for a finding about something missing
/// the place to look is where the thing would have been.
fn evidence_of(
    row: &Dependencies,
    declared: usize,
    project_fingerprint: &FingerprintId,
) -> Vec<Evidence> {
    // `resolved_into` is never empty for a row in `ECOSYSTEMS`, and the fallback
    // is the manifest rather than an empty location, because an empty location is
    // an anchor `EvidenceAnchor::is_checkable` refuses and this function would
    // then be building unsupported evidence out of a table edit.
    let primary = row
        .resolved_into
        .first()
        .copied()
        .unwrap_or(row.declared_in);

    vec![
        Evidence::new(
            // An observed fact rather than a deterministic check: SURE read a
            // file and counted, and it ran nothing to do it. The same class
            // `crate::db_migrations` gives the declarations its own claims rest
            // on, and the one that means a reader can go and look.
            EvidenceClass::ObservedFact,
            format!("`{}` declares {declared} packages.", row.declared_in),
            EvidenceAnchor::new(
                AnchorSubject::File,
                row.declared_in,
                format!(
                    "the file that says what this {} project depends on",
                    row.name
                ),
            ),
            Some(project_fingerprint.clone()),
            Severity::Note,
        ),
        Evidence::new(
            EvidenceClass::ObservedFact,
            format!("The walk of this project met none of {}.", row.sentinels()),
            EvidenceAnchor::new(
                AnchorSubject::Directory,
                primary,
                format!(
                    "where the packages this {} project declares would be after an install",
                    row.name
                ),
            ),
            Some(project_fingerprint.clone()),
            Severity::Note,
        ),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    // The unit tests here hold the two rules that do not need a project: the
    // table's own content, and the reading derived from a state. Everything that
    // needs a walk of a real directory is in `tests/dependency_state.rs`, where
    // the fixture idiom lives.

    use super::{
        A_WALK_THAT_DID_NOT_FINISH, Dependencies, ECOSYSTEMS, InstallState, Reading,
        node_declarations, sentinels_of,
    };
    use crate::discover::Ecosystem;

    #[test]
    fn the_table_holds_only_the_ecosystems_absence_settles_for() {
        // Named rather than counted, because the point of this test is that a row
        // is a decision. Python is absent because an interpreter first on the
        // path is not in the project and a walk cannot see it; Rust is absent
        // because `cargo test` fetches what it needs and an absent `target` is
        // where the next command writes. Neither is an ecosystem SURE does not
        // read, and both reasons are in the module documentation.
        let named: Vec<&str> = ECOSYSTEMS.iter().map(|row| row.name).collect();
        assert_eq!(named, ["Node"]);

        // And each row's own fields, so that a row added later has to answer
        // these questions rather than inherit an answer.
        for row in ECOSYSTEMS {
            assert_eq!(row.ecosystem, Ecosystem::Node);
            assert!(
                !row.resolved_into.is_empty(),
                "{}: a row with no sentinel can never be in place",
                row.name
            );
            assert!(
                row.resolved_into.iter().all(|name| !name.is_empty()),
                "{}: an empty sentinel would match an empty path",
                row.name
            );
        }
    }

    #[test]
    fn the_first_sentinel_is_the_directory_a_check_for_node_reads_from() {
        // Load-bearing: `evidence_of` anchors the absence at
        // `resolved_into[0]`, so the order of this list decides where a reader is
        // sent to look. The list must also name Yarn's plug'n'play files, because
        // a PnP project resolves packages with no `node_modules` at all and
        // leaving them out would produce a finding on every one of them.
        let node = ECOSYSTEMS
            .iter()
            .find(|row| row.ecosystem == Ecosystem::Node)
            .expect("the node row");
        assert_eq!(node.resolved_into[0], "node_modules");
        assert!(node.resolved_into.contains(&".pnp.cjs"), "{node:?}");
        assert!(node.resolved_into.contains(&".pnp.js"), "{node:?}");

        // A cache is not a resolution. The scanner's own vocabulary separates
        // `Vendored` from `Cache`, and counting one as an install would suppress
        // the finding for exactly the project that needs it.
        assert!(
            !node
                .resolved_into
                .iter()
                .any(|name| name.contains("cache") || name.contains(".yarn")),
            "a download cache resolves nothing on its own: {:?}",
            node.resolved_into
        );
    }

    #[test]
    fn the_sentinels_are_rendered_as_a_person_reads_them() {
        // The sentence a claim carries is the only place this list is rendered,
        // so the joining is part of what a person reads rather than cosmetics.
        let one = Dependencies {
            ecosystem: Ecosystem::Node,
            name: "Node",
            declared_in: "package.json",
            resolved_into: &["node_modules"],
            declarations: node_declarations,
        };
        assert_eq!(one.sentinels(), "`node_modules`");

        let two = Dependencies {
            resolved_into: &["node_modules", ".pnp.cjs"],
            ..one
        };
        assert_eq!(two.sentinels(), "`node_modules` or `.pnp.cjs`");

        let three = Dependencies {
            resolved_into: &["node_modules", ".pnp.cjs", ".pnp.js"],
            ..one
        };
        assert_eq!(
            three.sentinels(),
            "`node_modules`, `.pnp.cjs` or `.pnp.js`",
            "the last one takes `or` and the ones before it take commas"
        );

        // Empty is unreachable from the table and is written out anyway, because
        // a `join` that panicked on it would turn a table edit into a crash in a
        // report.
        let none = Dependencies {
            resolved_into: &[],
            ..one
        };
        assert_eq!(none.sentinels(), "");
    }

    #[test]
    fn only_an_observed_absence_reads_as_something_other_than_the_code() {
        // The first half of the acceptance, over the whole vocabulary rather than
        // over the arm somebody thought of.
        assert_eq!(InstallState::NotInPlace.reading(), Reading::TheDependencies);
        assert!(!Reading::TheDependencies.names_the_code());
        assert!(
            Reading::TheDependencies.plain_explanation().is_some(),
            "a reading that is not about the code has to say so"
        );

        // **A met sentinel reads as the code, and that is not a claim that the
        // packages are good.** SURE observed nothing that stands between a
        // failure and the code; saying more would be reading a directory name as
        // the package manager's answer.
        assert_eq!(
            InstallState::InPlace {
                met: "node_modules"
            }
            .reading(),
            Reading::TheCode
        );

        // **An unfinished walk reads as the code too, and that is a decision.**
        // SURE has observed nothing at all that bears on the failure, and a
        // caveat with no observation behind it printed on every project SURE
        // could not finish reading would teach a reader to skip the sentence
        // where it does matter.
        assert_eq!(
            InstallState::CannotTell(A_WALK_THAT_DID_NOT_FINISH).reading(),
            Reading::TheCode
        );

        // And the direction that matters: no state whose packages were not
        // *observed* absent may produce the qualifier.
        for state in [
            InstallState::InPlace {
                met: "node_modules",
            },
            InstallState::CannotTell(A_WALK_THAT_DID_NOT_FINISH),
        ] {
            assert!(
                state.reading().names_the_code(),
                "{state:?} would tell a person their packages are missing on no observation"
            );
            assert!(
                state.reading().plain_explanation().is_none(),
                "{state:?} adds a sentence to a failure it is reading straight"
            );
        }

        assert_eq!(
            Reading::TheCode.plain_explanation(),
            None,
            "the product adds no line to a failure it reads straight"
        );
    }

    #[test]
    fn the_sentinels_of_a_claim_are_read_back_out_of_the_table() {
        // `Assessed` stores the ecosystem's name rather than the row, so the
        // sentence is built by looking the row up again. A name that is not in
        // the table renders as nothing rather than panicking, because a claim is
        // not a place to discover a table edit.
        assert_eq!(sentinels_of("Node"), ECOSYSTEMS[0].sentinels());
        assert_eq!(sentinels_of("nothing by that name"), "");
    }
}

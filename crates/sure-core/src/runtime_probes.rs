//! What SURE would start and look at, for the checks that need a project
//! running.
//!
//! `P5-T001`'s acceptance, and it is one sentence:
//!
//! > *Runtime probes specify execution/network requirements and target
//! > component.*
//!
//! Three claims, and this module is arranged so that none of them is a promise
//! about this file's prose. **Execution and network requirements** are the
//! [`ActionKind`] each probe declares; the requirements are derived from it by
//! [`CheckProposal::new`], and the answer to the network question is
//! [`ExecutionRequirements::can_touch_network`] — read out of the domain rather
//! than restated here. **The target component** is a
//! [`Component`](crate::components::Component)'s own path, looked up in the
//! [`ComponentGraph`], so a probe cannot name a place the scan did not find.
//! And a probe *is* a [`CheckProposal`] rather than a second vocabulary beside
//! one, which is what makes it droppable into a
//! [`CheckSchedule`](crate::schedule::CheckSchedule) with nothing anywhere
//! deciding twice what a check is.
//!
//! # The promise this module is the first to keep
//!
//! [`crate::checks::node`] documents why four of [`ScriptRole`]'s eight variants
//! produce checks and four do not:
//!
//! > *`dev` and `start` **do not finish**. They are servers, and a check is
//! > something that ends. Starting one is what [`crate::service`] and
//! > `checks.browser_probe` are for, with their own permissions and their own
//! > timeouts.*
//!
//! That sentence was written when those two roles had no consumer at all, and
//! this module is the consumer. [`ScriptRole::Start`] and [`ScriptRole::Dev`]
//! are read here and nowhere else in the shipped source, and the command they
//! yield is the command a probe would run.
//!
//! # Two kinds, and the second one is not a route check
//!
//! [`ProbeKind`] has two rows and they are two different questions:
//!
//! - [`Serve`](ProbeKind::Serve) — *does this project start?* One
//!   [`ActionKind::StartService`], planned for a component whose manifest
//!   declares a command that starts it.
//! - [`Interface`](ProbeKind::Interface) — *does the running thing answer a
//!   browser without errors?* One [`ActionKind::BrowserProbe`], planned for the
//!   same components, because an app SURE has no way to start is an app it has
//!   no way to browse.
//!
//! **A route check is deliberately absent**, and the reason is
//! [`CheckReason`]'s: a probe that asked whether `/api/health` answers would have
//! to name where SURE read that a route exists, and nothing in the discovery
//! holds one — a `package.json` declares scripts and dependencies, not routes.
//! A reason built for it would therefore name a file that says no such thing,
//! which is exactly the claim [`CheckReason::names_something`] and
//! [`CheckReason::anchor`] exist to refuse. Routes are `P5-T003`'s work and they
//! arrive with the reading that can point at them.
//!
//! **[`ActionKind::ExternalService`] is absent for the same kind of reason** and
//! it is a boundary rather than a gap: reaching a payment provider or a mail
//! relay is `P5-T006`'s subject, and its acceptance says what SURE must do there
//! — a need for a real external system becomes *"needs_external_verification /
//! cannot_confirm"* in the acceptance's own words, **not** a local check that
//! passed. Planning one here would be this module inventing a local substitute
//! for a check that is not local. Neither word is a Rust item in this workspace
//! yet; they are quoted from `tasks/tasks.json`, and `P5-T006` is where they
//! become one.
//!
//! **[`ActionKind::ArbitraryCommand`] is absent and it is forbidden rather than
//! absent.** `P5-T005`'s acceptance says a project may describe safe local
//! acceptance flows *without arbitrary free-form shell*, and the way to hold
//! that is for the only actions a probe can declare to be actions with a
//! meaning. There is no field here a caller can put a command line in: the
//! string a probe carries is the project's own declared script, rendered by the
//! package manager that runs it, and [`ProbePlan::of`] is the only constructor.
//!
//! # `auto` and `always` are not decorative, and they answer differently
//!
//! [`CheckPreference`] has three values and the interesting thing about them
//! here is that **they do not mean the same thing for the two kinds**, which is
//! why each row of [`PROBES`] carries its own answer rather than the module
//! deciding once:
//!
//! | preference | `Serve` | `Interface` |
//! |---|---|---|
//! | [`Auto`](CheckPreference::Auto) | planned, when the manifest declares a command that starts it | **not planned** |
//! | [`Always`](CheckPreference::Always) | planned | planned |
//! | [`Never`](CheckPreference::Never) | not planned, and reported as a reduction | not planned, and reported as a reduction |
//!
//! `Auto` is documented as *"run it when the discovered project shape suggests
//! it is worth running"*, and the two kinds can see different amounts of the
//! project shape. A `start` script in a `package.json` **is** the shape that
//! suggests a service; there is no manifest field anywhere that says a project
//! has an interface — not in the discovery, not in this crate — so under `auto`
//! an interface probe would be SURE deciding from nothing that a project has a
//! browser-visible surface. It plans none, and it **says so** in
//! [`ProbePlan::not_planned`] rather than leaving a reader to assume an
//! interface was checked and was fine. `always` is the way to ask for one, and
//! [`CheckPreference`]'s own documentation is where the authority for that lives
//! — *"`Always` is a preference about effort, never a grant of authority"*, so a
//! probe planned under `always` still needs [`Permission::ConnectService`] and
//! is still refused under [`InspectOnly`](ExecutionMode::InspectOnly).
//!
//! **`start` beats `dev` when a manifest declares both**, and the reason is that
//! the alternative is an order that depends on the order two names were read in.
//! [`ScriptRole::Start`] is the role whose name says the project serves; `dev` is
//! the one that says somebody is working on it. Both are tried, in that order,
//! and `start_beats_dev_when_a_manifest_declares_both` is what holds it.
//!
//! # A component with no way to start is a value and not a silence
//!
//! [`NotPlanned`] is [`MissingCommand`](crate::checks::MissingCommand)'s shape
//! one level up, for the reason that type's own documentation gives: **a check
//! that cannot be proposed produces no plan entry, so a report built from the
//! plan would say nothing at all about a project SURE could not start** — and a
//! reader takes the absence of a row for the absence of a problem. Every
//! component that has a readable manifest and was not planned appears in
//! [`ProbePlan::not_planned`] with the reason, and
//! `every_component_is_either_probed_or_explained_under_every_preference` is the
//! cover statement rather than this paragraph.
//!
//! The reason is [`MissingKind`] rather than a second vocabulary invented here.
//! [`crate::checks`] already states the three ways a project leaves SURE without
//! a command — *not declared*, *declared and not a command*, *declared and
//! nothing says what runs it* — and a probe that could not be planned is one of
//! those three facts about one of two roles. `NotACommand` is the one worth
//! spelling out: a `package.json` with `"start": {}` is a manifest that
//! declared something, and a report that called it "no start script" would be
//! describing a broken manifest as a project that never wrote one.
//!
//! # What a probe declares, and the one place capability and cost part company
//!
//! A probe declares **exactly one action**, and it is the whole of what the
//! check would do. The temptation to add a second is real and it is refused in
//! both directions:
//!
//! [`ActionKind::StartService`] answers `can_touch_network() == true`, so a
//! second [`NetworkAccess`](ActionKind::NetworkAccess) action would add no
//! capability — but it would add [`Permission::Network`] to the check's
//! requirements, and that permission is about reaching *beyond this machine*.
//! Requiring it to talk to `127.0.0.1` would ask a user to grant something the
//! check does not need, which is the shape
//! [`ExecutionRequirements::blocked_by`]'s documentation warns about from the
//! other end — *a prompt that named something the user had already granted is a
//! sentence asking them to do something they have done*.
//!
//! **And `StartService`'s network answer is not a mistake to be smoothed over.**
//! It requires [`Permission::RunProjectCode`] and it can reach the network, and
//! those are two different questions: what a check costs a user in trust, and
//! what it can touch once it runs. A project's own start command may download
//! dependencies, bind a port or call out, and SURE has no way to know which — so
//! the capability is declared and no convenience accessor here renames it. A
//! caller asking *can this reach the network* reads
//! [`ExecutionRequirements::can_touch_network`], which is the domain's answer,
//! and the two tests
//! `a_probe_declares_the_network_it_can_touch_and_not_the_permission_it_does_not_need`
//! and `a_probe_specifies_the_requirements_its_action_costs` hold both halves.
//!
//! # What it does not do
//!
//! **It runs nothing, starts nothing and opens nothing.** There is no
//! [`Command`](std::process::Command) here, no port, no timeout and no browser;
//! the actions are declarations and the running is `P5-T002`'s, `P5-T003`'s and
//! `P5-T004`'s work. It reads nothing either: every fact is a field of a
//! discovery result, the same rule [`crate::checks`] states for its proposers.
//!
//! **It does not decide whether a probe is allowed to run.** That is
//! [`PlanBuilder`]'s, and through it [`decide`](sure_domain::execution::decide)
//! — nothing here compares a mode to anything.
//!
//! **It has no caller in this crate yet.** The seam is public and a later task
//! wires it; `NodeChecks` was in the same state one phase earlier.
//!
//! **It does not witness that a service is good.** A project whose start script
//! is `"start": "sleep 600"` gets a serve probe, and a project whose start script
//! exits immediately gets one too. Whether the thing that came up is the thing
//! the project means is not a question a table of roles can answer, and saying
//! so here is the same limit [`crate::browser`] states about its drivers.

use std::fmt;
use std::path::{Path, PathBuf};

use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::ActionKind;
use sure_domain::severity::Severity;
use sure_domain::variants::variants;

use crate::checks::node::{Runner, command_for, components};
use crate::checks::{MissingKind, check_id};
use crate::components::ComponentGraph;
use crate::config::{CheckPreference, ChecksConfig, ScopeReduction};
use crate::discover::node::{NodeProject, Package, ScriptRole};
use crate::scan::display_path;
use crate::schedule::{CheckProposal, CheckReason, PlanBuilder};

/// What SURE would do with a running project.
///
/// Two kinds, and the module documentation argues both the second kind's
/// presence and the absence of the two that a reader is most likely to expect:
/// a route check, and one against a real external service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProbeKind {
    /// Start the component and see that it comes up.
    Serve,
    /// Drive a browser at the started component and watch what it says.
    Interface,
}

variants!(
    /// Every kind, in the order a report reads them.
    ProbeKind { Serve, Interface }
);

impl ProbeKind {
    /// The readable half of this kind's check identifiers.
    ///
    /// Not the public name: [`check_id`] drops everything outside `[a-z0-9]`
    /// from this, so it is written in the shape the identifier scheme wants.
    const fn tag(self) -> &'static str {
        match self {
            Self::Serve => "serve",
            Self::Interface => "interface",
        }
    }

    /// The phrase a check's title is built from.
    ///
    /// **Spelled for someone who is not a programmer**, like every other
    /// `plain_description` in this repository, and deliberately a phrase about
    /// the *check* rather than about the command: the command is in the reason,
    /// one line below.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Serve => "start the project",
            Self::Interface => "check the interface in a browser",
        }
    }

    /// The permission this kind's check costs, from the domain's own table.
    ///
    /// A convenience over [`ActionKind::required_permission`] and **not** a
    /// second answer to the question: it is one hop from the row's action, so
    /// the two cannot disagree. `Permission::ALL` order, the deduplication and
    /// the difference between a permission and a mode all belong to
    /// [`ExecutionRequirements`](crate::schedule::ExecutionRequirements).
    #[must_use]
    pub const fn action(self) -> ActionKind {
        match self {
            Self::Serve => ActionKind::StartService,
            Self::Interface => ActionKind::BrowserProbe,
        }
    }

    /// The reduction a report states when this kind is switched off.
    #[must_use]
    pub const fn reduction(self) -> ScopeReduction {
        match self {
            Self::Serve => ScopeReduction::LocalServicesDisabled,
            Self::Interface => ScopeReduction::BrowserProbeDisabled,
        }
    }

    /// Which of the two `checks.*` settings governs this kind.
    ///
    /// The only place the two settings are told apart, so
    /// [`ProbePlan::reductions`] and the planning loop cannot disagree about
    /// which one is being read.
    #[must_use]
    pub const fn preference(self, checks: &ChecksConfig) -> CheckPreference {
        match self {
            Self::Serve => checks.start_local_services,
            Self::Interface => checks.browser_probe,
        }
    }
}

/// The weights one kind of probe carries, for every kind of probe there is.
///
/// A table rather than a `match` per weight, for the reason
/// [`crate::checks::node`] gives for its own: the kinds have to agree about four
/// things, and four matches that agree by hand are four places for a kind to be
/// added to only one of. [`ProbeKind::ALL`] and this table's own order are
/// asserted against each other rather than assumed to agree.
///
/// **What is *not* in here is deliberate.** The action, the permission it costs
/// and the setting that governs the kind are all functions of the kind alone,
/// and they are three adjacent `match self` arms on [`ProbeKind`] instead —
/// where the compiler refuses a new variant rather than the table quietly
/// missing it. What remains here is the four things that are *judgements* about
/// a kind rather than readings of one, which is why each has an argument in the
/// table's documentation below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeRow {
    /// How bad it is if the probe does not pass.
    severity: Severity,
    /// Whether the project cannot be trusted for hand-off when it does not pass.
    critical: bool,
    /// What the probe's result would be worth.
    evidence_class: EvidenceClass,
    /// Whether `auto` plans this kind for a component that serves.
    ///
    /// **The field that makes `auto` and `always` mean different things**, and
    /// its own documentation is the module's: a manifest can say a project
    /// serves, and no manifest says a project has an interface.
    auto_plans_it: bool,
}

/// The kinds SURE probes, and what each one's check is.
///
/// **The order is the report's**: a service is started before anything looks at
/// it, and [`ScopeReduction::ALL`] happens to run the same way, which
/// `the_reductions_are_in_the_order_a_report_states_them` checks rather than
/// leaves to coincidence.
///
/// # Why the two weights are what they are
///
/// A **serve** probe is [`MustFix`](Severity::MustFix) and critical. `P5-T002`'s
/// acceptance is two sentences, and the second is *"Startup failure remains
/// explicit"* — a project that declares how to start itself and does not start
/// is a project that does not run, which is the same claim a failing build makes
/// and it belongs in the same class. It is also the precondition for everything
/// else this phase does: nothing can be probed in an app that never came up.
///
/// An **interface** probe is [`ShouldFixFirst`](Severity::ShouldFixFirst) and
/// not critical, and that is a decision about what SURE can distinguish rather
/// than about how much a broken page matters. A browser probe's failure modes
/// include ones SURE cannot tell from a real defect — no browser installed, a
/// headless quirk, a first paint slower than the timeout — and `P5-T004`'s
/// acceptance says an unavailable browser must **never** report pass, so the
/// honest status for *could not look* is a check that did not run. **A critical
/// check that did not run holds the whole run out of green** — the domain's own
/// `blocks_green` rule — which would let a machine without Chrome refuse a
/// hand-off. The weight says the finding matters and the missing `critical` says
/// SURE is not the authority on whether this machine could look.
///
/// # Why both evidence classes are `ObservedFact`, which is not what the
/// proposers next door chose
///
/// [`crate::checks::node`] labels each of its four checks
/// [`DeterministicCheck`](EvidenceClass::DeterministicCheck) and gives the
/// reason: *"the same project state gives the same answer, which is the whole of
/// what makes it worth running."*
///
/// **That reason does not hold here, and copying the label because it is the one
/// the neighbours use is exactly the mistake this paragraph exists to stop.** A
/// probe's answer is not a function of the project state: it depends on whether
/// the port was free, whether `node_modules` was installed, whether a browser
/// exists, and what the machine's environment holds. What SURE establishes is
/// that *this machine, at this moment*, started the project and that a browser
/// talked to it — which is [`ObservedFact`](EvidenceClass::ObservedFact)'s own
/// definition, *captured directly from the OS, a harness or the current project
/// state*, and is the stronger of the two ranks rather than the weaker. A serve
/// probe's severity is `must_fix`, and
/// [`can_alone_support_must_fix`](EvidenceClass::can_alone_support_must_fix)
/// answers `true` for both — which
/// `every_probe_declares_the_action_its_kind_names` reads out of the domain
/// rather than trusting this table.
///
/// **And the four weights below have to survive one more hop than that test
/// takes**, which is why
/// `a_probe_carries_the_weight_it_argues_for_and_nothing_louder` is not a second
/// reading of this table. Three of the four rows are
/// arguments about what a user sees — a serve probe's failure holds a run out of
/// green, an interface probe's does not — and the place those arguments come
/// true or false is `blocks_green`, on a result built from a schedule. A table
/// that agreed with itself would pass the first test and fail the second.
const PROBES: &[(ProbeKind, ProbeRow)] = &[
    (
        ProbeKind::Serve,
        ProbeRow {
            severity: Severity::MustFix,
            critical: true,
            evidence_class: EvidenceClass::ObservedFact,
            auto_plans_it: true,
        },
    ),
    (
        ProbeKind::Interface,
        ProbeRow {
            severity: Severity::ShouldFixFirst,
            critical: false,
            evidence_class: EvidenceClass::ObservedFact,
            auto_plans_it: false,
        },
    ),
];

/// One runtime probe: a check, the component it is about, and what it would do.
///
/// **A composition and not a restatement.** The seven fields a check is made of
/// live in [`CheckProposal`] and are not copied here, so there is no second
/// place for a title, a reason or a requirement list to be written down and
/// drift. What this type adds to a proposal is exactly the two things
/// `P5-T001`'s acceptance names that a proposal cannot carry: **which component
/// the probe is about**, as a place the [`ComponentGraph`] found, and **which
/// kind** of probe it is.
///
/// There is no constructor and no setter. The only way to get one is
/// [`ProbePlan::of`], which is also the only place that looks a component up —
/// so a probe whose component is not a component of the project is not a value
/// that can be built and then checked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProbe {
    proposal: CheckProposal,
    component: PathBuf,
    kind: ProbeKind,
}

impl RuntimeProbe {
    /// The check, with its reason, evidence class and execution requirements.
    ///
    /// The network question is answered one hop from here:
    /// `probe.proposal().requirements().can_touch_network()`.
    #[must_use]
    pub const fn proposal(&self) -> &CheckProposal {
        &self.proposal
    }

    /// Which kind of probe this is.
    #[must_use]
    pub const fn kind(&self) -> ProbeKind {
        self.kind
    }

    /// The component directory the probe is about.
    ///
    /// Relative to the project root, and **empty for the root itself**, which is
    /// [`Component::path`](crate::components::Component::path)'s own convention
    /// rather than a second one — this is that field, looked up rather than
    /// rebuilt. [`Self::target`] is the same place written the way a report
    /// spells a path.
    #[must_use]
    pub fn component(&self) -> &Path {
        &self.component
    }

    /// The component as text, with `/` on every platform.
    ///
    /// **A path no reader can see is a path no reader can go and look at**, and
    /// the root component's path is the empty string. `display_path` turns that
    /// into the same nothing, so this is the one that names the project itself.
    #[must_use]
    pub fn target(&self) -> String {
        named(&self.component)
    }

    /// The sentence a report shows for this probe.
    ///
    /// Built from the title, the reason and the evidence class, in the shape
    /// [`ScheduledCheck::plain_description`](crate::schedule::ScheduledCheck::plain_description)
    /// uses, so a probe's row and an ordinary check's row read alike in one
    /// report.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "{} - {} ({})",
            self.proposal.title(),
            self.proposal.reason().plain_description(),
            self.proposal.evidence_class().as_str()
        )
    }
}

/// A component SURE would not probe, and why.
///
/// [`MissingCommand`](crate::checks::MissingCommand)'s shape one level up; see
/// the module documentation for why a gap has to be a value rather than an
/// absent row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotPlanned {
    component: PathBuf,
    kind: ProbeKind,
    because: NotPlannedBecause,
}

impl NotPlanned {
    /// The component that was not planned.
    #[must_use]
    pub fn component(&self) -> &Path {
        &self.component
    }

    /// Which kind of probe was not planned.
    #[must_use]
    pub const fn kind(&self) -> ProbeKind {
        self.kind
    }

    /// Why.
    #[must_use]
    pub const fn because(&self) -> &NotPlannedBecause {
        &self.because
    }

    /// The sentence a report shows for this gap.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "{} in {}: {}",
            self.kind.plain_description(),
            named(&self.component),
            self.because.plain_description()
        )
    }
}

/// Why SURE would not probe a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotPlannedBecause {
    /// The manifest leaves SURE without a command that starts this component.
    ///
    /// **The three ways that happens are [`MissingKind`]'s and not a second
    /// list**, because a probe that cannot be planned is one of those three
    /// facts about one of two roles — and the difference decides what a reader
    /// should do, which is the whole of why those variants are separate.
    NoCommand(MissingKind),
    /// `auto`, and a manifest does not say a project has an interface.
    ///
    /// The component *does* serve; what it does not do is tell SURE that there
    /// is anything to look at. See the module documentation for why this is
    /// reported rather than passed over in silence.
    AutoCannotSeeAnInterface,
}

impl NotPlannedBecause {
    /// The sentence a report shows.
    ///
    /// **The four ways are [`MissingKind::plain_explanation`]'s sentences and
    /// not a second wording written here.** A probe SURE could not plan is the
    /// same four facts a declared check SURE could not propose is faced with,
    /// and a report that described one manifest in two vocabularies would be
    /// making two claims about it. What this adds is the half those sentences
    /// leave implicit — what *this* is that the project declares no way to do —
    /// because a check's title says what it is for and a gap's does not.
    #[must_use]
    pub fn plain_description(&self) -> String {
        match self {
            Self::NoCommand(kind) => format!(
                "SURE has no command to start it. {}",
                kind.plain_explanation()
            ),
            Self::AutoCannotSeeAnInterface => {
                "a manifest does not say whether a project has an interface, and SURE \
                 does not start a browser on a guess. `checks.browser_probe: always` \
                 asks for this check."
                    .to_owned()
            }
        }
    }
}

impl fmt::Display for NotPlannedBecause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.plain_description())
    }
}

/// Why SURE would not produce a probe plan at all.
///
/// One variant, and it is a disagreement between two values rather than a
/// property of a project. **Refused rather than repaired**, on the module's own
/// terms: repairing it would mean either inventing a component the scan did not
/// find — a probe about a place that is not in the report — or dropping the
/// probe silently, which is the absent row this module exists to refuse. A
/// caller that ignores the `Err` gets no plan, which is loud.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanRefused {
    /// A readable manifest is for a directory the component graph does not hold.
    ///
    /// `ProbePlan::of` takes a [`NodeProject`] and a [`ComponentGraph`] as two
    /// independent arguments, and nothing in the type system says they came
    /// from one discovery result. When they did not, this is the value that
    /// says so — and it names both halves, because *which* manifest and *which*
    /// directory is the whole of what a caller needs to see the disagreement.
    NoSuchComponent {
        /// The manifest SURE read, as the scan spells it.
        manifest: String,
        /// The component directory that manifest implies, as the scan spells it.
        component: String,
    },
}

impl fmt::Display for PlanRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchComponent {
                manifest,
                component,
            } => write!(
                f,
                "{manifest} was read, and {component} is not a component of this \
                 project, so a probe about it would name a place the scan did not find"
            ),
        }
    }
}

/// What SURE would do to a project to check it while it is running.
///
/// Built from a discovery result and a configuration and nothing else, by
/// [`Self::of`], which is the only constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbePlan {
    probes: Vec<RuntimeProbe>,
    not_planned: Vec<NotPlanned>,
    reductions: Vec<ScopeReduction>,
}

impl ProbePlan {
    /// What SURE would probe in this project, and what it would not.
    ///
    /// **One component per readable manifest**, exactly as
    /// [`NodeChecks::of`](crate::checks::node::NodeChecks::of) has it and for
    /// the same reason: a manifest SURE could not read is not a manifest that is
    /// absent, and this module inherits that line by asking the same function
    /// for the same list rather than walking the workspace itself.
    ///
    /// **The order is the discovery's** — the root first, then the members in
    /// the order the workspace declaration resolved them — and it is not a
    /// property of the project: [`PlanBuilder`] sorts, and a caller that read a
    /// meaning out of this order would be depending on the order a directory was
    /// walked.
    ///
    /// # Errors
    ///
    /// [`PlanRefused::NoSuchComponent`] when a readable manifest names a
    /// directory the graph does not hold. See that variant for why this is
    /// refused rather than repaired.
    pub fn of(
        graph: &ComponentGraph,
        project: &NodeProject,
        preferences: &ChecksConfig,
    ) -> Result<Self, PlanRefused> {
        let mut plan = Self {
            probes: Vec::new(),
            not_planned: Vec::new(),
            reductions: reductions(preferences),
        };

        // One runner for the whole project, for the reason `NodeChecks::of`
        // states: a package manager is a property of the installation and not of
        // a member, so two components cannot be told two different things.
        let runner = Runner::of(&project.managers);

        for (manifest, package) in components(project) {
            let directory = directory_of(&manifest);
            let Some(component) = graph.get(&directory) else {
                return Err(PlanRefused::NoSuchComponent {
                    manifest,
                    component: named(&directory),
                });
            };
            let component = component.path.clone();

            // The command is asked for once per component rather than once per
            // kind: both kinds stand on the same fact — this component can be
            // started — and asking twice would be two chances to answer
            // differently.
            let serving = serving_command(package, runner);

            for &(kind, row) in PROBES {
                let preference = kind.preference(preferences);
                if preference.disables() {
                    // Already in `reductions`, which is the project-wide record
                    // of a whole class of check being switched off. A per-
                    // component gap beside it would be a second, longer way of
                    // saying one thing the user did on purpose.
                    continue;
                }

                let command = match &serving {
                    Ok(found) => found.clone(),
                    Err(kind_without_a_command) => {
                        plan.not_planned.push(NotPlanned {
                            component: component.clone(),
                            kind,
                            because: NotPlannedBecause::NoCommand(kind_without_a_command.clone()),
                        });
                        continue;
                    }
                };

                if preference == CheckPreference::Auto && !row.auto_plans_it {
                    plan.not_planned.push(NotPlanned {
                        component: component.clone(),
                        kind,
                        because: NotPlannedBecause::AutoCannotSeeAnInterface,
                    });
                    continue;
                }

                plan.probes.push(RuntimeProbe {
                    proposal: CheckProposal::new(
                        // The same namespace as the four declared checks, with
                        // `node` in the tag for the reason the identifier scheme
                        // gives: two ecosystems describing one directory must not
                        // name one check twice. **Whichever role answered is
                        // deliberately not in it**: there is one serve probe per
                        // component whether `start` or `dev` supplied the
                        // command, so a project that renamed its `dev` script to
                        // `start` has not changed which check this is, and a
                        // stored result from the run before still joins to this
                        // one.
                        check_id(&manifest, &format!("node{}", kind.tag())),
                        titled(kind, &component),
                        row.severity,
                        row.critical,
                        row.evidence_class,
                        CheckReason::DeclaredCommand {
                            declared_in: manifest.clone(),
                            command,
                        },
                        &[kind.action()],
                    ),
                    component: component.clone(),
                    kind,
                });
            }
        }

        Ok(plan)
    }

    /// The probes SURE would perform.
    #[must_use]
    pub fn probes(&self) -> &[RuntimeProbe] {
        &self.probes
    }

    /// The components SURE would not probe, one entry per kind per component.
    #[must_use]
    pub fn not_planned(&self) -> &[NotPlanned] {
        &self.not_planned
    }

    /// The settings that made this plan smaller than it could have been.
    ///
    /// **Only the two this plan owns**, and the filtering is the point:
    /// [`Config::scope_reductions`](crate::config::Config::scope_reductions)
    /// reports every setting that narrows a run, including
    /// [`ExistingTestsDisabled`](ScopeReduction::ExistingTestsDisabled), which
    /// is the four declared checks' subject and not this module's. A plan that
    /// re-reported it would state a reduction twice in one report and be wrong
    /// about which layer it was describing.
    ///
    /// The order is [`ScopeReduction::ALL`]'s, which is a report's, and
    /// [`PROBES`]'s own order is asserted to agree with it rather than assumed
    /// to.
    #[must_use]
    pub fn reductions(&self) -> &[ScopeReduction] {
        &self.reductions
    }

    /// Whether there is nothing to probe and nothing recorded.
    ///
    /// True for a project discovery found a `package.json` for and could not
    /// read — the same case
    /// [`NodeChecks::is_empty`](crate::checks::node::NodeChecks::is_empty)
    /// documents — and **not** true for a project that declares nothing to
    /// start, which produces gaps rather than nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.probes.is_empty() && self.not_planned.is_empty()
    }

    /// Hands every probe to a plan builder.
    ///
    /// **The builder's refusals are the record, which is why nothing is
    /// returned here**, exactly as [`NodeChecks::add_to`] has it: a proposal
    /// this module got wrong cannot disappear by being ignored. Nothing this
    /// module builds can be refused in the first place — every probe has a
    /// title, a reason naming a file and a command, and exactly one action, and
    /// `nothing_this_module_builds_is_refused_and_every_identifier_is_distinct`
    /// is what holds that rather than this sentence.
    ///
    /// [`NodeChecks::add_to`]: crate::checks::node::NodeChecks::add_to
    pub fn add_to(&self, builder: &mut PlanBuilder) {
        for probe in &self.probes {
            // The `Err` is the refusal, and it is not dropped: `propose` has
            // already pushed it onto the builder's own list by the time this
            // returns it, which is the contract that function documents.
            if let Err(refusal) = builder.propose(probe.proposal.clone()) {
                debug_assert!(
                    builder.refused().contains(&refusal),
                    "the builder returned a refusal it did not record"
                );
            }
        }
    }

    /// One line per probe, then one per gap, in the plan's own order.
    #[must_use]
    pub fn plain_description(&self) -> Vec<String> {
        self.probes
            .iter()
            .map(RuntimeProbe::plain_description)
            .chain(self.not_planned.iter().map(NotPlanned::plain_description))
            .collect()
    }
}

/// The component directory a manifest path sits in.
///
/// `package.json` alone is the root, and `packages/web/package.json` is
/// `packages/web` — which is [`Component::path`](crate::components::Component::path)'s
/// own spelling, and the reason this is a derived path rather than a second
/// field somewhere.
fn directory_of(manifest: &str) -> PathBuf {
    match Path::new(manifest).parent() {
        Some(directory) => directory.to_path_buf(),
        // Unreachable: every manifest path is built by joining `MANIFEST` onto
        // something, so every one of them has a parent. Folded into the shape it
        // would mean — the root — rather than panicking on a shipped path.
        None => PathBuf::new(),
    }
}

/// A component path as a report spells it, with the root named rather than
/// blank.
///
/// **The root's path is empty and an empty string in a sentence is a hole**, so
/// it is named here once instead of at each of the four places a probe or a gap
/// says where it is about.
fn named(component: &Path) -> String {
    if component.as_os_str().is_empty() {
        "the project root".to_owned()
    } else {
        display_path(component)
    }
}

/// The title a probe would have.
///
/// [`crate::checks::node`]'s `titled` one level up: the component is in the title
/// because a workspace has several, and two rows saying *start the project* with
/// nothing to tell them apart is what a monorepo report is made of. The root is
/// the project itself, so its title is the bare one.
fn titled(kind: ProbeKind, component: &Path) -> String {
    if component.as_os_str().is_empty() {
        return kind.plain_description().to_owned();
    }
    format!(
        "{} in {}",
        kind.plain_description(),
        display_path(component)
    )
}

/// The command that starts this component, or why SURE has none.
///
/// **Two roles are asked and they are asked in a fixed order.** `start` is the
/// role whose name says the project serves and `dev` is the one that says
/// somebody is working on it, so `start` wins when a manifest declares both —
/// and the alternative is that the command a user is asked to allow depends on
/// which of two names something happened to read first.
///
/// The error is the *first* one rather than the second, because the second is
/// what SURE looked at after `start` had already failed: a project that declares
/// neither should hear about `start`, which is the name a person would use, and
/// not about `dev`, which it never mentioned. A project that declares `dev` and
/// not `start` is not a gap at all and does not reach the error.
fn serving_command(package: &Package, runner: Runner) -> Result<String, MissingKind> {
    const SERVING_ROLES: &[ScriptRole] = &[ScriptRole::Start, ScriptRole::Dev];

    let mut first: Option<MissingKind> = None;
    for &role in SERVING_ROLES {
        match command_for(package, role, runner) {
            Ok(command) => return Ok(command),
            Err(kind) => {
                if first.is_none() {
                    first = Some(kind);
                }
            }
        }
    }

    // Unreachable: `SERVING_ROLES` is not empty, so the loop either returned or
    // set `first` at least once. Folded into the shape it would mean — a
    // component SURE has no command for — rather than panicking on a shipped
    // path, the same call `command_for` makes about its own unreachable arm.
    Err(first.unwrap_or(MissingKind::NotDeclared))
}

/// The settings that made this plan smaller than it could have been.
///
/// Derived from [`CheckPreference::disables`] rather than comparing each setting
/// to `Never` here, so that the question *is this check switched off* has one
/// answer in the product — the same predicate
/// [`Config::scope_reductions`](crate::config::Config::scope_reductions) reads.
fn reductions(preferences: &ChecksConfig) -> Vec<ScopeReduction> {
    let mut collected = Vec::new();
    for &(kind, _) in PROBES {
        if kind.preference(preferences).disables() {
            collected.push(kind.reduction());
        }
    }
    collected
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    use crate::discover::node::{Managers, PackageManager};
    use serde_json::json;

    /// A manifest with these scripts and no lockfile of its own.
    fn package(scripts: serde_json::Value) -> Package {
        Package::from_json(&json!({ "name": "fixture", "scripts": scripts }))
            .expect("this fixture is a manifest")
    }

    /// A project whose manager SURE agreed on, so that a declared script has a
    /// command rather than a `NoRunner` gap.
    fn run_by_npm() -> Runner {
        Runner::Agreed(PackageManager::Npm)
    }

    #[test]
    fn start_beats_dev_when_a_manifest_declares_both() {
        let declared = package(json!({ "start": "node server.js", "dev": "nodemon" }));
        // `npm start` rather than `npm run start`: npm has a shorthand for
        // `start`, and the command SURE would run is the package manager's
        // rendering of the script rather than this module's idea of it.
        assert_eq!(
            serving_command(&declared, run_by_npm()).unwrap(),
            "npm start",
            "a manifest that declares both roles did not answer with `start`"
        );
    }

    #[test]
    fn dev_is_the_answer_when_start_is_not_declared() {
        let declared = package(json!({ "dev": "nodemon" }));
        assert_eq!(
            serving_command(&declared, run_by_npm()).unwrap(),
            "npm run dev"
        );
    }

    #[test]
    fn a_manifest_that_declares_neither_hears_about_start() {
        let declared = package(json!({ "build": "tsc -b" }));
        assert_eq!(
            serving_command(&declared, run_by_npm()),
            Err(MissingKind::NotDeclared),
            "the gap named a role the manifest never mentioned"
        );
    }

    #[test]
    fn a_start_script_that_is_not_a_command_is_not_a_script_that_is_absent() {
        let declared = package(json!({ "start": ["node", "server.js"] }));
        assert_eq!(
            serving_command(&declared, run_by_npm()),
            Err(MissingKind::NotACommand),
            "a manifest that declared something unusable was described as one \
             that declared nothing"
        );
    }

    #[test]
    fn the_command_comes_from_the_runner_and_the_gap_does_not() {
        let declared = package(json!({ "start": "node server.js" }));
        // The variant and not the sentence: *which* of the two reasons a
        // project can leave SURE without a runner it gets is `crate::checks`'s
        // own question, and its own tests are where that is answered. What is
        // this module's is that a manifest which declared a start command
        // produced a gap about the runner rather than one about the script.
        assert!(
            matches!(
                serving_command(&declared, Runner::of(&Managers::default())),
                Err(MissingKind::NoRunner { .. })
            ),
            "a manifest that declared a start command was described as one that \
             did not"
        );
    }

    #[test]
    fn a_manifest_at_the_root_is_the_root_and_a_members_is_its_directory() {
        assert_eq!(directory_of("package.json"), PathBuf::new());
        assert_eq!(
            directory_of("packages/web/package.json"),
            PathBuf::from("packages").join("web")
        );
        assert_eq!(named(&directory_of("package.json")), "the project root");
        assert_eq!(
            named(&directory_of("packages/web/package.json")),
            "packages/web",
            "a component path was spelled with a platform separator"
        );
    }

    #[test]
    fn the_root_title_is_bare_and_a_members_names_the_component() {
        assert_eq!(
            titled(ProbeKind::Serve, &directory_of("package.json")),
            "start the project"
        );
        assert_eq!(
            titled(ProbeKind::Serve, &directory_of("packages/web/package.json")),
            "start the project in packages/web",
            "two components in one workspace would print one title"
        );
    }

    #[test]
    fn the_reductions_are_the_two_settings_this_plan_owns() {
        let switched_off = ChecksConfig {
            start_local_services: CheckPreference::Never,
            browser_probe: CheckPreference::Never,
            ..ChecksConfig::default()
        };
        assert_eq!(
            reductions(&switched_off),
            vec![
                ScopeReduction::LocalServicesDisabled,
                ScopeReduction::BrowserProbeDisabled,
            ]
        );
        assert!(
            !reductions(&switched_off).contains(&ScopeReduction::ExistingTestsDisabled),
            "this plan re-reported a reduction about the declared checks"
        );

        let default = ChecksConfig::default();
        assert!(
            reductions(&default).is_empty(),
            "a default configuration reported a reduction, so SURE would tell a \
             project it had switched something off that it had not"
        );
    }

    #[test]
    fn the_table_covers_every_kind_exactly_once() {
        let listed: Vec<ProbeKind> = PROBES.iter().map(|&(kind, _)| kind).collect();
        assert_eq!(
            listed,
            ProbeKind::ALL.to_vec(),
            "the table and the enum disagree about which kinds exist"
        );
    }

    #[test]
    fn every_probe_declares_the_action_its_kind_names() {
        // **Not a row against a match**, which would be a rule about this file.
        // The domain's own table is what `ActionKind` is checked against here,
        // so a probe whose action stopped being a thing SURE can plan fails
        // rather than being noticed in a report.
        for &(kind, row) in PROBES {
            let action = kind.action();
            assert!(
                ActionKind::ALL.contains(&action),
                "{kind:?} declares {action:?}, which is not an action in the domain"
            );
            if row.critical {
                assert!(
                    row.evidence_class.can_alone_support_must_fix(),
                    "{kind:?} is a critical check whose evidence class cannot carry \
                     a must-fix finding on its own"
                );
            }
        }
    }

    #[test]
    fn each_kind_reads_its_own_setting() {
        let only_services_on = ChecksConfig {
            start_local_services: CheckPreference::Always,
            browser_probe: CheckPreference::Never,
            ..ChecksConfig::default()
        };
        assert_eq!(
            ProbeKind::Serve.preference(&only_services_on),
            CheckPreference::Always
        );
        assert_eq!(
            ProbeKind::Interface.preference(&only_services_on),
            CheckPreference::Never,
            "the two kinds read one setting, so switching one off would silence \
             the other"
        );
    }

    #[test]
    fn the_reductions_are_in_the_order_a_report_states_them() {
        let from_the_table: Vec<ScopeReduction> =
            PROBES.iter().map(|&(kind, _)| kind.reduction()).collect();
        let from_the_enum: Vec<ScopeReduction> = ScopeReduction::ALL
            .iter()
            .copied()
            .filter(|reduction| from_the_table.contains(reduction))
            .collect();
        assert_eq!(
            from_the_table, from_the_enum,
            "the order this module would report reductions in is not the order \
             `ScopeReduction::ALL` puts them in, so one report would state them two ways"
        );
    }

    #[test]
    fn the_ways_a_manifest_can_leave_sure_without_a_command_read_differently() {
        // **The failure this closes is the one the module documentation spends a
        // paragraph on**: a `package.json` with `"start": {}` is a manifest that
        // declared something, and a report that called it *no start script* would
        // be describing a broken manifest as a project that never wrote one. The
        // variant already carries which of the ways it was, so a gap is *about*
        // the right fact either way — this is about the sentence, which is the
        // half a user reads.
        let sentences: Vec<String> = MissingKind::ALL
            .iter()
            .map(|kind| NotPlannedBecause::NoCommand(kind.clone()).plain_description())
            .collect();

        for (kind, sentence) in MissingKind::ALL.iter().zip(&sentences) {
            assert!(
                sentence.contains(&kind.plain_explanation()),
                "a gap about {kind:?} does not show {kind:?}'s own sentence, and the \
                 fact a reader is given is then one the project did not create: \
                 {sentence}"
            );
        }

        // And the same thing from the other side, which is the half that does not
        // restate the shape of the sentence: two of these ways reported in one
        // wording is a user who cannot tell them apart.
        let mut distinct = sentences.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            sentences.len(),
            "two of the ways a project can leave SURE without a command are \
             reported in the same sentence: {sentences:?}"
        );
    }

    #[test]
    fn the_tag_is_what_the_identifier_scheme_keeps() {
        for &(kind, _) in PROBES {
            let tag = kind.tag();
            assert!(
                tag.chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()),
                "{tag} is not written in the shape `check_id` keeps, so the readable \
                 half of every identifier built from it would be dropped"
            );
        }
    }
}

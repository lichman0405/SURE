//! What a project's own declared local services become.
//!
//! [`crate::runtime_probes`] plans a check that starts a project from what a
//! manifest *says about itself* — a `start` script, rendered through the package
//! manager that would run it. That rendering is a line of text, and turning a
//! line of text back into a program and an argument vector is the parse
//! `docs/adr/0014-planned-check-execution-contract.md` rejected. So a probe for
//! a discovered project is an observation rather than a run, and this module is
//! the other half: a project that writes down *how* it is started, in a form
//! SURE can hold as data, gets the run.
//!
//! ```text
//! checks.services in sure.yaml
//!        |
//!   config::services::ServiceDeclaration   (name, Launcher, port, readiness, page)
//!        |
//!   ServicePlan::of                        (validate; refuse by value)
//!        |
//!   DeclaredService { CheckProposal, ServiceCheckSpec, Option<BrowserCheckSpec> }
//!        |
//!   PlanBuilder::propose                   (and nothing further)
//! ```
//!
//! # The declaration is the door, and it is a narrow one
//!
//! A declaration names a [`Launcher`], a port on loopback and a path that answers
//! once the service is up. **There is no field anywhere in it that accepts a
//! command to run**, and this module never splits a string into arguments: the
//! program is [`NODE`], a name on this machine's `PATH` and not a resolved path,
//! and the argument vector is exactly one element — the entry the declaration
//! names, which stays one argument however many spaces it contains. A project
//! that could write `command: bash -c "…"` could ask SURE to run anything it
//! liked; a project that writes `kind: node_entry` has asked for one thing SURE
//! has a sentence for.
//!
//! # Every way a declaration can be wrong is a value
//!
//! [`ServiceRefusal`] has one variant per way, each with a sentence, and **a
//! refusal is never a dropped row**: a declaration that cannot become a check
//! appears in [`ServicePlan::refusals`] and reaches the stage-4 report, rather
//! than disappearing from a plan a reader would then take for a project SURE had
//! nothing to say about. The same rule as [`crate::checks::MissingCommand`] and
//! [`crate::runtime_probes::NotPlanned`], for the same reason.
//!
//! **Readiness and page are validated through
//! [`Endpoint::loopback`](crate::probe::Endpoint::loopback) and not through a
//! second copy of its rule.** That reuse is what makes a non-loopback URL, a
//! userinfo authority, an external redirect and a backslash path
//! *unrepresentable* here rather than merely unlikely: the endpoint has no host
//! field to fill in, and its own constructor refuses a path that would extend
//! the authority or add a header. A second `path_is_safe` in this file would be
//! a second answer to *what may be written into a request line*, and the day one
//! of the two was edited the checks would disagree about what is safe.
//!
//! # Planning ends here, at `propose`
//!
//! This module decides **which checks exist and what they would do**, and
//! nothing further. It constructs no
//! [`AdmittedCommand`](crate::enforce::AdmittedCommand), calls no
//! [`Enforcement::admitted`](crate::enforce::Enforcement::admitted) and starts
//! nothing: whether a check may run is the mode's answer, and the plan's answer
//! is a `CheckOperation` a later stage is handed. The route from a proposal to a
//! process is one route for the whole product, and this file joins it at the
//! same place every other proposer does.
//!
//! **A browser check costs its service's launch and needs its own permission.**
//! `pipeline.rs`'s permission-plan loop maps a
//! [`CheckOperation::Browser`](crate::planned_work::CheckOperation::Browser) to
//! the *service's* command — a browser check starts the service, so that is what
//! it launches — and the page's own permission is **not** implied by authorising
//! that launch:
//! [`crate::browser::absence`] asks [`decide`](sure_domain::execution::decide)
//! about [`ActionKind::BrowserProbe`] separately, which needs
//! [`Permission::ConnectService`](sure_domain::execution::Permission::ConnectService)
//! — where starting the service needs
//! [`Permission::RunProjectCode`](sure_domain::execution::Permission::RunProjectCode)
//! — so a run that permitted a service and not the page gets a serve row and an
//! absence rather than a look. That was read and left alone; it is written here
//! because it is the question a reader of this file is most likely to have.

use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use crate::checks::{CHECK_LIMITS, check_id};
use crate::config::{CheckPreference, ChecksConfig, Config, Launcher, ServiceDeclaration};
use crate::planned_work::{
    BrowserCheckSpec, CheckOperation, CommandSpec, PlannedWork, Readiness, ServiceCheckSpec,
    WorkRefusal,
};
use crate::probe::{Endpoint, EndpointError};
use crate::process::Environment;
use crate::runtime_probes::ProbeKind;
use crate::scan::display_path;
use crate::schedule::{CheckProposal, CheckReason, PlanBuilder};

/// The program SURE starts a declared service with.
///
/// **A name and never a path.** Which `node` this machine runs is the machine's
/// own answer, through `PATH` and its extension rules, and it is the same answer
/// `crate::process` gives for every other program SURE starts. Naming a resolved
/// executable would freeze one machine's layout into a check, and the second
/// machine would run a path that is not there.
const NODE: &str = "node";

/// How long a declared service is given to come up, and then to answer.
///
/// The same shape as [`crate::planned_check_runner`]'s `ONE_EXCHANGE`: one
/// number, in one place, with the argument for it written down rather than
/// repeated at each call site. A minute is long enough for a cold development
/// server on the projects this build is aimed at, and it is an order of magnitude
/// below the fifteen minutes [`CHECK_LIMITS`] gives the command itself — which
/// matters for a reason `crate::runtime_start` states rather than for taste: a
/// window that is not shorter than the command's whole-life deadline is refused
/// there, because the service would be stopped by its own budget before the
/// window ever closed. Sixty seconds cannot reach that refusal from here.
const A_SERVICES_WINDOW: Duration = Duration::from_secs(60);

/// The readable half of the service row's check identifier.
///
/// [`check_id`] keeps only `[a-z0-9]` from this, so it is written in the shape
/// the identifier scheme wants. See that function's own documentation for why
/// the tag is filtered rather than escaped, and why two checks sharing one
/// identifier are reported rather than merged.
const SERVICE_TAG: &str = "service";

/// The readable half of the browser row's check identifier.
///
/// **A second tag, so that the two rows are two checks.** One declaration plans
/// a check that starts the service and a check that looks at a page on it; an
/// identifier derived from the same tag for both would make them one check in a
/// schedule, and the second would be reported as a duplicate of the first rather
/// than run.
const SERVICE_PAGE_TAG: &str = "servicepage";

/// What SURE would do about a project's declared local services.
///
/// Built by [`Self::of`], which is the only constructor, from **the project root
/// and the project's own `checks` settings** — one reading of the filesystem and
/// one configuration, so a plan cannot describe a directory two ways.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePlan {
    services: Vec<DeclaredService>,
    refusals: Vec<ServiceRefusal>,
    gaps: Vec<ServiceGap>,
}

impl ServicePlan {
    /// What the project's declarations become, and what they could not.
    ///
    /// **The declarations are read in the order the file lists them**, which is
    /// not a property of the project: [`PlanBuilder`] sorts, and the order here
    /// is only the order a refusal is reported in — the order a reader of
    /// `sure.yaml` would look for it.
    ///
    /// Nothing here reads the clock, opens a socket or checks whether `node` is
    /// installed. Every question is about the project's own files and its own
    /// configuration, so a plan made on a machine without `node` is the same
    /// plan; the check reports the missing program when it runs, which is where
    /// `crate::process` gives that answer.
    ///
    /// **Every declaration is validated before any preference is read**, so a
    /// project that has switched local services off still learns that one of its
    /// declarations names a file outside the project — that is a defect in the
    /// file, and it stays one whatever the setting says. Within a declaration the
    /// readiness path and the page are validated before a preference is
    /// consulted for the same reason, and the browser preference is then read
    /// before the page, which is what decides that a page-less declaration under
    /// `auto` records the setting rather than the missing page.
    #[must_use]
    pub fn of(root: &Path, checks: &ChecksConfig) -> Self {
        let mut services: Vec<DeclaredService> = Vec::new();
        let mut refusals: Vec<ServiceRefusal> = Vec::new();
        let mut gaps: Vec<ServiceGap> = Vec::new();
        let mut declared: Vec<String> = Vec::new();

        for declaration in &checks.services {
            // The name as a user reads it, trimmed once and used everywhere: an
            // identifier, a title and a sentence should not disagree about
            // whether trailing spaces are part of a service's name.
            let name = declaration.name.trim().to_owned();
            if name.is_empty() {
                refusals.push(ServiceRefusal::NameIsEmpty);
                continue;
            }
            if declared.contains(&name) {
                refusals.push(ServiceRefusal::NameAppearsTwice { name });
                continue;
            }

            let directory = match declared_directory(root, declaration) {
                Ok(directory) => directory,
                Err(refusal) => {
                    refusals.push(refusal);
                    continue;
                }
            };
            let entry = match declared_entry(root, &directory.full, &declaration.launcher) {
                Ok(entry) => entry,
                Err(refusal) => {
                    refusals.push(refusal);
                    continue;
                }
            };
            if declaration.port == 0 {
                refusals.push(ServiceRefusal::PortIsZero { name });
                continue;
            }
            let readiness = match endpoint_of(&name, declaration.port, &declaration.readiness) {
                Ok(endpoint) => endpoint,
                Err(refusal) => {
                    refusals.push(refusal);
                    continue;
                }
            };
            let page = match &declaration.page {
                Some(page) => match endpoint_of(&name, declaration.port, page) {
                    Ok(endpoint) => Some(endpoint),
                    Err(refusal) => {
                        refusals.push(refusal);
                        continue;
                    }
                },
                None => None,
            };

            // Everything the declaration named has been read. It is a service
            // SURE would start from here on, and the name is spent: a second
            // declaration using it is a duplicate whatever happens below.
            declared.push(name.clone());

            // A service nobody asked SURE to start is not a service, so the two
            // rows stand or fall together: a browser check on a service this
            // project does not start would start it. `checks.browser_probe` is
            // not consulted for it at all — the setting that decided is the one
            // the gap names, and asking twice would put a second sentence under
            // one decision.
            if !plans(ProbeKind::Serve.preference(checks), ProbeKind::Serve) {
                gaps.push(ServiceGap::LocalServicesDisabled { name });
                continue;
            }

            let built = declared_service(&name, &entry, &directory, readiness, page, checks);
            gaps.extend(built.gaps);
            refusals.extend(built.refusals);
            services.push(built.service);
        }

        Self {
            services,
            refusals,
            gaps,
        }
    }

    /// The declarations that became checks.
    #[must_use]
    pub fn services(&self) -> &[DeclaredService] {
        &self.services
    }

    /// The declarations that could not, one value per declaration.
    ///
    /// **One refusal per declaration and never a merged sentence**: two services
    /// with two different problems are two things to fix, and a report that
    /// combined them would name one file for two readers looking at different
    /// ones.
    #[must_use]
    pub fn refusals(&self) -> &[ServiceRefusal] {
        &self.refusals
    }

    /// The checks that were not planned, and which setting or missing field
    /// decided that.
    ///
    /// **Not a `ScopeReduction` and deliberately not reported as one.**
    /// [`Config::scope_reductions`](crate::config::Config::scope_reductions)
    /// already states [`ScopeReduction::LocalServicesDisabled`] and
    /// [`ScopeReduction::BrowserProbeDisabled`](crate::config::ScopeReduction)
    /// for the run as a whole; a plan that re-reported them per declaration would
    /// state one reduction twice, in two vocabularies, and be wrong about which
    /// layer it was describing. What is here instead is the part the run-wide
    /// setting cannot say: *which* declaration it applied to, and what a
    /// preference that is not `never` still left unplanned.
    #[must_use]
    pub fn gaps(&self) -> &[ServiceGap] {
        &self.gaps
    }

    /// Whether the project declared nothing, and nothing was recorded.
    ///
    /// True for the project that writes no `checks.services` at all, which is
    /// every project today unless somebody asked for one — and **not** true for
    /// a project whose declarations were all refused, which produces refusals
    /// rather than nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.services.is_empty() && self.refusals.is_empty() && self.gaps.is_empty()
    }

    /// Hands every check to a plan builder.
    ///
    /// **The builder's refusals are the record, which is why nothing is
    /// returned here**, exactly as
    /// [`ProbePlan::add_to`](crate::runtime_probes::ProbePlan::add_to) has it: a
    /// proposal this module got wrong cannot disappear by being ignored. Nothing
    /// this module builds can be refused in the first place — every check has a
    /// title, a reason naming a file and a command, and exactly one action — and
    /// the unit tests below hold the two operations rather than this sentence.
    pub fn add_to(&self, builder: &mut PlanBuilder) {
        for declared in &self.services {
            propose(
                builder,
                declared.service.clone(),
                CheckOperation::Service(declared.service_spec.clone()),
            );
            if let (Some(proposal), Some(spec)) = (&declared.browser, &declared.browser_spec) {
                propose(
                    builder,
                    proposal.clone(),
                    CheckOperation::Browser(spec.clone()),
                );
            }
        }
    }

    /// One line per service, then one per refusal, then one per gap, in the
    /// plan's own order.
    #[must_use]
    pub fn plain_description(&self) -> Vec<String> {
        self.services
            .iter()
            .map(DeclaredService::plain_description)
            .chain(self.refusals.iter().map(ServiceRefusal::plain_description))
            .chain(self.gaps.iter().map(ServiceGap::plain_description))
            .collect()
    }
}

/// One declaration that became a check, or two.
///
/// **A composition and not a restatement.** The title, the severity, the
/// criticality, the evidence class, the reason and the action list live in
/// [`CheckProposal`] and are not copied here; what this type adds is the work
/// beside each proposal — the [`ServiceCheckSpec`] that would start the service,
/// and the [`BrowserCheckSpec`] that would look at a page once it is up. A
/// proposal and its operation are built together and stored together, so a
/// caller cannot hold one without the other.
///
/// There is no constructor and no setter: the only way to get one is
/// [`ServicePlan::of`], which is also the only place a declaration is validated
/// against the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredService {
    name: String,
    directory: PathBuf,
    service: CheckProposal,
    service_spec: ServiceCheckSpec,
    browser: Option<CheckProposal>,
    browser_spec: Option<BrowserCheckSpec>,
}

impl DeclaredService {
    /// The name the declaration gave this service.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The check that starts it.
    #[must_use]
    pub fn service(&self) -> &CheckProposal {
        &self.service
    }

    /// The check that looks at a page on it, when the declaration named one.
    ///
    /// `None` is a project that declared no page, and it is the only way to get
    /// one: a declaration that named a page has a browser row whenever the
    /// preference plans it, and a row that could not be built is a
    /// [`ServiceRefusal`] rather than a service with this field quietly empty.
    #[must_use]
    pub fn browser(&self) -> Option<&CheckProposal> {
        self.browser.as_ref()
    }

    /// The sentence a report shows for this declaration.
    ///
    /// Built from the service row's title, the directory the declaration runs in
    /// and the reason, in the shape
    /// [`RuntimeProbe::plain_description`](crate::runtime_probes::RuntimeProbe::plain_description)
    /// uses, so a declared service's row and a discovered component's row read
    /// alike in one report.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "{} in {}: {}",
            self.service.title(),
            named(&self.directory),
            self.service.reason().plain_description()
        )
    }
}

/// The outcome of one declaration: the rows it became, and what was left over.
///
/// Internal to [`ServicePlan::of`], and the reason it is three lists rather than
/// an early return: a declaration can produce a check, a gap and a refusal at
/// once — a planned service whose page could not be built is exactly that — and
/// each of the three belongs to a different reader.
struct Built {
    service: DeclaredService,
    gaps: Vec<ServiceGap>,
    refusals: Vec<ServiceRefusal>,
}

/// Why a declared service could not become a check.
///
/// One variant per way, each holding what a reader needs to find the declaration
/// in their own file. **None of these is a warning about what might go wrong**:
/// each names a value SURE will not act on, in the shape
/// [`crate::probe::EndpointError`] and
/// [`crate::checks::MissingKind`] use, so that the reason survives to whatever
/// reports it instead of becoming a sentence written where it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRefusal {
    /// A declaration has no name.
    NameIsEmpty,
    /// Two declarations have the same name.
    NameAppearsTwice {
        /// The name both declarations used.
        name: String,
    },
    /// The declared directory is not inside the project.
    DirectoryOutsideTheProject {
        /// The directory the declaration named.
        directory: String,
    },
    /// The declared directory is not there.
    DirectoryIsNotThere {
        /// The directory the declaration named.
        directory: String,
    },
    /// The entry does not stay inside the directory the declaration runs in.
    EntryIsOutsideTheProject {
        /// The entry the declaration named.
        entry: String,
    },
    /// The entry is not a file `node` would start.
    EntryIsNotAJavaScriptFile {
        /// The entry the declaration named.
        entry: String,
    },
    /// The entry is not there.
    EntryIsNotThere {
        /// The entry the declaration named.
        entry: String,
    },
    /// The declaration says port zero.
    PortIsZero {
        /// The name of the service.
        name: String,
    },
    /// A path the declaration wrote cannot be written into a request line.
    UnsafePath {
        /// The name of the service.
        name: String,
        /// The path the declaration wrote.
        path: String,
        /// Why the probe's own constructor refused it.
        error: EndpointError,
    },
    /// The page's browser check could not be built.
    ///
    /// **The unreachable arm of a `Result` that is not allowed to be
    /// unwrapped.** The service SURE builds always names the port it answers on
    /// — `Readiness::Answers` is the only readiness this module constructs — and
    /// the page went through [`Endpoint::loopback`] one step earlier, so neither
    /// refusal [`BrowserCheckSpec::new`] documents can arise from here. It is a
    /// variant rather than a `debug_assert` because a refusal SURE cannot
    /// explain is still a check SURE does not plan, and the honest thing to
    /// carry is the refusal's own sentence.
    BrowserRefused {
        /// The name of the service.
        name: String,
        /// Why the browser check was refused.
        refusal: WorkRefusal,
    },
}

impl ServiceRefusal {
    /// One line of plain language naming what to fix.
    #[must_use]
    pub fn plain_description(&self) -> String {
        match self {
            Self::NameIsEmpty => "a `checks.services` entry has no name, and the name is \
                 what tells two services in one file apart."
                .to_owned(),
            Self::NameAppearsTwice { name } => format!(
                "`checks.services` declares {name} twice. The second one is not planned: \
                 two services with one name would be two checks a report could not tell \
                 apart."
            ),
            Self::DirectoryOutsideTheProject { directory } => format!(
                "the declared directory {directory} is not inside the project, and SURE \
                 only starts what it was handed. A path that leaves the project and a \
                 link that leads out of it are refused for the same reason."
            ),
            Self::DirectoryIsNotThere { directory } => {
                format!("the declared directory {directory} is not a directory of this project.")
            }
            Self::EntryIsOutsideTheProject { entry } => format!(
                "the declared entry {entry} does not lead to a file inside the project, \
                 so SURE will not start it. A path that climbs out and a link that \
                 points out are refused for the same reason."
            ),
            Self::EntryIsNotAJavaScriptFile { entry } => format!(
                "the declared entry {entry} is not a JavaScript file. `node_entry` starts \
                 a file ending in .js, .mjs or .cjs, and nothing else is guessed at."
            ),
            Self::EntryIsNotThere { entry } => format!(
                "the declared entry {entry} is not a file that is there, so there is \
                 nothing to start."
            ),
            Self::PortIsZero { name } => format!(
                "{name} declares port 0. Zero is not a port SURE can ask on, so there is \
                 no address to check and nothing was picked in the project's place."
            ),
            Self::UnsafePath { name, path, error } => format!(
                "{name} declares the path {path:?}, which cannot be asked for as it \
                 stands: {error}."
            ),
            Self::BrowserRefused { name, refusal } => format!(
                "the browser check for {name} could not be built, so only the service is \
                 planned: {refusal}."
            ),
        }
    }
}

impl std::fmt::Display for ServiceRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.plain_description())
    }
}

/// A check a declared service asked for that was not planned, and why.
///
/// The gap is about one declaration, and its sentence names the setting that
/// decided it — a reader has to be able to tell *the project switched this off*
/// from *SURE does not open a page on a guess*, because one is their own
/// decision and the other is a preference they can change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceGap {
    /// `checks.start_local_services: never`, so the service is not started.
    LocalServicesDisabled {
        /// The name of the service.
        name: String,
    },
    /// `checks.browser_probe: never`, so no browser looks at it.
    BrowserProbeDisabled {
        /// The name of the service.
        name: String,
    },
    /// `checks.browser_probe: auto`, which does not open a page on a guess.
    ///
    /// Recorded for a declaration whether or not it named a page: the preference
    /// is what decided, and a page would not have changed the answer under
    /// `auto`. See [`ServicePlan::of`]'s ordering.
    AutoDoesNotLookAtAPage {
        /// The name of the service.
        name: String,
    },
    /// `checks.browser_probe: always`, and the declaration named no page.
    NoPageToLookAt {
        /// The name of the service.
        name: String,
    },
}

impl ServiceGap {
    /// One line of plain language naming the setting or the missing field.
    #[must_use]
    pub fn plain_description(&self) -> String {
        match self {
            Self::LocalServicesDisabled { name } => format!(
                "{name} is not started: this project set `checks.start_local_services` to \
                 `never`."
            ),
            Self::BrowserProbeDisabled { name } => format!(
                "{name} is not looked at in a browser: this project set \
                 `checks.browser_probe` to `never`."
            ),
            Self::AutoDoesNotLookAtAPage { name } => format!(
                "{name} is not looked at in a browser: `checks.browser_probe: auto` does \
                 not open a page, because nothing in a project's shape says it has an \
                 interface. `checks.browser_probe: always` asks for this check."
            ),
            Self::NoPageToLookAt { name } => format!(
                "{name} names no page, so no browser is opened: there is nothing to look \
                 at, and SURE did not invent one."
            ),
        }
    }
}

impl std::fmt::Display for ServiceGap {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.plain_description())
    }
}

/// A declaration's directory, as a working directory and as the declaration
/// spelled it.
struct Directory {
    /// Absolute: the root joined with what the declaration named.
    full: PathBuf,
    /// The path the declaration named, empty for the root.
    ///
    /// Held so a sentence can say where a service runs in the project's own
    /// terms. [`display_path`] turns it into the `/`-separated spelling a report
    /// uses, and the empty path is the root — the same convention
    /// [`RuntimeProbe::component`](crate::runtime_probes::RuntimeProbe::component)
    /// uses for the project itself.
    declared: PathBuf,
}

/// The directory a declaration runs in, or why there is none.
///
/// Three questions in one place, because they are one decision — *may SURE run
/// something here* — and a caller that asked them separately could forget one.
fn declared_directory(
    root: &Path,
    declaration: &ServiceDeclaration,
) -> Result<Directory, ServiceRefusal> {
    let Some(named) = declaration.directory.as_deref() else {
        return Ok(Directory {
            full: root.to_path_buf(),
            declared: PathBuf::new(),
        });
    };
    let declared = Path::new(named);
    if !stays_inside(declared) {
        return Err(ServiceRefusal::DirectoryOutsideTheProject {
            directory: named.to_owned(),
        });
    }
    // Joined after the refusal rather than before it: `root.join("../../../etc")`
    // is a path that *means* somewhere else, and a path this function only ever
    // builds from something it has already agreed to is one fewer thing to
    // reason about downstream.
    let full = root.join(declared);
    if !full.is_dir() {
        return Err(ServiceRefusal::DirectoryIsNotThere {
            directory: named.to_owned(),
        });
    }
    // After *is it there*, so that a directory that does not exist gets the
    // sentence about not existing rather than the one about leading elsewhere.
    // A link is the case the textual test above cannot see: `escape` is a name
    // inside the project and a junction to somewhere outside it, and the command
    // SURE builds runs with this directory as its working directory.
    if !resolves_inside(root, &full) {
        return Err(ServiceRefusal::DirectoryOutsideTheProject {
            directory: named.to_owned(),
        });
    }
    Ok(Directory {
        full,
        declared: declared.to_path_buf(),
    })
}

/// The entry a declaration names, or why SURE will not start it.
///
/// The entry comes back **as the declaration wrote it** and not as a joined
/// path: it is the argument the program is handed, and the directory it is
/// relative to is the command's working directory. Rewriting it into an absolute
/// path would put a machine's layout into the argument vector of a check whose
/// command is otherwise the same on every machine.
///
/// What it is handed for that reason is `root` **and** the directory: the one is
/// what the file has to resolve inside, the other is where it has to be found.
/// Both are asked of the path that actually exists rather than of the spelling,
/// so an entry reached through a link out of the project is refused here even
/// though every component of its name is a plain one.
fn declared_entry(
    root: &Path,
    directory: &Path,
    launcher: &Launcher,
) -> Result<String, ServiceRefusal> {
    let entry = match launcher {
        Launcher::NodeEntry { entry } => entry,
    };
    let path = Path::new(entry);
    if !stays_inside(path) {
        return Err(ServiceRefusal::EntryIsOutsideTheProject {
            entry: entry.clone(),
        });
    }
    if !is_javascript(path) {
        return Err(ServiceRefusal::EntryIsNotAJavaScriptFile {
            entry: entry.clone(),
        });
    }
    let full = directory.join(path);
    if !full.is_file() {
        return Err(ServiceRefusal::EntryIsNotThere {
            entry: entry.clone(),
        });
    }
    // Against the project root and not against the directory, because that is
    // the claim the sentence makes and the one the whole module rests on: every
    // path SURE starts a program with is inside the tree it was handed. The
    // directory has been resolved by the time this runs, so a file reached
    // through a link is the only way past the test above.
    if !resolves_inside(root, &full) {
        return Err(ServiceRefusal::EntryIsOutsideTheProject {
            entry: entry.clone(),
        });
    }
    Ok(entry.clone())
}

/// Whether a path a declaration wrote stays inside the thing it is named in.
///
/// **A path that leaves is refused rather than resolved**, which is the
/// direction this repository takes everywhere a project names a place: `..` is
/// refused even when it would land back inside the project, because *would it*
/// depends on the other components, and a rule that has to be evaluated is a
/// rule that can be evaluated wrongly. Only a plain relative path passes —
/// `Component::Normal` and the leading `.` a person may write.
///
/// **This reads the text of the path and not the file system, so it is one of
/// two locks rather than the whole door.** It cannot see a link: a directory that
/// *is* a junction, or an entry reached through one, is inside the project by its
/// spelling and somewhere else by its resolution. [`resolves_inside`] asks that
/// second question of everything this one has already agreed to, and a caller
/// needs both — this one produces the better sentence for `..` and an absolute
/// path, and that one catches what no sentence can read.
///
/// Absolute paths are refused by the same function and for a stronger reason:
/// `C:\Windows` and `/etc` are not relative to anything, so a declaration naming
/// one is asking SURE to leave the tree it was handed. On Windows a
/// root-relative `\Windows` is *not* absolute as far as [`Path::is_absolute`] is
/// concerned, and it is refused here anyway because its first component is a root
/// rather than a name.
fn stays_inside(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

/// Whether a path that is there resolves to a place inside the project.
///
/// **The second lock on [`stays_inside`]'s door, and the one that costs a
/// syscall.** A link inside the project — a junction on Windows, a symbolic link
/// anywhere — passes every textual test and leads out of the tree, and a
/// declaration naming one would have SURE start a program outside the project it
/// was handed while every sentence in this module said otherwise. `mklink /J`
/// needs no privilege at all, so this is not a rule about a hostile project with
/// administrator rights; it is a rule about the ordinary working tree.
///
/// **Both sides are resolved before they are compared**, so the comparison is
/// between two answers from the same function. `std::fs::canonicalize` returns a
/// verbatim `\\?\C:\…` path on Windows and answers `/private/var` for `/var` on
/// macOS, and a check that resolved one side only would refuse a project whose
/// root is reached through either — a false refusal, which is the visible
/// direction but still a project's declaration turned into nothing.
///
/// [`crate::paths::is_within`] does the comparing, because it is this
/// repository's one answer to *is this below that*: component by component rather
/// than by string prefix, case folded the way the platform folds it, and through
/// the verbatim prefix a canonicalised path carries. It is the same question the
/// store asks about its own evidence, asked about a program SURE is about to
/// start.
///
/// **A path that cannot be resolved is not inside**, which is this module's
/// direction everywhere: a declaration SURE cannot confirm is a refusal with a
/// sentence attached rather than a check that quietly starts something.
fn resolves_inside(root: &Path, full: &Path) -> bool {
    let (Ok(root), Ok(full)) = (std::fs::canonicalize(root), std::fs::canonicalize(full)) else {
        return false;
    };
    crate::paths::is_within(&full, &root)
}

/// Whether a name is one of the three extensions `node` starts.
///
/// The comparison ignores case, and that is this repository's Windows
/// discipline rather than a convenience: `App.JS` is a JavaScript file to a user
/// on the platform SURE is primarily developed on, and refusing it would refuse
/// a project that works. What the file is *checked* against is still the
/// declaration — it must exist, and it must be inside the directory — so the
/// extension is a question about intent and not the whole of the name.
fn is_javascript(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &["js", "mjs", "cjs"];
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXTENSIONS
                .iter()
                .any(|wanted| extension.eq_ignore_ascii_case(wanted))
        })
}

/// The loopback endpoint a path names on a declaration's own port.
fn endpoint_of(name: &str, port: u16, path: &str) -> Result<Endpoint, ServiceRefusal> {
    Endpoint::loopback(port, path).map_err(|error| ServiceRefusal::UnsafePath {
        name: name.to_owned(),
        path: path.to_owned(),
        error,
    })
}

/// The two checks one declaration asks for, and the gaps a preference left.
///
/// **`Err` is a browser row that could not be built and never a service row that
/// could not**: the service row is complete by the time the browser is
/// attempted, so a refused browser is a refusal *beside* a planned service
/// rather than instead of one. See [`ServiceRefusal::BrowserRefused`] for why
/// that arm is unreachable from the values this module builds and why it is
/// still handled.
fn declared_service(
    name: &str,
    entry: &str,
    directory: &Directory,
    readiness: Endpoint,
    page: Option<Endpoint>,
    checks: &ChecksConfig,
) -> Built {
    // The work both rows stand on: one command, started in the declaration's own
    // directory, with the entry as its only argument.
    //
    // **The environment is not SURE's own**, which is `docs/adr/0014`'s decision
    // 11 and the one thing here that is not like a declared check: a service is
    // a program SURE starts *in order to watch it*, and SURE's own environment is
    // a fact about the machine that the project never asked for. A declared
    // check is the project's own command in the user's own environment, and
    // `crate::checks::command_operation` says so; this is the other case, and the
    // two are different on purpose.
    //
    // Nothing is passed at all rather than a curated list, because a listed
    // variable would be SURE deciding what a project's service needs. What a
    // service that needs `PATH` does is `crate::process`'s business — a program
    // is found by name for every check SURE runs, this one included.
    let command = CommandSpec::new(
        NODE,
        &directory.full,
        Environment::only(Vec::new()),
        CHECK_LIMITS,
    )
    .with_arguments(vec![entry]);

    // Built before either proposal, because both of them stand on it: the
    // browser check carries the very same service — same command, same port,
    // same readiness path — rather than a second copy that could disagree.
    let service_spec = ServiceCheckSpec::new(
        command,
        Readiness::Answers {
            endpoint: readiness,
        },
        A_SERVICES_WINDOW,
    );

    let row = ProbeKind::Serve.row();
    let service = CheckProposal::new(
        check_id(name, SERVICE_TAG),
        format!("start {name}"),
        row.severity,
        row.critical,
        row.evidence_class,
        declared_reason(entry),
        &[ProbeKind::Serve.action()],
    );

    let mut gaps = Vec::new();
    let mut refusals = Vec::new();

    // The preference is asked **before** the page, and the order is the answer to
    // *which reason does a declaration with no page get*. Under `auto` the reason
    // is the preference — declaring a page would not have changed anything, so a
    // gap naming the missing page would be a sentence that implies a change the
    // setting does not allow. `NoPageToLookAt` therefore appears only where
    // adding a page really is the one thing that would plan this check, which is
    // `always`.
    let preference = ProbeKind::Interface.preference(checks);
    let browser = if !plans(preference, ProbeKind::Interface) {
        gaps.push(browser_gap(preference, name));
        None
    } else if let Some(page) = &page {
        match BrowserCheckSpec::new(service_spec.clone(), page.path(), expectation(name, page)) {
            Ok(spec) => {
                let row = ProbeKind::Interface.row();
                Some((
                    CheckProposal::new(
                        check_id(name, SERVICE_PAGE_TAG),
                        format!("check {name} in a browser"),
                        row.severity,
                        row.critical,
                        row.evidence_class,
                        declared_reason(entry),
                        &[ProbeKind::Interface.action()],
                    ),
                    spec,
                ))
            }
            Err(refusal) => {
                refusals.push(ServiceRefusal::BrowserRefused {
                    name: name.to_owned(),
                    refusal,
                });
                None
            }
        }
    } else {
        gaps.push(ServiceGap::NoPageToLookAt {
            name: name.to_owned(),
        });
        None
    };

    let (browser, browser_spec) = match browser {
        Some((proposal, spec)) => (Some(proposal), Some(spec)),
        None => (None, None),
    };

    Built {
        service: DeclaredService {
            name: name.to_owned(),
            directory: directory.declared.clone(),
            service,
            service_spec,
            browser,
            browser_spec,
        },
        gaps,
        refusals,
    }
}

/// Whether a preference plans this kind's check.
///
/// **[`ProbeKind::row`]'s `auto_plans_it` and not a decision of this module's.**
/// The three preferences do not mean the same thing for the two kinds —
/// `runtime_probes.rs` argues that at length — and a declared service read by a
/// second rule would be graded differently from a discovered one for no reason a
/// user could see. `never` and `always` are the two answers the values carry;
/// `auto` is the one that is a judgement about the kind, and it is read from the
/// kind.
fn plans(preference: CheckPreference, kind: ProbeKind) -> bool {
    match preference {
        CheckPreference::Never => false,
        CheckPreference::Always => true,
        CheckPreference::Auto => kind.row().auto_plans_it,
    }
}

/// Why a browser row this preference did not plan was not planned.
fn browser_gap(preference: CheckPreference, name: &str) -> ServiceGap {
    if preference.disables() {
        return ServiceGap::BrowserProbeDisabled {
            name: name.to_owned(),
        };
    }
    // `never` is the only preference that switches a check off, so this is
    // `auto` — the preference that plans an interface row only where the kind's
    // own row says it does, and `ProbeKind::Interface`'s says it does not.
    ServiceGap::AutoDoesNotLookAtAPage {
        name: name.to_owned(),
    }
}

/// The reason both of a declaration's checks carry.
///
/// **`declared_in` is [`Config::FILE_NAME`] and not a second copy of the name.**
/// `sure.yaml` is where a declaration is written, and it is the constant the
/// loader reads the file by; spelling it out here again would be two names for
/// one file, which is exactly the drift this repository refuses elsewhere.
///
/// **`command` is a rendering *from* the typed command and never the other way
/// round.** The typed form is the [`Launcher`] the declaration carries, and what
/// a reader is shown is that value written out — `node`, a space, the entry. The
/// string is built here from the same two values the argument vector is built
/// from, so it cannot become the source of either: nothing anywhere takes this
/// rendering apart again.
fn declared_reason(entry: &str) -> CheckReason {
    CheckReason::DeclaredCommand {
        declared_in: Config::FILE_NAME.to_owned(),
        command: format!("{NODE} {entry}"),
    }
}

/// What a browser check on a declared service is opened to see.
///
/// **A sentence about the page and nothing beyond it.** The first half is what
/// the check reads; the second is the limit, said out loud, because a project
/// that reads "checked in a browser" could otherwise take it for a claim that
/// its sign-in, its checkout or its database was exercised. None of them is, and
/// nothing here reaches anything but the service SURE started on loopback.
fn expectation(name: &str, page: &Endpoint) -> String {
    format!(
        "the page {name} serves at {page} is opened on this machine and read: whether it \
         arrives with the browser reporting no error. Nothing beyond that page is \
         exercised — no account is created, nothing is paid for, no message is sent and \
         nothing is stored."
    )
}

/// A component path as a report spells it, with the root named rather than
/// blank.
///
/// [`crate::runtime_probes`] has the same function over the same convention and
/// the same reason: an empty path in a sentence is a hole, so the root is named
/// once here rather than at each place a sentence needs it.
fn named(directory: &Path) -> String {
    if directory.as_os_str().is_empty() {
        "the project root".to_owned()
    } else {
        display_path(directory)
    }
}

/// Hand one check to the builder, holding the contract `PlanBuilder` documents.
///
/// **A refusal here is a defect in this module rather than a fact about a
/// project**, which is why it is a `debug_assert` and not a value: every check
/// built above has a title, a reason naming a file and a command, and exactly one
/// action. The `debug_assert` mirrors
/// [`ProbePlan::add_to`](crate::runtime_probes::ProbePlan::add_to), which holds
/// the same contract on the same builder — and the refusal is *not* dropped even
/// so: `propose` has already recorded it on the builder's own list by the time
/// it returns.
fn propose(builder: &mut PlanBuilder, proposal: CheckProposal, operation: CheckOperation) {
    let work = PlannedWork::new(proposal, operation);
    if let Err(refusal) = builder.propose(work) {
        debug_assert!(
            builder.refused().contains(&refusal),
            "the builder returned a refusal it did not record"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Tests for **this** module.
    //!
    //! Every fixture is a real directory on disk, because most of what this
    //! module decides is asked of the file system: whether a directory is inside
    //! the project, whether it is there, whether an entry is a file with one of
    //! three extensions. A fixture made of paths that do not exist would exercise
    //! a validation that never ran, and would pass just as well if it were
    //! deleted.

    use super::*;
    use crate::consent::PermissionPlan;
    use crate::enforce::Enforcement;
    use crate::schedule::ScheduledCheck;
    use std::ffi::{OsStr, OsString};
    use sure_domain::execution::{
        ActionKind, ExecutionDecision, ExecutionMode, ExecutionPermissions, Permission,
    };
    use sure_domain::ids::{CheckId, FingerprintId};
    use sure_domain::severity::Severity;

    fn scratch(name: &str) -> PathBuf {
        sure_testkit::scratch::directory("service plan", name)
    }

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        std::fs::write(path, text)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
    }

    /// A project with one `server.js` at the root and nothing else.
    fn project(name: &str) -> PathBuf {
        let root = scratch(name);
        write(&root.join("server.js"), "// a service\n");
        root
    }

    /// A directory link at `link`, leading to `target`.
    ///
    /// **A junction on Windows rather than a symbolic link**, because
    /// `std::os::windows::fs::symlink_dir` needs Developer Mode or administrator
    /// rights and `mklink /J` needs neither — `discover_node.rs` probed both on
    /// this host before this test was written, and the answer there is the answer
    /// here. That is not a weaker mechanism for the purpose: a junction is what a
    /// link out of a working tree actually is on the platform SURE is developed
    /// on, and it is created without any privilege, which is the whole reason
    /// [`resolves_inside`] exists. The arguments are spelled with backslashes
    /// because `mklink` reads `/` as the start of one of its own switches.
    #[cfg(windows)]
    fn link_directory(target: &Path, link: &Path) {
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap_or_else(|error| panic!("cannot run cmd: {error}"));
        assert!(
            output.status.success(),
            "mklink /J {} {} failed: {}{}",
            link.display(),
            target.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// A directory link at `link`, leading to `target`.
    #[cfg(unix)]
    fn link_directory(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap_or_else(|error| {
            panic!(
                "cannot link {} to {}: {error}",
                link.display(),
                target.display()
            )
        });
    }

    fn declaration(name: &str, port: u16) -> ServiceDeclaration {
        ServiceDeclaration {
            name: name.to_owned(),
            directory: None,
            launcher: Launcher::NodeEntry {
                entry: "server.js".to_owned(),
            },
            port,
            readiness: "/healthz".to_owned(),
            page: None,
        }
    }

    fn config(declarations: Vec<ServiceDeclaration>) -> ChecksConfig {
        ChecksConfig {
            services: declarations,
            ..ChecksConfig::default()
        }
    }

    /// A builder for a run that may start the project's services.
    fn an_executing_builder() -> PlanBuilder {
        PlanBuilder::new(
            ExecutionMode::HostConfirmed,
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        )
    }

    /// The plan, as the schedule a builder made of it holds.
    fn planned(plan: &ServicePlan) -> Vec<ScheduledCheck> {
        let mut builder = an_executing_builder();
        plan.add_to(&mut builder);
        assert!(
            builder.refused().is_empty(),
            "every check this module builds must survive `PlanBuilder::propose`: {:?}",
            builder.refused()
        );
        builder.build().checks().to_vec()
    }

    fn command_of(scheduled: &ScheduledCheck) -> CommandSpec {
        match scheduled.operation() {
            CheckOperation::Service(service) => service.command().clone(),
            other => panic!("expected a service check, got {other:?}"),
        }
    }

    #[test]
    fn a_declaration_plans_a_node_command_with_the_entry_as_one_argument() {
        let root = project("valid");
        let plan = ServicePlan::of(&root, &config(vec![declaration("api", 4310)]));

        assert_eq!(
            plan.refusals().to_vec(),
            Vec::new(),
            "a complete declaration was refused"
        );
        assert_eq!(
            plan.gaps().to_vec(),
            vec![ServiceGap::AutoDoesNotLookAtAPage {
                name: "api".to_owned()
            }],
            "the default preference is `auto`, which never opens a page: the gap names \
             the setting that decided, and a page-less declaration under `auto` gets the \
             same sentence as one that named a page, because declaring one would not \
             have changed the answer"
        );
        assert_eq!(plan.services().len(), 1);

        let checks = planned(&plan);
        assert_eq!(checks.len(), 1, "one declaration is one check, not none");
        assert_eq!(checks[0].proposal().title(), "start api");
        assert_eq!(
            checks[0].proposal().id(),
            &check_id("api", "service"),
            "the identifier is derived from the declaration's name and its tag, so two \
             runs of this planner name one check the same way"
        );
        assert_eq!(checks[0].proposal().severity(), Severity::MustFix);
        assert!(checks[0].proposal().critical());
        assert_eq!(
            checks[0].proposal().requirements().actions().to_vec(),
            vec![ActionKind::StartService],
            "the service row's action is the one whose permission decides it"
        );

        let command = command_of(&checks[0]);
        assert_eq!(
            command.program(),
            OsStr::new("node"),
            "the program is the name `node` and not a path this machine resolved"
        );
        assert_eq!(
            command.arguments().to_vec(),
            vec![OsString::from("server.js")],
            "the entry is exactly one argument: a string that was split here would be a \
             second argument with a different meaning"
        );
        assert_eq!(
            command.working_directory(),
            root.as_path(),
            "the command runs in the directory the declaration named, which is the root \
             when it named none"
        );
        assert_eq!(
            command.environment(),
            &Environment::only(Vec::new()),
            "a service is not given SURE's own environment by accident"
        );

        let CheckOperation::Service(spec) = checks[0].operation() else {
            panic!("a declaration must plan a service operation");
        };
        let Readiness::Answers { endpoint } = spec.readiness() else {
            panic!("the readiness path is what SURE asks for; `StaysUp` was not declared");
        };
        assert_eq!(endpoint.path(), "/healthz");
        assert_eq!(endpoint.address().port(), 4310);
        assert_eq!(spec.window(), A_SERVICES_WINDOW);
    }

    #[test]
    fn an_entry_with_a_space_stays_one_argument() {
        let root = scratch("space");
        write(&root.join("my server.js"), "// a service\n");
        let mut declaration = declaration("api", 4310);
        declaration.launcher = Launcher::NodeEntry {
            entry: "my server.js".to_owned(),
        };

        let plan = ServicePlan::of(&root, &config(vec![declaration]));
        let checks = planned(&plan);
        let command = command_of(&checks[0]);

        assert_eq!(
            command.arguments().to_vec(),
            vec![OsString::from("my server.js")],
            "a space in an entry is a space in a name, and never a separator: nothing \
             here splits a string into arguments"
        );
    }

    #[test]
    fn a_declared_page_under_always_plans_a_browser_check_on_the_same_service() {
        let root = project("page");
        let mut declaration = declaration("api", 4310);
        declaration.page = Some("/".to_owned());
        let plan = ServicePlan::of(
            &root,
            &ChecksConfig {
                browser_probe: CheckPreference::Always,
                ..config(vec![declaration])
            },
        );

        assert_eq!(
            plan.gaps().to_vec(),
            Vec::new(),
            "`always` with a page plans both rows"
        );
        let checks = planned(&plan);
        assert_eq!(checks.len(), 2, "a declaration with a page is two checks");

        // Found by what they are rather than by where they landed: the plan's own
        // order is `PlanBuilder`'s, and it puts the browser row first because a
        // browser check runs none of the project's code. A test that read
        // `checks[0]` would be testing the schedule's ordering rule.
        let service = checks
            .iter()
            .find_map(|scheduled| match scheduled.operation() {
                CheckOperation::Service(service) => Some(service),
                _ => None,
            })
            .expect("a declaration always plans the check that starts it");
        let browser_row = checks
            .iter()
            .find(|scheduled| matches!(scheduled.operation(), CheckOperation::Browser(_)))
            .expect("`always` with a page plans the check that opens it");
        let CheckOperation::Browser(browser) = browser_row.operation() else {
            panic!("the row was found as a browser check one line above");
        };

        assert_eq!(browser_row.proposal().title(), "check api in a browser");
        assert_eq!(
            browser_row.proposal().severity(),
            Severity::ShouldFixFirst,
            "the browser row is the one a project can hand over without, which is its \
             own row's grading rather than a second opinion"
        );
        assert!(!browser_row.proposal().critical());
        assert_eq!(browser.path(), "/");
        assert_eq!(
            browser.service().readiness(),
            service.readiness(),
            "the browser check must stand on the very service that was planned, not on a \
             second one that could disagree about the port"
        );
        assert_eq!(
            browser.service().command(),
            service.command(),
            "one declaration, one command"
        );
        assert!(browser.expectation().contains("api"));
        assert!(
            browser.expectation().contains("http://127.0.0.1:4310/"),
            "the expectation names the page the browser is sent to: {:?}",
            browser.expectation()
        );
        // What a browser check establishes is that a page arrived, and the
        // sentence has to say so *and* say what it does not cover: a project that
        // read "checked in a browser" could otherwise take it for a claim about
        // its sign-in or its checkout, neither of which is exercised.
        for limit in [
            "Nothing beyond that page is exercised",
            "no account is created",
            "nothing is paid for",
            "no message is sent",
            "nothing is stored",
        ] {
            assert!(
                browser.expectation().contains(limit),
                "the expectation must state its own limit, and {limit:?} is missing from \
                 {:?}",
                browser.expectation()
            );
        }
    }

    #[test]
    fn auto_opens_no_page_and_never_plans_nothing() {
        let root = project("preferences");
        let mut with_page = declaration("api", 4310);
        with_page.page = Some("/".to_owned());

        // `auto` is the default, and the interface row's own row does not plan
        // itself — the same rule a discovered component gets.
        let auto = ServicePlan::of(&root, &config(vec![with_page.clone()]));
        assert_eq!(
            auto.gaps().to_vec(),
            vec![ServiceGap::AutoDoesNotLookAtAPage {
                name: "api".to_owned()
            }],
            "`auto` with a page records the gap rather than opening one"
        );
        assert_eq!(planned(&auto).len(), 1, "the service row is still planned");

        // `never` on the service switches off both rows, because a browser check
        // on a service this project does not start would start it.
        let never = ServicePlan::of(
            &root,
            &ChecksConfig {
                start_local_services: CheckPreference::Never,
                browser_probe: CheckPreference::Always,
                ..config(vec![with_page.clone()])
            },
        );
        assert_eq!(
            never.gaps().to_vec(),
            vec![ServiceGap::LocalServicesDisabled {
                name: "api".to_owned()
            }],
            "the setting that decided is named once, and the browser setting is not \
             consulted for a service that will not be started"
        );
        assert!(never.services().is_empty());
        assert!(planned(&never).is_empty());

        // A project that declared nothing is empty, and says so rather than
        // reporting a gap for a service nobody wrote down.
        let none = ServicePlan::of(&root, &config(Vec::new()));
        assert!(none.is_empty());
        assert!(none.plain_description().is_empty());
    }

    #[test]
    fn always_with_no_page_records_that_rather_than_inventing_a_target() {
        let root = project("no page");
        let plan = ServicePlan::of(
            &root,
            &ChecksConfig {
                browser_probe: CheckPreference::Always,
                ..config(vec![declaration("api", 4310)])
            },
        );

        assert_eq!(
            plan.gaps().to_vec(),
            vec![ServiceGap::NoPageToLookAt {
                name: "api".to_owned()
            }],
            "a browser check needs a page, and a declaration that named none has none"
        );
        assert_eq!(planned(&plan).len(), 1);
    }

    /// A directory outside the project, for the link fixtures to lead to.
    ///
    /// **A directory of its own and not a path under the project**, because the
    /// fixture has to be one the planner should refuse to reach: a link that led
    /// to somewhere inside the project would be a link this module is right to
    /// allow, and a test built on one would pass with [`resolves_inside`]
    /// deleted.
    fn outside_project(name: &str) -> PathBuf {
        let outside = scratch(name);
        write(&outside.join("server.js"), "// somebody else's service\n");
        outside
    }

    #[test]
    fn a_declared_directory_that_is_a_link_out_of_the_project_is_refused() {
        // The case `stays_inside` cannot see: `escape` is a plain relative path
        // of one ordinary component, so every textual test in this module accepts
        // it, and it is the *working directory* of the command SURE would build —
        // which would be a program started outside the tree SURE was handed.
        let root = project("linked directory");
        link_directory(
            &outside_project("linked directory outside"),
            &root.join("escape"),
        );

        let mut declaration = declaration("api", 4310);
        declaration.directory = Some("escape".to_owned());
        let plan = ServicePlan::of(&root, &config(vec![declaration]));

        assert_eq!(
            plan.refusals().to_vec(),
            vec![ServiceRefusal::DirectoryOutsideTheProject {
                directory: "escape".to_owned(),
            }],
        );
        assert!(
            plan.services().is_empty() && planned(&plan).is_empty(),
            "a directory SURE will not run in is not a service SURE starts"
        );
    }

    #[test]
    fn a_declared_entry_reached_through_a_link_is_refused() {
        // The same link, one component deeper: the directory is the project root
        // itself, the entry is a file that is there, and the only thing wrong
        // with it is where it *is*.
        let root = project("linked entry");
        link_directory(
            &outside_project("linked entry outside"),
            &root.join("escape"),
        );

        let mut declaration = declaration("api", 4310);
        declaration.launcher = Launcher::NodeEntry {
            entry: "escape/server.js".to_owned(),
        };
        let plan = ServicePlan::of(&root, &config(vec![declaration]));

        assert_eq!(
            plan.refusals().to_vec(),
            vec![ServiceRefusal::EntryIsOutsideTheProject {
                entry: "escape/server.js".to_owned(),
            }],
        );
        assert!(
            plan.services().is_empty() && planned(&plan).is_empty(),
            "a file SURE will not start is not a service SURE starts"
        );
    }

    #[test]
    fn every_way_a_declaration_can_be_wrong_is_a_refusal_with_a_sentence() {
        let root = project("refusals");
        std::fs::create_dir_all(root.join("web")).expect("a member directory");
        write(&root.join("web").join("main.js"), "// a member service\n");

        let mut outside = declaration("outside", 4310);
        outside.directory = Some("../outside".to_owned());

        let mut missing_directory = declaration("gone", 4310);
        missing_directory.directory = Some("web/missing".to_owned());

        let mut outside_entry = declaration("escape", 4310);
        outside_entry.launcher = Launcher::NodeEntry {
            entry: "../outside.js".to_owned(),
        };

        let mut shell = declaration("shell", 4310);
        shell.launcher = Launcher::NodeEntry {
            entry: "server.sh".to_owned(),
        };

        let mut absent = declaration("absent", 4310);
        absent.launcher = Launcher::NodeEntry {
            entry: "missing.js".to_owned(),
        };

        let zero = declaration("zero", 0);

        let mut unsafe_readiness = declaration("unsafe", 4310);
        unsafe_readiness.readiness = "healthz".to_owned();

        let mut unsafe_page = declaration("unsafe page", 4310);
        unsafe_page.page = Some("\\rooted".to_owned());

        let expected = [
            (declaration("", 4310), ServiceRefusal::NameIsEmpty),
            (
                declaration("api", 4310),
                ServiceRefusal::NameAppearsTwice {
                    name: "api".to_owned(),
                },
            ),
            (
                outside,
                ServiceRefusal::DirectoryOutsideTheProject {
                    directory: "../outside".to_owned(),
                },
            ),
            (
                missing_directory,
                ServiceRefusal::DirectoryIsNotThere {
                    directory: "web/missing".to_owned(),
                },
            ),
            (
                outside_entry,
                ServiceRefusal::EntryIsOutsideTheProject {
                    entry: "../outside.js".to_owned(),
                },
            ),
            (
                shell,
                ServiceRefusal::EntryIsNotAJavaScriptFile {
                    entry: "server.sh".to_owned(),
                },
            ),
            (
                absent,
                ServiceRefusal::EntryIsNotThere {
                    entry: "missing.js".to_owned(),
                },
            ),
            (
                zero,
                ServiceRefusal::PortIsZero {
                    name: "zero".to_owned(),
                },
            ),
        ];

        // One pass per declaration, so that the refusals under test are the
        // answers to *one* wrong field rather than to a file full of them, and
        // so that the duplicate is a duplicate of a declaration that is
        // otherwise valid — which is the case a reader actually writes.
        for (index, (declaration, refusal)) in expected.iter().enumerate() {
            let duplicated = matches!(refusal, ServiceRefusal::NameAppearsTwice { .. });
            let mut declarations = vec![declaration.clone()];
            if duplicated {
                declarations.insert(0, declaration.clone());
            }
            let plan = ServicePlan::of(&root, &config(declarations));

            assert_eq!(
                plan.refusals(),
                std::slice::from_ref(refusal),
                "declaration {index} ({declaration:?}) was refused for the wrong reason"
            );
            if duplicated {
                assert_eq!(
                    plan.services().len(),
                    1,
                    "the first declaration of a name is planned and the second is \
                     refused, rather than both or neither"
                );
            } else {
                assert!(
                    plan.services().is_empty(),
                    "a refused declaration is not also a planned check"
                );
            }
            let sentence = plan.refusals()[0].plain_description();
            assert!(
                !sentence.trim().is_empty() && sentence.contains("."),
                "a refusal is a sentence a user can act on: {sentence:?}"
            );
        }

        // The two paths that go through `probe::Endpoint`, which is the refusal
        // this module reuses rather than re-implements.
        for (declaration, path) in [(unsafe_readiness, "healthz"), (unsafe_page, "\\rooted")] {
            let name = declaration.name.clone();
            let plan = ServicePlan::of(&root, &config(vec![declaration]));
            match plan.refusals() {
                [
                    ServiceRefusal::UnsafePath {
                        name: refused,
                        path: refused_path,
                        error,
                    },
                ] => {
                    assert_eq!(refused, &name, "the refusal names the declaration");
                    assert_eq!(refused_path, path);
                    assert_eq!(
                        *error,
                        EndpointError::UnsafePath {
                            path: path.to_owned()
                        }
                    );
                }
                other => panic!("expected one `UnsafePath` refusal for {path:?}, got {other:?}"),
            }
            assert!(
                plan.refusals()[0].plain_description().contains(&name),
                "the sentence names the declaration a reader has to go and fix"
            );
        }
    }

    #[test]
    fn a_name_that_is_only_whitespace_is_an_empty_name() {
        let root = project("whitespace");
        let plan = ServicePlan::of(&root, &config(vec![declaration("   ", 4310)]));

        assert_eq!(
            plan.refusals().to_vec(),
            vec![ServiceRefusal::NameIsEmpty],
            "a name a reader cannot see is not a name, and the identifier derived from it \
             would be one nobody could find"
        );
    }

    #[test]
    fn two_declarations_are_two_identifiers_and_the_same_two_on_every_reading() {
        let root = project("identifiers");
        let declarations = vec![declaration("api", 4310), declaration("worker", 4311)];

        let first = ServicePlan::of(&root, &config(declarations.clone()));
        let second = ServicePlan::of(&root, &config(declarations));

        let ids: Vec<CheckId> = first
            .services()
            .iter()
            .map(|service| service.service().id().clone())
            .collect();
        assert_ne!(
            ids[0], ids[1],
            "two names are two checks, not one run twice"
        );
        assert_eq!(
            first, second,
            "the same declarations and the same files are the same plan"
        );
        assert_eq!(
            ids,
            second
                .services()
                .iter()
                .map(|service| service.service().id().clone())
                .collect::<Vec<_>>(),
            "an identifier that changed between two readings of one project would be two \
             checks a report could not join"
        );
    }

    #[test]
    fn the_service_row_needs_the_permission_to_start_a_service_and_nothing_less() {
        // The check's requirements are `ActionKind::StartService`'s, and the two
        // answers below are the domain's rather than this module's: a declaration
        // asks for the check, and the mode and the user's permissions decide
        // whether it runs.
        let root = project("admission");
        let plan = ServicePlan::of(&root, &config(vec![declaration("api", 4310)]));
        let id = plan.services()[0].service().id().clone();

        let schedule_under = |mode, permissions| {
            let mut builder = PlanBuilder::new(mode, permissions);
            plan.add_to(&mut builder);
            builder.build()
        };

        let inspecting = schedule_under(
            ExecutionMode::InspectOnly,
            ExecutionPermissions::inspect_only(),
        );
        assert_eq!(inspecting.len(), 1, "the check is planned, not dropped");
        let blocked = &inspecting.checks()[0];
        assert_eq!(blocked.decision(), ExecutionDecision::Denied);
        assert_eq!(
            blocked.blocked_by(),
            Some(Permission::RunProjectCode),
            "the entry names the one permission that would change the answer"
        );

        let mut permission_plan = PermissionPlan::new(
            ExecutionMode::InspectOnly,
            FingerprintId::generate(),
            ExecutionPermissions::inspect_only(),
        );
        let scheduled = inspecting.planned_checks();
        let CheckOperation::Service(spec) = inspecting.checks()[0].operation() else {
            panic!("a declaration plans a service operation");
        };
        permission_plan.add(
            scheduled[0].clone(),
            spec.command().program(),
            spec.command().arguments().to_vec(),
        );
        let stopped = Enforcement::of("service-plan-inspecting", permission_plan, &scheduled);
        assert!(
            stopped.admitted().next().is_none(),
            "a mode that runs nothing admitted the command that starts a project's service"
        );

        let executing = schedule_under(
            ExecutionMode::HostConfirmed,
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        let scheduled = executing.planned_checks();
        let mut permission_plan = PermissionPlan::new(
            ExecutionMode::HostConfirmed,
            FingerprintId::generate(),
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        let CheckOperation::Service(spec) = executing.checks()[0].operation() else {
            panic!("a declaration plans a service operation");
        };
        permission_plan.add(
            scheduled[0].clone(),
            spec.command().program(),
            spec.command().arguments().to_vec(),
        );
        let admitted = Enforcement::of("service-plan-executing", permission_plan, &scheduled);
        assert!(
            admitted.stopped().is_empty(),
            "a granted run stopped a check it should have admitted: {:?}",
            admitted.stopped()
        );
        assert_eq!(
            admitted.admitted().count(),
            1,
            "the command that starts the declared service is the one command admitted"
        );
        assert_eq!(
            admitted
                .admitted()
                .next()
                .map(|command| command.command().check().id().clone()),
            Some(id),
            "the admitted command is the declared service's own check"
        );
    }
}

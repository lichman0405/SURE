//! Executable work, typed at the moment a check is proposed.
//!
//! # Why this module exists
//!
//! The plan has always said *what* a run intends to check and never *what
//! running one is*. `checks::node::NodeChecks::of` asks its discovery for a
//! rendered line — `npm run build` — and files it in
//! [`CheckReason::DeclaredCommand`](crate::schedule::CheckReason), which is
//! display text: it is what a report prints. From that point the only
//! description of the work is the sentence a person reads, and the shortest
//! way to execute it — split the string, start the pieces — is the act
//! `docs/architecture/EXECUTION_SAFETY.md` and the repository's invariants
//! forbid. A string that a report prints must never become the reason a
//! program runs.
//!
//! So the work is a value here, and it is a value **before** anything decides
//! whether it may run: [`PlannedWork`] holds a check's proposal and the
//! operation that would carry it out, and it is built where the check is
//! proposed. A caller that has one of them cannot have the other missing, and
//! that is a fact about a type rather than a rule for a reader to remember.
//!
//! # A command is named, never spelled
//!
//! [`CommandSpec`] carries a program, an argument vector, a working directory,
//! an environment policy, a deadline and an output bound as **named fields**.
//! There is no variant that takes a string to be split and no function in this
//! module that splits one. `process::ProcessRequest` has the same rule one
//! layer down; this is that rule carried back to where the work is first known,
//! so that the layer which knows a `package.json` declared `build` is also the
//! layer that decides the program is `npm` and the argument is `run build`.
//!
//! # A name with no extension is not a program on Windows
//!
//! [`crate::process`] records the measurement and this module inherits the
//! consequence. `CreateProcess` completes a name that has **no** extension with
//! `.exe` and nothing else, so a bare `npm` is not found on a machine where
//! `npm.cmd` is on `PATH`; and a `.cmd` named *with* its extension does start,
//! by starting a command interpreter to run the text inside it. A plan that
//! wrote `npm` would plan a command that cannot start and would report a spawn
//! failure where the truth is that this build will not run a batch file. A plan
//! that wrote `npm run build` would be the thing this module exists to prevent.
//!
//! [`ProgramPath`] is therefore a value with a question — *what is this name,
//! on this machine?* — and [`Resolution`] is its answer, in four parts that are
//! four different sentences in a report: an executable this build starts, a file
//! whose start would be an interpreter's, a file that is not a program, and
//! nothing at all. Which of the four came back is decided by the files on disk
//! and by no string formatting anywhere in this crate.
//!
//! # What this module does not decide
//!
//! **It does not decide whether the work may run.** Nothing here consults an
//! [`ExecutionMode`](sure_domain::execution::ExecutionMode), a permission or a
//! consent, and nothing here can: `crate::enforce` is the only thing that
//! answers that question and [`crate::enforce::AdmittedCommand`] is the only
//! value that carries a *yes*. A `CommandSpec` is not a launch and cannot be
//! made into one — `crate::process` and `crate::service` take what they start
//! from the enforcement, and this type is what the enforcement was asked about.
//!
//! **It does not run anything.** There is no spawn, no thread and no I/O in
//! this file. `tests/spawn_sites.rs` counts the places that build a `Command`
//! and this module is not one of them, which is what makes it safe for the
//! discovery layer to hold.
//!
//! **It does not make a resolved path trustworthy.** A name found on `PATH` is
//! a file on this machine that something else could replace between the plan and
//! the run. Resolving early is about saying the *honest* thing — "there is an
//! `npm.cmd` here and this build will not start it" rather than "there is no
//! `npm`" — and not about pinning a program down.
//!
//! **It does not search the current directory, and Windows would.** An empty
//! element of `PATH` means *the current directory* to `CreateProcess`, so a
//! project that writes `npm.exe` into its own folder is a project whose build
//! check resolves to its own file. [`ProgramPath::directories`] drops empty
//! entries for exactly that reason. The cost is real and is stated rather than
//! hidden: on a machine whose `PATH` ends in a separator, SURE would not find a
//! program that a shell in the same directory would — and a plan that names the
//! wrong program is a worse failure than a plan that finds none, because the
//! first one is a permission question asked about something the project chose.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sure_domain::ids::FingerprintId;
use sure_domain::status::CheckResult;

use crate::process::{Environment, Limits};
use crate::schedule::CheckProposal;

/// One check, and the work that would carry it out if it runs.
///
/// The two halves are one value because they are one decision. A check whose
/// operation could not be produced is not a check with a missing field — it is a
/// check SURE cannot carry out, and it belongs in the plan as a declaration it
/// could not plan rather than as a proposal whose work something else is
/// expected to remember. [`crate::schedule::PlanBuilder::propose`] takes this
/// type and not a [`CheckProposal`], so that gap cannot be opened by forgetting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedWork {
    proposal: CheckProposal,
    operation: CheckOperation,
}

impl PlannedWork {
    /// Bind a check to the work that would carry it out.
    #[must_use]
    pub const fn new(proposal: CheckProposal, operation: CheckOperation) -> Self {
        Self {
            proposal,
            operation,
        }
    }

    /// The check, as a report reads it.
    #[must_use]
    pub const fn proposal(&self) -> &CheckProposal {
        &self.proposal
    }

    /// What running the check means.
    #[must_use]
    pub const fn operation(&self) -> &CheckOperation {
        &self.operation
    }
}

/// What running a check is.
///
/// Four kinds, and the list is closed on purpose: a fifth kind of work is a
/// fifth kind of question about the host, and this build answers four. Every
/// variant is data — nothing in this enum starts anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOperation {
    /// The answer is already known. A detector read the project while the plan
    /// was being made, and there is no process to start because there is
    /// nothing left to ask.
    Precomputed(PrecomputedEvidence),
    /// Run one command, once, under a deadline, and read what it said.
    Command(CommandSpec),
    /// Start a program that is meant to stay up, ask it one question on
    /// loopback, and stop it.
    Service(ServiceCheckSpec),
    /// Start a service and look at a page it serves, in a browser this machine
    /// already has.
    Browser(BrowserCheckSpec),
}

impl CheckOperation {
    /// Whether carrying this out starts a process at all.
    ///
    /// [`CheckOperation::Precomputed`] is the only `false`, and it is the one
    /// that matters for the invariant the census in `tests/spawn_sites.rs`
    /// protects: a plan made entirely of precomputed work reaches no runner and
    /// no process, whatever the mode says.
    #[must_use]
    pub const fn starts_a_process(&self) -> bool {
        !matches!(self, Self::Precomputed(_))
    }

    /// The sentence a report shows for this kind of work.
    #[must_use]
    pub const fn plain_description(&self) -> &'static str {
        match self {
            Self::Precomputed(_) => "already observed while the plan was made",
            Self::Command(_) => "one command, once, under a deadline",
            Self::Service(_) => "started, asked one question, and stopped",
            Self::Browser(_) => "a page served on loopback, read by a browser",
        }
    }
}

/// What a detector already found, before any process was considered.
///
/// **This is evidence and not a verdict**, which is why the four answers are the
/// four things a detector can honestly know and not the six statuses a check can
/// have. The mapping to a status belongs to [`Self::to_result`], where it can be
/// read in one place, and the arrow always points the same way: a detector that
/// could not read what it needed says so, and never that the property holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticObservation {
    /// The detector read the project and the property it looks for is there.
    Holds,
    /// The detector read the project and found the property contradicted.
    ///
    /// A contradiction is the only shape in which a detector is allowed to say
    /// the project is wrong, because it is the only one where the detector has
    /// both the rule and the thing that breaks it.
    Contradicted,
    /// The detector read the project and found something it cannot settle — a
    /// candidate, a partial reading, a file too large to finish.
    Candidate,
    /// The detector could not read what it needed: a file that was not there, a
    /// parse that failed, a query the project did not answer.
    CouldNotRun,
}

/// An observation a detector made, and the sentence a person reads about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecomputedEvidence {
    observation: StaticObservation,
    detail: String,
}

impl PrecomputedEvidence {
    /// An observation, with the detail line that says what was read.
    #[must_use]
    pub fn new(observation: StaticObservation, detail: impl Into<String>) -> Self {
        Self {
            observation,
            detail: detail.into(),
        }
    }

    /// The detector read the project and found what it was looking for.
    #[must_use]
    pub fn holds(detail: impl Into<String>) -> Self {
        Self::new(StaticObservation::Holds, detail)
    }

    /// The detector read the project and found the property contradicted.
    #[must_use]
    pub fn contradicted(detail: impl Into<String>) -> Self {
        Self::new(StaticObservation::Contradicted, detail)
    }

    /// The detector found a candidate and cannot settle it.
    #[must_use]
    pub fn candidate(detail: impl Into<String>) -> Self {
        Self::new(StaticObservation::Candidate, detail)
    }

    /// The detector could not read what it needed.
    #[must_use]
    pub fn could_not_run(detail: impl Into<String>) -> Self {
        Self::new(StaticObservation::CouldNotRun, detail)
    }

    /// What the detector observed.
    #[must_use]
    pub const fn observation(&self) -> StaticObservation {
        self.observation
    }

    /// The detail line.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// This observation as the result of the check it belongs to.
    ///
    /// The check's severity, evidence class and identity come from its
    /// proposal, so an observation cannot carry a weight its check did not
    /// claim. The fingerprint is a parameter for the same reason it is a
    /// parameter everywhere else in this repository: a result is evidence
    /// **about a project state**, and a result that did not name one would be
    /// evidence about a state nobody can point at.
    ///
    /// A candidate is a [`CheckStatus::Warning`](sure_domain::status::CheckStatus)
    /// and not an `Unknown`, because the detector did observe something and the
    /// difference between "here is a thing worth looking at" and "SURE has no
    /// evidence" is a difference a reader acts on. A detector that could not run
    /// is an `Error`, which for a critical check is not a pass.
    #[must_use]
    pub fn to_result(&self, proposal: &CheckProposal, fingerprint: &FingerprintId) -> CheckResult {
        let (id, title) = (proposal.id().clone(), proposal.title().to_owned());
        let (severity, critical) = (proposal.severity(), proposal.critical());
        let class = proposal.evidence_class();
        match self.observation {
            StaticObservation::Holds => {
                CheckResult::pass(id, title, severity, critical, class, fingerprint.clone())
                    .with_reason(self.detail.clone())
            }
            StaticObservation::Contradicted => {
                CheckResult::fail(id, title, severity, critical, class, fingerprint.clone())
                    .with_reason(self.detail.clone())
            }
            StaticObservation::Candidate => {
                CheckResult::warning(id, title, severity, critical, class, fingerprint.clone())
                    .with_reason(self.detail.clone())
            }
            StaticObservation::CouldNotRun => CheckResult::errored(
                id,
                title,
                severity,
                critical,
                self.detail.clone(),
                fingerprint.clone(),
            ),
        }
    }
}

/// One command: a program, its arguments, and the rules it runs under.
///
/// Every field is named and none of them is a string to be interpreted. An
/// argument holding a space is one argument and stays one argument; on Windows
/// the operating system is handed the vector it is given, and there is no
/// quoting step for SURE to get wrong or right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
    environment: Environment,
    limits: Limits,
}

impl CommandSpec {
    /// A command, with the rules it will run under.
    ///
    /// `working_directory` must be absolute: `process::ProcessRequest` refuses
    /// anything else, and it refuses it there rather than here so that there is
    /// one place where that rule is enforced rather than two that can disagree.
    #[must_use]
    pub fn new(
        program: impl Into<OsString>,
        working_directory: impl Into<PathBuf>,
        environment: Environment,
        limits: Limits,
    ) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
            working_directory: working_directory.into(),
            environment,
            limits,
        }
    }

    /// The arguments, in the order the program will receive them.
    #[must_use]
    pub fn with_arguments<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments = arguments.into_iter().map(Into::into).collect();
        self
    }

    /// The program, as the plan decided to name it.
    #[must_use]
    pub fn program(&self) -> &OsStr {
        &self.program
    }

    /// The arguments, as the plan decided them.
    #[must_use]
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    /// The directory the command runs in, absolute.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// What the command is given to start with.
    #[must_use]
    pub const fn environment(&self) -> &Environment {
        &self.environment
    }

    /// The deadline and the output bound.
    #[must_use]
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    // There is deliberately no method here that turns a spec into the runner's
    // request type. Doing that is the moment this module names
    // `process::ProcessRequest` in code rather than in prose, and
    // `tests/spawn_sites.rs` reads that name as the statement "this file can
    // call the runner". No file has a caller for it yet — the runner is
    // `planned_check_runner`, and it adds that door where the census can see
    // who wanted it and whether the support ceiling has to move with it.
}

/// A service: a program that is meant to stay up, and the one question to ask it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCheckSpec {
    command: CommandSpec,
    readiness: Readiness,
    window: Duration,
}

impl ServiceCheckSpec {
    /// A service, what will be asked of it, and how long it is given.
    #[must_use]
    pub const fn new(command: CommandSpec, readiness: Readiness, window: Duration) -> Self {
        Self {
            command,
            readiness,
            window,
        }
    }

    /// The command that starts it.
    #[must_use]
    pub const fn command(&self) -> &CommandSpec {
        &self.command
    }

    /// What counts as ready.
    #[must_use]
    pub const fn readiness(&self) -> &Readiness {
        &self.readiness
    }

    /// How long the service is given to become ready, and how long it is kept.
    #[must_use]
    pub const fn window(&self) -> Duration {
        self.window
    }
}

/// The address a service check can ask about.
///
/// **There is no host field, and that is the invariant.** A service check asks a
/// question of a project's own process on the machine it was started on, so the
/// host is loopback and is not a decision anyone gets to make; a variant that
/// carried one would be a variant that could carry `example.com`, and the
/// repository's "no silent external network validation" rule would then rest on
/// every caller rather than on this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    /// Nothing is asked of it beyond staying up for the window.
    ///
    /// The weakest of the four and named so that it cannot be mistaken for a
    /// readiness: a service that stayed up and answered nothing is not a service
    /// SURE has confirmed anything about, and the result says exactly that.
    StaysUp,
    /// It answers on this loopback port.
    Answers {
        /// The port.
        port: u16,
        /// The path asked for, beginning with `/`.
        path: String,
    },
}

impl Readiness {
    /// The address a probe of this readiness would be sent to, if any.
    ///
    /// `127.0.0.1` by name rather than `localhost`, because `localhost` is a
    /// name this machine resolves and a name can be pointed elsewhere.
    #[must_use]
    pub fn loopback_url(&self) -> Option<String> {
        match self {
            Self::StaysUp => None,
            Self::Answers { port, path } => Some(format!("http://127.0.0.1:{port}{path}")),
        }
    }
}

/// A browser check: a service, and the page it is expected to serve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserCheckSpec {
    service: ServiceCheckSpec,
    path: String,
    expectation: String,
}

impl BrowserCheckSpec {
    /// A page to read, on the service this check starts.
    ///
    /// The host is not a parameter for the same reason it is not one in
    /// [`Readiness`]: the page is the supervised service's own, on loopback. It
    /// is passed to the browser as a URL because a browser takes one, and the
    /// URL is built by [`Self::loopback_url`] rather than assembled by a caller.
    #[must_use]
    pub fn new(
        service: ServiceCheckSpec,
        path: impl Into<String>,
        expectation: impl Into<String>,
    ) -> Self {
        Self {
            service,
            path: path.into(),
            expectation: expectation.into(),
        }
    }

    /// The service this check starts and stops.
    #[must_use]
    pub const fn service(&self) -> &ServiceCheckSpec {
        &self.service
    }

    /// The path on it the browser opens.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// What the observation is for.
    #[must_use]
    pub fn expectation(&self) -> &str {
        &self.expectation
    }

    /// The loopback URL the browser is sent to, or `None` if the service's own
    /// readiness does not name a port to ask.
    #[must_use]
    pub fn loopback_url(&self) -> Option<String> {
        match self.service.readiness() {
            Readiness::StaysUp => None,
            Readiness::Answers { port, .. } => {
                Some(format!("http://127.0.0.1:{port}{}", self.path))
            }
        }
    }
}

/// What a name on `PATH` turns out to be, on the machine that asked.
///
/// Four answers rather than two, because "not found" and "found, and this build
/// will not start it" are different sentences in a report and a user acts on
/// them differently: the first is a tool that is not installed, the second is a
/// tool that is installed and is a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A file this build starts directly.
    Executable(PathBuf),
    /// A file of that name is there, and starting it starts an interpreter:
    /// a `.cmd`, `.bat` or `.ps1` on Windows. This build does not start one, and
    /// `crate::safety` records why a batch file's name is not evidence of what
    /// running it does.
    InterpreterRequired(PathBuf),
    /// A file of that name is there and is not something an operating system
    /// starts as a program.
    NotAProgram(PathBuf),
    /// Nothing of that name is on this path.
    Absent,
}

impl Resolution {
    /// The file that was found, if one was.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Executable(path) | Self::InterpreterRequired(path) | Self::NotAProgram(path) => {
                Some(path)
            }
            Self::Absent => None,
        }
    }

    /// Whether this build would start what was found.
    #[must_use]
    pub const fn is_startable(&self) -> bool {
        matches!(self, Self::Executable(_))
    }

    /// What the plan should name as the program, if it names one.
    ///
    /// The **found** name and not the name that was asked for. That is the whole
    /// point of resolving at all: a plan that wrote `npm` would name a program
    /// Windows cannot start, and the failure would arrive at run time as a spawn
    /// error rather than at plan time as the truth — there is an `npm.cmd` here,
    /// and this build will not run it.
    #[must_use]
    pub fn program_name(&self) -> Option<&Path> {
        self.path()
    }

    /// One line for a report.
    #[must_use]
    pub fn plain_description(&self) -> String {
        match self {
            Self::Executable(path) => {
                format!("{} is a program this build can start", path.display())
            }
            Self::InterpreterRequired(path) => format!(
                "{} is there and starting it starts an interpreter, so this build does not start it",
                path.display()
            ),
            Self::NotAProgram(path) => format!(
                "{} is there and is not a program this build starts",
                path.display()
            ),
            Self::Absent => "nothing of that name is on PATH".to_owned(),
        }
    }
}

/// The directories a program name is looked for in.
///
/// A value rather than a call to [`std::env::var_os`] buried in a builder,
/// because a plan that depends on the machine's `PATH` is a plan whose result
/// depends on the machine — and a test that wants to ask *what would this plan
/// say on a machine where only `npm.cmd` is installed* cannot ask it if the
/// only `PATH` available is the one the test is running under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramPath {
    search: OsString,
}

impl ProgramPath {
    /// A search path, as the platform spells one.
    #[must_use]
    pub fn from_search_path(search: impl Into<OsString>) -> Self {
        Self {
            search: search.into(),
        }
    }

    /// This machine's `PATH`.
    ///
    /// An empty string when the variable is not set, which makes every name
    /// [`Resolution::Absent`] rather than an error: a machine with no `PATH` is
    /// a machine where nothing is found by name, and saying so is the honest
    /// answer rather than a panic.
    #[must_use]
    pub fn of_this_machine() -> Self {
        Self::from_search_path(std::env::var_os("PATH").unwrap_or_default())
    }

    /// The directories, in the order the platform lists them.
    ///
    /// **Empty entries are dropped, and that is a decision rather than
    /// tidiness.** On Windows an empty element of `PATH` means *the current
    /// directory*, and a current directory that is searched for a program name
    /// is a program name that can be answered by a file the project just wrote:
    /// drop `npm.exe` into a project, run SURE in it, and the plan resolves to
    /// the project's own file. SURE resolves a name to decide what it is about
    /// to ask permission for, so an answer the project could place on disk is
    /// the one answer that must not be given. `Command::new` on Windows would
    /// search it, and this module deliberately does not do what the operating
    /// system would do here — the difference is stated in the module comment so
    /// that it does not read as an oversight.
    pub fn directories(&self) -> impl Iterator<Item = PathBuf> + '_ {
        std::env::split_paths(&self.search).filter(|directory| !directory.as_os_str().is_empty())
    }

    /// What `name` is, on this path, completed the way this platform completes
    /// a bare name.
    #[must_use]
    pub fn resolve(&self, name: &OsStr) -> Resolution {
        self.resolve_with(name, completions())
    }

    /// [`Self::resolve`] against a table of completions handed in.
    ///
    /// **The table is a parameter so that the platform this build is not running
    /// on can be tested on it.** The defect this split was made for is the one
    /// `completions` records: a table that was wrong on macOS and Linux and
    /// right on Windows, where every test that would have read the wrong half
    /// was `#[cfg(windows)]` — so the bug sat in precisely the place no test on
    /// this machine could reach, and the fix for it would have been just as
    /// unreachable as the bug. An argument turns "wrong on the other platform"
    /// from a fact about the build machine into a case, and a case can fail
    /// here.
    fn resolve_with(&self, name: &OsStr, completions: &[(&str, Completion)]) -> Resolution {
        let named = Path::new(name);
        let spelled_out = named.extension().is_some();
        for directory in self.directories() {
            if spelled_out {
                // A caller that named an extension gets that name and no other.
                // Completing `thing.exe` with a second extension would be SURE
                // answering a question nobody asked.
                let candidate = directory.join(named);
                if candidate.is_file() {
                    return classify(&candidate);
                }
                continue;
            }
            for (extension, kind) in completions {
                let candidate = if extension.is_empty() {
                    // The platform completes a bare name with nothing, so the
                    // candidate is the name itself. This is a branch rather
                    // than a `format!` so that the joined spelling is never
                    // produced at all: `{name}.` is a different file name from
                    // `{name}`, and looking for it would report a program that
                    // is installed as one that is not.
                    directory.join(named)
                } else {
                    directory.join(format!(
                        "{}.{extension}",
                        named.as_os_str().to_string_lossy()
                    ))
                };
                if candidate.is_file() {
                    return match kind {
                        Completion::Executable => Resolution::Executable(candidate),
                        Completion::Interpreter => Resolution::InterpreterRequired(candidate),
                    };
                }
            }
        }
        Resolution::Absent
    }
}

/// One extension a name with none of its own is completed with.
enum Completion {
    /// An image this build starts directly.
    Executable,
    /// A file whose start is an interpreter's.
    Interpreter,
}

/// The completions, in the order the operating system would try them.
///
/// Windows' own `PATHEXT` order is `.COM;.EXE;.BAT;.CMD`, and the two script
/// extensions are kept in that order here so that a machine with both would be
/// reported as it would behave. `.PS1` is **not** in `PATHEXT` — Windows would
/// never start `foo.ps1` by that name — and it is listed last so that a machine
/// with both an `.exe` and a `.ps1` reports the `.exe`, and one with only the
/// `.ps1` says there is a script there rather than saying there is nothing.
#[cfg(windows)]
const fn completions() -> &'static [(&'static str, Completion)] {
    &[
        ("com", Completion::Executable),
        ("exe", Completion::Executable),
        ("bat", Completion::Interpreter),
        ("cmd", Completion::Interpreter),
        ("ps1", Completion::Interpreter),
    ]
}

/// On a platform without `PATHEXT` there is one completion and it is the name.
///
/// The empty suffix is the completion, not a wildcard: `execvp` starts the file
/// whose name is the one it was handed, so `cargo` on `PATH` is a file called
/// `cargo`. **An empty list here was wrong, and wrong in the direction this
/// repository treats as serious.** It meant a bare name could only ever answer
/// [`Resolution::Absent`] on macOS and Linux — every installed program reported
/// as one that is not there, on the two platforms this build is required to stay
/// portable to, and only on those, because the Windows list below is not empty.
/// Nothing caught it while P18-T002 stood: the table of completions had no test
/// of its own off Windows, and the tests that would have read it were
/// `#[cfg(windows)]` because the *interesting* cases are Windows cases. The
/// predicate was right on the machine it was written on and false on the two it
/// was written for, which is the shape this repository keeps finding.
#[cfg(any(not(windows), test))]
const WITHOUT_PATHEXT: &[(&str, Completion)] = &[("", Completion::Executable)];

#[cfg(not(windows))]
const fn completions() -> &'static [(&'static str, Completion)] {
    WITHOUT_PATHEXT
}

/// What a file that was found is.
#[cfg(windows)]
fn classify(path: &Path) -> Resolution {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
    match extension.as_deref() {
        Some("com" | "exe") => Resolution::Executable(path.to_path_buf()),
        Some("bat" | "cmd" | "ps1") => Resolution::InterpreterRequired(path.to_path_buf()),
        _ => Resolution::NotAProgram(path.to_path_buf()),
    }
}

/// On a platform that starts a file by its own rules, a file that is there is a
/// program this build will start: whether it runs is the operating system's
/// question, and this build does not pretend to answer it.
#[cfg(not(windows))]
fn classify(path: &Path) -> Resolution {
    if path.is_file() {
        Resolution::Executable(path.to_path_buf())
    } else {
        Resolution::NotAProgram(path.to_path_buf())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::process::Limits;
    use crate::schedule::CheckReason;
    use sure_domain::evidence::EvidenceClass;
    use sure_domain::ids::CheckId;
    use sure_domain::severity::Severity;

    fn a_proposal() -> CheckProposal {
        CheckProposal::new(
            CheckId::generate(),
            "the project declares a test script",
            Severity::ShouldFixFirst,
            true,
            EvidenceClass::ObservedFact,
            CheckReason::ProjectWide,
            &[sure_domain::execution::ActionKind::ReadFile],
        )
    }

    fn a_fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    fn a_spec() -> CommandSpec {
        CommandSpec::new(
            "cargo",
            PathBuf::from(r"C:\project"),
            Environment::inherited(),
            Limits::new(Duration::from_secs(5), 4096, 4096),
        )
        .with_arguments(["test", "--workspace", "a path with spaces"])
    }

    /// A directory of this test's own, holding the named files.
    fn a_directory_holding(name: &str, files: &[&str]) -> PathBuf {
        let directory = sure_testkit::scratch::directory("sure planned work", name);
        for file in files {
            std::fs::write(directory.join(file), b"fixture\n").expect("the fixture file");
        }
        directory
    }

    #[test]
    fn work_carries_its_operation_and_its_identity_together() {
        let proposal = a_proposal();
        let id = proposal.id().clone();
        let work = PlannedWork::new(proposal, CheckOperation::Command(a_spec()));
        assert_eq!(work.proposal().id(), &id);
        assert!(matches!(work.operation(), CheckOperation::Command(_)));
        assert!(work.operation().starts_a_process());
    }

    #[test]
    fn precomputed_work_is_the_only_kind_that_starts_nothing() {
        let work = PlannedWork::new(
            a_proposal(),
            CheckOperation::Precomputed(PrecomputedEvidence::holds("the script is there")),
        );
        assert!(!work.operation().starts_a_process());
    }

    #[test]
    fn an_argument_with_a_space_in_it_is_one_argument() {
        let spec = a_spec();
        assert_eq!(
            spec.arguments(),
            ["test", "--workspace", "a path with spaces"]
        );
        assert_eq!(spec.program(), OsStr::new("cargo"));
    }

    #[test]
    fn a_detector_that_holds_produces_a_pass_and_one_that_could_not_run_does_not() {
        let proposal = a_proposal();
        let fingerprint = a_fingerprint();
        let held = PrecomputedEvidence::holds("package.json declares scripts.test")
            .to_result(&proposal, &fingerprint);
        assert_eq!(held.status, sure_domain::status::CheckStatus::Pass);
        assert!(!held.blocks_green());

        let unread = PrecomputedEvidence::could_not_run("package.json could not be parsed")
            .to_result(&proposal, &fingerprint);
        assert_eq!(unread.status, sure_domain::status::CheckStatus::Error);
        assert!(
            unread.blocks_green(),
            "a detector that could not read what it needed must not leave a critical check green"
        );

        let candidate = PrecomputedEvidence::candidate("a route that looks like a stub")
            .to_result(&proposal, &fingerprint);
        assert_eq!(candidate.status, sure_domain::status::CheckStatus::Warning);

        let contradicted =
            PrecomputedEvidence::contradicted("the lockfile and the manifest disagree")
                .to_result(&proposal, &fingerprint);
        assert_eq!(contradicted.status, sure_domain::status::CheckStatus::Fail);
    }

    #[test]
    fn a_result_carries_the_proposals_own_weight_and_the_fingerprint_it_was_given() {
        let proposal = a_proposal();
        let fingerprint = a_fingerprint();
        let result = PrecomputedEvidence::holds("it is there").to_result(&proposal, &fingerprint);
        assert_eq!(result.evidence_class, EvidenceClass::ObservedFact);
        assert_eq!(result.severity, Severity::ShouldFixFirst);
        assert!(result.critical);
        assert_eq!(result.project_fingerprint, fingerprint);
    }

    #[test]
    fn a_readiness_names_loopback_and_nothing_else() {
        let answers = Readiness::Answers {
            port: 4321,
            path: "/health".to_owned(),
        };
        assert_eq!(
            answers.loopback_url().as_deref(),
            Some("http://127.0.0.1:4321/health")
        );
        assert_eq!(Readiness::StaysUp.loopback_url(), None);
    }

    #[test]
    fn a_browser_check_reads_the_service_it_starts() {
        let service = ServiceCheckSpec::new(
            a_spec(),
            Readiness::Answers {
                port: 5173,
                path: "/".to_owned(),
            },
            Duration::from_secs(20),
        );
        let check = BrowserCheckSpec::new(service, "/post/1", "the post has a title");
        assert_eq!(
            check.loopback_url().as_deref(),
            Some("http://127.0.0.1:5173/post/1")
        );
        assert_eq!(check.expectation(), "the post has a title");
    }

    #[test]
    fn a_name_that_is_not_there_is_absent_and_not_an_error() {
        let path =
            ProgramPath::from_search_path(a_directory_holding("absent", &[]).into_os_string());
        assert_eq!(path.resolve(OsStr::new("nothing-here")), Resolution::Absent);
        assert_eq!(
            path.resolve(OsStr::new("nothing-here")).program_name(),
            None
        );
    }

    #[test]
    fn a_platform_that_completes_a_bare_name_with_nothing_finds_the_name_itself() {
        // The macOS and Linux table, handed to `resolve_with` on Windows. That
        // is the whole reason `resolve_with` takes a table: the defect this
        // guards was a table that was wrong on the two platforms whose tests
        // could not run there, so a test that only ran on those two would have
        // been the same defect one layer up.
        let directory = a_directory_holding("no completion", &["npm"]);
        let search = ProgramPath::from_search_path(directory.clone().into_os_string());
        assert_eq!(
            search.resolve_with(OsStr::new("npm"), WITHOUT_PATHEXT),
            Resolution::Executable(directory.join("npm")),
            "a program that is on PATH must not be reported as absent just \
             because this platform completes a bare name with nothing"
        );
    }

    // There is deliberately no test here for the other half of the empty
    // completion — that a joined suffix would build the candidate `npm.` rather
    // than `npm` — and the reason it is absent is worth more than the test was.
    // **The fixture cannot be built on Windows.** A file created as `npm.` is
    // stored as `npm`: the Win32 layer strips trailing dots and spaces, so on
    // this machine `directory.join("npm.")` and `directory.join("npm")` name one
    // file, and a directory holding the first is a directory holding the second.
    // Measured rather than assumed: the test that asserted otherwise found
    // `…\trailing dot-6\npm` `Executable` on the first run.
    //
    // So the two implementations are indistinguishable here for *any* fixture,
    // and the distinction exists only on the platform where `npm.` is its own
    // file. What catches a joined suffix there is the test above this comment:
    // on a platform that completes a bare name with nothing, the join produces
    // `npm.`, the branch produces `npm`, and only one of those is the file the
    // fixture put on the path. **The collision the empty suffix guards against
    // is therefore reachable, and reachable nowhere the tests run** — which is
    // the same sentence as the defect it repairs, one layer along.

    #[cfg(windows)]
    #[test]
    fn a_name_with_no_extension_is_completed_the_way_windows_completes_it() {
        let directory = a_directory_holding("completed", &["thing.exe", "npm.cmd"]);
        let path = ProgramPath::from_search_path(directory.clone().into_os_string());

        match path.resolve(OsStr::new("thing")) {
            Resolution::Executable(found) => assert_eq!(found, directory.join("thing.exe")),
            other => panic!("thing.exe is on the path, so it is an executable, not {other:?}"),
        }

        match path.resolve(OsStr::new("npm")) {
            Resolution::InterpreterRequired(found) => {
                assert_eq!(found, directory.join("npm.cmd"));
                assert!(!path.resolve(OsStr::new("npm")).is_startable());
            }
            other => {
                panic!("only npm.cmd is there, so the honest answer is a script and not {other:?}")
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_batch_file_named_with_its_extension_is_the_file_that_was_named() {
        let directory = a_directory_holding("spelled-out", &["thing.cmd"]);
        let path = ProgramPath::from_search_path(directory.clone().into_os_string());
        assert_eq!(
            path.resolve(OsStr::new("thing.cmd")),
            Resolution::InterpreterRequired(directory.join("thing.cmd"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_ps1_is_never_a_program_this_build_starts() {
        let directory = a_directory_holding("script", &["deploy.ps1"]);
        let path = ProgramPath::from_search_path(directory.clone().into_os_string());
        match path.resolve(OsStr::new("deploy")) {
            Resolution::InterpreterRequired(found) => {
                assert_eq!(found, directory.join("deploy.ps1"))
            }
            other => panic!("deploy.ps1 is there and is a script, not {other:?}"),
        }
        assert!(!path.resolve(OsStr::new("deploy.ps1")).is_startable());
    }

    #[cfg(windows)]
    #[test]
    fn an_explicit_extension_is_looked_for_by_that_name_and_no_other() {
        let directory = a_directory_holding("explicit", &["thing.exe"]);
        let path = ProgramPath::from_search_path(directory.into_os_string());
        assert_eq!(
            path.resolve(OsStr::new("thing.cmd")),
            Resolution::Absent,
            "a caller that named .cmd asked about .cmd, and completing it with .exe would be \
             answering a different question"
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_executable_wins_when_a_machine_has_both() {
        let directory = a_directory_holding("both", &["thing.bat", "thing.exe"]);
        let path = ProgramPath::from_search_path(directory.clone().into_os_string());
        assert_eq!(
            path.resolve(OsStr::new("thing")),
            Resolution::Executable(directory.join("thing.exe"))
        );
    }

    #[test]
    fn the_directories_are_the_platforms_own_reading_of_the_variable() {
        let first = std::env::temp_dir();
        let second = std::env::temp_dir().join("sure second directory");
        let joined = std::env::join_paths([first.clone(), second.clone()]).unwrap();
        let path = ProgramPath::from_search_path(joined);
        let directories: Vec<PathBuf> = path.directories().collect();
        assert_eq!(directories, vec![first, second]);
    }

    #[test]
    fn a_machine_with_no_path_finds_nothing_rather_than_failing() {
        let path = ProgramPath::from_search_path(OsString::new());
        assert_eq!(path.resolve(OsStr::new("cargo")), Resolution::Absent);
        assert_eq!(path.directories().count(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn an_empty_path_entry_does_not_become_the_current_directory() {
        let directory = a_directory_holding("planted", &["planted.exe"]);
        let planted = directory.join("planted.exe");
        assert!(planted.is_file(), "the fixture is on disk");

        let with_an_empty_entry =
            std::env::join_paths([PathBuf::from(""), directory.clone()]).unwrap();
        let path = ProgramPath::from_search_path(with_an_empty_entry);
        assert_eq!(
            path.directories().count(),
            1,
            "the empty entry means the current directory and is dropped"
        );

        // The one direct way to observe the rule from here: with the fixture's
        // own directory removed, nothing is found even though the empty entry
        // is still in the variable, so the current directory was never read.
        let only_an_empty_entry = ProgramPath::from_search_path(OsString::from(";"));
        assert_eq!(
            only_an_empty_entry.resolve(OsStr::new("planted")),
            Resolution::Absent
        );
    }
}

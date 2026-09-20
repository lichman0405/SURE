//! What SURE would ask a container runtime to do, and what that is not.
//!
//! `P3-T008` acceptance: *"Docker/Podman absence is nonfatal."* / *"Container
//! plan controls mounts/network/working dir and is described as limited
//! isolation, not perfect sandboxing."*
//!
//! # What this module is
//!
//! [`ContainerPlan`] is the argument vector SURE would hand a container runtime
//! for one command, built as data: an image, a bind mount, a network mode and a
//! working directory, each of which is a field rather than a default that
//! somebody else applies. [`Enforcement`](crate::enforce::Enforcement) says which
//! command lines may run at all; this says what the one around them would look
//! like, and [`ContainerPlan::arguments`] is the whole of it.
//!
//! **Nothing here runs anything, and nothing here builds a [`Command`].**
//! [`ContainerPlan::arguments`] returns a `Vec<String>`, which is a value a test
//! can read. Finding a runtime is [`discover`], which is a filesystem search and
//! not an execution — the same distinction [`crate::doctor`] draws for the same
//! reason, and the reason the spawn census in `tests/spawn_sites.rs` is unchanged
//! by this file. Starting a container is where the first real spawn site outside
//! `crate::process` would go, and that is deliberately not this task.
//!
//! # Absence is a value, and that is the first acceptance sentence
//!
//! [`Availability`] has no `Error` variant because a machine without Docker or
//! Podman is not a failure: it is a machine where checks run on the host under
//! the mode and permissions `P3-T005`, `P3-T006` and `P3-T007` built. A caller
//! cannot forget to handle it, because the only way to get a [`Runtime`] out is
//! to match, and `Container` mode's own documentation says what happens then.
//! **An error would have made a normal machine a broken one**, and the failure
//! mode of that is a user told their setup is wrong when it is not.
//!
//! # Where "limited isolation" is, and why it is not marketing
//!
//! The second acceptance sentence has two halves and the second is the one with
//! teeth. [`isolation_claim`] is the sentence this build will say about a
//! container, it says **limited isolation**, and it names what the plan leaves
//! open rather than what it closes. Three facts it is built on, each of which a
//! reader can check:
//!
//! - **The container shares the host's kernel.** A container runtime is not a
//!   virtual machine, and a kernel bug is a way out of a container in a way it
//!   is not a way out of a VM.
//! - **Every mount is a door, and the plan opens one.** The project directory is
//!   visible inside the container by construction; whether it is visible
//!   read-write is [`Access`], and the default is
//!   [`ReadOnly`](Access::ReadOnly).
//! - **The network is the other door.** [`Network::Off`] is `--network none`,
//!   which is a real absence of a route rather than a rule, and it is the
//!   default; a check whose commands need the network gets
//!   [`Network::On`](Network#variant.On), which is a bridge with outbound access.
//!
//! [`OVERCLAIMS`] is the list of phrases a description of this mode may not use,
//! [`overclaims`] is the rule that tells a claim from a denial, and
//! `tests/container_isolation_claim.rs` applies both to every shipped source file
//! and every document under `docs/`. **Four places were overclaiming when this
//! task started** and each is corrected in the commit that added this module —
//! `docs/adr/0009`, `docs/architecture/EXECUTION_SAFETY.md` and two sentences in
//! `sure_domain::execution`. It is a check rather than a note because the wording
//! *is* the acceptance sentence, and because prose is the one thing in this
//! repository that no type checks.
//!
//! # What this does not decide
//!
//! **Whether a check may write inside the project is not a question
//! [`CommandEffects`] can answer**, and that is why [`Access`] is an argument
//! rather than a derivation. The five categories are *static, dynamic host,
//! install, network, destructive*; `npm test` writes `target/` or `node_modules`
//! and is `DynamicHost`, and `git status` writes nothing and is `Static`. **The
//! category that would answer this does not exist**, and its absence is the open
//! owner decision the handoff calls *"where the 'writes inside the project'
//! category lives"*. So the default is read-only, widening is a call a reader
//! can see at the call site, and nothing here guesses.
//!
//! The network *is* derivable, because `Network` is one of the five categories,
//! and it is derived in [`ContainerPlan::for_command`]. The asymmetry is the
//! point rather than an oversight.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use sure_domain::execution::{CommandClass, CommandEffects};

/// Where the project is mounted inside the container.
///
/// A fixed path rather than the host's, because the container's filesystem is
/// not the host's: `C:\Users\...` is not a path inside a Linux container image,
/// and a plan that passed one through would be describing something that cannot
/// exist. One constant rather than a parameter, so that two plans for the same
/// project cannot disagree about where the project is.
pub const PROJECT_ROOT_IN_CONTAINER: &str = "/project";

/// A container runtime SURE knows how to ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Runtime {
    /// Docker, including Docker Desktop on Windows and macOS.
    Docker,
    /// Podman, which is rootless by default and is Docker's CLI-compatible peer.
    Podman,
}

impl Runtime {
    /// Every runtime, in the order [`discover`] looks for them.
    ///
    /// The order is a rule rather than a preference: a machine with both gets
    /// Docker, and *which one was found is reported* by
    /// [`Availability::Found`] rather than left for a reader to assume. Podman is
    /// second because it is the less common of the two, and a discovery rule
    /// that preferred the rarer one would surprise more machines than it helped.
    pub const ALL: &'static [Runtime] = &[Self::Docker, Self::Podman];

    /// The stable name, which is also the program name SURE searches for.
    ///
    /// One method rather than two, because they are the same string on every
    /// platform and two methods with the same body are two places to change.
    /// The wire name is this one as well — `sure.yaml`'s `execution.mode`
    /// spells the *mode* `container`, and this names the runtime underneath it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }

    /// The program name to search `PATH` for.
    #[must_use]
    pub const fn program(self) -> &'static str {
        self.as_str()
    }
}

/// Where a container runtime is, or that this machine has none.
///
/// The second acceptance sentence's first half lives here: there is no error
/// variant, so a caller cannot treat "no runtime" as a failure it must report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// A runtime was found on `PATH`, at this path.
    Found {
        /// Which runtime it is.
        runtime: Runtime,
        /// Where the program is.
        program: PathBuf,
    },
    /// Nothing SURE knows how to ask was found.
    ///
    /// Not a failure and not an empty success: it is the answer that says the
    /// checks run on the host instead, under whatever mode and permissions are
    /// in force.
    Absent,
}

impl Availability {
    /// Look for a runtime in the given `PATH`-shaped search path.
    ///
    /// The search path is a parameter rather than ambient state for the reason
    /// [`crate::doctor`] records at the same place: setting `PATH` in a test
    /// would need `std::env::set_var`, which is `unsafe` in edition 2024 and so
    /// unavailable in a workspace that forbids `unsafe`.
    #[must_use]
    pub fn in_path(search_path: &OsStr) -> Self {
        Runtime::ALL
            .iter()
            .find_map(|runtime| {
                crate::doctor::find_in(search_path, runtime.program())
                    .map(|program| (runtime, program))
            })
            .map_or(Self::Absent, |(runtime, program)| Self::Found {
                runtime: *runtime,
                program,
            })
    }

    /// The same, against this process's own `PATH`.
    #[must_use]
    pub fn on_this_machine() -> Self {
        match std::env::var_os("PATH") {
            Some(search_path) => Self::in_path(&search_path),
            // No `PATH` at all is the same answer as a `PATH` holding no
            // runtime, and it is a real state on Windows services. Reporting it
            // as absent rather than panicking is the whole of this function's
            // contract with its caller.
            None => Self::Absent,
        }
    }

    /// Whether a runtime was found.
    #[must_use]
    pub const fn is_found(&self) -> bool {
        matches!(self, Self::Found { .. })
    }

    /// The runtime, if there is one.
    #[must_use]
    pub const fn runtime(&self) -> Option<Runtime> {
        match self {
            Self::Found { runtime, .. } => Some(*runtime),
            Self::Absent => None,
        }
    }

    /// What to tell a user, in one sentence, either way.
    #[must_use]
    pub fn explain(&self) -> String {
        match self {
            Self::Found { runtime, program } => format!(
                "Checks can run in a container: {} was found at {}.",
                runtime.as_str(),
                program.display()
            ),
            Self::Absent => {
                let names: Vec<&str> = Runtime::ALL.iter().map(|it| it.as_str()).collect();
                format!(
                    "No container runtime was found, so checks run on this computer instead. \
                     SURE looked for {} on PATH.",
                    names.join(" or ")
                )
            }
        }
    }
}

/// Whether the container has a route off this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Network {
    /// No route out. `--network none`, which is the absence of an interface
    /// rather than a rule about one.
    #[default]
    Off,
    /// Outbound access by way of the runtime's default bridge, which is what a
    /// check that installs packages or calls an API needs. Nothing is published
    /// inward: the plan emits no `--publish`.
    On,
}

impl Network {
    /// The value the runtime is given.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "none",
            Self::On => "bridge",
        }
    }
}

/// Whether the container can change what it sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Access {
    /// The mount is readable and not writable.
    ///
    /// The default, because the question this answers — *may a check write
    /// inside the project?* — is one nothing in this build has an answer for.
    /// See this module's *What this does not decide*.
    #[default]
    ReadOnly,
    /// The mount is readable and writable, which the caller asked for by name.
    ReadWrite,
}

/// One directory the container can see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// Where it is on this computer.
    host: PathBuf,
    /// Where it is inside the container.
    container: PathBuf,
    /// What may be done to it there.
    access: Access,
}

impl Mount {
    /// Where it is on this computer.
    #[must_use]
    pub fn host(&self) -> &Path {
        &self.host
    }

    /// Where it is inside the container.
    #[must_use]
    pub fn container(&self) -> &Path {
        &self.container
    }

    /// Whether the container can change it.
    #[must_use]
    pub const fn access(&self) -> Access {
        self.access
    }

    /// The `--mount` value, which is the runtime's own spelling.
    ///
    /// `type=bind,source=…,target=…,readonly` rather than `-v source:target:ro`
    /// because the short form separates its fields with the one character a
    /// Windows path is guaranteed to contain. **A path containing a comma or an
    /// `=` cannot be expressed in this form at all**, and
    /// [`ContainerPlan::new`] refuses rather than emitting an argument the
    /// runtime would split somewhere else — an ambiguity that fails open.
    fn specification(&self) -> String {
        let mut value = format!(
            "type=bind,source={},target={}",
            self.host.display(),
            self.container.display()
        );
        if self.access == Access::ReadOnly {
            value.push_str(",readonly");
        }
        value
    }
}

/// Why a container plan could not be built.
///
/// Every variant is a refusal at the point of construction rather than a
/// surprise at the point of use, which is the rule
/// [`ProcessRequest`](crate::process::ProcessRequest) states from its own side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// No image was named.
    ///
    /// A container with no image is not a container, and a plan that carried an
    /// empty string would be a plan whose argument vector has a hole in it.
    EmptyImage,

    /// The project path holds a character the `--mount` form cannot carry.
    ///
    /// `--mount` is a comma-separated, `key=value` list, so a host path
    /// containing `,` or `=` re-splits into fields that are not this plan's.
    /// The short `-v` form would take the path but cannot take a Windows path,
    /// so there is no spelling that works and this is a refusal rather than a
    /// fallback. Refusing is the direction that fails closed.
    PathNotExpressible {
        /// The path as it was given.
        path: PathBuf,
        /// Which character cannot be carried.
        offending: char,
    },

    /// The working directory is not inside the project mount.
    ///
    /// A container's working directory that the container cannot see is a
    /// process that starts in some other directory, and every relative path in
    /// the command then means something else. Checking it here rather than
    /// letting the runtime pick is the difference between a plan and a surprise.
    WorkingDirectoryOutsideTheMount {
        /// The working directory as it was given.
        working_directory: PathBuf,
        /// Where the project is inside the container.
        mount: PathBuf,
    },
}

/// Everything SURE would tell a container runtime, as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerPlan {
    runtime: Runtime,
    image: String,
    project: Mount,
    network: Network,
    working_directory: PathBuf,
}

impl ContainerPlan {
    /// A plan for running one command in `image` against `project`.
    ///
    /// The defaults are the narrow ones and each is a field below:
    /// [`Access::ReadOnly`], [`Network::Off`], and a working directory of
    /// [`PROJECT_ROOT_IN_CONTAINER`]. Nothing is granted by omission.
    pub fn new(
        runtime: Runtime,
        image: impl Into<String>,
        project: impl Into<PathBuf>,
    ) -> Result<Self, PlanError> {
        let image = image.into();
        if image.trim().is_empty() {
            return Err(PlanError::EmptyImage);
        }
        let host = project.into();
        for offending in [',', '='] {
            if host.to_string_lossy().contains(offending) {
                return Err(PlanError::PathNotExpressible {
                    path: host,
                    offending,
                });
            }
        }
        Ok(Self {
            runtime,
            image,
            project: Mount {
                host,
                container: PathBuf::from(PROJECT_ROOT_IN_CONTAINER),
                access: Access::ReadOnly,
            },
            network: Network::Off,
            working_directory: PathBuf::from(PROJECT_ROOT_IN_CONTAINER),
        })
    }

    /// A plan whose network matches what the command would use.
    ///
    /// The one part of this plan a command's categories *can* answer: `Network`
    /// is one of the five, so a command classified into it gets
    /// [`Network::On`](Network#variant.On) and everything else gets
    /// [`Network::Off`]. Nothing here widens the project mount, because no
    /// category answers that question — see the module documentation.
    pub fn for_command(
        runtime: Runtime,
        image: impl Into<String>,
        project: impl Into<PathBuf>,
        effects: &CommandEffects,
    ) -> Result<Self, PlanError> {
        Ok(Self::new(runtime, image, project)?.with_network(
            if effects.contains(CommandClass::Network) {
                Network::On
            } else {
                Network::Off
            },
        ))
    }

    /// The same plan with the given network mode.
    #[must_use]
    pub fn with_network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    /// The same plan with a writable project mount.
    ///
    /// A call rather than a default, so that the one widening this plan can make
    /// is a line somebody wrote and a reviewer can see. The reason is in the
    /// module documentation: no category in [`CommandEffects`] says a command
    /// writes inside the project, so a derivation would be a guess wearing a
    /// rule's clothes.
    #[must_use]
    pub fn with_writable_project(mut self) -> Self {
        self.project.access = Access::ReadWrite;
        self
    }

    /// The same plan with its working directory set.
    ///
    /// Refuses a directory outside the project mount, because the container
    /// cannot see one and a process that starts elsewhere makes every relative
    /// path in the command mean something else.
    pub fn with_working_directory(
        mut self,
        directory: impl Into<PathBuf>,
    ) -> Result<Self, PlanError> {
        let directory = directory.into();
        let mount = PathBuf::from(PROJECT_ROOT_IN_CONTAINER);
        if !directory.starts_with(&mount) {
            return Err(PlanError::WorkingDirectoryOutsideTheMount {
                working_directory: directory,
                mount,
            });
        }
        self.working_directory = directory;
        Ok(self)
    }

    /// Which runtime this plan is for.
    #[must_use]
    pub const fn runtime(&self) -> Runtime {
        self.runtime
    }

    /// The image, as it will be named to the runtime.
    #[must_use]
    pub fn image(&self) -> &str {
        &self.image
    }

    /// The one directory the container can see.
    #[must_use]
    pub const fn project_mount(&self) -> &Mount {
        &self.project
    }

    /// Whether the container has a route off this machine.
    #[must_use]
    pub const fn network(&self) -> Network {
        self.network
    }

    /// Where inside the container the command would start.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// The argument vector, one element per argument, for the command word
    /// alone — the caller supplies the program.
    ///
    /// Nothing is interpolated through a shell: every element is one argument,
    /// which is the rule `CLAUDE.md` states and the reason a plan is a
    /// `Vec<String>` rather than a command line. The image is last, and the
    /// command to run inside it is the caller's to append — a plan that named a
    /// command would be this module deciding what to run.
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        vec![
            "run".to_owned(),
            "--rm".to_owned(),
            "--mount".to_owned(),
            self.project.specification(),
            "--network".to_owned(),
            self.network.as_str().to_owned(),
            "--workdir".to_owned(),
            self.working_directory.display().to_string(),
            self.image.clone(),
        ]
    }

    /// The argument vector as the operating system would carry it.
    ///
    /// Separate from [`arguments`](Self::arguments) so that the readable form is
    /// what tests read and the `OsString` form is what a runner would be handed,
    /// with no silent conversion in between.
    #[must_use]
    pub fn arguments_as_os_strings(&self) -> Vec<OsString> {
        self.arguments().into_iter().map(OsString::from).collect()
    }
}

/// What this build will say about running a check in a container.
///
/// **The honest sentence, and the second acceptance sentence is about it.** It
/// is a constant rather than a method so that a caller cannot assemble a
/// friendlier one, and it is written to be read by the person deciding whether
/// to switch the mode on. It names what a container gives — a separate
/// filesystem and its own process namespace — before it names what it does not,
/// because a sentence that only listed limits would be read as a warning about
/// something that is nonetheless a boundary.
#[must_use]
pub const fn isolation_claim() -> &'static str {
    "A container gives the check its own filesystem and its own process namespace, \
     and it does not give it a separate kernel or a promise about what it can reach. \
     The project directory is visible inside it, and whether that is read-only is a \
     setting you can see. This is limited isolation: it narrows what a check can \
     touch, and it is not a sandbox."
}

/// The phrases a description of this mode may not use unqualified.
///
/// Kept here rather than in the test so that the list and the sentence it governs
/// are read together. `plain_description` is what a user reads before agreeing
/// to the mode, so an overclaim there is the expensive kind; the same phrase in
/// an architecture note is read by whoever is deciding how much to trust it. The
/// test that applies this list to the whole repository's prose is
/// `tests/container_isolation_claim.rs`, and it reads this constant rather than
/// repeating it — the list and the sentence it governs are one thing.
///
/// **These are the phrases, not the words.** A description that says *"this is
/// not an isolated container"* is making the honest claim, and a rule against the
/// word `isolated` would fail it. So the unit is the phrase a reader would take
/// as the claim, and the test allows one that a negation introduces.
pub const OVERCLAIMS: &[&str] = &["isolated container", "isolated environment"];

/// Whether `sentence` uses `phrase` as a claim rather than denying it.
///
/// Shipped rather than kept in the test because it is the rule the two things
/// above are written against, and because two copies of a rule are two rules:
/// the unit tests here and `tests/container_isolation_claim.rs` ask this one
/// function, so a sentence the module accepts cannot be one the repository
/// rejects.
///
/// # Why the clause and not the sentence
///
/// A denial excuses the phrase only inside the clause the phrase sits in.
/// *"Docker is not installed, and the container is an isolated container"* is
/// two claims, and a window wide enough to see the first `not` would excuse the
/// second — which is the failure this whole check exists to catch, arriving as
/// its own bug. So the search for a negation starts after the last `.`, `,`,
/// `;`, `:` or newline before the phrase.
///
/// **The comma was added by this function's own test, not by its author.** The
/// first version used `.`, `;`, `:` and the newline, and the sentence above —
/// written into that test as the example of an overclaim that must be caught —
/// passed the rule, because a comma is where English puts the joint between two
/// independent clauses and a rule that only understands full stops reads the
/// second one as part of the first. Widening the boundary makes the rule
/// **stricter**, so the direction of the mistake was the safe one and the
/// mistake was still real: the check would have excused exactly the sentence its
/// own documentation named as the thing it was for.
///
/// # What it deliberately cannot tell
///
/// A quotation of a claim is not a denial. A document that reproduces an old
/// sentence in order to correct it will be flagged, and the rule does not
/// pretend otherwise — say which word moved rather than reproducing it. A rule
/// that tried to read intent would be a rule that could be argued with, and the
/// cheap, checkable version of this is worth more than the clever one.
///
/// # The denials, listed rather than pattern-matched
///
/// `ends_with("nt")` would catch every contraction and also `front`, `content`
/// and `important`, so the list is written out. A lint that can be satisfied by
/// an unrelated word is the shape of false green this repository is built
/// against, and a longer list is the cheaper half of that trade.
#[must_use]
pub fn overclaims(sentence: &str, phrase: &str) -> bool {
    let lowered = sentence.to_lowercase();
    let mut from = 0;
    while let Some(offset) = lowered[from..].find(phrase) {
        let position = from + offset;
        if !denied_in_the_same_clause(&lowered[..position]) {
            return true;
        }
        from = position + phrase.len();
    }
    false
}

/// Whether the words before a phrase, within its own clause, deny it.
fn denied_in_the_same_clause(before: &str) -> bool {
    const DENIALS: &[&str] = &[
        "not", "never", "no", "rather", "instead", "cannot", "isnt", "arent", "wasnt", "werent",
        "dont", "doesnt", "didnt", "wont", "cant",
    ];
    let clause = match before.rfind(['.', ',', ';', ':', '\n']) {
        Some(at) => &before[at + 1..],
        None => before,
    };
    clause.split_whitespace().any(|word| {
        let cleaned: String = word.chars().filter(char::is_ascii_alphabetic).collect();
        DENIALS.contains(&cleaned.as_str())
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn plan() -> ContainerPlan {
        ContainerPlan::new(Runtime::Docker, "node:22-slim", "/tmp/project").expect("a plan")
    }

    /// A temporary directory this call can call its own.
    ///
    /// Under the system temp directory and not under the workspace's
    /// `target/tmp`, which is where the code this replaces already put it. That
    /// choice is left where it was rather than moved as a side effect of
    /// replacing one allocator: the test builds a `PATH` out of nothing but this
    /// directory, so what it asserts does not depend on where it is, and a
    /// module that changed one *other* thing in the same commit would be harder
    /// to read.
    ///
    /// The claiming rules are `sure_testkit::scratch`'s, and are the same
    /// wherever the root is — a run directory named for this process's id, and
    /// each call's directory inside it named by a counter only this process can
    /// advance, so a second process testing the same thing cannot be handed the
    /// directory this one is asserting about.
    fn a_directory_of_our_own() -> PathBuf {
        sure_testkit::scratch::under(&std::env::temp_dir(), "sure-container-search-order", "run")
    }

    /// What `doctor` appends to a name with no extension, which is what a
    /// program on this platform is actually called.
    #[cfg(windows)]
    const EXECUTABLE_SUFFIX: &str = ".exe";
    #[cfg(not(windows))]
    const EXECUTABLE_SUFFIX: &str = "";

    #[test]
    fn the_image_is_last_and_the_command_is_not_this_modules_to_name() {
        // The shape a caller appends to. A plan that ended in a command would be
        // this module deciding what to run, which is `Enforcement`'s question.
        let arguments = plan().arguments();
        assert_eq!(arguments.last().map(String::as_str), Some("node:22-slim"));
        assert!(
            !arguments.iter().any(|it| it == "sh" || it == "-c"),
            "a plan must not name an interpreter: {arguments:?}"
        );
    }

    #[test]
    fn a_plan_with_nothing_asked_for_mounts_read_only_and_has_no_route_out() {
        // The two defaults, and they are asserted together because they are one
        // claim: a plan that grants nothing grants neither of these.
        let plan = plan();
        assert_eq!(plan.project_mount().access(), Access::ReadOnly);
        assert_eq!(plan.network(), Network::Off);
        assert!(
            plan.project_mount().specification().ends_with(",readonly"),
            "{}",
            plan.project_mount().specification()
        );
        let arguments = plan.arguments();
        let network = arguments
            .iter()
            .position(|it| it == "--network")
            .expect("a network argument");
        assert_eq!(arguments.get(network + 1).map(String::as_str), Some("none"));
    }

    #[test]
    fn nothing_publishes_a_port_and_nothing_grants_a_privilege() {
        // The two widenings this plan does not have a method for, checked rather
        // than documented: a plan that gained either would be a plan whose
        // isolation claim is no longer the one `isolation_claim` makes.
        for plan in [
            plan(),
            plan().with_network(Network::On),
            plan().with_writable_project(),
        ] {
            let arguments = plan.arguments();
            for forbidden in ["--privileged", "--publish", "-p", "--pid", "--cap-add"] {
                assert!(
                    !arguments.iter().any(|it| it == forbidden),
                    "{forbidden} appeared in {arguments:?}"
                );
            }
        }
    }

    #[test]
    fn the_working_directory_is_inside_the_container_and_never_a_host_path() {
        let plan = plan();
        assert_eq!(
            plan.working_directory(),
            Path::new(PROJECT_ROOT_IN_CONTAINER)
        );
        assert_eq!(
            plan.project_mount().container(),
            Path::new(PROJECT_ROOT_IN_CONTAINER)
        );
        assert_eq!(
            plan.project_mount().host(),
            Path::new("/tmp/project"),
            "the host path belongs to the mount, not to the working directory"
        );
    }

    #[test]
    fn a_working_directory_the_container_cannot_see_is_refused() {
        let error = plan()
            .with_working_directory("/elsewhere")
            .expect_err("refused");
        assert_eq!(
            error,
            PlanError::WorkingDirectoryOutsideTheMount {
                working_directory: PathBuf::from("/elsewhere"),
                mount: PathBuf::from(PROJECT_ROOT_IN_CONTAINER),
            }
        );
        assert!(plan().with_working_directory("/project/src").is_ok());
    }

    #[test]
    fn an_empty_image_is_refused_rather_than_carried() {
        for image in ["", "   "] {
            assert_eq!(
                ContainerPlan::new(Runtime::Docker, image, "/tmp/project").expect_err("refused"),
                PlanError::EmptyImage
            );
        }
    }

    #[test]
    fn a_path_the_mount_form_cannot_carry_is_refused_rather_than_split() {
        // `--mount` is a comma-separated key=value list, so these two characters
        // re-split into fields that are not this plan's. The refusal is the
        // direction that fails closed: the alternative is an argument vector
        // that means something other than what the plan says.
        for (path, offending) in [
            ("/tmp/a,b", ','),
            ("/tmp/a=b", '='),
            ("C:\\Users\\me\\a,b\\project", ','),
        ] {
            assert_eq!(
                ContainerPlan::new(Runtime::Docker, "alpine", path).expect_err("refused"),
                PlanError::PathNotExpressible {
                    path: PathBuf::from(path),
                    offending,
                }
            );
        }
        assert!(
            ContainerPlan::new(Runtime::Docker, "alpine", "C:\\Users\\me\\my project").is_ok(),
            "a space is not an ambiguity, and refusing it would refuse most Windows paths"
        );
    }

    #[test]
    fn a_writable_mount_is_a_call_and_the_specification_says_so() {
        let read_only = plan();
        let writable = plan().with_writable_project();
        assert_eq!(read_only.project_mount().access(), Access::ReadOnly);
        assert_eq!(writable.project_mount().access(), Access::ReadWrite);
        assert!(
            !writable
                .project_mount()
                .specification()
                .contains("readonly")
        );
        assert_eq!(
            read_only.project_mount().specification(),
            "type=bind,source=/tmp/project,target=/project,readonly"
        );
        assert_eq!(
            writable.project_mount().specification(),
            "type=bind,source=/tmp/project,target=/project"
        );
    }

    #[test]
    fn the_network_follows_the_one_category_that_can_answer_it() {
        let with_network = CommandEffects::of(&[CommandClass::Network, CommandClass::Install]);
        let without = CommandEffects::of(&[CommandClass::DynamicHost]);
        assert_eq!(
            ContainerPlan::for_command(Runtime::Docker, "alpine", "/tmp/p", &with_network)
                .expect("a plan")
                .network(),
            Network::On
        );
        assert_eq!(
            ContainerPlan::for_command(Runtime::Docker, "alpine", "/tmp/p", &without)
                .expect("a plan")
                .network(),
            Network::Off
        );
        assert_eq!(
            ContainerPlan::for_command(
                Runtime::Docker,
                "alpine",
                "/tmp/p",
                &CommandEffects::anything()
            )
            .expect("a plan")
            .network(),
            Network::On,
            "anything() includes Network, which is the cautious reading of an unknown command"
        );
    }

    #[test]
    fn the_network_never_widens_the_mount() {
        // The asymmetry, held as a test rather than trusted to the comment: the
        // category that would answer the mount question does not exist, so no
        // command's effects may move it.
        for effects in [
            CommandEffects::static_only(),
            CommandEffects::anything(),
            CommandEffects::of(&[CommandClass::Install]),
        ] {
            let plan = ContainerPlan::for_command(Runtime::Docker, "alpine", "/tmp/p", &effects)
                .expect("a plan");
            assert_eq!(
                plan.project_mount().access(),
                Access::ReadOnly,
                "{effects:?} widened the mount"
            );
        }
    }

    #[test]
    fn every_runtime_is_searchable_under_its_own_name() {
        // The name searched for and the name reported are one string, so a
        // discovery that found `podman` and reported `docker` is not expressible.
        for runtime in Runtime::ALL {
            assert_eq!(runtime.program(), runtime.as_str());
            assert_eq!(runtime.as_str(), runtime.as_str().to_lowercase());
        }
        assert_eq!(
            Runtime::ALL
                .iter()
                .map(|it| it.as_str())
                .collect::<Vec<_>>(),
            vec!["docker", "podman"],
            "the search order is a rule, and this is where it is written down"
        );
    }

    #[test]
    fn a_search_path_with_no_runtime_answers_absent_and_not_an_error() {
        // The first acceptance sentence, as a value: there is nothing to
        // propagate, nothing to `?`, and a caller who ignores this gets a
        // compiler error rather than a runtime failure.
        let availability = Availability::in_path(OsStr::new(""));
        assert_eq!(availability, Availability::Absent);
        assert!(!availability.is_found());
        assert_eq!(availability.runtime(), None);
        let sentence = availability.explain();
        assert!(sentence.contains("docker"), "{sentence}");
        assert!(sentence.contains("podman"), "{sentence}");
        assert!(
            sentence.contains("run on this computer instead"),
            "absence must say what happens instead, not only what is missing: {sentence}"
        );
    }

    #[test]
    fn the_first_runtime_in_the_search_order_is_the_one_reported() {
        // A machine with both gets `Runtime::ALL`'s first, and a machine with one
        // gets that one — and *which* is reported rather than left to the
        // caller's assumption, which is what the variant's own field is for.
        //
        // Written against a real directory rather than `PATH`, so it says the
        // same thing on all three platforms and on a machine with no runtime at
        // all. `doctor::find_in` is what actually answers, and its own tests
        // cover the search; what is being checked here is the *order* and the
        // reporting, which are this module's.
        // **A directory of this call's own**, made unique by `create_dir` rather
        // than by its name. This was one fixed path —
        // `<temp>/sure-container-search-order`, with no per-test and no per-call
        // component — so two processes running this test at once wrote the same
        // two files and either was free to `remove_dir_all` the directory the
        // other was asserting about. Nothing here is a program anything *runs*:
        // the bytes are literally `not a program`, and `Availability::in_path`
        // asks the filesystem whether a name is there and, on a Unix-like
        // platform, whether it is marked executable. Neither direction of
        // `ETXTBSY` is reachable from this file. It is unique anyway because a
        // path two tests can reach is a path whose answer depends on which of
        // them got there first, which is the reason the rest of this crate's
        // test modules give for doing the same thing.
        let root = a_directory_of_our_own();

        // Nothing there yet, on a search path that is nothing but that directory.
        let only = std::env::join_paths([&root]).expect("one entry");
        assert_eq!(Availability::in_path(&only), Availability::Absent);

        let program = |runtime: Runtime| {
            let path = root.join(format!("{}{}", runtime.program(), EXECUTABLE_SUFFIX));
            // The execute bit is passed rather than applied afterwards, and the
            // bytes land beside `path` and are renamed onto it: a stand-in for a
            // program is put at its path by the same door as every other program
            // in this tree. The contents being `not a program` is the point of
            // the test, not a reason to write them by a different route.
            sure_testkit::write_program(&path, b"not a program", 0o755).expect("a file");
            path
        };

        let podman_only = program(Runtime::Podman);
        assert_eq!(
            Availability::in_path(&only),
            Availability::Found {
                runtime: Runtime::Podman,
                program: podman_only,
            },
            "one runtime present is the one reported"
        );

        let docker = program(Runtime::Docker);
        assert_eq!(
            Availability::in_path(&only),
            Availability::Found {
                runtime: Runtime::Docker,
                program: docker,
            },
            "with both present the order in `Runtime::ALL` decides, and the \
             answer says which — a machine that quietly changed runtime between \
             two runs would be a support conversation nobody could have"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_isolation_claim_says_limited_and_says_what_it_is_not() {
        let claim = isolation_claim();
        assert!(claim.contains("limited isolation"), "{claim}");
        assert!(claim.contains("not a sandbox"), "{claim}");
        assert!(
            claim.contains("separate kernel") && claim.contains("project directory"),
            "the claim has to name the doors as well as the walls: {claim}"
        );
    }

    #[test]
    fn no_sentence_this_module_produces_carries_an_overclaim() {
        // The second acceptance sentence, over every sentence this module can
        // produce — the same list `tests/container_isolation_claim.rs` applies to
        // the rest of the repository, applied first to the sentences that are
        // closest to the user. `denies` is that test's helper, deliberately not
        // duplicated: two copies of a rule are two rules.
        let sentences = [
            isolation_claim(),
            &Availability::Absent.explain(),
            &Availability::Found {
                runtime: Runtime::Podman,
                program: PathBuf::from("/usr/bin/podman"),
            }
            .explain(),
        ];
        for sentence in sentences {
            for phrase in OVERCLAIMS {
                assert!(
                    !overclaims(sentence, phrase),
                    "`{phrase}` is used as a claim in: {sentence}"
                );
            }
        }
    }

    #[test]
    fn the_overclaim_rule_tells_a_claim_from_a_denial() {
        // The rule's own test, the way `spawn_sites.rs` tests its matcher: a
        // check whose matcher is wrong is a check that passes for the wrong
        // reason, and this one has two ways to be wrong — missing a claim and
        // excusing one.
        for claim in [
            "The check runs in an isolated container.",
            "SURE will use an isolated environment.",
            "Docker is not installed, and the container is an isolated container.",
        ] {
            assert!(
                overclaims(claim, "isolated container")
                    || overclaims(claim, "isolated environment"),
                "a claim went unnoticed: {claim}"
            );
        }
        for denial in [
            "This is not an isolated container; it is limited isolation.",
            "That is limited isolation rather than an isolated environment.",
            "SURE will never use an isolated container.",
            "There is no isolated container here.",
        ] {
            for phrase in OVERCLAIMS {
                assert!(
                    !overclaims(denial, phrase),
                    "`{phrase}` is denied in {denial:?} and was read as a claim"
                );
            }
        }
        assert!(
            !overclaims(
                "This is limited isolation, not a sandbox.",
                "isolated container"
            ),
            "a sentence that never uses the phrase cannot be using it as a claim"
        );
    }
}

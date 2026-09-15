//! Execution trust.
//!
//! Running a project's tests, build or start command executes code that may
//! have been written by an AI. SURE never blurs static inspection and code
//! execution, and the permissions below are deliberately separate: being
//! allowed to run the tests is not permission to install packages, reach the
//! network, write to the project or talk to a service.

use serde::{Deserialize, Serialize};

use crate::ids::CheckId;
use crate::variants::variants;

/// The category of an action, used for trust classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// Reading a file's contents.
    ReadFile,
    /// Reading a directory listing.
    ListDirectory,
    /// Reading project metadata and configuration.
    ReadMetadata,
    /// Parsing source without running it.
    StaticAnalysis,
    /// Running an automated test suite.
    RunTests,
    /// Building or compiling the project.
    Build,
    /// Type checking without emitting artifacts.
    TypeCheck,
    /// Linting or formatting checks.
    Lint,
    /// Starting a local service.
    StartService,
    /// Probing a local port or URL.
    LocalProbe,
    /// Driving a real browser against a local address.
    BrowserProbe,
    /// Installing project dependencies.
    InstallDependencies,
    /// Reaching the network.
    NetworkAccess,
    /// Writing to files inside the project.
    WriteProjectFile,
    /// Deleting files inside the project.
    DeleteProjectFile,
    /// Running a command SURE does not recognise.
    ArbitraryCommand,
    /// Connecting to an external service.
    ExternalService,
}

variants!(ActionKind {
    ReadFile,
    ListDirectory,
    ReadMetadata,
    StaticAnalysis,
    RunTests,
    Build,
    TypeCheck,
    Lint,
    StartService,
    LocalProbe,
    BrowserProbe,
    InstallDependencies,
    NetworkAccess,
    WriteProjectFile,
    DeleteProjectFile,
    ArbitraryCommand,
    ExternalService
});

impl ActionKind {
    /// Whether performing this action executes project-controlled code.
    #[must_use]
    pub const fn executes_project_code(self) -> bool {
        matches!(
            self,
            Self::RunTests
                | Self::Build
                | Self::TypeCheck
                | Self::Lint
                | Self::StartService
                | Self::InstallDependencies
                | Self::ArbitraryCommand
        )
    }

    /// Whether this action can reach outside the machine.
    #[must_use]
    pub const fn can_touch_network(self) -> bool {
        matches!(
            self,
            Self::NetworkAccess
                | Self::InstallDependencies
                | Self::ExternalService
                | Self::BrowserProbe
                | Self::StartService
        )
    }

    /// Whether this action can change anything on disk.
    #[must_use]
    pub const fn can_modify_disk(self) -> bool {
        matches!(
            self,
            Self::WriteProjectFile | Self::DeleteProjectFile | Self::InstallDependencies
        )
    }

    /// The permission this action needs granted before it may run.
    #[must_use]
    pub const fn required_permission(self) -> Permission {
        match self {
            Self::ReadFile
            | Self::ListDirectory
            | Self::ReadMetadata
            | Self::StaticAnalysis
            | Self::LocalProbe => Permission::Inspect,
            Self::RunTests | Self::Build | Self::TypeCheck | Self::Lint | Self::StartService => {
                Permission::RunProjectCode
            }
            Self::BrowserProbe | Self::ExternalService => Permission::ConnectService,
            Self::InstallDependencies => Permission::InstallDependencies,
            Self::NetworkAccess => Permission::Network,
            Self::WriteProjectFile | Self::DeleteProjectFile => Permission::WriteProject,
            Self::ArbitraryCommand => Permission::RunProjectCode,
        }
    }
}

/// One permission SURE may be granted. Each is independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read-only analysis that runs no project code.
    Inspect,
    /// Execute the project's own commands on this machine.
    RunProjectCode,
    /// Install or update project dependencies.
    InstallDependencies,
    /// Allow outbound network access.
    Network,
    /// Allow SURE to write inside the project directory.
    WriteProject,
    /// Allow connecting to a local or external service.
    ConnectService,
}

variants!(
    /// Every permission, in the order shown to a user.
    Permission { Inspect, RunProjectCode, InstallDependencies, Network, WriteProject, ConnectService }
);

impl Permission {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::RunProjectCode => "run_project_code",
            Self::InstallDependencies => "install_dependencies",
            Self::Network => "network",
            Self::WriteProject => "write_project",
            Self::ConnectService => "connect_service",
        }
    }

    /// The question SURE asks before using this permission.
    #[must_use]
    pub const fn consent_prompt(self) -> &'static str {
        match self {
            Self::Inspect => "Read your project's files and configuration",
            Self::RunProjectCode => "Run your project's own commands, such as its tests",
            Self::InstallDependencies => "Install the packages your project depends on",
            Self::Network => "Let this check use the internet",
            Self::WriteProject => "Change files inside your project",
            Self::ConnectService => "Connect to a service this check needs",
        }
    }

    /// Whether this permission is never granted implicitly by inspect-only mode.
    #[must_use]
    pub const fn is_read_only(self) -> bool {
        matches!(self, Self::Inspect)
    }
}

/// Which permissions have been granted, and where the grant came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPermissions {
    /// Read-only analysis. Always granted.
    pub inspect: bool,
    /// Running project commands on this machine.
    pub run_project_code: bool,
    /// Installing dependencies.
    pub install_dependencies: bool,
    /// Outbound network access.
    pub network: bool,
    /// Writing inside the project directory.
    pub write_project: bool,
    /// Connecting to a service.
    pub connect_service: bool,
}

impl ExecutionPermissions {
    /// Nothing beyond reading files.
    #[must_use]
    pub const fn inspect_only() -> Self {
        Self {
            inspect: true,
            run_project_code: false,
            install_dependencies: false,
            network: false,
            write_project: false,
            connect_service: false,
        }
    }

    /// Whether a specific permission is granted.
    #[must_use]
    pub const fn allows(&self, permission: Permission) -> bool {
        match permission {
            Permission::Inspect => self.inspect,
            Permission::RunProjectCode => self.run_project_code,
            Permission::InstallDependencies => self.install_dependencies,
            Permission::Network => self.network,
            Permission::WriteProject => self.write_project,
            Permission::ConnectService => self.connect_service,
        }
    }

    /// Grant or revoke one permission.
    pub fn set(&mut self, permission: Permission, granted: bool) {
        match permission {
            Permission::Inspect => self.inspect = granted,
            Permission::RunProjectCode => self.run_project_code = granted,
            Permission::InstallDependencies => self.install_dependencies = granted,
            Permission::Network => self.network = granted,
            Permission::WriteProject => self.write_project = granted,
            Permission::ConnectService => self.connect_service = granted,
        }
    }

    /// Whether any permission beyond read-only inspection is granted.
    #[must_use]
    pub const fn exceeds_inspection(&self) -> bool {
        self.run_project_code
            || self.install_dependencies
            || self.network
            || self.write_project
            || self.connect_service
    }

    /// Permissions that are granted, for a summary line or a consent prompt.
    #[must_use]
    pub fn granted(&self) -> Vec<Permission> {
        Permission::ALL
            .iter()
            .copied()
            .filter(|permission| self.allows(*permission))
            .collect()
    }
}

impl Default for ExecutionPermissions {
    fn default() -> Self {
        Self::inspect_only()
    }
}

/// How much of the project's code SURE is allowed to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// No project code is executed. File and configuration parsing only.
    InspectOnly,
    /// The user allowed a specific set of project commands to run on this machine.
    HostConfirmed,
    /// Supported checks run inside an isolated container.
    ///
    /// This is not a perfect security boundary: mounts, network and privileges
    /// are evaluated and reported, not assumed safe.
    Container,
}

variants!(ExecutionMode {
    InspectOnly,
    HostConfirmed,
    Container
});

impl ExecutionMode {
    /// The stable wire name, matching `sure.yaml`'s `execution.mode`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InspectOnly => "inspect_only",
            Self::HostConfirmed => "host_confirmed",
            Self::Container => "container",
        }
    }

    /// The strongest permission set this mode can reach without further consent.
    #[must_use]
    pub const fn baseline_permissions(self) -> ExecutionPermissions {
        match self {
            Self::InspectOnly | Self::Container => ExecutionPermissions::inspect_only(),
            Self::HostConfirmed => ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        }
    }

    /// Whether any project code runs at all in this mode.
    #[must_use]
    pub const fn runs_project_code(self) -> bool {
        matches!(self, Self::HostConfirmed | Self::Container)
    }

    /// Plain-language description for the consent prompt.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::InspectOnly => {
                "SURE will read your project's files only. Nothing in your project will be run."
            }
            Self::HostConfirmed => {
                "SURE will run the project commands you approve, on this computer."
            }
            Self::Container => {
                "SURE will run supported checks inside an isolated container on this computer."
            }
        }
    }
}

/// The user's explicit go-ahead for a specific set of commands on this machine.
///
/// Host execution without one of these is a bug, not a default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostConsent {
    /// Exactly what the user agreed to run, as displayed to them.
    pub approved_commands: Vec<ApprovedCommand>,
    /// When consent was given, as an RFC 3339 timestamp supplied by the caller.
    pub granted_at: String,
    /// Who or what granted it, for the audit trail.
    pub granted_by: ConsentGrantor,
}

/// One command the user approved, with the exact argument vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedCommand {
    /// The program to run.
    pub program: String,
    /// The arguments, kept as a vector so nothing is re-parsed by a shell.
    pub args: Vec<String>,
    /// The working directory, relative to the project root where possible.
    pub working_directory: String,
    /// Which planned check this approval covers.
    pub check: CheckId,
    /// What SURE had read the command as when the user was shown it.
    ///
    /// **The other four fields say what would run. This one says what the user
    /// was told it was, and it is the only field a later build can disagree
    /// with.** [`crate::execution`] has no classifier in it — the reading comes
    /// from `sure_core::safety`, which is deterministic on the program and the
    /// argument vector — so within one build the two always agree. But an
    /// approval is a record that outlives the build that wrote it, and the
    /// question *may this run* is put again by a classifier that may since have
    /// learned something. `P3-T006`'s acceptance is about categories, and a
    /// record that does not carry the categories the user approved cannot be
    /// checked against the categories the command now falls into.
    ///
    /// It is also the audit answer to a question the other four cannot answer:
    /// a reader of this record can see that the user was told the command
    /// *can destroy data*, rather than having to trust that whoever wrote the
    /// prompt said so.
    pub effects: CommandEffects,
}

impl ApprovedCommand {
    /// A display form for the consent prompt and the audit log.
    ///
    /// This is for humans only. Execution always uses the argument vector.
    #[must_use]
    pub fn display(&self) -> String {
        let mut out = self.program.clone();
        for arg in &self.args {
            out.push(' ');
            if arg.contains(' ') {
                out.push('"');
                out.push_str(arg);
                out.push('"');
            } else {
                out.push_str(arg);
            }
        }
        out
    }
}

/// Where a consent decision came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentGrantor {
    /// The person at the keyboard answered a prompt.
    InteractiveUser,
    /// A user-level SURE configuration file outside the project granted it.
    UserConfiguration,
    /// An organization policy file. Future team edition; never a project file.
    OrganizationPolicy,
    /// A project file asked for it.
    ///
    /// A project may **request** a privileged behaviour; it can never grant it,
    /// so this value exists only to record the request that was escalated.
    ProjectRequestEscalated,
}

variants!(ConsentGrantor {
    InteractiveUser,
    UserConfiguration,
    OrganizationPolicy,
    ProjectRequestEscalated
});

impl ConsentGrantor {
    /// Whether this grantor is allowed to grant execution authority at all.
    ///
    /// Project-controlled configuration cannot grant itself more authority than
    /// user or organization policy allows.
    #[must_use]
    pub const fn can_grant(self) -> bool {
        matches!(
            self,
            Self::InteractiveUser | Self::UserConfiguration | Self::OrganizationPolicy
        )
    }
}

/// A decision about whether an action may be performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionDecision {
    /// The permission is granted; proceed.
    Allowed,
    /// The permission is not granted, and could be requested.
    NeedsConsent,
    /// The permission is not granted and no prompt can be made here.
    Denied,
}

variants!(ExecutionDecision {
    Allowed,
    NeedsConsent,
    Denied
});

impl ExecutionDecision {
    /// Whether the action may proceed.
    #[must_use]
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// Whether this decision leaves the check unperformed.
    #[must_use]
    pub const fn leaves_check_unperformed(self) -> bool {
        !self.is_allowed()
    }
}

/// Decide whether an action may proceed under the current mode and permissions.
///
/// A denied action is reported as `skipped` with a reason. It is never a pass,
/// and it never quietly disappears from the report.
#[must_use]
pub fn decide(
    action: ActionKind,
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
) -> ExecutionDecision {
    let required = action.required_permission();
    if !permissions.allows(required) {
        return ExecutionDecision::Denied;
    }
    // An unrecognised command is never covered by a blanket grant, not even in
    // host-confirmed mode: SURE cannot classify it, so it always needs its own
    // approval naming the exact argument vector.
    if action == ActionKind::ArbitraryCommand {
        return ExecutionDecision::NeedsConsent;
    }
    if action.executes_project_code() && !mode.runs_project_code() {
        return ExecutionDecision::NeedsConsent;
    }
    ExecutionDecision::Allowed
}

/// What a command may do, in the categories SURE keeps apart.
///
/// `P3-T004` acceptance: *"Static/read-only, dynamic host, install, network,
/// destructive categories are distinct."* **Distinct is the load-bearing word.**
/// These are not rungs of one ladder, and the answer for a command is not a
/// single value: `cargo add serde` installs a package *and* reaches the
/// registry, and collapsing those into one category would lose the fact the
/// consent path needs — that two separate permissions are involved. The set is
/// [`CommandEffects`].
///
/// The list is short on purpose. Categories the acceptance does not name —
/// writing inside the project, connecting to a service — are [`ActionKind`]'s
/// and [`Permission`]'s, and a classifier that invented a sixth value here
/// would be answering a question this vocabulary does not have. A command whose
/// only visible effect is one of those is *not* classified as
/// [`Static`](Self::Static): it is unrecognised, which is the cautious answer
/// rather than the tidy one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandClass {
    /// Reads and reports.
    ///
    /// It runs a program — a rule that only ever answered "nothing happened"
    /// would be about a different question — but not the project's code, it
    /// installs nothing, it does not reach the network, and it cannot destroy
    /// anything. `git status` is the shape of it.
    Static,
    /// Runs project-controlled code on this machine.
    ///
    /// The project's own tests, its build, its start command: whatever the
    /// command is, the code that runs was written by whoever wrote the project.
    DynamicHost,
    /// Changes which packages the project depends on.
    Install,
    /// Reaches the network.
    Network,
    /// Can destroy work that re-running the command does not bring back.
    ///
    /// Deletion, history rewriting, forced pushes. Deliberately not folded into
    /// [`Permission::WriteProject`]: `rm -rf` outside the project and
    /// `git push --force` are neither of them a write inside the project, and a
    /// consent for one would be read as a consent for the other.
    Destructive,
}

variants!(CommandClass {
    Static,
    DynamicHost,
    Install,
    Network,
    Destructive
});

impl CommandClass {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::DynamicHost => "dynamic_host",
            Self::Install => "install",
            Self::Network => "network",
            Self::Destructive => "destructive",
        }
    }

    /// What a command of this category does, in plain language.
    ///
    /// Written to complete the sentence **"because the command ___"**, which is
    /// how [`Permission::consent_prompt`] is explained to a user: the prompt is
    /// the permission being asked for and this is the reason it is being asked
    /// for. It is a clause rather than a sentence for that reason, and it is
    /// here rather than in the prompt-building code because a category's plain
    /// description is a fact about the category, in the same way
    /// [`ExecutionMode::plain_description`] is a fact about the mode.
    ///
    /// It describes the *command*, never the code the command runs — SURE has
    /// not read that, and a sentence that implied otherwise would be the one
    /// thing this vocabulary exists to avoid.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Static => "only reads things",
            Self::DynamicHost => "runs the project's own code",
            Self::Install => "installs or updates packages",
            Self::Network => "reaches the network",
            Self::Destructive => "can destroy data",
        }
    }

    /// The permission that covers a command of this category, when one does.
    ///
    /// **`Destructive` answers `None`, and that is a finding rather than a gap
    /// left open by accident.** [`Permission`] has six values and not one of
    /// them is about destruction: [`Permission::WriteProject`] is writing
    /// *inside the project*, which `rm -rf ..\..\` is not, and
    /// `git push --force` is not a write at all. Mapping the category onto the
    /// nearest of the six would make a user who granted one permission read as
    /// having granted the other, which is the false green this repository calls
    /// the worst outcome there is. The narrow thing the domain does cover is
    /// [`ActionKind::DeleteProjectFile`]; the category is wider than it, so the
    /// honest answer is that nothing here covers it — and deciding what should
    /// is `P3-T005`'s question, which is why this returns the question rather
    /// than an answer.
    #[must_use]
    pub const fn required_permission(self) -> Option<Permission> {
        match self {
            Self::Static => Some(Permission::Inspect),
            Self::DynamicHost => Some(Permission::RunProjectCode),
            Self::Install => Some(Permission::InstallDependencies),
            Self::Network => Some(Permission::Network),
            Self::Destructive => None,
        }
    }
}

/// Every category one command may fall into.
///
/// A set rather than a severity, and **never empty**. The order is
/// [`CommandClass::ALL`]'s rather than the order rules happened to fire in, so
/// two runs that agree about a command produce the same value and a report
/// written today is readable tomorrow.
///
/// Constructed by union rather than assembled from a list, so emptiness is not
/// a state this type can reach by accident: [`single`](Self::single) is one
/// category, [`union`](Self::union) of two non-empty sets is non-empty, and the
/// two named constructors are the two answers that are not about a single
/// category. The only door that takes an arbitrary list is [`of`](Self::of),
/// and what it does with an empty one is stated there.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct CommandEffects {
    /// Canonical: sorted by [`CommandClass::ALL`]'s order, no duplicates, never
    /// empty. Every constructor routes through [`CommandEffects::canonical`].
    classes: Vec<CommandClass>,
}

impl CommandEffects {
    /// One category on its own.
    #[must_use]
    pub fn single(class: CommandClass) -> Self {
        Self {
            classes: vec![class],
        }
    }

    /// Everything except [`CommandClass::Static`].
    ///
    /// "SURE did not read this command" is an answer about danger, not about
    /// safety, so it is the four categories that carry a risk and not all five.
    /// A value that included `Static` would say both "this reads and reports"
    /// and "this may destroy", which is not a cautious answer but a meaningless
    /// one — and it would make [`is_static_only`](Self::is_static_only) false
    /// for the wrong reason.
    #[must_use]
    pub fn anything() -> Self {
        Self::of(
            &CommandClass::ALL
                .iter()
                .copied()
                .filter(|class| *class != CommandClass::Static)
                .collect::<Vec<_>>(),
        )
    }

    /// Exactly [`CommandClass::Static`].
    ///
    /// Spelled out rather than left to `single(Static)` at the call sites: this
    /// is the one value the product treats as permission to proceed without
    /// asking, so it should be readable as that sentence in the code that
    /// answers it.
    #[must_use]
    pub fn static_only() -> Self {
        Self::single(CommandClass::Static)
    }

    /// Every category in `classes`, and nothing else.
    ///
    /// # An empty list is `anything`, and that is deliberate
    ///
    /// A rule that named no category has not said the command is safe; it has
    /// said nothing. The two ways to read that are "no effects" — which would
    /// be a rule that silently permits everything it forgot to mention — and
    /// "unknown", which is what [`anything`](Self::anything) means. So an empty
    /// list lands on the cautious side, and a caller that meant "static" has to
    /// say [`CommandClass::Static`].
    #[must_use]
    pub fn of(classes: &[CommandClass]) -> Self {
        if classes.is_empty() {
            return Self::anything();
        }
        Self::canonical(classes)
    }

    /// Both sets at once.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let mut classes = self.classes;
        classes.extend(other.classes);
        Self::canonical(&classes)
    }

    /// Sorted by the enum's own order, without duplicates.
    fn canonical(classes: &[CommandClass]) -> Self {
        let mut kept: Vec<CommandClass> = CommandClass::ALL
            .iter()
            .copied()
            .filter(|class| classes.contains(class))
            .collect();
        if kept.is_empty() {
            // Only reachable from a list that holds something `ALL` does not,
            // which cannot happen while both are generated from the same enum.
            kept = vec![CommandClass::Static];
        }
        Self { classes: kept }
    }

    /// The categories, in [`CommandClass::ALL`] order.
    #[must_use]
    pub fn classes(&self) -> &[CommandClass] {
        &self.classes
    }

    /// Whether this command may fall into `class`.
    #[must_use]
    pub fn contains(&self, class: CommandClass) -> bool {
        self.classes.contains(&class)
    }

    /// Whether the only thing SURE can say about this command is that it reads
    /// and reports.
    ///
    /// The one predicate a caller should branch on to proceed without asking,
    /// and it is true of exactly one value.
    #[must_use]
    pub fn is_static_only(&self) -> bool {
        self.classes.len() == 1 && self.classes[0] == CommandClass::Static
    }

    /// Whether this set holds every category in `other`.
    ///
    /// The relationship between a set of categories somebody agreed to and the
    /// set a command actually falls into, and it is deliberately one-way: a
    /// consent given for `git clean` *covers* nothing else, while a category set
    /// that has grown a category the agreement does not name is not covered by
    /// it. Writing it as equality would refuse a command that narrowed — and
    /// more importantly it would make the interesting direction, the one where
    /// something gained a danger nobody agreed to, unreadable at the call site.
    #[must_use]
    pub fn covers(&self, other: &Self) -> bool {
        other
            .classes
            .iter()
            .all(|class| self.classes.contains(class))
    }
}

impl<'de> Deserialize<'de> for CommandEffects {
    /// Read a set of categories back, refusing one that names nothing.
    ///
    /// **The door that does not match [`of`](CommandEffects::of), and the
    /// difference is the direction "cautious" points in.** `of` answers
    /// [`anything`](CommandEffects::anything) for an empty list, because a rule
    /// that named no category has said nothing about a command, and for a
    /// *classification* saying nothing must land on the dangerous side. A value
    /// arriving over a wire is not a classification — it is a claim about
    /// something that already happened — and for the set of categories a user
    /// approved, the cautious direction is the other one. Reading an empty list
    /// as `anything` would turn *this approval covers nothing* into *this
    /// approval covers everything*, which is the false green in its purest form.
    ///
    /// Everything else goes through [`canonical`](CommandEffects::canonical), so
    /// the order the classes arrive in and any repetition are the writer's
    /// business rather than the reader's.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let classes = Vec::<CommandClass>::deserialize(deserializer)?;
        if classes.is_empty() {
            return Err(<D::Error as serde::de::Error>::custom(
                "a set of command categories is never empty: `of(&[])` answers `anything` for a \
                 rule that named nothing, but a value that says a command falls into no category \
                 has said something SURE cannot read",
            ));
        }
        Ok(Self::canonical(&classes))
    }
}

impl std::fmt::Display for CommandEffects {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, class) in self.classes.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            formatter.write_str(class.as_str())?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn modes_have_the_frozen_wire_names() {
        assert_eq!(ExecutionMode::InspectOnly.as_str(), "inspect_only");
        assert_eq!(ExecutionMode::HostConfirmed.as_str(), "host_confirmed");
        assert_eq!(ExecutionMode::Container.as_str(), "container");
    }

    #[test]
    fn inspect_only_grants_nothing_beyond_reading() {
        let permissions = ExecutionMode::InspectOnly.baseline_permissions();
        assert!(permissions.allows(Permission::Inspect));
        assert_eq!(permissions.granted(), vec![Permission::Inspect]);
        assert!(!permissions.exceeds_inspection());
        assert!(!ExecutionMode::InspectOnly.runs_project_code());
    }

    #[test]
    fn host_confirmed_does_not_imply_install_network_or_write() {
        // The four decisions are separate. Approving test execution must never
        // silently approve `npm install`, network access or file writes.
        let permissions = ExecutionMode::HostConfirmed.baseline_permissions();
        assert!(permissions.allows(Permission::RunProjectCode));
        assert!(!permissions.allows(Permission::InstallDependencies));
        assert!(!permissions.allows(Permission::Network));
        assert!(!permissions.allows(Permission::WriteProject));
        assert!(!permissions.allows(Permission::ConnectService));
    }

    #[test]
    fn every_permission_is_independent_of_the_others() {
        for &permission in Permission::ALL {
            let mut permissions = ExecutionPermissions::inspect_only();
            if permission == Permission::Inspect {
                continue;
            }
            permissions.set(permission, true);
            assert!(permissions.allows(permission));
            for &other in Permission::ALL {
                if other == permission || other == Permission::Inspect {
                    continue;
                }
                assert!(
                    !permissions.allows(other),
                    "granting {permission:?} also granted {other:?}"
                );
            }
        }
    }

    #[test]
    fn action_kinds_map_to_the_right_permission() {
        assert_eq!(
            ActionKind::RunTests.required_permission(),
            Permission::RunProjectCode
        );
        assert_eq!(
            ActionKind::InstallDependencies.required_permission(),
            Permission::InstallDependencies
        );
        assert_eq!(
            ActionKind::NetworkAccess.required_permission(),
            Permission::Network
        );
        assert_eq!(
            ActionKind::DeleteProjectFile.required_permission(),
            Permission::WriteProject
        );
        assert_eq!(
            ActionKind::BrowserProbe.required_permission(),
            Permission::ConnectService
        );
        assert_eq!(
            ActionKind::ReadFile.required_permission(),
            Permission::Inspect
        );
        assert_eq!(
            ActionKind::StaticAnalysis.required_permission(),
            Permission::Inspect
        );
    }

    #[test]
    fn inspect_only_denies_running_project_code() {
        let decision = decide(
            ActionKind::RunTests,
            ExecutionMode::InspectOnly,
            &ExecutionMode::InspectOnly.baseline_permissions(),
        );
        assert_eq!(decision, ExecutionDecision::Denied);
        assert!(decision.leaves_check_unperformed());
    }

    #[test]
    fn host_confirmed_allows_running_tests_but_not_installing() {
        let permissions = ExecutionMode::HostConfirmed.baseline_permissions();
        assert!(
            decide(
                ActionKind::RunTests,
                ExecutionMode::HostConfirmed,
                &permissions
            )
            .is_allowed()
        );
        assert_eq!(
            decide(
                ActionKind::InstallDependencies,
                ExecutionMode::HostConfirmed,
                &permissions
            ),
            ExecutionDecision::Denied
        );
    }

    #[test]
    fn an_arbitrary_command_always_needs_its_own_approval() {
        for mode in [ExecutionMode::HostConfirmed, ExecutionMode::Container] {
            let mut permissions = mode.baseline_permissions();
            permissions.set(Permission::RunProjectCode, true);
            assert_eq!(
                decide(ActionKind::ArbitraryCommand, mode, &permissions),
                ExecutionDecision::NeedsConsent,
                "{mode:?}"
            );
        }
    }

    #[test]
    fn container_mode_does_not_grant_anything_extra_by_itself() {
        let permissions = ExecutionMode::Container.baseline_permissions();
        assert!(ExecutionMode::Container.runs_project_code());
        assert_eq!(permissions.granted(), vec![Permission::Inspect]);
        assert_eq!(
            decide(ActionKind::RunTests, ExecutionMode::Container, &permissions),
            ExecutionDecision::Denied
        );
    }

    #[test]
    fn a_project_file_cannot_grant_execution_authority() {
        assert!(!ConsentGrantor::ProjectRequestEscalated.can_grant());
        assert!(ConsentGrantor::InteractiveUser.can_grant());
        assert!(ConsentGrantor::UserConfiguration.can_grant());
        assert!(ConsentGrantor::OrganizationPolicy.can_grant());
    }

    #[test]
    fn an_approved_command_displays_without_a_shell_round_trip() {
        let command = ApprovedCommand {
            program: "cargo".to_owned(),
            args: vec!["test".to_owned(), "--all-features".to_owned()],
            working_directory: ".".to_owned(),
            check: CheckId::generate(),
            effects: CommandEffects::single(CommandClass::DynamicHost),
        };
        assert_eq!(command.display(), "cargo test --all-features");

        let spaced = ApprovedCommand {
            args: vec!["--path".to_owned(), "C:\\Program Files\\x".to_owned()],
            ..command
        };
        assert_eq!(spaced.display(), "cargo --path \"C:\\Program Files\\x\"");
    }

    #[test]
    fn an_approval_says_what_the_user_was_told_the_command_was() {
        // The fifth field, and the one an audit reads. The first four say what
        // would run; this one says what SURE told the user it would be doing,
        // and it survives the trip through the store so that a reader does not
        // have to trust that whoever wrote the prompt said so.
        let command = ApprovedCommand {
            program: "git".to_owned(),
            args: vec!["clean".to_owned(), "-fdx".to_owned()],
            working_directory: ".".to_owned(),
            check: CheckId::generate(),
            effects: CommandEffects::single(CommandClass::Destructive),
        };
        let text = serde_json::to_string(&command).expect("a consent record is serializable");
        assert!(
            text.contains("\"destructive\""),
            "the reading is in the record: {text}"
        );
        let read: ApprovedCommand =
            serde_json::from_str(&text).expect("and it reads back as the same value");
        assert_eq!(read, command);
    }

    #[test]
    fn a_set_of_categories_read_back_is_canonical_or_refused() {
        // Two rules, and they are the same rule: what comes off a wire is a
        // *set*, so order and repetition are the writer's business — and a set
        // with nothing in it is not a value this type has, so it is refused
        // rather than widened to `anything`.
        let read = |text: &str| serde_json::from_str::<CommandEffects>(text);

        assert_eq!(
            read("[\"network\",\"install\",\"network\"]").expect("a set, however spelled"),
            CommandEffects::of(&[CommandClass::Install, CommandClass::Network]),
            "the canonical order is the enum's, whatever order arrived"
        );
        assert!(
            read("[]").is_err(),
            "an empty set is not a value SURE reads"
        );
        assert!(
            read("[\"telepathy\"]").is_err(),
            "a category this build does not know is not guessed at"
        );
    }

    #[test]
    fn only_a_rule_that_named_nothing_widens_to_anything() {
        // The two doors, side by side. `of(&[])` is a classifier that forgot to
        // name a category, and saying nothing about a command has to land on the
        // dangerous side. A deserialized `[]` is a record claiming a command
        // falls into no category at all, which is not a claim this type can
        // hold, and reading it as `anything` would turn "approved nothing" into
        // "approved everything".
        assert_eq!(CommandEffects::of(&[]), CommandEffects::anything());
        assert!(serde_json::from_str::<CommandEffects>("[]").is_err());
    }

    #[test]
    fn covers_is_one_way_and_says_which_way() {
        let approved = CommandEffects::of(&[CommandClass::Install, CommandClass::Network]);
        assert!(approved.covers(&CommandEffects::single(CommandClass::Install)));
        assert!(approved.covers(&approved));

        // A command that gained a category nobody agreed to is not covered —
        // which is the direction the whole rule exists for.
        let grown = CommandEffects::of(&[
            CommandClass::Install,
            CommandClass::Network,
            CommandClass::Destructive,
        ]);
        assert!(!approved.covers(&grown));
        assert!(grown.covers(&approved));

        // And a command that still falls inside what was approved is covered even
        // when it no longer falls into all of it, because nothing unapproved is
        // executing and refusing it would be refusing the safer reading.
        assert!(approved.covers(&CommandEffects::single(CommandClass::Install)));
        assert!(approved.covers(&CommandEffects::single(CommandClass::Network)));

        // Falling inside is not the same as shrinking, and `Static` is not a subset
        // of `Install, Network` — it is a different reading, and this answers
        // `false` for it. That combination cannot reach a gate: a static-only
        // command is permitted by `Permission::Inspect` before any consent is
        // consulted, so the question this method answers is only ever asked about a
        // command that needs consent.
        assert!(!approved.covers(&CommandEffects::static_only()));
    }

    #[test]
    fn mode_descriptions_are_plain_language() {
        for mode in [
            ExecutionMode::InspectOnly,
            ExecutionMode::HostConfirmed,
            ExecutionMode::Container,
        ] {
            let text = mode.plain_description().to_lowercase();
            assert!(text.starts_with("sure will"), "{mode:?}");
            for banned in ["execute", "sandbox", "sandboxed", "process spawn"] {
                assert!(!text.contains(banned), "{mode:?} uses jargon '{banned}'");
            }
        }
    }

    #[test]
    fn command_classes_have_the_frozen_wire_names() {
        assert_eq!(CommandClass::Static.as_str(), "static");
        assert_eq!(CommandClass::DynamicHost.as_str(), "dynamic_host");
        assert_eq!(CommandClass::Install.as_str(), "install");
        assert_eq!(CommandClass::Network.as_str(), "network");
        assert_eq!(CommandClass::Destructive.as_str(), "destructive");
    }

    #[test]
    fn every_command_class_explains_itself_as_the_reason_a_permission_is_asked_for() {
        // Each description completes "because the command ___", which is the
        // sentence a consent prompt is built from — so each is a clause written
        // mid-sentence, and two categories that explained themselves the same
        // way would make a prompt unable to say which one it was asking about.
        let mut seen: Vec<&str> = Vec::new();
        for &class in CommandClass::ALL {
            let text = class.plain_description();
            assert!(!text.is_empty(), "{class:?} has no description");
            assert!(
                !text.ends_with('.'),
                "{class:?} is a clause, not a sentence"
            );
            assert!(
                !text.chars().next().is_some_and(char::is_uppercase),
                "{class:?} is written to follow the words before it"
            );
            for banned in ["execute", "sandbox", "spawn", "process", "invoke"] {
                assert!(!text.contains(banned), "{class:?} uses jargon '{banned}'");
            }
            assert!(
                !seen.contains(&text),
                "{class:?} explains itself as {text:?}, which another category already said"
            );
            seen.push(text);
        }
        assert_eq!(seen.len(), CommandClass::ALL.len());
    }

    #[test]
    fn the_five_command_classes_are_distinct() {
        // The acceptance sentence, as a loop rather than a reading: for every
        // ordered pair the two are different values, carry different wire
        // names, and — where both name a permission — need different
        // permissions. `Destructive` is the one with no permission, and the
        // test below is where that is held.
        for &left in CommandClass::ALL {
            for &right in CommandClass::ALL {
                if left == right {
                    continue;
                }
                assert_ne!(left.as_str(), right.as_str());
                if let (Some(a), Some(b)) =
                    (left.required_permission(), right.required_permission())
                {
                    assert_ne!(
                        a, b,
                        "{left:?} and {right:?} are separate categories that need the same permission"
                    );
                }
            }
        }
    }

    #[test]
    fn destructive_is_not_folded_into_writing_the_project() {
        // The nearest of the six permissions is `WriteProject`, and it is the
        // wrong answer: `rm -rf` outside the project is not a write inside it,
        // and `git push --force` is not a write at all. Folding the category
        // into it would let a user who granted a write consent read as having
        // granted consent to destroy.
        assert_eq!(CommandClass::Destructive.required_permission(), None);

        // And the categories that *do* have a permission have the specific one
        // rather than a neighbouring one.
        assert_eq!(
            CommandClass::Static.required_permission(),
            Some(Permission::Inspect)
        );
        assert_eq!(
            CommandClass::DynamicHost.required_permission(),
            Some(Permission::RunProjectCode)
        );
        assert_eq!(
            CommandClass::Install.required_permission(),
            Some(Permission::InstallDependencies)
        );
        assert_eq!(
            CommandClass::Network.required_permission(),
            Some(Permission::Network)
        );
    }

    #[test]
    fn a_command_can_be_in_two_classes_at_once() {
        // `cargo add serde` installs a package and reaches the registry, and a
        // set is what keeps both facts. A single-valued answer would have to
        // drop one of them.
        let effects = CommandEffects::of(&[CommandClass::Install, CommandClass::Network]);
        assert_eq!(effects.classes().len(), 2);
        assert!(effects.contains(CommandClass::Install));
        assert!(effects.contains(CommandClass::Network));
        assert!(!effects.contains(CommandClass::Static));
        assert!(!effects.is_static_only());
    }

    #[test]
    fn the_order_classes_were_named_in_does_not_change_the_answer() {
        let forward = CommandEffects::of(&[
            CommandClass::Network,
            CommandClass::Install,
            CommandClass::Destructive,
        ]);
        let reversed = CommandEffects::of(&[
            CommandClass::Destructive,
            CommandClass::Install,
            CommandClass::Network,
        ]);
        assert_eq!(forward, reversed);
        assert_eq!(
            forward.classes(),
            [
                CommandClass::Install,
                CommandClass::Network,
                CommandClass::Destructive
            ],
            "the canonical order is the enum's, so a report written today is readable tomorrow"
        );

        // Duplicates are the same set, not a longer one.
        let repeated = CommandEffects::of(&[CommandClass::Network, CommandClass::Network]);
        assert_eq!(repeated, CommandEffects::single(CommandClass::Network));
    }

    #[test]
    fn no_set_of_command_classes_is_empty() {
        // A rule that named no class has not said the command is safe, it has
        // said nothing, and "nothing" has to land on the cautious side.
        let unnamed = CommandEffects::of(&[]);
        assert_eq!(unnamed, CommandEffects::anything());
        assert!(!unnamed.is_static_only());

        for effects in [
            CommandEffects::anything(),
            CommandEffects::static_only(),
            CommandEffects::of(&[]),
            CommandEffects::single(CommandClass::Install),
            CommandEffects::static_only().union(CommandEffects::anything()),
            CommandEffects::single(CommandClass::Install)
                .union(CommandEffects::single(CommandClass::Network)),
        ] {
            assert!(!effects.classes().is_empty(), "{effects:?} is an empty set");
        }
    }

    #[test]
    fn anything_is_every_class_but_static() {
        let anything = CommandEffects::anything();
        assert_eq!(
            anything.classes(),
            [
                CommandClass::DynamicHost,
                CommandClass::Install,
                CommandClass::Network,
                CommandClass::Destructive,
            ],
            "an unread command is a statement about danger, and `static` is not one of the dangers"
        );
        assert!(!anything.contains(CommandClass::Static));
    }

    #[test]
    fn only_static_alone_reads_as_safe_to_proceed() {
        // The one value a caller branches on to run without asking.
        assert!(CommandEffects::static_only().is_static_only());
        for &class in CommandClass::ALL {
            let single = CommandEffects::single(class);
            assert_eq!(
                single.is_static_only(),
                class == CommandClass::Static,
                "{class:?} alone was read as static"
            );
        }
    }

    #[test]
    fn union_of_two_sets_holds_both_and_neither_loses_static() {
        let read = CommandEffects::static_only();
        assert!(read.contains(CommandClass::Static));

        let read_installing = read
            .clone()
            .union(CommandEffects::single(CommandClass::Install));
        assert_eq!(
            read_installing.classes(),
            [CommandClass::Static, CommandClass::Install]
        );
        assert!(!read_installing.is_static_only());

        // Unioning the same set twice is the same set.
        assert_eq!(read.clone().union(read), CommandEffects::static_only());
    }

    #[test]
    fn command_effects_display_as_their_wire_names() {
        assert_eq!(CommandEffects::static_only().to_string(), "static");
        assert_eq!(
            CommandEffects::of(&[CommandClass::Install, CommandClass::Network]).to_string(),
            "install, network"
        );
        assert_eq!(
            CommandEffects::anything().to_string(),
            "dynamic_host, install, network, destructive"
        );
    }
}

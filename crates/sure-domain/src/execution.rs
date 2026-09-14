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
        };
        assert_eq!(command.display(), "cargo test --all-features");

        let spaced = ApprovedCommand {
            args: vec!["--path".to_owned(), "C:\\Program Files\\x".to_owned()],
            ..command
        };
        assert_eq!(spaced.display(), "cargo --path \"C:\\Program Files\\x\"");
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
}

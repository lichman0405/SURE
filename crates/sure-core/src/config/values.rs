//! The vocabulary `sure.yaml` is written in.
//!
//! Every enum here is a closed set of words a user may write in a project
//! configuration file. They are declared with [`variants!`] so that the list a
//! "use one of: ..." message is generated from is the same list the enum
//! defines, rather than a copy that drifts.
//!
//! Two of these types are not parsed from the file at all: [`ProjectRequest`]
//! and [`ScopeReduction`] are *derived* from it, and exist so that a caller
//! cannot inspect the parsed settings without being handed the list of ways the
//! file asks for more authority, or gives up checking.

use serde::{Deserialize, Serialize};
use sure_domain::execution::Permission;
use sure_domain::variants::variants;

/// User-supplied redaction rules.
///
/// These are applied on top of the built-in detectors. They are intended for
/// project-specific secrets that the built-in pattern list does not recognise,
/// such as internal token prefixes or constant values that appear in logs.
///
/// Putting raw secrets in a configuration file is a risk in itself: the file
/// may be checked into version control or shared. This setting exists so that
/// a user-level configuration can name secrets that SURE should mask, not so
/// that secrets should be stored in project files.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RedactionConfig {
    /// Literal strings to redact wherever they appear.
    pub literals: Vec<String>,
    /// Regular expressions whose matches are redacted.
    ///
    /// Each pattern is compiled when the configuration is loaded; an invalid
    /// pattern is a configuration error rather than a runtime panic.
    pub patterns: Vec<String>,
}

/// How much of a project's activity SURE is allowed to keep.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum PrivacyMode {
    /// Source code and evidence stay on this machine. External analysis only
    /// when the user has configured it.
    ///
    /// The default, per `docs/security/PRIVACY.md`. Written here rather than as
    /// a hand-written `Default` impl so that the two cannot disagree.
    #[default]
    #[serde(rename = "local_first")]
    LocalFirst,
    /// Nothing about this project is sent to any external service.
    #[serde(rename = "fully_local")]
    FullyLocal,
    /// External analysis and sync features, explicitly enabled.
    ///
    /// Documented as a future mode; see [`PrivacyMode::is_available`].
    #[serde(rename = "cloud_enhanced")]
    CloudEnhanced,
}

variants!(
    /// Every privacy mode, least exposure first.
    PrivacyMode { LocalFirst, FullyLocal, CloudEnhanced }
);

impl PrivacyMode {
    /// The stable wire name, matching `sure.yaml`'s `privacy.mode`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalFirst => "local_first",
            Self::FullyLocal => "fully_local",
            Self::CloudEnhanced => "cloud_enhanced",
        }
    }

    /// Whether this release implements the mode.
    ///
    /// Accepting a mode SURE cannot honour would let a project file claim a
    /// privacy guarantee that nothing enforces, which is the false-green
    /// failure this product exists to prevent.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::LocalFirst | Self::FullyLocal)
    }

    /// Whether code or evidence may leave this machine under this mode.
    #[must_use]
    pub const fn allows_external_analysis(self) -> bool {
        !matches!(self, Self::FullyLocal)
    }
}

/// How firmly SURE intervenes before a risky action.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum ProtectionMode {
    /// Warn or block clearly dangerous operations, where the integration can
    /// decide before an action happens.
    ///
    /// The default, per `docs/security/PROTECTION_MODE.md`.
    #[default]
    #[serde(rename = "standard")]
    Standard,
    /// Also ask before migrations, CI configuration, secret and config areas,
    /// and broad filesystem changes.
    #[serde(rename = "strict")]
    Strict,
    /// User-authored rules.
    ///
    /// Documented as a user-facing mode, but there is no rule editor in this
    /// release; see [`ProtectionMode::is_available`].
    #[serde(rename = "custom")]
    Custom,
}

variants!(
    /// Every protection mode, least intervention first.
    ProtectionMode { Standard, Strict, Custom }
);

impl ProtectionMode {
    /// The stable wire name, matching `sure.yaml`'s `protection.mode`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Strict => "strict",
            Self::Custom => "custom",
        }
    }

    /// Whether this release implements the mode.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Standard | Self::Strict)
    }
}

/// Which analysis provider, if any, SURE may use.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum AnalysisProvider {
    /// Deterministic checks only. No model is consulted.
    ///
    /// The default: consulting a model is something the user asks for, never
    /// something that happens because nothing said otherwise.
    #[default]
    #[serde(rename = "disabled")]
    Disabled,
    /// A command already installed on this machine.
    #[serde(rename = "local_command")]
    LocalCommand,
    /// The user's own Claude CLI and configuration.
    #[serde(rename = "claude_cli")]
    ClaudeCli,
    /// An OpenAI-compatible endpoint the user already has.
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
}

variants!(
    /// Every provider, most self-contained first.
    AnalysisProvider { Disabled, LocalCommand, ClaudeCli, OpenAiCompatible }
);

impl AnalysisProvider {
    /// The stable wire name, matching `sure.yaml`'s `analysis.provider`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::LocalCommand => "local_command",
            Self::ClaudeCli => "claude_cli",
            Self::OpenAiCompatible => "openai_compatible",
        }
    }

    /// Whether using this provider sends anything to a service outside this
    /// machine.
    ///
    /// [`AnalysisProvider::LocalCommand`] is `false`: the command runs locally,
    /// and whether that command then calls out is outside what SURE can see.
    /// That uncertainty is exactly why `privacy.mode: fully_local` with
    /// `local_command` is allowed while the two named external providers are
    /// not, and why the report must still state which provider was used.
    #[must_use]
    pub const fn is_external(self) -> bool {
        matches!(self, Self::ClaudeCli | Self::OpenAiCompatible)
    }

    /// Whether any model is consulted at all.
    #[must_use]
    pub const fn uses_a_model(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

/// What a project asks SURE to do about one optional check.
///
/// `Always` is a preference about effort, never a grant of authority: a check
/// still needs the execution permission its actions require.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum CheckPreference {
    /// Run it when the discovered project shape suggests it is worth running.
    ///
    /// The default: neither running work the project does not need nor
    /// skipping work it does.
    #[default]
    #[serde(rename = "auto")]
    Auto,
    /// Run it whenever the check is part of the plan.
    #[serde(rename = "always")]
    Always,
    /// Do not run it. The report says so.
    #[serde(rename = "never")]
    Never,
}

variants!(
    /// Every preference, least effort first.
    CheckPreference { Auto, Always, Never }
);

impl CheckPreference {
    /// The stable wire name, matching the `checks` section of `sure.yaml`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    /// Whether this preference stops the check from running.
    #[must_use]
    pub const fn disables(self) -> bool {
        matches!(self, Self::Never)
    }
}

/// How a report is written out.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum ReportFormat {
    /// For a person reading a terminal.
    ///
    /// The default: the report exists to be acted on, and a person is the one
    /// acting on it. `json` is for a caller that has to parse it.
    #[default]
    #[serde(rename = "human")]
    Human,
    /// For a program, with the same content and no decoration.
    #[serde(rename = "json")]
    Json,
}

variants!(ReportFormat { Human, Json });

impl ReportFormat {
    /// The stable wire name, matching `sure.yaml`'s `report.format`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Json => "json",
        }
    }

    /// Whether the output is intended to be parsed rather than read.
    #[must_use]
    pub const fn is_machine_readable(self) -> bool {
        matches!(self, Self::Json)
    }
}

/// A behaviour a project configuration file asks for but cannot grant.
///
/// `docs/architecture/CONFIG_AUTHORITY.md` treats the checked project as
/// untrusted: the same AI whose work is being evaluated may control the file.
/// Every setting that would widen what SURE may do therefore becomes one of
/// these, to be resolved against the user's own configuration by the authority
/// layer (P1-T011) rather than obeyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProjectRequest {
    /// Run the project's own commands, or run checks in a container.
    #[serde(rename = "run_project_code")]
    RunProjectCode,
    /// Install or update the project's dependencies.
    #[serde(rename = "install_dependencies")]
    InstallDependencies,
    /// Reach the network.
    #[serde(rename = "network")]
    Network,
    /// Keep full transcripts rather than the standard record.
    #[serde(rename = "full_recording")]
    FullRecording,
    /// Send usage data anywhere. This release has no telemetry implementation.
    #[serde(rename = "telemetry")]
    Telemetry,
    /// Send project content to a model service.
    #[serde(rename = "external_analysis")]
    ExternalAnalysis,
}

variants!(
    /// Every request, in the order a user should read them.
    ProjectRequest {
        RunProjectCode,
        InstallDependencies,
        Network,
        FullRecording,
        Telemetry,
        ExternalAnalysis
    }
);

impl ProjectRequest {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RunProjectCode => "run_project_code",
            Self::InstallDependencies => "install_dependencies",
            Self::Network => "network",
            Self::FullRecording => "full_recording",
            Self::Telemetry => "telemetry",
            Self::ExternalAnalysis => "external_analysis",
        }
    }

    /// The execution permission that would cover this request, when one does.
    ///
    /// A match is necessary but not sufficient. The user granting
    /// `run_project_code` in their own configuration is not the same as the
    /// user approving *this particular* command, and neither
    /// [`ProjectRequest::FullRecording`] nor [`ProjectRequest::Telemetry`] is
    /// an execution permission at all — recording more is not running more.
    #[must_use]
    pub const fn permission(self) -> Option<Permission> {
        match self {
            Self::RunProjectCode => Some(Permission::RunProjectCode),
            Self::InstallDependencies => Some(Permission::InstallDependencies),
            Self::Network => Some(Permission::Network),
            // Reaching a model service is a connection to a service. It is not a
            // licence to send source code anywhere, which is why the request is
            // still recorded separately.
            Self::ExternalAnalysis => Some(Permission::ConnectService),
            Self::FullRecording | Self::Telemetry => None,
        }
    }

    /// Plain-language description of what is being asked for.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::RunProjectCode => "run this project's own commands on this computer",
            Self::InstallDependencies => "install this project's dependencies",
            Self::Network => "use the internet",
            Self::FullRecording => "keep full transcripts of the session",
            Self::Telemetry => "send usage data to SURE's authors",
            Self::ExternalAnalysis => "send project content to a model service",
        }
    }
}

/// A setting that makes SURE check less than it otherwise would.
///
/// These are allowed — a project may turn off checks it does not want — but
/// they are *collected* so that a report can state them. A report that lists
/// what passed without mentioning that a whole class of check was switched off
/// describes a different run from the one that happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ScopeReduction {
    /// `checks.existing_tests: false`
    #[serde(rename = "existing_tests_disabled")]
    ExistingTestsDisabled,
    /// `checks.start_local_services: never`
    #[serde(rename = "local_services_disabled")]
    LocalServicesDisabled,
    /// `checks.browser_probe: never`
    #[serde(rename = "browser_probe_disabled")]
    BrowserProbeDisabled,
}

variants!(
    /// Every reduction, in report order.
    ScopeReduction { ExistingTestsDisabled, LocalServicesDisabled, BrowserProbeDisabled }
);

impl ScopeReduction {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExistingTestsDisabled => "existing_tests_disabled",
            Self::LocalServicesDisabled => "local_services_disabled",
            Self::BrowserProbeDisabled => "browser_probe_disabled",
        }
    }

    /// Plain-language description for the report's scope section.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::ExistingTestsDisabled => "The project turned off running its own test suite.",
            Self::LocalServicesDisabled => "The project turned off starting its local services.",
            Self::BrowserProbeDisabled => {
                "The project turned off checking the interface in a browser."
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_are_stable_and_unique() {
        // The macro pins each list; this pins the spellings.
        assert_eq!(
            PrivacyMode::ALL
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            vec!["local_first", "fully_local", "cloud_enhanced"]
        );
        assert_eq!(
            ProtectionMode::ALL
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            vec!["standard", "strict", "custom"]
        );
        assert_eq!(
            AnalysisProvider::ALL
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            vec![
                "disabled",
                "local_command",
                "claude_cli",
                "openai_compatible"
            ]
        );
        assert_eq!(
            CheckPreference::ALL
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            vec!["auto", "always", "never"]
        );
        assert_eq!(
            ReportFormat::ALL
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            vec!["human", "json"]
        );
        assert_eq!(
            ProjectRequest::ALL
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            vec![
                "run_project_code",
                "install_dependencies",
                "network",
                "full_recording",
                "telemetry",
                "external_analysis",
            ]
        );
        assert_eq!(
            ScopeReduction::ALL
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            vec![
                "existing_tests_disabled",
                "local_services_disabled",
                "browser_probe_disabled",
            ]
        );
    }

    #[test]
    fn serde_reads_and_writes_exactly_the_wire_names() {
        fn round_trip<T>(values: &[T])
        where
            T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
        {
            for value in values {
                let json = serde_json::to_string(value).expect("serialize");
                let back: T = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(&back, value, "{json} did not round trip");
            }
        }
        round_trip(PrivacyMode::ALL);
        round_trip(ProtectionMode::ALL);
        round_trip(AnalysisProvider::ALL);
        round_trip(CheckPreference::ALL);
        round_trip(ReportFormat::ALL);
        round_trip(ProjectRequest::ALL);
        round_trip(ScopeReduction::ALL);
    }

    #[test]
    fn serde_spells_each_variant_the_way_as_str_does() {
        // `serde(rename_all = "snake_case")` would render `OpenAiCompatible` as
        // `open_ai_compatible`. The renames are explicit so the file a user
        // writes and the word SURE prints are the same word.
        macro_rules! agree {
            ($name:ident) => {
                for value in $name::ALL {
                    assert_eq!(
                        serde_json::to_string(value).expect("serialize"),
                        format!("\"{}\"", value.as_str()),
                        "{}::{:?}",
                        stringify!($name),
                        value
                    );
                }
            };
        }
        agree!(PrivacyMode);
        agree!(ProtectionMode);
        agree!(AnalysisProvider);
        agree!(CheckPreference);
        agree!(ReportFormat);
        agree!(ProjectRequest);
        agree!(ScopeReduction);
    }

    #[test]
    fn defaults_are_the_least_exposed_choice() {
        assert_eq!(PrivacyMode::default(), PrivacyMode::LocalFirst);
        assert_eq!(ProtectionMode::default(), ProtectionMode::Standard);
        assert_eq!(AnalysisProvider::default(), AnalysisProvider::Disabled);
        assert_eq!(CheckPreference::default(), CheckPreference::Auto);
        assert_eq!(ReportFormat::default(), ReportFormat::Human);
        assert!(!AnalysisProvider::default().uses_a_model());
    }

    #[test]
    fn only_the_documented_modes_are_available_in_this_release() {
        assert!(PrivacyMode::LocalFirst.is_available());
        assert!(PrivacyMode::FullyLocal.is_available());
        assert!(
            !PrivacyMode::CloudEnhanced.is_available(),
            "PRIVACY.md describes cloud-enhanced as a future mode; \
             accepting it would claim a guarantee nothing enforces"
        );
        assert!(ProtectionMode::Standard.is_available());
        assert!(ProtectionMode::Strict.is_available());
        assert!(!ProtectionMode::Custom.is_available());
    }

    #[test]
    fn external_analysis_is_named_honestly() {
        assert!(!AnalysisProvider::Disabled.is_external());
        assert!(
            !AnalysisProvider::LocalCommand.is_external(),
            "a local command runs here; whether it calls out is not something SURE can see"
        );
        assert!(AnalysisProvider::ClaudeCli.is_external());
        assert!(AnalysisProvider::OpenAiCompatible.is_external());

        assert!(!PrivacyMode::FullyLocal.allows_external_analysis());
        assert!(PrivacyMode::LocalFirst.allows_external_analysis());
        assert!(PrivacyMode::CloudEnhanced.allows_external_analysis());
    }

    #[test]
    fn only_two_requests_are_execution_permissions() {
        assert_eq!(
            ProjectRequest::Network.permission(),
            Some(Permission::Network)
        );
        assert_eq!(
            ProjectRequest::RunProjectCode.permission(),
            Some(Permission::RunProjectCode)
        );
        assert_eq!(
            ProjectRequest::InstallDependencies.permission(),
            Some(Permission::InstallDependencies)
        );
        assert_eq!(
            ProjectRequest::FullRecording.permission(),
            None,
            "recording more is not running more, so no execution permission covers it"
        );
        assert_eq!(ProjectRequest::Telemetry.permission(), None);
    }

    #[test]
    fn every_request_and_reduction_explains_itself_in_plain_words() {
        for request in ProjectRequest::ALL {
            assert!(!request.plain_description().is_empty());
            assert!(!request.plain_description().contains('_'));
        }
        for reduction in ScopeReduction::ALL {
            assert!(reduction.plain_description().ends_with('.'));
        }
        assert!(ReportFormat::Json.is_machine_readable());
        assert!(!ReportFormat::Human.is_machine_readable());
        assert!(CheckPreference::Never.disables());
        assert!(!CheckPreference::Always.disables());
    }
}

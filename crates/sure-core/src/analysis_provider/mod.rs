//! Runtime abstraction for model-backed or command-backed analysis.
//!
//! Deterministic checks live in [`crate::checks`] and do not use this module.
//! This provider is only consulted when a check explicitly asks for analysis,
//! and the default [`AnalysisProvider`](crate::config::AnalysisProvider) is
//! [`Disabled`](crate::config::AnalysisProvider::Disabled), so a run that never
//! configures a provider never instantiates one.
//!
//! The abstraction is intentionally thin: it is one trait, one request/response
//! pair, and one factory. The check engine stays in [`crate::checks`]; this
//! module only gives that engine a uniform door to call when a check decides it
//! needs model input.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{AnalysisConfig, AnalysisProvider};
use crate::process::{Cancellation, Limits, Outcome, ProcessRequest, Termination};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, NotCheckedReason};

/// How long a local-command analysis may run before it is stopped.
///
/// A minute is long enough for a small wrapper script and short enough that a
/// hanging command does not hold a check indefinitely.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// How many bytes SURE keeps from each stream of a local-command analysis.
const DEFAULT_OUTPUT_BYTES: usize = 64 * 1024;

/// A request for analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisRequest<'a> {
    /// The prompt or question the check wants answered.
    pub prompt: &'a str,
}

/// The response from an analysis provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResponse {
    /// The provider's answer, as text.
    pub text: String,
}

impl AnalysisResponse {
    /// Wrap `text` as a response.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Why analysis could not be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError {
    /// The provider is disabled.
    Disabled,
    /// No command was configured for the local-command provider.
    MissingCommand,
    /// The local command could not be run or did not finish cleanly.
    LocalCommandFailed {
        /// What went wrong, in plain words.
        message: String,
    },
    /// The Claude CLI provider could not run or did not finish cleanly.
    ClaudeCliFailed {
        /// What went wrong, in plain words.
        message: String,
    },
    /// The OpenAI-compatible provider is not implemented in this release.
    OpenAiCompatibleNotImplemented,
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "Analysis is disabled."),
            Self::MissingCommand => write!(
                f,
                "The local-command provider needs a command, but none was configured."
            ),
            Self::LocalCommandFailed { message } => {
                write!(f, "The local command failed: {message}")
            }
            Self::ClaudeCliFailed { message } => {
                write!(f, "The Claude CLI provider failed: {message}")
            }
            Self::OpenAiCompatibleNotImplemented => write!(
                f,
                "The OpenAI-compatible provider is not implemented in this release."
            ),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// Something that can perform model-backed or command-backed analysis.
pub trait Analyzer: Send + Sync {
    /// Perform analysis for `request`.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError::Disabled`] when the provider is disabled, or a
    /// provider-specific error when the provider could not produce an answer.
    fn analyze(&self, request: AnalysisRequest<'_>) -> Result<AnalysisResponse, AnalysisError>;
}

/// Provider that refuses every request.
///
/// This is the runtime partner of
/// [`AnalysisProvider::Disabled`](crate::config::AnalysisProvider::Disabled).
#[derive(Debug, Clone, Copy, Default)]
pub struct DisabledAnalyzer;

impl Analyzer for DisabledAnalyzer {
    fn analyze(&self, _request: AnalysisRequest<'_>) -> Result<AnalysisResponse, AnalysisError> {
        Err(AnalysisError::Disabled)
    }
}

/// Provider that runs a configured local command and returns its output.
///
/// The configured command is run in the project root. The prompt from the
/// request is appended as the final argument, so a wrapper script can read it
/// without any shell being involved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCommandAnalyzer {
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
}

impl LocalCommandAnalyzer {
    /// Build a provider that runs `command` in `working_directory`.
    ///
    /// `command` must contain at least one element: the program to run. The
    /// remaining elements are passed as arguments, in order, before the prompt.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError::MissingCommand`] when `command` is empty.
    #[must_use]
    pub fn new(command: Vec<String>, working_directory: impl Into<PathBuf>) -> Self {
        let mut parts = command.into_iter();
        let program = parts.next().map_or_else(OsString::new, Into::into);
        let arguments: Vec<OsString> = parts.map(Into::into).collect();
        Self {
            program,
            arguments,
            working_directory: working_directory.into(),
        }
    }
}

impl Analyzer for LocalCommandAnalyzer {
    fn analyze(&self, request: AnalysisRequest<'_>) -> Result<AnalysisResponse, AnalysisError> {
        let cancellation = Cancellation::new();
        let limits = Limits::new(DEFAULT_TIMEOUT, DEFAULT_OUTPUT_BYTES, DEFAULT_OUTPUT_BYTES);

        let mut arguments = self.arguments.clone();
        arguments.push(prepare_prompt(request.prompt).into());

        let process_request =
            ProcessRequest::new(&self.program, &self.working_directory, limits, cancellation)
                .with_arguments(arguments);

        let outcome = crate::process::run(&process_request).map_err(|error| {
            AnalysisError::LocalCommandFailed {
                message: error.to_string(),
            }
        })?;

        local_command_response(outcome)
    }
}

/// Turn a finished local command into an analysis response or error.
fn local_command_response(outcome: Outcome) -> Result<AnalysisResponse, AnalysisError> {
    match outcome.termination() {
        Termination::Exited { code: Some(0) } => Ok(AnalysisResponse::new(crate::redact::redact(
            &outcome.stdout().text_lossy(),
        ))),
        Termination::Exited { code: Some(code) } => {
            let stderr = crate::redact::redact(&outcome.stderr().text_lossy());
            Err(AnalysisError::LocalCommandFailed {
                message: format!("exited with code {code}: {stderr}"),
            })
        }
        Termination::Exited { code: None } => Err(AnalysisError::LocalCommandFailed {
            message: "exited without a code".to_owned(),
        }),
        Termination::TimedOut { .. } => Err(AnalysisError::LocalCommandFailed {
            message: "timed out".to_owned(),
        }),
        Termination::Cancelled { .. } => Err(AnalysisError::LocalCommandFailed {
            message: "was cancelled".to_owned(),
        }),
        Termination::CancelledBeforeStart => Err(AnalysisError::LocalCommandFailed {
            message: "was cancelled before it started".to_owned(),
        }),
    }
}

/// Strip secrets from a prompt before it is handed to a provider.
///
/// External providers must not receive raw secrets, and a local command may log
/// its arguments. Redacting here keeps the secret out of both places.
fn prepare_prompt(prompt: &str) -> String {
    crate::redact::redact(prompt)
}

/// Provider that invokes the user-installed Anthropic Claude CLI.
///
/// The provider runs `claude -p <prompt>` in the project root. The prompt is
/// redacted before it leaves SURE, and both stdout and stderr are bounded so
/// a runaway CLI cannot fill memory. The returned text is a model assessment,
/// not deterministic evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeCliAnalyzer {
    program: OsString,
    working_directory: PathBuf,
}

impl ClaudeCliAnalyzer {
    /// Build a provider that runs `claude` in `working_directory`.
    #[must_use]
    pub fn new(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            program: "claude".into(),
            working_directory: working_directory.into(),
        }
    }

    #[cfg(test)]
    fn with_program(program: impl Into<OsString>, working_directory: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            working_directory: working_directory.into(),
        }
    }
}

impl Analyzer for ClaudeCliAnalyzer {
    fn analyze(&self, request: AnalysisRequest<'_>) -> Result<AnalysisResponse, AnalysisError> {
        let cancellation = Cancellation::new();
        let limits = Limits::new(DEFAULT_TIMEOUT, DEFAULT_OUTPUT_BYTES, DEFAULT_OUTPUT_BYTES);

        let prompt = prepare_prompt(request.prompt);
        let arguments: Vec<OsString> = vec!["-p".into(), prompt.into()];

        let process_request =
            ProcessRequest::new(&self.program, &self.working_directory, limits, cancellation)
                .with_arguments(arguments);

        let outcome = crate::process::run(&process_request).map_err(|error| {
            AnalysisError::ClaudeCliFailed {
                message: error.to_string(),
            }
        })?;

        claude_cli_response(outcome)
    }
}

/// Turn a finished Claude CLI run into an analysis response or error.
fn claude_cli_response(outcome: Outcome) -> Result<AnalysisResponse, AnalysisError> {
    match outcome.termination() {
        Termination::Exited { code: Some(0) } => {
            Ok(AnalysisResponse::new(outcome.stdout().text_lossy()))
        }
        Termination::Exited { code: Some(code) } => {
            let stderr = outcome.stderr().text_lossy();
            Err(AnalysisError::ClaudeCliFailed {
                message: format!("exited with code {code}: {stderr}"),
            })
        }
        Termination::Exited { code: None } => Err(AnalysisError::ClaudeCliFailed {
            message: "exited without a code".to_owned(),
        }),
        Termination::TimedOut { .. } => Err(AnalysisError::ClaudeCliFailed {
            message: "timed out".to_owned(),
        }),
        Termination::Cancelled { .. } => Err(AnalysisError::ClaudeCliFailed {
            message: "was cancelled".to_owned(),
        }),
        Termination::CancelledBeforeStart => Err(AnalysisError::ClaudeCliFailed {
            message: "was cancelled before it started".to_owned(),
        }),
    }
}

/// Placeholder for the OpenAI-compatible provider.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenAiCompatibleAnalyzer {
    /// The endpoint the config named, kept for the real implementation.
    #[allow(dead_code)]
    endpoint: String,
    /// The model the config named, kept for the real implementation.
    #[allow(dead_code)]
    model: Option<String>,
}

impl OpenAiCompatibleAnalyzer {
    /// Build a placeholder from the endpoint and model the user configured.
    #[must_use]
    pub fn new(endpoint: String, model: Option<String>) -> Self {
        Self { endpoint, model }
    }
}

impl Analyzer for OpenAiCompatibleAnalyzer {
    fn analyze(&self, _request: AnalysisRequest<'_>) -> Result<AnalysisResponse, AnalysisError> {
        Err(AnalysisError::OpenAiCompatibleNotImplemented)
    }
}

/// Build the analyzer described by `config`.
///
/// `project_root` is the working directory for the local-command provider.
///
/// # Errors
///
/// Returns [`AnalysisError::MissingCommand`] when the local-command provider is
/// chosen but no command is configured.
pub fn build(
    config: &AnalysisConfig,
    project_root: &Path,
) -> Result<Box<dyn Analyzer>, AnalysisError> {
    match config.provider {
        AnalysisProvider::Disabled => Ok(Box::new(DisabledAnalyzer)),
        AnalysisProvider::LocalCommand => {
            let Some(command) = &config.command else {
                return Err(AnalysisError::MissingCommand);
            };
            Ok(Box::new(LocalCommandAnalyzer::new(
                command.clone(),
                project_root,
            )))
        }
        AnalysisProvider::ClaudeCli => Ok(Box::new(ClaudeCliAnalyzer::new(project_root))),
        AnalysisProvider::OpenAiCompatible => Ok(Box::new(OpenAiCompatibleAnalyzer::new(
            config.endpoint.clone().unwrap_or_default(),
            config.model.clone(),
        ))),
    }
}

/// Build a [`CheckResult`] for a check that could not run because analysis is disabled.
///
/// This is the honest way to record a model-backed check when the configured
/// provider is [`AnalysisProvider::Disabled`]: the check is skipped, the reason
/// says the provider is missing, and the result blocks green when the check was
/// critical. The check engine stays independent of the provider — it only calls
/// this helper after the provider itself returns [`AnalysisError::Disabled`].
#[must_use]
pub fn disabled_result(
    id: CheckId,
    title: impl Into<String>,
    severity: Severity,
    critical: bool,
    fingerprint: FingerprintId,
) -> CheckResult {
    CheckResult::not_run(
        id,
        title,
        severity,
        critical,
        NotCheckedReason::AnalysisProviderDisabled,
        fingerprint,
    )
    .with_reason("No analysis provider is configured.".to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::AnalysisConfig;

    fn request(prompt: &str) -> AnalysisRequest<'_> {
        AnalysisRequest { prompt }
    }

    /// A command that echoes its last argument, portable across Windows and Unix.
    fn echo_command() -> Vec<String> {
        if cfg!(windows) {
            vec!["cmd".to_owned(), "/c".to_owned(), "echo".to_owned()]
        } else {
            vec!["echo".to_owned()]
        }
    }

    /// A command that exits with a non-zero code, portable across Windows and Unix.
    fn failing_command() -> Vec<String> {
        if cfg!(windows) {
            vec![
                "cmd".to_owned(),
                "/c".to_owned(),
                "exit".to_owned(),
                "7".to_owned(),
            ]
        } else {
            vec!["false".to_owned()]
        }
    }

    #[test]
    fn disabled_analyzer_refuses_every_request() {
        let analyzer = DisabledAnalyzer;
        let error = analyzer.analyze(request("anything")).unwrap_err();
        assert_eq!(error, AnalysisError::Disabled);
        assert!(error.to_string().contains("disabled"));
    }

    #[test]
    fn local_command_analyzer_runs_command_and_returns_stdout() {
        let temp = std::env::temp_dir();
        let analyzer = LocalCommandAnalyzer::new(echo_command(), &temp);

        let response = analyzer.analyze(request("hello from sure")).unwrap();

        assert!(response.text.contains("hello from sure"));
    }

    #[test]
    fn local_command_analyzer_reports_non_zero_exit_as_failure() {
        let temp = std::env::temp_dir();
        let analyzer = LocalCommandAnalyzer::new(failing_command(), &temp);

        let error = analyzer.analyze(request("ignored")).unwrap_err();
        assert!(matches!(error, AnalysisError::LocalCommandFailed { .. }));
        let message = error.to_string();
        assert!(
            message.contains("exited") || message.contains("failed"),
            "{message}"
        );
    }

    fn require_analyzer(result: Result<Box<dyn Analyzer>, AnalysisError>) -> Box<dyn Analyzer> {
        match result {
            Ok(analyzer) => analyzer,
            Err(error) => panic!("expected an analyzer, got {error}"),
        }
    }

    fn require_error(result: Result<Box<dyn Analyzer>, AnalysisError>) -> AnalysisError {
        match result {
            Err(error) => error,
            Ok(_) => panic!("expected an error, got an analyzer"),
        }
    }

    fn temp_root() -> PathBuf {
        std::env::temp_dir()
    }

    #[test]
    fn build_dispatches_to_disabled() {
        let config = AnalysisConfig {
            provider: AnalysisProvider::Disabled,
            ..AnalysisConfig::default()
        };
        let root = temp_root();
        let analyzer = require_analyzer(build(&config, &root));
        let error = analyzer.analyze(request("x")).unwrap_err();
        assert_eq!(error, AnalysisError::Disabled);
    }

    #[test]
    fn build_dispatches_to_local_command() {
        let config = AnalysisConfig {
            provider: AnalysisProvider::LocalCommand,
            command: Some(echo_command()),
            ..AnalysisConfig::default()
        };
        let root = temp_root();
        let analyzer = require_analyzer(build(&config, &root));
        let response = analyzer.analyze(request(" dispatched ")).unwrap();
        assert!(response.text.contains(" dispatched "));
    }

    #[test]
    fn build_local_command_without_command_is_missing_command() {
        let config = AnalysisConfig {
            provider: AnalysisProvider::LocalCommand,
            command: None,
            ..AnalysisConfig::default()
        };
        let root = temp_root();
        let error = require_error(build(&config, &root));
        assert_eq!(error, AnalysisError::MissingCommand);
    }

    #[test]
    fn build_dispatches_to_claude_cli() {
        let config = AnalysisConfig {
            provider: AnalysisProvider::ClaudeCli,
            ..AnalysisConfig::default()
        };
        let root = temp_root();
        let analyzer = require_analyzer(build(&config, &root));
        // The default program is `claude`, which is not expected to be installed
        // in test environments. The analyzer must report a provider failure, not
        // a placeholder.
        let error = analyzer.analyze(request("x")).unwrap_err();
        assert!(matches!(error, AnalysisError::ClaudeCliFailed { .. }));
        let message = error.to_string();
        assert!(
            message.contains("failed") || message.contains("claude"),
            "{message}"
        );
    }

    #[test]
    fn claude_cli_analyzer_runs_program_and_returns_stdout() {
        let temp = std::env::temp_dir();
        let analyzer = ClaudeCliAnalyzer::with_program("echo", &temp);

        let response = analyzer.analyze(request("hello from claude")).unwrap();

        assert!(response.text.contains("hello from claude"));
    }

    #[test]
    fn claude_cli_analyzer_redacts_prompt_before_passing_it() {
        let temp = std::env::temp_dir();
        let analyzer = ClaudeCliAnalyzer::with_program("echo", &temp);

        let response = analyzer
            .analyze(request("token sk-abcdefghijklmnopqrstuvwxyz01"))
            .unwrap();

        assert!(!response.text.contains("sk-abcdefghijklmnopqrstuvwxyz01"));
        assert!(response.text.contains("***"));
    }

    #[test]
    fn build_dispatches_to_openai_compatible_placeholder() {
        let config = AnalysisConfig {
            provider: AnalysisProvider::OpenAiCompatible,
            endpoint: Some("https://models.example.com/v1".to_owned()),
            model: Some("test-model".to_owned()),
            ..AnalysisConfig::default()
        };
        let root = temp_root();
        let analyzer = require_analyzer(build(&config, &root));
        let error = analyzer.analyze(request("x")).unwrap_err();
        assert_eq!(error, AnalysisError::OpenAiCompatibleNotImplemented);
    }

    #[test]
    fn deterministic_checks_need_no_provider() {
        // The whole point of the abstraction: a check that does not ask for
        // analysis never builds a provider. This test proves the default config
        // builds the disabled provider and that provider is safe to ignore.
        let config = AnalysisConfig::default();
        assert_eq!(config.provider, AnalysisProvider::Disabled);
        let root = temp_root();
        let analyzer = require_analyzer(build(&config, &root));
        assert_eq!(
            analyzer.analyze(request("x")).unwrap_err(),
            AnalysisError::Disabled
        );
    }

    #[test]
    fn disabled_result_records_analysis_provider_disabled_reason() {
        let fp = FingerprintId::generate();
        let id = CheckId::generate();
        let result = disabled_result(
            id.clone(),
            "semantic intent match",
            Severity::ShouldFixFirst,
            true,
            fp.clone(),
        );

        assert_eq!(result.id, id);
        assert_eq!(result.title, "semantic intent match");
        assert_eq!(result.status, sure_domain::status::CheckStatus::Skipped);
        assert_eq!(
            result.not_checked_reason,
            Some(NotCheckedReason::AnalysisProviderDisabled)
        );
        assert!(
            result.reason.contains("analysis provider"),
            "reason should explain the provider is missing: {}",
            result.reason
        );
        assert!(
            result.blocks_green(),
            "a critical model-backed check that could not run must block green"
        );
    }
}

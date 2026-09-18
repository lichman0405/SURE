//! `sure hook ingest`, and what it does in this build.
//!
//! # Why this is a module rather than an arm of `commands.rs`
//!
//! The same reason `crate::check` is one: [`Command::report`] is a match from a
//! command to a value, and the looking happens here so the match stays readable.
//!
//! # What the hook does
//!
//! 1. Read the raw JSON event from stdin.
//! 2. Normalise it with the harness-specific normaliser.
//! 3. Validate it via the existing ingestion path.
//! 4. Persist it via the existing session event store.
//! 5. For `pre-tool-use` events, evaluate a protection decision using the
//!    existing execution-safety machinery and return it as a structured JSON
//!    response.

use std::io::{self, Read};
use std::path::Path;

use sure_core::config::Config;
use sure_core::execution::{ExecutionMode, ExecutionPermissions};
use sure_core::fingerprint::{FingerprintOptions, project_fingerprint};
use sure_core::harness_event::ingest_event_str;
use sure_core::hook_protection::{decide_claude_code_tool, decide_cursor_tool};
use sure_core::ids::EventId;
use sure_core::normalizer::{claude_code, cursor};
use sure_core::paths::Paths;
use sure_core::session_event_store::SessionEventStore;
use sure_core::store::Store;

use crate::cli::HookAction;
use crate::report::{Failed, Report};

/// Run a hook action and produce a report.
#[must_use]
pub fn run(action: &HookAction) -> Report {
    match action {
        HookAction::Ingest { source, event_kind } => {
            let mut stdin = String::new();
            if let Err(error) = io::stdin().read_to_string(&mut stdin) {
                return failed("SURE could not read standard input.", error.to_string());
            }

            if stdin.trim().is_empty() {
                return failed(
                    "No event was read from standard input.",
                    "The harness did not provide an event.".to_owned(),
                );
            }

            run_ingest(source.as_deref(), event_kind.as_deref(), &stdin)
        }
    }
}

fn run_ingest(source: Option<&str>, event_kind: Option<&str>, stdin: &str) -> Report {
    // Normalise based on source.
    let envelope = match source {
        Some("cursor") => match cursor::normalize(stdin) {
            Ok(envelope) => envelope,
            Err(error) => return failed("The event could not be normalised.", error.to_string()),
        },
        Some("claude-code") => match claude_code::normalize(stdin) {
            Ok(envelope) => envelope,
            Err(error) => return failed("The event could not be normalised.", error.to_string()),
        },
        Some(other) => {
            return failed(
                "The event source is not supported.",
                format!("Source '{other}' is not one SURE knows how to ingest."),
            );
        }
        None => {
            return failed(
                "No source was given.",
                "Use --source <name> to tell SURE which harness sent the event.".to_owned(),
            );
        }
    };

    // Validate via the existing ingestion path.
    let json = match envelope.to_json() {
        Ok(json) => json,
        Err(error) => {
            return failed(
                "The normalised event could not be serialised.",
                error.to_string(),
            );
        }
    };

    let ingested = match ingest_event_str(&json) {
        Ok(ingested) => ingested,
        Err(error) => return failed("The event did not pass validation.", error.to_string()),
    };

    // Determine project root for store and config.
    let project_root = envelope.project_root.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("."))
    });

    // Best-effort persistence. The harness cares most about the decision for
    // pre-tool-use; a store failure should not block the operation.
    let _ = persist_event(&ingested, &project_root);

    // Evaluate protection for pre-tool-use events.
    let is_pre_tool_use =
        event_kind == Some("pre-tool-use") || envelope.event_type == "tool.requested";

    if is_pre_tool_use {
        let tool = envelope
            .payload
            .get("tool")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let (mode, permissions) = load_execution_config(&project_root);

        let decision = match source {
            Some("claude-code") => decide_claude_code_tool(tool, mode, &permissions),
            _ => decide_cursor_tool(tool, mode, &permissions),
        };
        return Report::HookDecision(decision);
    }

    // For non-pre-tool-use events, report success.
    Report::HookDecision(sure_core::hook_protection::ProtectionDecision::allow())
}

fn persist_event(
    ingested: &sure_core::harness_event::IngestedEvent,
    project_root: &str,
) -> Result<(), String> {
    let paths = Paths::discover().map_err(|e| e.to_string())?;
    let project_path = Path::new(project_root);
    let store = Store::open(&paths, project_path).map_err(|e| e.to_string())?;
    let session_store = SessionEventStore::new(&store);

    let fingerprint = match project_fingerprint(project_path, &FingerprintOptions::default()) {
        Ok(fp) => fp,
        Err(e) => return Err(e.to_string()),
    };

    let event_id = EventId::generate();

    session_store
        .persist(ingested, project_root, &fingerprint.id, &event_id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn load_execution_config(project_root: &str) -> (ExecutionMode, ExecutionPermissions) {
    let path = Path::new(project_root);
    match Config::load(path) {
        Ok(loaded) => {
            let config = loaded.config;
            let mut permissions = ExecutionPermissions::inspect_only();
            permissions.run_project_code = config.execution.mode.runs_project_code();
            permissions.install_dependencies = config.execution.allow_dependency_install;
            permissions.network = config.execution.allow_network;
            (config.execution.mode, permissions)
        }
        Err(_) => {
            // Invalid config file: fall back to the safest defaults.
            (
                ExecutionMode::InspectOnly,
                ExecutionPermissions::inspect_only(),
            )
        }
    }
}

fn failed(what: &'static str, detail: String) -> Report {
    Report::Failed(Box::new(Failed {
        command: "hook",
        what,
        detail,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_testkit::repository_root;

    fn fixture(name: &str) -> String {
        let path = repository_root()
            .join("integrations")
            .join("cursor")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    fn claude_fixture(name: &str) -> String {
        let path = repository_root()
            .join("integrations")
            .join("claude-code")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    #[test]
    fn cursor_pre_tool_use_returns_decision() {
        let text = fixture("pre-tool-use.json");
        let report = run_ingest(Some("cursor"), Some("pre-tool-use"), &text);
        let decision = match &report {
            Report::HookDecision(d) => d,
            other => panic!("expected HookDecision, got {other:?}"),
        };
        // Default config is inspect_only, so Shell (ArbitraryCommand) is blocked.
        assert_eq!(
            decision.decision,
            sure_core::hook_protection::ProtectionDecisionKind::Block
        );
        assert!(decision.reason.is_some());
    }

    #[test]
    fn cursor_session_start_allows_without_decision() {
        let text = fixture("session-start.json");
        let report = run_ingest(Some("cursor"), Some("session-start"), &text);
        let decision = match &report {
            Report::HookDecision(d) => d,
            other => panic!("expected HookDecision, got {other:?}"),
        };
        assert_eq!(
            decision.decision,
            sure_core::hook_protection::ProtectionDecisionKind::Allow
        );
    }

    #[test]
    fn missing_source_fails() {
        let report = run_ingest(None, Some("pre-tool-use"), "{}");
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn unsupported_source_fails() {
        let report = run_ingest(Some("unknown"), Some("pre-tool-use"), "{}");
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn invalid_json_fails() {
        let report = run_ingest(Some("cursor"), Some("pre-tool-use"), "{not json");
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn empty_stdin_fails() {
        let report = run_ingest(Some("cursor"), Some("pre-tool-use"), "");
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn claude_code_session_start_allows_without_decision() {
        let text = claude_fixture("session-start.json");
        let report = run_ingest(Some("claude-code"), Some("session-start"), &text);
        let decision = match &report {
            Report::HookDecision(d) => d,
            other => panic!("expected HookDecision, got {other:?}"),
        };
        assert_eq!(
            decision.decision,
            sure_core::hook_protection::ProtectionDecisionKind::Allow
        );
    }

    #[test]
    fn claude_code_pre_tool_use_uses_protection_decision() {
        let text = claude_fixture("pre-tool-use.json");
        let report = run_ingest(Some("claude-code"), Some("pre-tool-use"), &text);
        let decision = match &report {
            Report::HookDecision(d) => d,
            other => panic!("expected HookDecision, got {other:?}"),
        };
        // Default config is InspectOnly; Bash maps to ArbitraryCommand, which is
        // denied in InspectOnly.
        assert_eq!(
            decision.decision,
            sure_core::hook_protection::ProtectionDecisionKind::Block
        );
        assert!(
            decision
                .reason
                .as_deref()
                .expect("blocked decision should have a reason")
                .contains("execution mode")
        );
    }

    #[test]
    fn claude_code_invalid_json_fails() {
        let report = run_ingest(Some("claude-code"), Some("pre-tool-use"), "{not json");
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }
}

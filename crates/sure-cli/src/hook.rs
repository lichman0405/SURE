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
use sure_core::full_recording::{self, FullRecordingConsent};
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
    let paths = match Paths::discover() {
        Ok(p) => p,
        Err(error) => {
            return failed(
                "SURE could not discover its data directories.",
                error.to_string(),
            );
        }
    };
    run_ingest_with_paths(source, event_kind, stdin, &paths)
}

fn run_ingest_with_paths(
    source: Option<&str>,
    event_kind: Option<&str>,
    stdin: &str,
    paths: &Paths,
) -> Report {
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
    let event_id = EventId::generate();
    let _ = persist_event_with_paths(&ingested, &project_root, &event_id, paths);

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

fn persist_event_with_paths(
    ingested: &sure_core::harness_event::IngestedEvent,
    project_root: &str,
    event_id: &EventId,
    paths: &Paths,
) -> Result<sure_core::ids::FingerprintId, String> {
    let project_path = Path::new(project_root);
    let store = Store::open(paths, project_path).map_err(|e| e.to_string())?;
    let session_store = SessionEventStore::new(&store);

    let fingerprint = match project_fingerprint(project_path, &FingerprintOptions::default()) {
        Ok(fp) => fp,
        Err(e) => return Err(e.to_string()),
    };

    session_store
        .persist(ingested, project_root, &fingerprint.id, event_id)
        .map_err(|e| e.to_string())?;

    // Best-effort full recording. Failure must not affect the hook decision.
    let consent = match Config::load(project_path) {
        Ok(loaded) if loaded.config.privacy.full_recording => FullRecordingConsent::Full,
        _ => FullRecordingConsent::ProjectionOnly,
    };
    let _ = full_recording::persist_full_recording(
        &store,
        ingested,
        event_id,
        project_root,
        &fingerprint.id,
        consent,
    );

    Ok(fingerprint.id)
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

    // --- full recording opt-in integration tests --------------------------------

    use sure_core::full_recording::full_recordings_for_project;
    use sure_protocol::event::EventEnvelope;

    fn scratch_hook_dir(name: &str) -> std::path::PathBuf {
        repository_root()
            .join("target")
            .join("tmp")
            .join(name)
            .join(format!("{}", std::process::id()))
    }

    fn make_claude_event(
        project_root: &std::path::Path,
    ) -> sure_core::harness_event::IngestedEvent {
        let envelope = EventEnvelope::new("claude-code", "tool.completed", "2026-09-18T12:00:00Z")
            .with_capability_tier(sure_core::capability::CapabilityTier::Observed)
            .with_session_id("test-session")
            .with_project_root(project_root.to_string_lossy().into_owned())
            .with_payload(serde_json::json!({"tool": "Bash", "output": "hello"}));

        ingest_event_str(&envelope.to_json().expect("envelope serialises")).expect("event ingests")
    }

    #[test]
    fn full_recording_is_stored_when_opted_in() {
        let tmp = scratch_hook_dir("hook-full-recording-opted-in");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");

        let project = tmp.join("project");
        std::fs::create_dir_all(&project).expect("create project");
        std::fs::write(
            project.join("sure.yaml"),
            "privacy:\n  full_recording: true\n",
        )
        .expect("write config");

        let data = tmp.join("data");
        let config = tmp.join("config");
        let paths = Paths::from_roots(data, config).expect("paths are valid");

        let ingested = make_claude_event(&project);
        let event_id = EventId::generate();

        let fingerprint =
            persist_event_with_paths(&ingested, &project.to_string_lossy(), &event_id, &paths)
                .expect("persist succeeds");

        let store = Store::open_at(&paths.store_file()).expect("store opens");
        let recordings = full_recordings_for_project(&store, &fingerprint, 10).expect("query");

        assert_eq!(recordings.len(), 1, "one full recording should exist");
        assert_eq!(recordings[0].event_type, "tool.completed");
        assert_eq!(recordings[0].source, "claude-code");
        assert_eq!(recordings[0].event_id, Some(event_id.as_str().to_owned()));
    }

    #[test]
    fn full_recording_is_not_stored_when_not_opted_in() {
        let tmp = scratch_hook_dir("hook-full-recording-opted-out");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");

        let project = tmp.join("project");
        std::fs::create_dir_all(&project).expect("create project");
        std::fs::write(
            project.join("sure.yaml"),
            "privacy:\n  full_recording: false\n",
        )
        .expect("write config");

        let data = tmp.join("data");
        let config = tmp.join("config");
        let paths = Paths::from_roots(data, config).expect("paths are valid");

        let ingested = make_claude_event(&project);
        let event_id = EventId::generate();

        let fingerprint =
            persist_event_with_paths(&ingested, &project.to_string_lossy(), &event_id, &paths)
                .expect("persist succeeds");

        let store = Store::open_at(&paths.store_file()).expect("store opens");
        let recordings = full_recordings_for_project(&store, &fingerprint, 10).expect("query");

        assert!(
            recordings.is_empty(),
            "no full recording should exist when opt-in is false"
        );
    }

    #[test]
    fn claude_code_check_repair_recheck_round_trip() {
        let tmp = scratch_hook_dir("claude-e2e-check-repair-recheck");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");

        let project = tmp.join("project");
        std::fs::create_dir_all(&project).expect("create project");

        let data = tmp.join("data");
        let config = tmp.join("config");
        let paths = Paths::from_roots(data, config).expect("paths are valid");

        let project_root = project.to_string_lossy().into_owned();
        let session_id = "claude-e2e-session";

        let session_start = serde_json::json!({
            "event": "SessionStart",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "timestamp_utc": "2026-09-18T12:00:00Z",
            "source": "claude-code",
        })
        .to_string();

        let check_pre = serde_json::json!({
            "event": "PreToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Bash",
            "args": {"command": "npm test", "workdir": &project_root},
            "timestamp_utc": "2026-09-18T12:01:00Z",
            "source": "claude-code",
        })
        .to_string();

        let check_post = serde_json::json!({
            "event": "PostToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Bash",
            "args": {"command": "npm test", "workdir": &project_root},
            "error": {"exit_code": 1, "message": "tests failed"},
            "timestamp_utc": "2026-09-18T12:01:05Z",
            "source": "claude-code",
        })
        .to_string();

        let repair_pre = serde_json::json!({
            "event": "PreToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Edit",
            "args": {"old_string": "bad", "new_string": "good"},
            "path": "src/lib.rs",
            "timestamp_utc": "2026-09-18T12:02:00Z",
            "source": "claude-code",
        })
        .to_string();

        let repair_post = serde_json::json!({
            "event": "PostToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Edit",
            "args": {"old_string": "bad", "new_string": "good"},
            "path": "src/lib.rs",
            "result": {"ok": true},
            "timestamp_utc": "2026-09-18T12:02:05Z",
            "source": "claude-code",
        })
        .to_string();

        let recheck_pre = serde_json::json!({
            "event": "PreToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Bash",
            "args": {"command": "npm test", "workdir": &project_root},
            "timestamp_utc": "2026-09-18T12:03:00Z",
            "source": "claude-code",
        })
        .to_string();

        let recheck_post = serde_json::json!({
            "event": "PostToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Bash",
            "args": {"command": "npm test", "workdir": &project_root},
            "result": {"exit_code": 0, "stdout": "Tests: 5 passed, 5 total", "stderr": ""},
            "timestamp_utc": "2026-09-18T12:03:05Z",
            "source": "claude-code",
        })
        .to_string();

        let stop = serde_json::json!({
            "event": "Stop",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "timestamp_utc": "2026-09-18T12:04:00Z",
            "source": "claude-code",
        })
        .to_string();

        let events = [
            ("session-start", session_start),
            ("pre-tool-use", check_pre),
            ("post-tool-use", check_post),
            ("pre-tool-use", repair_pre),
            ("post-tool-use", repair_post),
            ("pre-tool-use", recheck_pre),
            ("post-tool-use", recheck_post),
            ("stop", stop),
        ];

        for (kind, json) in &events {
            let report = run_ingest_with_paths(Some("claude-code"), Some(kind), json, &paths);
            if *kind == "pre-tool-use" {
                let decision = match &report {
                    Report::HookDecision(d) => d,
                    other => panic!("expected HookDecision for {kind}, got {other:?}"),
                };
                assert_eq!(
                    decision.decision,
                    sure_core::hook_protection::ProtectionDecisionKind::Block,
                    "pre-tool-use should be blocked in InspectOnly mode"
                );
                assert!(
                    decision.reason.is_some(),
                    "blocked decision should have a reason"
                );
            } else {
                let decision = match &report {
                    Report::HookDecision(d) => d,
                    other => panic!("expected HookDecision for {kind}, got {other:?}"),
                };
                assert_eq!(
                    decision.decision,
                    sure_core::hook_protection::ProtectionDecisionKind::Allow,
                    "non-pre-tool-use should be allowed"
                );
            }
        }

        // Open the store and verify persisted events.
        let store = Store::open(&paths, &project).expect("store opens");
        let session_store = SessionEventStore::new(&store);

        let sessions = session_store
            .sessions_past_retention(i64::MAX)
            .expect("query sessions");
        assert_eq!(sessions.len(), 1, "exactly one session should exist");
        assert_eq!(
            sessions[0].harness_session_id.as_deref(),
            Some(session_id),
            "session id should match"
        );

        let stored_events = session_store
            .events_for_session(sessions[0].row_id)
            .expect("query events");
        assert_eq!(stored_events.len(), 8, "all 8 events should be stored");

        let expected_types = [
            "session.started",
            "tool.requested",
            "tool.completed",
            "tool.requested",
            "tool.completed",
            "tool.requested",
            "tool.completed",
            "session.stopped",
        ];

        // events_for_session returns newest first, so reverse to match insertion order.
        let mut ordered_events = stored_events;
        ordered_events.reverse();

        for (i, expected) in expected_types.iter().enumerate() {
            assert_eq!(
                ordered_events[i].event_type, *expected,
                "event {i} should have type {expected}"
            );
        }

        // Verify no full recordings were stored (default is off).
        let fingerprint =
            project_fingerprint(&project, &FingerprintOptions::default()).expect("fingerprint");
        let recordings =
            full_recordings_for_project(&store, &fingerprint.id, 10).expect("query recordings");
        assert!(
            recordings.is_empty(),
            "no full recording should exist when opt-in is false (default)"
        );
    }
}

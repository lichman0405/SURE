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

use sure_core::config::Authority;
use sure_core::config::Config;
use sure_core::execution::{ExecutionMode, ExecutionPermissions};
use sure_core::fingerprint::{FingerprintOptions, project_fingerprint};
use sure_core::full_recording::{
    self, DEFAULT_FULL_RECORDING_RETENTION_DAYS, FullRecordingConsent,
};
use sure_core::harness_event::ingest_event_str;
use sure_core::hook_protection::{decide_claude_code_tool, decide_cursor_tool};
use sure_core::ids::EventId;
use sure_core::normalizer::{claude_code, codex, cursor};
use sure_core::paths::Paths;
use sure_core::session_event_store::SessionEventStore;
use sure_core::store::Store;

use crate::cli::HookAction;
use crate::report::{Failed, Report};

/// Run a hook action and produce a report.
///
/// `store` is the store directory the caller named on the command line, or
/// `None` for the platform's own per-user location — see
/// [`sure_core::paths::Paths::discover_at`]. A harness launcher that wants a
/// hook's events to land somewhere other than the user's own store names it
/// there, in the command it runs; nothing in the project can.
#[must_use]
pub fn run(action: &HookAction, store: Option<&Path>) -> Report {
    match action {
        HookAction::Ingest { source, event_kind } => {
            let mut stdin = String::new();
            if let Err(error) = io::stdin().read_to_string(&mut stdin) {
                return failed("SURE could not read standard input.", error.to_string());
            }

            let event_text = without_byte_order_mark(&stdin);
            if event_text.trim().is_empty() {
                return failed(
                    "No event was read from standard input.",
                    "The harness did not provide an event.".to_owned(),
                );
            }

            run_ingest(source.as_deref(), event_kind.as_deref(), event_text, store)
        }
    }
}

/// The event text as SURE should read it off standard input.
///
/// Windows PowerShell 5.1 writes a UTF-8 byte-order mark in front of every
/// payload it pipes to a native command, whatever `$OutputEncoding` is set to
/// (measured 2026-09-18 against `powershell.exe`, with standard input as a pipe
/// and as a redirected file: the first three bytes were `EF BB BF`, while
/// `pwsh` 7 wrote none). A byte-order mark is not JSON, so without this every
/// event arriving from the Windows default shell would be refused as invalid —
/// and the Codex, Cursor and Claude Code launchers all reach SURE through that
/// pipe. The mark says nothing about the event, so dropping a leading one
/// invents nothing; everything after it is passed on exactly as it arrived.
fn without_byte_order_mark(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

fn run_ingest(
    source: Option<&str>,
    event_kind: Option<&str>,
    stdin: &str,
    store: Option<&Path>,
) -> Report {
    let paths = match Paths::discover_at(store) {
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
        // Codex's payload carries `hook_event_name`, so `--source codex` needs
        // no event-kind argument: the event names itself and SURE reads the
        // name Codex sent rather than one the manifest repeated. Four of the
        // twelve events Codex sends map; the rest are refused by name, with the
        // reason, in `crates/sure-core/src/normalizer/codex.rs` and in the table
        // in `integrations/codex/README.md`.
        Some("codex") => match codex::normalize(stdin) {
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
            // Codex is documented to name its shell tool `Bash` and to match its
            // patch tool as `Edit` or `Write`, which is the vocabulary this
            // classifier already recognises. A Codex name outside it — a local
            // function such as `update_plan`, or an MCP tool — falls to the
            // classifier's unknown branch, which fails closed rather than
            // passing. `integrations/codex/README.md` records that limitation
            // and why the decision is advisory on this harness.
            Some("claude-code" | "codex") => decide_claude_code_tool(tool, mode, &permissions),
            // `Some("cursor")`, and anything else: the match above has already
            // refused a source SURE does not know.
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
    // How long the recording is kept, arbitrated the way every other setting in
    // `sure_core::config` is: the user may name any duration, and a project may
    // only shorten it. A project asking to keep the data for longer is not
    // obeyed and does not raise the number — `Authority` records it as a refused
    // `ProjectRequest::ExtendedRetention`, and what is written is the shorter
    // period. "Recording more is not running more", and keeping it longer is not
    // either.
    //
    // The fallback is the *default*, and it is the only thing here that resolves
    // on failure. A configuration file that will not parse is a run whose
    // settings SURE does not know, and the safe direction is the shorter one:
    // falling back to nothing at all would be the same as the default, but
    // falling back to whatever the project asked for would let an unreadable
    // user file become a longer retention than SURE's own.
    let retention_days = Authority::load(project_path, &paths.user_config_file())
        .map_or(DEFAULT_FULL_RECORDING_RETENTION_DAYS, |authority| {
            authority.full_recording_retention_days().value
        });
    let _ = full_recording::persist_full_recording(
        &store,
        ingested,
        event_id,
        project_root,
        &fingerprint.id,
        consent,
        retention_days,
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
    use sure_core::hook_protection::ProtectionDecisionKind;
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

    /// Locations for an event this test ingests, so that the row it produces
    /// lands in a store this test named rather than in the store on the machine
    /// running the suite.
    ///
    /// `sure hook ingest` takes the same thing as `--store-dir`, on the command
    /// line; a unit test reaches it as the named roots of [`Paths::from_roots`].
    /// The events below are the fixtures under `integrations/`, so a row written
    /// to the real store would be an event from a fixture added to somebody's
    /// history, for as long as that machine lives.
    fn store_of_our_own(name: &str) -> Paths {
        let root = scratch_hook_dir(name);
        Paths::from_roots(root.join("data"), root.join("config"))
            .expect("the scratch locations are absolute")
    }

    #[test]
    fn cursor_pre_tool_use_returns_decision() {
        let text = fixture("pre-tool-use.json");
        let report = run_ingest_with_paths(
            Some("cursor"),
            Some("pre-tool-use"),
            &text,
            &store_of_our_own("cursor-pre-tool-use"),
        );
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
        let report = run_ingest_with_paths(
            Some("cursor"),
            Some("session-start"),
            &text,
            &store_of_our_own("cursor-session-start"),
        );
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
        // The four refusals below happen before SURE reaches a store at all —
        // there is nothing to read, nothing to normalise, nothing to record — so
        // they name none, which is also the honest statement that the store is
        // not what they are about.
        let report = run_ingest(None, Some("pre-tool-use"), "{}", None);
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn unsupported_source_fails() {
        let report = run_ingest(Some("unknown"), Some("pre-tool-use"), "{}", None);
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn invalid_json_fails() {
        let report = run_ingest(Some("cursor"), Some("pre-tool-use"), "{not json", None);
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn empty_stdin_fails() {
        let report = run_ingest(Some("cursor"), Some("pre-tool-use"), "", None);
        assert!(
            matches!(report, Report::Failed(_)),
            "expected Failed, got {report:?}"
        );
    }

    #[test]
    fn claude_code_session_start_allows_without_decision() {
        let text = claude_fixture("session-start.json");
        let report = run_ingest_with_paths(
            Some("claude-code"),
            Some("session-start"),
            &text,
            &store_of_our_own("claude-code-session-start"),
        );
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
        let report = run_ingest_with_paths(
            Some("claude-code"),
            Some("pre-tool-use"),
            &text,
            &store_of_our_own("claude-code-pre-tool-use"),
        );
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
        let report = run_ingest(Some("claude-code"), Some("pre-tool-use"), "{not json", None);
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

    #[test]
    fn cursor_check_repair_recheck_round_trip() {
        let tmp = scratch_hook_dir("cursor-e2e-check-repair-recheck");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");

        let project = tmp.join("project");
        std::fs::create_dir_all(&project).expect("create project");

        let data = tmp.join("data");
        let config = tmp.join("config");
        let paths = Paths::from_roots(data, config).expect("paths are valid");

        let project_root = project.to_string_lossy().into_owned();
        let session_id = "cursor-e2e-session";

        let session_start = serde_json::json!({
            "event": "sessionStart",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "timestamp_utc": "2026-09-18T12:00:00Z",
            "source": "cursor",
        })
        .to_string();

        // Read is allowed in InspectOnly.
        let read_pre = serde_json::json!({
            "event": "preToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Read",
            "args": {"path": "src/lib.rs"},
            "timestamp_utc": "2026-09-18T12:00:30Z",
            "source": "cursor",
        })
        .to_string();

        let check_pre = serde_json::json!({
            "event": "preToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Shell",
            "args": {"command": "npm test", "workdir": &project_root},
            "timestamp_utc": "2026-09-18T12:01:00Z",
            "source": "cursor",
        })
        .to_string();

        let check_post = serde_json::json!({
            "event": "postToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Shell",
            "args": {"command": "npm test", "workdir": &project_root},
            "error": {"exit_code": 1, "message": "tests failed"},
            "timestamp_utc": "2026-09-18T12:01:05Z",
            "source": "cursor",
        })
        .to_string();

        let repair_pre = serde_json::json!({
            "event": "preToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Write",
            "args": {"content": "good"},
            "path": "src/lib.rs",
            "timestamp_utc": "2026-09-18T12:02:00Z",
            "source": "cursor",
        })
        .to_string();

        let repair_post = serde_json::json!({
            "event": "postToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Write",
            "args": {"content": "good"},
            "path": "src/lib.rs",
            "result": {"ok": true},
            "timestamp_utc": "2026-09-18T12:02:05Z",
            "source": "cursor",
        })
        .to_string();

        let recheck_pre = serde_json::json!({
            "event": "preToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Shell",
            "args": {"command": "npm test", "workdir": &project_root},
            "timestamp_utc": "2026-09-18T12:03:00Z",
            "source": "cursor",
        })
        .to_string();

        let recheck_post = serde_json::json!({
            "event": "postToolUse",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "tool": "Shell",
            "args": {"command": "npm test", "workdir": &project_root},
            "result": {"exit_code": 0, "stdout": "Tests: 5 passed, 5 total", "stderr": ""},
            "timestamp_utc": "2026-09-18T12:03:05Z",
            "source": "cursor",
        })
        .to_string();

        let stop = serde_json::json!({
            "event": "stop",
            "harness_session_id": session_id,
            "project_root": &project_root,
            "timestamp_utc": "2026-09-18T12:04:00Z",
            "source": "cursor",
        })
        .to_string();

        let events = [
            ("session-start", session_start, None),
            (
                "pre-tool-use",
                read_pre,
                Some(ProtectionDecisionKind::Allow),
            ),
            (
                "pre-tool-use",
                check_pre,
                Some(ProtectionDecisionKind::Block),
            ),
            ("post-tool-use", check_post, None),
            (
                "pre-tool-use",
                repair_pre,
                Some(ProtectionDecisionKind::Block),
            ),
            ("post-tool-use", repair_post, None),
            (
                "pre-tool-use",
                recheck_pre,
                Some(ProtectionDecisionKind::Block),
            ),
            ("post-tool-use", recheck_post, None),
            ("stop", stop, None),
        ];

        for (kind, json, expected) in &events {
            let report = run_ingest_with_paths(Some("cursor"), Some(kind), json, &paths);
            let decision = match &report {
                Report::HookDecision(d) => d,
                other => panic!("expected HookDecision for {kind}, got {other:?}"),
            };

            if let Some(expected_kind) = expected {
                assert_eq!(
                    decision.decision,
                    *expected_kind,
                    "{kind} should be {:?} in InspectOnly mode",
                    expected_kind.as_str()
                );
                if *expected_kind == ProtectionDecisionKind::Block {
                    assert!(
                        decision.reason.is_some(),
                        "blocked decision should have a reason"
                    );
                }
            } else {
                assert_eq!(
                    decision.decision,
                    ProtectionDecisionKind::Allow,
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
        assert_eq!(stored_events.len(), 9, "all 9 events should be stored");

        let expected_types = [
            "session.started",
            "tool.requested",
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

    // --- codex --------------------------------------------------------------
    //
    // Every test here runs through `run_ingest_with_paths` with a scratch
    // `Paths`, like the two round trips above, so none of them open or write
    // the machine's real store. That is no longer the only way to keep them off
    // it: `sure hook ingest --store-dir DIR` names the store on the command
    // line, which is what `tests/cli_contract.rs` drives against a real process
    // and a store it names. Both are the same mechanism from two sides, and the
    // process that has none of it — a hook a harness starts with no argument —
    // gets the platform's own location, which is where a user's history belongs.

    fn codex_fixture(name: &str) -> String {
        let path = repository_root()
            .join("integrations")
            .join("codex")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    /// A Codex fixture with its `cwd` pointed at a real scratch directory.
    ///
    /// The payload's `cwd` becomes the event's project root, and the store keys
    /// a session by project, so a fixture pointed at a path that does not exist
    /// would persist nothing and prove nothing. The field names still come from
    /// the fixture; only the value of `cwd` is replaced.
    fn codex_event_at(name: &str, project_root: &str) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&codex_fixture(name)).expect("fixture is JSON");
        value["cwd"] = serde_json::Value::String(project_root.to_owned());
        value.to_string()
    }

    #[test]
    fn codex_session_start_allows_without_a_decision() {
        let tmp = scratch_hook_dir("codex-session-start");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");
        let paths = Paths::from_roots(tmp.join("data"), tmp.join("config")).expect("paths");

        let report = run_ingest_with_paths(
            Some("codex"),
            None,
            &codex_event_at("session-start.json", &tmp.to_string_lossy()),
            &paths,
        );
        let decision = match &report {
            Report::HookDecision(d) => d,
            other => panic!("expected HookDecision, got {other:?}"),
        };
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn codex_pre_tool_use_gets_a_protection_decision() {
        let tmp = scratch_hook_dir("codex-pre-tool-use");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");
        let paths = Paths::from_roots(tmp.join("data"), tmp.join("config")).expect("paths");

        // Codex's documented shell tool name is `Bash`, which the shared
        // classifier already knows. The scratch project has no `sure.yaml`, so
        // the mode is the safest one and a shell command is blocked.
        let report = run_ingest_with_paths(
            Some("codex"),
            None,
            &codex_event_at("pre-tool-use.json", &tmp.to_string_lossy()),
            &paths,
        );
        let decision = match &report {
            Report::HookDecision(d) => d,
            other => panic!("expected HookDecision, got {other:?}"),
        };
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn codex_tool_name_outside_the_classifier_vocabulary_fails_closed() {
        let tmp = scratch_hook_dir("codex-unknown-tool");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");
        let paths = Paths::from_roots(tmp.join("data"), tmp.join("config")).expect("paths");

        // A Codex local function tool, not one of the names the classifier was
        // written for. It must not read as "nothing to object to".
        let mut value: serde_json::Value =
            serde_json::from_str(&codex_fixture("pre-tool-use.json")).expect("fixture is JSON");
        value["cwd"] = serde_json::Value::String(tmp.to_string_lossy().into_owned());
        value["tool_name"] = serde_json::Value::String("update_plan".to_owned());

        let report = run_ingest_with_paths(Some("codex"), None, &value.to_string(), &paths);
        let decision = match &report {
            Report::HookDecision(d) => d,
            other => panic!("expected HookDecision, got {other:?}"),
        };
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
    }

    #[test]
    fn codex_stop_is_refused_and_writes_nothing() {
        let tmp = scratch_hook_dir("codex-stop-refused");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");
        let project = tmp.join("project");
        std::fs::create_dir_all(&project).expect("create project");
        let paths = Paths::from_roots(tmp.join("data"), tmp.join("config")).expect("paths");
        let project_root = project.to_string_lossy().into_owned();

        // `Stop` ends a turn, not a session, and SURE has no turn-end event
        // type. The refusal has to be visible and it has to leave no row
        // behind: an event SURE cannot mean must not become evidence.
        let refused = run_ingest_with_paths(
            Some("codex"),
            None,
            &codex_event_at("stop-not-mapped.json", &project_root),
            &paths,
        );
        match &refused {
            Report::Failed(failure) => {
                assert!(
                    failure.detail.contains("turn"),
                    "the refusal must name why: {}",
                    failure.detail
                );
                assert!(
                    failure.detail.contains("Nothing was recorded"),
                    "the refusal must say nothing was recorded: {}",
                    failure.detail
                );
            }
            other => panic!("expected Failed for Stop, got {other:?}"),
        }

        // One mapped event, then read the store back: exactly one row, and it
        // is the mapped one.
        let accepted = run_ingest_with_paths(
            Some("codex"),
            None,
            &codex_event_at("session-end.json", &project_root),
            &paths,
        );
        assert!(
            matches!(accepted, Report::HookDecision(_)),
            "expected the mapped event to ingest, got {accepted:?}"
        );

        let store = Store::open(&paths, &project).expect("store opens");
        let session_store = SessionEventStore::new(&store);
        let sessions = session_store
            .sessions_past_retention(i64::MAX)
            .expect("query sessions");
        assert_eq!(sessions.len(), 1, "one session should exist");
        let stored = session_store
            .events_for_session(sessions[0].row_id)
            .expect("query events");
        assert_eq!(stored.len(), 1, "Stop must not have left a row behind");
        assert_eq!(stored[0].event_type, "session.stopped");
    }

    #[test]
    fn codex_session_events_round_trip_into_one_session() {
        let tmp = scratch_hook_dir("codex-e2e-round-trip");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");
        let project = tmp.join("project");
        std::fs::create_dir_all(&project).expect("create project");
        let paths = Paths::from_roots(tmp.join("data"), tmp.join("config")).expect("paths");
        let project_root = project.to_string_lossy().into_owned();

        // No event-kind argument: this is what `integrations/codex/hooks/hooks.json`
        // invokes, because the payload carries `hook_event_name` itself.
        for name in [
            "session-start.json",
            "pre-tool-use.json",
            "post-tool-use.json",
            "session-end.json",
        ] {
            let report = run_ingest_with_paths(
                Some("codex"),
                None,
                &codex_event_at(name, &project_root),
                &paths,
            );
            assert!(
                matches!(report, Report::HookDecision(_)),
                "{name} should have ingested, got {report:?}"
            );
        }

        let store = Store::open(&paths, &project).expect("store opens");
        let session_store = SessionEventStore::new(&store);
        let sessions = session_store
            .sessions_past_retention(i64::MAX)
            .expect("query sessions");
        assert_eq!(
            sessions.len(),
            1,
            "all four events share one Codex session id, so they are one session"
        );
        assert_eq!(
            sessions[0].harness_session_id.as_deref(),
            Some("codex-session-001")
        );

        let mut stored = session_store
            .events_for_session(sessions[0].row_id)
            .expect("query events");
        assert_eq!(stored.len(), 4, "all four mapped events should be stored");
        stored.reverse();
        let types: Vec<&str> = stored.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(
            types,
            vec![
                "session.started",
                "tool.requested",
                "tool.completed",
                "session.stopped"
            ]
        );
    }

    #[test]
    fn only_a_leading_byte_order_mark_is_not_part_of_the_event() {
        assert_eq!(without_byte_order_mark("\u{feff}{}"), "{}");
        assert_eq!(without_byte_order_mark("{}"), "{}");
        // One inside the payload belongs to the payload, and is left alone.
        assert_eq!(
            without_byte_order_mark("{\"text\":\"\u{feff}\"}"),
            "{\"text\":\"\u{feff}\"}"
        );
    }

    #[test]
    fn codex_payload_behind_a_byte_order_mark_still_ingests() {
        let tmp = scratch_hook_dir("codex-byte-order-mark");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create scratch");
        let project = tmp.join("project");
        std::fs::create_dir_all(&project).expect("create project");
        let paths = Paths::from_roots(tmp.join("data"), tmp.join("config")).expect("paths");

        // What Windows PowerShell 5.1 pipes in front of the event, measured on
        // 2026-09-18: a UTF-8 byte-order mark, then the payload.
        let marked = format!(
            "\u{feff}{}",
            codex_event_at("session-start.json", &project.to_string_lossy())
        );

        // Handed over as-is it is not JSON, which is the failure `run()` exists
        // to avoid.
        match run_ingest_with_paths(Some("codex"), None, &marked, &paths) {
            Report::Failed(failure) => assert!(
                failure.detail.contains("not valid JSON"),
                "a byte-order mark should be the reason the payload is refused: {}",
                failure.detail
            ),
            other => panic!("expected the marked payload to be refused, got {other:?}"),
        }

        // Handed over the way `run()` hands it over, it ingests, and the store
        // holds the one event.
        let report = run_ingest_with_paths(
            Some("codex"),
            None,
            without_byte_order_mark(&marked),
            &paths,
        );
        assert!(
            matches!(report, Report::HookDecision(_)),
            "the payload behind the mark should ingest, got {report:?}"
        );
        let store = Store::open(&paths, &project).expect("store opens");
        let session_store = SessionEventStore::new(&store);
        let sessions = session_store
            .sessions_past_retention(i64::MAX)
            .expect("query sessions");
        assert_eq!(sessions.len(), 1, "the marked payload is one session");
        let stored = session_store
            .events_for_session(sessions[0].row_id)
            .expect("query events");
        assert_eq!(stored.len(), 1, "and one event");
    }
}

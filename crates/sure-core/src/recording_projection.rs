//! Summarized projection of harness events without full transcripts.
//!
//! The standard recording projection discards verbose arguments, outputs, and
//! raw transcripts, keeping only a one-line summary of what happened. This is
//! the privacy-conscious default: activity is recorded, but not the content of
//! that activity.
//!
//! Projections are stored as [`RecordKind::Recording`] because they are raw
//! captured material, not a project statement with a schema. A wrapper object
//! identifies them as `standard_projection` and carries a retention deadline.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sure_domain::ids::FingerprintId;

use crate::harness_event::IngestedEvent;
use crate::redact;
use crate::store::{HistoryFilter, RecordKind, Store, StoreError, StoredRecord};

/// Default retention for standard projections: 7 days.
pub const DEFAULT_PROJECTION_RETENTION_DAYS: i64 = 7;

/// One summarized activity extracted from a harness event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum StandardProjection {
    /// A tool was invoked.
    Tool {
        /// Tool name, e.g. "Bash" or "Read".
        name: String,
        /// Whether the tool started, completed, or errored.
        status: ToolStatus,
        /// Duration in milliseconds, if the payload reports one.
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        /// One-line human-readable summary.
        summary: String,
    },
    /// A shell command was executed.
    Command {
        /// The program that ran, e.g. "cargo" or "git".
        program: String,
        /// A short argument signature: first argument or a count.
        argument_signature: String,
        /// Exit status, if known.
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_status: Option<i32>,
    },
    /// A git operation occurred.
    Git {
        /// The git subcommand, e.g. "commit" or "checkout".
        subcommand: String,
        /// The branch name, if available.
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        /// Number of files affected, if the payload reports it.
        #[serde(skip_serializing_if = "Option::is_none")]
        files_affected_count: Option<usize>,
    },
    /// A file was touched.
    File {
        /// What happened to the file.
        operation: FileOperation,
        /// The basename of the path (no directory).
        basename: String,
        /// File size in bytes, if available.
        #[serde(skip_serializing_if = "Option::is_none")]
        size_bytes: Option<u64>,
    },
    /// A build, test, compilation, or run occurred.
    BuildTest {
        /// Which kind of activity.
        kind: BuildTestKind,
        /// The target being built or tested.
        target: String,
        /// Whether it succeeded.
        success: bool,
        /// Additional counts (pass, fail, skip, etc.) if available.
        #[serde(skip_serializing_if = "Option::is_none")]
        counts: Option<Value>,
    },
    /// An outcome, result, or verdict was reported.
    Outcome {
        /// The outcome name, e.g. "verdict".
        outcome_name: String,
        /// Whether the outcome was successful.
        success: bool,
    },
}

/// Lifecycle state of a tool invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    /// The tool began execution.
    Started,
    /// The tool finished without error.
    Completed,
    /// The tool finished with an error.
    Error,
}

/// What happened to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    /// The file was read.
    Read,
    /// The file was written.
    Write,
    /// The file was deleted.
    Delete,
}

/// The kind of build or test activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildTestKind {
    /// A build step.
    Build,
    /// A test run.
    Test,
    /// A compilation.
    Compile,
    /// A program run.
    Run,
}

/// Why a projection could not be persisted.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectionStoreError {
    /// The underlying store reported an error.
    Store {
        /// What the store said.
        message: String,
    },
    /// The projection document could not be serialized.
    Serialize {
        /// What serde said.
        message: String,
    },
}

impl std::fmt::Display for ProjectionStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store { message } => {
                write!(
                    f,
                    "SURE could not store the projection.\n\n{}\n\n\
                     Nothing was written.",
                    redact::escape_control_characters(message)
                )
            }
            Self::Serialize { message } => {
                write!(
                    f,
                    "SURE could not turn the projection into JSON: {}\n\n\
                     Nothing was written.",
                    redact::escape_control_characters(message)
                )
            }
        }
    }
}

impl std::error::Error for ProjectionStoreError {}

impl From<StoreError> for ProjectionStoreError {
    fn from(error: StoreError) -> Self {
        Self::Store {
            message: error.to_string(),
        }
    }
}

/// Wraps a [`Store`] to provide standard-projection persistence and querying.
#[derive(Debug)]
pub struct ProjectionStore<'a> {
    store: &'a Store,
}

impl<'a> ProjectionStore<'a> {
    /// Wrap an open store.
    #[must_use]
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Persist a standard projection about a project.
    ///
    /// The projection is wrapped in a JSON object that identifies it as a
    /// `standard_projection` and includes a retention deadline.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionStoreError`] if the projection cannot be persisted.
    pub fn persist(
        &self,
        projection: &StandardProjection,
        project_root: &str,
        fingerprint: &FingerprintId,
    ) -> Result<i64, ProjectionStoreError> {
        let retained_until_ms =
            retention_deadline_ms(Self::now_ms(), DEFAULT_PROJECTION_RETENTION_DAYS);

        let wrapper = json!({
            "standard_projection": true,
            "retained_until_ms": retained_until_ms,
            "projection": projection,
        });

        let id = self
            .store
            .append_for(RecordKind::Recording, &wrapper, project_root, fingerprint)
            .map_err(ProjectionStoreError::from)?;
        Ok(id)
    }

    /// Read back every standard projection for a project, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionStoreError`] if the query fails.
    pub fn projections_for_project(
        &self,
        fingerprint: &FingerprintId,
        limit: usize,
    ) -> Result<Vec<StoredProjection>, ProjectionStoreError> {
        let filter = HistoryFilter {
            project_fingerprint: Some(fingerprint),
            kind: Some(RecordKind::Recording),
            include_recordings: true,
        };

        let records = self
            .store
            .history(&filter, limit)
            .map_err(ProjectionStoreError::from)?;

        let mut results = Vec::new();
        for record in records {
            if let Some(projection) = Self::extract_projection(&record) {
                results.push(StoredProjection {
                    record_id: record.id,
                    written_at_ms: record.written_at_ms,
                    projection,
                });
            }
        }
        Ok(results)
    }

    /// List projections whose retention has expired.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionStoreError`] if the query fails.
    pub fn projections_past_retention(
        &self,
        fingerprint: &FingerprintId,
        before_ms: i64,
    ) -> Result<Vec<StoredProjection>, ProjectionStoreError> {
        let all = self.projections_for_project(fingerprint, 10_000)?;
        Ok(all
            .into_iter()
            .filter(|sp| sp.projection.retained_until_ms() < before_ms)
            .collect())
    }

    fn extract_projection(record: &StoredRecord) -> Option<WrappedProjection> {
        if !record.document.get("standard_projection")?.as_bool()? {
            return None;
        }
        let projection_value = record.document.get("projection")?;
        let projection: StandardProjection =
            serde_json::from_value(projection_value.clone()).ok()?;
        let retained_until_ms = record.document.get("retained_until_ms")?.as_i64()?;
        Some(WrappedProjection {
            projection,
            retained_until_ms,
        })
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64)
    }
}

/// A projection together with its retention deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedProjection {
    /// The parsed projection.
    pub projection: StandardProjection,
    /// When this projection should be discarded, milliseconds since epoch.
    pub retained_until_ms: i64,
}

impl WrappedProjection {
    /// The retention deadline.
    #[must_use]
    pub fn retained_until_ms(&self) -> i64 {
        self.retained_until_ms
    }
}

/// A projection as read back from the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredProjection {
    /// The row id in `records`.
    pub record_id: i64,
    /// When it was written.
    pub written_at_ms: i64,
    /// The wrapped projection including retention metadata.
    pub projection: WrappedProjection,
}

/// Map an ingested event to a standard projection, if it matches a known
/// category.
///
/// Returns `None` when the event type does not correspond to any supported
/// activity category.
#[must_use]
pub fn project(event: &IngestedEvent) -> Option<StandardProjection> {
    let event_type = event.envelope.event_type.as_str();
    let payload = &event.envelope.payload;

    if event_type.contains("tool") {
        Some(project_tool(payload, event_type))
    } else if event_type.contains("command") || event_type.contains("shell") {
        Some(project_command(payload))
    } else if event_type.contains("git") || source_names_git(&event.envelope.source) {
        Some(project_git(payload, event_type))
    } else if event_type.contains("file") {
        Some(project_file(payload, event_type))
    } else if event_type.contains("build")
        || event_type.contains("test")
        || event_type.contains("compile")
        || event_type.contains("run")
    {
        Some(project_build_test(payload, event_type))
    } else if event_type.contains("outcome")
        || event_type.contains("result")
        || event_type.contains("verdict")
    {
        Some(project_outcome(payload))
    } else {
        None
    }
}

/// Persist a projection directly using the given store.
///
/// This is the free-function equivalent of [`ProjectionStore::persist`].
///
/// # Errors
///
/// Returns [`ProjectionStoreError`] if the projection cannot be persisted.
pub fn persist_projection(
    store: &Store,
    projection: &StandardProjection,
    project_root: &str,
    fingerprint: &FingerprintId,
) -> Result<i64, ProjectionStoreError> {
    ProjectionStore::new(store).persist(projection, project_root, fingerprint)
}

/// Compute a retention deadline from a start time and retention days.
#[must_use]
pub fn retention_deadline_ms(start_ms: i64, retention_days: i64) -> i64 {
    start_ms.saturating_add(retention_days.saturating_mul(86_400_000))
}

// ---------------------------------------------------------------------------
// Category-specific projection builders
// ---------------------------------------------------------------------------

fn project_tool(payload: &Value, event_type: &str) -> StandardProjection {
    let name = safe_string(payload.get("tool").and_then(Value::as_str), "unknown");
    let status = if event_type.contains("started") {
        ToolStatus::Started
    } else if event_type.contains("error") || event_type.contains("failed") {
        ToolStatus::Error
    } else {
        ToolStatus::Completed
    };
    let duration_ms = payload.get("duration_ms").and_then(Value::as_u64);
    let summary = safe_string(
        payload.get("summary").and_then(Value::as_str),
        &format!("{name} {status:?}"),
    );

    StandardProjection::Tool {
        name,
        status,
        duration_ms,
        summary,
    }
}

fn project_command(payload: &Value) -> StandardProjection {
    let program = safe_string(payload.get("program").and_then(Value::as_str), "unknown");
    let args = payload.get("args");
    let argument_signature = args
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(Value::as_str)
        .map(|s| s.to_owned())
        .or_else(|| {
            args.and_then(Value::as_array)
                .map(|arr| format!("{} args", arr.len()))
        })
        .unwrap_or_else(|| String::from("no args"));
    let exit_status = payload
        .get("exit_code")
        .and_then(Value::as_i64)
        .map(|v| v as i32);

    StandardProjection::Command {
        program,
        argument_signature,
        exit_status,
    }
}

fn project_git(payload: &Value, event_type: &str) -> StandardProjection {
    let subcommand = safe_string(payload.get("subcommand").and_then(Value::as_str), {
        // Infer subcommand from event type if not explicitly given.
        if event_type.contains("commit") {
            "commit"
        } else if event_type.contains("checkout") {
            "checkout"
        } else if event_type.contains("pull") {
            "pull"
        } else if event_type.contains("push") {
            "push"
        } else if event_type.contains("merge") {
            "merge"
        } else if event_type.contains("branch") {
            "branch"
        } else {
            "unknown"
        }
    });
    let branch = payload
        .get("branch")
        .and_then(Value::as_str)
        .map(|s| s.to_owned());
    let files_affected_count = payload
        .get("files_affected")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .or_else(|| {
            payload
                .get("files")
                .and_then(Value::as_array)
                .map(|arr| arr.len())
        });

    StandardProjection::Git {
        subcommand,
        branch,
        files_affected_count,
    }
}

fn project_file(payload: &Value, event_type: &str) -> StandardProjection {
    let operation = if payload.get("deleted").and_then(Value::as_bool) == Some(true)
        || event_type.contains("delete")
    {
        FileOperation::Delete
    } else if payload.get("written").and_then(Value::as_bool) == Some(true)
        || payload.get("operation").and_then(Value::as_str) == Some("write")
        || event_type.contains("write")
    {
        FileOperation::Write
    } else {
        FileOperation::Read
    };

    let basename = payload
        .get("path")
        .and_then(Value::as_str)
        .map(std::path::Path::new)
        .and_then(std::path::Path::file_name)
        .and_then(|os| os.to_str())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| safe_string(payload.get("basename").and_then(Value::as_str), "unknown"));

    let size_bytes = payload.get("size").and_then(Value::as_u64);

    StandardProjection::File {
        operation,
        basename,
        size_bytes,
    }
}

fn project_build_test(payload: &Value, event_type: &str) -> StandardProjection {
    let kind = if event_type.contains("test") {
        BuildTestKind::Test
    } else if event_type.contains("compile") {
        BuildTestKind::Compile
    } else if event_type.contains("run") {
        BuildTestKind::Run
    } else {
        BuildTestKind::Build
    };

    let target = safe_string(payload.get("target").and_then(Value::as_str), "unknown");
    let success = payload.get("success").and_then(Value::as_bool) == Some(true);
    let counts = payload.get("counts").cloned();

    StandardProjection::BuildTest {
        kind,
        target,
        success,
        counts,
    }
}

fn project_outcome(payload: &Value) -> StandardProjection {
    let outcome_name = safe_string(payload.get("outcome").and_then(Value::as_str), "unknown");
    let success = payload.get("success").and_then(Value::as_bool) == Some(true);

    StandardProjection::Outcome {
        outcome_name,
        success,
    }
}

fn source_names_git(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower == "git" || lower.contains("git-")
}

/// Turn an optional attacker-controlled string into a safe owned string,
/// escaping control characters and applying a default when absent.
fn safe_string(value: Option<&str>, default: &str) -> String {
    match value {
        Some(text) => redact::escape_control_characters(text),
        None => String::from(default),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;
    use sure_domain::capability::CapabilityTier;
    use sure_protocol::PROTOCOL_VERSION;
    use sure_protocol::event::EventEnvelope;

    use crate::harness_event::IngestedEvent;
    use crate::store::{HistoryFilter, Store};

    fn scratch(name: &str) -> std::path::PathBuf {
        crate::store::scratch_root().join(format!("{name}-{}", std::process::id()))
    }

    fn store_in(name: &str) -> Store {
        let dir = scratch(name);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("cannot clear {}: {e}", dir.display()),
        }
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Store::open_at(&dir.join("sure.db")).expect("the store opens")
    }

    fn make_event(event_type: &str, payload: Value) -> IngestedEvent {
        IngestedEvent {
            envelope: EventEnvelope::new("claude-code", event_type, "2026-09-14T09:10:56.827Z")
                .with_capability_tier(CapabilityTier::Observed)
                .with_session_id("session-42")
                .with_project_root("C:\\work\\my project")
                .with_payload(payload),
            protocol_version: PROTOCOL_VERSION,
            document_kind: sure_protocol::documents::DocumentKind::Event,
        }
    }

    #[test]
    fn tool_event_is_projected() {
        let event = make_event(
            "tool.completed",
            json!({
                "tool": "Bash",
                "duration_ms": 1200,
                "summary": "ran tests"
            }),
        );
        let proj = project(&event).expect("tool event projects");
        match proj {
            StandardProjection::Tool {
                name,
                status,
                duration_ms,
                summary,
            } => {
                assert_eq!(name, "Bash");
                assert_eq!(status, ToolStatus::Completed);
                assert_eq!(duration_ms, Some(1200));
                assert_eq!(summary, "ran tests");
            }
            other => panic!("expected Tool projection, got {other:?}"),
        }
    }

    #[test]
    fn tool_started_event_maps_to_started_status() {
        let event = make_event("tool.started", json!({"tool": "Read"}));
        let proj = project(&event).unwrap();
        match proj {
            StandardProjection::Tool { status, .. } => {
                assert_eq!(status, ToolStatus::Started);
            }
            other => panic!("expected Tool projection, got {other:?}"),
        }
    }

    #[test]
    fn tool_error_event_maps_to_error_status() {
        let event = make_event("tool.error", json!({"tool": "Edit"}));
        let proj = project(&event).unwrap();
        match proj {
            StandardProjection::Tool { status, .. } => {
                assert_eq!(status, ToolStatus::Error);
            }
            other => panic!("expected Tool projection, got {other:?}"),
        }
    }

    #[test]
    fn command_event_is_projected() {
        let event = make_event(
            "command.executed",
            json!({
                "program": "cargo",
                "args": ["test", "--workspace"],
                "exit_code": 0
            }),
        );
        let proj = project(&event).expect("command event projects");
        match proj {
            StandardProjection::Command {
                program,
                argument_signature,
                exit_status,
            } => {
                assert_eq!(program, "cargo");
                assert_eq!(argument_signature, "test");
                assert_eq!(exit_status, Some(0));
            }
            other => panic!("expected Command projection, got {other:?}"),
        }
    }

    #[test]
    fn command_with_no_args_uses_count_or_default() {
        let event = make_event("shell.run", json!({"program": "ls", "args": []}));
        let proj = project(&event).unwrap();
        match proj {
            StandardProjection::Command {
                argument_signature, ..
            } => {
                assert_eq!(argument_signature, "0 args");
            }
            other => panic!("expected Command projection, got {other:?}"),
        }
    }

    #[test]
    fn git_event_is_projected() {
        let event = make_event(
            "git.commit",
            json!({
                "subcommand": "commit",
                "branch": "main",
                "files_affected": 3
            }),
        );
        let proj = project(&event).expect("git event projects");
        match proj {
            StandardProjection::Git {
                subcommand,
                branch,
                files_affected_count,
            } => {
                assert_eq!(subcommand, "commit");
                assert_eq!(branch, Some("main".to_owned()));
                assert_eq!(files_affected_count, Some(3));
            }
            other => panic!("expected Git projection, got {other:?}"),
        }
    }

    #[test]
    fn git_event_infers_subcommand_from_event_type() {
        let event = make_event("git.push", json!({}));
        let proj = project(&event).unwrap();
        match proj {
            StandardProjection::Git { subcommand, .. } => {
                assert_eq!(subcommand, "push");
            }
            other => panic!("expected Git projection, got {other:?}"),
        }
    }

    #[test]
    fn file_write_event_is_projected() {
        let event = make_event(
            "file.write",
            json!({
                "path": "C:\\work\\my project\\src\\main.rs",
                "size": 1024
            }),
        );
        let proj = project(&event).expect("file event projects");
        match proj {
            StandardProjection::File {
                operation,
                basename,
                size_bytes,
            } => {
                assert_eq!(operation, FileOperation::Write);
                assert_eq!(basename, "main.rs");
                assert_eq!(size_bytes, Some(1024));
            }
            other => panic!("expected File projection, got {other:?}"),
        }
    }

    #[test]
    fn file_delete_event_is_projected() {
        let event = make_event("file.delete", json!({"path": "old.txt", "deleted": true}));
        let proj = project(&event).unwrap();
        match proj {
            StandardProjection::File { operation, .. } => {
                assert_eq!(operation, FileOperation::Delete);
            }
            other => panic!("expected File projection, got {other:?}"),
        }
    }

    #[test]
    fn build_event_is_projected() {
        let event = make_event(
            "build.finished",
            json!({
                "target": "sure-core",
                "success": true,
                "counts": {"warnings": 2}
            }),
        );
        let proj = project(&event).expect("build event projects");
        match proj {
            StandardProjection::BuildTest {
                kind,
                target,
                success,
                counts,
            } => {
                assert_eq!(kind, BuildTestKind::Build);
                assert_eq!(target, "sure-core");
                assert!(success);
                assert_eq!(counts, Some(json!({"warnings": 2})));
            }
            other => panic!("expected BuildTest projection, got {other:?}"),
        }
    }

    #[test]
    fn test_event_is_projected() {
        let event = make_event(
            "test.finished",
            json!({
                "target": "unit tests",
                "success": false,
                "counts": {"passed": 10, "failed": 2}
            }),
        );
        let proj = project(&event).unwrap();
        match proj {
            StandardProjection::BuildTest {
                kind,
                success,
                counts,
                ..
            } => {
                assert_eq!(kind, BuildTestKind::Test);
                assert!(!success);
                assert_eq!(counts, Some(json!({"passed": 10, "failed": 2})));
            }
            other => panic!("expected BuildTest projection, got {other:?}"),
        }
    }

    #[test]
    fn outcome_event_is_projected() {
        let event = make_event(
            "outcome.reported",
            json!({
                "outcome": "lint",
                "success": true
            }),
        );
        let proj = project(&event).expect("outcome event projects");
        match proj {
            StandardProjection::Outcome {
                outcome_name,
                success,
            } => {
                assert_eq!(outcome_name, "lint");
                assert!(success);
            }
            other => panic!("expected Outcome projection, got {other:?}"),
        }
    }

    #[test]
    fn unknown_event_type_returns_none() {
        let event = make_event("telemetry.heartbeat", json!({"cpu": 12}));
        assert!(project(&event).is_none());
    }

    #[test]
    fn projection_roundtrips_through_store() {
        let store = store_in("projection_roundtrip");
        let projection = StandardProjection::Tool {
            name: "Bash".to_owned(),
            status: ToolStatus::Completed,
            duration_ms: Some(500),
            summary: "echo hello".to_owned(),
        };
        let fingerprint = FingerprintId::generate();

        let id = persist_projection(&store, &projection, "C:\\work\\my project", &fingerprint)
            .expect("persist succeeds");
        assert!(id > 0);

        let proj_store = ProjectionStore::new(&store);
        let back = proj_store
            .projections_for_project(&fingerprint, 10)
            .expect("query succeeds");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].projection.projection, projection);
        assert_eq!(back[0].record_id, id);
    }

    #[test]
    fn projections_are_excluded_from_default_history() {
        let store = store_in("projection_hidden");
        let projection = StandardProjection::Command {
            program: "cargo".to_owned(),
            argument_signature: "build".to_owned(),
            exit_status: Some(0),
        };
        let fingerprint = FingerprintId::generate();

        persist_projection(&store, &projection, "C:\\work\\my project", &fingerprint).unwrap();

        // Default history excludes recordings.
        let default = store
            .history(&HistoryFilter::for_project(&fingerprint), 10)
            .unwrap();
        assert!(default.is_empty());

        // Including recordings reveals the projection.
        let with_recordings = HistoryFilter {
            include_recordings: true,
            ..HistoryFilter::for_project(&fingerprint)
        };
        let found = store.history(&with_recordings, 10).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn retention_metadata_is_included_in_wrapper() {
        let store = store_in("projection_retention");
        let projection = StandardProjection::Outcome {
            outcome_name: "pass".to_owned(),
            success: true,
        };
        let fingerprint = FingerprintId::generate();

        persist_projection(&store, &projection, "C:\\work\\my project", &fingerprint).unwrap();

        let with_recordings = HistoryFilter {
            include_recordings: true,
            ..HistoryFilter::for_project(&fingerprint)
        };
        let records = store.history(&with_recordings, 10).unwrap();
        assert_eq!(records.len(), 1);

        let doc = &records[0].document;
        assert_eq!(doc.get("standard_projection"), Some(&json!(true)));
        assert!(
            doc.get("retained_until_ms")
                .and_then(Value::as_i64)
                .unwrap()
                > 0,
            "retention deadline should be positive"
        );
    }

    #[test]
    fn past_retention_query_finds_only_expired_items() {
        let store = store_in("projection_past_retention");
        let projection = StandardProjection::Git {
            subcommand: "commit".to_owned(),
            branch: Some("main".to_owned()),
            files_affected_count: None,
        };
        let fingerprint = FingerprintId::generate();

        persist_projection(&store, &projection, "C:\\work\\my project", &fingerprint).unwrap();

        let proj_store = ProjectionStore::new(&store);

        // Nothing is past retention at time 0.
        assert!(
            proj_store
                .projections_past_retention(&fingerprint, 0)
                .unwrap()
                .is_empty()
        );

        // Everything is past retention at i64::MAX.
        let expired = proj_store
            .projections_past_retention(&fingerprint, i64::MAX)
            .unwrap();
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn attacker_controlled_text_is_escaped_in_projection_fields() {
        let event = make_event(
            "tool.completed",
            json!({
                "tool": "Bash\nfoo",
                "summary": "line1\nline2\tend"
            }),
        );
        let proj = project(&event).unwrap();
        match proj {
            StandardProjection::Tool { name, summary, .. } => {
                assert!(!name.contains('\n'), "newline was not escaped: {name}");
                assert!(
                    !summary.contains('\n'),
                    "newline was not escaped: {summary}"
                );
                assert!(!summary.contains('\t'), "tab was not escaped: {summary}");
                assert_eq!(name, "Bash\\nfoo");
                assert_eq!(summary, "line1\\nline2\\tend");
            }
            other => panic!("expected Tool projection, got {other:?}"),
        }
    }

    #[test]
    fn error_messages_escape_attacker_input() {
        let err = ProjectionStoreError::Serialize {
            message: "bad\ninput".to_owned(),
        };
        let text = err.to_string();
        assert!(
            text.contains("bad\\ninput"),
            "attacker-controlled newline was not escaped: {text}"
        );
    }

    #[test]
    fn retention_deadline_does_not_overflow() {
        let max = i64::MAX;
        let result = retention_deadline_ms(max, 1);
        assert_eq!(result, i64::MAX);
    }

    #[test]
    fn source_named_git_triggers_git_projection() {
        let event = IngestedEvent {
            envelope: EventEnvelope::new("git-hook", "push.received", "2026-09-14T09:10:56.827Z")
                .with_payload(json!({"subcommand": "push", "branch": "dev"})),
            protocol_version: PROTOCOL_VERSION,
            document_kind: sure_protocol::documents::DocumentKind::Event,
        };
        let proj = project(&event).expect("git-source event projects");
        match proj {
            StandardProjection::Git {
                subcommand, branch, ..
            } => {
                assert_eq!(subcommand, "push");
                assert_eq!(branch, Some("dev".to_owned()));
            }
            other => panic!("expected Git projection, got {other:?}"),
        }
    }

    #[test]
    fn multiple_projections_are_newest_first() {
        let store = store_in("projection_ordering");
        let fingerprint = FingerprintId::generate();
        let proj_store = ProjectionStore::new(&store);

        for i in 0..5 {
            let projection = StandardProjection::Tool {
                name: format!("tool-{i}"),
                status: ToolStatus::Completed,
                duration_ms: None,
                summary: format!("summary-{i}"),
            };
            proj_store
                .persist(&projection, "C:\\work\\my project", &fingerprint)
                .unwrap();
        }

        let back = proj_store
            .projections_for_project(&fingerprint, 10)
            .unwrap();
        assert_eq!(back.len(), 5);
        // Newest first means descending record ids.
        let ids: Vec<i64> = back.iter().map(|sp| sp.record_id).collect();
        let mut descending = ids.clone();
        descending.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(ids, descending, "projections are not newest-first");
    }
}

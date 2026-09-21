//! One-time allowances: what a user granted, and how a request spends it.
//!
//! `P13-T005`'s acceptance is that a broad delete, a force push and a sensitive
//! read are represented **with a one-time override path**. The representation
//! and the naming are `crate::hook_protection`'s; this module is the path.
//!
//! # Why an allowance is a stored row and not a token in a process
//!
//! A harness calls a hook as a **fresh process per event**, so a grant that
//! lived in memory would be gone before the request it was meant for arrived.
//! *One-time* therefore needs durable state with a used/not-used distinction,
//! and `docs/security/PROTECTION_MODE.md` already draws the shape of that
//! conversation (`[Allow once] [Do not allow]`) while promising nothing about
//! it. This is what stands behind the words.
//!
//! # What is stored, and what is not
//!
//! A grant is the tool name and the subject **exactly as the user typed them**,
//! plus when it was recorded and when it stops being usable — and a use is the
//! row it spent. Nothing else: no command output, no file contents, no prompt
//! text, and nothing the harness sent. `docs/security/PRIVACY.md` is the rule
//! and the record is what makes it checkable.
//!
//! The subject is the request in the harness's own terms: the **command line**
//! for a shell tool, the **path** for a tool that names one. Matching is exact
//! for the tool and the subject, because the sentence a user is shown says
//! *this exact request* and an allowance that matched loosely would be a
//! different statement from the one SURE makes. The project root is compared
//! with separators and case folded, because a Windows path arrives spelled both
//! ways and the same project is the same project.
//!
//! # What this module does not claim
//!
//! An allowance is a decision SURE reaches, not a fact about a harness. Both
//! integrations are Observed (Tier 1), so nothing here establishes that a
//! harness reads the answer, and the sentence an allowance produces says *SURE
//! would let this one through* rather than *it ran*. Whether the command ran is
//! not something this build can confirm.

use serde::{Deserialize, Serialize};
use sure_domain::ids::FingerprintId;

use crate::store::{RecordKind, Store, StoreError, StoredRecord};

/// How long an allowance lasts when the user does not say.
///
/// Long enough to answer the harness after reading the sentence, short enough
/// that a grant nobody used is not still waiting tomorrow. The user can ask for
/// anything from a minute to [`MAX_MINUTES`].
pub const DEFAULT_MINUTES: u32 = 30;

/// The longest an allowance may be asked to last: one day.
///
/// A cap rather than no cap because *one-time* is the whole of what an
/// allowance is, and a grant with an unbounded life is a standing permission
/// wearing that name. A day is the longest any of these three acts is worth
/// asking about twice.
pub const MAX_MINUTES: u32 = 24 * 60;

/// How many allowance rows one spend reads.
///
/// A bound rather than a promise: the rows are filtered to one kind and one
/// project, so a store that reaches this has something wrong with it, and the
/// answer then is to spend nothing (see [`Store::spend_allowance`]).
///
/// [`Store::spend_allowance`]: crate::store::Store::spend_allowance
pub const SCAN_LIMIT: usize = 4096;

/// Whether SURE will record an allowance for this long.
///
/// Zero minutes is refused rather than read as "until further notice": a grant
/// that has already expired when it is written is not an allowance, and an
/// answer that accepted it would leave a user believing they had one.
#[must_use]
pub const fn window_is_allowed(minutes: u32) -> bool {
    minutes >= 1 && minutes <= MAX_MINUTES
}

/// One allowance the user recorded.
///
/// The fields are the user's own words, and they are stored rather than
/// interpreted: this module never classifies a subject, and nothing here reads
/// a command line. What a subject *is* — a broad delete, a force push, a read
/// of credentials — is decided when a request arrives and is read from the
/// request, which is the only moment at which there is a request to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// The tool name, exactly as the user typed it.
    pub tool: String,
    /// The command line or the path, exactly as the user typed it.
    pub subject: String,
    /// When the user recorded it, in milliseconds since the epoch.
    pub granted_at_ms: i64,
    /// When it stops being usable, in milliseconds since the epoch.
    pub not_after_ms: i64,
}

/// One request's use of one allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spent {
    /// The row id of the grant this use was spent against.
    ///
    /// The id rather than a counter or a flag, because the store is
    /// append-only and a row's id is stable for the life of the file: it is
    /// what makes *which grant was spent, and when* answerable afterwards
    /// without rewriting the row that was spent.
    pub grant: i64,
    /// When it was spent, in milliseconds since the epoch.
    pub spent_at_ms: i64,
}

/// What one row of the allowance kind holds.
///
/// Two shapes under one kind, because they are two halves of one statement and
/// a reader that has the grants has to have the uses beside them. They are
/// tagged rather than told apart by their fields: a row that carried neither a
/// grant's window nor a spend's grant id would otherwise decode into something
/// plausible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowanceRecord {
    /// The user recorded an allowance.
    Grant(Grant),
    /// One request spent one.
    Spent(Spent),
}

/// Read one stored row as an allowance.
///
/// # Errors
///
/// [`StoreError::Decode`] if the row is not one of the two shapes, which stops
/// the read rather than being skipped — [`crate::approval`]'s rule and
/// [`Store::history`]'s.
pub fn read_back(row: StoredRecord) -> Result<AllowanceRecord, StoreError> {
    debug_assert!(
        row.kind == RecordKind::Allowance,
        "the query asked for one kind and got another"
    );
    row.decode()
}

/// The oldest allowance this request may spend, if there is one.
///
/// Pure, over rows the caller has already read, so that the same question can
/// be asked inside the store's write transaction (where it must be asked
/// atomically) and in a test (where it must be askable without a store).
///
/// Oldest first is the whole of the ordering: a user who recorded two
/// allowances for the same request meant two uses, and spending the older one
/// first spends them in the order they were given. An expired grant is not
/// spent and is not removed — the row stays as what it was: a permission the
/// user gave that nothing used.
///
/// # Errors
///
/// [`StoreError::Decode`] if any allowance row for this project cannot be read.
/// A row SURE cannot read is a row whose state it does not know, and guessing
/// that the unknown row was not a grant would be reading a hole as a "no".
pub fn outstanding(
    rows: &[StoredRecord],
    project_root: &str,
    tool: &str,
    subject: &str,
    now_ms: i64,
) -> Result<Option<i64>, StoreError> {
    let root = folded_root(project_root);
    let mut spent: Vec<i64> = Vec::new();
    let mut grants: Vec<(i64, i64, Grant)> = Vec::new();

    for row in rows {
        if row.kind != RecordKind::Allowance {
            continue;
        }
        if folded_root(row.project_root.as_deref().unwrap_or_default()) != root {
            continue;
        }
        match read_back(row.clone())? {
            AllowanceRecord::Grant(grant) => {
                if grant.tool == tool && grant.subject == subject {
                    grants.push((row.id, row.written_at_ms, grant));
                }
            }
            AllowanceRecord::Spent(use_of) => spent.push(use_of.grant),
        }
    }

    grants.sort_by_key(|(id, written_at_ms, _)| (*written_at_ms, *id));
    Ok(grants
        .into_iter()
        .find(|(id, _, grant)| !spent.contains(id) && grant.not_after_ms > now_ms)
        .map(|(id, _, _)| id))
}

/// Write an allowance down, as the grant the user just made.
///
/// The caller has already decided that the window is one SURE will record: see
/// [`window_is_allowed`], which the command checks before it reaches here.
///
/// # Errors
///
/// [`StoreError::MalformedRow`] if the grant cannot be turned into JSON, plus
/// everything [`Store::append_allowance`] reports, including
/// [`StoreError::Busy`] if another process held the write lock past the store's
/// timeout.
pub fn record(
    store: &Store,
    project_root: &str,
    fingerprint: &FingerprintId,
    tool: &str,
    subject: &str,
    minutes: u32,
    now_ms: i64,
) -> Result<i64, StoreError> {
    let allowance = AllowanceRecord::Grant(Grant {
        tool: tool.to_owned(),
        subject: subject.to_owned(),
        granted_at_ms: now_ms,
        not_after_ms: now_ms.saturating_add(i64::from(minutes) * 60_000),
    });
    store.append_allowance(&allowance, project_root, fingerprint)
}

/// A project root as SURE compares it.
///
/// Separators folded and case folded, on every platform rather than only on
/// Windows: a grant recorded from `C:\work\app` and a request arriving from
/// `c:/work/app` are the same project, and a rule whose answer changed with the
/// spelling of a path would be a rule two machines could disagree about. A
/// trailing separator is dropped, because it names the same directory.
fn folded_root(root: &str) -> String {
    root.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::store::HistoryFilter;

    const MINUTE: i64 = 60_000;

    fn scratch(name: &str) -> std::path::PathBuf {
        crate::store::scratch_root().join(format!("{name}-{}", std::process::id()))
    }

    fn store_in(name: &str) -> Store {
        let dir = scratch(name);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear {}: {error}", dir.display()),
        }
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Store::open_at(&dir.join("sure.db")).expect("the store opens")
    }

    fn fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    fn rows(store: &Store) -> Vec<StoredRecord> {
        store
            .history(
                &HistoryFilter {
                    project_fingerprint: None,
                    kind: Some(RecordKind::Allowance),
                    include_recordings: false,
                },
                SCAN_LIMIT,
            )
            .expect("the rows read back")
    }

    #[test]
    fn a_window_outside_the_bounds_is_not_one_sure_will_record() {
        assert!(!window_is_allowed(0));
        assert!(window_is_allowed(1));
        assert!(window_is_allowed(DEFAULT_MINUTES));
        assert!(window_is_allowed(MAX_MINUTES));
        assert!(!window_is_allowed(MAX_MINUTES + 1));
    }

    #[test]
    fn an_allowance_is_found_by_the_request_it_was_recorded_for() {
        let store = store_in("allowance-found");
        let fingerprint = fingerprint();
        let now = 1_700_000_000_000;
        let id = record(
            &store,
            "C:\\work\\app",
            &fingerprint,
            "Bash",
            "rm -rf build/",
            DEFAULT_MINUTES,
            now,
        )
        .expect("the allowance is written");

        // The same request, from the same project, spelled the other way.
        assert_eq!(
            outstanding(&rows(&store), "c:/work/app/", "Bash", "rm -rf build/", now).unwrap(),
            Some(id)
        );
    }

    #[test]
    fn an_allowance_is_not_found_for_another_tool_subject_or_project() {
        let store = store_in("allowance-not-found");
        let fingerprint = fingerprint();
        let now = 1_700_000_000_000;
        record(
            &store,
            "C:\\work\\app",
            &fingerprint,
            "Bash",
            "rm -rf build/",
            DEFAULT_MINUTES,
            now,
        )
        .expect("the allowance is written");

        let rows = rows(&store);
        for (root, tool, subject) in [
            ("C:\\work\\app", "Shell", "rm -rf build/"),
            ("C:\\work\\app", "Bash", "rm -rf dist/"),
            ("C:\\work\\other", "Bash", "rm -rf build/"),
            // Exact, not a prefix: a subject that merely starts the same way is
            // a different command.
            ("C:\\work\\app", "Bash", "rm -rf build/ && rm -rf /"),
        ] {
            assert_eq!(
                outstanding(&rows, root, tool, subject, now).unwrap(),
                None,
                "{tool} {subject} in {root}"
            );
        }
    }

    #[test]
    fn an_allowance_that_has_expired_is_not_spent() {
        let store = store_in("allowance-expired");
        let now = 1_700_000_000_000;
        record(
            &store,
            "C:\\work\\app",
            &fingerprint(),
            "Bash",
            "rm -rf build/",
            DEFAULT_MINUTES,
            now,
        )
        .expect("the allowance is written");

        let rows = rows(&store);
        assert!(
            outstanding(&rows, "C:\\work\\app", "Bash", "rm -rf build/", now)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            outstanding(
                &rows,
                "C:\\work\\app",
                "Bash",
                "rm -rf build/",
                now + i64::from(DEFAULT_MINUTES) * MINUTE
            )
            .unwrap(),
            None,
            "a grant is usable until its window closes and not at the moment it closes"
        );
    }

    #[test]
    fn an_allowance_that_was_spent_is_not_outstanding_again() {
        let store = store_in("allowance-spent");
        let now = 1_700_000_000_000;
        let id = record(
            &store,
            "C:\\work\\app",
            &fingerprint(),
            "Bash",
            "rm -rf build/",
            DEFAULT_MINUTES,
            now,
        )
        .expect("the allowance is written");

        assert_eq!(
            store
                .spend_allowance("C:\\work\\app", "Bash", "rm -rf build/", now)
                .expect("the spend is written"),
            Some(id)
        );
        assert_eq!(
            outstanding(&rows(&store), "C:\\work\\app", "Bash", "rm -rf build/", now).unwrap(),
            None,
            "one request spends one allowance, and the next one has none"
        );
    }

    #[test]
    fn two_allowances_are_two_uses_oldest_first() {
        let store = store_in("allowance-two");
        let now = 1_700_000_000_000;
        let first = record(
            &store,
            "C:\\work\\app",
            &fingerprint(),
            "Delete",
            ".",
            DEFAULT_MINUTES,
            now,
        )
        .expect("the first allowance is written");
        let second = record(
            &store,
            "C:\\work\\app",
            &fingerprint(),
            "Delete",
            ".",
            DEFAULT_MINUTES,
            now + MINUTE,
        )
        .expect("the second allowance is written");
        assert_ne!(first, second);

        assert_eq!(
            store
                .spend_allowance("C:\\work\\app", "Delete", ".", now + 2 * MINUTE)
                .expect("the first spend"),
            Some(first)
        );
        assert_eq!(
            store
                .spend_allowance("C:\\work\\app", "Delete", ".", now + 3 * MINUTE)
                .expect("the second spend"),
            Some(second)
        );
        assert_eq!(
            store
                .spend_allowance("C:\\work\\app", "Delete", ".", now + 4 * MINUTE)
                .expect("the third spend"),
            None
        );
    }

    /// A row SURE cannot read is not a row it may read past: an unknown row
    /// could have been a grant, and answering "no allowance" for it would be
    /// reading a hole as a no.
    #[test]
    fn a_row_that_is_not_an_allowance_stops_the_question_rather_than_answering_it() {
        let store = store_in("allowance-damaged");
        store
            .append_for(
                RecordKind::Allowance,
                &serde_json::json!({"tool": "Bash"}),
                "C:\\work\\app",
                &fingerprint(),
            )
            .expect("a row of the right kind with the wrong shape");
        match outstanding(&rows(&store), "C:\\work\\app", "Bash", "rm -rf build/", 0) {
            Err(StoreError::Decode { .. }) => {}
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_recording_is_not_an_allowance_and_is_never_counted() {
        // The kind filter is what keeps a recording out of this question, and
        // the row is written with the same project so that only the kind can
        // tell them apart.
        let store = store_in("allowance-recording");
        let fingerprint = fingerprint();
        store
            .append_recording(&serde_json::json!({"tool": "Bash"}), Some("C:\\work\\app"))
            .expect("a recording");
        assert_eq!(
            outstanding(&rows(&store), "C:\\work\\app", "Bash", "rm -rf build/", 0).unwrap(),
            None
        );
        let _ = fingerprint;
    }
}

//! Re-check lifecycle and history.
//!
//! SURE does not trust an agent's claim that a finding is fixed. This module
//! compares a new run's findings and check results with the open findings from
//! previous runs, resolves the ones whose re-check checks now pass, and keeps
//! the ones that still have evidence.
//!
//! Every run's findings and check results are stored as records, so a previous
//! run stays auditable even after a finding is closed.

use std::collections::{HashMap, HashSet};

use sure_domain::finding::{Finding, FindingStatus};
use sure_domain::ids::{CheckId, FindingId, FingerprintId};
use sure_domain::status::{CheckResult, CheckStatus};

use crate::store::{RecordKind, Store, StoreError};

/// A stable identity for a finding across runs.
///
/// Finding IDs are minted per run, so the same issue in two runs would look like
/// two different findings. The key is built from the human title and the first
/// checkable evidence anchor, which is what SURE can point a repair at.
///
/// On Windows, file paths are case-insensitive, so the location/locator are
/// normalised to lowercase. The title is kept exact: two findings with the same
/// location but different user-facing descriptions are different issues.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FindingKey {
    title: String,
    location: String,
    locator: String,
}

impl FindingKey {
    /// Build a key from a finding, if the finding has at least one checkable
    /// anchor.
    #[must_use]
    pub fn from_finding(finding: &Finding) -> Option<Self> {
        let anchor = finding.evidence.iter().find(|e| e.anchor.is_checkable())?;
        Some(Self::new(
            &finding.title,
            &anchor.anchor.location,
            &anchor.anchor.locator,
        ))
    }

    fn new(title: &str, location: &str, locator: &str) -> Self {
        Self {
            title: title.to_owned(),
            location: normalise_path(location),
            locator: normalise_path(locator),
        }
    }
}

fn normalise_path(text: &str) -> String {
    let trimmed = text.trim();
    #[cfg(windows)]
    {
        trimmed.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        trimmed.to_owned()
    }
}

/// What changed between the previous run and the current run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleUpdate {
    /// Findings to report for the current run. Open findings are first, then
    /// resolved ones, each group most-serious first.
    pub findings: Vec<Finding>,
    /// Findings that were open before and that new evidence resolved.
    pub resolved: Vec<Finding>,
    /// Findings that were open before and stayed open.
    pub kept_open: Vec<Finding>,
}

/// Inputs to the re-check lifecycle.
#[derive(Debug, Clone)]
pub struct LifecycleInputs<'a> {
    /// Open findings from previous runs for the same project.
    pub previous_open: &'a [Finding],
    /// Findings produced by the current run.
    pub current_findings: &'a [Finding],
    /// Outcomes of checks run in this pass.
    pub check_results: &'a [CheckResult],
    /// For each previous finding id, the checks that can observe whether it is
    /// fixed. When a finding has no entry here, its re-check is treated as
    /// absent and the finding stays open unless a current finding matches it.
    pub rechecks: &'a [(FindingId, Vec<CheckId>)],
}

/// Reconcile previous findings with current findings and check results.
///
/// The returned [`LifecycleUpdate::findings`] are the ones that belong in the
/// current run's verdict:
///
/// - every current finding, reported as `Open`;
/// - every previous open finding that did not reappear but whose re-check
///   checks all passed, reported as `Resolved`;
/// - every previous open finding that did not reappear and whose re-check
///   checks did not all pass, reported as `Open`.
///
/// A previous finding that matches a current finding by key is considered the
/// same issue; the current finding is reported and the previous one is not
/// duplicated. A previous finding with no matching current finding and no
/// passing re-check stays open, because SURE cannot honestly say it was fixed.
#[must_use]
pub fn reconcile(
    inputs: LifecycleInputs<'_>,
    current_fingerprint: FingerprintId,
) -> LifecycleUpdate {
    let current_keys: HashMap<FindingKey, &Finding> = inputs
        .current_findings
        .iter()
        .filter_map(|f| FindingKey::from_finding(f).map(|k| (k, f)))
        .collect();

    let result_by_id: HashMap<CheckId, CheckStatus> = inputs
        .check_results
        .iter()
        .map(|r| (r.id.clone(), r.status))
        .collect();

    let recheck_map: HashMap<FindingId, &[CheckId]> = inputs
        .rechecks
        .iter()
        .map(|(id, checks)| (id.clone(), checks.as_slice()))
        .collect();

    let mut findings: Vec<Finding> = inputs.current_findings.to_vec();
    let mut resolved = Vec::new();
    let mut kept_open = Vec::new();

    for previous in inputs.previous_open {
        let Some(key) = FindingKey::from_finding(previous) else {
            // A previous finding with no checkable anchor cannot be matched or
            // resolved; carry it forward as-is so it does not vanish.
            findings.push(clone_for_state(
                previous,
                current_fingerprint.clone(),
                FindingStatus::Open,
            ));
            kept_open.push(previous.clone());
            continue;
        };

        if current_keys.contains_key(&key) {
            // The issue still exists in the current run. The current finding is
            // already in `findings`; the previous one is history.
            kept_open.push(previous.clone());
            continue;
        }

        if recheck_passed(&recheck_map, &result_by_id, &previous.id) {
            let closed = clone_for_state(
                previous,
                current_fingerprint.clone(),
                FindingStatus::Resolved,
            );
            findings.push(closed.clone());
            resolved.push(closed);
        } else {
            let carried =
                clone_for_state(previous, current_fingerprint.clone(), FindingStatus::Open);
            findings.push(carried.clone());
            kept_open.push(carried);
        }
    }

    findings.sort_by_key(|f| std::cmp::Reverse(f.severity));
    resolved.sort_by_key(|f| std::cmp::Reverse(f.severity));
    kept_open.sort_by_key(|f| std::cmp::Reverse(f.severity));

    LifecycleUpdate {
        findings,
        resolved,
        kept_open,
    }
}

fn recheck_passed(
    recheck_map: &HashMap<FindingId, &[CheckId]>,
    result_by_id: &HashMap<CheckId, CheckStatus>,
    finding_id: &FindingId,
) -> bool {
    let Some(checks) = recheck_map.get(finding_id) else {
        // No re-check list means SURE does not know how to observe a fix.
        return false;
    };
    if checks.is_empty() {
        return false;
    }
    checks
        .iter()
        .all(|id| result_by_id.get(id) == Some(&CheckStatus::Pass))
}

fn clone_for_state(
    finding: &Finding,
    fingerprint: FingerprintId,
    status: FindingStatus,
) -> Finding {
    Finding {
        id: FindingId::generate(),
        status,
        fingerprint,
        ..finding.clone()
    }
}

/// How many stored records one read of the earlier runs walks.
///
/// # Why this is a bound with a number rather than `0`
///
/// [`Store::history`] documents `0` as *"returns none"* and the implementation
/// binds it as `LIMIT 0`, so a read that is meant to find the previous run's open
/// findings could only ever return an empty list — and the caller then reports
/// *"no earlier run left anything open"* as a fact about the project rather than
/// as a failure to look. That was this function until `P7-T012`.
///
/// # Why this number
///
/// A bound rather than a promise, in the shape of [`crate::allowance::SCAN_LIMIT`]
/// and for the same reason. **The query is newest-first and the project filter is
/// applied in Rust after the limit**, so the budget is spent before this module
/// knows which of the rows belong to the project it was asked about: any limit
/// that is merely "big enough for this project" silently drops that project's
/// older runs, which is the same failure as `0` with a quieter symptom. The
/// number is therefore chosen against the store rather than against a project —
/// far more runs than any project accumulates between two checks, and a store
/// that reaches it has something wrong with it.
///
/// **That is where the resemblance to `allowance::SCAN_LIMIT` ends, and this
/// paragraph used to claim otherwise.** `Store::spend_allowance` reads its bound
/// and then acts on it — `store/mod.rs:583` returns `None` when the scan
/// saturates, and `None` means spend nothing, so an exhausted allowance fails
/// **closed**. [`previous_open_findings`] has no equivalent check and returns
/// `Ok` whether or not the scan saturated, so an empty list here means "nothing
/// was left open" and "I did not read far enough to know" at the same time. The
/// number is the same and the failure direction is opposite: a bound that fails
/// in `spend_allowance` makes SURE *do* less, and a bound that fails here makes
/// SURE *claim* less is wrong, which is a false green rather than a refusal. The
/// 4096 is a judgement about scale and no fixture or test was found that reaches
/// it; the direction is owned by `P15-T029`.
pub const HISTORY_SCAN_LIMIT: usize = 4096;

/// Read every open finding for `project_root` from earlier runs.
///
/// Open here means [`FindingStatus::needs_attention`]: `Open` and
/// `CannotConfirm`. Resolved and accepted findings are not carried forward.
///
/// # One entry per finding, not one per row
///
/// [`store_run`] writes **every** finding on **every** run, so a project checked
/// five times has five stored rows per open finding. Returning them all made the
/// re-check report a count that grew with how often the project had been looked
/// at rather than with how many problems it had — measured, before this rule
/// existed: three runs over one unchanged project, and the third said `6 earlier
/// finding(s) stayed open` for three findings.
///
/// The identity to deduplicate on is [`FindingKey`], because the id is minted per
/// run — that is what the key exists for. [`Store::history`] returns newest-first,
/// so the record kept is the most recent reading of that finding. A finding with
/// no key at all (nothing a reader could be pointed at) is **kept rather than
/// dropped**: SURE cannot tell two of them apart, so it cannot choose between them
/// without discarding a record it has no basis to call a duplicate.
///
/// # Why there is no project-state filter
///
/// There was one, and it could never fire: it skipped a record whose
/// `project_fingerprint` equalled the current run's, but a
/// [`ProjectFingerprint`](sure_domain::vocabulary::ProjectFingerprint) carries a
/// **minted** id beside its content digest, so two runs over identical bytes carry
/// two different ids. The comparison is now gone rather than corrected to
/// [`matches`](sure_domain::vocabulary::ProjectFingerprint::matches), because
/// filtering by state would drop exactly the findings a re-check is for: a finding
/// an earlier run left open against the state the project is still in is the one a
/// reader most needs to see. Deduplicating on the key bounds the list; the state
/// it was raised against does not make it any less open.
///
/// # Errors
///
/// Returns [`StoreError::MalformedRow`] when a stored finding does not decode,
/// and whatever the store returns for a query it cannot answer.
pub fn previous_open_findings(
    store: &Store,
    project_root: &str,
) -> Result<Vec<Finding>, StoreError> {
    let filter = crate::store::HistoryFilter {
        project_fingerprint: None,
        kind: Some(RecordKind::Document(
            sure_protocol::documents::DocumentKind::Finding,
        )),
        include_recordings: false,
    };
    let records = store.history(&filter, HISTORY_SCAN_LIMIT)?;
    let mut seen: HashSet<FindingKey> = HashSet::new();
    let mut findings = Vec::new();
    for record in records {
        if record.project_root.as_deref() != Some(project_root) {
            continue;
        }
        let finding: Finding = record.decode()?;
        if !finding.status.needs_attention() {
            continue;
        }
        if let Some(key) = FindingKey::from_finding(&finding)
            && !seen.insert(key)
        {
            continue;
        }
        findings.push(finding);
    }
    Ok(findings)
}

/// Persist a run's findings and check results so later runs can compare.
pub fn store_run(
    store: &Store,
    project_root: &str,
    fingerprint: &FingerprintId,
    findings: &[Finding],
    check_results: &[CheckResult],
) -> Result<RunRecord, StoreError> {
    let mut finding_rows = Vec::new();
    for finding in findings {
        let document = serde_json::to_value(finding).map_err(|error| StoreError::MalformedRow {
            id: 0,
            message: error.to_string(),
        })?;
        let row = store.append_for(
            RecordKind::Document(sure_protocol::documents::DocumentKind::Finding),
            &document,
            project_root,
            fingerprint,
        )?;
        finding_rows.push(row);
    }

    let mut check_result_rows = Vec::new();
    for result in check_results {
        let document = serde_json::to_value(result).map_err(|error| StoreError::MalformedRow {
            id: 0,
            message: error.to_string(),
        })?;
        let row = store.append_for(
            RecordKind::Document(sure_protocol::documents::DocumentKind::CheckResult),
            &document,
            project_root,
            fingerprint,
        )?;
        check_result_rows.push(row);
    }

    Ok(RunRecord {
        finding_rows,
        check_result_rows,
    })
}

/// Identifiers of a stored run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    pub finding_rows: Vec<i64>,
    pub check_result_rows: Vec<i64>,
}

/// The findings from a run that are still open and relevant to the next run.
///
/// This is a convenience over [`reconcile`] for callers that only need the
/// current open set and not the resolved/new split.
pub fn open_after_reconcile(
    inputs: LifecycleInputs<'_>,
    current_fingerprint: FingerprintId,
) -> Vec<Finding> {
    reconcile(inputs, current_fingerprint)
        .findings
        .into_iter()
        .filter(|f| f.status.needs_attention())
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::evidence::{AnchorSubject, Evidence, EvidenceAnchor, EvidenceClass};
    use sure_domain::finding::{
        AssessmentSource, FindingBuilder, FindingStatus, SeverityRationale,
    };
    use sure_domain::ids::{CheckId, FingerprintId};
    use sure_domain::severity::Severity;

    fn fingerprint() -> FingerprintId {
        FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddee").unwrap()
    }

    fn another_fingerprint() -> FingerprintId {
        FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddef").unwrap()
    }

    /// A store under `target/tmp`, git-ignored and on the same volume as the
    /// checkout. The name carries the process id for the reason `store/mod.rs`'s
    /// copy of this helper records in full: freshness must not depend on a
    /// deletion succeeding, because on Windows a file another process holds
    /// cannot be deleted and the failure is easy to swallow.
    fn store_in(name: &str) -> Store {
        let dir = crate::store::scratch_root().join(format!("{name}-{}", std::process::id()));
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear {}: {error}", dir.display()),
        }
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Store::open_at(&dir.join("sure.db")).expect("the store opens")
    }

    fn finding_with(title: &str, location: &str, status: FindingStatus) -> Finding {
        let fp = fingerprint();
        FindingBuilder::new(
            AssessmentSource::DeterministicCheck,
            SeverityRationale::BlocksHandOff,
        )
        .id(FindingId::generate())
        .title(title)
        .severity(Severity::MustFix)
        .status(status)
        .explanation("the send path returns before the provider is called")
        .user_impact("users believe a message was delivered")
        .next_step("call the provider")
        .fingerprint(fp.clone())
        .evidence(vec![Evidence::new(
            EvidenceClass::DeterministicCheck,
            "the send path returns before the provider is called",
            EvidenceAnchor::new(AnchorSubject::File, location, "line 42"),
            Some(fp),
            Severity::MustFix,
        )])
        .build()
        .expect("fixture finding is valid")
    }

    #[test]
    fn a_new_finding_is_reported_open() {
        let new = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let update = reconcile(
            LifecycleInputs {
                previous_open: &[],
                current_findings: std::slice::from_ref(&new),
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 1);
        assert_eq!(update.findings[0].id, new.id);
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert!(update.resolved.is_empty());
        assert!(update.kept_open.is_empty());
    }

    #[test]
    fn a_reappearing_finding_is_kept_open() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let current = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: std::slice::from_ref(&current),
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 1);
        assert_eq!(update.findings[0].id, current.id);
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert_eq!(update.kept_open.len(), 1);
        assert_eq!(update.kept_open[0].id, previous.id);
        assert!(update.resolved.is_empty());
    }

    #[test]
    fn a_previous_finding_with_passing_rechecks_is_resolved() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let recheck = CheckId::generate();
        let check_result = CheckResult::pass(
            recheck.clone(),
            "email send check",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            another_fingerprint(),
        );
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[check_result],
                rechecks: &[(previous.id.clone(), vec![recheck])],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 1);
        assert_eq!(update.findings[0].status, FindingStatus::Resolved);
        assert_eq!(update.resolved.len(), 1);
        assert!(update.kept_open.is_empty());
    }

    #[test]
    fn a_previous_finding_without_passing_rechecks_stays_open() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let recheck = CheckId::generate();
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[],
                rechecks: &[(previous.id.clone(), vec![recheck])],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 1);
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert!(update.resolved.is_empty());
        assert_eq!(update.kept_open.len(), 1);
    }

    #[test]
    fn a_failed_recheck_keeps_the_finding_open() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let recheck = CheckId::generate();
        let check_result = CheckResult::fail(
            recheck.clone(),
            "email send check",
            Severity::MustFix,
            true,
            EvidenceClass::DeterministicCheck,
            another_fingerprint(),
        );
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[check_result],
                rechecks: &[(previous.id.clone(), vec![recheck])],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 1);
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert!(update.resolved.is_empty());
        assert_eq!(update.kept_open.len(), 1);
    }

    #[test]
    fn all_rechecks_must_pass_to_resolve() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let first = CheckId::generate();
        let second = CheckId::generate();
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[CheckResult::pass(
                    first.clone(),
                    "first",
                    Severity::MustFix,
                    true,
                    EvidenceClass::DeterministicCheck,
                    another_fingerprint(),
                )],
                rechecks: &[(previous.id.clone(), vec![first, second])],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert!(update.resolved.is_empty());
    }

    #[test]
    fn a_finding_without_rechecks_cannot_be_resolved() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert!(update.resolved.is_empty());
    }

    #[test]
    fn resolved_finding_reappears_as_open() {
        let previous = finding_with(
            "Email not sent",
            "src/email/send.rs",
            FindingStatus::Resolved,
        );
        let current = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let update = reconcile(
            LifecycleInputs {
                previous_open: &[],
                current_findings: std::slice::from_ref(&current),
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 1);
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        // The resolved previous finding is not in `previous_open`, so it is
        // ignored.
        assert!(!previous.status.needs_attention());
    }

    #[test]
    fn keys_match_case_insensitively_on_windows() {
        let previous = finding_with("Email not sent", "src/EMAIL/SEND.rs", FindingStatus::Open);
        let current = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: std::slice::from_ref(&current),
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );

        // Stated outside the platform blocks on purpose: these hold whichever
        // answer the platform gives to *is this the same file*, so asserting
        // them inside one branch would leave the other branch's version of them
        // unrun on two of the three machines this suite runs on. The earlier
        // finding stayed open either way, it is a finding of this run rather
        // than last run's record (`clone_for_state` mints this run's id), and
        // its evidence still anchors the file the previous run complained about.
        assert_eq!(update.kept_open.len(), 1);
        assert_eq!(update.kept_open[0].title, previous.title);
        assert_eq!(update.kept_open[0].status, FindingStatus::Open);
        assert_eq!(
            update.kept_open[0].evidence[0].anchor.location,
            "src/EMAIL/SEND.rs"
        );

        // What the platform decides is how many findings a reader sees. Windows
        // folds case in `normalise_path`, so the two spellings are one file and
        // the current finding is the only one reported; on a case-sensitive
        // filesystem they are two files, so the current finding is reported and
        // the previous one is carried forward beside it by `reconcile`'s last
        // `else` — the same carry as
        // `a_previous_finding_without_passing_rechecks_stays_open` and
        // `a_previous_finding_that_did_not_reappear_is_carried_beside_the_current_ones`,
        // which expect the one `kept_open` entry asserted above and do so on
        // every platform. So the two branches agree about everything except the
        // count, and the Unix branch's count is the only claim in this test that
        // cannot be run on the machine this repository is developed on.
        #[cfg(windows)]
        {
            assert_eq!(update.findings.len(), 1);
            // Matched by key, so the entry in `kept_open` is the previous
            // finding itself rather than a fresh copy of it.
            assert_eq!(update.kept_open[0].id, previous.id);
        }
        #[cfg(not(windows))]
        assert_eq!(update.findings.len(), 2);
    }

    /// The shape the case-sensitive branch of that test has, written so that it
    /// holds on every platform: a previous finding for one file and a current
    /// finding for another. The previous finding is neither matched nor
    /// resolvable, so it is reported open *and* counted as kept open — which is
    /// why the `#[cfg(not(windows))]` branch above expects a non-empty
    /// `kept_open` rather than an empty one. Kept platform-independent on
    /// purpose: this is the statement that a Unix-only branch cannot make
    /// locally on the machine this repository is developed on.
    #[test]
    fn a_previous_finding_that_did_not_reappear_is_carried_beside_the_current_ones() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let current = finding_with("Email not sent", "src/email/other.rs", FindingStatus::Open);
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: std::slice::from_ref(&current),
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 2);
        assert_eq!(update.kept_open.len(), 1);
        assert_eq!(update.kept_open[0].title, previous.title);
        assert_eq!(update.kept_open[0].status, FindingStatus::Open);
        // Not the previous id: a carried finding belongs to this run and gets
        // this run's id. The anchor is what says it is the same issue.
        assert_ne!(update.kept_open[0].id, previous.id);
        assert_eq!(
            update.kept_open[0].evidence[0].anchor.location,
            "src/email/send.rs"
        );
        // And it is reported as well as counted: the verdict a reader sees has
        // it in `findings`, and `kept_open` is what the progress sentence counts.
        assert!(
            update
                .findings
                .iter()
                .any(|f| f.id == update.kept_open[0].id && f.status == FindingStatus::Open)
        );
        assert!(update.resolved.is_empty());
    }

    #[test]
    fn findings_are_sorted_most_severe_first() {
        let note = FindingBuilder::new(
            AssessmentSource::ObservedFact,
            SeverityRationale::Informational,
        )
        .id(FindingId::generate())
        .title("note")
        .severity(Severity::Note)
        .status(FindingStatus::Open)
        .fingerprint(fingerprint())
        .evidence(vec![Evidence::new(
            EvidenceClass::ObservedFact,
            "s",
            EvidenceAnchor::new(AnchorSubject::File, "src/a.rs", "line 1"),
            Some(fingerprint()),
            Severity::Note,
        )])
        .build()
        .unwrap();

        let must_fix = finding_with("must fix", "src/b.rs", FindingStatus::Open);

        let update = reconcile(
            LifecycleInputs {
                previous_open: &[],
                current_findings: &[note.clone(), must_fix.clone()],
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings[0].severity, Severity::MustFix);
        assert_eq!(update.findings[1].severity, Severity::Note);
    }

    #[test]
    fn cannot_confirm_is_carried_forward() {
        let previous = finding_with(
            "Email not sent",
            "src/email/send.rs",
            FindingStatus::CannotConfirm,
        );
        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert_eq!(update.kept_open.len(), 1);
    }

    #[test]
    fn a_finding_with_no_checkable_anchor_is_carried_open() {
        let previous = FindingBuilder::new(
            AssessmentSource::ModelAssessment,
            SeverityRationale::Informational,
        )
        .id(FindingId::generate())
        .title("model-only uncertainty")
        .severity(Severity::Note)
        .status(FindingStatus::Open)
        .fingerprint(fingerprint())
        .evidence(vec![Evidence::new(
            EvidenceClass::ModelAssessment,
            "inferred",
            EvidenceAnchor::model_only("model-only conclusion"),
            Some(fingerprint()),
            Severity::Note,
        )])
        .build()
        .unwrap();

        let update = reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[],
                rechecks: &[],
            },
            another_fingerprint(),
        );
        assert_eq!(update.findings.len(), 1);
        assert_eq!(update.findings[0].status, FindingStatus::Open);
        assert_eq!(update.kept_open.len(), 1);
    }

    #[test]
    fn open_after_reconcile_returns_only_attention_needed() {
        let previous = finding_with("Email not sent", "src/email/send.rs", FindingStatus::Open);
        let recheck = CheckId::generate();
        let open = open_after_reconcile(
            LifecycleInputs {
                previous_open: std::slice::from_ref(&previous),
                current_findings: &[],
                check_results: &[CheckResult::pass(
                    recheck.clone(),
                    "email send check",
                    Severity::MustFix,
                    true,
                    EvidenceClass::DeterministicCheck,
                    another_fingerprint(),
                )],
                rechecks: &[(previous.id.clone(), vec![recheck])],
            },
            another_fingerprint(),
        );
        assert!(open.is_empty());
    }

    /// The read that finds what an earlier run left open is only useful if a run
    /// wrote it down and it reports each finding once.
    ///
    /// # Mutations this test is built to redden
    ///
    /// - `HISTORY_SCAN_LIMIT = 0` — the value this read carried before
    ///   `P7-T012`. `Store::history` documents `0` as "return none", and the
    ///   implementation passes it through to SQLite as `LIMIT 0`, so the read
    ///   answers "no earlier run left anything open" about a store holding two
    ///   runs of three open findings. Both count assertions fail.
    /// - `HISTORY_SCAN_LIMIT = 2` — a small non-zero limit, which is the tempting
    ///   fix and is not one: the query is newest-first, so the scan spends its
    ///   rows on the newest readings and never reaches an earlier run's rows at
    ///   all. The first count assertion fails while the store still holds every
    ///   one of the findings.
    /// - deleting the `seen.insert(key)` guard in [`previous_open_findings`] —
    ///   each run stores every finding, so one unchanged project checked twice
    ///   reports six entries for three problems. Measured on the CLI before the
    ///   guard existed: three runs over one unchanged project, and the third
    ///   said `6 earlier finding(s) stayed open` for three findings.
    #[test]
    fn a_previous_runs_open_findings_are_reported_once_each() {
        let store = store_in("recheck-lifecycle-history");
        let project = "C:/projects/sendmail";
        let fixtures = [
            ("Email not sent", "src/email/send.rs"),
            ("Retry loop never ends", "src/email/retry.rs"),
            ("Timeout is one second", "src/email/config.rs"),
        ];
        let open_in = |fixtures: &[(&str, &str)]| -> Vec<Finding> {
            fixtures
                .iter()
                .map(|(title, location)| finding_with(title, location, FindingStatus::Open))
                .collect()
        };

        // Run one: three open findings.
        store_run(&store, project, &fingerprint(), &open_in(&fixtures), &[])
            .expect("the first run is stored");

        // Run two: the same three findings with freshly minted ids — which is
        // what a second run over unchanged bytes produces, because a finding id
        // is generated per run — and one the run itself resolved.
        let mut second = open_in(&fixtures);
        second.push(finding_with(
            "Closed by the repair",
            "src/email/other.rs",
            FindingStatus::Resolved,
        ));
        store_run(&store, project, &fingerprint(), &second, &[]).expect("the second run is stored");

        let mut open: Vec<String> = previous_open_findings(&store, project)
            .expect("the history reads")
            .into_iter()
            .map(|finding| finding.title)
            .collect();
        open.sort();
        assert_eq!(
            open,
            [
                "Email not sent",
                "Retry loop never ends",
                "Timeout is one second"
            ],
            "two runs of three open findings are three findings, and the resolved one is not among them"
        );

        // Another project's finding is not this project's open list, however
        // much of the store it shares.
        store_run(
            &store,
            "C:/projects/other",
            &fingerprint(),
            &[finding_with(
                "Someone else's problem",
                "src/other.rs",
                FindingStatus::Open,
            )],
            &[],
        )
        .expect("the other project's run is stored");
        let mut after: Vec<String> = previous_open_findings(&store, project)
            .expect("the history reads")
            .into_iter()
            .map(|finding| finding.title)
            .collect();
        after.sort();
        assert_eq!(
            after, open,
            "another project's findings are not this project's open list"
        );
    }
}

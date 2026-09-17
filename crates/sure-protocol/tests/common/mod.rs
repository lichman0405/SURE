//! One real instance of each document, shared by the conformance and round-trip
//! tests.
//!
//! Shared rather than duplicated so that the two tests cannot disagree about
//! what a valid document is. If they each built their own, a change that broke
//! conformance could be masked by a round-trip test still passing against a
//! fixture that was never checked.
//!
//! Every fixture is a *realistic* instance, not a minimal one: a finding with an
//! empty evidence list, or a repair contract with no acceptance criteria, would
//! conform to its schema and would not be something SURE would ever write. The
//! tests are about the documents SURE actually produces.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Cargo compiles this module once per test target, and each target uses a
// different subset of the fixtures. Without this, a fixture used by one target
// warns in the other, and the warning that matters — a fixture no test uses at
// all — becomes impossible to see among the noise.
#![allow(dead_code)]

use serde_json::{Value, json};
use sure_domain::capability::CapabilityTier;
use sure_domain::evidence::{
    AnchorSubject, ClaimAssessment, Evidence, EvidenceAnchor, EvidenceClass,
};
use sure_domain::finding::{
    AssessmentSource, Finding, FindingBuilder, FindingStatus, SeverityRationale,
};
use sure_domain::ids::{CheckId, ClaimId, FindingId, FingerprintId, RepairId, SessionId};
use sure_domain::intent::{IntentSource, Requirement};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, NotCheckedReason};
use sure_domain::vocabulary::{Claim, GitState, ProjectFingerprint, RepairContract};
use sure_protocol::documents::DocumentKind;
use sure_protocol::event::EventEnvelope;

/// A fingerprint for the fixtures below.
///
/// Fixed rather than generated, so that a round-trip failure names a value that
/// is the same on every run.
pub fn fingerprint() -> FingerprintId {
    FingerprintId::parse("fp_01j2m8q5aaaabbbbccccddddee")
        .expect("a well-formed fingerprint identifier")
}

/// One anchor, so the evidence arrays are real rather than empty.
pub fn evidence(class: EvidenceClass) -> Evidence {
    Evidence::new(
        class,
        "the send path only writes to the terminal",
        EvidenceAnchor::new(AnchorSubject::File, "src/email/send.rs", "line 42"),
        Some(fingerprint()),
        Severity::MustFix,
    )
}

pub fn finding() -> Finding {
    FindingBuilder::new(
        AssessmentSource::DeterministicCheck,
        SeverityRationale::BlocksHandOff,
    )
    .id(FindingId::generate())
    .title("Email is reported as sent but nothing is sent")
    .severity(Severity::MustFix)
    .status(FindingStatus::Open)
    .explanation("The app says the email went out, and the code only prints a line.")
    .user_impact("People believe a message was delivered when it was not.")
    .next_step("Call the configured provider on the real send path.")
    .evidence(vec![evidence(EvidenceClass::DeterministicCheck)])
    .fingerprint(fingerprint())
    .technical_details(json!({"file": "src/email/send.rs", "line": 42}))
    .build()
    .expect("fixture finding is valid")
}

pub fn claim() -> Claim {
    Claim {
        id: ClaimId::generate(),
        claim_text: "All tests pass and the feature is complete.".to_owned(),
        claim_type: "tests_pass".to_owned(),
        assessment: ClaimAssessment::Contradicted,
        evidence: vec![evidence(EvidenceClass::ObservedFact)],
        session: Some(SessionId::generate()),
    }
}

pub fn check_result() -> CheckResult {
    CheckResult::fail(
        CheckId::generate(),
        "the provider is called on the real send path",
        Severity::MustFix,
        true,
        EvidenceClass::DeterministicCheck,
        fingerprint(),
    )
    .with_reason("the send path returns before the provider is called")
}

pub fn requirement() -> Requirement {
    Requirement::new(
        "req-1",
        "Users must be able to reset their password by email.",
        IntentSource::ExplicitUserGoal,
    )
    .with_raw_retained(true)
}

pub fn repair() -> RepairContract {
    RepairContract {
        id: RepairId::generate(),
        issue_id: FindingId::generate(),
        problem: "The app reports that an email was sent, but the real send path only logs."
            .to_owned(),
        why_it_matters: "Users are told an action succeeded when no email leaves the app."
            .to_owned(),
        required_fix: vec!["Call the configured email provider on the real send path.".to_owned()],
        preserve: vec!["The current successful UI flow.".to_owned()],
        acceptance: vec!["The provider is called on a successful send.".to_owned()],
        recheck: vec![CheckId::generate()],
        forbidden_shortcuts: vec!["Logging a line that looks like a provider response.".to_owned()],
        evidence: vec![evidence(EvidenceClass::DeterministicCheck)],
    }
}

pub fn envelope() -> EventEnvelope {
    EventEnvelope::new("claude-code", "tool.completed", "2026-09-14T09:10:56.827Z")
        .with_capability_tier(CapabilityTier::Observed)
        .with_session_id("harness-7")
        .with_project_root("C:\\work\\my project")
        .with_payload(json!({"tool": "Bash", "exit_code": 0}))
}

/// A content fingerprint with no Git state, which is the shape a project
/// outside a repository produces.
pub fn content_fingerprint() -> ProjectFingerprint {
    ProjectFingerprint::content("sha256:0f1e2d3c")
}

/// A Git-derived fingerprint, which carries an optional sub-object.
pub fn git_fingerprint() -> ProjectFingerprint {
    ProjectFingerprint::git(
        "abc123",
        GitState {
            head: "deadbeef".to_owned(),
            dirty: true,
            dirty_digest: Some("d1g3st".to_owned()),
            untracked_digest: None,
            branch: Some("main".to_owned()),
        },
    )
}

/// A check that could not run, which is the case that must never look like a
/// pass.
pub fn not_run_result() -> CheckResult {
    CheckResult::not_run(
        CheckId::generate(),
        "start the app and load the home page",
        Severity::MustFix,
        true,
        NotCheckedReason::ExecutionNotAuthorized,
        fingerprint(),
    )
}

/// The JSON SURE would write for one document, and the schema that describes it.
///
/// The pairing is the point: a case cannot check a document against the wrong
/// schema, because there is no way to name them separately.
pub fn documents() -> Vec<(DocumentKind, Value)> {
    vec![
        (
            DocumentKind::Finding,
            serde_json::to_value(finding()).expect("a finding serializes"),
        ),
        (
            DocumentKind::Claim,
            serde_json::to_value(claim()).expect("a claim serializes"),
        ),
        (
            DocumentKind::CheckResult,
            serde_json::to_value(check_result()).expect("a check result serializes"),
        ),
        (
            DocumentKind::ProjectIntent,
            serde_json::to_value(requirement()).expect("a requirement serializes"),
        ),
        (
            DocumentKind::Repair,
            serde_json::to_value(repair()).expect("a repair contract serializes"),
        ),
        (
            DocumentKind::Event,
            serde_json::to_value(envelope()).expect("an envelope serializes"),
        ),
    ]
}

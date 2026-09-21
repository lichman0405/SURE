//! Golden tests that lock down plain-language output across all report forms.
//!
//! Each scenario builds a representative [`ProjectVerdict`] and asserts that
//! terminal, Markdown, HTML and JSON reports contain the expected phrases and
//! do not contain forbidden ones.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_cli::human_report::{HumanReportSettings, render_verdict};
use sure_cli::json_report::render_json_report;
use sure_cli::portable_report::{render_html, render_markdown};
use sure_core::capability::CapabilityReport;
use sure_core::evidence::{
    AnchorSubject, ClaimAssessment, Evidence, EvidenceAnchor, EvidenceClass,
};
use sure_core::ids::{CheckId, ClaimId, FingerprintId};
use sure_core::intent::{IntentSource, ProjectIntent, Requirement};
use sure_core::severity::Severity;
use sure_core::status::{
    Aggregate, AggregateSeverity, CheckResult, CheckStatus, CoverageSummary, NotCheckedReason,
    StatusCounts,
};
use sure_core::vocabulary::{
    AssessmentSource, Claim, FindingBuilder, FindingStatus, ProjectVerdict, SeverityRationale,
};

fn fingerprint() -> FingerprintId {
    FingerprintId::generate()
}

fn green_aggregate() -> Aggregate {
    Aggregate {
        severity: AggregateSeverity::Green,
        headline: AggregateSeverity::Green.headline().to_owned(),
        counts: StatusCounts::tally([CheckStatus::Pass]),
        blocking: Vec::new(),
        coverage: CoverageSummary {
            counts: StatusCounts::tally([CheckStatus::Pass]),
            critical_not_checked: Vec::new(),
            critical_errored: Vec::new(),
            critical_failed: Vec::new(),
            critical_out_of_scope: Vec::new(),
            critical_checked: 1,
        },
    }
}

fn not_enough_checked_aggregate() -> Aggregate {
    Aggregate {
        severity: AggregateSeverity::NotEnoughChecked,
        headline: AggregateSeverity::NotEnoughChecked.headline().to_owned(),
        counts: StatusCounts::default(),
        blocking: Vec::new(),
        coverage: CoverageSummary {
            counts: StatusCounts::default(),
            critical_not_checked: Vec::new(),
            critical_errored: Vec::new(),
            critical_failed: Vec::new(),
            critical_out_of_scope: Vec::new(),
            critical_checked: 0,
        },
    }
}

fn a_finding(severity: Severity) -> sure_core::vocabulary::Finding {
    let fp = fingerprint();
    let source = if severity.blocks_hand_off() {
        AssessmentSource::DeterministicCheck
    } else {
        AssessmentSource::ObservedFact
    };
    let rationale = SeverityRationale::for_severity(severity).unwrap();
    FindingBuilder::new(source, rationale)
        .title("A finding")
        .severity(severity)
        .status(FindingStatus::Open)
        .explanation("Something is wrong.")
        .user_impact("It matters.")
        .next_step("Fix it.")
        .fingerprint(fp.clone())
        .evidence(vec![Evidence::new(
            EvidenceClass::ObservedFact,
            "evidence",
            EvidenceAnchor::new(AnchorSubject::File, "src/x.rs", "line 1"),
            Some(fp.clone()),
            severity,
        )])
        .build()
        .expect("valid finding")
}

fn model_only_finding() -> sure_core::vocabulary::Finding {
    let fp = fingerprint();
    FindingBuilder::new(
        AssessmentSource::ModelAssessment,
        SeverityRationale::Informational,
    )
    .title("Model-only uncertainty")
    .severity(Severity::Note)
    .status(FindingStatus::Open)
    .explanation("The model inferred something but cannot point to a project location.")
    .user_impact("This may be nothing.")
    .next_step("Look for a concrete symptom.")
    .fingerprint(fp.clone())
    .evidence(vec![Evidence::new(
        EvidenceClass::ModelAssessment,
        "inferred from prompt",
        EvidenceAnchor::model_only("model-only conclusion"),
        Some(fp.clone()),
        Severity::Note,
    )])
    .build()
    .expect("valid note finding")
}

fn build_verdict(
    aggregate: Aggregate,
    findings: Vec<sure_core::vocabulary::Finding>,
    not_checked: Vec<CheckResult>,
) -> ProjectVerdict {
    ProjectVerdict {
        fingerprint: fingerprint(),
        aggregate,
        intent: ProjectIntent::empty(),
        capability: CapabilityReport::cli(),
        findings,
        not_checked,
        claim_checks: Vec::new(),
    }
}

fn trusted_intent() -> ProjectIntent {
    ProjectIntent::from_requirements(vec![Requirement::new(
        "r1",
        "build a thing",
        IntentSource::ExplicitUserGoal,
    )])
}

fn assert_deterministic<F: Fn(&ProjectVerdict) -> String>(verdict: &ProjectVerdict, render: F) {
    let a = render(verdict);
    let b = render(verdict);
    assert_eq!(a, b, "rendering must be deterministic for the same verdict");
}

fn assert_html_well_formed(html: &str) {
    assert!(
        html.starts_with("<!doctype html>"),
        "html must start with doctype: {html}"
    );
    assert!(html.contains("<html"), "html must open html tag: {html}");
    assert!(html.contains("</html>"), "html must close html tag: {html}");
    assert!(
        html.contains("<title>SURE Report</title>"),
        "html must have title: {html}"
    );
    for ch in html.chars() {
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            panic!(
                "html contains raw control character U+{:04X}: {html:?}",
                ch as u32
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Green project, no findings, trusted intent
// ---------------------------------------------------------------------------
#[test]
fn green_trusted_says_ready_and_no_caveat() {
    let verdict = ProjectVerdict {
        fingerprint: fingerprint(),
        aggregate: green_aggregate(),
        intent: trusted_intent(),
        capability: CapabilityReport::cli(),
        findings: Vec::new(),
        not_checked: Vec::new(),
        claim_checks: Vec::new(),
    };

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    // Terminal
    assert!(term.contains("ready to hand off"), "terminal: {term}");
    assert!(
        !term.contains("cannot confirm that it matches your original request"),
        "terminal must not have caveat when intent is trusted: {term}"
    );

    // Markdown
    assert!(md.contains("ready to hand off"), "markdown: {md}");
    assert!(
        !md.contains("cannot confirm that it matches your original request"),
        "markdown must not have caveat when intent is trusted: {md}"
    );

    // HTML
    assert!(html.contains("ready to hand off"), "html: {html}");
    assert!(
        !html.contains("cannot confirm that it matches your original request"),
        "html must not have caveat when intent is trusted: {html}"
    );

    // JSON
    assert!(json.contains("\"severity\":\"green\""), "json: {json}");
    assert!(json.contains("\"ready_for_hand_off\":true"), "json: {json}");
    assert!(
        json.contains("\"must_caveat_requirements\":false"),
        "json: {json}"
    );

    assert_deterministic(&verdict, |v| {
        render_verdict(v, HumanReportSettings::default())
    });
    assert_deterministic(&verdict, render_markdown);
    assert_deterministic(&verdict, render_html);
    assert_deterministic(&verdict, render_json_report);
    assert_html_well_formed(&html);
}

// ---------------------------------------------------------------------------
// 2. Green aggregate + open must-fix finding → false-green protection
// ---------------------------------------------------------------------------
#[test]
fn green_with_must_fix_says_not_ready() {
    let verdict = build_verdict(
        green_aggregate(),
        vec![a_finding(Severity::MustFix)],
        Vec::new(),
    );

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    // All forms must say not ready
    assert!(
        term.contains("This project is not ready to hand off."),
        "terminal: {term}"
    );
    assert!(
        md.contains("This project is not ready to hand off."),
        "markdown: {md}"
    );
    assert!(
        html.contains("This project is not ready to hand off."),
        "html: {html}"
    );

    // Must list the finding
    assert!(term.contains("A finding"), "terminal: {term}");
    assert!(md.contains("A finding"), "markdown: {md}");
    assert!(html.contains("A finding"), "html: {html}");

    // Must not claim the project is ready (the exact sentence from the summary)
    assert!(
        !term.contains("This project looks ready to hand off."),
        "terminal: {term}"
    );
    assert!(
        !md.contains("This project looks ready to hand off."),
        "markdown: {md}"
    );
    assert!(
        !html.contains("This project looks ready to hand off."),
        "html: {html}"
    );

    // JSON
    assert!(
        json.contains("\"ready_for_hand_off\":false"),
        "json: {json}"
    );
    assert!(
        json.contains("\"severity\":\"green\""),
        "json aggregate must still be green: {json}"
    );

    assert_deterministic(&verdict, |v| {
        render_verdict(v, HumanReportSettings::default())
    });
    assert_deterministic(&verdict, render_markdown);
    assert_deterministic(&verdict, render_html);
    assert_deterministic(&verdict, render_json_report);
    assert_html_well_formed(&html);
}

// ---------------------------------------------------------------------------
// 3. Not enough checked
// ---------------------------------------------------------------------------
#[test]
fn not_enough_checked_says_so() {
    let verdict = build_verdict(not_enough_checked_aggregate(), Vec::new(), Vec::new());

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    assert!(
        term.contains("Not enough could be checked"),
        "terminal: {term}"
    );
    assert!(md.contains("Not enough could be checked"), "markdown: {md}");
    assert!(html.contains("Not enough could be checked"), "html: {html}");

    assert!(
        term.contains("This project is not ready to hand off."),
        "terminal: {term}"
    );
    assert!(
        md.contains("This project is not ready to hand off."),
        "markdown: {md}"
    );
    assert!(
        html.contains("This project is not ready to hand off."),
        "html: {html}"
    );

    assert!(
        json.contains("\"severity\":\"not_enough_checked\""),
        "json: {json}"
    );
    assert!(
        json.contains("\"ready_for_hand_off\":false"),
        "json: {json}"
    );

    assert_deterministic(&verdict, |v| {
        render_verdict(v, HumanReportSettings::default())
    });
    assert_deterministic(&verdict, render_markdown);
    assert_deterministic(&verdict, render_html);
    assert_deterministic(&verdict, render_json_report);
    assert_html_well_formed(&html);
}

// ---------------------------------------------------------------------------
// 4. After-the-fact intent → caveat appears
// ---------------------------------------------------------------------------
#[test]
fn after_the_fact_caveat_appears() {
    let verdict = build_verdict(green_aggregate(), Vec::new(), Vec::new());

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    assert!(
        term.contains("cannot confirm that it matches your original request"),
        "terminal: {term}"
    );
    assert!(
        md.contains("cannot confirm that it matches your original request"),
        "markdown: {md}"
    );
    assert!(
        html.contains("cannot confirm that it matches your original request"),
        "html: {html}"
    );

    assert!(
        json.contains("\"must_caveat_requirements\":true"),
        "json: {json}"
    );
    assert!(
        json.contains("\"caveat\":\""),
        "json must contain caveat text: {json}"
    );

    assert_deterministic(&verdict, |v| {
        render_verdict(v, HumanReportSettings::default())
    });
    assert_deterministic(&verdict, render_markdown);
    assert_deterministic(&verdict, render_html);
    assert_deterministic(&verdict, render_json_report);
    assert_html_well_formed(&html);
}

// ---------------------------------------------------------------------------
// 5. Not-checked checks → visible with titles and reasons
// ---------------------------------------------------------------------------
#[test]
fn not_checked_checks_are_visible_with_reasons() {
    let not_checked = vec![
        CheckResult::not_run(
            CheckId::generate(),
            "run tests",
            Severity::MustFix,
            true,
            NotCheckedReason::ExecutionNotAuthorized,
            fingerprint(),
        ),
        CheckResult::errored(
            CheckId::generate(),
            "lint check",
            Severity::ShouldFixFirst,
            false,
            "the linter crashed",
            fingerprint(),
        ),
    ];
    let verdict = build_verdict(green_aggregate(), Vec::new(), not_checked);

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    // Section heading
    assert!(term.contains("Could not check"), "terminal: {term}");
    assert!(md.contains("Could not check"), "markdown: {md}");
    assert!(html.contains("Could not check"), "html: {html}");

    // Titles
    assert!(term.contains("run tests"), "terminal: {term}");
    assert!(term.contains("lint check"), "terminal: {term}");
    assert!(md.contains("run tests"), "markdown: {md}");
    assert!(md.contains("lint check"), "markdown: {md}");
    assert!(html.contains("run tests"), "html: {html}");
    assert!(html.contains("lint check"), "html: {html}");

    // Reasons
    assert!(
        term.contains("running your project's code") || term.contains("ExecutionNotAuthorized"),
        "terminal must show reason for skipped check: {term}"
    );
    assert!(
        term.contains("the linter crashed"),
        "terminal must show reason for errored check: {term}"
    );
    assert!(
        md.contains("running your project's code") || md.contains("ExecutionNotAuthorized"),
        "markdown must show reason for skipped check: {md}"
    );
    assert!(
        md.contains("the linter crashed"),
        "markdown must show reason for errored check: {md}"
    );
    assert!(
        html.contains("running your project") || html.contains("ExecutionNotAuthorized"),
        "html must show reason for skipped check: {html}"
    );
    assert!(
        html.contains("the linter crashed"),
        "html must show reason for errored check: {html}"
    );

    // Critical marker
    assert!(term.contains("[critical]"), "terminal: {term}");
    assert!(md.contains("[critical]"), "markdown: {md}");
    assert!(html.contains("[critical]"), "html: {html}");

    // JSON
    assert!(json.contains("\"title\":\"run tests\""), "json: {json}");
    assert!(json.contains("\"title\":\"lint check\""), "json: {json}");

    assert_deterministic(&verdict, |v| {
        render_verdict(v, HumanReportSettings::default())
    });
    assert_deterministic(&verdict, render_markdown);
    assert_deterministic(&verdict, render_html);
    assert_deterministic(&verdict, render_json_report);
    assert_html_well_formed(&html);
}

// ---------------------------------------------------------------------------
// 6. Model-only finding → flagged and checkable anchors omitted
// ---------------------------------------------------------------------------
#[test]
fn model_only_finding_is_flagged_and_omits_anchors() {
    let verdict = build_verdict(green_aggregate(), vec![model_only_finding()], Vec::new());

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    // Flagged as model-only
    assert!(
        term.contains("model or inference only — this may be wrong"),
        "terminal: {term}"
    );
    assert!(
        md.contains("model or inference only — this may be wrong"),
        "markdown: {md}"
    );
    assert!(
        html.contains("model or inference only — this may be wrong"),
        "html: {html}"
    );

    // No checkable file anchors
    assert!(
        !term.contains("at src/"),
        "terminal must omit file anchors: {term}"
    );
    assert!(
        !md.contains("at `src/"),
        "markdown must omit file anchors: {md}"
    );
    assert!(
        !html.contains("<code>src/"),
        "html must omit file anchors: {html}"
    );

    // JSON
    assert!(json.contains("\"is_model_only\":true"), "json: {json}");
    assert!(
        json.contains("\"evidence_anchors\":[]"),
        "json must have empty anchors: {json}"
    );

    assert_deterministic(&verdict, |v| {
        render_verdict(v, HumanReportSettings::default())
    });
    assert_deterministic(&verdict, render_markdown);
    assert_deterministic(&verdict, render_html);
    assert_deterministic(&verdict, render_json_report);
    assert_html_well_formed(&html);
}

// ---------------------------------------------------------------------------
// 7. AI-claim checks → explained plainly across all report forms
// ---------------------------------------------------------------------------
fn a_claim(assessment: ClaimAssessment, text: &str) -> Claim {
    Claim {
        id: ClaimId::generate(),
        claim_text: text.to_owned(),
        claim_type: "test_ran".to_owned(),
        assessment,
        reason: String::from("A recorded harness event supports this claim."),
        evidence: Vec::new(),
        session: None,
    }
}

#[test]
fn ai_claim_section_explains_assessments_plainly() {
    let verdict = ProjectVerdict {
        fingerprint: fingerprint(),
        aggregate: green_aggregate(),
        intent: trusted_intent(),
        capability: CapabilityReport::cli(),
        findings: Vec::new(),
        not_checked: Vec::new(),
        claim_checks: vec![
            a_claim(ClaimAssessment::Confirmed, "I ran the tests"),
            a_claim(
                ClaimAssessment::CannotConfirm,
                "I fixed every outstanding bug",
            ),
        ],
    };

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    assert!(term.contains("AI claims"), "terminal: {term}");
    assert!(md.contains("## AI claims"), "markdown: {md}");
    assert!(html.contains("id=\"claims\""), "html: {html}");
    assert!(json.contains("\"claim_checks\""), "json: {json}");

    assert!(term.contains("Confirmed"), "terminal: {term}");
    assert!(md.contains("Confirmed"), "markdown: {md}");
    assert!(html.contains("Confirmed"), "html: {html}");
    assert!(
        json.contains("\"assessment\":\"Confirmed\""),
        "json: {json}"
    );

    assert!(term.contains("Cannot confirm"), "terminal: {term}");
    assert!(md.contains("Cannot confirm"), "markdown: {md}");
    assert!(html.contains("Cannot confirm"), "html: {html}");
    assert!(
        json.contains("\"assessment\":\"Cannot confirm\""),
        "json: {json}"
    );

    assert_deterministic(&verdict, |v| {
        render_verdict(v, HumanReportSettings::default())
    });
    assert_deterministic(&verdict, render_markdown);
    assert_deterministic(&verdict, render_html);
    assert_deterministic(&verdict, render_json_report);
    assert_html_well_formed(&html);
}

//! The verdict sentence appears once in each report form.
//!
//! `sure_core::project_verdict::render_summary` is self-contained: its first line
//! is `verdict.aggregate.headline`. Every renderer here pushed that headline a
//! second time above the summary, so the sentence a reader meets first was
//! printed twice with nothing between the copies but a blank line — measured on a
//! real `sure check` run and recorded in `docs/development/DOGFOOD.md`, §7.1.
//!
//! The terminal, Markdown and HTML forms are all checked here, and the JSON frame
//! is checked with them: it carries the headline once, under
//! `aggregate.headline`, and never carried `render_summary`, so the doubling
//! never reached a script.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_cli::human_report::{HumanReportSettings, render_verdict};
use sure_cli::json_report::render_json_report;
use sure_cli::portable_report::{render_html, render_markdown};
use sure_core::capability::CapabilityReport;
use sure_core::ids::FingerprintId;
use sure_core::intent::ProjectIntent;
use sure_core::status::{Aggregate, AggregateSeverity, CoverageSummary, StatusCounts};
use sure_core::vocabulary::ProjectVerdict;

/// The verdict a first run on a project with nothing to check produces, which is
/// the one this defect was measured on: the headline and the summary's first
/// sentence are the same frozen string.
fn a_verdict() -> ProjectVerdict {
    let headline = AggregateSeverity::NotEnoughChecked.headline().to_owned();
    ProjectVerdict {
        fingerprint: FingerprintId::generate(),
        aggregate: Aggregate {
            severity: AggregateSeverity::NotEnoughChecked,
            headline,
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
        },
        intent: ProjectIntent::empty(),
        capability: CapabilityReport::cli(),
        findings: Vec::new(),
        not_checked: Vec::new(),
        claim_checks: Vec::new(),
    }
}

#[test]
fn the_verdict_sentence_appears_once_in_every_report_form() {
    let verdict = a_verdict();
    let sentence = verdict.aggregate.headline.clone();

    let term = render_verdict(&verdict, HumanReportSettings::default());
    let md = render_markdown(&verdict);
    let html = render_html(&verdict);
    let json = render_json_report(&verdict);

    // Printed rather than only asserted, so a failure here shows the reader what
    // the report actually says instead of a count.
    println!("--- terminal ---\n{term}\n--- markdown ---\n{md}\n--- html ---\n{html}\n");

    for (form, text) in [
        ("terminal", &term),
        ("markdown", &md),
        ("html", &html),
        ("json", &json),
    ] {
        assert_eq!(
            text.matches(&sentence).count(),
            1,
            "the verdict sentence {sentence:?} appears {} time(s) in the {form} report, and a \
             reader should meet it once:\n{text}",
            text.matches(&sentence).count()
        );
    }
}

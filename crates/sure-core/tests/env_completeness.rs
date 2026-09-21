//! Environment and configuration key completeness, end to end.
//!
//! `crate::references`'s own tests hold the reading and
//! `crate::env_completeness`'s hold the sentences; this file drives the pair
//! over real directories, which is where they have to add up to something true
//! about a project — and where `P4-T006`'s acceptance sentence can be checked.
//!
//! # The claim this file exists to hold
//!
//! > Missing key/documentation mismatches are reported without requiring secret
//! > values.
//!
//! It is two claims, and neither is the kind a test is naturally good at.
//!
//! **"Reported"** is a claim about a list, so every test below asserts the
//! verdict *and* the sentence, in the way `tests/setup_validation.rs` does for
//! its own. A check that answered `Cannot confirm` to everything would fail
//! every one of them.
//!
//! **"Without requiring secret values"** is a claim about something that did not
//! happen, and `assert!(!text.contains(CANARY))` is equally true when the value
//! was never in the fixture, when the file was never opened, and when the report
//! is empty because the pass did nothing. Every canary test here therefore
//! asserts three things together — the key **was** found, the file **was** read,
//! and the canary is in none of the four surfaces a report has: the `Debug`
//! rendering, a claim's reason, a claim's `plain_description`, and an evidence
//! anchor's excerpt. `the_helper_that_looks_for_the_canary_can_see_a_canary_that_did_reach_a_report`
//! is what holds that search to account, because a search that can never find
//! anything is not a search.
//!
//! The anchor's `excerpt` is the field worth naming: it is the one place
//! `EvidenceAnchor` offers for the text a finding was read from, and the line a
//! key is read on is the line its value is on. `no_claim_quotes_a_project_line`
//! asks about it across a whole project rather than one claim.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::env_completeness::{AssessedKey, CompletenessReport};
use sure_core::references::{KeyStatus, ReferenceOptions};
use sure_domain::evidence::ClaimAssessment;
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

/// A value that must never reach a report.
///
/// Shaped like a live credential, because a redaction bug is usually a bug about
/// *shape* — a rule that skips values starting with `sk-`, or one that stops at
/// the first `-`. A canary that looked like `secret123` would exercise none of
/// that.
const CANARY: &str = "sk-live-CANARY-9f3a2b7c4d";

/// A directory under the workspace's git-ignored `target/tmp`.
///
/// The claiming rules live in `sure_testkit::scratch`, and the history that put
/// them there is this repository's: clearing a fixed path with
/// `let _ = remove_dir_all(..)` and then treating it as fresh fails on Windows,
/// and the test then describes a directory that was never emptied. So nothing
/// here is adopted — a directory is taken with `create_dir`, which fails when
/// the name is taken, and one that is already there is skipped rather than
/// entered — and the helper clears only directories carrying *its own*
/// process's id, which no live process can own.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        Self {
            project: sure_testkit::scratch::directory("env completeness", test),
        }
    }

    fn write(&self, relative: &str, contents: &str) -> &Self {
        let full = self.project.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        std::fs::write(&full, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        self
    }

    fn discovery(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default()).expect("discover")
    }

    fn report(&self) -> CompletenessReport {
        CompletenessReport::of(&self.discovery(), &FingerprintId::generate())
    }

    fn report_with(&self, options: &ReferenceOptions) -> CompletenessReport {
        CompletenessReport::with_options(&self.discovery(), options, &FingerprintId::generate())
    }

    fn report_at(&self, fingerprint: &FingerprintId) -> CompletenessReport {
        CompletenessReport::of(&self.discovery(), fingerprint)
    }
}

/// Everything a report says, as one string, for a canary to be looked for in.
///
/// `Debug` rather than a rendered report, because `Debug` is what a log line and
/// a failing assertion print, and it is the surface a leaked field would reach
/// without anybody choosing to render it. It is the strictest of the four.
fn everything(report: &CompletenessReport) -> String {
    let mut text = format!("{report:?}");
    for claim in report.claims() {
        text.push_str(&claim.plain_description());
        text.push('\n');
        for evidence in claim.evidence() {
            text.push_str(&format!("{evidence:?}\n"));
        }
    }
    text
}

/// The claim about one key, or a panic naming what the report has instead.
fn claim_for<'a>(report: &'a CompletenessReport, key: &str) -> &'a AssessedKey {
    report
        .claims()
        .iter()
        .find(|claim| claim.key() == key)
        .unwrap_or_else(|| {
            let found: Vec<&str> = report.claims().iter().map(AssessedKey::key).collect();
            panic!("no claim about {key}; the report has {found:?}")
        })
}

/// The keys the report makes a claim about, sorted, for a readable assertion.
fn claimed_keys(report: &CompletenessReport) -> Vec<String> {
    let mut keys: Vec<String> = report
        .claims()
        .iter()
        .map(|claim| claim.key().to_owned())
        .collect();
    keys.sort();
    keys
}

#[test]
fn a_project_whose_two_sides_disagree_is_settled_one_key_at_a_time() {
    let fixture = Fixture::new("disagreement");
    fixture
        .write(
            ".env.example",
            "# Copy this file to .env and fill it in.\n\
             DATABASE_URL=postgres://user:password@localhost/app\n\
             LEGACY_TOKEN=\n",
        )
        .write(
            "src/app.ts",
            "const url = process.env.DATABASE_URL;\n\
             const key = process.env.STRIPE_KEY;\n\
             const mode = process.env.NODE_ENV;\n",
        );

    let report = fixture.report();
    assert!(report.is_complete(), "a two-file project is read whole");
    assert_eq!(
        report.counts(),
        vec![
            (ClaimAssessment::Confirmed, 3),
            (ClaimAssessment::Contradicted, 0),
            (ClaimAssessment::CannotConfirm, 0),
            (ClaimAssessment::NotCheckable, 0),
        ]
    );

    // The key the two sides agree about makes no claim at all, and is still
    // visible where the reading put it — a report that dropped it would make
    // "no disagreement" indistinguishable from "not looked at".
    assert!(!claimed_keys(&report).contains(&"DATABASE_URL".to_owned()));
    let agreed = report.references().get("DATABASE_URL").expect("read");
    assert_eq!(agreed.status, KeyStatus::ReadAndDeclared);
    assert!(agreed.is_read() && agreed.is_declared());

    // Read and named nowhere. The sentence names the file and the line, so a
    // reader can go and look.
    let stripe = claim_for(&report, "STRIPE_KEY");
    assert_eq!(stripe.status(), KeyStatus::ReadButNotDeclared);
    assert_eq!(stripe.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(stripe.severity(), Severity::ShouldFixFirst);
    assert!(!stripe.is_set_aside());
    assert!(
        stripe
            .reason()
            .contains("`src/app.ts` line 2 asks for this key"),
        "{}",
        stripe.reason()
    );
    assert!(
        stripe
            .reason()
            .contains("no example file or document in this project names it"),
        "{}",
        stripe.reason()
    );
    assert_eq!(stripe.evidence().len(), 1);
    assert_eq!(stripe.evidence()[0].anchor.location, "src/app.ts");
    assert!(
        stripe.evidence()[0].anchor.locator.contains("line 2"),
        "{:?}",
        stripe.evidence()[0].anchor
    );

    // Named and read by nothing. Confirmed as worded, and the sentence carries
    // the limit rather than leaving a reader to conclude the key is dead.
    let legacy = claim_for(&report, "LEGACY_TOKEN");
    assert_eq!(legacy.status(), KeyStatus::DeclaredButNotRead);
    assert_eq!(legacy.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(legacy.severity(), Severity::CanFixLater);
    assert!(
        legacy
            .reason()
            .contains("`.env.example` line 3 names this key"),
        "{}",
        legacy.reason()
    );
    assert!(
        legacy
            .reason()
            .contains("rather than a statement that the key is unused"),
        "{}",
        legacy.reason()
    );

    // A variable the machine provides, read and named nowhere, and not counted
    // against the project.
    let node_env = claim_for(&report, "NODE_ENV");
    assert_eq!(node_env.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(node_env.severity(), Severity::Note);
    assert!(node_env.is_set_aside());
    assert!(
        node_env
            .reason()
            .contains("provides to a process before any project code runs"),
        "{}",
        node_env.reason()
    );

    // The two lists the reader is owed, and the split between what the project
    // owes and what the machine provides.
    let owed: Vec<&str> = report.findings().map(AssessedKey::key).collect();
    assert_eq!(owed, vec!["LEGACY_TOKEN", "STRIPE_KEY"]);
    let set_aside: Vec<&str> = report.set_aside().map(AssessedKey::key).collect();
    assert_eq!(set_aside, vec!["NODE_ENV"]);

    let sentence = report.plain_description();
    assert!(
        sentence.contains("3 environment or configuration keys asked for"),
        "{sentence}"
    );
    assert!(
        sentence.contains("2 named in its examples or documents"),
        "{sentence}"
    );
    assert!(
        sentence.contains("1 key asked for and named nowhere"),
        "{sentence}"
    );
    assert!(
        sentence.contains("1 key named and asked for nowhere"),
        "{sentence}"
    );
    assert!(
        sentence.contains("1 key provided by the machine rather than by the project"),
        "{sentence}"
    );
}

#[test]
fn a_project_that_names_nothing_says_so_rather_than_reporting_no_disagreements() {
    // The false-green this check is most exposed to: a report with no claims
    // reads the same whether the two sides agreed or the pass found nothing to
    // compare. The sentence has to tell them apart, and this is the project
    // where it does not.
    let fixture = Fixture::new("empty");
    fixture.write("src/main.rs", "fn main() { println!(\"hello\"); }\n");

    let report = fixture.report();
    assert!(report.is_complete());
    assert!(report.claims().is_empty(), "{:?}", claimed_keys(&report));

    let sentence = report.plain_description();
    assert!(
        sentence.contains("0 environment or configuration keys"),
        "{sentence}"
    );
    assert!(
        sentence.contains("0 named in its examples or documents"),
        "{sentence}"
    );
    assert!(
        sentence.contains("0 keys asked for and named nowhere"),
        "{sentence}"
    );
    assert!(!sentence.contains("did not finish"), "{sentence}");

    // And the same sentence on a project where every key matches says the keys
    // were there, which is the other half of telling the two apart.
    let matched = Fixture::new("all matched");
    matched
        .write(".env.example", "DATABASE_URL=postgres://localhost/app\n")
        .write("src/app.js", "const url = process.env.DATABASE_URL;\n");
    let matched = matched.report();
    assert!(matched.claims().is_empty());
    let sentence = matched.plain_description();
    assert!(
        sentence.contains("1 environment or configuration key asked for"),
        "{sentence}"
    );
    assert!(
        sentence.contains("1 named in its examples or documents"),
        "{sentence}"
    );
    assert_ne!(sentence, report.plain_description());
}

#[test]
fn a_reading_that_did_not_finish_leaves_every_claim_without_a_verdict() {
    // Candidates are read in path order, so `.env.example` and `README.md` are
    // read and `src/app.ts` is not. The one claim the report can make is about
    // the file it did read, and it is exactly the claim the unread file could
    // have settled.
    let fixture = Fixture::new("unfinished");
    fixture
        .write(".env.example", "LEGACY_TOKEN=\n")
        .write(
            "README.md",
            "# Project\n\nSet `LEGACY_TOKEN` before starting.\n",
        )
        .write("src/app.ts", "const token = process.env.LEGACY_TOKEN;\n");

    let report = fixture.report_with(&ReferenceOptions::default().with_max_files(2));
    assert!(!report.is_complete(), "two files were not read");

    // The reading's own rendering of a path, which is `/` on every platform.
    // Asserting on the raw path would make this test say something different on
    // Windows from what it says on the two platforms CI runs it on.
    let unread: Vec<String> = report
        .unread()
        .iter()
        .map(|file| file.display_path())
        .collect();
    assert_eq!(unread, vec!["src/app.ts".to_owned()], "{unread:?}");

    let legacy = claim_for(&report, "LEGACY_TOKEN");
    assert_eq!(legacy.status(), KeyStatus::DeclaredButNotRead);
    assert_eq!(legacy.assessment(), ClaimAssessment::CannotConfirm);
    assert!(
        legacy.evidence().is_empty(),
        "a claim with no verdict has nothing to anchor: {:?}",
        legacy.evidence()
    );
    assert!(
        legacy.reason().contains("did not finish"),
        "{}",
        legacy.reason()
    );
    assert!(
        legacy.reason().contains("src/app.ts"),
        "{}",
        legacy.reason()
    );
    // The severity is still the finding's, so a caller deciding whether to go
    // and look has what it needs.
    assert_eq!(legacy.severity(), Severity::CanFixLater);
    assert_eq!(
        report.with_assessment(ClaimAssessment::Confirmed).count(),
        0
    );
    assert!(
        report
            .plain_description()
            .contains("SURE did not finish reading the project"),
        "{}",
        report.plain_description()
    );
}

#[test]
fn a_value_the_project_writes_never_reaches_the_report() {
    // The canary is written on all three sides — the value beside a declared
    // key, a documented example, and a fallback in the source — and every one
    // of them is a line a report could quote.
    let fixture = Fixture::new("canary");
    fixture
        .write(
            ".env.example",
            &format!("API_KEY={CANARY}\n# API_KEY is required.\n"),
        )
        .write(
            "README.md",
            &format!("# Setup\n\nSet the key:\n\n```sh\nexport API_KEY={CANARY}\n```\n"),
        )
        .write(
            "src/app.ts",
            &format!(
                "const key = process.env.API_KEY ?? \"{CANARY}\";\n\
                 const other = process.env.UNNAMED_KEY ?? \"{CANARY}\";\n"
            ),
        );

    let report = fixture.report();

    // Three things asserted together, so that a search that found nothing
    // because nothing happened cannot pass: a key was found, the file it was
    // found in was read, and the value is nowhere.
    let claim = claim_for(&report, "UNNAMED_KEY");
    assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(claim.evidence().len(), 1);
    assert_eq!(claim.evidence()[0].anchor.location, "src/app.ts");
    // The key the example declares and the source reads is matched, so the two
    // sides were both read rather than one of them having produced nothing.
    assert_eq!(
        report.references().get("API_KEY").expect("read").status,
        KeyStatus::ReadAndDeclared
    );
    assert!(report.is_complete(), "every file was read");

    let text = everything(&report);
    assert!(
        !text.contains(CANARY),
        "the value reached the report, {} bytes in",
        text.find(CANARY).unwrap_or_default()
    );
    assert!(text.contains("UNNAMED_KEY"), "the key itself must be there");
    // The reading behind the assessment holds the keys too, so a leak through
    // the reading would be caught by the same search.
    assert!(format!("{:?}", report.references()).contains("API_KEY"));
}

#[test]
fn the_helper_that_looks_for_the_canary_can_see_a_canary_that_did_reach_a_report() {
    // A canary search is only worth what it would find, so this holds it: the
    // search string is matched against a text the value *is* in, and against
    // the surfaces the search covers.
    let fixture = Fixture::new("canary helper");
    fixture
        .write(".env.example", &format!("API_KEY={CANARY}\n"))
        .write("src/app.ts", "const key = process.env.UNNAMED_KEY;\n");
    let report = fixture.report();

    let text = everything(&report);
    assert!(!text.contains(CANARY));
    // The same search over a text that does contain it finds it, so a passing
    // assertion above is about the report rather than about the search.
    let with_canary = format!("{text}\n{CANARY}\n");
    assert!(with_canary.contains(CANARY));
    // And the search reaches the four surfaces: the `Debug` rendering, the
    // evidence summary and anchor, the reason, and the plain description.
    let claim = claim_for(&report, "UNNAMED_KEY");
    assert!(text.contains(claim.reason()), "the reason is searched");
    assert!(
        text.contains(&claim.plain_description()),
        "the line is searched"
    );
    assert!(
        text.contains(&claim.evidence()[0].summary),
        "the evidence summary is searched"
    );
    assert!(
        text.contains(&claim.evidence()[0].anchor.locator),
        "the evidence anchor is searched"
    );
}

#[test]
fn no_claim_quotes_a_project_line() {
    // The one field of an anchor that exists to hold project text. The line a
    // key is read on is the line its value is on, and the line a template
    // declares it on is a line with a value beside it.
    let fixture = Fixture::new("no quotes");
    fixture
        .write(
            ".env.example",
            &format!("API_KEY={CANARY}\nUNUSED_KEY={CANARY}\n"),
        )
        .write(
            "README.md",
            &format!("# Setup\n\n```sh\nexport DOCUMENTED_KEY={CANARY}\n```\n"),
        )
        .write(
            "src/app.ts",
            &format!(
                "const key = process.env.API_KEY ?? \"{CANARY}\";\n\
                 const db = process.env.MISSING_KEY;\n"
            ),
        );

    let report = fixture.report();
    // One key on each side of both comparisons, so the loop below covers a read
    // claim and a declared claim rather than two of one kind.
    assert_eq!(
        claimed_keys(&report),
        vec![
            "DOCUMENTED_KEY".to_owned(),
            "MISSING_KEY".to_owned(),
            "UNUSED_KEY".to_owned(),
        ]
    );
    for claim in report.claims() {
        assert!(
            !claim.evidence().is_empty(),
            "{} was not anchored",
            claim.key()
        );
        for evidence in claim.evidence() {
            assert!(
                evidence.anchor.excerpt.is_empty(),
                "{} quotes a project line: {:?}",
                claim.key(),
                evidence.anchor
            );
            assert!(evidence.anchor.is_checkable(), "{:?}", evidence.anchor);
        }
    }
}

#[test]
fn the_file_the_values_are_in_is_not_read_here_either() {
    let fixture = Fixture::new("dot env");
    fixture
        .write(".env", &format!("REAL_SECRET={CANARY}\nAPI_KEY={CANARY}\n"))
        .write(".env.local", &format!("LOCAL_SECRET={CANARY}\n"))
        .write(".env.example", "API_KEY=\n")
        .write("src/app.ts", "const key = process.env.API_KEY;\n");

    let report = fixture.report();

    // The key in the example is found and matched, so the pass ran.
    assert_eq!(
        report.references().get("API_KEY").expect("read").status,
        KeyStatus::ReadAndDeclared
    );
    // The keys that exist only in the file the values are in are not in the
    // report at all, and that file is not reported as unread either — declining
    // to open it is a decision rather than a loss of coverage.
    assert!(report.references().get("REAL_SECRET").is_none());
    assert!(report.references().get("LOCAL_SECRET").is_none());
    let unread: Vec<String> = report
        .unread()
        .iter()
        .map(|file| file.display_path())
        .collect();
    assert!(
        !unread.iter().any(|path| path.contains(".env")),
        "{unread:?}"
    );
    assert!(!everything(&report).contains(CANARY));
}

#[test]
fn a_claim_is_bound_to_the_state_it_was_read_against() {
    let fixture = Fixture::new("fingerprint");
    fixture
        .write("src/app.ts", "const key = process.env.STRIPE_KEY;\n")
        .write("src/other.ts", "const key = process.env.QUEUE_URL;\n");

    let fingerprint = FingerprintId::generate();
    let report = fixture.report_at(&fingerprint);
    assert_eq!(report.claims().len(), 2);
    for claim in report.claims() {
        for evidence in claim.evidence() {
            assert!(
                evidence.is_fresh_for(&fingerprint),
                "{} carries evidence for another state",
                claim.key()
            );
        }
    }
    // A different state does not make this reading's evidence speak for it,
    // which is the whole reason the fingerprint is a parameter.
    assert!(!report.claims()[0].evidence()[0].is_fresh_for(&FingerprintId::generate()));
}

#[test]
fn nothing_a_project_can_write_adds_a_line_to_a_message() {
    // Every sentence this check writes carries a key name, and the reason that
    // is safe is that the reading accepts a name only when it is a plain
    // identifier. This is that claim asked of a real project written to break
    // it: a template whose lines try to end one and start another, a document
    // with prose in it, and a source file whose keys are quoted with something
    // that is not a key in them.
    let fixture = Fixture::new("hostile text");
    fixture
        .write(
            ".env.example",
            "SAFE_KEY=1\n\
             NOT_A_KEY_USE=A\\\\nB\n\
             # comment with \u{2028} in it\n\
             \u{2029}UNICODE_KEY=2\n",
        )
        .write(
            "README.md",
            "# Setup\n\nA table:\n\n| KEY | Meaning |\n| --- | --- |\n| TABLE_KEY | a |\n\n\
             ```sh\nexport DOC_KEY=1\n```\n\nA path with a space and Unicode: `docs/日本語/note.md`.\n",
        )
        .write(
            "src/app.ts",
            "const a = process.env.SAFE_KEY;\n\
             const b = process.env[\"QUOTED_KEY\"];\n\
             const c = process.env[`TEMPLATE_${x}`];\n\
             const d = process.env[\"not a key\"];\n\
             const e = process.env[\"\"];\n",
        );

    let report = fixture.report();
    // What the reading makes of it, which is the input every sentence here is
    // written from: the three declarations it can see, the read that has no
    // declaration, and the key both sides name, which makes no claim.
    assert_eq!(
        claimed_keys(&report),
        vec![
            "DOC_KEY".to_owned(),
            "NOT_A_KEY_USE".to_owned(),
            "QUOTED_KEY".to_owned(),
            "UNICODE_KEY".to_owned(),
        ]
    );
    // The shapes it must have refused, because a name that is not an identifier
    // is the one thing that could put a line of the project's writing into a
    // sentence: an argument built at run time, a phrase, an empty string, and a
    // word that is only mentioned in a table rather than assigned.
    for refused in ["not a key", "", "TEMPLATE_", "TABLE_KEY"] {
        assert!(
            report.references().get(refused).is_none(),
            "{refused:?} was read as a key"
        );
    }

    for claim in report.claims() {
        let line = claim.plain_description();
        assert!(
            !line.chars().any(char::is_control),
            "{}: {:?}",
            claim.key(),
            line
        );
        assert!(
            !line.contains('\u{2028}') && !line.contains('\u{2029}'),
            "{line:?}"
        );
        // And the key in the sentence is the key rather than a longer stretch
        // of the project that happened to contain one.
        assert!(
            line.starts_with(&format!("{}: ", claim.key())),
            "{line:?} does not start with the key"
        );
    }
}

#[test]
fn a_key_read_in_many_places_is_anchored_and_says_how_many_there_were() {
    let fixture = Fixture::new("many reads");
    let mut source = String::new();
    for index in 0..20 {
        source.push_str(&format!("const value{index} = process.env.REPEATED_KEY;\n"));
    }
    fixture.write("src/app.ts", &source);

    let report = fixture.report();
    let claim = claim_for(&report, "REPEATED_KEY");
    assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(
        claim.evidence().len(),
        sure_core::env_completeness::MAX_ANCHORS
    );
    assert!(
        claim
            .reason()
            .contains("20 places on this side of the comparison"),
        "{}",
        claim.reason()
    );
}

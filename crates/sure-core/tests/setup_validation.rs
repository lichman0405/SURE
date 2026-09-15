//! README and setup claim validation, end to end.
//!
//! `crate::setup`'s own tests hold the reading: which line names a script,
//! which reading a documented path has, which manifest governs which document.
//! This file drives the whole thing over real directories, which is where those
//! parts have to add up to something true about a project — and where the two
//! acceptance sentences for `P4-T005` can actually be checked.
//!
//! # The two claims this file exists to hold
//!
//! > Safe claims/paths/scripts can be validated.
//! > Arbitrary README shell text is not blindly run.
//!
//! The first is a claim that SURE **settles** things, and a validator that
//! answered `Cannot confirm` to everything would satisfy no test that asks for a
//! `Confirmed` and no test that asks for a `Contradicted`. So the tests below
//! pin all four assessments, each against a project built to produce it.
//!
//! The second is a claim about **something that did not happen**, and a test is
//! bad at those: *"no file was created"* is equally true when the pass never
//! opened the README, when the README had no commands in it, and when the pass
//! did nothing at all. `the_documented_command_left_no_trace_and_was_still_read`
//! therefore asserts three things together — the command **was** read, a claim
//! **was** produced from it, and the project directory is **unchanged** — and
//! `the_helper_that_looks_for_new_files_can_see_a_new_file` holds the third
//! assertion to account, because a snapshot comparison that can never report a
//! difference is not a comparison.
//!
//! # Why every contradiction here is written out in full
//!
//! `Contradicted` is the verdict that can be wrong about a project that is fine,
//! so each test that expects one asserts the **sentence** as well as the verdict.
//! A verdict says SURE decided; the sentence is what a reader would act on, and
//! the failure this guards against is a `Contradicted` whose reason names the
//! wrong file.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::scan::ScanOptions;
use sure_core::setup::SetupReport;
use sure_domain::evidence::{ClaimAssessment, EvidenceClass};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

/// A directory under the workspace's git-ignored `target/tmp`.
///
/// Unique per call and **never cleared**, which is the pattern this repository
/// settled on after a false report: clearing a fixed path with
/// `let _ = remove_dir_all(..)` and then treating it as fresh fails on Windows,
/// and the test then describes a directory that was never emptied. A path nobody
/// has used before needs no removal. Uniqueness comes from `create_dir`, not
/// from the name, so two processes given the same id cannot collide — a
/// directory that exists is skipped rather than adopted.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("setup validation");
        std::fs::create_dir_all(&base)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
        for _ in 0..1_000 {
            let project = base.join(format!("{test}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
            match std::fs::create_dir(&project) {
                Ok(()) => return Self { project },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create {}: {error}", project.display()),
            }
        }
        panic!("no free fixture name under {}", base.display());
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

    fn report(&self) -> SetupReport {
        SetupReport::of(&self.discovery(), &FingerprintId::generate())
    }
}

/// Everything in a directory, recursively, as sorted project-relative paths.
///
/// The whole tree rather than the one file the canary command would have
/// written, because the interesting failure is not that SURE ran *that* command
/// — it is that SURE ran *something*.
fn everything_under(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, found: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, found);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.sort();
    found
}

/// Every claim a report holds, as `document:line assessment`.
fn shape(report: &SetupReport) -> Vec<String> {
    report
        .claims()
        .iter()
        .map(|assessed| {
            format!(
                "{}:{} {:?}",
                assessed.claim().display_path(),
                assessed.claim().line(),
                assessed.assessment()
            )
        })
        .collect()
}

/// The assessment of the one claim a report holds, with the report's shape in
/// the message so a missing or extra claim is visible rather than incidental.
fn only(report: &SetupReport) -> &sure_core::setup::Assessed {
    assert_eq!(report.claims().len(), 1, "{:?}", shape(report));
    &report.claims()[0]
}

// ---------------------------------------------------------------------------
// Scripts: a documented command is a claim about a manifest SURE already read.
// ---------------------------------------------------------------------------

#[test]
fn a_script_the_manifest_declares_is_confirmed() {
    let fixture = Fixture::new("script declared");
    fixture
        .write(
            "package.json",
            r#"{ "name": "demo", "scripts": { "build": "tsc -p ." } }"#,
        )
        .write(
            "README.md",
            "# Demo\n\n## Build\n\n```bash\nnpm install\nnpm run build\n```\n",
        );

    let report = fixture.report();
    assert!(report.is_complete(), "{:?}", report.unread());
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(assessed.reason(), "package.json declares a `build` script.");
    // The claim carries the document's own words and where they were, so a
    // reader can go and look without SURE having decided what they meant.
    assert_eq!(assessed.claim().quoted(), "npm run build");
    assert_eq!(assessed.claim().line(), 7);
    assert_eq!(assessed.claim().document(), Path::new("README.md"));
    // `npm install` is a command SURE reads and a script it does not: neither a
    // shorthand nor a `run`, so it is not a claim at all.
    assert_eq!(
        assessed.evidence().len(),
        1,
        "a confirmed claim rests on what was read"
    );
}

#[test]
fn a_script_no_manifest_declares_is_contradicted_and_names_what_is_there() {
    let fixture = Fixture::new("script missing");
    fixture
        .write(
            "package.json",
            r#"{ "scripts": { "build": "tsc", "test": "jest" } }"#,
        )
        .write("README.md", "```bash\nnpm run deploy\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
    // The names are the project's own and they are in the sentence on purpose:
    // a reader looking for `deploy` needs to see what the manifest does declare.
    assert_eq!(
        assessed.reason(),
        "package.json declares `build`, `test`, and none of them is `deploy`."
    );
    assert_eq!(
        assessed.evidence().len(),
        1,
        "a contradiction rests on the manifest that was read"
    );
}

#[test]
fn a_script_entry_that_is_not_a_command_is_contradicted_as_that_and_not_as_absent() {
    let fixture = Fixture::new("script not a command");
    fixture
        .write(
            "package.json",
            r#"{ "scripts": { "build": { "command": "tsc" } } }"#,
        )
        .write("README.md", "```bash\nnpm run build\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    // The name is in the file and what it declares is not a command string, so
    // npm will not run it. Reporting this as *not declared* would send a reader
    // looking for a script that is right there.
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
    assert_eq!(
        assessed.reason(),
        "package.json has a `build` entry, and what it declares is not a command a package \
         manager will run."
    );
}

#[test]
fn a_command_the_manifest_was_not_read_for_cannot_be_settled() {
    let fixture = Fixture::new("manifest unread");
    // Invalid JSON: the file is there and SURE could not read it. The verdict is
    // *cannot confirm* rather than *no such script*, because the second is a
    // claim about a file SURE did not understand.
    fixture
        .write("package.json", "{ \"scripts\": ")
        .write("README.md", "```bash\nnpm run build\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::CannotConfirm);
    assert!(
        assessed
            .reason()
            .starts_with("SURE did not read package.json"),
        "{}",
        assessed.reason()
    );
    assert!(
        assessed.evidence().is_empty(),
        "a claim SURE could not settle rests on nothing"
    );
}

#[test]
fn a_command_with_no_manifest_above_it_is_contradicted() {
    let fixture = Fixture::new("no manifest");
    fixture.write("README.md", "```bash\nnpm run build\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
    assert_eq!(
        assessed.reason(),
        "There is no package.json at or above README.md, so nothing there declares a `build` \
         script."
    );
}

#[test]
fn a_command_with_no_manifest_is_not_contradicted_by_a_walk_that_stopped_early() {
    let fixture = Fixture::new("walk stopped early");
    // The same two files, walked twice. With the default depth the walk reaches
    // everything and there is no manifest above the document, so the documented
    // command cannot work and SURE says so. With the walk stopped above `src/`,
    // SURE did not reach everything it set out to reach and *there is no
    // package.json above this document* becomes a sentence about the part of the
    // project it happened to walk.
    fixture
        .write("README.md", "```bash\nnpm run build\n```\n")
        .write("src/app.ts", "export {};\n");

    let whole = fixture.report();
    assert!(whole.is_complete());
    assert_eq!(only(&whole).assessment(), ClaimAssessment::Contradicted);

    let shallow = DiscoverOptions::default().with_scan(ScanOptions::default().with_max_depth(1));
    let discovery = discover(&fixture.project, &shallow).expect("discover");
    assert!(
        !discovery.is_complete(),
        "the walk was supposed to stop early"
    );
    let stopped = SetupReport::of(&discovery, &FingerprintId::generate());
    let assessed = only(&stopped);
    assert_eq!(assessed.assessment(), ClaimAssessment::CannotConfirm);
    assert_eq!(
        assessed.reason(),
        "SURE did not read every part of this project, and there is no package.json at or \
         above README.md, so the manifest that declares `build` may be one SURE did not reach."
    );
    assert!(assessed.evidence().is_empty());
}

#[test]
fn a_command_in_a_member_is_a_claim_about_the_members_manifest() {
    let fixture = Fixture::new("member manifest");
    // The root declares `build` and `apps/web` declares `dev`. `npm run build`
    // started in `apps/web` resolves against the member, so the member's table
    // is the one that answers — and a rule of *any manifest declares it* would
    // confirm this against the wrong file.
    fixture
        .write(
            "package.json",
            r#"{ "name": "root", "workspaces": ["apps/*"], "scripts": { "build": "turbo run build" } }"#,
        )
        .write(
            "apps/web/package.json",
            r#"{ "name": "web", "scripts": { "dev": "vite" } }"#,
        )
        .write("apps/web/README.md", "```bash\nnpm run build\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
    assert_eq!(
        assessed.reason(),
        "apps/web/package.json declares `dev`, and none of them is `build`."
    );
    assert_eq!(assessed.claim().document(), Path::new("apps/web/README.md"));
}

#[test]
fn the_nearest_manifest_wins_and_not_the_one_that_sorts_first() {
    let fixture = Fixture::new("nearest manifest");
    // The rule is the *nearest* manifest at or above the document, and taking
    // the first candidate in path order is a different rule that agrees with it
    // by accident whenever the member's name sorts below `package.json`.
    // `apps/web` does; `zed` does not, and this is the directory where the two
    // rules part.
    fixture
        .write(
            "package.json",
            r#"{ "name": "root", "workspaces": ["zed"], "scripts": { "build": "tsc" } }"#,
        )
        .write(
            "zed/package.json",
            r#"{ "name": "zed", "scripts": { "dev": "vite" } }"#,
        )
        .write("zed/README.md", "```bash\nnpm run build\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
    assert_eq!(
        assessed.reason(),
        "zed/package.json declares `dev`, and none of them is `build`."
    );
}

#[test]
fn a_manifest_the_discovery_never_read_makes_the_claim_unanswerable() {
    let fixture = Fixture::new("manifest outside the workspace");
    // `tools/` is not a workspace member, so the discovery never opened its
    // manifest — but the walk saw the file, and a command in `tools/README.md`
    // is governed by it. The honest answer is that SURE did not read it.
    //
    // This is the test for the decision that the manifest table is built from
    // the scan rather than from the discovery: without it the governing manifest
    // would be invisible and SURE would say there is no `package.json` above a
    // document that is sitting next to one.
    fixture
        .write(
            "package.json",
            r#"{ "name": "root", "workspaces": ["apps/*"], "scripts": { "build": "tsc" } }"#,
        )
        .write(
            "tools/package.json",
            r#"{ "scripts": { "seal": "sh seal.sh" } }"#,
        )
        .write("tools/README.md", "```bash\nnpm run seal\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::CannotConfirm);
    assert_eq!(
        assessed.reason(),
        "SURE did not read tools/package.json, so it cannot say whether `seal` is declared \
         there."
    );
}

#[test]
fn a_command_that_names_no_script_is_not_a_claim_at_all() {
    let fixture = Fixture::new("no script named");
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write(
            "README.md",
            "# Demo\n\n```bash\nnpm install\nnpm ci\nmake build\ncargo test\n```\n",
        );

    let report = fixture.report();
    assert!(report.claims().is_empty(), "{:?}", shape(&report));
    assert_eq!(
        report.documents().commands().len(),
        4,
        "the documents pass still read all four lines"
    );
    assert!(report.plain_description().contains("0 claims"));
}

// ---------------------------------------------------------------------------
// Paths: a documented path is a claim about the walk SURE already did.
// ---------------------------------------------------------------------------

#[test]
fn a_path_the_project_has_is_confirmed() {
    let fixture = Fixture::new("path present");
    fixture
        .write("README.md", "Put your key in `config/local.env`.\n")
        .write("config/local.env", "KEY=\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(
        assessed.reason(),
        "config/local.env is a file in this project."
    );
    assert_eq!(assessed.claim().quoted(), "config/local.env");
}

#[test]
fn a_folder_the_project_has_is_confirmed_as_a_folder() {
    let fixture = Fixture::new("folder present");
    fixture
        .write("README.md", "Sources live in `src/components`.\n")
        .write("src/components/Button.tsx", "export {};\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(
        assessed.reason(),
        "src/components is a folder in this project."
    );
}

#[test]
fn a_path_the_project_does_not_have_is_contradicted_and_names_where_it_looked() {
    let fixture = Fixture::new("path absent");
    fixture.write("README.md", "Read `docs/setup.md` first.\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
    assert_eq!(
        assessed.reason(),
        "SURE read this project and found nothing at `docs/setup.md`. It looked at `docs/setup.md`."
    );
    assert_eq!(assessed.evidence().len(), 1);
}

#[test]
fn a_path_is_looked_for_beside_its_document_as_well_as_at_the_root() {
    let fixture = Fixture::new("path beside");
    // A link is relative to the document that carries it. Nothing at
    // `apps/web/docs/api.md`, so the reading that holds is the project-root one.
    fixture
        .write("apps/web/README.md", "See [the API](docs/api.md).\n")
        .write("docs/api.md", "# API\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(assessed.reason(), "docs/api.md is a file in this project.");
}

#[test]
fn the_reading_beside_the_document_wins_when_both_are_there() {
    let fixture = Fixture::new("both readings");
    // Both readings exist. SURE reports the one next to the document, because
    // that is what a reader who clicked the link would get, and the sentence
    // names the file it found rather than the one the document wrote.
    fixture
        .write("apps/web/README.md", "See [the API](docs/api.md).\n")
        .write("apps/web/docs/api.md", "# web API\n")
        .write("docs/api.md", "# root API\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(
        assessed.reason(),
        "apps/web/docs/api.md is a file in this project."
    );
}

#[test]
fn a_path_under_something_the_walk_skipped_cannot_be_settled() {
    let fixture = Fixture::new("path skipped");
    // SURE does not look inside `node_modules`, so *there is no such file* is
    // not something it can say about a path in there. The verdict has to be
    // *cannot confirm*, and the sentence has to say which directory it did not
    // look in.
    fixture
        .write(
            "README.md",
            "The rule lives in `node_modules/eslint/index.js`.\n",
        )
        .write("node_modules/eslint/index.js", "module.exports = {};\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::CannotConfirm);
    assert!(
        assessed
            .reason()
            .starts_with("SURE did not look at node_modules"),
        "{}",
        assessed.reason()
    );
    assert!(
        assessed
            .reason()
            .ends_with("So it cannot say whether `node_modules/eslint/index.js` is there."),
        "{}",
        assessed.reason()
    );
    assert!(assessed.evidence().is_empty());
}

#[test]
fn a_path_that_climbs_out_of_the_project_is_not_checked_at_all() {
    let fixture = Fixture::new("path outside");
    // SURE says it did not check, rather than that it looked and did not find:
    // the second sentence would be a claim about the user's disk outside the
    // project they asked about.
    fixture.write("README.md", "The host file is `../../etc/hosts`.\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::NotCheckable);
    assert_eq!(
        assessed.reason(),
        "`../../etc/hosts` is outside this project, and SURE only looks inside a project."
    );
    assert!(assessed.evidence().is_empty());
}

#[test]
fn a_url_a_document_names_is_not_a_path_in_the_project() {
    let fixture = Fixture::new("url not a path");
    // The endpoint is refused before this module sees it — `documents` refuses
    // an absolute reference in a span and in a link target alike — and this is
    // the end-to-end reading of that decision. A URL reaching this layer would
    // become `Contradicted`, which is a finding about a README that is correct.
    fixture.write(
        "README.md",
        "Deploy to `https://example.com/app` and read [the docs](docs/index.md).\n",
    );

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.claim().quoted(), "docs/index.md");
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
}

#[test]
fn a_document_that_names_no_path_has_no_path_claims() {
    let fixture = Fixture::new("no paths");
    // `Cargo.toml` is a file and `process.env.NAME` is a variable, and the two
    // have one shape: a span with no separator in it is not read. Both are in
    // the documents pass's list of refusals, and neither is a claim here.
    fixture.write(
        "README.md",
        "Edit `Cargo.toml` and read `process.env.NAME`.\n",
    );

    let report = fixture.report();
    assert!(report.claims().is_empty(), "{:?}", shape(&report));
}

// ---------------------------------------------------------------------------
// The report as a whole.
// ---------------------------------------------------------------------------

#[test]
fn the_claims_are_in_the_order_a_reader_walks_the_project() {
    let fixture = Fixture::new("claim order");
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write("docs/setup.md", "# Setup\n")
        .write(
            "README.md",
            "```bash\nnpm run build\n```\n\nSee `docs/setup.md`.\n",
        )
        .write(
            "apps/web/README.md",
            "```bash\nnpm run build\n```\n\nSee `docs/gone.md`.\n",
        );

    let report = fixture.report();
    // Document order, then line order — the order a person reads the project
    // in. The root README sorts before `apps/` because `R` is below `a`, which
    // is a fact about the comparison and not a claim about either document.
    assert_eq!(
        shape(&report),
        [
            "README.md:2 Confirmed",
            "README.md:5 Confirmed",
            "apps/web/README.md:2 Confirmed",
            "apps/web/README.md:5 Contradicted",
        ]
    );
}

#[test]
fn every_assessment_has_a_row_even_when_no_claim_reached_it() {
    let fixture = Fixture::new("counts");
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write("README.md", "```bash\nnpm run build\n```\n");

    let report = fixture.report();
    let counts = report.counts();
    assert_eq!(counts.len(), 4, "one row per assessment: {counts:?}");
    assert_eq!(
        counts
            .iter()
            .map(|(assessment, count)| (assessment.label(), *count))
            .collect::<Vec<_>>(),
        [
            ("Confirmed", 1),
            ("Contradicted", 0),
            ("Cannot confirm", 0),
            ("Not checked", 0),
        ]
    );
    assert_eq!(
        report.with_assessment(ClaimAssessment::Confirmed).count(),
        1
    );
}

#[test]
fn a_document_that_was_not_read_end_to_end_says_so() {
    let fixture = Fixture::new("not complete");
    // A fence with no language on it is a block SURE did not read: it may be a
    // command block or it may be a sample of the thing the tool prints. The
    // claims from the blocks it *did* read are still made, and the report says
    // it did not finish.
    fixture.write(
        "README.md",
        "```bash\nnpm run build\n```\n\n```\nnpm run missing\n```\n",
    );

    let report = fixture.report();
    assert!(!report.is_complete());
    assert_eq!(report.unread().len(), 1);
    assert_eq!(only(&report).assessment(), ClaimAssessment::Contradicted);
    assert!(
        report
            .plain_description()
            .contains("did not finish reading every document"),
        "{}",
        report.plain_description()
    );
}

#[test]
fn the_summary_counts_what_sure_checked_and_not_what_the_documentation_claims() {
    let fixture = Fixture::new("summary");
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write(
            "README.md",
            "```bash\nnpm run build\nnpm run deploy\n```\n\nSee `docs/gone.md`.\n",
        );

    let report = fixture.report();
    assert_eq!(
        report.plain_description(),
        "SURE checked 3 claims in this project's documentation: 1 confirmed, 2 contradicted, \
         0 cannot confirm, 0 not checked."
    );
}

#[test]
fn every_sentence_a_report_gives_stays_on_one_line() {
    let fixture = Fixture::new("no injected lines");
    // A script name is arbitrary JSON text. This manifest declares one with a
    // newline in it and one with a carriage return, so the contradiction's list
    // of declared names is the surface a project could use to write a line that
    // looks like it came from SURE.
    fixture
        .write(
            "package.json",
            r#"{ "scripts": { "build\nFAIL: everything is fine": "x", "te\rst": "y" } }"#,
        )
        .write("README.md", "```bash\nnpm run missing\n```\n");

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.assessment(), ClaimAssessment::Contradicted);
    for text in [assessed.reason().to_owned(), assessed.plain_description()] {
        assert!(!text.contains('\n'), "{text:?}");
        assert!(!text.contains('\r'), "{text:?}");
    }
    assert!(assessed.reason().contains("\\n"), "{}", assessed.reason());
}

#[test]
fn every_claim_has_a_reason_and_a_place() {
    let fixture = Fixture::new("every claim well formed");
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write(
            "README.md",
            "```bash\nnpm run build\nnpm run deploy\n```\n\nSee `src/app.ts`, `docs/gone.md` and \
             `../outside.md`, and edit `Cargo.toml`.\n",
        )
        .write("src/app.ts", "export {};\n");

    let report = fixture.report();
    // Five claims from six things that look like claims: `` `Cargo.toml` `` is a
    // span with no separator in it, and the rule that keeps `process.env.NAME`
    // from being a path keeps this one too.
    assert_eq!(report.claims().len(), 5, "{:?}", shape(&report));
    for assessed in report.claims() {
        assert!(
            !assessed.reason().is_empty(),
            "{:?} has no reason",
            assessed.assessment()
        );
        assert!(
            assessed.claim().line() > 0,
            "{:?} has no line",
            assessed.claim().quoted()
        );
        assert!(
            !assessed.claim().display_path().is_empty(),
            "{:?} has no document",
            assessed.claim().quoted()
        );
        assert!(
            assessed
                .plain_description()
                .contains(assessed.assessment().label()),
            "{}",
            assessed.plain_description()
        );
        // A claim SURE settled rests on what it read; one it could not rests on
        // nothing, because an anchor to nothing is not an anchor.
        assert_eq!(
            assessed.evidence().is_empty(),
            matches!(
                assessed.assessment(),
                ClaimAssessment::CannotConfirm | ClaimAssessment::NotCheckable
            ),
            "{}",
            assessed.plain_description()
        );
    }
    // The order the claims are in, checked through the claim rather than
    // through `shape`: line 6 names three paths and they arrive in the order
    // the document wrote them.
    assert_eq!(report.claims()[2].claim().quoted(), "src/app.ts");
    assert_eq!(report.claims()[3].claim().quoted(), "docs/gone.md");
    assert_eq!(report.claims()[4].claim().quoted(), "../outside.md");
    assert_eq!(
        report.claims()[4].assessment(),
        ClaimAssessment::NotCheckable
    );
}

// ---------------------------------------------------------------------------
// The acceptance: arbitrary README shell text is not blindly run.
// ---------------------------------------------------------------------------

/// A README whose commands would leave a trace if anything ran them.
///
/// Every one of these is a real shell command that writes a file, and one of
/// them is a script name a manifest does not declare, so the pass has to read
/// them *and* decide about them without either one happening.
const CANARY: &str = "# Demo\n\
                      \n\
                      ```bash\n\
                      touch canary-from-bash\n\
                      npm run build\n\
                      ```\n\
                      \n\
                      ```powershell\n\
                      New-Item canary-from-powershell\n\
                      ```\n";

#[test]
fn the_documented_command_left_no_trace_and_was_still_read() {
    let fixture = Fixture::new("canary");
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write("README.md", CANARY);
    let before = everything_under(&fixture.project);

    let discovery = fixture.discovery();
    let report = SetupReport::of(&discovery, &FingerprintId::generate());

    // The pass read the document: three commands, and one claim from them.
    assert_eq!(report.documents().commands().len(), 3);
    assert_eq!(report.claims().len(), 1, "{:?}", shape(&report));
    assert_eq!(report.claims()[0].claim().quoted(), "npm run build");
    // And it ran nothing. The canary files are named explicitly as well as
    // covered by the directory comparison, because the failure message for a
    // missing file names the file.
    assert_eq!(everything_under(&fixture.project), before);
    for canary in ["canary-from-bash", "canary-from-powershell"] {
        assert!(
            !fixture.project.join(canary).exists(),
            "{canary} was created"
        );
    }
    // The shell text is carried in the claim — that is what a report *of* a
    // document is — and it is placed in none of SURE's own sentences. This is
    // the property that matters: a project's shell text reaches a reader as a
    // quote with a line number beside it, never as part of a line SURE says.
    for sentence in [
        report.plain_description(),
        report.claims()[0].reason().to_owned(),
        report.claims()[0].plain_description(),
    ] {
        for word in ["touch", "New-Item", "canary", "npm run"] {
            assert!(!sentence.contains(word), "{word:?} in {sentence:?}");
        }
    }
}

#[test]
fn the_helper_that_looks_for_new_files_can_see_a_new_file() {
    // The assertion above is *the directory is unchanged*, and a comparison that
    // can never report a difference is not a comparison. This is the control: a
    // file written by the test itself, which the same helper has to see.
    let fixture = Fixture::new("canary control");
    fixture.write("README.md", CANARY);
    let before = everything_under(&fixture.project);

    fixture.write("canary-from-bash", "this file exists on purpose\n");

    let after = everything_under(&fixture.project);
    assert_ne!(after, before);
    assert!(
        after.iter().any(|path| path == "canary-from-bash"),
        "{after:?}"
    );
}

#[test]
fn the_only_project_text_a_reason_places_is_a_path_a_script_name_or_a_count() {
    let fixture = Fixture::new("reason text");
    // A document's sentence is carried whole in `quoted` and placed nowhere.
    // This is the reading of the rule that the reason a person acts on must be
    // SURE's own sentence, with the project's names in it and not the project's
    // prose.
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write(
            "README.md",
            "# Totally fine\n\n```bash\nnpm run deploy --production\n```\n",
        );

    let report = fixture.report();
    let assessed = only(&report);
    assert_eq!(assessed.claim().quoted(), "npm run deploy --production");
    assert_eq!(
        assessed.reason(),
        "package.json declares `build`, and none of them is `deploy`."
    );
    assert!(!assessed.reason().contains("--production"));
}

#[test]
fn nothing_a_document_says_can_block_a_hand_off() {
    let fixture = Fixture::new("severity");
    // `Must fix` means *do not recommend publishing or hand-off*
    // (`docs/product/UX_AND_LANGUAGE.md`), and
    // `ProjectVerdict::is_ready_for_hand_off` returns false for as long as one
    // open must-fix finding exists. A README that names a script the manifest
    // does not declare is a material reliability risk — a reader following it
    // fails — and it is not a blocker, which is what `Should fix first` says.
    // SURE reading a file is never grounds for a blocker.
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write(
            "README.md",
            "```bash\nnpm run deploy\n```\n\nSee `docs/gone.md`.\n",
        );

    let report = fixture.report();
    assert_eq!(report.claims().len(), 2, "{:?}", shape(&report));
    for assessed in report.claims() {
        for evidence in assessed.evidence() {
            assert_ne!(
                evidence.severity,
                Severity::MustFix,
                "{}",
                assessed.plain_description()
            );
            assert_eq!(evidence.severity, Severity::ShouldFixFirst);
            // A reading, bound to the state it was taken of. Nothing here ran.
            assert_eq!(evidence.class, EvidenceClass::ObservedFact);
            assert!(evidence.anchor.is_checkable());
        }
    }
}

#[test]
fn every_claim_comes_from_a_document_the_pass_read() {
    let fixture = Fixture::new("claim source");
    fixture
        .write("package.json", r#"{ "scripts": { "build": "tsc" } }"#)
        .write("README.md", "```bash\nnpm run build\n```\n")
        .write("docs/guide.md", "```bash\nnpm run missing\n```\n");

    let report = fixture.report();
    // The claims and the documents are the same reading, not two: every claim's
    // document is one the documents pass opened, and every command that names a
    // script produced exactly one claim.
    let named: Vec<String> = report
        .documents()
        .commands()
        .iter()
        .filter(|command| command.text.contains("npm run"))
        .map(|command| command.display_path())
        .collect();
    let claimed: Vec<String> = report
        .claims()
        .iter()
        .map(|assessed| assessed.claim().display_path())
        .collect();
    assert_eq!(named, ["README.md", "docs/guide.md"]);
    assert_eq!(claimed, named);
}

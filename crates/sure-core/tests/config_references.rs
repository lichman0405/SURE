//! Environment-variable and config reference discovery, end to end.
//!
//! `crate::references`'s own tests hold the extractors: which needle takes which
//! key, which shape is a key and which is a call the code built at run time,
//! which file is a template and which is the file the values are in. This file
//! drives the whole pass over real directories, which is where those parts have
//! to add up to something true about a project.
//!
//! # The two claims this file exists to hold
//!
//! P2-T008 acceptance:
//!
//! > Finds required keys without collecting values.
//! > Can compare references with examples/docs.
//!
//! The first is a claim about **absence**, and absence is the thing a test is
//! worst at asserting: `assert!(!text.contains(CANARY))` passes just as well
//! when the value was never in the fixture, when the file was never opened, or
//! when the report is empty because the pass did nothing. Every canary test
//! below therefore asserts three things together — the key *was* found, the file
//! *was* read, and the value is *not* in the report — so that a test that
//! passes because nothing happened cannot pass. `the_helper_that_looks_for_the_canary_can_see_a_value_that_did_reach_the_report`
//! is what holds the canary itself to that.
//!
//! The second is a claim about **two sides**, and the tests below keep them
//! apart on purpose: a Markdown file that quotes `process.env.NAME` is a
//! document, not a source file, and a test that let one count as both would
//! never notice the pass collapsing into a single grep.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::references::{
    DeclarationKind, KeyStatus, ReadForm, ReferenceOptions, ReferenceReport, UnreadReason,
};

/// A value that must never reach a report.
///
/// Shaped like a live credential, because a redaction bug is usually a bug about
/// *shape* — a rule that skips values starting with `sk-`, or one that stops at
/// the first `-`. A canary that looked like `secret123` would exercise none of
/// that.
const CANARY: &str = "sk-live-CANARY-9f3a2b7c4d";

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
            .join("config references");
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
        self.write_bytes(relative, contents.as_bytes())
    }

    fn write_bytes(&self, relative: &str, contents: &[u8]) -> &Self {
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

    fn report(&self) -> ReferenceReport {
        ReferenceReport::of(&self.discovery())
    }

    fn report_with(&self, options: &ReferenceOptions) -> ReferenceReport {
        ReferenceReport::with_options(&self.discovery(), options)
    }
}

/// Everything a report says, as one string, for a canary to be looked for in.
///
/// `Debug` rather than a rendered report, because `Debug` is what a log line and
/// a failing assertion print, and it is the surface a leaked field would reach
/// without anybody choosing to render it. It is the strictest of the three.
fn everything(report: &ReferenceReport) -> String {
    format!("{report:?}")
}

/// The names of the keys with this status, sorted, for a readable assertion.
fn names_with(report: &ReferenceReport, status: KeyStatus) -> Vec<String> {
    let mut names: Vec<String> = report
        .with_status(status)
        .map(|key| key.name.clone())
        .collect();
    names.sort();
    names
}

#[test]
fn every_family_of_read_is_found_in_a_real_project() {
    let fixture = Fixture::new("every family");
    fixture
        .write(
            "src/app.js",
            "const db = process.env.DATABASE_URL;\n\
             const port = process.env[\"PORT\"];\n",
        )
        .write(
            "svc/main.py",
            "import os\n\
             db = os.environ[\"DATABASE_URL\"]\n\
             key = os.environ.get(\"API_KEY\")\n\
             token = os.getenv(\"API_TOKEN\")\n",
        )
        .write(
            "src/lib.rs",
            "fn db() -> String { std::env::var(\"DATABASE_URL\").unwrap_or_default() }\n\
             fn home() -> Option<std::ffi::OsString> { env::var_os(\"HOME_DIR\") }\n",
        );

    let report = fixture.report();
    let form_of = |name: &str| {
        report
            .get(name)
            .unwrap_or_else(|| panic!("{name} was not found at all"))
            .first_read()
            .expect("a read")
            .form
    };

    assert_eq!(form_of("DATABASE_URL"), ReadForm::NodeProperty);
    assert_eq!(form_of("PORT"), ReadForm::NodeComputed);
    assert_eq!(form_of("API_KEY"), ReadForm::PythonEnviron);
    assert_eq!(form_of("API_TOKEN"), ReadForm::PythonGetenv);
    assert_eq!(form_of("HOME_DIR"), ReadForm::RustEnv);

    // The three languages are three files, and the pass names all three rather
    // than stopping at the first ecosystem it recognises.
    let paths: Vec<String> = report
        .get("DATABASE_URL")
        .expect("DATABASE_URL")
        .reads
        .iter()
        .map(|read| read.display_path())
        .collect();
    assert_eq!(paths, ["src/app.js", "src/lib.rs", "svc/main.py"]);
}

#[test]
fn a_key_read_and_declared_in_an_example_file_is_matched() {
    let fixture = Fixture::new("matched");
    fixture
        .write("src/app.js", "const db = process.env.DATABASE_URL;\n")
        .write("src/other.js", "const url = process.env[\"DATABASE_URL\"];\n")
        .write(
            ".env.example",
            "# Copy this file to .env and fill it in.\nDATABASE_URL=postgres://localhost/app\nPORT=3000\n",
        );

    let report = fixture.report();

    let key = report.get("DATABASE_URL").expect("DATABASE_URL");
    assert_eq!(key.status, KeyStatus::ReadAndDeclared);
    // Two reads and one declaration, and every one of them names its file.
    assert_eq!(key.reads.len(), 2);
    assert_eq!(key.declarations.len(), 1);
    assert_eq!(key.declarations[0].display_path(), ".env.example");
    assert_eq!(key.declarations[0].kind, DeclarationKind::Example);
    assert_eq!(key.declarations[0].line, 2);

    // A key the example declares and nothing reads is the other one-sided arm,
    // and it is in the same report rather than in a separate question.
    assert_eq!(
        names_with(&report, KeyStatus::DeclaredButNotRead),
        ["PORT"],
        "PORT is declared and never read"
    );
    assert_eq!(
        names_with(&report, KeyStatus::ReadButNotDeclared),
        Vec::<String>::new()
    );
}

#[test]
fn a_key_read_and_nowhere_declared_is_reported_without_claiming_it_is_missing() {
    let fixture = Fixture::new("not declared");
    fixture
        .write("src/app.js", "const t = process.env.UNDOCUMENTED_TOKEN;\n")
        .write(".env.example", "# nothing here names that key\n");

    let report = fixture.report();
    assert!(
        report.is_complete(),
        "the fixture is small enough to be read whole, or this test proves nothing"
    );

    let key = report.get("UNDOCUMENTED_TOKEN").expect("the key");
    assert_eq!(key.status, KeyStatus::ReadButNotDeclared);
    assert!(key.is_read());
    assert!(!key.is_declared());

    // The wording rule, at the level a person reads it. The sentence says what
    // SURE read, not what the project has.
    let sentence = key.status.plain_description().to_lowercase();
    for word in ["missing", "undefined", "absent"] {
        assert!(
            !sentence.contains(word),
            "the sentence says {word:?}: {sentence}"
        );
    }
}

#[test]
fn the_value_beside_a_key_never_appears_in_the_report() {
    let fixture = Fixture::new("canary source");
    fixture
        .write(
            "src/app.js",
            &format!("const a = process.env.API_KEY;\nconst b = \"{CANARY}\";\n"),
        )
        .write(
            ".env.example",
            &format!("API_KEY={CANARY}\n# a comment naming {CANARY}\n"),
        )
        .write(
            "README.md",
            &format!("Set `API_KEY` to your key.\n\n```sh\nexport API_KEY={CANARY}\n```\n"),
        );

    let report = fixture.report();

    // The value was really there, in all three files, and the reading really
    // happened: without both of these the absence below would be free.
    assert!(
        std::fs::read_to_string(fixture.project.join("src/app.js"))
            .expect("read")
            .contains(CANARY),
        "the canary has to be in the fixture or the rest of this test is vacuous"
    );
    let key = report.get("API_KEY").expect("the key was found");
    assert_eq!(key.status, KeyStatus::ReadAndDeclared);
    assert_eq!(key.reads.len(), 1);
    assert_eq!(key.declarations.len(), 2);

    assert!(
        !everything(&report).contains(CANARY),
        "the value reached the report"
    );
    // And not a fragment of it, which is what a redaction that stopped at the
    // first `-` would leave behind.
    for fragment in ["sk-live", "9f3a2b7c4d", "CANARY"] {
        assert!(
            !everything(&report).contains(fragment),
            "a fragment of the value ({fragment}) reached the report"
        );
    }
}

#[test]
fn the_file_the_values_are_in_is_never_opened() {
    let fixture = Fixture::new("real env");
    fixture
        .write("src/app.js", "const t = process.env.REAL_SECRET;\n")
        .write(".env", &format!("REAL_SECRET={CANARY}\n"))
        .write(".env.local", &format!("REAL_SECRET={CANARY}\n"))
        .write(".env.test", &format!("REAL_SECRET={CANARY}\n"));

    let report = fixture.report();

    // `REAL_SECRET` is read and not declared, which is exactly what a project
    // whose only mention of it is in `.env` looks like. The arm is the evidence:
    // a pass that had opened `.env` would have found the declaration and said
    // `ReadAndDeclared`.
    let key = report.get("REAL_SECRET").expect("the key");
    assert_eq!(
        key.status,
        KeyStatus::ReadButNotDeclared,
        "a real .env file was read as a declaration"
    );
    assert!(key.declarations.is_empty());

    // Looking and deciding not to open are different things, and this pass
    // reports the second as neither a finding nor a loss of coverage.
    let unread: Vec<String> = report.unread().iter().map(|u| u.display_path()).collect();
    for name in [".env", ".env.local", ".env.test"] {
        assert!(
            !unread.contains(&name.to_owned()),
            "{name} was reported unread"
        );
    }
    assert!(
        report.is_complete(),
        "declining to open a file is not a loss"
    );

    assert!(
        !everything(&report).contains(CANARY),
        "the value reached the report"
    );
}

#[test]
fn an_example_file_with_the_same_contents_is_read_and_says_so() {
    // The control for the test above: the same key, the same value, in a file
    // whose name marks it a template. If this test and that one both passed with
    // the same fixture the pair would be proving nothing, so the two differ in
    // exactly one byte of the file name.
    let fixture = Fixture::new("template control");
    fixture
        .write("src/app.js", "const t = process.env.REAL_SECRET;\n")
        .write(".env.example", &format!("REAL_SECRET={CANARY}\n"));

    let report = fixture.report();
    assert_eq!(
        report.get("REAL_SECRET").expect("the key").status,
        KeyStatus::ReadAndDeclared
    );
    assert!(
        !everything(&report).contains(CANARY),
        "the value reached the report"
    );
}

#[test]
fn a_markdown_file_is_a_document_and_never_a_source_file() {
    let fixture = Fixture::new("docs side");
    fixture
        .write("src/app.js", "const a = process.env.FROM_CODE;\n")
        .write(
            "README.md",
            "The service reads `process.env.FROM_DOCS`.\n\n\
             ```sh\nexport FROM_DOCS=1\nFROM_CODE=1\n```\n\
             Mentioned in prose and never assigned: process.env.NEVER_ASSIGNED\n",
        );

    let report = fixture.report();

    // `FROM_DOCS` is named by the assignment and *not* read: a `.md` file is a
    // document, so the `process.env` in it is prose rather than a call. A pass
    // that searched every file for every needle would say `ReadAndDeclared`
    // here, and that is the bug this test exists to catch.
    let docs = report.get("FROM_DOCS").expect("FROM_DOCS");
    assert_eq!(docs.status, KeyStatus::DeclaredButNotRead);
    assert!(docs.reads.is_empty(), "{:?}", docs.reads);
    assert_eq!(docs.declarations[0].kind, DeclarationKind::Document);
    assert_eq!(docs.declarations[0].display_path(), "README.md");

    // The assignment in the fence is a declaration of a key the code also
    // reads, so the two sides meet.
    assert_eq!(
        report.get("FROM_CODE").expect("FROM_CODE").status,
        KeyStatus::ReadAndDeclared
    );

    // A mention in prose is not a declaration — the stated limit, pinned end to
    // end rather than only in the unit test.
    assert!(
        report.get("NEVER_ASSIGNED").is_none(),
        "a prose mention was read as a declaration"
    );
}

#[test]
fn the_sentence_says_whether_the_reading_finished() {
    let complete = Fixture::new("sentence complete");
    complete.write("src/app.js", "const a = process.env.ONE;\n");
    let report = complete.report();
    assert!(report.is_complete());
    assert!(
        !report.plain_description().contains("did not finish"),
        "{}",
        report.plain_description()
    );

    let partial = Fixture::new("sentence partial");
    partial.write("src/app.js", "const a = process.env.ONE;\n");
    let report = partial.report_with(&ReferenceOptions::default().with_max_files(0));
    assert!(!report.is_complete());
    assert!(
        report.plain_description().contains("did not finish"),
        "{}",
        report.plain_description()
    );
    // The count is of what SURE read, so it is zero rather than wrong.
    assert!(report.plain_description().starts_with("SURE found 0"));
}

#[test]
fn the_sentence_counts_each_side_and_not_the_keys_there_are() {
    let fixture = Fixture::new("sentence counts");
    fixture.write(
        "src/app.js",
        "const a = process.env.READ_ONLY;\nconst b = process.env.BOTH;\n",
    );
    fixture.write(".env.example", "BOTH=1\nEXAMPLE_ONLY=2\n");
    fixture.write("README.md", "Set DOC_ONLY=1 to point it somewhere else.\n");

    let report = fixture.report();
    assert!(report.is_complete());

    // Two keys are read, three are declared, four exist. The sentence carries
    // the two counts and never the total: the total is not something SURE read,
    // and a sentence built from it would state how many keys the project has.
    assert_eq!(report.keys().len(), 4);
    assert_eq!(
        report.plain_description(),
        concat!(
            "SURE found 2 environment or configuration keys asked for in source files ",
            "and 3 named in the project's examples or documents."
        )
    );
    // The two counts overlap on exactly one key, which is why neither can be
    // recovered from the other and why the sentence has to carry both.
    assert_eq!(
        names_with(&report, KeyStatus::ReadAndDeclared),
        ["BOTH"],
        "the one key on both sides"
    );
}

#[test]
fn a_file_too_large_to_read_makes_the_pass_incomplete_and_is_named() {
    let fixture = Fixture::new("too large");
    fixture.write("src/big.js", &"const x = 1;\n".repeat(64));

    let report = fixture.report_with(&ReferenceOptions::default().with_max_file_bytes(32));

    assert!(!report.is_complete());
    let unread = report.unread();
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].display_path(), "src/big.js");
    match &unread[0].reason {
        UnreadReason::TooLarge { limit } => assert_eq!(*limit, 32),
        other => panic!("expected TooLarge, got {other:?}"),
    }
    assert!(
        report.keys().is_empty(),
        "nothing was read, so nothing was found"
    );
}

#[test]
fn the_file_budget_stops_the_pass_and_says_which_file_it_stopped_at() {
    let fixture = Fixture::new("file budget");
    // Named so that the order is known: the pass reads candidates in path
    // order, so the file it stops at is the third one and not whichever the
    // filesystem happened to return last.
    for name in ["a.js", "b.js", "c.js"] {
        fixture.write(&format!("src/{name}"), "const x = process.env.KEY;\n");
    }

    let report = fixture.report_with(&ReferenceOptions::default().with_max_files(2));

    assert!(!report.is_complete());
    let unread = report.unread();
    assert_eq!(
        unread.iter().map(|u| u.display_path()).collect::<Vec<_>>(),
        ["src/c.js"],
        "the budget has to stop at the same file every run"
    );
    match &unread[0].reason {
        UnreadReason::OutOfBudget { limit } => assert_eq!(*limit, 2),
        other => panic!("expected OutOfBudget, got {other:?}"),
    }
    // What was read is still reported: a partial pass reports what it found.
    assert_eq!(report.get("KEY").expect("KEY").reads.len(), 2);
}

#[test]
fn the_byte_budget_stops_the_pass_before_the_file_budget_does() {
    let fixture = Fixture::new("byte budget");
    let body = "const a = process.env.KEY;\n";
    for name in ["a.js", "b.js", "c.js"] {
        fixture.write(&format!("src/{name}"), body);
    }

    // The budget is one file's worth of bytes plus one, computed from the file
    // rather than guessed at: one file fits, the second exactly fills the
    // budget, and the third is the one that is refused.
    let one = body.len() as u64;
    let report = fixture.report_with(&ReferenceOptions::default().with_max_total_bytes(one + 1));

    assert!(!report.is_complete());
    assert_eq!(
        report
            .unread()
            .iter()
            .map(|u| u.display_path())
            .collect::<Vec<_>>(),
        ["src/c.js"]
    );
    match &report.unread()[0].reason {
        UnreadReason::OutOfBytes { limit } => assert_eq!(*limit, one + 1),
        other => panic!("expected OutOfBytes, got {other:?}"),
    }
    // The two files that fit before the byte budget ran out were read, so this
    // is a partial pass and not a pass that refused everything.
    assert_eq!(report.get("KEY").expect("KEY").reads.len(), 2);
}

#[test]
fn a_source_file_that_is_not_text_is_reported_rather_than_decoded_lossily() {
    let fixture = Fixture::new("not text");
    // A UTF-16 byte-order mark and a Latin-1 byte, in a file whose name says it
    // is JavaScript. Decoding this lossily would put replacement characters
    // where the key is.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend_from_slice(b"const a = process.env.KEY;\n\xe9");
    fixture.write_bytes("src/app.js", &bytes);

    let report = fixture.report();
    assert!(!report.is_complete());
    assert_eq!(report.unread().len(), 1);
    assert!(matches!(
        report.unread()[0].reason,
        UnreadReason::NotText { .. }
    ));
}

#[test]
fn a_path_with_spaces_and_non_ascii_characters_is_read_like_any_other() {
    let fixture = Fixture::new("unicode path");
    fixture.write(
        "packages/项目 with spaces/index.js",
        "const a = process.env.UNICODE_PATH_KEY;\n",
    );
    fixture.write(".env.example", "UNICODE_PATH_KEY=\n");

    let report = fixture.report();
    let key = report.get("UNICODE_PATH_KEY").expect("the key");
    assert_eq!(key.status, KeyStatus::ReadAndDeclared);
    assert_eq!(
        key.first_read().expect("a read").display_path(),
        "packages/项目 with spaces/index.js",
        "a path on every platform is written with `/` and nothing is escaped"
    );
}

#[test]
fn every_key_came_from_one_of_the_two_sides_and_the_status_says_which() {
    let fixture = Fixture::new("invariant");
    fixture
        .write(
            "src/app.js",
            "const a = process.env.BOTH;\n\
             const b = process.env.ONLY_READ;\n\
             const c = process.env.BOTH;\n",
        )
        .write(".env.example", "BOTH=1\nONLY_DECLARED=2\nBOTH=\n");

    let report = fixture.report();
    assert!(report.is_complete());

    assert!(!report.keys().is_empty(), "the fixture has to produce keys");
    for key in report.keys() {
        // The invariant the fallback arm in `with_options` is written against:
        // a key exists because one side put it there, so at least one side is
        // non-empty and the status is the pair of booleans rather than a guess.
        assert!(
            key.is_read() || key.is_declared(),
            "{} came from neither side",
            key.name
        );
        let expected = match (key.is_read(), key.is_declared()) {
            (true, true) => KeyStatus::ReadAndDeclared,
            (true, false) => KeyStatus::ReadButNotDeclared,
            (false, true) => KeyStatus::DeclaredButNotRead,
            (false, false) => unreachable!("checked above"),
        };
        assert_eq!(key.status, expected, "{}", key.name);
    }

    assert_eq!(names_with(&report, KeyStatus::ReadAndDeclared), ["BOTH"]);
    assert_eq!(
        names_with(&report, KeyStatus::ReadButNotDeclared),
        ["ONLY_READ"]
    );
    assert_eq!(
        names_with(&report, KeyStatus::DeclaredButNotRead),
        ["ONLY_DECLARED"]
    );

    // A key read twice is one key with two reads, which is what makes the count
    // in the sentence a count of keys rather than of mentions.
    assert_eq!(report.get("BOTH").expect("BOTH").reads.len(), 2);
    assert_eq!(report.get("BOTH").expect("BOTH").declarations.len(), 2);
    assert_eq!(report.keys().len(), 3);
}

#[test]
fn a_file_that_was_never_meant_to_be_read_is_not_a_candidate() {
    let fixture = Fixture::new("not candidates");
    fixture
        .write("src/app.js", "const a = process.env.KEY;\n")
        // A dependency tree, a build output and a data file: none of them is
        // this project's source, and a pass that read them would be reporting
        // on somebody else's keys.
        .write(
            "node_modules/dep/index.js",
            "const a = process.env.FROM_A_DEPENDENCY;\n",
        )
        .write(
            "target/debug/build.js",
            "const a = process.env.FROM_BUILD_OUTPUT;\n",
        )
        .write("data/keys.txt", "FROM_A_DATA_FILE=1\n");

    let report = fixture.report();
    assert!(report.get("KEY").is_some());
    for absent in ["FROM_A_DEPENDENCY", "FROM_BUILD_OUTPUT", "FROM_A_DATA_FILE"] {
        assert!(report.get(absent).is_none(), "{absent} was reported");
    }
    assert!(
        report.is_complete(),
        "a directory the scan leaves out on purpose is not a loss for this pass"
    );
}

#[test]
fn the_helper_that_looks_for_the_canary_can_see_a_value_that_did_reach_the_report() {
    // Guards the guard. Every canary assertion above is of the form "this
    // string is not in the report", and every one of them passes for free if
    // `everything` renders nothing, if the field carrying it is not reached by
    // `Debug`, or if the string could not survive into the report's shape at
    // all. So the same string is put somewhere the report *does* render — a
    // file path — and the same helper is asked to find it.
    //
    // A path is the right place for the control rather than a key, because a
    // path is exactly what a leaked field would look like: it is a `String` the
    // report holds and prints, and the whole design of [`Reference`] is that a
    // value has no equivalent place to sit.
    let fixture = Fixture::new("canary control");
    fixture.write(
        &format!("src/{CANARY}.js"),
        "const a = process.env.ORDINARY_KEY;\n",
    );

    let report = fixture.report();
    assert!(
        report.get("ORDINARY_KEY").is_some(),
        "the pass has to have read the file"
    );
    assert!(
        everything(&report).contains(CANARY),
        "the helper cannot see a value sitting in a field, so its absences prove nothing"
    );
}

#[test]
fn a_key_that_is_only_in_a_comment_is_counted_as_a_read_and_the_report_says_so() {
    // The stated limit, end to end. Pinned rather than fixed: see the module
    // documentation and `a_mention_inside_a_comment_is_still_a_read`.
    let fixture = Fixture::new("comment limit");
    fixture
        .write("src/app.js", "// legacy: read from process.env.OLD_NAME\n")
        .write(".env.example", "# nothing yet\n");

    let report = fixture.report();
    let key = report.get("OLD_NAME").expect("the key");
    assert_eq!(key.status, KeyStatus::ReadButNotDeclared);
    assert_eq!(key.first_read().expect("a read").line, 1);
    assert!(
        Path::new("src/app.js") == key.first_read().expect("a read").path,
        "the read has to name the file it came from"
    );
}

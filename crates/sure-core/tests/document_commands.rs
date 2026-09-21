//! README and specification command extraction, end to end.
//!
//! `crate::documents`'s own tests hold the scanner: which fence is a fence,
//! which tag is a shell, which line inside a block is a command and which is the
//! author's prose. This file drives the whole pass over real directories, which
//! is where those parts have to add up to something true about a project.
//!
//! # The claim this file exists to hold
//!
//! P2-T009 acceptance:
//!
//! > Extracted commands are untrusted documentation and never auto-executed
//! > merely because documented.
//!
//! The first half is a claim about a **label**, and the second is a claim about
//! **something that did not happen**. The second is the harder one, because a
//! test is bad at asserting that nothing happened: *"no file was created"* is
//! equally true when the pass never opened the README, when the README had no
//! commands in it, and when the pass did nothing at all.
//!
//! So `the_documented_command_left_no_trace_and_the_document_was_read` asserts
//! three things together — the command **was** found, the document **was** read,
//! and the project directory is **unchanged**. A pass that quietly did nothing
//! fails the first two. `the_helper_that_looks_for_new_files_can_see_a_new_file`
//! is what holds that third assertion to account, since a snapshot comparison
//! that can never report a difference is not a comparison.
//!
//! The label half is held by
//! `the_commands_arrive_at_the_intent_model_already_labelled`, which drives the
//! commands through the real [`may_claim_full_fulfilment`] gate rather than
//! reading the label off and agreeing with it.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::documents::{
    DocumentOptions, DocumentReport, PathForm, ShellLanguage, UnreadReason,
};
use sure_core::intent::{
    IntentSource, ProjectIntent, RequirementAuthority, may_claim_full_fulfilment,
};

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
            project: sure_testkit::scratch::directory("document commands", test),
        }
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

    fn report(&self) -> DocumentReport {
        DocumentReport::of(&self.discovery())
    }

    fn report_with(&self, options: &DocumentOptions) -> DocumentReport {
        DocumentReport::with_options(&self.discovery(), options)
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

/// Everything a report says, as one string, for a canary to be looked for in.
///
/// `Debug` rather than a rendered report, because `Debug` is what a log line and
/// a failing assertion print, and it is the surface a field would reach without
/// anybody choosing to render it. It is the strictest of the three.
fn everything(report: &DocumentReport) -> String {
    format!("{report:?}")
}

/// The text of every command in a report, in order.
fn texts(report: &DocumentReport) -> Vec<String> {
    report
        .commands()
        .iter()
        .map(|command| command.text.clone())
        .collect()
}

/// The text of every path a report's documents name, in order.
fn paths(report: &DocumentReport) -> Vec<String> {
    report
        .paths()
        .iter()
        .map(|path| path.text.clone())
        .collect()
}

#[test]
fn a_readme_setup_section_is_read_whole() {
    let fixture = Fixture::new("setup section");
    fixture.write(
        "README.md",
        "# Demo\n\
         \n\
         ## Setup\n\
         \n\
         ```bash\n\
         npm install\n\
         npm run build\n\
         ```\n\
         \n\
         ## Running it\n\
         \n\
         ```console\n\
         $ npm start\n\
         \n\
         Server listening on 3000\n\
         ```\n",
    );

    let report = fixture.report();
    assert!(report.is_complete(), "{:?}", report.unread());
    assert_eq!(
        texts(&report),
        ["npm install", "npm run build", "npm start"]
    );
    assert_eq!(report.documents(), [Path::new("README.md")]);

    let sections: Vec<Option<&str>> = report
        .commands()
        .iter()
        .map(|command| command.section.as_deref())
        .collect();
    assert_eq!(sections, [Some("Setup"), Some("Setup"), Some("Running it")]);

    // The console block's output line is not a command, and the `$` that marked
    // the real one is not part of it.
    assert!(report.commands()[2].prompt);
    assert!(
        !everything(&report).contains("Server listening"),
        "output is not a command"
    );
}

#[test]
fn the_documented_command_left_no_trace_and_the_document_was_read() {
    let fixture = Fixture::new("canary");
    fixture.write(
        "README.md",
        "# Demo\n\
         \n\
         ## Setup\n\
         \n\
         ```bash\n\
         npm install && echo SURE-RAN-THIS > SURE-RAN-THIS.txt\n\
         ```\n",
    );

    let before = everything_under(&fixture.project);
    let report = fixture.report();
    let after = everything_under(&fixture.project);

    // 1. The command was found. Without this the two assertions below would pass
    //    on a pass that read nothing.
    assert_eq!(
        texts(&report),
        ["npm install && echo SURE-RAN-THIS > SURE-RAN-THIS.txt"]
    );
    // 2. The document was read, and the report says so rather than leaving the
    //    finding to be inferred from the absence of an `unread` entry.
    assert_eq!(report.documents(), [Path::new("README.md")]);
    assert!(everything(&report).contains("npm install"));
    // 3. Nothing ran. Not the redirection, and nothing else either.
    assert_eq!(before, after, "building the report changed the project");
    assert!(
        !fixture.project.join("SURE-RAN-THIS.txt").exists(),
        "a documented command was executed"
    );
}

#[test]
fn the_helper_that_looks_for_new_files_can_see_a_new_file() {
    // The control for the canary above. `before == after` is only evidence if
    // the comparison could have come out unequal, so this makes it come out
    // unequal on purpose — by writing the file the canary command would have
    // written.
    let fixture = Fixture::new("canary control");
    fixture.write("README.md", "# Demo\n");
    let before = everything_under(&fixture.project);

    fixture.write("SURE-RAN-THIS.txt", "pwned\n");
    let after = everything_under(&fixture.project);

    assert_ne!(before, after, "the helper cannot see a new file");
    assert_eq!(after, ["README.md", "SURE-RAN-THIS.txt"]);
}

#[test]
fn a_command_is_recorded_as_one_string_and_never_as_a_program_and_arguments() {
    let fixture = Fixture::new("unsplit");
    fixture.write(
        "README.md",
        "```bash\n\
         npm ci && npm run build -- --minify\n\
         ```\n",
    );

    let report = fixture.report();
    assert_eq!(
        texts(&report),
        ["npm ci && npm run build -- --minify"],
        "one line is one command; nothing here split it on the shell operator"
    );

    // The type has no program and no argument vector. This is what says so:
    // every byte of the line survives into the report, so nothing was parsed
    // out of it and kept instead.
    let command = &report.commands()[0];
    assert_eq!(
        command.text.len(),
        "npm ci && npm run build -- --minify".len()
    );
    assert!(command.text.contains("&&"));
    assert!(!command.prompt);
    assert_eq!(command.language, ShellLanguage::Posix);
}

#[test]
fn the_commands_arrive_at_the_intent_model_already_labelled() {
    let fixture = Fixture::new("labelled");
    fixture.write("README.md", "```bash\nnpm test\n```\n");

    let report = fixture.report();
    let requirements = report.as_requirements();
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].source, IntentSource::ProjectSpec);
    assert_eq!(
        requirements[0].authority(),
        RequirementAuthority::DocumentedInstruction
    );

    // And the gate that decides whether SURE may say "everything you asked for
    // is done" does not count a documented command towards it — even when there
    // is fresh evidence for every requirement there is.
    let intent = ProjectIntent::from_requirements(requirements);
    assert!(!intent.has_user_requirement());
    assert!(intent.is_after_the_fact());
    assert!(!may_claim_full_fulfilment(&intent, 1));
    assert_eq!(intent.user_requirements().count(), 0);
}

#[test]
fn every_shell_family_is_read_out_of_a_real_project() {
    let fixture = Fixture::new("families");
    fixture.write(
        "README.md",
        "```bash\ndocker compose up\n```\n\
         \n\
         ```console\n$ npm test\n```\n\
         \n\
         ```cmd\nsetup.bat /quiet\n```\n\
         \n\
         ```powershell\n.\\build.ps1 -Release\n```\n",
    );

    let report = fixture.report();
    assert_eq!(
        texts(&report),
        [
            "docker compose up",
            "npm test",
            "setup.bat /quiet",
            ".\\build.ps1 -Release"
        ]
    );
    assert_eq!(
        report
            .commands()
            .iter()
            .map(|command| command.language)
            .collect::<Vec<_>>(),
        [
            ShellLanguage::Posix,
            ShellLanguage::Console,
            ShellLanguage::Windows,
            ShellLanguage::PowerShell
        ]
    );
}

#[test]
fn a_readme_whose_setup_steps_are_a_numbered_list_is_read() {
    // The shape this pass would otherwise walk past and call a complete reading
    // of a document with nothing in it.
    let fixture = Fixture::new("list steps");
    fixture.write(
        "README.md",
        "# Demo\n\
         \n\
         1. Install the dependencies:\n\
         \n\
             ```bash\n\
             npm install\n\
             ```\n\
         \n\
         2. Run the tests:\n\
         \n\
             ```console\n\
             $ npm test\n\
             ```\n",
    );

    let report = fixture.report();
    assert!(report.is_complete(), "{:?}", report.unread());
    assert_eq!(texts(&report), ["npm install", "npm test"]);
}

#[test]
fn a_block_that_is_not_a_shell_is_read_as_nothing_and_is_not_a_gap() {
    let fixture = Fixture::new("not a shell");
    fixture.write(
        "README.md",
        "# Demo\n\
         \n\
         ```json\n\
         { \"scripts\": { \"test\": \"jest\" } }\n\
         ```\n\
         \n\
         ```text\n\
         npm install\n\
         ```\n\
         \n\
         ```bash\n\
         npm test\n\
         ```\n",
    );

    let report = fixture.report();
    assert_eq!(texts(&report), ["npm test"]);
    assert!(
        report.is_complete(),
        "the author stated a language for each block, so nothing was lost"
    );
}

#[test]
fn a_readme_with_an_untagged_block_reports_an_incomplete_reading() {
    let fixture = Fixture::new("untagged");
    fixture.write(
        "README.md",
        "# Demo\n\
         \n\
         ```bash\n\
         npm test\n\
         ```\n\
         \n\
         ```\n\
         npm run migrate\n\
         ```\n",
    );

    let report = fixture.report();
    assert_eq!(
        texts(&report),
        ["npm test"],
        "the untagged block is not read"
    );
    assert!(!report.is_complete(), "and the report says so");

    let unread = report.unread();
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].display_path(), "README.md");
    assert_eq!(unread[0].reason, UnreadReason::UntaggedFence { line: 7 });

    // The sentence is where a person meets it, and it must not read as though
    // the reading was finished.
    let sentence = report.plain_description();
    assert!(sentence.contains("did not finish reading"), "{sentence}");
    assert!(
        sentence.contains("SURE read 1 document and found 1 command"),
        "{sentence}"
    );
}

#[test]
fn the_sentence_counts_what_was_read_and_not_what_the_project_has() {
    let fixture = Fixture::new("sentence counts");
    fixture.write("README.md", "```bash\nnpm test\n```\n");
    fixture.write("docs/setup.md", "```bash\nnpm install\n```\n");
    fixture.write("CONTRIBUTING.md", "No commands here.\n");

    let report = fixture.report();
    assert!(report.is_complete());
    assert_eq!(report.documents().len(), 3);
    assert_eq!(report.commands().len(), 2);
    assert_eq!(
        report.plain_description(),
        "SURE read 3 documents and found 2 commands."
    );
}

#[test]
fn a_project_with_no_documents_is_not_a_project_whose_documents_have_no_commands() {
    let fixture = Fixture::new("no documents");
    fixture.write("src/app.js", "console.log('hello');\n");
    fixture.write("package.json", "{\"name\":\"demo\"}\n");

    let report = fixture.report();
    assert!(report.commands().is_empty());
    assert!(
        report.documents().is_empty(),
        "nothing was read, and it says so"
    );
    assert!(report.is_complete(), "there was nothing to fail to read");
    assert_eq!(
        report.plain_description(),
        "SURE read 0 documents and found 0 commands."
    );
}

#[test]
fn only_readme_files_and_markdown_are_documents() {
    let fixture = Fixture::new("which files");
    fixture.write("README.md", "```bash\nfrom readme\n```\n");
    fixture.write("docs/guide.md", "```bash\nfrom guide\n```\n");
    fixture.write("NOTES.MD", "```bash\nfrom notes\n```\n");
    fixture.write("setup.txt", "```bash\nfrom text file\n```\n");
    fixture.write(".env.example", "FROM_EXAMPLE=1\n");
    fixture.write("package.json", "{\"scripts\":{\"test\":\"jest\"}}\n");

    let report = fixture.report();
    // Path order, and path order is **byte order**, not alphabetical: `Path`'s
    // `Ord` compares components as encoded bytes, so every uppercase name sorts
    // before every lowercase one and `NOTES.MD` comes before `README.md` before
    // `docs/`. That is also the order the walk yields them in, so a report and a
    // directory listing agree. It reads oddly, which is exactly why it is
    // written down here rather than left to be discovered as a bug.
    assert_eq!(
        texts(&report),
        ["from notes", "from readme", "from guide"],
        "documents are read in path order, and only documents are read"
    );
    assert_eq!(report.documents().len(), 3);
}

#[test]
fn commands_are_reported_in_document_and_line_order() {
    let fixture = Fixture::new("order");
    fixture.write("zz-last.md", "```bash\nzebra\n```\n\n```bash\napple\n```\n");
    fixture.write("aa-first.md", "```powershell\nmiddle\n```\n");

    let report = fixture.report();
    assert_eq!(
        texts(&report),
        ["middle", "zebra", "apple"],
        "by document and then by line, not by the family of shell"
    );
}

#[test]
fn a_document_the_pass_could_not_open_is_reported() {
    let fixture = Fixture::new("budget");
    fixture.write("README.md", "```bash\nnpm test\n```\n");
    fixture.write("docs/one.md", "```bash\nnpm install\n```\n");

    // One document, so the second is refused rather than skipped quietly.
    let report = fixture.report_with(&DocumentOptions::default().with_max_files(1));
    assert!(!report.is_complete());
    assert_eq!(report.unread().len(), 1);
    assert!(matches!(
        report.unread()[0].reason,
        UnreadReason::OutOfBudget { limit: 1 }
    ));
    assert_eq!(report.unread()[0].display_path(), "docs/one.md");
    // The one it did read is still reported, rather than the whole pass being
    // thrown away because it could not finish.
    assert_eq!(report.documents(), [Path::new("README.md")]);
    assert_eq!(texts(&report), ["npm test"]);

    // The number is the sentence's whole job. It says what SURE *read*, so a
    // refusal has to make it lower than the number of documents present — and
    // this is the only test where the two differ, so asserting the clause about
    // not finishing and leaving the count unasserted would leave the count
    // unasserted everywhere.
    let sentence = report.plain_description();
    assert_eq!(
        sentence,
        "SURE read 1 document and found 1 command, and did not finish reading every code block in them, so a command it did not find may be one it did not look for."
    );
}

#[test]
fn a_document_bigger_than_the_limit_is_refused_before_it_is_opened() {
    let fixture = Fixture::new("too large");
    fixture.write("README.md", "```bash\nnpm test\n```\n");
    fixture.write("docs/huge.md", &"x".repeat(4096));

    let report = fixture.report_with(&DocumentOptions::default().with_max_file_bytes(1024));
    assert_eq!(texts(&report), ["npm test"]);
    assert!(!report.is_complete());
    assert_eq!(report.unread().len(), 1);
    assert_eq!(report.unread()[0].display_path(), "docs/huge.md");
    assert_eq!(
        report.unread()[0].reason,
        UnreadReason::TooLarge { limit: 1024 },
        "the limit that was reached, not a measurement of the file"
    );
}

#[test]
fn a_pass_that_has_read_its_byte_budget_refuses_the_rest() {
    let fixture = Fixture::new("byte budget");
    let body = "```bash\nnpm test\n```\n";
    fixture.write("README.md", body);
    fixture.write("docs/one.md", body);
    fixture.write("docs/two.md", body);

    // The budget is on the whole pass rather than on one file, and it is
    // computed from the document rather than guessed at: the first document
    // fits, the second exactly fills the budget, and the third is the one that
    // is refused.
    //
    // It is checked against what has *already* been read, which is what
    // `UnreadReason::OutOfBytes` says in as many words — "the pass had already
    // read as many bytes as it will in one run" — and what the sibling pass in
    // `references.rs` does with the same field names and the same sentence. So
    // a pass overshoots its budget by at most one document. The alternative,
    // refusing a document whose size would take the total over, is a real
    // reading of "budget" and is **not** what either pass does; the two must
    // not come to mean different things, which is why the arithmetic here is
    // spelled out rather than left to a round number that happens to work.
    let one = body.len() as u64;
    let report = fixture.report_with(&DocumentOptions::default().with_max_total_bytes(one + 1));

    assert_eq!(
        report.documents(),
        [Path::new("README.md"), Path::new("docs/one.md")],
        "the two documents that fit before the budget ran out were read"
    );
    assert_eq!(texts(&report), ["npm test", "npm test"]);
    assert_eq!(report.unread().len(), 1);
    assert_eq!(report.unread()[0].display_path(), "docs/two.md");
    assert_eq!(
        report.unread()[0].reason,
        UnreadReason::OutOfBytes { limit: one + 1 }
    );
    assert!(!report.is_complete());
}

#[test]
fn a_file_that_is_not_utf8_text_is_reported_rather_than_decoded_lossily() {
    let fixture = Fixture::new("not text");
    fixture.write("README.md", "```bash\nnpm test\n```\n");
    fixture.write_bytes("docs/broken.md", &[0x23, 0x20, 0xff, 0xfe, 0x0a]);

    let report = fixture.report();
    assert_eq!(texts(&report), ["npm test"]);
    assert!(!report.is_complete());
    assert!(matches!(
        report.unread()[0].reason,
        UnreadReason::NotText { .. }
    ));
    assert_eq!(report.unread()[0].display_path(), "docs/broken.md");
}

#[test]
fn a_path_with_spaces_and_unicode_is_read_like_any_other() {
    // Windows discipline: the fixture path itself has a space in it, and this
    // one adds a document whose name does too.
    let fixture = Fixture::new("spaces");
    fixture.write(
        "docs/getting started — はじめに.md",
        "```bash\nnpm run dev\n```\n",
    );

    let report = fixture.report();
    assert_eq!(texts(&report), ["npm run dev"]);
    assert_eq!(
        report.commands()[0].display_path(),
        "docs/getting started — はじめに.md",
        "the path is reported with forward slashes on every platform"
    );
}

#[test]
fn a_document_full_of_prose_yields_no_commands_and_no_gap() {
    let fixture = Fixture::new("prose only");
    fixture.write(
        "README.md",
        "# Demo\n\
         \n\
         To get started, run `npm install` and then `npm test`. \
         The project requires Node 20 or later.\n\
         \n\
         ## Notes\n\
         \n\
         See CONTRIBUTING.md for how to run the tests.\n",
    );

    let report = fixture.report();
    assert!(
        report.commands().is_empty(),
        "prose is not parsed for commands"
    );
    assert!(report.is_complete());
    assert_eq!(report.documents(), [Path::new("README.md")]);
    // Said plainly, because a reader who expected the two backticked mentions to
    // be found should be able to see that they were not even looked for.
    assert_eq!(
        report.plain_description(),
        "SURE read 1 document and found 0 commands."
    );
}

#[test]
fn a_document_that_names_paths_reports_them_beside_its_commands() {
    let fixture = Fixture::new("documented paths");
    fixture.write(
        "README.md",
        "# Setup\n\
         \n\
         Put your key in `config/local.toml`, following\n\
         [`docs/keys.md`](docs/keys.md#rotation).\n\
         \n\
         ## Build\n\
         \n\
         ```bash\n\
         cp .env.example .env\n\
         ```\n",
    );

    let report = fixture.report();

    // The fence is a command and the prose is not; the prose names two paths and
    // the fence names none, and that separation is the one walk doing both.
    assert_eq!(texts(&report), ["cp .env.example .env"]);
    assert_eq!(paths(&report), ["config/local.toml", "docs/keys.md"]);

    let span = &report.paths()[0];
    assert_eq!(span.line, 3);
    assert_eq!(span.section.as_deref(), Some("Setup"));
    assert_eq!(span.form, PathForm::CodeSpan);
    assert_eq!(span.display_path(), "README.md");

    let link = &report.paths()[1];
    assert_eq!(link.line, 4);
    assert_eq!(link.form, PathForm::LinkTarget);
    assert_eq!(link.text, "docs/keys.md", "the fragment is not a file");
    assert_eq!(link.section.as_deref(), Some("Setup"));
}

#[test]
fn the_paths_of_two_documents_come_back_in_document_and_line_order() {
    let fixture = Fixture::new("path order");
    fixture.write("README.md", "See [a](docs/a.md).\n");
    fixture.write("docs/notes.md", "Then `src/b.ts`.\n");

    let report = fixture.report();
    assert_eq!(
        paths(&report),
        ["docs/a.md", "src/b.ts"],
        "README.md sorts before docs/notes.md and its path is on the earlier line"
    );
    assert_eq!(report.paths()[1].display_path(), "docs/notes.md");
}

#[test]
fn a_document_the_pass_never_opened_contributes_no_paths() {
    let fixture = Fixture::new("paths out of budget");
    fixture.write("README.md", "See [a](docs/a.md).\n");
    fixture.write("docs/notes.md", "See [b](docs/b.md).\n");

    let report = fixture.report_with(&DocumentOptions::default().with_max_files(1));
    assert_eq!(paths(&report), ["docs/a.md"]);
    assert_eq!(
        report.unread().len(),
        1,
        "and the file it did not open is said"
    );
    assert!(!report.is_complete());
}

#[test]
fn the_paths_are_what_the_document_said_and_not_what_is_on_disk() {
    // The boundary this module is, written as a test so that a later reader does
    // not have to take it from a comment: `docs/gone.md` does not exist and
    // `docs/here.md` does, and the report says the same thing about both. Whether
    // a documented path is there is `P4-T005`'s question; this pass answers only
    // what the document named.
    let fixture = Fixture::new("paths unchecked");
    fixture.write("docs/here.md", "nothing in particular\n");
    fixture.write(
        "README.md",
        "See [gone](docs/gone.md) and [here](docs/here.md).\n",
    );

    let report = fixture.report();
    assert_eq!(paths(&report), ["docs/gone.md", "docs/here.md"]);
    assert!(report.is_complete());
    let printed = everything(&report);
    assert!(
        !printed.contains("missing"),
        "the pass that reads documents does not decide that a path is absent: {printed}"
    );
}

#[test]
fn the_report_says_what_sure_did_not_do_with_what_it_found() {
    let note = DocumentReport::execution_note();
    assert!(note.contains("has not run"), "{note}");
    assert!(
        note.contains("just because a document in the project says to"),
        "{note}"
    );
}

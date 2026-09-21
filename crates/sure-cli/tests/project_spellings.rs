//! One directory, several spellings: which of them is one project.
//!
//! `P15-T030`'s subject, measured through a real `sure check` process rather
//! than through the library. A session is recorded for one directory, and the
//! same command is then run against that directory **named five ways**: as the
//! hook recorded it, with forward slashes, with an upper-case drive and
//! upper-case path components, with a trailing separator, and with a `.`
//! segment inside it. Every one of those names one directory to Windows, and
//! before this task the first reported `(capability tier 1, observed)` with
//! four events counted while the other four reported `(capability tier 0,
//! snapshot) SURE counted no session events for this project: SURE's store
//! holds none for it` — a confident, specific and false statement about the
//! store, produced by comparing two path strings instead of two directories.
//!
//! # Why this is a separate file, and why it drives a process
//!
//! The test that should have caught this cannot:
//! `crates/sure-cli/src/check.rs`'s
//! `the_tier_in_a_check_report_is_the_one_the_projects_events_earn` records the
//! session with the same `root` string it later checks, so both sides of the
//! comparison are the identical spelling and any normalisation passes it.
//! Naming the project two ways needs a second reading, and a reading through
//! `sure check` is the one that carries the whole path a user's argument takes
//! — `discovery.root`, the project-root text the pipeline hands the capability
//! report, and the store's own rows — rather than a library call that starts
//! halfway down it.
//!
//! # The direction that must not be traded for this one
//!
//! Making more spellings match is only correct where they really are one
//! directory. A normalisation eager enough to fold two genuinely different
//! projects together would make one project's report count another project's
//! session as its own, raise the tier on evidence that is not about it, and
//! name it in the counted sentence — a false green in the product's own voice,
//! which `CLAUDE.md` ranks above every visible error. So the second test here
//! measures the other direction on the same store: a different directory is
//! never this project, and its events are reported as another project's.
//!
//! # What this file does not claim
//!
//! Whether an upper-case spelling reaches the same directory is a fact about
//! the **volume**, not about the operating system's name: CI run `35544579833`
//! measured macOS and Linux answering oppositely in one workflow. So the
//! volume is asked here, with `sure_core::paths::case_rule_of_volume`, and the
//! assertion is made against the answer that came back — and the test fails
//! rather than falling back when the volume cannot be asked, because a fallback
//! would let the case-keeping arm pass without anything having been asked.
//!
//! # The case-keeping arm asserts the ABSENCE of a verdict, and that is a
//! # correction rather than the original design
//!
//! Asking the volume which case rule it keeps is not the same as knowing what
//! answer the other rule produces, and the first version of this file confused
//! the two: it asserted `(0, 0, 4)` — a different directory that exists — for a
//! spelling that on a case-keeping volume names nothing at all. Nothing on the
//! machine it was written on could reach that arm, so it shipped unexecuted,
//! and CI run `35554370747` reached it on `ubuntu-latest` and failed it. What
//! the arm asserts now is the property it was always for — SURE does not fold a
//! spelling that names nothing into the project — and the `(0, 0, 4)` reading is
//! measured on the second test, where the other directory really is there.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use sure_core::paths::{CaseSensitivity, case_rule_of_volume};

/// The binary this package builds, as cargo hands it to its integration tests.
const SURE: &str = env!("CARGO_BIN_EXE_sure");

/// A directory of this test's own, under the workspace's git-ignored
/// `target/tmp`. See `sure_testkit::scratch` for the reasoning.
fn a_directory_of_our_own(what: &str) -> PathBuf {
    sure_testkit::scratch::directory("sure project spellings", what)
}

/// A project SURE can read, with a name whose case can be flipped — which is
/// what the volume probe reads to answer the case question.
fn a_project_of_our_own(what: &str) -> PathBuf {
    let project = a_directory_of_our_own(what);
    std::fs::write(project.join("README.md"), "# a small project\n")
        .unwrap_or_else(|error| panic!("cannot write into {}: {error}", project.display()));
    // `README.md` is the cased entry the probe flips. A project with nothing in
    // it at all could not be probed, and this test would then be measuring the
    // fallback rather than the answer.
    std::fs::write(project.join("main.py"), "def main():\n    print('hello')\n")
        .unwrap_or_else(|error| panic!("cannot write into {}: {error}", project.display()));
    project
}

/// A store directory for this file's processes to share.
fn a_store_of_our_own() -> PathBuf {
    a_directory_of_our_own("store")
}

/// Record one four-event Codex session against `project`, the way a hook does.
///
/// Through `sure hook ingest`, the product's own entry point for a harness
/// event, and **against the spelling the caller passes** — which is what makes
/// the readings below about a differently-spelt argument rather than about two
/// halves of one string.
///
/// The tool request is a `Read`, which is deliberate: a `PreToolUse` for a
/// `Bash` or a `Write` is *refused* by the execution mode this machine's
/// settings leave in force, and a refused request is a protection decision this
/// file has no business asserting about. A `Read` proceeds under the settings in
/// force, so every event below is recorded by an ingest that finished — and a
/// session whose events are all really in the store is the only precondition
/// the readings mean to be about.
fn record_a_codex_session(store: &Path, project: &str) {
    for (index, event) in ["SessionStart", "PreToolUse", "PostToolUse", "SessionEnd"]
        .iter()
        .enumerate()
    {
        let document = serde_json::json!({
            "hook_event_name": event,
            "session_id": "session-p15t030",
            "cwd": project,
            "tool_name": "Read",
            "tool_input": {"file_path": "README.md"},
            "tool_response": {"ok": true},
            "reason": "other",
            "source": "startup",
        })
        .to_string();
        let mut command = Command::new(SURE);
        command
            .arg("hook")
            .arg("ingest")
            .arg("--store-dir")
            .arg(store)
            .arg("--source")
            .arg("codex")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap_or_else(|error| {
            panic!("cannot start {SURE} for event {index} ({event}): {error}")
        });
        {
            use std::io::Write;
            let stdin = child.stdin.as_mut().expect("stdin was piped");
            stdin
                .write_all(document.as_bytes())
                .unwrap_or_else(|error| panic!("cannot write the event: {error}"));
        }
        let output = child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("the ingest did not finish: {error}"));
        assert!(
            output.status.success(),
            "a Codex {event} was refused, so this test has no session to count: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Run one `sure check` against `project` and hand back its frame, whatever the
/// frame says.
///
/// The machine form rather than the sentence, because the four numbers this
/// task is about — the tier, the events counted, the events attributed to other
/// projects, and whether the run finished at all — are fields in it, and a
/// reader of a human report would have to parse prose to get them.
///
/// Split out from [`check`] because **"did not reach a verdict" is itself an
/// answer some arms have to assert**, not a failure of the test asking. The
/// case-keeping arm below is the one that needs it: on a case-keeping volume an
/// upper-cased spelling does not exist, and a run that refuses to discover a
/// project there is SURE behaving correctly, so a helper that panicked on it
/// would be a test forbidding the right answer.
fn check_raw(store: &Path, project: &str) -> Value {
    let output = Command::new(SURE)
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("--store-dir")
        .arg(store)
        .arg(project)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("cannot start {SURE} to check {project:?}: {error}"));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!(
            "`sure check {project:?}` did not write one JSON object ({error}).\n\
             stdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Run one `sure check` against `project`, requiring that it reached a verdict.
///
/// The arms that assert a tier want [`check`]; the arm that asserts the ABSENCE
/// of one wants [`check_raw`].
fn check(store: &Path, project: &str) -> Value {
    let frame = check_raw(store, project);
    assert_eq!(
        frame["details"]["state"], "finished",
        "the run did not reach a verdict for {project:?}, so it says nothing about the tier: {}",
        frame["details"]["stopped_at"]
    );
    frame
}

/// What a run says about the capability tier, as the four readings this task
/// turns on: the tier, the events counted for this project, the events it read
/// for other projects, and the sentence a person would read.
struct Reading {
    tier: u64,
    events: u64,
    elsewhere: u64,
    summary: String,
}

fn reading_of(frame: &Value) -> Reading {
    let capability = &frame["details"]["report"]["capability"];
    Reading {
        tier: capability["tier"]
            .as_u64()
            .unwrap_or_else(|| panic!("the frame carries no tier: {capability}")),
        events: capability["evidence"]["events"]
            .as_u64()
            .unwrap_or_else(|| panic!("the frame carries no count: {capability}")),
        elsewhere: capability["evidence"]["elsewhere"]
            .as_u64()
            .unwrap_or_else(|| panic!("the frame carries no elsewhere count: {capability}")),
        summary: capability["summary"]
            .as_str()
            .unwrap_or_else(|| panic!("the frame carries no sentence: {capability}"))
            .to_owned(),
    }
}

/// The spellings of one directory that name it on every platform.
///
/// Returned as `(what it is, the spelling)` so a failure names the spelling
/// rather than a number.
fn spellings_that_are_universal(root: &Path) -> Vec<(String, String)> {
    let root_text = root.to_string_lossy().into_owned();
    let mut spellings = vec![("as the hook recorded it".to_owned(), root_text.clone())];
    // On Windows both separators are separators to `Path`'s own parser and to
    // the volume; on Unix this is the same string, because there is no second
    // spelling of a separator there. Either way it names this directory.
    spellings.push((
        "with forward slashes".to_owned(),
        root_text.replace('\\', "/"),
    ));
    spellings.push((
        "with a trailing separator".to_owned(),
        format!("{root_text}{}", std::path::MAIN_SEPARATOR),
    ));
    spellings.push((
        "with a `.` segment inside it".to_owned(),
        root.parent()
            .expect("the scratch directory has a parent")
            .join(".")
            .join(root.file_name().expect("and a name"))
            .to_string_lossy()
            .into_owned(),
    ));
    spellings
}

/// **Every spelling of one directory is one project**, measured by running the
/// command once per spelling against one store.
///
/// The session is recorded once, against the directory as it is spelt on disk.
/// Each spelling below is then handed to a separate `sure check` process, and
/// each must report the same tier, the same count, and no events attributed to
/// another project.
#[test]
fn the_project_named_the_ways_windows_treats_as_one_directory_reports_one_tier() {
    let store = a_store_of_our_own();
    let project = a_project_of_our_own("DemoProj");
    record_a_codex_session(&store, &project.to_string_lossy());

    for (what, spelling) in spellings_that_are_universal(&project) {
        let reading = reading_of(&check(&store, &spelling));
        assert_eq!(
            (reading.tier, reading.events, reading.elsewhere),
            (1, 4, 0),
            "the project {what} ({spelling:?}) is the directory the session was recorded for, \
             and the report did not count its events. It said: {}",
            reading.summary
        );
        assert!(
            reading
                .summary
                .contains("SURE counted 4 session events recorded for this project"),
            "the sentence for {what} ({spelling:?}) does not say what was counted: {}",
            reading.summary
        );
    }

    // The spelling whose answer is not the same on every platform, and the
    // volume is asked rather than the operating system — see this file's header.
    let upper_case = project.to_string_lossy().to_uppercase();
    let case = case_rule_of_volume(&project).unwrap_or_else(|| {
        panic!(
            "the volume holding {} could not be asked, so this test could not say which answer \
             it was asserting about {upper_case:?}",
            project.display()
        )
    });
    match case {
        CaseSensitivity::Insensitive => {
            let reading = reading_of(&check(&store, &upper_case));
            assert_eq!(
                (reading.tier, reading.events, reading.elsewhere),
                (1, 4, 0),
                "this volume folds case, so {upper_case:?} is the directory the session was \
                 recorded for, and the report did not count its events. It said: {}",
                reading.summary
            );
        }
        CaseSensitivity::Sensitive => {
            // On a case-keeping volume the upper-cased spelling is not a
            // differently-spelt name for this directory — it is a path that does
            // not exist, and the two are different assertions.
            //
            // This arm asserted `(0, 0, 4)` — the answer for a DIFFERENT
            // directory that DOES exist — and it was never executed on the
            // machine the task was written on, because that volume folds case.
            // CI run `35554370747` reached it for the first time and it failed,
            // for the reason the arm could not have been right: the run stopped
            // at discovery instead of reaching a verdict, so the helper's
            // `finished` assertion fired and the tier assertion was never even
            // the thing being tested.
            //
            // `a_different_directory_is_never_this_project` below already
            // measures `(0, 0, 4)`, on a second directory that really exists.
            // What is asserted here instead is the thing this arm is FOR: SURE
            // must not fold a spelling that names nothing into the project. A
            // report that counted the project's four events would be that fold.
            let frame = check_raw(&store, &upper_case);
            assert_ne!(
                frame["details"]["state"], "finished",
                "this volume keeps case, so {upper_case:?} does not exist and is not the directory \
                 the session was recorded for — yet SURE reached a verdict about it: {frame}"
            );
            assert_eq!(
                frame["details"]["stopped_at"], "discover",
                "the run over the non-existent spelling {upper_case:?} stopped somewhere other \
                 than discovery, so this arm has stopped measuring what it says it measures: \
                 {frame}"
            );

            // The control, and the reason the assertion above is worth making: a
            // run that refused EVERY path would satisfy it. The project is still
            // read by its own spelling, and must still count its own session.
            let reading = reading_of(&check(&store, &project.to_string_lossy()));
            assert_eq!(
                (reading.tier, reading.events, reading.elsewhere),
                (1, 4, 0),
                "asking about a spelling that names nothing changed what this project's own \
                 report says. It said: {}",
                reading.summary
            );
        }
    }
}

/// **A different directory is never this project**, and its own session is
/// never counted as this project's.
///
/// The direction the fix must not trade away, measured on the same store as the
/// test above: two sibling directories, one session, one of them asked about.
/// The sibling is a genuinely different project, so the report must count
/// nothing and must say how many events it read for other projects — the
/// sentence that lets a reader diagnose the understating case rather than
/// concluding SURE has never seen them.
///
/// The last reading is the control. A report that counted nothing for
/// *everything* would satisfy every assertion before it, so the directory the
/// session really was recorded for is read here too and must count four.
#[test]
fn a_different_directory_is_never_this_project() {
    let store = a_store_of_our_own();
    let mine = a_project_of_our_own("mine");
    let theirs = a_project_of_our_own("theirs");
    record_a_codex_session(&store, &mine.to_string_lossy());

    let reading = reading_of(&check(&store, &theirs.to_string_lossy()));
    assert_eq!(
        (reading.tier, reading.events, reading.elsewhere),
        (0, 0, 4),
        "a different directory's report counted this project's session: {}",
        reading.summary
    );
    assert!(
        reading
            .summary
            .contains("SURE also read 4 session events recorded for other projects"),
        "the sentence does not say that the events it read belong to another project: {}",
        reading.summary
    );

    let control = reading_of(&check(&store, &mine.to_string_lossy()));
    assert_eq!(
        (control.tier, control.events, control.elsewhere),
        (1, 4, 0),
        "the control: the directory the session was recorded for does count it: {}",
        control.summary
    );
}

//! The command line contract, checked where a user meets it: the process.
//!
//! # Why this file runs the binary
//!
//! The two acceptance criteria — that the commands parse, and that the human
//! and machine-readable output paths are separated — are claims about what a
//! *process* does. Which stream a byte went to, and what status the process
//! returned, are not observable from inside: the unit tests in `src/` check the
//! decisions, and this file checks that the decisions are what a shell sees.
//!
//! A test that called the renderers directly would pass while the binary wrote
//! both of them to stdout, which is precisely the failure the criterion exists
//! to rule out.
//!
//! # What is asserted
//!
//! 1. the status, for every command the docs list;
//! 2. that the human form of a refusal goes to stderr and the machine form to
//!    stdout, and that neither writes to the other's stream;
//! 3. that the machine form is one line of JSON, so a pipeline can read a
//!    stream of responses;
//! 4. that a command with nothing to do never exits 0 — SURE's own false green;
//! 5. that nothing in the crate can write to a stream outside `output.rs`.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The binary this package builds, as cargo hands it to its integration tests.
const SURE: &str = env!("CARGO_BIN_EXE_sure");

/// What one run of the binary did.
struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

impl Run {
    fn succeeded(&self) -> bool {
        self.status == 0
    }
}

fn run(args: &[&str]) -> Run {
    let output = Command::new(SURE)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("could not run {SURE}: {error}"));
    Run {
        status: output.status.code().expect("the process exited on its own"),
        stdout: String::from_utf8(output.stdout).expect("stdout is utf-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is utf-8"),
    }
}

/// Every command `docs/architecture/CLI.md` lists, with arguments that reach it.
///
/// A command that needs a subcommand is given one, so that this list tests the
/// command rather than the grammar's refusal to run half of one.
const EVERY_COMMAND: &[&[&str]] = &[
    &["check"],
    &["recheck"],
    &["repair"],
    &["history"],
    &["history", "list"],
    &["history", "show"],
    &["history", "delete"],
    &["history", "export"],
    &["doctor"],
    &["config"],
    &["config", "paths"],
    &["config", "show"],
    &["config", "validate"],
    &["hook", "ingest"],
    &["explain"],
    &["protocol"],
    &["version"],
];

/// The commands this build carries out.
///
/// Every other command in [`EVERY_COMMAND`] answers `unavailable`, and the
/// statuses below are only meaningful while that holds.
const IMPLEMENTED: &[&[&str]] = &[
    &["check"],
    &["doctor"],
    &["hook", "ingest"],
    &["protocol"],
    &["recheck"],
    &["repair"],
    &["version"],
];

/// The commands whose answer is the same on every machine, and so is a status
/// this file can demand.
///
/// `doctor` is not one of them, and that is the point of it: it reports on the
/// machine it runs on, and a machine where SURE found something wrong about its
/// own files earns status 1. Demanding 0 here would turn this file into a claim
/// that the machine running it is clean, which is a claim about a machine and
/// not about SURE.
const ALWAYS_OK: &[&[&str]] = &[&["protocol"], &["version"]];

/// The commands that ran and answered nothing, used where a refusal is the
/// subject rather than a report.
///
/// `check`, `recheck` and `repair` used to be here and are not any more: they
/// answer with a verdict, and a verdict is an answer — it goes to standard
/// output and it is what a person piping the report to a file wanted. What a
/// refusal looks like is still worth testing, so the commands that *are* refused
/// stay, including the destructive one whose wording a user needs most.
const REFUSED: &[&[&str]] = &[
    &["history", "list"],
    &["history", "delete"],
    &["explain"],
    &["config"],
];

#[test]
fn every_documented_command_parses() {
    // The first acceptance criterion, literally. `parse` here means the process
    // reached its own code: a command line clap rejects exits 2 and prints
    // "unrecognized subcommand", and both are checked for rather than assumed.
    for args in EVERY_COMMAND {
        let run = run(args);
        assert_ne!(
            run.status,
            2,
            "`sure {}` was not accepted:\n{}",
            args.join(" "),
            run.stderr
        );
        assert!(
            !run.stderr.contains("unrecognized subcommand")
                && !run.stderr.contains("unexpected argument"),
            "`sure {}` was rejected by the parser:\n{}",
            args.join(" "),
            run.stderr
        );
    }
}

#[test]
fn no_command_that_did_nothing_reports_success() {
    // The criterion this file exists for. Every command here is recognised and
    // none of them can do its job yet, so every one of them must say so with a
    // non-zero status — including the ones that only *look* harmless, like
    // printing a history that happens to be empty. A script that ran `sure
    // check` in CI today must fail the build, not pass it.
    for args in EVERY_COMMAND {
        if IMPLEMENTED.contains(args) {
            continue;
        }
        let run = run(args);
        assert_ne!(
            run.status,
            0,
            "`sure {}` exited 0 without doing anything:\nstdout: {}\nstderr: {}",
            args.join(" "),
            run.stdout,
            run.stderr
        );
        assert_eq!(
            run.status,
            3,
            "`sure {}` failed with {}, which is not the status for a command that \
             exists and cannot run:\n{}",
            args.join(" "),
            run.status,
            run.stderr
        );
    }
}

#[test]
fn the_commands_that_work_report_success() {
    // The other half. A build where nothing exits 0 is a build that has stopped
    // answering, and the previous test would be satisfied by it.
    for args in ALWAYS_OK {
        let run = run(args);
        assert!(
            run.succeeded(),
            "`sure {}` did not succeed:\n{}",
            args.join(" "),
            run.stderr
        );
        assert!(!run.stdout.trim().is_empty(), "it printed nothing");
    }
}

#[test]
fn a_doctor_report_is_an_answer_however_it_turns_out() {
    // `sure doctor` is the first command whose status depends on what it found,
    // so it is the first place where the two ways of reading a result can come
    // apart: the status says whether the answer was clean, and the streams say
    // whether there was an answer at all. Both are checked against the frame the
    // same run produced, rather than against what this machine happens to hold.
    let human = run(&["doctor"]);
    assert!(
        matches!(human.status, 0 | 1),
        "`sure doctor` exited {}. It either answered (0) or answered that something is \
         wrong (1); anything else means it did not run:\n{}",
        human.status,
        human.stderr
    );
    assert!(
        human.stderr.is_empty(),
        "the report went to standard error, so `sure doctor > report.txt` would leave \
         the file empty:\n{}",
        human.stderr
    );
    assert!(!human.stdout.trim().is_empty(), "it printed nothing");

    let machine = run(&["--format", "json", "doctor"]);
    let frame: serde_json::Value = serde_json::from_str(machine.stdout.trim())
        .unwrap_or_else(|error| panic!("`sure --format json doctor` is not JSON: {error}"));
    assert_eq!(
        machine.status, human.status,
        "the two paths disagree about the status:\n{}",
        machine.stdout
    );
    assert_eq!(
        frame["outcome"].as_str(),
        Some(if human.status == 0 { "ok" } else { "not_green" }),
        "the body and the status disagree about the same run: {frame}"
    );
    assert_eq!(
        frame["exit_code"].as_i64(),
        Some(i64::from(human.status)),
        "the frame's own status is not the one the process returned: {frame}"
    );
    // The reason a script can act on, and the reason the status is 1: a machine
    // that is clean reports no problems, and the frame is where that list is.
    let problems = frame["details"]["problems"]
        .as_array()
        .expect("a doctor frame lists what it found wrong, even when the list is empty");
    assert_eq!(
        problems.is_empty(),
        human.status == 0,
        "the status and the problems disagree: {frame}"
    );
}

#[test]
fn the_protocol_version_sure_reports_is_the_one_it_will_talk_to() {
    // The handshake rule is exact equality, so the number SURE prints and the
    // number that gets a yes have to be the same number. Taken from the frame
    // rather than written here: a copy in this file would keep passing after the
    // two had drifted apart, and a build that announced one version while
    // accepting another is the failure a handshake exists to prevent.
    let reported: serde_json::Value =
        serde_json::from_str(run(&["--format", "json", "protocol"]).stdout.trim())
            .expect("`sure protocol` answers with a frame");
    let speaks = u32::try_from(reported["protocol_version"].as_u64().expect("a version"))
        .expect("a protocol version this build can speak");

    let agreed = run(&["protocol", "--speaks", &speaks.to_string()]);
    assert!(
        agreed.succeeded(),
        "this build speaks protocol {speaks} and would not agree to talk to it:\n{}",
        agreed.stderr
    );
    assert!(
        agreed.stderr.is_empty(),
        "a caller that can talk to SURE was handed a complaint as well as an answer, and a \
         complaint on that stream is how an adapter decides to give up:\n{}",
        agreed.stderr
    );
    assert!(!agreed.stdout.trim().is_empty(), "it printed nothing");

    // Both directions of the refusal, because they are not the same answer. A
    // build that said no to everything would pass a check that only ever asked
    // about one number, and a caller told the wrong direction retries with the
    // fix that cannot work.
    assert!(
        speaks > 0,
        "the older-caller direction has no version below {speaks} to be asked about"
    );
    for (other, moves) in [(speaks + 1, "sure"), (speaks - 1, "caller")] {
        let other_text = other.to_string();
        let args = ["protocol", "--speaks", other_text.as_str()];

        let refused = run(&args);
        assert_eq!(
            refused.status, 3,
            "`sure protocol --speaks {other}` returned {}. A protocol this build will not \
             speak is a command it cannot carry out; a caller that read 0 would send events \
             SURE then refuses:\n{}",
            refused.status, refused.stderr
        );
        assert!(
            refused.stdout.is_empty(),
            "the complaint went to standard output, where a caller reading an answer would \
             take it for one:\n{}",
            refused.stdout
        );
        assert!(
            refused.stderr.contains(&other_text) && refused.stderr.contains(&speaks.to_string()),
            "`sure protocol --speaks {other}` does not name both versions, so the person \
             reading it cannot tell which one to change:\n{}",
            refused.stderr
        );

        let mut machine: Vec<&str> = vec!["--format", "json"];
        machine.extend_from_slice(&args);
        let frame = run(&machine);
        assert_eq!(frame.status, refused.status, "{}", frame.stdout);
        let frame: serde_json::Value =
            serde_json::from_str(frame.stdout.trim()).expect("one frame");
        assert_eq!(frame["outcome"].as_str(), Some("unavailable"), "{frame}");
        assert_eq!(
            frame["details"]["agreed"],
            serde_json::json!(false),
            "{frame}"
        );
        assert_eq!(
            frame["details"]["update"].as_str(),
            Some(moves),
            "the frame does not say which side has to move: {frame}"
        );
        assert_eq!(
            frame["details"]["caller_speaks"].as_u64(),
            Some(u64::from(other))
        );
        assert_eq!(
            frame["details"]["sure_speaks"].as_u64(),
            Some(u64::from(speaks))
        );
    }
}

#[test]
fn a_refusal_reads_on_the_terminal_and_leaves_standard_output_empty() {
    // `sure check > report.txt` has to put a report in the file and leave the
    // complaint where the person can see it. For a command that produced
    // nothing, the file must therefore be empty rather than full of prose.
    for args in REFUSED {
        let run = run(args);
        assert!(
            run.stdout.is_empty(),
            "`sure {}` wrote to standard output when it had no answer:\n{}",
            args.join(" "),
            run.stdout
        );
        assert!(
            run.stderr.contains("is not implemented in this build"),
            "`sure {}` did not say what was wrong:\n{}",
            args.join(" "),
            run.stderr
        );
    }
}

#[test]
fn the_machine_form_is_one_object_on_one_line_of_standard_output() {
    // A pipeline reads a stream of responses by splitting on newlines, so the
    // machine form is one line, always, and never carries a complaint from
    // somewhere else in the program.
    for args in EVERY_COMMAND {
        let mut machine: Vec<&str> = vec!["--format", "json"];
        machine.extend_from_slice(args);
        let run = run(&machine);

        assert!(
            run.stderr.is_empty(),
            "`sure {} --format json` wrote to standard error, which a script did not ask \
             for:\n{}",
            args.join(" "),
            run.stderr
        );
        assert_eq!(
            run.stdout.lines().count(),
            1,
            "`sure {} --format json` wrote {} lines:\n{}",
            args.join(" "),
            run.stdout.lines().count(),
            run.stdout
        );
        assert!(run.stdout.ends_with('\n'), "the line is not terminated");

        let frame: serde_json::Value = serde_json::from_str(run.stdout.trim())
            .unwrap_or_else(|error| panic!("`sure {}` is not JSON: {error}", args.join(" ")));
        assert_eq!(
            frame["command"].as_str().map(|name| name.split(' ').next()),
            Some(Some(args[0])),
            "the frame names a different command than the one that ran"
        );
        assert!(
            frame["protocol_version"].is_u64() && frame["sure_version"].is_string(),
            "the frame does not say which build and protocol produced it: {frame}"
        );
    }
}

#[test]
fn a_goal_with_no_words_is_a_failure_and_not_a_wrong_command_line() {
    // The one way this file can reach `--goal` from here. A goal with words in
    // it is written to the store **this machine really uses**, and there is no
    // environment variable that could point it somewhere else — on Windows the
    // data directory comes from `SHGetKnownFolderPath`, which ignores
    // `LOCALAPPDATA`. So a test here would be adding an invented requirement to
    // the history of whoever ran the suite, and it would look like a passing
    // test. A goal with no words in it is refused before SURE looks for its
    // store, which makes this the whole of the flag's process-level coverage;
    // `src/check.rs` drives the rest against locations it names.
    //
    // Status 5, and the two statuses it is not: 2 would mean the parser rejected
    // the command line, and `--goal ""` is accepted — an empty goal is a goal
    // with nothing in it, which is a thing a user can type. 3 would mean this
    // build cannot record a goal, and it can. 5 is "it tried and did not finish",
    // which is what happened.
    for args in [
        &["check", "--goal", ""][..],
        &["check", "--goal", "   "][..],
    ] {
        let human = run(args);
        assert_eq!(
            human.status,
            5,
            "`sure {}` returned {}. A goal with no words in it is a run that could not \
             finish:\n{}",
            args.join(" "),
            human.status,
            human.stderr
        );
        assert!(
            human.stdout.is_empty(),
            "the complaint went to standard output, where `sure check > report.txt` would \
             put it in the file and read as a report:\n{}",
            human.stdout
        );
        assert!(
            human.stderr.contains("could not finish"),
            "`sure check --goal ''` does not say it did not finish:\n{}",
            human.stderr
        );
        // The promise a user has to be able to rely on without reading the code:
        // a run that failed did not change their history.
        assert!(
            human.stderr.contains("Nothing was recorded"),
            "`sure check --goal ''` does not say whether the history changed:\n{}",
            human.stderr
        );
        assert!(
            !human.stderr.contains("is not implemented in this build"),
            "a run that tried is worded as a command this build lacks:\n{}",
            human.stderr
        );

        let mut machine: Vec<&str> = vec!["--format", "json"];
        machine.extend_from_slice(args);
        let machine = run(&machine);
        assert_eq!(machine.status, human.status, "{}", machine.stdout);
        let frame: serde_json::Value =
            serde_json::from_str(machine.stdout.trim()).expect("one frame");
        assert_eq!(frame["outcome"].as_str(), Some("failed"), "{frame}");
        assert_eq!(frame["exit_code"].as_i64(), Some(5), "{frame}");
        assert_eq!(frame["command"].as_str(), Some("check"), "{frame}");
        assert_eq!(
            frame["details"]["what"].as_str(),
            human
                .stderr
                .lines()
                .find(|line| line.contains("Nothing was recorded"))
                .map(str::trim),
            "the two paths disagree about what did not happen: {frame}"
        );
        assert!(
            frame["details"]["detail"]
                .as_str()
                .is_some_and(|detail| !detail.is_empty()),
            "the frame does not say what stopped the run: {frame}"
        );
    }
}

#[test]
fn the_two_paths_disagree_about_nothing_that_matters() {
    // The separation has to be a difference in *shape*, not in *content*. If
    // the machine form and the human form could describe different outcomes,
    // a person and their build script would be reading two different products.
    for args in REFUSED {
        let human = run(args);
        let mut machine: Vec<&str> = vec!["--format", "json"];
        machine.extend_from_slice(args);
        let machine = run(&machine);
        let frame: serde_json::Value = serde_json::from_str(machine.stdout.trim()).unwrap();

        assert_eq!(
            human.status,
            i32::try_from(frame["exit_code"].as_i64().expect("an exit code"))
                .expect("a status that fits"),
            "the two paths disagree about the status for `sure {}`",
            args.join(" ")
        );
        let instead = frame["instead"]
            .as_str()
            .expect("a refusal says what happened");
        assert!(
            human.stderr.contains(instead),
            "the human form of `sure {}` does not say what the machine form does:\n\
             machine: {instead}\nhuman: {}",
            args.join(" "),
            human.stderr
        );
    }
}

#[test]
fn a_check_of_a_real_project_answers_with_a_verdict_and_never_with_status_three() {
    // P7-T010's acceptance, at the process. `sure check` used to exit 3 — "this
    // build cannot carry that out" — and the reason was true when it was
    // written: nothing sequenced the stages. It does now, so the status that
    // says "SURE cannot check a project" is a lie about this build, and 1 —
    // "it checked, and the project is not clean" — is what happened.
    //
    // The project is this crate, which is a real one: a `Cargo.toml`, sources,
    // and two declared commands that would run its code.
    let human = run(&["check"]);
    assert_eq!(
        human.status, 1,
        "`sure check` returned {}. 3 would mean this build cannot check a project, which it \
         can; 0 would mean the project is clean, which no run in this build can establish.\n\
         stdout:\n{}\nstderr:\n{}",
        human.status, human.stdout, human.stderr
    );
    assert!(
        human.stdout.contains("SURE checked"),
        "the report does not say what it is about:\n{}",
        human.stdout
    );
    assert!(
        human.stdout.contains("NOT CHECKED"),
        "the run does not mark what it did not check:\n{}",
        human.stdout
    );
    assert!(
        human.stdout.contains("SURE exited with status 1"),
        "the report does not say what the status means, once the exit code is gone into a \
         pipe:\n{}",
        human.stdout
    );
    assert!(
        !human.stderr.contains("is not implemented in this build"),
        "a command that ran is worded as one this build lacks:\n{}",
        human.stderr
    );

    // The same run, read by a script. What must never appear there is `ok`, and
    // it must not appear for the reason the stage log gives.
    let machine = run(&["--format", "json", "check"]);
    assert_eq!(machine.status, human.status);
    let frame: serde_json::Value =
        serde_json::from_str(machine.stdout.trim()).expect("one frame on one line");
    assert_eq!(frame["command"].as_str(), Some("check"));
    assert_ne!(
        frame["outcome"].as_str(),
        Some("ok"),
        "a run that did not finish its stages reported itself clean: {frame}"
    );
    assert_eq!(
        frame["details"]["state"].as_str(),
        Some("finished"),
        "{frame}"
    );
    assert_eq!(
        frame["details"]["green"],
        serde_json::json!(false),
        "{frame}"
    );
    assert_eq!(frame["details"]["stopped_at"], serde_json::Value::Null);
    let stages = frame["details"]["stages"]
        .as_array()
        .expect("the frame carries the stage log");
    assert_eq!(stages.len(), 12, "{frame}");
    assert!(
        stages
            .iter()
            .any(|stage| stage["outcome"] == serde_json::json!("not_run")),
        "this build runs no project code, so a run over a real crate must record a stage \
         that did not run: {frame}"
    );
    assert!(
        stages.iter().all(|stage| stage["detail"]
            .as_str()
            .is_some_and(|text| !text.is_empty())),
        "a stage does not say what it did: {frame}"
    );
}

#[test]
fn the_three_pipeline_commands_are_one_orchestrator_with_three_purposes() {
    // "`sure recheck` and `sure repair` reach the same orchestrator through
    // `Command::report` rather than a second path into the engine." Compared
    // here rather than asserted: the three runs are about the same project, they
    // walk the same twelve stages in the same order, and the only differences
    // are the stages at the end that say what each command is for.
    let mut frames = Vec::new();
    for command in ["check", "recheck", "repair"] {
        let mut args = vec!["--format", "json", command];
        let run = run(&args);
        assert_eq!(run.status, 1, "`sure {command}` returned {}", run.status);
        let frame: serde_json::Value = serde_json::from_str(run.stdout.trim())
            .unwrap_or_else(|error| panic!("`sure {command}` is not one frame: {error}"));
        assert_eq!(frame["command"].as_str(), Some(command), "{frame}");
        args.clear();
        frames.push(frame);
    }

    let project = frames[0]["details"]["project"].clone();
    assert!(project.is_string(), "{}", frames[0]);
    for frame in &frames {
        assert_eq!(
            frame["details"]["project"], project,
            "two commands checked two different projects"
        );
        assert_eq!(frame["details"]["mode"], frames[0]["details"]["mode"]);
        let names: Vec<&str> = frame["details"]["stages"]
            .as_array()
            .expect("a stage log")
            .iter()
            .map(|stage| stage["stage"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            names,
            [
                "discover",
                "resolve-intent",
                "fingerprint",
                "plan",
                "static-checks",
                "dynamic-checks",
                "completeness",
                "model-assessment",
                "claim-checking",
                "aggregate",
                "repair-contract",
                "recheck",
            ],
            "`sure {}` did not walk the documented stages in order",
            frame["details"]["purpose"]
        );
    }

    // The last two stages are what separates them, and each says which of the
    // three it is answering.
    let last = |frame: &serde_json::Value| frame["details"]["stages"][11]["outcome"].clone();
    assert_eq!(frames.iter().map(last).collect::<Vec<_>>().len(), 3);
    assert_eq!(
        frames[2]["details"]["stages"][11]["outcome"],
        serde_json::json!("not_part_of_work"),
        "`sure repair` claims to compare against an earlier run: {}",
        frames[2]
    );
    assert_ne!(
        frames[1]["details"]["stages"][11]["outcome"],
        serde_json::json!("not_part_of_work"),
        "`sure recheck` recorded its own stage as not part of the run: {}",
        frames[1]
    );
    assert_ne!(
        frames[2]["details"]["stages"][10]["outcome"],
        serde_json::json!("not_part_of_work"),
        "`sure repair` recorded the stage it exists for as not part of the run: {}",
        frames[2]
    );
    assert_eq!(
        frames[0]["details"]["stages"][10]["outcome"],
        serde_json::json!("not_part_of_work"),
        "`sure check` claims to write repair instructions: {}",
        frames[0]
    );
}

#[test]
fn a_project_that_cannot_be_read_is_status_five_and_not_status_three() {
    // The other half of the status rule, and the one a user meets by mistyping a
    // path. 5 is "it tried and did not finish": a statement about this run. 3 is
    // "this build cannot carry that out": a statement about the build, whose
    // remedy is a newer SURE — and telling a user that would send them looking
    // for an upgrade instead of at their path.
    let missing = format!("{}/does-not-exist", env!("CARGO_MANIFEST_DIR"));
    let human = run(&["check", &missing]);
    assert_eq!(
        human.status, 5,
        "`sure check <missing>` returned {}:\n{}",
        human.status, human.stderr
    );
    assert!(
        human.stdout.is_empty(),
        "a run with no verdict wrote to standard output:\n{}",
        human.stdout
    );
    assert!(
        human.stderr.contains("could not finish"),
        "the complaint does not say the run did not finish:\n{}",
        human.stderr
    );
    assert!(
        human.stderr.contains("No verdict was produced"),
        "a failed run does not say that it is not a statement about the project:\n{}",
        human.stderr
    );
    assert!(
        !human.stderr.contains("is not implemented in this build"),
        "a run that tried is worded as a command this build lacks:\n{}",
        human.stderr
    );

    let machine = run(&["--format", "json", "check", &missing]);
    assert_eq!(machine.status, 5);
    let frame: serde_json::Value = serde_json::from_str(machine.stdout.trim()).expect("one frame");
    assert_eq!(frame["outcome"].as_str(), Some("failed"), "{frame}");
    assert_eq!(frame["exit_code"].as_i64(), Some(5), "{frame}");
    assert_eq!(
        frame["details"]["state"].as_str(),
        Some("stopped"),
        "a run that produced no verdict says it finished: {frame}"
    );
    assert_eq!(
        frame["details"]["stopped_at"].as_str(),
        Some("discover"),
        "the frame does not say where the run stopped: {frame}"
    );
    assert_eq!(
        frame["details"]["report"],
        serde_json::Value::Null,
        "a run that produced no verdict carries one: {frame}"
    );
}

#[test]
fn the_format_flag_is_taken_in_both_positions_and_refuses_a_third_value() {
    let before = run(&["--format", "json", "version"]);
    let after = run(&["version", "--format", "json"]);
    assert_eq!(before.stdout, after.stdout);
    assert!(before.succeeded() && after.succeeded());

    // An unknown format is a wrong command line, not a silent fall back to the
    // human one. Falling back would put prose where a script expected JSON, and
    // the script would report a parse error rather than the real problem.
    let wrong = run(&["--format", "yaml", "version"]);
    assert_eq!(wrong.status, 2, "{}", wrong.stderr);
    assert!(wrong.stdout.is_empty(), "{}", wrong.stdout);
}

#[test]
fn an_unknown_command_or_flag_is_a_wrong_command_line() {
    // 2, not 3 and not 0. The three mean different things and a user acts on
    // them differently: 2 is "you typed it wrong", 3 is "SURE cannot do that",
    // 0 is "it is done".
    for args in [
        &["chekc"][..],
        &["check", "--deep"][..],
        &["--format"][..],
        &[][..],
        // A version is a number. An adapter that passed a word — a tag, a
        // branch, `latest` — must be told it typed the command line wrong,
        // rather than be answered "SURE cannot talk to that", which reads as a
        // version that exists and is refused.
        &["protocol", "--speaks", "latest"][..],
        &["protocol", "--speaks"][..],
    ] {
        let run = run(args);
        assert_eq!(
            run.status,
            2,
            "`sure {}` returned {}:\n{}",
            args.join(" "),
            run.status,
            run.stderr
        );
    }
}

#[test]
fn the_two_spellings_of_the_version_agree_about_the_number() {
    // `sure version` is SURE's own rendering and `sure --version` is clap's.
    // Two spellings of one fact is where drift starts, so the number is what is
    // compared and the difference in prefix is not.
    let subcommand = run(&["version"]);
    let flag = run(&["--version"]);
    assert!(subcommand.succeeded() && flag.succeeded());
    let number = env!("CARGO_PKG_VERSION");
    assert!(subcommand.stdout.contains(number), "{}", subcommand.stdout);
    assert!(flag.stdout.contains(number), "{}", flag.stdout);
}

#[test]
fn hook_ingest_reads_standard_input_and_evaluates_protection() {
    // P11-T006: `sure hook ingest` reads stdin, normalises the event, and
    // for pre-tool-use events evaluates a protection decision.
    let payload = r#"{
        "event": "preToolUse",
        "harness_session_id": "cursor-session-001",
        "tool": "Shell",
        "args": {"command": "npm test", "workdir": "C:\\Users\\dev\\sample-project"},
        "timestamp_utc": "2026-09-18T12:01:00Z",
        "source": "cursor"
    }"#;

    let mut child = Command::new(SURE)
        .args([
            "--format",
            "json",
            "hook",
            "ingest",
            "--source",
            "cursor",
            "pre-tool-use",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("a child process");
    let mut stdin = child.stdin.take().expect("the pipe");
    stdin.write_all(payload.as_bytes()).expect("write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("the child exits");
    let status = output.status.code().expect("the process exited on its own");
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");

    // Default config is inspect_only, so Shell (ArbitraryCommand) is blocked.
    assert_ne!(status, 0, "a blocked tool should exit non-zero: {stdout}");

    let frame: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|error| panic!("hook ingest is not valid JSON: {error}\n{stdout}"));
    assert_eq!(frame["decision"].as_str(), Some("block"), "{frame}");
    assert!(
        frame["reason"].as_str().is_some_and(|r| !r.is_empty()),
        "a block decision should carry a reason: {frame}"
    );
}

// --- the source scan ----------------------------------------------------

/// The crate's own sources, with the path they came from.
fn sources() -> Vec<(PathBuf, String)> {
    fn walk(directory: &PathBuf, into: &mut Vec<(PathBuf, String)>) {
        let entries = std::fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                into.push((path, text));
            }
        }
    }
    let mut found = Vec::new();
    walk(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    assert!(!found.is_empty(), "the crate has no sources");
    found
}

#[test]
fn only_the_output_module_writes_to_a_stream() {
    // "Human and machine-readable output are separated" is satisfied by there
    // being no third way out. A `println!` anywhere else is the one thing that
    // can break that: it is invisible until somebody pipes the output into a
    // JSON reader, and by then it is in a release.
    //
    // A source scan rather than a test of behaviour, because what is being
    // ruled out is an *absence*, and no run of this binary can demonstrate that
    // a line of code was never written. The tokens are written with the call's
    // syntax so that prose about them does not match.
    const ALLOWED: &str = "output.rs";
    const WRITES: &[&str] = &[
        "println!(",
        "print!(",
        "eprintln!(",
        "eprint!(",
        "io::stdout()",
        "io::stderr()",
    ];

    for (path, text) in sources() {
        if path.file_name().is_some_and(|name| name == ALLOWED) {
            continue;
        }
        for token in WRITES {
            assert!(
                !text.contains(token),
                "{} writes to a stream with `{token}`. Output goes through \
                 src/output.rs, which is what keeps the two paths apart.",
                path.display()
            );
        }
    }
}

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
//! 5. that nothing in the crate can write to a stream outside `output.rs`;
//! 6. that the store's location is the caller's to name and no one else's, and
//!    that a full run of this file leaves the store of the person running it
//!    byte-identical.
//!
//! # Where these runs keep their evidence
//!
//! **Every process this file starts names a store with `--store-dir`**, in a
//! directory of its own under `target/tmp` — see [`a_store_of_our_own`]. Before
//! that flag existed, the runs that write (`sure check --goal`, `sure hook
//! ingest`) wrote the store of whoever ran the suite, and there was no
//! environment variable that could have stopped them: on Windows the per-user
//! data directory comes from `SHGetKnownFolderPath`, which ignores
//! `LOCALAPPDATA`. One test here names no store at all, and it is the one that
//! checks a caller who names nothing still gets the platform's own location; it
//! reads that store and compares its bytes before and after, so a redirect that
//! had been dropped fails a test rather than editing a history.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sure_core::full_recording::DEFAULT_FULL_RECORDING_RETENTION_DAYS;
use sure_core::paths::Paths;
use sure_core::session_event_store::SessionEventStore;
use sure_core::store::{HistoryFilter, RecordKind, Store};

/// The binary this package builds, as cargo hands it to its integration tests.
const SURE: &str = env!("CARGO_BIN_EXE_sure");

/// What one run of the binary did.
struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

impl Run {
    /// What the process left behind.
    fn of(output: &Output) -> Self {
        Self {
            status: output.status.code().expect("the process exited on its own"),
            stdout: String::from_utf8(output.stdout.clone()).expect("stdout is utf-8"),
            stderr: String::from_utf8(output.stderr.clone()).expect("stderr is utf-8"),
        }
    }

    fn succeeded(&self) -> bool {
        self.status == 0
    }
}

/// A directory of this test's own, under the workspace's git-ignored `target/tmp`.
///
/// Unique per call, and made unique by `create_dir` rather than by the name, so
/// that two tests in one process cannot be handed the same one. Never cleared:
/// clearing a fixed path and then treating it as fresh fails on Windows, where a
/// deletion can fail silently, and the test then describes a directory that was
/// never emptied.
fn a_directory_of_our_own(what: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("tmp")
        .join("sure cli contract");
    std::fs::create_dir_all(&base)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
    for _ in 0..1_000 {
        let candidate = base.join(format!("{what}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return candidate,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("cannot create {}: {error}", candidate.display()),
        }
    }
    panic!("no free directory under {}", base.display());
}

/// A store directory for one process to use.
///
/// **Every process this file starts names one.** A test that ran the default
/// would read — and `sure check --goal`, and `sure hook ingest`, would *write* —
/// `%LOCALAPPDATA%\SURE\sure.db`, the store of the person running the suite: an
/// invented requirement in somebody's history, and a test result that depends on
/// what is in it. `--store-dir` is the mechanism, and it is a value in the
/// child's own argument vector, so nothing in the project a test points SURE at
/// can change where the row goes.
///
/// The directory is created and left empty: a store SURE has never written looks
/// like exactly that, and a test that wants one *written* says so itself.
///
/// It sits beside [`a_project_of_our_own`] rather than inside it, and that is not
/// a coincidence: a store inside the project it is recording about is refused by
/// `Paths::ensure_outside`, so nesting the two would make these tests exercise a
/// refusal instead of a redirect.
fn a_store_of_our_own() -> PathBuf {
    a_directory_of_our_own("store")
}

/// A project for a check to be about, with something in it to read.
///
/// Under the same `target/tmp` root as the stores and never the repository
/// itself: these tests write nothing into the project, but a test that pointed
/// SURE at this repository would be checking the tree its own results come from.
fn a_project_of_our_own() -> PathBuf {
    let project = a_directory_of_our_own("project");
    std::fs::write(project.join("README.md"), "# a small project\n")
        .unwrap_or_else(|error| panic!("cannot write into {}: {error}", project.display()));
    project
}

/// The same, with the store named by the caller of this function.
///
/// This is how every process in this file is started, apart from the one that
/// deliberately names no store. A spawn that went straight to `Command::new(SURE)`
/// would be a spawn with the machine's own store.
fn sure_in_a_store(store: &Path) -> Command {
    let mut command = Command::new(SURE);
    command.arg("--store-dir").arg(store);
    // No test wants a child of this file waiting on this process's standard
    // input; the one test that pipes an event in says so itself.
    command.stdin(Stdio::null());
    command
}

/// The same, with no store named at all, so that the default is what runs.
///
/// One test uses this, the one that checks a caller who names nothing still gets
/// the platform's own location. It reads the store on this machine rather than
/// writing to it — `sure doctor` has its own tests for not creating a store —
/// and it takes the store's bytes before and after, so a diagnostic that had
/// started writing would fail here rather than quietly edit somebody's history.
fn sure_without_a_store() -> Command {
    let mut command = Command::new(SURE);
    command.stdin(Stdio::null());
    command
}

/// Run the binary in a store of this test's own, and collect what it did.
fn run(args: &[&str]) -> Run {
    run_in_a_store(&a_store_of_our_own(), args)
}

/// The same, in a store the caller named.
fn run_in_a_store(store: &Path, args: &[&str]) -> Run {
    let mut command = sure_in_a_store(store);
    command.args(args);
    Run::of(
        &command
            .output()
            .unwrap_or_else(|error| panic!("could not run {SURE}: {error}")),
    )
}

/// The store the person running the suite really uses: its path, and its bytes.
///
/// Read through the same `Paths::discover` a command that names no store uses,
/// so this is exactly the file a spawn that forgot `--store-dir` would write.
/// `bytes` is `None` on a machine that has no store yet, which the comparison
/// below understands rather than treats as a failure.
struct MachineStore {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

fn the_store_on_this_machine() -> MachineStore {
    let path = Paths::discover()
        .expect("this machine reports a per-user location for SURE's files")
        .store_file();
    let bytes = std::fs::read(&path).ok();
    MachineStore { path, bytes }
}

/// Assert that nothing since `before` touched the store on this machine.
///
/// What it can see: this test, and every process this file starts. What it
/// cannot see: a write from a test binary that has not run yet, or from a `sure`
/// process somebody started by hand. A full `cargo test --workspace` covers the
/// first and a digest of this file taken before and after that run covers both —
/// this is the assertion that catches the inside of one binary.
fn assert_untouched(before: &MachineStore, when: &str) {
    let now = the_store_on_this_machine();
    assert_eq!(
        now.path,
        before.path,
        "the store SURE uses moved {when}: {} rather than {}",
        now.path.display(),
        before.path.display()
    );
    match (&before.bytes, &now.bytes) {
        (None, None) => {}
        (Some(was), Some(is)) => assert!(
            was == is,
            "{} was {} bytes {when} and is {} bytes now, so something in this suite wrote the \
             store of the person running it. Every process this file starts names a store with \
             `--store-dir`, and a named store is not this file: if one of them reached it, the \
             mechanism is not in force. If they all named one, the writer came from outside \
             this file — another test binary, a `sure` process somebody started, or a harness \
             hook — and the digest of this file taken across the whole workspace run is what \
             shows it.",
            before.path.display(),
            was.len(),
            is.len()
        ),
        (None, Some(_)) => panic!(
            "{} did not exist {when} and exists now, so something created the store of the \
             person running this suite",
            before.path.display()
        ),
        (Some(_), None) => panic!("{} existed {when} and is gone now", before.path.display()),
    }
}

/// Every command `docs/architecture/CLI.md` lists, with arguments that reach it.
///
/// A command that needs a subcommand is given one, so that this list tests the
/// command rather than the grammar's refusal to run half of one — and a
/// subcommand that needs an argument is given one for the same reason.
/// `sure history delete` takes a scope and nothing else, and the scope here is
/// `--all` against a store this file named and never wrote, so the invocation is
/// destructive in shape and inert in fact.
const EVERY_COMMAND: &[&[&str]] = &[
    &["check"],
    &["recheck"],
    &["repair"],
    &["history"],
    &["history", "list"],
    &["history", "list", "--limit", "1"],
    &["history", "show", "no-such-session"],
    &["history", "delete", "--all"],
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

/// The commands this build cannot carry out.
///
/// Every other command in [`EVERY_COMMAND`] runs, and what it answers is checked
/// by a test of its own. What this list is *for* is the status: a command here
/// must exit 3 and nothing else, and a command not here must never exit 3 —
/// which is what stops a command quietly going back to refusing, and what stops
/// a command that cannot run being reported as one that did.
///
/// `sure history list` and `sure history delete` were here, and are not any
/// more: the inspect-and-delete surface is what a user needs most from a
/// history, and it is now carried out. `sure history export` is the one
/// subcommand of that surface still refused, and the reason is in
/// `crate::history`.
const REFUSED: &[&[&str]] = &[
    &["history", "export"],
    &["explain"],
    &["config"],
    &["config", "paths"],
    &["config", "show"],
    &["config", "validate"],
];

/// The commands whose answer is the same on every machine, and so is a status
/// this file can demand.
///
/// `doctor` and `history` are not, and that is the point of both: `doctor`
/// reports on the machine it runs on, and a machine where SURE found something
/// wrong about its own files earns status 1; `history` reports on what that
/// machine has recorded, and a machine with nothing recorded is a different
/// answer from one with sessions. Demanding 0 here would turn this file into a
/// claim about the machine running it, which is a claim about a machine and not
/// about SURE.
const ALWAYS_OK: &[&[&str]] = &[&["protocol"], &["version"]];

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
fn no_command_this_build_cannot_carry_out_reports_success() {
    // The criterion this file exists for. Every command here is recognised and
    // cannot do its job yet, so every one of them must say so with the status
    // that means "this build cannot carry that out" — never 0, and never a
    // status that means something else. A script that ran `sure config validate`
    // in CI today must fail the build, not pass it.
    for args in REFUSED {
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
fn every_command_this_build_carries_out_runs_rather_than_refusing() {
    // The other half of the same criterion, and the half that did not exist
    // while almost nothing ran: a command that is **not** on the refused list
    // must not answer with a refusal. Without this the list above could grow a
    // name it has no business holding — a command could go back to saying "this
    // build cannot carry that out" and the test above would still pass, because
    // it only ever looks at the names it was given.
    //
    // Status 3 and the refusal's sentence are checked separately, because they
    // are two ways a user learns the same wrong thing: one from `$?` in a
    // script, one from the terminal.
    for args in EVERY_COMMAND {
        if REFUSED.contains(args) {
            continue;
        }
        let run = run(args);
        assert_ne!(
            run.status,
            3,
            "`sure {}` returned 3, which says this build cannot carry it out:\n{}",
            args.join(" "),
            run.stderr
        );
        assert!(
            !run.stderr.contains("is not implemented in this build"),
            "`sure {}` is worded as a command this build lacks:\n{}",
            args.join(" "),
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
    // The half of the goal's process-level coverage that writes nothing; the
    // half that writes is `a_named_store_directory_is_the_one_a_real_run_writes_to`
    // below. A goal with no words in it is refused before SURE looks for its
    // store at all, so the store this run was given stays as empty as it was —
    // which is the assertion at the end, and it is the one that says the refusal
    // happened before the row rather than after it.
    //
    // Status 5, and the two statuses it is not: 2 would mean the parser rejected
    // the command line, and `--goal ""` is accepted — an empty goal is a goal
    // with nothing in it, which is a thing a user can type. 3 would mean this
    // build cannot record a goal, and it can. 5 is "it tried and did not finish",
    // which is what happened.
    let store = a_store_of_our_own();
    for args in [
        &["check", "--goal", ""][..],
        &["check", "--goal", "   "][..],
    ] {
        let human = run_in_a_store(&store, args);
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
        let machine = run_in_a_store(&store, &machine);
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

    // And the store this run was given is still empty: a goal with no words in
    // it is refused before SURE opens anything, so there is no database to clean
    // up and nothing a later run would find.
    assert!(
        !store.join("sure.db").exists(),
        "a goal with no words in it created the store SURE was told to use"
    );
    assert!(
        !store.join("sure.db-wal").exists() && !store.join("sure.db-shm").exists(),
        "a goal with no words in it left a journal beside where the store would go"
    );
}

// --- the history: inspecting it, and being rid of it --------------------

/// The store file a run given this directory writes.
fn store_file(store: &Path) -> PathBuf {
    store.join("sure.db")
}

/// `sure history`, as a script reads it.
///
/// The frame's `details`, which is where every field below is read from. The
/// run is asserted to have answered rather than refused first, because a
/// refusal's `details` is not a history and a test that read one would be
/// asserting about the wrong object.
fn history_frame(store: &Path, args: &[&str]) -> serde_json::Value {
    let mut full: Vec<&str> = vec!["--format", "json", "history"];
    full.extend_from_slice(args);
    let run = run_in_a_store(store, &full);
    assert_ne!(
        run.status,
        3,
        "`sure history {}` says this build cannot carry it out:\n{}",
        args.join(" "),
        run.stderr
    );
    let frame: serde_json::Value =
        serde_json::from_str(run.stdout.trim()).unwrap_or_else(|error| {
            panic!(
                "`sure --format json history {}` is not one frame: {error}\nstdout:\n{}",
                args.join(" "),
                run.stdout
            )
        });
    assert_eq!(
        frame["exit_code"].as_i64(),
        Some(i64::from(run.status)),
        "the frame and the process disagree about the same run: {frame}"
    );
    frame
}

/// Record one real event with `sure hook ingest`, in a session of its own.
///
/// The binary rather than a row written here: what the history has to be able to
/// read is what SURE actually wrote, and an event this test inserted by hand
/// would be the test's idea of an event.
///
/// `afterFileEdit` is not a `pre-tool-use` event, so the run records the event
/// and allows — no protection decision, and therefore no second thing this
/// helper's callers have to think about.
fn record_one_event(store: &Path, project: &Path, harness_session: &str) {
    let payload = serde_json::json!({
        "event": "afterFileEdit",
        "harness_session_id": harness_session,
        "project_root": project.to_str().expect("this test's paths are utf-8"),
        "timestamp_utc": "2026-09-19T12:01:00Z",
        "source": "cursor",
        "path": "src/lib.rs",
    })
    .to_string();

    let mut command = sure_in_a_store(store);
    command.args(["hook", "ingest", "--source", "cursor"]);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("a child process");
    let mut stdin = child.stdin.take().expect("the pipe");
    stdin.write_all(payload.as_bytes()).expect("write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("the child exits");
    assert_eq!(
        output.status.code(),
        Some(0),
        "`sure hook ingest` did not record the event, so there is no history to test:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The store, opened by this process.
///
/// Read back rather than taken on trust: `sure history` says what it removed,
/// and a test that believed it would be checking a report against itself. What
/// the assertions below read is the file the child process wrote.
fn open_the_store(store: &Path) -> Store {
    let file = store_file(store);
    Store::open_at(&file).unwrap_or_else(|error| panic!("cannot open {}: {error}", file.display()))
}

#[test]
fn a_history_with_nothing_in_it_says_so_rather_than_printing_an_empty_table() {
    // The first thing a user does after reading that SURE keeps a history is
    // look at it, and on a machine where nothing has been recorded that must be
    // an answer rather than a blank page. "Nothing was recorded" and "SURE could
    // not tell you" have to be different answers, and the second one is a
    // failure with a status.
    let store = a_store_of_our_own();
    let machine = the_store_on_this_machine();

    let human = run_in_a_store(&store, &["history"]);
    assert_eq!(
        human.status, 0,
        "`sure history` on a machine with nothing recorded exited {}. An empty history is an \
         answer, not a failure:\nstdout: {}\nstderr: {}",
        human.status, human.stdout, human.stderr
    );
    assert!(
        human.stderr.is_empty(),
        "the answer went to standard error, so `sure history > history.txt` would leave the \
         file empty:\n{}",
        human.stderr
    );
    assert!(
        human
            .stdout
            .contains("SURE has recorded nothing on this machine yet."),
        "the empty history does not say it is empty:\n{}",
        human.stdout
    );
    assert!(
        human
            .stdout
            .contains(&store_file(&store).display().to_string()),
        "the empty history does not say which store it looked in, so a user who named one \
         cannot tell whether it was used:\n{}",
        human.stdout
    );
    assert!(
        !human.stdout.contains("kept until"),
        "an empty history printed the fields of a session that is not there:\n{}",
        human.stdout
    );

    let frame = history_frame(&store, &[]);
    assert_eq!(frame["outcome"].as_str(), Some("ok"), "{frame}");
    assert_eq!(frame["details"]["total"].as_u64(), Some(0), "{frame}");
    assert_eq!(
        frame["details"]["sessions"],
        serde_json::json!([]),
        "{frame}"
    );
    assert_eq!(
        frame["details"]["store_present"].as_bool(),
        Some(false),
        "a store that is not there was reported as present, or the other way round: {frame}"
    );

    // And it did not create the store it was reporting on. This is the assertion
    // that makes the empty answer worth anything: a command that reported an
    // empty history by making one would have changed the thing it was reporting,
    // and the file it left would be the evidence of a run that claimed to find
    // nothing.
    assert!(
        !store_file(&store).exists(),
        "`sure history` created {} in a store directory it was asked to report on",
        store_file(&store).display()
    );
    assert!(
        !store.join("sure.db-wal").exists() && !store.join("sure.db-shm").exists(),
        "`sure history` left a journal beside where the store would go"
    );
    assert_untouched(&machine, "by a history that found nothing");
}

#[test]
fn a_recorded_session_is_visible_with_its_project_its_harness_and_how_long_it_is_kept() {
    // The inspect half of the acceptance criterion, at the process. Before this
    // task there was no way for a user to see that anything had been recorded at
    // all: the store is a SQLite file in a per-user directory, and both harness
    // integrations told people to run exactly this command.
    let store = a_store_of_our_own();
    let project = a_project_of_our_own();
    let machine = the_store_on_this_machine();
    record_one_event(&store, &project, "cursor-session-inspect");

    let human = run_in_a_store(&store, &["history"]);
    assert_eq!(human.status, 0, "{}", human.stderr);
    for field in [
        "session",
        "project",
        "harness",
        "started",
        "kept for",
        "kept until",
    ] {
        assert!(
            human.stdout.contains(field),
            "the listing does not say `{field}` for the one session that is there:\n{}",
            human.stdout
        );
    }
    assert!(
        human.stdout.contains(&project.display().to_string()),
        "the listing does not say which project the session was recorded against:\n{}",
        human.stdout
    );
    assert!(
        human.stdout.contains("cursor"),
        "the listing does not say which harness sent the events:\n{}",
        human.stdout
    );
    // The sentence that stops a deadline being read as a deletion. Nothing in
    // this build removes a session because a date passed, and a listing that
    // printed a date without saying so would leave a user believing their
    // history was being cleaned up when it was not.
    assert!(
        human.stdout.contains("not a job it runs"),
        "the listing prints a `kept until` date without saying that nothing acts on it:\n{}",
        human.stdout
    );

    let frame = history_frame(&store, &[]);
    assert_eq!(frame["details"]["total"].as_u64(), Some(1), "{frame}");
    let session = &frame["details"]["sessions"][0];
    assert_eq!(
        session["project_root"].as_str(),
        project.to_str(),
        "{frame}"
    );
    assert_eq!(session["harness"].as_str(), Some("cursor"), "{frame}");
    assert_eq!(
        session["harness_session_id"].as_str(),
        Some("cursor-session-inspect"),
        "{frame}"
    );
    // The retention in force for the session row, which is the session event
    // store's own and is not the setting this task made configurable: a session
    // and its events are kept for `DEFAULT_SESSION_RETENTION_DAYS` whatever
    // `privacy.full_recording_retention_days` says, because that setting is
    // about the *recording*, which is a different row. Read from the frame and
    // compared with the constant rather than with a 90 written here: the number
    // is allowed to change, and a test that pinned it would have to be edited by
    // whoever changed it rather than telling them.
    assert_eq!(
        session["retention_days"].as_i64(),
        Some(sure_core::session_event_store::DEFAULT_SESSION_RETENTION_DAYS),
        "the session was recorded with a retention that is not this build's default: {frame}"
    );
    assert!(
        session["retained_until_ms"]
            .as_i64()
            .is_some_and(|until| until > 0),
        "the session carries no retention deadline: {frame}"
    );

    // One session, and the events recorded in it.
    let id = session["sure_session_id"]
        .as_str()
        .expect("a session carries SURE's own identifier")
        .to_owned();
    let shown = history_frame(&store, &["show", &id]);
    assert_eq!(
        shown["details"]["session"]["sure_session_id"].as_str(),
        Some(id.as_str()),
        "{shown}"
    );
    let events = shown["details"]["events"]
        .as_array()
        .expect("a shown session lists its events");
    assert_eq!(events.len(), 1, "{shown}");
    assert_eq!(
        events[0]["event_type"].as_str(),
        Some("file.edited"),
        "{shown}"
    );
    assert!(
        events[0]["record"]["id"].as_i64().is_some(),
        "the event does not say which record it wrote, so a user cannot look one up: {shown}"
    );

    // An id that is not there is a run that did not finish — not a refusal, and
    // not an empty listing under a name that promised one session.
    let missing = run_in_a_store(&store, &["history", "show", "no-such-session"]);
    assert_eq!(
        missing.status, 5,
        "`sure history show <id that is not there>` returned {}. 3 would say this build cannot \
         show a session, and it can; 0 would be an answer to a question nobody answered:\n\
         stdout: {}\nstderr: {}",
        missing.status, missing.stdout, missing.stderr
    );
    assert!(
        missing.stdout.is_empty(),
        "a run with no answer wrote to standard output:\n{}",
        missing.stdout
    );
    assert!(
        missing.stderr.contains("could not finish"),
        "the complaint does not say the run did not finish:\n{}",
        missing.stderr
    );
    assert_untouched(&machine, "by a history that found one session");
}

#[test]
fn a_delete_removes_the_session_its_events_and_its_records_and_says_how_many() {
    // The delete half of the acceptance criterion, and the half that has to be
    // checked against the file rather than against the sentence: a delete that
    // reached `records` and stopped would leave the session and its events in
    // `sessions` and `session_events`, and the report would still say a number.
    //
    // Two projects, because the scope has to mean something: a delete scoped to
    // one of them may not touch the other, and the only way to check that is for
    // there to be another one.
    let store = a_store_of_our_own();
    let kept = a_project_of_our_own();
    let doomed = a_project_of_our_own();
    let machine = the_store_on_this_machine();
    record_one_event(&store, &kept, "keep-me");
    record_one_event(&store, &doomed, "delete-me");

    assert_eq!(
        history_frame(&store, &[])["details"]["total"].as_u64(),
        Some(2),
        "two ingests did not produce two sessions, so this test would prove nothing"
    );

    // The rows the delete has to reach, read from the store by this process, so
    // that what is asserted afterwards is the absence of *those* rows rather
    // than a number the command printed.
    let (doomed_session, doomed_event) = {
        let opened = open_the_store(&store);
        let sessions = SessionEventStore::new(&opened);
        let all = sessions.sessions(10).expect("the sessions this test wrote");
        let doomed_session = all
            .iter()
            .find(|session| session.project_root == doomed.display().to_string())
            .unwrap_or_else(|| panic!("no session for {}: {all:?}", doomed.display()))
            .clone();
        let events = sessions
            .events_for_session(doomed_session.row_id)
            .expect("the events of a session this test wrote");
        assert_eq!(events.len(), 1, "the ingest did not write one event");
        (doomed_session, events[0].clone())
    };
    let doomed_id = doomed_session.sure_session_id.as_str().to_owned();
    let record_row = doomed_event
        .record_row_id
        .expect("every event in this build owns a record");

    let deleted = history_frame(&store, &["delete", "--session", &doomed_id]);
    assert_eq!(
        deleted["details"]["removed"],
        serde_json::json!(true),
        "{deleted}"
    );
    assert_eq!(
        deleted["details"]["deleted"]["sessions"].as_u64(),
        Some(1),
        "{deleted}"
    );
    assert_eq!(
        deleted["details"]["deleted"]["events"].as_u64(),
        Some(1),
        "{deleted}"
    );
    assert_eq!(
        deleted["details"]["deleted"]["records"].as_u64(),
        Some(1),
        "the delete reported removing a session and left the record it wrote: {deleted}"
    );
    assert_eq!(
        deleted["details"]["deleted"]["recordings"].as_u64(),
        Some(0),
        "there was no full recording to remove: {deleted}"
    );

    // And the rows really went.
    {
        let opened = open_the_store(&store);
        let sessions = SessionEventStore::new(&opened);
        assert!(
            sessions
                .session_by_sure_id(&doomed_id)
                .expect("a lookup")
                .is_none(),
            "the session is still in `sessions` after a delete that reported removing it"
        );
        assert!(
            sessions
                .events_for_session(doomed_session.row_id)
                .expect("a lookup")
                .is_empty(),
            "the session's events are still in `session_events`"
        );
        assert!(
            opened.record(record_row).expect("a lookup").is_none(),
            "the record the event wrote is still in `records`"
        );
        assert_eq!(
            sessions.session_count().expect("a count"),
            1,
            "the delete removed more than the scope it was given"
        );
    }

    // The other project's session was not in the scope, and the second scope
    // proves it is still there by removing it.
    let remaining = history_frame(&store, &[]);
    assert_eq!(
        remaining["details"]["sessions"][0]["project_root"].as_str(),
        kept.to_str(),
        "the survivor is not the session the delete was not asked about: {remaining}"
    );
    let kept_root = kept.display().to_string();
    let second = history_frame(&store, &["delete", "--project", &kept_root]);
    assert_eq!(
        second["details"]["deleted"]["sessions"].as_u64(),
        Some(1),
        "{second}"
    );
    assert_eq!(
        history_frame(&store, &[])["details"]["total"].as_u64(),
        Some(0),
        "a delete scoped to a project left a session behind"
    );

    // A scope that matches nothing says it matched nothing, in those words and
    // in the frame, rather than reporting a deletion that did not happen.
    let nothing = run_in_a_store(&store, &["history", "delete", "--all"]);
    assert_eq!(nothing.status, 0, "{}", nothing.stderr);
    assert!(
        nothing.stdout.contains("Nothing was deleted."),
        "a delete that matched nothing did not say so:\n{}",
        nothing.stdout
    );
    let again = history_frame(&store, &["delete", "--all"]);
    assert_eq!(
        again["details"]["removed"],
        serde_json::json!(false),
        "{again}"
    );
    assert_eq!(
        again["details"]["deleted"]["sessions"].as_u64(),
        Some(0),
        "{again}"
    );
    assert_untouched(&machine, "by a delete that named a store");
}

#[test]
fn a_delete_reaches_a_full_recording_that_its_session_event_wrote() {
    // A full recording is a **second** `records` row, written beside the one the
    // event owns, and nothing in the schema links the two: `session_events`
    // holds the id of the record the event wrote, and the recording carries its
    // event's id inside its JSON document. `PRAGMA foreign_keys` is not set
    // anywhere in this workspace, so the `REFERENCES` clauses in the migrations
    // are documentation rather than enforcement and there is no cascade to
    // inherit. A delete that only walked the columns would therefore leave every
    // full recording behind — the one thing a user deleting their history most
    // wants gone.
    //
    // The rows are written here through the same functions `sure hook ingest`
    // calls, with one `EventId` shared by both writes, because that sharing is
    // the whole of the link and building it by hand is the only way to be sure
    // the test is about it.
    let store_dir = a_store_of_our_own();
    let project = a_project_of_our_own();
    let project_root = project.to_str().expect("this test's paths are utf-8");

    let payload = serde_json::json!({
        "event": "afterFileEdit",
        "harness_session_id": "cursor-session-recording",
        "project_root": project_root,
        "timestamp_utc": "2026-09-19T12:01:00Z",
        "source": "cursor",
        "path": "src/lib.rs",
    })
    .to_string();
    let envelope = sure_core::normalizer::cursor::normalize(&payload).expect("a cursor event");
    let ingested = sure_core::harness_event::ingest_event_str(
        &envelope
            .to_json()
            .expect("an envelope this build can serialise"),
    )
    .expect("a valid event");

    let store = Store::open_at(&store_file(&store_dir)).expect("a store this test made");
    let fingerprint = sure_core::fingerprint::project_fingerprint(
        &project,
        &sure_core::fingerprint::FingerprintOptions::default(),
    )
    .expect("a project this build can fingerprint")
    .id;
    let event_id = sure_core::ids::EventId::generate();
    SessionEventStore::new(&store)
        .persist(&ingested, project_root, &fingerprint, &event_id)
        .expect("an event");
    let recording_row = sure_core::full_recording::persist_full_recording(
        &store,
        &ingested,
        &event_id,
        project_root,
        &fingerprint,
        sure_core::full_recording::FullRecordingConsent::Full,
        1,
    )
    .expect("a full recording");
    let sure_session_id = SessionEventStore::new(&store)
        .sessions(10)
        .expect("the session this test wrote")[0]
        .sure_session_id
        .as_str()
        .to_owned();
    assert!(
        store.record(recording_row).expect("a lookup").is_some(),
        "this test did not manage to write a full recording, so it would prove nothing"
    );

    // The recording's own deadline is in the surface the user inspects, because
    // `records` has no retention column and the document is where a recording
    // says how long it is kept.
    let shown = history_frame(&store_dir, &["show", &sure_session_id]);
    let record = &shown["details"]["events"][0]["record"];
    assert_eq!(record["kind"].as_str(), Some("event"), "{shown}");
    assert!(
        record["retained_until_ms"].is_null(),
        "the record the event owns is not the recording: {shown}"
    );

    let deleted = history_frame(&store_dir, &["delete", "--all"]);
    assert_eq!(
        deleted["details"]["deleted"]["recordings"].as_u64(),
        Some(1),
        "the delete did not reach the full recording: {deleted}"
    );
    assert_eq!(
        deleted["details"]["deleted"]["records"].as_u64(),
        Some(1),
        "the delete did not reach the record the event wrote: {deleted}"
    );
    assert!(
        store.record(recording_row).expect("a lookup").is_none(),
        "the full recording survived a delete that reported removing it"
    );
}

#[test]
fn a_project_that_shortens_the_recording_retention_is_what_the_recording_says() {
    // Criterion: "Full recording retention configurable" — at the process rather
    // than at the function. The tests beside `persist_full_recording` prove that
    // the number handed in is the number written down; what they cannot prove is
    // that `sure hook ingest` hands in the *arbitrated* number instead of the
    // default, and that seam is where a regression would really happen, because
    // the hook is the only writer of recordings in this build.
    //
    // The project names **zero** days, and that is what makes this the same test
    // on every machine. A project may only shorten, and it is measured against
    // the user's own settings file at the platform's real config path — a file no
    // test can redirect and no test may write. Zero is shorter than, or equal to,
    // whatever that file says, so the resolved number is zero either way: named
    // by the project when the user allowed more, named by the user when the user's
    // file says zero too. A test naming one day would pass on a machine whose
    // user file says nothing and fail on one whose user file says zero.
    let store = a_store_of_our_own();
    let project = a_project_of_our_own();
    std::fs::write(
        project.join("sure.yaml"),
        "privacy:\n  full_recording: true\n  full_recording_retention_days: 0\n",
    )
    .expect("a project file this test wrote");
    record_one_event(&store, &project, "cursor-session-retention");

    let store = open_the_store(&store);
    let recordings = store
        .history(
            &HistoryFilter {
                project_fingerprint: None,
                kind: Some(RecordKind::Recording),
                include_recordings: true,
            },
            10,
        )
        .expect("the recording the hook was asked to write");
    assert_eq!(
        recordings.len(),
        1,
        "the hook did not write the full recording this test is about, so it would prove nothing"
    );
    let until = recordings[0].document["retained_until_ms"]
        .as_i64()
        .expect("a recording says when it is kept until");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("a clock after 1970")
        .as_millis() as i64;
    let a_minute = 60_000;
    assert!(
        (until - now).abs() <= a_minute,
        "a recording asked to be kept for no time at all is kept until {until}, which is {} ms \
         from now",
        until - now
    );
    assert!(
        until < now + DEFAULT_FULL_RECORDING_RETENTION_DAYS * 86_400_000 - a_minute,
        "the project's number was ignored and the default was written instead"
    );
}

#[test]
fn a_delete_without_a_scope_or_with_two_is_a_wrong_command_line() {
    // The non-interactive decision, checked where a user meets it. Nothing
    // prompts, so the scope on the command line *is* the consent — which means a
    // command line that names no scope, or names two, has to be refused before
    // anything runs. It is a wrong command line (2) and not a run that failed
    // (5): SURE never looked at the store, so there is no run to describe.
    let store = a_store_of_our_own();
    let project = a_project_of_our_own();
    record_one_event(&store, &project, "cursor-session-scope");

    let before = history_frame(&store, &[]);
    assert_eq!(before["details"]["total"].as_u64(), Some(1));

    for args in [
        &["history", "delete"][..],
        &["history", "delete", "--all", "--session", "x"][..],
        &["history", "delete", "--session", "x", "--project", "y"][..],
    ] {
        let run = run_in_a_store(&store, args);
        assert_eq!(
            run.status,
            2,
            "`sure {}` returned {}. A delete that names no scope, or two, is a command line \
             SURE cannot act on:\n{}",
            args.join(" "),
            run.status,
            run.stderr
        );
        assert!(
            run.stdout.is_empty(),
            "a refused command line wrote to standard output:\n{}",
            run.stdout
        );
    }

    // Nothing went. A wrong command line that had deleted anyway would be the
    // worst of both: a user told their command was refused, and a history that
    // had lost a session.
    let after = history_frame(&store, &[]);
    assert_eq!(
        after["details"]["total"].as_u64(),
        Some(1),
        "a refused delete removed a session anyway"
    );
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
fn a_run_says_which_privacy_mode_it_was_under_and_what_it_did_about_models() {
    // P13-T002, at the process: the two things a user could not find out before
    // it. Which privacy mode is running, and whether a model was consulted.
    //
    // Everything asserted here is a *membership* in a closed set rather than a
    // value, and that is deliberate. A real process reads the user's own settings
    // file outside the project, and that directory cannot be moved by a flag or
    // an environment variable, so a test demanding `local_first` here would be a
    // claim about the machine it ran on. The values themselves — including the
    // arbitration between the project's file and the user's — are tested where
    // they can be set: `crates/sure-cli/src/check.rs` and
    // `crates/sure-core/src/privacy.rs`.
    let human = run(&["check"]);
    let machine = run(&["--format", "json", "check"]);
    let frame: serde_json::Value =
        serde_json::from_str(machine.stdout.trim()).expect("one frame on one line");

    let mode = frame["details"]["privacy"]["mode"]
        .as_str()
        .unwrap_or_else(|| panic!("the frame does not say which mode was in effect: {frame}"));
    assert!(
        ["local_first", "fully_local"].contains(&mode),
        "the run reports a mode this release does not implement: {mode}"
    );
    assert_ne!(
        mode, "cloud_enhanced",
        "a mode that is refused at the settings file was reported as in effect"
    );
    assert!(
        frame["details"]["privacy"]["mode_set_by"].is_null()
            || ["user", "project"].contains(
                &frame["details"]["privacy"]["mode_set_by"]
                    .as_str()
                    .unwrap_or("")
            ),
        "the frame names a layer that cannot have set anything: {frame}"
    );
    let provider = frame["details"]["privacy"]["analysis_provider"]
        .as_str()
        .unwrap_or_else(|| panic!("the frame does not name a provider: {frame}"));
    assert!(
        [
            "disabled",
            "local_command",
            "claude_cli",
            "openai_compatible"
        ]
        .contains(&provider),
        "the frame names a provider this release does not have: {provider}"
    );

    // The human form says the same thing, in a sentence: a person who never
    // passes `--format json` is the one who most needs to know.
    assert!(
        human.stdout.contains("Privacy and model use"),
        "the report has no privacy section:\n{}",
        human.stdout
    );
    assert!(
        human.stdout.contains(&format!("Mode in effect: {mode}")),
        "the two renderings disagree about the mode in effect ({mode}):\n{}",
        human.stdout
    );
    assert!(
        human
            .stdout
            .contains(&format!("Analysis provider: {provider}")),
        "the two renderings disagree about the provider ({provider}):\n{}",
        human.stdout
    );

    // And the model question is answered rather than omitted. This build has no
    // check that asks for model-backed analysis, so `consulted` cannot be the
    // answer — the honest one is that no model was consulted, or that this run
    // cannot say. Silence is the state this test exists to rule out.
    let state = frame["details"]["model_use"]["state"]
        .as_str()
        .unwrap_or_else(|| panic!("the frame does not say what happened about models: {frame}"));
    assert!(
        [
            "no_provider",
            "nothing_asked",
            "provider_unusable",
            "consulted",
            "cannot_confirm"
        ]
        .contains(&state),
        "the frame names a model state this build does not have: {state}"
    );
    assert_ne!(
        state, "consulted",
        "no check in this build asks for model-backed analysis, so no run can report one"
    );
    assert!(
        human.stdout.contains("No model was consulted")
            || human
                .stdout
                .contains("cannot say whether a model was consulted"),
        "the report neither states that no model was consulted nor that it cannot say:\n{}",
        human.stdout
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
    //
    // This is the test the store's location was made choosable for. With no
    // store named, this run opened and wrote the store of whoever ran the suite
    // — measured on 2026-09-18 as 4096 bytes and one `event` row for the session
    // named below. It names a store now, and the machine's own store is
    // compared before and after, so a build where the flag stopped reaching
    // this command fails here instead of adding to somebody's history.
    let store = a_store_of_our_own();
    let machine = the_store_on_this_machine();

    let payload = r#"{
        "event": "preToolUse",
        "harness_session_id": "cursor-session-001",
        "tool": "Shell",
        "args": {"command": "npm test", "workdir": "C:\\Users\\dev\\sample-project"},
        "timestamp_utc": "2026-09-18T12:01:00Z",
        "source": "cursor"
    }"#;

    let mut command = sure_in_a_store(&store);
    command.args([
        "--format",
        "json",
        "hook",
        "ingest",
        "--source",
        "cursor",
        "pre-tool-use",
    ]);
    let mut child = command
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

    // The event really was persisted, and to the store the caller named: this
    // is the half that says the redirect is a redirect rather than a run that
    // silently stored nothing.
    assert!(
        store.join("sure.db").is_file(),
        "`sure hook ingest --store-dir {}` wrote no store, so the event it read was not kept \
         where it was told to keep it",
        store.display()
    );
    assert_untouched(&machine, "by an ingest that named a store");
}

/// One `preToolUse` read, from Cursor, aimed at `path` in `project`, run in
/// `store`.
///
/// The payload is what a harness sends, and it is built with `serde_json` rather
/// than with a `format!` string because the one field that is a Windows path
/// would otherwise have to be escaped by hand — and a path escaped wrongly is a
/// test about a project that does not exist.
fn ingest_a_read(store: &Path, project: &Path, path: &str) -> Run {
    let payload = serde_json::json!({
        "event": "preToolUse",
        "harness_session_id": "p13t004-mode",
        "project_root": project.to_string_lossy(),
        "tool": "Read",
        "path": path,
        "timestamp_utc": "2026-09-19T09:00:00Z",
        "source": "cursor",
    })
    .to_string();

    let mut command = sure_in_a_store(store);
    command.args([
        "--format",
        "json",
        "hook",
        "ingest",
        "--source",
        "cursor",
        "pre-tool-use",
    ]);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("a child process");
    let mut stdin = child.stdin.take().expect("the pipe");
    stdin.write_all(payload.as_bytes()).expect("write to stdin");
    drop(stdin);
    Run::of(&child.wait_with_output().expect("the child exits"))
}

#[test]
fn the_protection_mode_a_project_names_is_the_one_sure_decides_under() {
    // P13-T004, at the process level and through the command a harness runs:
    // one read, run twice, differing only in the mode the project's own
    // `sure.yaml` names. `docs/security/PROTECTION_MODE.md` makes secret/config
    // areas strict's, so a read of `.env` is allowed under standard and held
    // under strict, and the held one leaves the process with status 1 — which is
    // what a launcher reads, and what this test is really about.
    //
    // Both runs name a store of this test's own, and the machine's store is
    // compared before and after.
    //
    // The mode is resolved from two layers and the other one is the user's own
    // settings file, which no test may write. Rather than assume it holds
    // nothing, the precondition is stated: if this machine has one, the runs
    // below are not the experiment they say they are.
    let user_settings = Paths::discover()
        .expect("this machine reports a per-user location for SURE's files")
        .user_config_file();
    assert!(
        !user_settings.is_file(),
        "this machine has a user settings file at {}, and a protection mode in it would be \
         answering instead of the project's own file. These runs are about the project layer: on a \
         machine with user settings, read what they hold before believing the standard case below.",
        user_settings.display()
    );

    let machine = the_store_on_this_machine();
    let store = a_store_of_our_own();

    // Standard: the read goes through, and the answer says so out loud.
    let standard = a_project_of_our_own();
    std::fs::write(
        standard.join("sure.yaml"),
        "protection:\n  mode: standard\n",
    )
    .expect("write the project's settings");
    let run = ingest_a_read(&store, &standard, ".env");
    assert!(
        run.succeeded(),
        "standard lets this read through, so the process exits 0: status {} and output {}",
        run.status,
        run.stdout
    );
    let frame: serde_json::Value = serde_json::from_str(run.stdout.trim())
        .unwrap_or_else(|error| panic!("hook ingest is not valid JSON: {error}\n{}", run.stdout));
    assert_eq!(frame["decision"].as_str(), Some("allow"), "{frame}");
    assert!(
        frame["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "an allow the rule reached carries the reason it reached it with: {frame}"
    );

    // Strict: the same request is held, with status 1 and a sentence about what
    // the file is.
    let strict = a_project_of_our_own();
    std::fs::write(strict.join("sure.yaml"), "protection:\n  mode: strict\n")
        .expect("write the project's settings");
    let run = ingest_a_read(&store, &strict, ".env");
    assert_eq!(
        run.status, 1,
        "a held request is `not_green`, which is 1: output {}",
        run.stdout
    );
    let frame: serde_json::Value = serde_json::from_str(run.stdout.trim())
        .unwrap_or_else(|error| panic!("hook ingest is not valid JSON: {error}\n{}", run.stdout));
    assert_eq!(frame["decision"].as_str(), Some("block"), "{frame}");
    let reason = frame["reason"]
        .as_str()
        .expect("a held request carries a reason");
    assert!(
        reason.contains("credentials"),
        "the reason says what the file is rather than which setting asked: {reason}"
    );
    assert!(
        !reason.contains("protection.mode"),
        "the reason is about the user's work, not policy jargon: {reason}"
    );

    // Strict's other half: it holds what the document names and nothing else, so
    // the same project lets an ordinary read through.
    let run = ingest_a_read(&store, &strict, "src/lib.rs");
    assert!(
        run.succeeded(),
        "strict adds a question about the document's categories, not about every read: status {} \
         and output {}",
        run.status,
        run.stdout
    );

    assert_untouched(&machine, "by ingests that named a store");
}

#[test]
fn a_named_store_directory_is_the_one_a_real_run_writes_to() {
    // Acceptance 1 and the writing half of acceptance 2, at the process level
    // and in the shape that can fail: a real run that writes, told to keep its
    // evidence somewhere the caller chose. If the mechanism were ignored, the
    // goal would land in the store of the person running the suite and the
    // comparison at the end would fail — which is why this test is the one the
    // `.cargo/config.toml` shape was not needed for: the flag is in the child's
    // own argument vector, so the child cannot get it from anywhere else.
    //
    // What it can see: this test and the process it starts. What it cannot see:
    // a write from a test binary that has not run yet, or from a `sure` process
    // somebody started by hand — a full `cargo test --workspace` covers the
    // first, and the digest of the store taken across that run covers both.
    let store = a_store_of_our_own();
    let project = a_project_of_our_own();
    let machine = the_store_on_this_machine();

    let goal = "make the upload reject a file over 10 MB instead of failing silently";
    let run = run_in_a_store(
        &store,
        &["check", "--goal", goal, project.to_str().expect("a path")],
    );

    // The same verdict a check of a readable project gets in this build: it ran,
    // and it cannot call the project clean. Not 0 (nothing here establishes
    // that) and not 3 (this build can check a project).
    assert_eq!(
        run.status, 1,
        "`sure check --goal …` returned {}:\nstdout:\n{}\nstderr:\n{}",
        run.status, run.stdout, run.stderr
    );
    assert!(
        run.stdout.contains("What SURE recorded"),
        "the report does not say what it wrote:\n{}",
        run.stdout
    );
    assert!(
        run.stdout.contains(goal),
        "the report does not carry the goal that was recorded:\n{}",
        run.stdout
    );

    // The row went into the store the caller named...
    let written = store.join("sure.db");
    assert!(
        written.is_file(),
        "a goal run that named {} wrote no store there",
        store.display()
    );
    // ...and the store this machine really uses is exactly as it was.
    assert_untouched(&machine, "by a check whose goal went to a named store");
}

#[test]
fn a_store_inside_the_project_is_refused_before_anything_is_recorded() {
    // Acceptance 4, at the process: a location inside the project being checked
    // is refused on the road `--store-dir` opened, in the shape the `--goal`
    // path already used. The refusal itself belongs to `Paths::ensure_outside`
    // and has its own tests in `sure-core`; what this checks is that the new
    // mechanism reaches the store through that rule rather than around it, and
    // that the process says what did not happen.
    let project = a_project_of_our_own();
    let inside = project.join(".sure-store");
    let goal = "make the upload reject a file over 10 MB instead of failing silently";

    let run = run_in_a_store(
        &inside,
        &["check", "--goal", goal, project.to_str().expect("a path")],
    );

    assert_eq!(
        run.status, 5,
        "a store inside the project returned {}. 2 would mean the parser refused the command \
         line; 3 would mean this build cannot record a goal, and it can. 5 is a run that tried \
         and did not finish:\nstdout:\n{}\nstderr:\n{}",
        run.status, run.stdout, run.stderr
    );
    assert!(
        run.stdout.is_empty(),
        "a run with no verdict wrote to standard output, where `sure check > report.txt` would \
         file it as a report:\n{}",
        run.stdout
    );
    // The three things the refusal has to say, in the words the rest of the
    // build uses for them: that it did not finish, that nothing was written,
    // and what it did instead of writing.
    assert!(
        run.stderr.contains("could not finish"),
        "the refusal does not say the run did not finish:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("Nothing was recorded"),
        "the refusal does not say whether the history changed:\n{}",
        run.stderr
    );
    assert!(
        run.stderr
            .contains("SURE stopped rather than treat it as authoritative"),
        "the refusal does not say what SURE did instead:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.contains(&inside.display().to_string())
            && run.stderr.contains(&project.display().to_string()),
        "the refusal does not name both the location and the project it is inside:\n{}",
        run.stderr
    );
    // And it is a statement about this run rather than about the build.
    assert!(
        !run.stderr.contains("is not implemented in this build"),
        "a run that tried is worded as a command this build lacks:\n{}",
        run.stderr
    );
    assert!(
        !inside.exists(),
        "SURE left {} inside the project it was recording a goal for",
        inside.display()
    );
}

#[test]
fn a_doctor_report_says_which_store_location_the_run_is_using() {
    // Acceptance 1's other half, and acceptance 3 in the same breath: a caller
    // can name the store, `sure doctor` reports the location it used, and a
    // caller who names nothing still gets the platform's own. The report is
    // where a redirect that was ignored and one that worked are told apart —
    // both print a path, and only one of them is the caller's.
    let store = a_store_of_our_own();

    let named = run_in_a_store(&store, &["--format", "json", "doctor"]);
    assert!(
        matches!(named.status, 0 | 1),
        "`sure doctor` exited {}:\n{}",
        named.status,
        named.stderr
    );
    let frame: serde_json::Value =
        serde_json::from_str(named.stdout.trim()).unwrap_or_else(|error| {
            panic!(
                "`sure --format json doctor` is not JSON: {error}\n{}",
                named.stdout
            )
        });
    assert_eq!(
        frame["details"]["places"]["store_location"], "caller",
        "the report does not say the store's location was the caller's: {frame}"
    );
    assert_eq!(
        frame["details"]["places"]["store_file"]["path"],
        serde_json::json!(store.join("sure.db").display().to_string()),
        "the report names a store other than the one the caller named: {frame}"
    );
    let human = run_in_a_store(&store, &["doctor"]);
    assert!(
        human.stdout.contains("named for this run"),
        "the human form does not say the location is the caller's:\n{}",
        human.stdout
    );
    assert!(
        human
            .stdout
            .contains(&store.join("sure.db").display().to_string()),
        "the human form does not print the store it is using:\n{}",
        human.stdout
    );

    // The default, unchanged: a run that names nothing is about the platform's
    // own per-user location, and it says so.
    let platform = Paths::discover().expect("this machine reports per-user locations");
    let machine = the_store_on_this_machine();
    let default = Run::of(
        &sure_without_a_store()
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap_or_else(|error| panic!("could not run {SURE}: {error}")),
    );
    assert!(
        matches!(default.status, 0 | 1),
        "`sure doctor` with no store named exited {}:\n{}",
        default.status,
        default.stderr
    );
    let frame: serde_json::Value = serde_json::from_str(default.stdout.trim())
        .unwrap_or_else(|error| panic!("`sure doctor` is not JSON: {error}\n{}", default.stdout));
    assert_eq!(
        frame["details"]["places"]["store_location"], "platform",
        "a caller who named nothing was reported as having named a location: {frame}"
    );
    assert_eq!(
        frame["details"]["places"]["store_file"]["path"],
        serde_json::json!(platform.store_file().display().to_string()),
        "a caller who named nothing did not get the platform's own store: {frame}"
    );
    // It read that store and did not change it: `sure doctor`'s own rule is
    // that a diagnostic does not alter what it is diagnosing.
    assert_untouched(&machine, "by a doctor run that named no store");
}

// --- the source scan ----------------------------------------------------

/// Every `.rs` file under a directory, with the text it holds.
///
/// A panic rather than a silent empty list when a directory cannot be read: a
/// scan over no files passes every assertion below, and a scan that proves
/// nothing while reporting success is the failure this repository is about.
fn rust_sources_under(directory: &Path) -> Vec<(PathBuf, String)> {
    fn walk(directory: &Path, into: &mut Vec<(PathBuf, String)>) {
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
    walk(directory, &mut found);
    assert!(
        !found.is_empty(),
        "{} holds no Rust sources, so a scan over it would prove nothing",
        directory.display()
    );
    found
}

/// This crate's own sources, with the path they came from.
fn sources() -> Vec<(PathBuf, String)> {
    rust_sources_under(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"))
}

/// The same, for the shipped code of every crate in the workspace.
///
/// `src` only, and not the test trees: a test may legitimately ask the platform
/// where SURE keeps its files in order to compare it with a location it made up,
/// while the code that *decides* where a run's store goes may not.
fn every_crates_sources() -> Vec<(PathBuf, String)> {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let entries =
        std::fs::read_dir(&crates).unwrap_or_else(|error| panic!("{}: {error}", crates.display()));
    let mut found = Vec::new();
    for entry in entries {
        let directory = entry.expect("a directory entry").path().join("src");
        if directory.is_dir() {
            found.extend(rust_sources_under(&directory));
        }
    }
    assert!(found.len() > 1, "the workspace has more than one crate");
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

#[test]
fn nothing_a_project_can_write_decides_where_the_store_goes() {
    // Acceptance 1's first half, as the absence it is. The store's location is
    // a value in the process's own argument vector (`--store-dir`) and there is
    // no second way in: not a file, and not an environment variable.
    //
    // A source scan, for the reason the test above is one — what is being ruled
    // out is a line of code, and no run can show that a line was never written.
    // The environment is the shape worth naming, because it is the tempting one:
    // a test harness sets the environment of the processes it starts, and so
    // does a checked project's own harness configuration (Claude Code's
    // `.claude/settings.json` has an `env` block, for one), so `SURE_STORE_DIR`
    // would be a store that a checked project can point at any directory it
    // likes. That is the defect acceptance 1 names, and the reason the module
    // documentation says there is deliberately no variable.
    //
    // Only the modules that decide the location are scanned. `SURE_OPENAI_API_KEY`
    // is read by the analysis provider and `PATH` by the tool search; neither is
    // a path SURE keeps anything in, and a scan over the whole workspace would
    // have to allow them and would then allow the thing it exists to catch.
    const DECIDES_THE_LOCATION: &[&str] = &[
        "crates/sure-core/src/paths/mod.rs",
        "crates/sure-cli/src/cli.rs",
        "crates/sure-cli/src/main.rs",
        "crates/sure-cli/src/commands.rs",
    ];
    const READS_THE_ENVIRONMENT: &[&str] = &["env::var", "env::vars", "var_os", "vars_os"];

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    for relative in DECIDES_THE_LOCATION {
        let path = workspace.join(relative);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        for token in READS_THE_ENVIRONMENT {
            assert!(
                !text.contains(token),
                "{} reads the environment with `{token}`. A store's location that a variable can \
                 set is a location the harness configuration of a checked project can set, which \
                 is a store the judged thing chooses. The location is `--store-dir` and nothing \
                 else.",
                path.display()
            );
        }
    }
}

#[test]
fn every_command_is_reached_by_the_location_the_caller_named() {
    // The other half of acceptance 1, and the half that is easy to lose: a
    // command that discovered the platform's location for itself would be a
    // command `--store-dir` cannot reach — `hook ingest` writing to a different
    // store than the verdict reads, which is worse than no mechanism at all.
    //
    // The no-argument `Paths::discover()` is that shape, because it is
    // `discover_at(None)`: it answers "the platform's own location" with nothing
    // able to say otherwise. It is allowed in exactly one file, the one that
    // defines it, and nowhere else in the workspace's shipped code. The scan is
    // over every crate's `src`, so a new crate cannot arrive with a second one.
    const DEFINES_IT: &[&str] = &["sure-core", "src", "paths", "mod.rs"];
    let mut defines_it = PathBuf::new();
    for part in DEFINES_IT {
        defines_it.push(part);
    }

    for (path, text) in every_crates_sources() {
        if path.ends_with(&defines_it) {
            continue;
        }
        assert!(
            !text.contains("Paths::discover()"),
            "{} discovers the platform's own store location for itself. Every command is handed \
             the location the caller named — `Command::report`'s argument, from `--store-dir` — \
             and a command that looked it up instead would keep its evidence somewhere the \
             verdict does not read. Call `Paths::discover_at(store)` with the location you were \
             given.",
            path.display()
        );
    }
}

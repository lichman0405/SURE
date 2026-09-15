//! What `sure_core::process` claims, checked by running real processes.
//!
//! # Why this file spawns its own test binary
//!
//! Every claim here is about a **process boundary** — what the child was given,
//! what it could see, what happened when it was stopped. A test that calls
//! `Command::new` and inspects the `Command` would be checking the builder
//! rather than the behaviour, and the interesting failures are all at the
//! boundary: an argument that got split, a variable that was passed on when it
//! should not have been, a stop that did not stop anything.
//!
//! So the child is **this test binary, run again** — the pattern
//! `tests/store_concurrency.rs` uses for the same reason. The child reports what
//! it found into a file, and the parent asserts on the file. A file rather than
//! stdout because a child's stdout is what two of these tests are *about*, and
//! because libtest's own output shares it.
//!
//! # What is not claimed here
//!
//! **Nothing about process trees on a platform that cannot reach them.**
//! [`Stop`] says which of the two stops happened, and the test asserts what the
//! platform can actually do rather than what would be nicest. `P3-T002` owns
//! the lifecycle coverage and `P3-T003` the Windows/Linux comparison; what is
//! here is the runner's own contract.
//!
//! **Not that a command that does not exist is classified.** Refusing to run
//! something is a decision about *whether*, which `P3-T004` and `P3-T005` make.
//! This file checks only that the machinery reports what it did.
//!
//! **Not that a batch file is refused, either.** Three of these tests are about
//! what Windows does with a `.cmd`, a `.bat` and a `.ps1` — that the first two
//! run, by way of a command interpreter the operating system starts for them,
//! and the third does not start at all. Refusing one is again a decision about
//! *whether*. What the runner does with the name it was given is a fact, and it
//! is the fact a caller has to have before it can decide anything — which is why
//! the modules say it out loud instead of leaving it to be discovered by the
//! first person whose `npm` did not run.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use sure_core::process::{
    Cancellation, CapturedOutput, Environment, Limits, Outcome, ProcessError, ProcessRequest, Stop,
    Termination,
};

/// The variable a spawned child reads its instructions from.
///
/// One variable holding the report path and one number, rather than several: a
/// child missing its instructions must fail loudly, and reading one variable
/// does that. It is also the variable the environment tests put in a
/// [`Environment::Only`] list, so a child that cannot see it cannot do anything
/// — which is the point of those tests rather than an accident.
const CHILD_ENV: &str = "SURE_PROCESS_TEST_CHILD";

/// The line the child writes into its report for each argument it was given.
const ARGUMENT_LINE: &str = "argument";

/// What [`child_writes_both_streams`] says on each stream.
///
/// Distinct sentences rather than one sentence twice: the assertion is that
/// each arrived on its own stream, and the same text on both would be satisfied
/// by a runner that read one pipe twice.
const SAID_ON_STDOUT: &str = "this is what it said\n";
const SAID_ON_STDERR: &str = "this is what went wrong\n";

/// A generous deadline for runs that are expected to finish on their own.
///
/// Generous on purpose: a runner that is slow on a loaded CI machine must not
/// fail a test that is about something else. The tests that are *about* the
/// deadline use a short one, and that is stated where it is used.
const PATIENT: Duration = Duration::from_secs(60);

/// A scratch directory for one test, named after the test and this process.
///
/// The name is not removed on entry: a path left behind by a killed run is
/// evidence, and silently clearing it would delete the thing a reader would
/// want to look at. It *is* removed on the way out, so repeated runs stay
/// independent.
fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("sure-process-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

/// Run one of the `#[ignore]`d children in this binary and hand back what came
/// of it, without asserting anything about the outcome.
///
/// `payload` is handed to the child **inside its argument vector**, as the value
/// of libtest's `--skip`, and is the reason the argument tests can be
/// behavioural: a payload a shell would have taken apart arrives either whole or
/// in pieces, and the child reports which.
fn run_child(
    mode: &str,
    payload: &str,
    working_directory: &Path,
    environment: Environment,
    limits: Limits,
    cancellation: Cancellation,
) -> Result<Outcome, ProcessError> {
    let executable = std::env::current_exe().expect("the test binary's own path");
    let request = ProcessRequest::new(executable, working_directory, limits, cancellation)
        .with_arguments([
            // `--exact`, so the filter cannot match another test whose name
            // starts the same way. `--ignored`, because a child is not a test
            // on its own. `--nocapture`, so what the child prints reaches the
            // stdout this runner captured rather than libtest's capture buffer.
            "--exact",
            mode,
            "--ignored",
            "--quiet",
            "--nocapture",
            // The payload, in the vector, one element. libtest treats it as a
            // skip filter, which matches nothing and removes nothing.
            "--skip",
            payload,
        ])
        .with_environment(environment);
    sure_core::process::run(&request)
}

/// The environment a child needs to do its job at all: one variable, holding
/// where to write and what number to use.
fn instructions(report: &Path, number: u64) -> OsString {
    OsString::from(format!("{}\n{number}", report.display()))
}

/// A report file nothing has written yet.
fn report_path(directory: &Path) -> PathBuf {
    directory.join("report.tsv")
}

/// Read a child's report as `key -> value`, keeping repeated keys in order.
///
/// Tab-separated, one fact per line, so a value containing anything but a
/// newline or a tab survives exactly. The argument test depends on that: the
/// payload is asserted byte for byte, and a format that trimmed or escaped it
/// would be testing the format.
fn consequences(path: &Path) -> Vec<(String, String)> {
    let text = fs::read_to_string(path).expect("the child's report");
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Every value reported for one key, in the order the child wrote them.
fn values<'a>(report: &'a [(String, String)], key: &str) -> Vec<&'a str> {
    report
        .iter()
        .filter(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
        .collect()
}

/// The one value reported for one key, or `None` if the child reported nothing
/// under it.
fn value<'a>(report: &'a [(String, String)], key: &str) -> Option<&'a str> {
    values(report, key).into_iter().next()
}

// ---------------------------------------------------------------------------
// The children. Each is ignored, so a normal run does not execute it, and each
// is reached by `run_child` with `--ignored --exact`.
// ---------------------------------------------------------------------------

/// The child that reports what it was given.
///
/// Everything the argument, directory and environment tests assert comes from
/// the file this writes. It reports rather than asserts: what is *correct* is
/// the parent's business, and a child that decided for itself would be a child
/// that could agree with a broken parent.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_reports_what_it_was_given() {
    let (instructions, _) = child_instructions();
    let mut lines = instructions.split('\n');
    let report = PathBuf::from(lines.next().expect("a report path"));

    let mut out = String::new();
    let cwd = std::env::current_dir().expect("a current directory");
    out.push_str(&format!("cwd\t{}\n", cwd.display()));
    for argument in std::env::args().skip(1) {
        out.push_str(&format!("{ARGUMENT_LINE}\t{argument}\n"));
    }
    let seen: BTreeMap<OsString, OsString> = std::env::vars_os().collect();
    out.push_str(&format!("environment_count\t{}\n", seen.len()));
    for name in [CHILD_ENV, "PATH", "SURE_PROCESS_TEST_CANARY"] {
        match seen.get(&OsString::from(name)) {
            Some(found) => out.push_str(&format!(
                "environment\t{name}\t{}\n",
                found.to_string_lossy()
            )),
            None => out.push_str(&format!("environment_absent\t{name}\n")),
        }
    }
    fs::write(&report, out).expect("the child's report");
}

/// The child that stays alive until something stops it.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_sleeps() {
    let (instructions, number) = child_instructions();
    let mut lines = instructions.split('\n');
    let report = PathBuf::from(lines.next().expect("a report path"));
    std::thread::sleep(Duration::from_millis(number));
    // Reached only if nothing stopped this process. Its absence is how a parent
    // tells "stopped" from "finished on its own", and it is written after the
    // sleep so that a stop at any point leaves no file.
    fs::write(&report, "finished\n").expect("the child's report");
}

/// The child that says a great deal.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_floods_stdout() {
    use std::io::Write;

    let (instructions, number) = child_instructions();
    let mut lines = instructions.split('\n');
    let _report = lines.next().expect("a report path");
    let chunk = vec![b'x'; 4096];
    let mut written = 0_u64;
    let mut stdout = std::io::stdout();
    while written < number {
        let size = chunk.len().min((number - written) as usize);
        stdout.write_all(&chunk[..size]).expect("a write");
        written += size as u64;
    }
    stdout.flush().expect("a flush");
}

/// The child that starts another process, and then stays alive itself.
///
/// Two processes deep, because one is not enough to test a claim about a tree.
/// The grandchild writes its report after sleeping, so **whether that file
/// exists** is how the parent tells a stop that reached it from a stop that
/// left it running — without a signal, a handle or a platform-specific
/// question, which is the only way the answer can be read on both platforms.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
// The grandchild is deliberately not waited on. Waiting for it would make this
// process the one that notices it die, and the whole question the test asks is
// whether the *runner* reached it — a child that reaped its own grandchild would
// answer that question itself and answer it wrongly. Not waiting is also what
// leaving a real process behind looks like, which is the thing being tested for.
#[allow(clippy::zombie_processes)]
fn child_starts_a_grandchild() {
    let (given, number) = child_instructions();
    let mut lines = given.split('\n');
    let grandchild_report = PathBuf::from(lines.next().expect("a grandchild report path"));

    let executable = std::env::current_exe().expect("the test binary's own path");
    Command::new(executable)
        .args([
            "--exact",
            "child_sleeps",
            "--ignored",
            "--quiet",
            // The grandchild gets no fresh environment: it inherits this
            // process's, which is exactly what the runner handed over. That is
            // what makes it a grandchild of the *runner's* child rather than a
            // process the test set up for itself.
        ])
        .env(CHILD_ENV, instructions(&grandchild_report, number))
        .spawn()
        .expect("a grandchild");

    // Long enough that the parent's deadline certainly comes first.
    std::thread::sleep(Duration::from_secs(30));
}

/// The child that says one thing on each of its output streams, and different
/// things on each.
///
/// Both facts matter. That it writes to standard error at all is what makes
/// "both output streams are captured" a claim a test can hold — every other
/// child here says nothing on standard error, so a runner that left it
/// unwatched would pass each of them. That the two are *different* is what
/// catches a runner that wired one stream into the other.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_writes_both_streams() {
    use std::io::Write;

    let _ = child_instructions();
    let mut stdout = std::io::stdout();
    stdout
        .write_all(SAID_ON_STDOUT.as_bytes())
        .expect("a write");
    stdout.flush().expect("a flush");
    let mut stderr = std::io::stderr();
    stderr
        .write_all(SAID_ON_STDERR.as_bytes())
        .expect("a write");
    stderr.flush().expect("a flush");
}

/// The child that ends with an exit code of its choosing.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_exits_with_the_code_it_was_given() {
    let (_, number) = child_instructions();
    std::process::exit(number as i32);
}

/// Read this child's instructions out of the one variable it is given.
fn child_instructions() -> (String, u64) {
    let raw = std::env::var(CHILD_ENV).expect("the child's instructions");
    let mut parts = raw.split('\n');
    let path = parts.next().expect("a report path").to_owned();
    let number: u64 = parts.next().expect("a number").parse().expect("a number");
    (path, number)
}

// ---------------------------------------------------------------------------
// The claims.
// ---------------------------------------------------------------------------

#[test]
fn an_argument_a_shell_would_have_taken_apart_arrives_whole() {
    // The acceptance is "direct args used instead of unsafe shell string
    // construction where possible". A test that read back `request.arguments()`
    // would be checking that the vector held what was put in it, which is true
    // of any builder. This one holds the behaviour: the string below is one
    // argument, and it is the kind of thing that stops being one argument the
    // moment anything between here and the child joins and re-splits it.
    //
    // It contains a space, a semicolon, a command substitution, a redirect, a
    // pipe and a quote. A shell given this text would run `echo`, would run
    // `whoami` and substitute its output, and would redirect into `out`. None
    // of that happens if nothing is a shell.
    const PAYLOAD: &str = "one two; echo $(whoami) > out | three \"four\"";

    let directory = scratch("arguments");
    let report = report_path(&directory);
    let outcome = run_child(
        "child_reports_what_it_was_given",
        PAYLOAD,
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");

    assert!(
        matches!(outcome.termination(), Termination::Exited { code: Some(0) }),
        "the child should have run to completion, got {:?}",
        outcome.termination()
    );

    let reported = consequences(&report);
    let arguments = values(&reported, ARGUMENT_LINE);

    // Exactly one element equals the payload, and it is not merely a substring
    // of one: the whole element is the payload or the test has not shown what
    // it says it shows.
    assert!(
        arguments.contains(&PAYLOAD),
        "the payload should arrive as exactly one argument; the child saw {arguments:#?}"
    );

    // And nothing was expanded. If a command substitution had run, `$(whoami)`
    // would not be here.
    assert!(
        arguments
            .iter()
            .any(|argument| argument.contains("$(whoami)")),
        "the payload should not have been expanded; the child saw {arguments:#?}"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn the_program_runs_in_the_directory_the_request_named() {
    // A directory whose name has a space and a character outside ASCII. Both are
    // ordinary on this platform and both are what a path gets mangled by when
    // it is pasted into something that re-splits it, so a working directory
    // that survives them is a working directory that was passed as a path.
    let parent = scratch("working-directory");
    let directory = parent.join("a directory with a space and \u{00e9}\u{4e2d}\u{6587}");
    fs::create_dir_all(&directory).expect("a directory with an awkward name");
    let report = report_path(&parent);

    let outcome = run_child(
        "child_reports_what_it_was_given",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    let reported = consequences(&report);
    let cwd = value(&reported, "cwd").expect("the child reported its directory");

    // Compared as paths rather than as strings: Windows will hand the same
    // directory back with a different drive-letter case, and refusing that
    // would be testing the platform's spelling rather than the directory.
    let seen = Path::new(cwd);
    assert_eq!(
        fs::canonicalize(seen).expect("the child's directory exists"),
        fs::canonicalize(&directory).expect("the directory we asked for exists"),
        "the child ran in {cwd}, not in {}",
        directory.display()
    );

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn nothing_of_sures_environment_is_passed_on_when_the_request_says_only() {
    let directory = scratch("environment-only");
    let report = report_path(&directory);
    // The child's whole environment is this one variable. It cannot read its
    // instructions unless that variable is here, so a child that reports at all
    // has already shown the list was honoured rather than emptied.
    let environment = Environment::only([(OsString::from(CHILD_ENV), instructions(&report, 0))]);

    let outcome = run_child(
        "child_reports_what_it_was_given",
        "unused",
        &directory,
        environment,
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    let reported = consequences(&report);
    assert_eq!(
        value(&reported, "environment_count"),
        Some("1"),
        "the child should see exactly the one variable it was given, and saw {:#?}",
        values(&reported, "environment")
    );
    assert_eq!(
        values(&reported, "environment_absent"),
        vec!["PATH", "SURE_PROCESS_TEST_CANARY"],
        "nothing SURE has should have reached the child, and neither should anything another \
         request set; the canary is the control against a list that is only cleared of the \
         variables this test happened to think of"
    );
    assert!(
        reported
            .iter()
            .any(|(key, found)| key == "environment" && found.starts_with(CHILD_ENV)),
        "the one variable the request named should be the one the child has"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_named_variable_is_kept_from_a_child_that_inherits_the_rest() {
    // The other half of the environment choice, and the control for the test
    // above: the same question asked of a request that *does* pass the
    // environment on. Without this, "PATH was absent" would be satisfied by a
    // runner that passed nothing on whichever way it was asked.
    let directory = scratch("environment-without");
    let report = report_path(&directory);

    let outcome = run_child(
        "child_reports_what_it_was_given",
        "unused",
        &directory,
        Environment::inherited()
            .with(CHILD_ENV, instructions(&report, 0))
            .with("SURE_PROCESS_TEST_CANARY", "set by the request")
            .without("PATH"),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    let reported = consequences(&report);
    assert!(
        values(&reported, "environment_absent").contains(&"PATH"),
        "PATH was named as one to keep back"
    );
    assert_eq!(
        value(&reported, "environment\tSURE_PROCESS_TEST_CANARY"),
        None,
        "the canary is reported under its own key, not this one"
    );
    assert!(
        reported.iter().any(|(key, found)| key == "environment"
            && *found == "SURE_PROCESS_TEST_CANARY\tset by the request"),
        "the variable the request set should have reached the child, and {:#?} did not",
        values(&reported, "environment")
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_child_that_ends_by_itself_reports_the_code_it_ended_with() {
    let directory = scratch("exit-code");
    let report = report_path(&directory);
    let outcome = run_child(
        "child_exits_with_the_code_it_was_given",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 3)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");

    // Three, not zero: a runner that reported success for everything would pass
    // an exit-code test that only ever asked for zero.
    assert_eq!(
        outcome.termination(),
        Termination::Exited { code: Some(3) },
        "the exit code should be the one the program ended with"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_run_that_passes_its_deadline_is_stopped_and_says_so() {
    let directory = scratch("timeout");
    let report = report_path(&directory);
    // The child would sleep far longer than the deadline, so a run that comes
    // back at all came back because it was stopped.
    let outcome = run_child(
        "child_sleeps",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 10_000)),
        Limits::new(Duration::from_millis(150), 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child starts");

    match outcome.termination() {
        Termination::TimedOut { stopped } => {
            // The platform's answer, stated rather than assumed. This is the
            // assertion that would have to change if Unix ever gained a way to
            // reach a tree, which is the point: it is not a claim about what
            // would be nice.
            #[cfg(windows)]
            assert_eq!(stopped, Stop::WholeTree);
            #[cfg(not(windows))]
            assert_eq!(stopped, Stop::ProcessOnly);
        }
        other => panic!("expected a deadline to stop the run, got {other:?}"),
    }

    // Stopped means stopped: the child writes its report after the sleep, and a
    // stopped child never gets there.
    assert!(
        !report.exists(),
        "the child reached the end of its sleep, so the deadline did not stop it"
    );
    assert!(
        outcome.took() >= Duration::from_millis(150),
        "the run should have lasted at least its deadline, and lasted {:?}",
        outcome.took()
    );
    // And not much longer. A lower bound alone would be satisfied by a runner
    // that noticed the deadline ten seconds late, which is a deadline that is
    // not being honoured — the allowance is for a loaded machine, not for a
    // poll interval nobody would accept.
    assert!(
        outcome.took() < Duration::from_secs(3),
        "the deadline was 150ms and the run took {:?}, so it was noticed far too late",
        outcome.took()
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_stopped_run_reaches_what_the_run_started_or_says_that_it_did_not() {
    // `Stop::WholeTree` is a claim, and a claim nothing holds is the failure
    // this repository goes looking for. The test above asserts which variant
    // comes back; this one asserts whether it is **true** — the difference
    // between a runner that stops a tree and a runner that says it did.
    //
    // The grandchild sleeps, then writes a file. Nothing sends it a signal and
    // nothing asks the platform what happened to it: the file either appears or
    // it does not, and that is the whole answer on both platforms.
    let directory = scratch("process-tree");
    let grandchild = report_path(&directory.join("grandchild"));
    fs::create_dir_all(grandchild.parent().expect("a parent")).expect("a grandchild directory");
    const GRANDCHILD_SLEEP_MS: u64 = 1_500;

    let outcome = run_child(
        "child_starts_a_grandchild",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&grandchild, GRANDCHILD_SLEEP_MS)),
        Limits::new(Duration::from_millis(250), 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child starts");

    let stopped = match outcome.termination() {
        Termination::TimedOut { stopped } => stopped,
        other => panic!("expected the deadline to stop the run, got {other:?}"),
    };

    // Long enough for the grandchild to have written its report if it were
    // still running when the parent stopped the outer process.
    std::thread::sleep(Duration::from_millis(GRANDCHILD_SLEEP_MS + 1_000));

    if cfg!(windows) {
        assert_eq!(stopped, Stop::WholeTree);
        assert!(
            !grandchild.exists(),
            "the run reported that it stopped the whole tree, and the grandchild ran to the end \
             of its sleep — so the report was wrong"
        );
    } else {
        // Not a skip. The gap is **measured** rather than described, so the day
        // a Unix build can reach a process tree this test fails and has to be
        // changed on purpose rather than being discovered by a reader.
        assert_eq!(stopped, Stop::ProcessOnly);
        assert!(
            grandchild.exists(),
            "this platform was expected to reach only the process itself, and the grandchild was \
             stopped anyway — `Stop::ProcessOnly` is now an understatement and the report should \
             say so"
        );
    }

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_cancelled_run_is_stopped_before_its_deadline() {
    let directory = scratch("cancelled");
    let report = report_path(&directory);
    let cancellation = Cancellation::new();
    let cancelled_by = cancellation.clone();

    let started = Instant::now();
    let canceller = std::thread::spawn(move || {
        // Long enough that the child is certainly running, short enough that
        // the deadline below is nowhere near.
        std::thread::sleep(Duration::from_millis(100));
        cancelled_by.cancel();
    });

    let outcome = run_child(
        "child_sleeps",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 10_000)),
        Limits::new(Duration::from_secs(30), 64 * 1024, 64 * 1024),
        cancellation,
    )
    .expect("the child starts");
    canceller.join().expect("the cancelling thread");

    assert!(
        matches!(outcome.termination(), Termination::Cancelled { .. }),
        "expected the cancellation to stop the run, got {:?}",
        outcome.termination()
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the cancellation was asked for 100ms in and the deadline was 30s, so a run that takes \
         this long ended on neither; it took {:?}",
        started.elapsed()
    );
    assert!(
        !report.exists(),
        "the child reached the end of its sleep, so the cancellation did not stop it"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_run_cancelled_before_it_started_never_starts_the_program() {
    // The distinction the whole `CancelledBeforeStart` variant exists for: a
    // caller starting something with an effect needs to know it never began,
    // because "started and was stopped" and "never started" are different
    // facts about the world.
    let directory = scratch("cancelled-before-start");
    let report = report_path(&directory);
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let outcome = run_child(
        "child_reports_what_it_was_given",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        cancellation,
    )
    .expect("a request that never starts is not an error");

    assert_eq!(outcome.termination(), Termination::CancelledBeforeStart);
    assert!(
        !report.exists(),
        "the program ran, so it was started and stopped rather than never started"
    );
    assert_eq!(
        outcome.took(),
        Duration::ZERO,
        "nothing ran, so nothing took time"
    );
    assert!(!outcome.stdout().was_truncated());
    assert_eq!(outcome.stdout().bytes().len(), 0);

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_talkative_program_has_the_beginning_of_its_output_kept_and_the_rest_counted() {
    let directory = scratch("truncated");
    let report = report_path(&directory);
    const LIMIT: usize = 4096;
    const WRITTEN: u64 = 200_000;

    let outcome = run_child(
        "child_floods_stdout",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, WRITTEN)),
        // The two bounds are **different on purpose**. Equal bounds would let a
        // runner that used one for both pass every test here, and "how much of
        // standard output" and "how much of standard error" are two separate
        // statements a caller makes. Ten times, so that a swap is a difference
        // of an order of magnitude rather than a neighbouring value.
        Limits::new(PATIENT, LIMIT, LIMIT * 10),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    let stdout = outcome.stdout();
    assert_eq!(
        stdout.bytes().len(),
        LIMIT,
        "exactly the bound for standard output should be kept — no more, and not less"
    );
    assert!(
        !outcome.stderr().was_truncated(),
        "nothing was written to standard error, so its own larger bound should not have been \
         reached and standard output's bound should not have been applied to it"
    );
    assert!(
        stdout.was_truncated(),
        "a stream that was cut short must say so, or a reader takes a beginning for the whole"
    );
    assert!(
        stdout.discarded_bytes() >= WRITTEN - stdout.bytes().len() as u64,
        "the child wrote {WRITTEN} bytes and {} were accounted for",
        stdout.bytes().len() as u64 + stdout.discarded_bytes()
    );
    assert_eq!(
        stdout.unfinished(),
        None,
        "the stream ended; being cut short by the bound is a different thing from not being read"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn what_the_program_says_on_each_stream_arrives_on_that_stream() {
    // "Both streams are captured" and "the two are not the same stream" are
    // two claims, and this holds both. The second is the one that needs a
    // child saying different things on each: a runner that captured standard
    // error into standard output, or read the same pipe twice, would pass every
    // other test in this file.
    //
    // The assertions are `contains` rather than equality because libtest shares
    // the child's standard output — with `--quiet --nocapture` it still prints
    // its own summary there when the child returns from the test rather than
    // exiting through `std::process::exit`. What is *not* shared is standard
    // error, so the negative assertions are the ones that carry the weight.
    let directory = scratch("both-streams");
    let report = report_path(&directory);
    let outcome = run_child(
        "child_writes_both_streams",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    let out = outcome.stdout().text_lossy();
    let error = outcome.stderr().text_lossy();

    assert!(
        out.contains(SAID_ON_STDOUT),
        "what the program said on standard output did not arrive there; it arrived as {out:?}"
    );
    assert!(
        error.contains(SAID_ON_STDERR),
        "what the program said on standard error did not arrive there; it arrived as {error:?}"
    );
    assert!(
        !out.contains("went wrong"),
        "the two streams were read as one; standard output holds {out:?}"
    );
    assert!(
        !error.contains("it said"),
        "the two streams were read as one; standard error holds {error:?}"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_program_that_says_little_is_not_reported_as_having_been_cut_short() {
    // The control for the test above. Without it, a runner that set
    // `was_truncated` for every run would pass, and the flag would be worth
    // nothing to the caller that reads it.
    let directory = scratch("not-truncated");
    let report = report_path(&directory);
    let outcome = run_child(
        "child_exits_with_the_code_it_was_given",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");

    let stdout = outcome.stdout();
    assert!(
        !stdout.was_truncated(),
        "nothing was written, so nothing was cut"
    );
    assert_eq!(stdout.discarded_bytes(), 0);
    assert_eq!(stdout.unfinished(), None);

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_working_directory_that_is_not_a_full_path_is_refused() {
    let relative = PathBuf::from("some").join("relative").join("place");
    let request = ProcessRequest::new(
        "a-program-that-is-never-started",
        relative.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );

    match sure_core::process::run(&request) {
        Err(ProcessError::WorkingDirectoryNotAbsolute { working_directory }) => {
            assert_eq!(working_directory, relative);
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_working_directory_that_is_not_there_is_named_as_the_directory() {
    // The reason this is checked before the process is started: the operating
    // system gives the same error for a missing directory as for a missing
    // program, so without the check a directory that had moved would be
    // reported as a program that was not installed.
    let directory = scratch("missing-directory");
    let missing = directory.join("not-there");
    let request = ProcessRequest::new(
        // A program that is also not there, so that the error can only be the
        // directory's if the directory is checked first.
        directory.join("no-such-program"),
        missing.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );

    match sure_core::process::run(&request) {
        Err(ProcessError::WorkingDirectoryUnusable {
            working_directory, ..
        }) => {
            assert_eq!(working_directory, missing);
        }
        other => panic!("expected the directory to be named, got {other:?}"),
    }

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_working_directory_that_is_a_file_is_refused_as_a_directory() {
    // The other half of "it is not there": the path exists, so `metadata`
    // succeeds, and the reason it cannot be run in is a different reason. The
    // two are reported under the same variant with the message doing the
    // distinguishing, which is only honest if both are reachable — an
    // `is_dir` check that was written as `exists` would pass the test above and
    // fail here, because the operating system refuses a *file* as a working
    // directory with an error about the directory, not about the program.
    let directory = scratch("directory-is-a-file");
    let file = directory.join("not-a-directory.txt");
    fs::write(&file, "this is a file").expect("a file where a directory was asked for");
    let request = ProcessRequest::new(
        // Again a program that is not there, so that the answer can only be the
        // directory's if the directory is the thing that was checked.
        directory.join("no-such-program"),
        file.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );

    match sure_core::process::run(&request) {
        Err(ProcessError::WorkingDirectoryUnusable {
            working_directory,
            message,
        }) => {
            assert_eq!(
                working_directory, file,
                "the refusal should name the path that was asked for"
            );
            assert!(
                !message.is_empty(),
                "a refusal that does not say why is one a caller cannot act on"
            );
        }
        other => panic!("expected a file to be refused as a directory, got {other:?}"),
    }

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_program_that_is_not_there_is_reported_as_not_started() {
    let directory = scratch("missing-program");
    let missing = directory.join("no-such-program-here");
    let request = ProcessRequest::new(
        missing.clone(),
        directory.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );

    match sure_core::process::run(&request) {
        Err(ProcessError::NotStarted { program, .. }) => {
            assert_eq!(program, missing.into_os_string())
        }
        other => panic!("expected the program to be reported as not started, got {other:?}"),
    }

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_command_line_offered_as_a_program_is_not_a_program() {
    // The structural claim, tested rather than asserted. A caller holding the
    // line a README gives has nowhere to put it, and this is what happens if
    // they put it where a program goes anyway: nothing splits it, nothing looks
    // for a shell, and the operating system is asked for a file with that name.
    let directory = scratch("command-line");
    let request = ProcessRequest::new(
        "npm install && npm test",
        directory.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );

    match sure_core::process::run(&request) {
        Err(ProcessError::NotStarted { .. }) => {}
        other => panic!("a command line is not a program, and this was {other:?}"),
    }

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn the_outcome_carries_when_the_run_happened_and_how_long_it_took() {
    let directory = scratch("timing");
    let report = report_path(&directory);
    let before = std::time::SystemTime::now();
    let outcome = run_child(
        "child_sleeps",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 60)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    let after = std::time::SystemTime::now();

    assert!(
        outcome.started_at() >= before && outcome.started_at() <= after,
        "the start should be between the two moments the parent can see"
    );
    assert!(
        outcome.took() >= Duration::from_millis(60),
        "the child slept 60ms, so the run cannot have been shorter; it was {:?}",
        outcome.took()
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_request_hands_back_every_part_of_itself_before_it_is_run() {
    // "Explicit" has to mean readable as well as stated. A caller that can
    // describe a run without making it is what `P3-T005`'s check plan needs,
    // and it is also the only way to see what a request holds without running
    // it — which is what a test that is *not* about behaviour can check.
    let directory = scratch("read-back");
    let cancellation = Cancellation::new();
    let limits = Limits::new(Duration::from_millis(1234), 11, 22);
    let request = ProcessRequest::new("program-name", &directory, limits, cancellation.clone())
        .with_arguments(["first", "second with a space"])
        .with_environment(Environment::only([(
            OsString::from("A"),
            OsString::from("B"),
        )]));

    assert_eq!(request.program(), std::ffi::OsStr::new("program-name"));
    assert_eq!(
        request.arguments(),
        [
            OsString::from("first"),
            OsString::from("second with a space")
        ],
        "two arguments, the second of them one argument containing a space"
    );
    assert_eq!(request.working_directory(), directory.as_path());
    assert_eq!(request.limits(), limits);
    assert_eq!(request.limits().timeout(), Duration::from_millis(1234));
    assert_eq!(request.limits().stdout_bytes(), 11);
    assert_eq!(request.limits().stderr_bytes(), 22);
    assert_eq!(
        request.environment(),
        &Environment::only([(OsString::from("A"), OsString::from("B"))])
    );

    // The cancellation on the request is the one the caller holds, not a copy
    // that stopped being connected to it.
    assert!(!request.cancellation().is_cancelled());
    cancellation.cancel();
    assert!(
        request.cancellation().is_cancelled(),
        "cancelling through the caller's handle must be visible to the request"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn an_output_bound_of_zero_keeps_nothing_and_still_says_what_was_written() {
    // A bound of zero is a way to say "this stream does not matter", and it
    // must not become a way to say "this stream was empty". The distinction is
    // `was_truncated`, and it is the whole reason the count of discarded bytes
    // exists rather than the kept bytes alone.
    let directory = scratch("zero-bound");
    let report = report_path(&directory);
    const WRITTEN: u64 = 50_000;

    let outcome = run_child(
        "child_floods_stdout",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, WRITTEN)),
        Limits::new(PATIENT, 0, 0),
        Cancellation::new(),
    )
    .expect("the child runs");

    let stdout: &CapturedOutput = outcome.stdout();
    assert_eq!(stdout.bytes().len(), 0);
    assert!(
        stdout.was_truncated(),
        "nothing kept and nothing written are different facts, and this one is the first"
    );
    assert!(stdout.discarded_bytes() >= WRITTEN);

    let _ = fs::remove_dir_all(&directory);
}

#[test]
#[cfg(windows)]
fn a_batch_file_named_with_its_extension_runs_and_windows_brings_the_interpreter() {
    // The other side of `a_command_line_offered_as_a_program_is_not_a_program`
    // above. SURE builds no command line and starts no shell of its own, and
    // that is a statement about **what SURE assembles**, not a fence around what
    // runs: hand this runner the path of a `.cmd` and the operating system
    // starts `cmd.exe` to run it. The measurement is from this machine, Windows
    // 11 with Rust 1.98 — the image of the process created for a batch file
    // reads back as `C:\Windows\System32\cmd.exe`.
    //
    // This test holds the fact, not the policy. Whether a caller may name a
    // batch file — a file whose whole content is shell text, which is what
    // `P3-T004` classifies and `P3-T005` asks permission for — is not settled
    // here, and this is not an argument either way. What it fixes is what
    // happens, so that the documentation and the next caller cannot be surprised
    // by it, and so that a change which made the runner refuse `.cmd` outright
    // would arrive as a failing test and a decision rather than a quiet edit.
    let directory = scratch("batch-file");
    let program = directory.join("says-something.cmd");
    fs::write(&program, "@echo off\r\necho it ran > ran.txt\r\n").expect("a batch file");

    let request = ProcessRequest::new(
        program,
        directory.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );
    let outcome = sure_core::process::run(&request).expect("the batch file runs");

    assert_eq!(
        outcome.termination(),
        Termination::Exited { code: Some(0) },
        "the batch file was expected to run to the end and report success"
    );
    let marker = fs::read_to_string(directory.join("ran.txt")).unwrap_or_else(|error| {
        panic!(
            "nothing was written by the batch file, so it did not run: {error}. The marker is \
             written relative to the working directory the request named, so its absence would \
             also mean the directory was not the one asked for."
        )
    });
    assert!(
        marker.contains("it ran"),
        "the marker is the batch file's own output, and it says {marker:?}"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
#[cfg(windows)]
fn a_name_with_no_extension_never_becomes_a_batch_file_that_is_right_there() {
    // The case that matters for a project directory. `build.cmd`, `test.cmd` and
    // `run.cmd` are how a Windows project states its commands, and if a bare
    // `build` reached `build.cmd`, then "SURE runs the program it was named"
    // would quietly mean "SURE runs a batch file out of the project" — a command
    // interpreter started on project-controlled text, which is what
    // `RUST_DESIGN.md` forbids building and which no caller would have had the
    // chance to classify.
    //
    // It does not happen, and the reason is the operating system's rule rather
    // than SURE's care: a name with no extension is completed with `.exe` and
    // nothing else. The batch file here is named after this process, so no copy
    // of it exists anywhere else on the machine and no search order can reach a
    // different one; the run happens in the directory the file is in, so "the
    // current directory was not searched" is not an explanation for this
    // passing.
    let directory = scratch("bare-name");
    let stem = format!("build{}", std::process::id());
    let batch = directory.join(format!("{stem}.cmd"));
    fs::write(&batch, "@echo off\r\necho it ran > ran.txt\r\n").expect("a batch file");

    let request = ProcessRequest::new(
        &stem,
        directory.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );

    match sure_core::process::run(&request) {
        Err(ProcessError::NotStarted { .. }) => {}
        other => panic!(
            "a name with no extension was expected not to reach {}, and this was {other:?}",
            batch.display()
        ),
    }
    assert!(
        !directory.join("ran.txt").exists(),
        "the run was reported as not started and the batch file ran anyway, which would mean the \
         report was wrong about the thing the product most needs it to be right about"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
#[cfg(windows)]
fn a_power_shell_script_cannot_be_started_as_a_program() {
    // The half of the old claim that survives measurement, and the reason it
    // survives: `CreateProcess` starts an *image*, and a `.ps1` is not one. The
    // failure is "not a valid Win32 application" — a different thing from the
    // "cannot find the file" above, and the difference is worth keeping: one is
    // a script that is not there, the other a script the operating system will
    // not start without being told how.
    //
    // Running one needs `powershell.exe -File script.ps1`, which is a command
    // line — the door this module does not have. So the refusal is not a missing
    // feature to be filled in later inside the runner; it is the door being
    // absent, and `P3-T005`'s consent question is where it would be opened, if
    // it ever is.
    let directory = scratch("power-shell");
    let program = directory.join("says-something.ps1");
    fs::write(&program, "Set-Content -Path ran.txt -Value 'it ran'\r\n").expect("a script");

    let request = ProcessRequest::new(
        program,
        directory.clone(),
        Limits::new(PATIENT, 1024, 1024),
        Cancellation::new(),
    );

    match sure_core::process::run(&request) {
        Err(ProcessError::NotStarted { .. }) => {}
        other => {
            panic!("a .ps1 is not a program the operating system starts, and this was {other:?}")
        }
    }
    assert!(
        !directory.join("ran.txt").exists(),
        "the script was reported as not started and ran anyway"
    );

    let _ = fs::remove_dir_all(&directory);
}

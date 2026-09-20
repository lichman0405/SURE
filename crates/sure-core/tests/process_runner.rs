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
//! platform can actually do rather than what would be nicest. The comparison
//! across the three platforms is the table below, which every one of these tests
//! is a row of; what is here is the runner's own contract, and the platform's
//! answer to it.
//!
//! # The platform matrix
//!
//! The runner's platform-specific behaviour is a short table, and every row of
//! it is held by a test that runs **on the platform it is about**. That is the
//! only form in which "CI covers platform-specific runner behaviour" is a fact
//! rather than an intention: each of the three jobs compiles different code, so
//! a row whose test is absent from a job's log is a row nothing checked there.
//! **Read a run by test name, never by total** — a green job says nothing about
//! the other two, and the three run different sets.
//!
//! | What differs | Windows | Linux | macOS |
//! | --- | --- | --- | --- |
//! | what a stop reaches | the whole tree, through `taskkill /T /F` | the process itself; nothing below it | the same as Linux |
//! | what a name may be | no extension is completed with `.exe` and nothing else | the name is the path, and the executable bit decides | the same as Linux |
//! | a file that is not an image | `.cmd` and `.bat` start, with an interpreter Windows supplies; `.ps1` does not start at all | a file whose first line names an interpreter starts; without the executable bit nothing starts | the same as Linux |
//! | a path that is not text | cannot be spelled at all — an unpaired surrogate is not a path | bytes are bytes: the name, the working directory and every argument arrive unchanged | **cannot be spelled at all either** — the filesystem refuses the name with `EILSEQ`, so the question cannot be asked here |
//!
//! The last column is not decoration. **`#[cfg(unix)]` spans Linux and macOS,
//! and this table is where they stop agreeing**: the row it breaks is the last
//! one, and the first version of this work had it as a single "Linux and macOS"
//! cell that was true of exactly one of them. CI said so — run `34929385200`,
//! `rust (macos-latest)`, `Os { code: 92, message: "Illegal byte sequence" }`
//! from the `fs::write` that creates the program — which is why the three
//! columns are written out and why the answer for each is held by a test rather
//! than by this paragraph.
//!
//! The first row is `P3-T002`'s; the other three are the ones `P3-T002` said
//! nothing about and `P3-T003` added. Where a row has two halves they are a pair
//! — a positive and a negative where the negative is the interesting half —
//! because "it did not run" and "it was not asked to run" are different facts
//! and only the second is a contract.
//!
//! **The lifecycle claims are about a process, not about a report.** A stop is
//! asked to prove itself against a grandchild that is given a way to say
//! whether it is still alive, because the runner's own answer to "did you stop
//! it" is the thing under test and not the thing that can settle it. That
//! instrument is written up at [`child_waits_to_be_released`], and the reason
//! the positive control is not optional is written up beside the assertion that
//! uses it.
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

/// How long the run in a tree test is given before its deadline stops it.
///
/// Generous rather than tight, and for a measured reason: what the test needs
/// is for the grandchild to have **started** before the deadline arrives, and
/// that costs two process starts on a machine nobody controls. Two seconds
/// leaves that twenty times over. What it must **not** be is long enough for
/// anything to happen after it, which is why the grandchild is given no
/// lifetime of its own — see [`child_waits_to_be_released`].
const TREE_DEADLINE: Duration = Duration::from_secs(2);

/// How often a waiting child looks for its release file.
///
/// Milliseconds, and the unit is in the name because the number goes through
/// the same channel as everything else a child is told.
const POLL_INTERVAL: u64 = 25;

/// How long a stopped run gets to prove a released grandchild is gone.
///
/// The grandchild polls every [`POLL_INTERVAL`], so a grandchild that is still
/// running writes within a small multiple of that; a second is forty of them.
/// This is a wait for something that must **not** happen, which is the only
/// kind of wait that has to be bounded by the observer rather than by the
/// observed.
const RELEASE_WAIT: Duration = Duration::from_secs(1);

/// How long a waiting child waits for a release that may never come.
///
/// A child whose parent failed its own test is a child nobody will release, and
/// a grandchild that waits forever is a process left on the machine by a
/// failing test — which is the thing these tests exist to prevent. Thirty
/// seconds bounds it without ever reaching it, because a released grandchild
/// writes in milliseconds and a dead one writes never.
const ABANDONED: Duration = Duration::from_secs(30);

/// A directory name that is ordinary on both platforms and mangles easily.
///
/// One spelling used by every test that needs it, so that "a path with a space
/// and a character outside ASCII" means the same thing in all of them and a
/// test cannot pass by being awkward in a way the others are not. **The space
/// is the half that breaks first**: a path is re-split on whitespace by
/// anything that turns a path into a command line, and the non-ASCII half is
/// the half that breaks silently, by arriving as different bytes rather than as
/// two pieces.
const AWKWARD: &str = "a directory with a space and \u{00e9}\u{4e2d}\u{6587}";

/// A string that is not ASCII, for the argument and output tests.
///
/// It carries three different problems on purpose, and they fail differently:
/// `\u{00e9}` and `\u{00f6}` are Latin-1 characters that a byte-oriented path
/// would pass through unchanged and a code-page conversion would not;
/// `\u{4e2d}\u{6587}` is outside Latin-1, so nothing but a real encoding
/// survives it; and `\u{1f389}` is **outside the Basic Multilingual Plane**, so
/// on Windows it is two UTF-16 code units and a surrogate pair — the case a
/// conversion that stops at the first unit truncates and a length computed in
/// code units gets wrong.
const NOT_ASCII: &str = "h\u{00e9}llo w\u{00f6}rld \u{4e2d}\u{6587} \u{1f389}";

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
    run_child_at(
        &executable,
        mode,
        payload,
        working_directory,
        environment,
        limits,
        cancellation,
    )
}

/// The same, but with the program named rather than assumed.
///
/// Only one test needs this, and it needs it for the reason the program path is
/// worth testing at all: **the program is the one part of a request that cannot
/// be given a name the test controls while it is still this binary.** Every
/// other test here runs `current_exe()`, whose path is whatever the build
/// directory happens to be. The test that is about the program's own path has
/// to put a copy somewhere awkward first, and that copy is what it hands here.
fn run_child_at(
    executable: &Path,
    mode: &str,
    payload: &str,
    working_directory: &Path,
    environment: Environment,
    limits: Limits,
    cancellation: Cancellation,
) -> Result<Outcome, ProcessError> {
    let request = ProcessRequest::new(
        executable.to_path_buf(),
        working_directory,
        limits,
        cancellation,
    )
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

/// The marker a waiting child writes the moment it is running.
///
/// A sibling of the report rather than a line inside it, because the whole
/// point of the marker is that it is written **before** anything the child is
/// waiting on. A child that has to finish before it reports has already proven
/// nothing about the moment in between.
fn started_path(report: &Path) -> PathBuf {
    report.with_extension("started")
}

/// The file that tells a waiting child it may finish.
fn released_path(report: &Path) -> PathBuf {
    report.with_extension("release")
}

/// Where the grandchild in a tree test writes, in a directory of its own.
///
/// The directory is made here rather than by the child: the grandchild is
/// started by another process, and a child that had to create its own report
/// directory would be a child that could fail for a reason the test is not
/// about.
fn grandchild_report(directory: &Path) -> PathBuf {
    let report = report_path(&directory.join("grandchild"));
    fs::create_dir_all(report.parent().expect("a grandchild directory"))
        .expect("a grandchild directory");
    report
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

/// The child that stays alive until a file says it may go, and says first that
/// it is there.
///
/// Two files at two moments, and the pair is the whole instrument.
/// [`started_path`] is written **before** the wait and is the positive control:
/// it is the only thing that tells "the stop reached this process" apart from
/// "this process never ran", and an assertion that a file is *absent* cannot
/// tell those apart on its own — both leave nothing behind. That is not
/// hypothetical here: a grandchild whose name did not match the filter it was
/// started with runs zero tests, exits cleanly and writes nothing, and the
/// "was it stopped?" assertion on Windows was satisfied by exactly that,
/// measured before this marker existed.
/// [`released_path`] is what a parent drops afterwards to ask whether this
/// process is still alive: one that survived notices within a poll and writes
/// its report, and one that was stopped cannot write anything at all.
///
/// Waiting on a file rather than sleeping for a fixed time, on purpose. A sleep
/// races the parent's deadline — it must be long enough not to finish early and
/// short enough to be worth waiting out, and on a fast machine the first of
/// those is what breaks. A release file takes the clock out of it: this process
/// is alive exactly until something stops it, whatever the machine did with the
/// time. The number it is given is the poll interval, not a duration to wait.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_waits_to_be_released() {
    let (given, number) = child_instructions();
    let mut lines = given.split('\n');
    let report = PathBuf::from(lines.next().expect("a report path"));

    fs::write(started_path(&report), "started\n").expect("the started marker");

    let release = released_path(&report);
    let poll = Duration::from_millis(number);
    let give_up = Instant::now() + ABANDONED;
    while !release.exists() {
        if Instant::now() >= give_up {
            // Nobody released this process, which means the test that started
            // it has already failed and is not waiting for an answer. Exiting
            // without a report is the honest outcome: nothing was measured.
            return;
        }
        std::thread::sleep(poll);
    }
    fs::write(&report, "finished\n").expect("the child's report");
}

/// The child that starts another process, and then stays alive itself.
///
/// Two processes deep, because one is not enough to test a claim about a tree.
/// The grandchild writes a marker when it starts and a report if it is ever
/// released, and the **pair** is how the parent reads the answer: the marker
/// says the tree went two deep and the grandchild was alive, and the report
/// says whether it is still alive now. Neither alone is enough, and the marker
/// is the half that was missing — a stop that reached a process and a process
/// that never existed leave the same nothing behind. No signal, no handle and
/// no platform-specific question is involved, which is the only way the answer
/// can be read on both platforms.
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
            "child_waits_to_be_released",
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

/// The child that says something that is not ASCII, on both streams.
///
/// Written as bytes rather than as a formatted string on purpose: the claim is
/// that what the program wrote is what SURE read, and a child that built its
/// output with `format!` would be reporting its own encoding decision rather
/// than the bytes it wrote.
#[test]
#[ignore = "spawned by the parent tests, not run on its own"]
fn child_says_something_not_ascii() {
    use std::io::Write;

    let _ = child_instructions();
    let mut stdout = std::io::stdout();
    stdout.write_all(NOT_ASCII.as_bytes()).expect("a write");
    stdout.flush().expect("a flush");
    let mut stderr = std::io::stderr();
    stderr.write_all(NOT_ASCII.as_bytes()).expect("a write");
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
fn a_program_at_a_path_with_a_space_and_unicode_is_the_program_that_runs() {
    // The test above holds the working directory. This holds the **program**,
    // which is a different position in the request and fails differently: a
    // working directory is handed to the operating system as a value of its
    // own, while a program path has to survive being turned into a command
    // line. On Windows the program and its arguments end up in one string that
    // the operating system splits again, so a path with a space in it starts
    // only if whoever built that string quoted it — and quoted it exactly once,
    // because a runner that quotes a path the process API was going to quote
    // itself has got it wrong in the other direction. That second failure is
    // not hypothetical: it is one of the mutations this module is held against,
    // and applied, this test fails in 0.01s with `os error 123`,
    // `ERROR_INVALID_NAME`, before any process is started.
    //
    // So the copy is placed in an awkward **directory** and given an awkward
    // **name**, because those are the two halves of "the path has a space in
    // it" and quoting fixes one of them.
    let parent = scratch("program-path");
    let directory = parent.join(AWKWARD);
    fs::create_dir_all(&directory).expect("a directory with an awkward name");

    // Windows starts a program by its extension, so the copy keeps one there and
    // does not need one on the platforms that mark a program with a mode bit
    // instead. `fs::copy` carries whichever of the two this source has, so the
    // copy is runnable for the same reason the original is — and if it were not,
    // the runner would refuse to start it and this test would fail on that
    // rather than pass for a reason nobody chose.
    let name = if cfg!(windows) {
        "a program with a space and \u{00e9}\u{4e2d}\u{6587}.exe"
    } else {
        "a program with a space and \u{00e9}\u{4e2d}\u{6587}"
    };
    let program = directory.join(name);
    let source = std::env::current_exe().expect("the test binary's own path");
    // Copied through a temporary name in the same directory and closed there,
    // then renamed onto `program`, so the path the runner is handed is one no
    // descriptor is open on. `fs::copy` here returns before the run and the file
    // is therefore already closed by then — but a descriptor is duplicated into
    // every child a `fork` makes, and the child is about to `execve` this path,
    // so "already closed by luck of ordering" is the weaker statement of the two.
    // See `sure_testkit::program`.
    sure_testkit::copy_program(&source, &program).expect("a copy of this test binary");

    // The report is what proves the copy **ran**: the child writes it into the
    // scratch directory, and nothing on this machine writes that file except a
    // process that reads the variable the child is given. The outcome is not
    // that evidence — it carries the request's program back to the caller, so
    // it says what was asked for rather than what ran, and it is asserted below
    // as the one thing about the copy that the outcome can speak to.
    let report = report_path(&parent);
    let outcome = run_child_at(
        &program,
        "child_reports_what_it_was_given",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the copied program runs");

    assert!(
        matches!(outcome.termination(), Termination::Exited { code: Some(0) }),
        "the copy should have run and ended cleanly; it ended with {:?}",
        outcome.termination()
    );
    // The outcome is about the copy rather than about some other path, which is
    // the whole of what it can say here: it is the request's program echoed
    // back, so it answers "which program is this outcome about", not "which file
    // did the operating system start".
    assert_eq!(
        Path::new(outcome.program()),
        program.as_path(),
        "the outcome is about a different program"
    );

    let reported = consequences(&report);
    let cwd = value(&reported, "cwd").expect("the copy reported its directory");
    assert_eq!(
        fs::canonicalize(Path::new(cwd)).expect("the copy's directory exists"),
        fs::canonicalize(&directory).expect("the directory we asked for exists"),
        "the copied program ran somewhere else"
    );

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn an_argument_that_is_not_ascii_arrives_as_one_argument_unchanged() {
    // The payload test above is about *splitting*: a shell-hostile string that
    // must stay one argument. This is about **encoding**, which fails without
    // splitting anything — a conversion that loses a character, truncates at a
    // surrogate pair, or re-encodes through a code page produces the right
    // number of arguments with the wrong bytes in them. An ASCII payload cannot
    // tell those two failures apart, and neither can an assertion that counts
    // arguments.
    let directory = scratch("non-ascii-argument");
    let report = report_path(&directory);

    let outcome = run_child(
        "child_reports_what_it_was_given",
        NOT_ASCII,
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    let reported = consequences(&report);
    let arguments = values(&reported, ARGUMENT_LINE);
    assert!(
        arguments.contains(&NOT_ASCII),
        "the argument should have arrived unchanged; the child saw {arguments:#?}"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_report_path_with_a_space_and_unicode_is_the_path_the_child_writes_to() {
    // The third of the four positions a path can occupy in a request, after the
    // program and the working directory: **inside a variable**. It is the one
    // where a path stops being a path and becomes an `OsString`, and on Windows
    // that is the environment block rather than the command line — a second
    // encoding surface, with its own conversion, its own length rule, and a
    // runner that is free to get one right and the other wrong.
    //
    // The assertion is deliberately the *file* rather than the string the child
    // read back. A child that reported the value it received would be reporting
    // what it read, and the claim is that the value it read is the value that
    // was sent: if the path arrived mangled, the write lands somewhere else and
    // there is no file to read. So the failure is loud and it cannot be
    // satisfied by echoing.
    let parent = scratch("non-ascii-variable");
    let directory = parent.join(AWKWARD);
    fs::create_dir_all(&directory).expect("a directory with an awkward name");
    let report = directory.join("report with a space \u{00e9}\u{4e2d}\u{6587}.tsv");

    let outcome = run_child(
        "child_reports_what_it_was_given",
        "unused",
        // The child's working directory is the ordinary parent; only the report
        // path is awkward, so a pass cannot come from the directory test above
        // by accident.
        &parent,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    assert!(
        report.exists(),
        "the child did not write to {} — it wrote nowhere, or somewhere else",
        report.display()
    );
    let reported = consequences(&report);
    assert!(
        value(&reported, "cwd").is_some(),
        "the file at an awkward path is not a report the child wrote"
    );

    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn what_a_program_writes_outside_ascii_comes_back_as_the_bytes_it_wrote() {
    // The fourth surface, and the only one that is not about what SURE *sends*:
    // this is about what it *reads back*. The drain copies bytes and never
    // decodes, so the contract is that the bytes arrive exactly — which is a
    // claim a `String`-shaped capture could not make, because by then an
    // encoding decision has already been made on the child's behalf.
    //
    // Standard error is asserted by equality and standard output by `contains`,
    // and the asymmetry is the platform's rather than a convenience: libtest
    // shares the child's standard output and prints its own summary there when
    // the child returns normally, so equality on standard output would be
    // asserting libtest's output as well as the child's. Standard error is not
    // shared, so equality there is exact.
    let directory = scratch("non-ascii-output");
    let report = report_path(&directory);

    let outcome = run_child(
        "child_says_something_not_ascii",
        "unused",
        &directory,
        Environment::inherited().with(CHILD_ENV, instructions(&report, 0)),
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    )
    .expect("the child runs");
    assert!(matches!(outcome.termination(), Termination::Exited { .. }));

    assert_eq!(
        outcome.stderr().bytes(),
        NOT_ASCII.as_bytes(),
        "standard error did not come back as the bytes the child wrote"
    );
    assert!(
        outcome
            .stdout()
            .bytes()
            .windows(NOT_ASCII.len())
            .any(|window| window == NOT_ASCII.as_bytes()),
        "standard output did not contain the bytes the child wrote; it held {:?}",
        outcome.stdout().text_lossy()
    );
    // Nothing was thrown away to get there, which is the part a truncating
    // conversion would get wrong without failing either assertion above on a
    // stream short enough to fit the bound.
    assert_eq!(outcome.stderr().discarded_bytes(), 0);
    assert!(outcome.stderr().unfinished().is_none());

    let _ = fs::remove_dir_all(&directory);
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

/// Start a child that starts a grandchild, stop it, and ask the grandchild
/// whether it is still there.
///
/// The asking is the same on both platforms and involves nothing but the file
/// system: the grandchild was alive when the stop was asked for (asserted here,
/// because every caller needs it and a caller that forgot would be reading an
/// absence that proves nothing), and afterwards it is either released and
/// writes or it is gone and cannot.
fn a_grandchild_after_the_stop(
    directory: &Path,
    grandchild: &Path,
    limits: Limits,
    cancellation: Cancellation,
) -> Outcome {
    let outcome = run_child(
        "child_starts_a_grandchild",
        "unused",
        directory,
        Environment::inherited().with(CHILD_ENV, instructions(grandchild, POLL_INTERVAL)),
        limits,
        cancellation,
    )
    .expect("the child starts");

    assert!(
        started_path(grandchild).exists(),
        "the grandchild never started, so this run says nothing about whether a stop reaches a \
         tree — a stop that reached it and a grandchild that never existed leave the same nothing \
         behind, which is why the marker is asserted before the absence is read"
    );

    // The question is asked by giving the grandchild a way to answer: a process
    // that is still running writes within a poll, and a process that was
    // stopped cannot write at all.
    fs::write(released_path(grandchild), "go\n").expect("the release");
    std::thread::sleep(RELEASE_WAIT);
    outcome
}

/// Assert where the grandchild stands, against what this platform can do.
///
/// Not a skip on either branch. The Unix side asserts the **opposite** outcome,
/// so the day a Unix build can reach a process tree this fails and the report
/// has to change on purpose rather than being discovered by a reader.
fn assert_what_the_platform_could_reach(stopped: Stop, grandchild: &Path) {
    if cfg!(windows) {
        assert_eq!(
            stopped,
            Stop::WholeTree,
            "the stop was reported as {stopped:?} on a platform that reaches a tree"
        );
        assert!(
            !grandchild.exists(),
            "the run reported that it stopped the whole tree, and the grandchild it was given \
             time to answer wrote its report anyway — so the report was wrong"
        );
    } else {
        assert_eq!(
            stopped,
            Stop::ProcessOnly,
            "the stop was reported as {stopped:?} on a platform expected to reach only the \
             process itself"
        );
        assert!(
            grandchild.exists(),
            "this platform was expected to reach only the process itself, and the grandchild was \
             stopped anyway — `Stop::ProcessOnly` is now an understatement and the report should \
             say so"
        );
    }
}

#[test]
fn a_stopped_run_reaches_what_the_run_started_or_says_that_it_did_not() {
    // `Stop::WholeTree` is a claim, and a claim nothing holds is the failure
    // this repository goes looking for. The test above asserts which variant
    // comes back; this one asserts whether it is **true** — the difference
    // between a runner that stops a tree and a runner that says it did.
    let directory = scratch("process-tree");
    let grandchild = grandchild_report(&directory);

    let outcome = a_grandchild_after_the_stop(
        &directory,
        &grandchild,
        Limits::new(TREE_DEADLINE, 64 * 1024, 64 * 1024),
        Cancellation::new(),
    );

    let stopped = match outcome.termination() {
        Termination::TimedOut { stopped } => stopped,
        other => panic!("expected the deadline to stop the run, got {other:?}"),
    };
    assert_what_the_platform_could_reach(stopped, &grandchild);

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn a_cancelled_run_reaches_what_it_started_too() {
    // The stop above is the runner's own clock deciding. This one is somebody
    // else deciding, and it is a separate claim: a caller that asks for a run
    // to stop needs the same tree reached as a deadline that expires, and one
    // of the two reaching it is not evidence about the other. The two are
    // separate tests rather than one with two branches because they are
    // separate promises.
    //
    // The cancellation is asked for **after the grandchild is up**, which is
    // what makes the answer mean something: cancelling a hundred milliseconds
    // in would race the two process starts, and a machine that lost that race
    // would report a tree that was never there. So the cancelling thread waits
    // for the marker the grandchild writes, and the wait has its own bound.
    let directory = scratch("process-tree-cancelled");
    let grandchild = grandchild_report(&directory);

    let cancellation = Cancellation::new();
    let cancelled_by = cancellation.clone();
    let marker = started_path(&grandchild);
    let canceller = std::thread::spawn(move || {
        let give_up = Instant::now() + ABANDONED;
        while !marker.exists() && Instant::now() < give_up {
            std::thread::sleep(Duration::from_millis(10));
        }
        cancelled_by.cancel();
    });

    let outcome = a_grandchild_after_the_stop(
        &directory,
        &grandchild,
        // Patient, because the cancellation is what should end this run; a
        // deadline that fired first would make the test about the wrong stop.
        Limits::new(PATIENT, 64 * 1024, 64 * 1024),
        cancellation,
    );
    canceller.join().expect("the cancelling thread");

    let stopped = match outcome.termination() {
        Termination::Cancelled { stopped } => stopped,
        other => panic!("expected the cancellation to stop the run, got {other:?}"),
    };
    assert_what_the_platform_could_reach(stopped, &grandchild);

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
    // A batch file is a program: Windows starts `cmd.exe` to run it, and this
    // test asserts that it ran. So it goes in through `sure_testkit::write_program`
    // — written beside its own name, closed and renamed onto it — rather than
    // straight to the path the runner is about to be handed.
    sure_testkit::write_program(&program, b"@echo off\r\necho it ran > ran.txt\r\n", 0o755)
        .expect("a batch file");

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
    // Not started by this test — that is what it asserts — but it is a batch
    // file, so it is a program Windows would run if the completion rule went the
    // other way, and it is put at its path by the same door as the one above.
    sure_testkit::write_program(&batch, b"@echo off\r\necho it ran > ran.txt\r\n", 0o755)
        .expect("a batch file");

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
    // Named to the runner and refused by it, because a `.ps1` is not an image.
    // Written through `sure_testkit::write_program` all the same: this is a
    // script the test asserts did **not** run, and a file that is complete and
    // closed at the path from the moment it exists is what makes that assertion
    // about the runner's decision rather than about timing.
    sure_testkit::write_program(
        &program,
        b"Set-Content -Path ran.txt -Value 'it ran'\r\n",
        0o755,
    )
    .expect("a script");

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

// ---------------------------------------------------------------------------
// The same request, on the three platforms. `P3-T003`.
//
// The claim these hold is one sentence: SURE hands the operating system a name
// and a vector of bytes, the platform decides what a name may be and how a file
// is run, and SURE does not decide for either of them. What it must not do is
// refuse a name the platform would have accepted, repair one the platform would
// have refused, or turn any of it into text on the way through.
//
// Each test runs on the platform it is about, which is what makes the coverage
// a fact about a CI run rather than a sentence in a document. The table in the
// module documentation is the index; read a run against it by test name.
//
// **The three are not two families plus a rounding error.** The first version of
// this section treated Linux and macOS as one thing and was wrong about the last
// row — macOS refuses a file name that is not valid UTF-8 before SURE is asked
// anything — and CI said so on the first run. Where the columns agree they agree
// because a test on each says so, not because `unix` was taken to mean it.
// ---------------------------------------------------------------------------

/// The smallest program a Unix test can put in front of the runner.
///
/// A file whose first line names an interpreter and whose body does nothing but
/// copy its first argument to the path in its second. `printf '%s'` rather than
/// `echo`, because an argument under test may be bytes that are not text and
/// `echo` is free to do what it likes with them.
#[cfg(unix)]
const A_SCRIPT: &[u8] = b"#!/bin/sh\nprintf '%s' \"$1\" > \"$2\"\n";

/// Write [`A_SCRIPT`] at `program` and give it `mode`.
///
/// The mode is set rather than left to the umask, because these tests are
/// *about* the executable bit and a premise the environment picks is not a
/// premise. The failure message is written out rather than kept short: this is
/// the one thing here that cannot be measured on the machine this branch is
/// developed on, so a platform that refuses the file has to say which platform
/// and why rather than leaving a reader with a bare `unwrap`.
///
/// **The mode is set on the file that is being written, before it is renamed
/// onto `program`** — [`sure_testkit::write_program`] does both — so the script
/// is executable at its own path from the instant that path names it, and never
/// in between. The script ends up being started by the operating system, which
/// is the whole of what these tests ask, so its path is the one path here that
/// most has to be complete and closed before anything looks at it.
#[cfg(unix)]
fn write_a_script(program: &Path, mode: u32) {
    sure_testkit::write_program(program, A_SCRIPT, mode).unwrap_or_else(|error| {
        panic!(
            "the program this test is about could not be put at {} with mode {mode:o}, so this \
             platform cannot be asked the question this test asks: {error:?}",
            program.display()
        )
    });
}

#[test]
#[cfg(unix)]
fn a_script_with_an_interpreter_line_runs_when_it_has_the_executable_bit() {
    // The Unix half of the `.cmd` tests above, and a different mechanism with
    // the same consequence: SURE names a file and assembles no command line,
    // and the operating system decides how a named file runs. On Windows that
    // decision is the extension, plus the interpreter Windows supplies for a
    // batch file; here it is the interpreter line inside the file, which SURE
    // never reads.
    //
    // The name has no extension on purpose. On this platform a name is a path
    // and nothing else, and a runner that had learned a rule about extensions
    // would be a runner that had learned Windows'.
    let directory = scratch("unix-executable-script");
    let report = directory.join("report.txt");
    let program = directory.join("says-something");
    write_a_script(&program, 0o755);

    let request = ProcessRequest::new(
        program,
        directory.clone(),
        Limits::new(PATIENT, 4096, 4096),
        Cancellation::new(),
    )
    .with_arguments([OsString::from("it ran"), report.clone().into_os_string()]);

    // This is the one place in the file where the test can fail for a reason
    // that belongs to the machine rather than to the runner: a filesystem that
    // lets the file be written and its mode set and will not let anything be
    // executed on it — which is how some container images mount `/tmp` — makes
    // the premise above true on disk and the run fail anyway. Naming that here
    // is the difference between a reader who knows which of the two happened
    // and one who reads a bare `expect` as the runner being broken.
    let outcome = sure_core::process::run(&request).expect(
        "the script runs, or this platform will not execute a file in the directory this test was \
         able to write one to, which is a fact about the mount and not about the runner",
    );

    assert_eq!(
        outcome.termination(),
        Termination::Exited { code: Some(0) },
        "a script that ran to the end reports the code it ended with"
    );
    let written = fs::read_to_string(&report).unwrap_or_else(|error| {
        panic!(
            "nothing was written by the script, so it did not run: {error}. The script writes to \
             the path in its second argument, so an absent report would also mean the arguments \
             did not arrive."
        )
    });
    assert_eq!(written, "it ran", "the report is the script's own output");

    let _ = fs::remove_dir_all(&directory);
}

#[test]
#[cfg(unix)]
fn the_same_script_without_the_executable_bit_is_not_a_program() {
    // The negative half, and the half that is about SURE rather than about the
    // operating system: the runner does not make a file runnable so that a
    // request can succeed. No `sh script`, no `chmod`, no interpreter of SURE's
    // own choosing — "the platform will not run this" is reported as what it
    // is, and a caller that meant to run it has to hear about it.
    //
    // The file is otherwise perfect: same bytes as the test above, same
    // interpreter line, same arguments. The mode is the only difference, which
    // is what makes this test about the mode.
    let directory = scratch("unix-not-executable");
    let report = directory.join("report.txt");
    let program = directory.join("says-something");
    write_a_script(&program, 0o644);

    let request = ProcessRequest::new(
        program.clone(),
        directory.clone(),
        Limits::new(PATIENT, 4096, 4096),
        Cancellation::new(),
    )
    .with_arguments([OsString::from("it ran"), report.clone().into_os_string()]);

    match sure_core::process::run(&request) {
        Err(ProcessError::NotStarted {
            program: reported, ..
        }) => assert_eq!(
            reported,
            program.into_os_string(),
            "the error must name the program it was given"
        ),
        other => panic!(
            "a file with no executable bit was expected not to start, and this was {other:?}"
        ),
    }
    assert!(
        !report.exists(),
        "the run was reported as not started and the script ran anyway, which would mean the \
         report was wrong about the thing the product most needs it to be right about"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
#[cfg(target_os = "linux")]
fn a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    // The sharpest difference between the families, and the question `P3-T002`
    // left open. A Windows path is UTF-16, so a name that is not valid Unicode
    // cannot exist on that side at all and `a_program_name_that_is_not_a_windows_path_is_refused_rather_than_mangled`
    // pins the refusal. A Unix path is bytes, and the bytes are the name: the
    // file is created with a name that is not valid UTF-8, named to the runner,
    // and run.
    //
    // **This is `target_os = "linux"` and not `unix`, and that was learned from
    // CI rather than reasoned about.** macOS holds those bytes in an `OsStr` as
    // happily as Linux does — the assertion style here works there — and its
    // filesystem will not have a file by that name: `fs::write` answers
    // `EILSEQ`, errno 92, "Illegal byte sequence". Run `34929385200` is where
    // this test first ran on macOS and said exactly that, at the `write_a_script`
    // call below. So the row has two answers on the Unix side and this is the
    // Linux one; `a_program_whose_name_is_not_valid_utf8_cannot_be_created_on_macos`
    // is the other.
    //
    // What is being checked is that nothing on the way turned any of it into
    // text. A `String` anywhere in the path — a `to_str`, a `to_string_lossy`,
    // a `display()` reused to build something — replaces those bytes, and the
    // replacement is either a refusal or, worse, a *different* file. Both are
    // visible here: the report has to hold the argument back byte for byte, and
    // the outcome has to name the program it was given.
    let directory = scratch("unix-not-text");
    let report = directory.join("report.bin");
    let program = directory.join(OsString::from_vec(b"says-\xff\xfe-something".to_vec()));
    write_a_script(&program, 0o755);

    let payload = OsString::from_vec(b"an argument \xff\xfe outside UTF-8".to_vec());

    let request = ProcessRequest::new(
        program.clone(),
        directory.clone(),
        Limits::new(PATIENT, 4096, 4096),
        Cancellation::new(),
    )
    .with_arguments([payload.clone(), report.clone().into_os_string()]);

    let outcome = sure_core::process::run(&request).expect("the program runs");

    assert_eq!(
        outcome.termination(),
        Termination::Exited { code: Some(0) },
        "the program that ran has to be the one that was named"
    );
    assert_eq!(
        outcome.program().as_bytes(),
        program.as_os_str().as_bytes(),
        "the outcome must carry the program back as the bytes it was given"
    );
    let written = fs::read(&report).unwrap_or_else(|error| {
        panic!(
            "nothing was written by the program, so it did not run: {error}. The program writes \
             to the path in its second argument, so an absent report would also mean the \
             arguments did not arrive."
        )
    });
    assert_eq!(
        written,
        payload.as_bytes(),
        "the argument has to arrive as the bytes it is; anything else is a conversion"
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
#[cfg(target_os = "macos")]
fn a_program_whose_name_is_not_valid_utf8_cannot_be_created_on_macos() {
    use std::os::unix::ffi::OsStringExt;

    // The macOS answer to the row above, and **it is not an answer about the
    // runner** — which is the whole reason it is a test rather than a note.
    // macOS is `unix`, `OsString::from_vec` gives it the same bytes, and its
    // filesystem will not hold a file whose name is not valid UTF-8: the create
    // returns `EILSEQ`. There is no request to make, so the claim "SURE hands
    // the bytes through" has no macOS half and the table above says so.
    //
    // **This test asserts a premise rather than a behaviour, and it is here so
    // that the premise cannot rot quietly.** If a later macOS or a later
    // filesystem accepts the name, this fails and the row has to be rewritten
    // on purpose — the same reason `assert_what_the_platform_could_reach` states
    // the platform's answer instead of skipping it. What it does *not* do is
    // check anything SURE wrote: `fs::write` is the standard library's, and the
    // only thing under test here is the operating system.
    //
    // The errno is asserted as a number and the sentence is not. `EILSEQ` is 92
    // on Darwin and 84 on Linux, so the number is this platform's and not a
    // portable constant — and unlike Windows' localized text it is not
    // translated, which is what makes it the part worth pinning.
    let directory = scratch("macos-not-text");
    let bytes = OsString::from_vec(b"says-\xff\xfe-something".to_vec());

    // The control comes **first**, and it is what makes the assertion about the
    // name rather than about the directory. The same bytes of program, in the
    // same directory, under a name the filesystem will take: without this, a
    // scratch path that was never created or a permission problem would satisfy
    // the assertion below just as well, and for the wrong reason.
    fs::write(directory.join("an ordinary name"), A_SCRIPT)
        .expect("the control: an ordinary name in the same directory is created");

    let refused = fs::write(directory.join(&bytes), A_SCRIPT);
    let errno = refused
        .as_ref()
        .err()
        .and_then(std::io::Error::raw_os_error);

    assert_eq!(
        errno,
        Some(92),
        "macOS was expected to refuse a file name that is not valid UTF-8 with EILSEQ (92), and \
         this is {refused:?}. If the name was accepted, the row above has a macOS half and this \
         test has to be rewritten; if it was refused with a different error, this test's number \
         is what is wrong."
    );

    let _ = fs::remove_dir_all(&directory);
}

#[test]
#[cfg(windows)]
fn a_program_name_that_is_not_a_windows_path_is_refused_rather_than_mangled() {
    use std::os::windows::ffi::OsStringExt;

    // The other side of the Unix test above. A Windows path is UTF-16, so a
    // name that is not valid Unicode is not a name this platform has: there is
    // no file to find and no search that could find one. `0xD800` is a high
    // surrogate with nothing after it, so it is not half of a pair.
    //
    // **What Windows says about that name was measured rather than predicted,
    // and the prediction was wrong.** The guess was `ERROR_INVALID_NAME` (123),
    // the error a path parser gives a path it cannot read. The measurement is
    // `ERROR_FILE_NOT_FOUND` (2) — the answer a name with nothing at it gets —
    // because the name reaches the file system as a name rather than being
    // rejected as syntax. That is asserted here as a *comparison* rather than
    // as a number: the same request with an ordinary name that is not there has
    // to come back with the same answer. The number is not written into the
    // test because the operating system's sentence is localized — this machine
    // answers in Chinese, and only the `(os error N)` suffix is stable — and
    // because the claim is the comparison, not the constant.
    let directory = scratch("not-a-windows-path");
    let limits = Limits::new(PATIENT, 4096, 4096);

    let program = directory.join(OsString::from_wide(&[0xD800, 0x0041]));
    let expected = program.clone().into_os_string();
    let unspellable = sure_core::process::run(&ProcessRequest::new(
        program,
        directory.clone(),
        limits,
        Cancellation::new(),
    ));

    let ordinary = directory.join("no-such-program-here");
    let missing = sure_core::process::run(&ProcessRequest::new(
        ordinary,
        directory.clone(),
        limits,
        Cancellation::new(),
    ));

    // Two things are asserted, and they are the two ways a runner could be
    // wrong about it. It must refuse, and the refusal must carry the name back
    // **as it was given**: a `to_string_lossy` on the way to the message would
    // replace the surrogate with U+FFFD, and a caller comparing the name it
    // sent with the name in the error would be reading a different path.
    let (program, message) = match unspellable {
        Err(ProcessError::NotStarted {
            program, message, ..
        }) => (program, message),
        other => panic!(
            "a name that is not a Windows path was expected not to start, and this was {other:?}"
        ),
    };
    assert_eq!(
        program, expected,
        "the error must carry the name back as it was given rather than as text"
    );

    let control = match missing {
        Err(ProcessError::NotStarted { message, .. }) => message,
        other => panic!(
            "the control — an ordinary name with nothing at it — was expected not to start, and \
             this was {other:?}"
        ),
    };
    assert_eq!(
        message, control,
        "a name that is not valid Unicode and a name with nothing at it have to be the same \
         answer, or the difference is something the operating system did not make"
    );

    let _ = fs::remove_dir_all(&directory);
}

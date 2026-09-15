//! What `sure_core::service` claims, checked by starting and stopping real
//! processes.
//!
//! The acceptance is *"Can start/stop supported local services with timeouts and
//! captured logs."* Each word of that is a claim about a boundary rather than
//! about a type: *started* means a process the operating system accepted,
//! *stopped* means a process that is gone, *with timeouts* means a run that ends
//! because its budget ran out, and *captured logs* means the bytes the process
//! wrote arriving in the outcome. So every test here runs something and asserts
//! on what happened to it, and none of them inspects a `Service` field and calls
//! that a start.
//!
//! # Why the program is a copy of this test binary, named `python`
//!
//! A service can only be started from an [`AdmittedCommand`], and a command is
//! only admitted if [`sure_core::safety`] classifies it — and classification
//! reads the **last path component** of the program. This test binary is called
//! `service_supervisor-<hash>.exe`, which classifies as an unknown program, which
//! [`sure_core::safety`] deliberately treats as *anything*: every permission
//! including the destructive ones, so no mode admits it, and it could not be
//! started through this module at all. That is the classifier working as
//! designed, and it is also why the tests here copy the binary to a file named
//! `python.exe` (or `python`) first: the child is still this binary, run again,
//! but the name is one the classifier reads as *runs the project's code*.
//!
//! **That a name can be moved from one file to another is a real limitation and
//! it is not this file's discovery** — `safety.rs` documents the same window
//! about `PATH`, and nothing here closes it. What this file is careful about is
//! that nothing it claims *depends* on the name being honest: the copy is made
//! inside the test, the path it is made at is the path that is classified, and
//! what is asserted afterwards is about the process that ran.
//!
//! # What is not claimed here
//!
//! **Nothing about readiness.** A start that returns `Ok` means a process is
//! running and SURE is reading it. Whether anything is listening, answering, or
//! even still alive is `P3-T010`'s probe, and this file asserts the absence of
//! that claim rather than leaving it to be assumed: the service the first test
//! starts is one that has written nothing but has a marker on disk, and the test
//! says so in those terms.
//!
//! **Nothing about stopping a process *tree*.** `process_runner.rs` owns that
//! claim, with a grandchild as its instrument. The services here are single
//! processes, and the stop's reach is the runner's contract rather than this
//! layer's.
//!
//! # The instrument that makes "it was stopped" a fact
//!
//! Every child here waits to be released, and writes its report **only** if it
//! was released. So a report file that does not exist after a stop is evidence
//! that the child never reached the end of its work — and the marker file it
//! writes before it starts waiting is evidence that it was running when the stop
//! arrived. Without both halves, "the report is missing" would be satisfied by a
//! child that never started, which is the failure mode these tests exist to
//! catch rather than to have.
//!
//! A child that waits is also a child that can be left on the machine by a
//! failing test, so it gives up after [`ABANDONED`] and says so in a file of its
//! own — which is the third state, and the one that must not be confused with
//! either of the other two. See [`child_serves_until_released`].

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sure_core::consent::{PermissionPlan, PlannedCheck};
use sure_core::enforce::Enforcement;
use sure_core::process::{Limits, ProcessError, Termination};
use sure_core::service::{ServiceError, Supervisor};
use sure_domain::execution::{ExecutionMode, ExecutionPermissions, Permission};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::NotCheckedReason;

/// A generous deadline for anything a test is waiting on that is expected to
/// happen.
///
/// Generous on purpose: a machine under load must not fail a test that is about
/// something else. The tests that are *about* a deadline use a short budget and
/// say so where they use it.
const PATIENT: Duration = Duration::from_secs(60);

/// The whole-life budget of a service a test intends to stop itself.
///
/// Long enough that the stop always arrives first — the alternative is a test
/// whose outcome depends on which of two things happened sooner, and the
/// timeout test below is the one test that is *about* that race.
const LONG_ENOUGH: Duration = Duration::from_secs(120);

/// The budget of a service that is meant to run out.
///
/// Two seconds, and the number is chosen against the child rather than against
/// the machine: the child waits [`ABANDONED`] before giving up on its own, which
/// is fifteen times this, so a run that comes back `TimedOut` came back because
/// of this budget and not because the child quit.
const BRIEF: Duration = Duration::from_secs(2);

/// How many bytes of each stream a test's services keep.
const ROOM: usize = 64 * 1024;

/// How often a waiting child looks for its release file.
///
/// Milliseconds, and the unit is in the name because the number travels through
/// the same channel as everything else a child is told. It is also the
/// resolution of the heartbeat below, which is why it is a constant rather than
/// a literal: two observations of the heartbeat four of these apart are what
/// "the child has stopped" is measured with.
const POLL_INTERVAL: u64 = 25;

/// How long a waiting child waits for a release that may never come.
///
/// A child whose parent failed its own test is a child nobody will release, and
/// a child that waits forever is a process left on the machine by a failing
/// test. Thirty seconds bounds it without ever reaching it: a released child
/// writes within one poll, and this is twelve hundred of them.
const ABANDONED: Duration = Duration::from_secs(30);

/// How long a stopped run gets to prove a released child is gone.
///
/// The child polls every [`POLL_INTERVAL`], so a living child writes within a
/// small multiple of that; a second is forty of them. This is a wait for
/// something that must **not** happen, which is the only kind of wait that has
/// to be bounded by the observer rather than by the observed.
const RELEASE_WAIT: Duration = Duration::from_secs(1);

/// What a child says on each stream before it starts waiting.
///
/// Distinct sentences rather than one sentence twice: the assertion is that each
/// arrived on its own stream, and the same text on both would be satisfied by a
/// runner that read one pipe twice.
const SAID_ON_STDOUT: &str = "the service is up on standard output";
const SAID_ON_STDERR: &str = "the service is up on standard error";

/// A directory name that is ordinary on both platforms and mangles easily.
///
/// One spelling, so that "a path with a space and a character outside ASCII"
/// means the same thing here as it does in `process_runner.rs`. **The space is
/// the half that breaks first**: a path is re-split on whitespace by anything
/// that turns a path into a command line, and a service is a program, a working
/// directory and a vector of arguments that all have to arrive whole.
const AWKWARD: &str = "a service directory with \u{00e9}\u{4e2d}\u{6587}";

// ---------------------------------------------------------------------------
// The harness.
// ---------------------------------------------------------------------------

/// A scratch directory for one test, named after the test and this process.
///
/// Not removed on entry: a directory left behind by a killed run is evidence.
/// Removed on the way out, so repeated runs stay independent.
fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("sure-service-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

/// A copy of this test binary, named so that classification reads it as
/// `python`, at `directory`.
///
/// The name is the whole trick and the module documentation is where it is
/// explained. `.exe` on Windows and nothing on the platforms that mark a program
/// with an execute bit instead, because those are the two rules
/// [`sure_core::safety`] applies to the name it is given — `std::fs::copy`
/// carries the permission bits across, so the copy is executable where that is
/// what matters.
fn as_python(directory: &Path) -> PathBuf {
    let name = if cfg!(windows) {
        "python.exe"
    } else {
        "python"
    };
    let path = directory.join(name);
    fs::copy(
        std::env::current_exe().expect("the test binary's own path"),
        &path,
    )
    .expect("a copy of the test binary");
    path
}

/// Every permission granted, so that a refusal in a test is the *service's*
/// answer and not the permission set's.
fn everything() -> ExecutionPermissions {
    let mut permissions = ExecutionPermissions::inspect_only();
    for permission in Permission::ALL {
        permissions.set(*permission, true);
    }
    permissions
}

/// An enforcement that has decided about exactly one command.
///
/// The plan holds one check with one command and the schedule holds that same
/// check, which is the smallest plan the product's own vocabulary admits. It is
/// built here rather than reached through `sure check` because the wiring is
/// `P4`'s work and this file is about what the service layer does with a
/// decision it is handed.
fn admits(
    program: &Path,
    arguments: &[String],
    mode: ExecutionMode,
    permissions: ExecutionPermissions,
) -> Enforcement {
    let check = PlannedCheck::new(CheckId::generate(), "the service", Severity::MustFix, true);
    let mut plan = PermissionPlan::new(mode, FingerprintId::generate(), permissions);
    plan.add(check.clone(), program.as_os_str(), arguments.to_vec());
    Enforcement::of("the service", plan, std::slice::from_ref(&check))
}

/// A supervisor for `directory`, with `limits`.
fn supervisor(directory: &Path, limits: Limits) -> Supervisor {
    Supervisor::new(directory, limits)
}

/// The budgets a test's services are held to.
fn limits(budget: Duration) -> Limits {
    Limits::new(budget, ROOM, ROOM)
}

/// The arguments that turn a copy of the test binary into one of the children
/// below, with `payload` carried in the argument vector.
///
/// `--exact`, so the filter cannot match another test whose name starts the same
/// way; `--ignored`, because a child is not a test on its own; `--nocapture`, so
/// what the child prints reaches the stdout the service captured rather than
/// libtest's capture buffer; and the payload as libtest's `--skip`, which
/// matches nothing and removes nothing.
///
/// **The payload travels in the argument vector because the environment is not
/// available.** A service is started with [`sure_core::process::Environment::inherited`]
/// — the service module says why — so the parent cannot hand a child a variable,
/// and it cannot set one for itself either: `std::env::set_var` is `unsafe` in
/// edition 2024 and this workspace forbids unsafe code outright. So the child
/// reads its instructions out of its own `argv`, which is the technique
/// `process_runner.rs` uses for a different reason.
fn child_arguments(mode: &str, report: &Path, number: u64) -> Vec<String> {
    vec![
        String::from("--exact"),
        mode.to_owned(),
        String::from("--ignored"),
        String::from("--quiet"),
        String::from("--nocapture"),
        String::from("--skip"),
        format!("{}\n{number}", report.display()),
    ]
}

/// The path a child writes its report into, once it is released.
fn report_path(directory: &Path) -> PathBuf {
    directory.join("report.tsv")
}

/// The marker a waiting child writes the moment it is running, before it waits
/// for anything.
fn started_path(report: &Path) -> PathBuf {
    report.with_extension("started")
}

/// The file that tells a waiting child it may finish.
fn released_path(report: &Path) -> PathBuf {
    report.with_extension("release")
}

/// The file a waiting child gives up into.
///
/// The third state, and the one that must not be read as either of the others: a
/// child that hit [`ABANDONED`] was **alive and not released**, so an assertion
/// that "the report is missing" would hold while the thing it was meant to show
/// — that the child was stopped — is false.
fn abandoned_path(report: &Path) -> PathBuf {
    report.with_extension("abandoned")
}

/// The file a waiting child rewrites on every poll, with the count so far.
///
/// **This name only ever holds a finished heartbeat.** The child writes each one
/// beside it and renames it into place; see [`child_serves_until_released`] for
/// the kill that makes the difference matter.
fn heartbeat_path(report: &Path) -> PathBuf {
    report.with_extension("beating")
}

/// Where the next heartbeat is written before it is renamed into place.
///
/// A file a reader never looks at, which is the point: what it holds may be half
/// of a write, and nothing depends on that.
fn heartbeat_in_progress_path(report: &Path) -> PathBuf {
    report.with_extension("beating-part")
}

/// Wait until `condition` holds, or `limit` passes, and say which it was.
fn wait_until(limit: Duration, condition: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL));
    }
    condition()
}

/// Wait until `path` stops changing, and hand back what it last held.
///
/// `None` means it was still changing when `limit` passed, which is a fact about
/// the thing being watched rather than a timeout to be swallowed. Two
/// observations [`POLL_INTERVAL`] × 4 apart are equal only when nothing is
/// writing, and the interval is the child's own: four of them is time for a
/// living writer to have written again with room to spare.
fn wait_until_quiet(path: &Path, limit: Duration) -> Option<String> {
    let deadline = Instant::now() + limit;
    let mut previous = fs::read_to_string(path).unwrap_or_default();
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL * 4));
        let now = fs::read_to_string(path).unwrap_or_default();
        if now == previous {
            return Some(now);
        }
        previous = now;
    }
    None
}

/// Read a child's report as `key -> value`.
fn consequences(path: &Path) -> Vec<(String, String)> {
    let text = fs::read_to_string(path).expect("the child's report");
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// The one value reported for one key.
fn value(report: &[(String, String)], key: &str) -> Option<String> {
    report
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

// ---------------------------------------------------------------------------
// The children. Each is ignored, so a normal run does not execute it, and each
// is reached by a service with `--ignored --exact`.
// ---------------------------------------------------------------------------

/// The payload a child was given, read out of its own argument vector.
fn child_payload() -> String {
    let mut arguments = std::env::args().skip_while(|argument| argument != "--skip");
    arguments.next();
    arguments
        .next()
        .expect("the child was given its instructions")
}

/// Where a child writes, and the number it writes about.
fn child_report_and_number() -> (PathBuf, u64) {
    let payload = child_payload();
    let (report, number) = payload
        .split_once('\n')
        .expect("a report path and a number, on two lines");
    (
        PathBuf::from(report),
        number.parse().expect("a number on the second line"),
    )
}

/// A service that comes up, says so on both streams, and then waits.
///
/// **It writes its report only if it was released**, and that is the whole
/// instrument: a report that does not exist after a stop is the difference
/// between a service that was stopped and one that finished. The marker it
/// writes first is the other half — it is evidence that the child was *running*
/// when the stop arrived, without which a missing report would prove nothing.
///
/// It gives up after [`ABANDONED`] and says so in a file of its own, because a
/// child that waits forever is a process left on the machine by a failing test.
/// The give-up file exists so that "not released" cannot be mistaken for
/// "stopped" — the two are indistinguishable from the report alone.
#[test]
#[ignore = "started by the service tests, not run on its own"]
fn child_serves_until_released() {
    let (report, number) = child_report_and_number();

    // Before anything else, so that the streams are captured whatever happens
    // next.
    println!("{SAID_ON_STDOUT}");
    eprintln!("{SAID_ON_STDERR}");
    fs::write(started_path(&report), b"up").expect("the marker");

    let deadline = Instant::now() + ABANDONED;
    let mut polls: u64 = 0;
    loop {
        if released_path(&report).exists() {
            fs::write(&report, format!("number\t{number}\n")).expect("the report");
            return;
        }
        if Instant::now() >= deadline {
            fs::write(abandoned_path(&report), b"nobody released it").expect("the give-up file");
            return;
        }
        polls += 1;
        // **Written aside and renamed into place, so that no reader ever sees a
        // heartbeat that was cut in half.** `fs::write` truncates before it
        // writes, and the test that reads this file reads it *after* stopping
        // this process: a kill landing between the truncate and the write would
        // leave a heartbeat that is empty, and empty is the one reading that
        // means "this child never got going" — a false sentence about a child
        // that had been polling. A rename is atomic on both platforms, so the
        // file under [`heartbeat_path`] is either the previous heartbeat or the
        // next one and never a state in between.
        let in_progress = heartbeat_in_progress_path(&report);
        fs::write(&in_progress, polls.to_string()).expect("the heartbeat");
        fs::rename(&in_progress, heartbeat_path(&report)).expect("the heartbeat in place");
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL));
    }
}

/// A service that reports where it ran and then ends with the code it was given.
///
/// The working directory is in the report because that is a claim the service
/// layer makes and this is the child that can hold it: a service is started *in*
/// a directory, and "in" is not visible in an outcome that only carries streams.
#[test]
#[ignore = "started by the service tests, not run on its own"]
fn child_ends_with_the_code_it_was_given() {
    let (report, number) = child_report_and_number();
    let directory = std::env::current_dir().expect("a working directory");
    fs::write(
        &report,
        format!("number\t{number}\ncwd\t{}\n", directory.display()),
    )
    .expect("the report");
    std::process::exit(number as i32);
}

// ---------------------------------------------------------------------------
// The claims.
// ---------------------------------------------------------------------------

#[test]
fn a_service_starts_is_stopped_and_both_of_its_streams_are_kept() {
    // The awkward directory is the working directory *and* where the program
    // lives, so one test holds the claim that a service's directory, program and
    // arguments all survive a path with a space and a character outside ASCII.
    let root = scratch("starts-and-stops");
    let directory = root.join(AWKWARD);
    fs::create_dir_all(&directory).expect("a working directory");
    let python = as_python(&directory);
    let report = report_path(&directory);
    let arguments = child_arguments("child_serves_until_released", &report, 11);

    let enforcement = admits(
        &python,
        &arguments,
        ExecutionMode::HostConfirmed,
        everything(),
    );
    let supervisor = supervisor(&directory, limits(LONG_ENOUGH));

    let service = supervisor
        .start(
            enforcement
                .admitted()
                .next()
                .expect("the one command this plan decided on"),
        )
        .expect("a service that starts");

    // The handle says what it was started from, which is the evidence a report
    // will carry — the command and the decision that let it run.
    assert_eq!(service.command().program(), python.as_os_str());
    assert_eq!(service.working_directory(), directory.as_path());

    // Readiness is not claimed: what is waited for is the marker the child
    // writes itself, and if it never appears the test says the service did not
    // come up rather than asserting something weaker.
    assert!(
        wait_until(PATIENT, || started_path(&report).exists()),
        "the service never got as far as writing its marker, so nothing here is \
         about stopping it"
    );
    assert!(
        !service.has_finished(),
        "the child waits to be released, so a service that has finished has \
         finished for a reason this test did not cause"
    );

    let outcome = service.stop().expect("a stop that is reported");

    assert!(
        matches!(outcome.termination(), Termination::Cancelled { .. }),
        "a service that was stopped reports the stop, not an ending of its own: {:?}",
        outcome.termination()
    );
    let stdout = outcome.stdout().text_lossy();
    let stderr = outcome.stderr().text_lossy();
    assert!(
        stdout.contains(SAID_ON_STDOUT),
        "standard output was captured and should contain what the child wrote: {stdout:?}"
    );
    assert!(
        stderr.contains(SAID_ON_STDERR),
        "standard error was captured and should contain what the child wrote: {stderr:?}"
    );

    // The stop is asked to prove itself. The child is released *after* the stop
    // has been reported, so a child that is still alive would write its report
    // within one poll of the release appearing — and a child that was stopped
    // writes nothing, in either file.
    fs::write(released_path(&report), b"released").expect("the release");
    std::thread::sleep(RELEASE_WAIT);
    assert!(
        !report_path(&directory).exists(),
        "the service wrote its report after it was stopped, so it was not stopped"
    );
    assert!(
        !abandoned_path(&report).exists(),
        "the child gave up rather than being stopped, which means the stop did \
         nothing and the child left on its own"
    );
}

#[test]
fn a_program_that_is_not_there_is_refused_at_the_start_and_not_returned_running() {
    // The program is a `python.exe` in a directory that is never created, and
    // the working directory is one that exists: two different absences, and this
    // test is about the first. Classification reads the *name*, so the plan
    // admits this command — `python` runs the project's code, whatever is or is
    // not at that path — and the operating system is what refuses it. Deciding
    // whether to run is the plan's business; starting is the operating
    // system's, and the service layer's job is to report which one said no.
    let root = scratch("not-there");
    let directory = root.join("working");
    fs::create_dir_all(&directory).expect("a working directory");
    let python = root.join("gone").join(if cfg!(windows) {
        "python.exe"
    } else {
        "python"
    });
    let report = report_path(&directory);

    let enforcement = admits(
        &python,
        &child_arguments("child_serves_until_released", &report, 12),
        ExecutionMode::HostConfirmed,
        everything(),
    );
    let supervisor = supervisor(&directory, limits(LONG_ENOUGH));

    let refused = supervisor
        .start(
            enforcement
                .admitted()
                .next()
                .expect("the one command this plan decided on"),
        )
        .expect_err("a program that is not there cannot be started");

    match refused {
        ServiceError::Runner(ProcessError::NotStarted { program, .. }) => {
            assert_eq!(program, python.as_os_str());
        }
        other => panic!(
            "a program that is not there is refused at the start, with the program \
             named: {other:?}"
        ),
    }
}

#[test]
fn a_service_that_ends_by_itself_reports_the_code_it_ended_with_and_where_it_ran() {
    let root = scratch("ends-by-itself");
    let directory = root.join("working");
    fs::create_dir_all(&directory).expect("a working directory");
    let python = as_python(&directory);
    let report = report_path(&directory);
    let arguments = child_arguments("child_ends_with_the_code_it_was_given", &report, 7);

    let enforcement = admits(
        &python,
        &arguments,
        ExecutionMode::HostConfirmed,
        everything(),
    );
    let supervisor = supervisor(&directory, limits(LONG_ENOUGH));

    let service = supervisor
        .start(
            enforcement
                .admitted()
                .next()
                .expect("the one command this plan decided on"),
        )
        .expect("a service that starts");

    // `has_finished` is the question a caller asks while holding a service, and
    // the answer is about the run rather than about success — a service that
    // died on its first line answers `true` too. What it ended as is the next
    // call's answer.
    assert!(
        wait_until(PATIENT, || service.has_finished()),
        "the child exits immediately, so a service that has not finished after \
         {PATIENT:?} is not running the child it was given"
    );

    let outcome = service.stop().expect("a stop that is reported");

    assert_eq!(
        outcome.termination(),
        Termination::Exited { code: Some(7) },
        "a service that ends by itself reports the code the program exited with"
    );
    let report = consequences(&report);
    assert_eq!(value(&report, "number").as_deref(), Some("7"));

    // Compared as **resolved paths rather than as strings**, which is the idiom
    // `process_runner.rs` already uses for the same claim, and which this
    // assertion was first written without. **That made it red on macOS and green
    // everywhere else, for a reason that had nothing to do with the service**: the
    // child reports `std::env::current_dir()`, which is the directory with every
    // symlink resolved, and this test built the expected path from
    // `std::env::temp_dir()`, which is not. On macOS the two spell the same
    // directory as `/private/var/…` and `/var/…`, because `/var` is a symlink —
    // measured, in run `34952200942`, where the failure was this assertion and the
    // five other tests in this file passed. The sibling file's comment names the
    // Windows version of the same trap (a drive letter's case) and was **not
    // enough**: the reason it gives is one platform's, and the rule it holds is
    // every platform's.
    let reported = value(&report, "cwd").expect("the child reported its directory");
    assert_eq!(
        fs::canonicalize(Path::new(&reported)).expect("the child's directory exists"),
        fs::canonicalize(&directory).expect("the directory we asked for exists"),
        "a service runs in the directory its supervisor was given — the child \
         reported {reported}, not {}",
        directory.display()
    );
}

#[test]
fn a_service_that_outlives_its_budget_is_stopped_by_the_deadline() {
    // The child waits [`ABANDONED`] — fifteen times this budget — so whatever
    // stopped it, it was not the child deciding to stop.
    let root = scratch("outlives-its-budget");
    let directory = root.join("working");
    fs::create_dir_all(&directory).expect("a working directory");
    let python = as_python(&directory);
    let report = report_path(&directory);
    let arguments = child_arguments("child_serves_until_released", &report, 13);

    let enforcement = admits(
        &python,
        &arguments,
        ExecutionMode::HostConfirmed,
        everything(),
    );
    let supervisor = supervisor(&directory, limits(BRIEF));

    let service = supervisor
        .start(
            enforcement
                .admitted()
                .next()
                .expect("the one command this plan decided on"),
        )
        .expect("a service that starts");

    // **The deadline has to be allowed to arrive before this test asks for a
    // stop, and that is the whole shape of the test.** `stop` cancels the run
    // and *then* waits, so a stop that arrives first is a cancellation and comes
    // back `Cancelled` — correctly, because a stopped run is what happened.
    // `TimedOut` is the answer to a budget that had already run out, and the
    // word "already" in `Service::stop`'s documentation is doing real work.
    assert!(
        wait_until(PATIENT, || service.has_finished()),
        "{} is fifteen times the {BRIEF:?} budget and `stop` has not been called, \
         so a run that is still going after {PATIENT:?} was stopped by neither",
        "the child's own give-up time"
    );

    let outcome = service.stop().expect("a stop that is reported");

    assert!(
        matches!(outcome.termination(), Termination::TimedOut { .. }),
        "a service still running when its budget expires is stopped by the \
         deadline: {:?}",
        outcome.termination()
    );
    assert!(
        outcome.took() >= BRIEF,
        "the run ended before its budget had passed, so the deadline is not what \
         ended it: it took {:?}",
        outcome.took()
    );
    assert!(
        !abandoned_path(&report).exists(),
        "the child gave up on its own, so this run ended for a reason the \
         deadline did not cause and the assertion above is about the wrong thing"
    );
    assert!(
        !report_path(&directory).exists(),
        "the child wrote its report, which it only does when it is released"
    );
}

#[test]
fn a_service_that_is_dropped_is_stopped_anyway() {
    // The claim the module documentation makes about the safe direction: a
    // forgotten service is stopped rather than left running. Nothing here calls
    // `stop`, and nothing here can read an outcome — the handle is gone, which
    // is exactly the position a caller that forgot is in.
    let root = scratch("dropped");
    let directory = root.join("working");
    fs::create_dir_all(&directory).expect("a working directory");
    let python = as_python(&directory);
    let report = report_path(&directory);
    let arguments = child_arguments("child_serves_until_released", &report, 14);

    let enforcement = admits(
        &python,
        &arguments,
        ExecutionMode::HostConfirmed,
        everything(),
    );
    let supervisor = supervisor(&directory, limits(LONG_ENOUGH));

    let service = supervisor
        .start(
            enforcement
                .admitted()
                .next()
                .expect("the one command this plan decided on"),
        )
        .expect("a service that starts");
    assert!(
        wait_until(PATIENT, || started_path(&report).exists()),
        "the service never came up, so this test would prove nothing about \
         dropping one"
    );

    // **And wait for the child to be in its loop, because a heartbeat that was
    // never written is not an instrument.** `started` is written one statement
    // before the first heartbeat, which on an idle machine is the same instant —
    // and on a loaded one the child can be descheduled between the two and killed
    // before it ever polls, which is what happened on macOS in run `34982674189`:
    // `wait_until_quiet` read a file that had never been created, saw two equal
    // empty readings and answered "quiet". The guard below caught it and said so,
    // which is that guard working — so this establishes the premise instead of
    // assuming it, and what the drop is then measured against is a child that was
    // demonstrably running and demonstrably waiting.
    assert!(
        wait_until(PATIENT, || heartbeat_path(&report).exists()),
        "the child never wrote a heartbeat, so it never reached its wait and \
         dropping the service here would measure something other than a stop"
    );

    drop(service);

    // **The heartbeat is the instrument, and it is the only one available.** A
    // dropped service hands back no outcome, so there is no report of a stop to
    // read — and writing the release file immediately would race the kill: a
    // child that saw the release before it died would write its report and the
    // test would call that a failure to stop. So the child rewrites a file on
    // every poll, and the test waits until that file stops changing. Two
    // observations four polls apart are equal only when nothing is writing.
    let quiet = wait_until_quiet(&heartbeat_path(&report), PATIENT);
    assert!(
        quiet.is_some(),
        "the child was still writing after {PATIENT:?}, so a dropped service is \
         a process left running — which is the one thing this default exists to \
         prevent"
    );
    assert!(
        !quiet.unwrap_or_default().is_empty(),
        "the heartbeat was never written, so the child had not reached its wait \
         and this test measured the wrong thing"
    );

    // Now the release, which a living child would answer with a report.
    fs::write(released_path(&report), b"released").expect("the release");
    std::thread::sleep(RELEASE_WAIT);
    assert!(
        !report_path(&directory).exists(),
        "the service was dropped and then answered the release, so it was still \
         running"
    );
    assert!(
        !abandoned_path(&report).exists(),
        "the child gave up on its own rather than being stopped by the drop"
    );
}

#[test]
fn a_mode_that_runs_nothing_admits_nothing_that_a_service_could_be_started_from() {
    // The seam the pipeline will use, and the reason `start` takes an
    // `AdmittedCommand` rather than a command line: in a mode that runs nothing,
    // there is no value to hand it. That is not a rule in this file — it is a
    // type with no constructor outside `enforce`, and this test is what says so
    // out loud.
    let root = scratch("inspect-only");
    let directory = root.join("working");
    fs::create_dir_all(&directory).expect("a working directory");
    let python = as_python(&directory);
    let report = report_path(&directory);
    let arguments = child_arguments("child_serves_until_released", &report, 15);
    let supervisor = supervisor(&directory, limits(LONG_ENOUGH));

    let refusing = admits(
        &python,
        &arguments,
        ExecutionMode::InspectOnly,
        everything(),
    );
    assert_eq!(
        refusing.admitted().count(),
        0,
        "a mode that runs no project code admits no command, whatever the \
         permissions say"
    );
    assert_eq!(
        refusing.stopped().len(),
        1,
        "and the check is stopped rather than dropped, so a report can say why"
    );
    assert_eq!(
        refusing.stopped()[0].not_checked_reason,
        Some(NotCheckedReason::ExecutionNotAuthorized)
    );

    // The same command, granted: the decision is about the mode and not about
    // the command, so the second half of this test is the positive control for
    // the first — without it, a plan that admitted nothing for any reason would
    // satisfy the assertion above.
    let allowing = admits(
        &python,
        &arguments,
        ExecutionMode::HostConfirmed,
        everything(),
    );
    let service = supervisor
        .start(
            allowing
                .admitted()
                .next()
                .expect("the mode that runs project code admits this command"),
        )
        .expect("a service that starts");
    assert!(
        wait_until(PATIENT, || started_path(&report).exists()),
        "the service never came up"
    );
    let outcome = service.stop().expect("a stop that is reported");
    assert!(matches!(
        outcome.termination(),
        Termination::Cancelled { .. }
    ));
}

//! `P5-T002`'s acceptance, checked from outside the crate.
//!
//! The task's sentence is two sentences:
//!
//! > *Supported service can be started/probed/terminated.*
//! > *Startup failure remains explicit.*
//!
//! The first is a claim about three things happening to a real process, so every
//! test here starts one and asserts on what happened to it. The second is the
//! harder of the two, because a start that failed is exactly where a check
//! quietly becomes a pass — so the tests about failure are written to fail if
//! the verdict is anything other than the one the module's table says, and each
//! one names the sentence it is checking for.
//!
//! # What is being tested, and what is deliberately not
//!
//! The command handed to a [`StartSmoke`] here is built by this file, not read
//! out of the project's manifest. That is not a shortcut: **turning a declared
//! line into a program and an argument vector is the executor's work**, one step
//! before this module, and `runtime_start.rs` says why it is not in there. What
//! a test can check is therefore what this module does with a command that was
//! admitted for a probe — and the probe in every test is a real one, planned by
//! the product from a real workspace on disk, so the *check identity* the smoke
//! runs under is the plan's and not this file's.
//!
//! # The instrument: a copy of this binary, named `python`
//!
//! A service can only be started from an [`AdmittedCommand`], and a command is
//! only admitted if [`sure_core::safety`] classifies it — and classification
//! reads the **last path component** of the program. This test binary is called
//! `runtime_start-<hash>.exe`, which classifies as an unknown program, which
//! `safety` deliberately treats as *anything* including the destructive
//! permissions, so no mode admits it. [`Fixture::python`] therefore copies the
//! binary to a file named `python.exe` (or `python`) first: the child is still
//! this binary, run again, under a name the classifier reads as *runs the
//! project's code*.
//!
//! The child is reached by starting it with `--ignored --exact <name>`, and it
//! is told what to do through its own argument vector — a service is started
//! with an inherited environment, so a parent cannot hand it a variable and
//! cannot set one for itself either (`std::env::set_var` is `unsafe` in edition
//! 2024 and this workspace forbids unsafe code outright).
//!
//! # The five things a child can do, and why each is needed
//!
//! | Child | What it does |
//! | --- | --- |
//! | [`ANSWER`] | binds the port, answers every request, and never ends |
//! | [`SILENT`] | binds the port, accepts the request, and says nothing |
//! | [`HANDOVER`] | starts an [`ANCHOR`] and ends once it has been asked |
//! | [`ANCHOR`] | binds the port, holds a question open, and lets go once the service is gone |
//! | [`DIE`] | ends immediately, after [`NOISE`] lines of noise and then why, on standard error |
//!
//! `ANSWER` and `SILENT` are the two answers a port can give, and they are what
//! separates *it came up and answered* from *something is listening* — the
//! distinction `P3-T010`'s probe exists for, carried all the way through this
//! module's verdict. `DIE` is a start that failed, which is the acceptance's
//! second sentence — and the one child that writes **more than the quote keeps**,
//! because what a failing program says is only half the claim: the other half is
//! that a report quotes the *tail* of it, which needs a program with a head to
//! drop.
//!
//! # Why one row takes two processes
//!
//! **`HANDOVER` and `ANCHOR` are one instrument in two processes**, and they are
//! what reaches the module's hardest row: *a service that outlives the window
//! and then ends by itself*. SURE asks a service and stops it with microseconds
//! in between, so a service whose death is *caused* by the question can never be
//! in that row — its death and the end of the exchange are the same instant, and
//! which of the two the runner notices first is a race this file lost once (the
//! failure read `error`, and the exchange had been reset by the service going
//! down with the request still unread). So the port belongs to a second process
//! that holds the question open, and the service ends when the **question** has
//! arrived: the anchor says so, the service writes its marker and goes, the
//! anchor waits a further [`LINGER`] and only then puts the connection down. The
//! order is causal in both directions and no part of it is two clocks agreeing.
//!
//! That shape is not a contrivance for a test. A real project's dev server is
//! exactly this: `npm run dev` starts a server and is not itself the server,
//! so the process SURE supervises can end while the port it opened is still
//! answering. The child here says nothing and answers nothing itself, which is
//! the honest version of it — what these two processes are for is *when* the
//! ending happens, not what the service does with a request.
//!
//! # What is not claimed here
//!
//! **Nothing about what the service does.** One request, one answer, no body is
//! read. Route checks are `P5-T003` and the browser check is `P5-T004`.
//!
//! **Nothing about several services at once.** One probe, one question, one
//! service; the handover row's second process is part of that row's instrument
//! and not a second service, and `P5-T007` is where a run with more than one
//! service lives.
//!
//! **Nothing about the split.** See above: this file builds commands, and the
//! module under test runs what it is given.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use sure_core::components::ComponentGraph;
use sure_core::config::{CheckPreference, ChecksConfig};
use sure_core::consent::{PermissionPlan, PlannedCheck};
use sure_core::discover::node::MANIFEST;
use sure_core::discover::{DiscoverOptions, Discovery, Ecosystem, Findings, NodeProject, discover};
use sure_core::enforce::Enforcement;
use sure_core::probe::{self, Endpoint};
use sure_core::process;
use sure_core::runtime_probes::{ProbeKind, ProbePlan, RuntimeProbe};
use sure_core::runtime_start::{Limits, LimitsError, QUOTED_LINES, SmokeRefused, StartSmoke};
use sure_domain::evidence::EvidenceClass;
use sure_domain::execution::{ExecutionMode, ExecutionPermissions, Permission};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{AggregateSeverity, CheckStatus, aggregate};

/// How long a service is given to still be running.
///
/// Three seconds, and the number is chosen against *how long a process takes to
/// get going on the slowest machine this runs on* rather than against anything
/// in the code: the child here is a fresh copy of a test binary that has to load
/// libtest before it reaches the line that binds the port. A window that a
/// loaded machine can overshoot would turn "it came up" into "the probe was
/// refused", which is a test that fails for a reason that is not the code's.
///
/// The one test that is *about* the window uses a shorter one and says so.
const WINDOW: Duration = Duration::from_secs(3);

/// The whole-life budget of every service here.
///
/// Generous, because no test in this file is about it: `Limits::new` refuses a
/// window that is not strictly shorter than this, and one test below checks that
/// refusal, but the runs here end because SURE stops them.
const BUDGET: Duration = Duration::from_secs(60);

/// What one exchange is allowed to take.
///
/// Half a second, which is what makes [`SILENT`] a *timeout* rather than an
/// observation: a port that says nothing must be reported as saying nothing, and
/// this is the bound at which SURE stops waiting for it. A connection that is
/// closed answers in microseconds, so nothing else here is near this.
const REQUEST: Duration = Duration::from_millis(500);

/// What one exchange is allowed to take when the service is *ending* while it
/// is open.
///
/// [`REQUEST`]'s half second is a bound on how long a service may take to say
/// something, and it is the bound that ends the [`SILENT`] child's exchange. The
/// handover row is the other way round: the exchange is meant to outlast the
/// service, and what ends it is the anchor letting go. A bound that ended it
/// first would produce the same failure for the wrong reason, so this one is an
/// order of magnitude past the whole of [`LINGER`] plus the two polls that
/// precede it, and it still fails in seconds if the instrument is wrong.
const ENDS_DURING: Duration = Duration::from_secs(5);

/// The whole-life budget of the one service whose **own** budget is what ends
/// it, the window it is given, and the exchange bound that must not be what ends
/// the exchange first.
///
/// The module has three ways for a run to end, and this ordering is the only one
/// that reaches the third: the service outlives the window (so SURE asks it its
/// question — without a question there is nothing to report and the run ends the
/// other way), the exchange is still open when the budget expires, and the
/// exchange's own bound is far enough out that it cannot be what closed the
/// connection. None of the three is [`BUDGET`], [`WINDOW`] or [`REQUEST`]: a
/// budget as generous as [`BUDGET`] could not be reached by a test at all, and a
/// window or an exchange bound as short as these would be what ended the run.
///
/// The margins are the point rather than the numbers. The window is twice the
/// slowest startup this file has seen, the budget is well past the window, and
/// the exchange bound is another two and a half seconds past the budget.
const SHORT_WINDOW: Duration = Duration::from_secs(2);
const SHORT_BUDGET: Duration = Duration::from_millis(3500);
const OUTLASTS_THE_BUDGET: Duration = Duration::from_secs(6);

/// How many bytes of each stream a service keeps, and how much of a response is
/// read. More than anything below writes.
const ROOM: usize = 64 * 1024;

/// What a child does once it has a connection.
const ANSWER: &str = "answer";
const SILENT: &str = "silent";
const HANDOVER: &str = "handover";
const ANCHOR: &str = "anchor";
const DIE: &str = "die";

/// The code a service that fails at once ends with, and the code the one that
/// ends when it is asked ends with. **Different numbers on purpose**: a run that
/// reports the wrong one of them has run the wrong child, and a shared code
/// would make that indistinguishable from a pass.
const DIES_WITH: i32 = 7;
const QUITS_WITH: i32 = 9;

/// The code a child gives up with when nothing ever connects to it.
///
/// A child that waits forever is a process left on the machine by a failing
/// test, so every child here is bounded — and this is the third state, the one
/// that must not be read as either of the others: a child that gave up was
/// **alive and never asked anything**, which looks exactly like "the service
/// came up" from the outside.
const GAVE_UP_WITH: i32 = 11;

/// How long a child waits before giving up. See [`GAVE_UP_WITH`].
const ABANDONED: Duration = Duration::from_secs(30);

/// How often a child looks for a connection. Milliseconds, because the child is
/// not the thing being measured.
const POLL: Duration = Duration::from_millis(25);

/// How long a [`SILENT`] child holds a connection open.
///
/// Comfortably longer than [`REQUEST`], so "the probe's deadline fired" and "the
/// child closed the connection" cannot be confused: the first is the case the
/// test is about, and a child that closed first would be reporting the same
/// outcome for a different reason.
const HOLD: Duration = Duration::from_secs(5);

/// How long an [`ANCHOR`] keeps a question open **after the service it belongs
/// to is gone**, before it puts the connection down.
///
/// This is the margin the module's hardest row rests on, and it is deliberately
/// measured from the ending rather than from the question: the anchor watches for
/// the service's own marker, so its release can never happen before the ending
/// it is supposed to come after, however slow the machine is. A hundred
/// milliseconds is a hundred times the runner's own poll, which is what has to
/// notice the ending — so the run is over, and reported as over, by the time the
/// exchange comes back to SURE.
const LINGER: Duration = Duration::from_millis(100);

/// What a child says on each stream once it is up.
///
/// Distinct sentences rather than one twice: the assertion is that each arrived
/// on its own stream, and one sentence on both would be satisfied by a runner
/// that read one pipe twice.
const SAID_ON_STDOUT: &str = "the smoke service is up on standard output";
const SAID_ON_STDERR: &str = "the smoke service is up on standard error";

/// What a [`DIE`] child says before it goes.
///
/// A whole sentence rather than a word, because the claim it is evidence for is
/// that **a start that failed says why in the program's own words** — and a
/// sentence is also what a report quotes, so this is the smallest thing that can
/// be quoted and still read as the program's own.
const REFUSED_TO_COME_UP: &str = "this service could not come up, and here is what happened";

/// How many lines a [`DIE`] child writes **before** the sentence above.
///
/// More than the quote keeps, on purpose. A report quotes the **tail** of what a
/// program wrote, and a program that wrote fewer lines than the quote keeps
/// cannot tell a tail from a whole stream: with seven lines of noise in front of
/// the sentence, the first three are dropped and the assertion that they are
/// *absent* is what makes "the tail" a checked claim rather than a description.
/// See [`DROPPED`] for the arithmetic, which is derived rather than restated.
const NOISE: usize = 7;

/// How many of those lines the quote leaves out.
///
/// Derived from the module's own [`QUOTED_LINES`], so that raising the quote's
/// size fails the assertion below with a number rather than leaving a test that
/// silently stopped being about the tail.
const DROPPED: usize = NOISE + 1 - QUOTED_LINES;

/// The two lines a service that answers writes back.
///
/// A complete response, and short enough to arrive in one segment: what this
/// module does with an answer is pass the status through, and 204 is a status
/// `probe.rs` maps to a pass without reading anything.
const ANSWERED_WITH: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";

/// The request every test makes.
const HEALTH: &str = "/health";

/// A directory name that is ordinary on both platforms and mangles easily.
const AWKWARD: &str = "a smoke directory with \u{00e9}\u{4e2d}\u{6587}";

// ---------------------------------------------------------------------------
// The fixture
// ---------------------------------------------------------------------------

/// A scratch workspace that removes itself, with a place beside it for the
/// child's program.
///
/// Under the workspace's own `target/` so the fixture is on the same volume as
/// the checkout, with a space and a character outside ASCII in the path — the
/// same discipline `runtime_probes.rs` and `service_supervisor.rs` follow, and
/// for the same reason: a path is the first thing that breaks a program started
/// with an argument vector, and this is the cheapest way to have one.
struct Fixture {
    scratch: PathBuf,
    project: PathBuf,
}

impl Fixture {
    /// How many names to try before giving up.
    ///
    /// One is the ordinary case: the name is either fresh or holds nothing but
    /// an earlier run's leftovers, which are cleared. A name that **cannot** be
    /// cleared is one something is still sitting in — the process that outlives
    /// the test, most likely — and the next counter value is another name.
    const NAMES: u32 = 16;

    /// A directory of this run's own, under a name **no earlier run can have
    /// left anything in**.
    ///
    /// The name is `<test>-<pid>-<counter>`, and Windows recycles process ids —
    /// measured on this machine, where two fixtures from processes twenty-eight
    /// minutes apart were handed the same id. Every assertion in this file reads
    /// [`bound_path`], [`asked_path`], [`died_path`] and [`abandoned_path`] out
    /// of this directory, so a run that landed on an earlier run's directory
    /// would read that run's markers as its own: the three it overwrites look
    /// right and the fourth — a give-up file, written about thirty seconds after
    /// the run that made it had already ended — fails an assertion whose message
    /// then describes something that never happened. That is not hypothetical:
    /// a failing run under load was observed to create no directory at all,
    /// which is what landing on an earlier run's looks like from outside.
    ///
    /// Clearing happens **here** rather than only in [`Drop`], because `Drop`
    /// runs while the process that outlives the test is sitting in the tree and
    /// cannot remove it (see there). A name that cannot be cleared is skipped
    /// rather than taken: a run that cannot have a directory of its own is a run
    /// about to assert on another one's.
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join(AWKWARD);
        for _ in 0..Self::NAMES {
            let unique = format!(
                "{test}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let scratch = root.join(unique);
            match fs::remove_dir_all(&scratch) {
                // Cleared, or never there: either way nothing of an earlier
                // run's is left in it.
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                // Somebody is still in it. Take another name rather than
                // adopt this one.
                Err(_) => continue,
            }
            let project = scratch.join("project");
            fs::create_dir_all(&project)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", project.display()));
            return Self { scratch, project };
        }
        panic!(
            "{} names in a row under {} were all taken by earlier runs and could \
             not be cleared, so this run cannot have a directory of its own",
            Self::NAMES,
            root.display()
        );
    }

    /// Write a file inside the project, creating the directories above it.
    fn write(&self, relative: &str, contents: &str) -> &Self {
        let full = self.project.join(relative);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        fs::write(&full, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        self
    }

    /// A copy of this test binary, named so that classification reads it as
    /// `python`, in a directory of its own.
    ///
    /// **Beside the project rather than inside it.** A program is named by its
    /// path and a service runs in a directory, and this fixture keeps the two
    /// apart so that a test which mixed them up would be visible: nothing here
    /// would be found by a walk of the project, and nothing in the project is
    /// the program that runs.
    fn python(&self) -> PathBuf {
        let directory = self.scratch.join("bin");
        fs::create_dir_all(&directory)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", directory.display()));
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

    /// The same path, with nothing at it.
    ///
    /// The name is the one [`Self::python`] uses, so the command is one the
    /// classifier admits and the only thing wrong with it is that there is no
    /// program there — which is the case the test using this is about.
    fn missing_python(&self) -> PathBuf {
        self.scratch.join("nothing-here").join(if cfg!(windows) {
            "python.exe"
        } else {
            "python"
        })
    }

    /// A path in the scratch directory, for a child's report.
    fn report(&self, name: &str) -> PathBuf {
        self.scratch.join(name)
    }

    fn discovery(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// The Node findings, or a panic naming what was found instead.
    ///
    /// A fixture that quietly stopped being a Node project would make every
    /// assertion below vacuous, so this is the one thing here that panics rather
    /// than returning an option.
    fn node(&self) -> NodeProject {
        let found = self.discovery();
        let report = found
            .report(Ecosystem::Node)
            .unwrap_or_else(|| panic!("this fixture is a Node project, and discovery found none"));
        match &report.findings {
            Findings::Node(node) => (**node).clone(),
            other => panic!("the Node report carried {other:?}"),
        }
    }

    /// A plan built the way a caller builds one: **one discovery, one graph, one
    /// project**.
    fn plan(&self, preferences: &ChecksConfig) -> ProbePlan {
        let discovery = self.discovery();
        let graph = ComponentGraph::of(&discovery);
        ProbePlan::of(&graph, &self.node(), preferences).unwrap_or_else(|error| panic!("{error}"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // **This fails on the ordinary path, and the failure is reported rather
        // than discarded.** The service is started in this tree and the process
        // that outlives the test inherits that working directory, so the removal
        // happens while a live process is sitting in it — which Windows refuses:
        // measured on this platform, removing a tree a running process has its
        // working directory in fails with `WinError 32`, and the same removal
        // succeeds once that process has ended. 404 of 404 of these directories
        // survived while this error was dropped, which is how "the fixture cleans
        // up after itself" stopped being true without anybody noticing; what it
        // left behind is what [`Fixture::new`] now refuses to take.
        //
        // On standard error rather than a panic, because it still must not turn a
        // passing test into a failing one — and libtest shows a test's output only
        // when that test fails, so a green run stays quiet.
        if let Err(error) = fs::remove_dir_all(&self.scratch) {
            eprintln!("left {} behind: {error}", self.scratch.display());
        }
    }
}

/// A workspace whose root declares a way to start and whose member declares a
/// different one, with a lockfile so that a package manager is agreed.
///
/// The lockfile is not decoration: **the runner is what a serve command is
/// rendered by**, and a project with no lockfile has no agreed package manager,
/// so the plan would hold a gap where these tests expect a probe. That failure
/// is loud rather than quiet — [`serve_probe`] names what the plan held instead —
/// but it is the fixture's job not to cause it.
fn workspace(fixture: &Fixture) -> &Fixture {
    fixture
        .write(
            MANIFEST,
            r#"{"name":"root","workspaces":["packages/*"],
                "scripts":{"start":"node server.js"}}"#,
        )
        .write(
            "packages/web/package.json",
            r#"{"name":"web","scripts":{"dev":"vite --host"}}"#,
        )
        .write("package-lock.json", "")
}

/// Every preference except the two this file varies.
fn preferences(services: CheckPreference, browser: CheckPreference) -> ChecksConfig {
    ChecksConfig {
        start_local_services: services,
        browser_probe: browser,
        ..ChecksConfig::default()
    }
}

/// The serve probe for one component, or a panic naming what the plan held.
fn serve_probe<'a>(plan: &'a ProbePlan, component: &str) -> &'a RuntimeProbe {
    probe_for(plan, ProbeKind::Serve, component)
}

/// The probe of one kind for one component, or a panic naming what the plan
/// held.
fn probe_for<'a>(plan: &'a ProbePlan, kind: ProbeKind, component: &str) -> &'a RuntimeProbe {
    plan.probes()
        .iter()
        .find(|probe| probe.kind() == kind && probe.component() == Path::new(component))
        .unwrap_or_else(|| {
            panic!(
                "no {kind:?} probe for {component:?}; the plan holds {:?} and could not plan {:?}",
                plan.probes()
                    .iter()
                    .map(|probe| (probe.kind(), probe.component().to_path_buf()))
                    .collect::<Vec<_>>(),
                plan.not_planned()
                    .iter()
                    .map(|gap| (gap.kind(), gap.component().to_path_buf()))
                    .collect::<Vec<_>>()
            )
        })
}

/// Every permission granted, so that a refusal in a test is the *mode's* answer
/// and not the permission set's.
fn everything() -> ExecutionPermissions {
    let mut permissions = ExecutionPermissions::inspect_only();
    for permission in Permission::ALL {
        permissions.set(*permission, true);
    }
    permissions
}

/// The planned check a probe's own proposal becomes.
///
/// **The probe's own identity, and that is the point.** [`StartSmoke::of`] looks
/// an admitted command up by check id, so a caller that invented one would get
/// [`SmokeRefused::NotAdmitted`] and be right to — and a caller that handed the
/// module a plan whose commands are for *other* checks would be asking it to run
/// work that was admitted for something else.
fn check_of(probe: &RuntimeProbe) -> PlannedCheck {
    let proposal = probe.proposal();
    PlannedCheck::new(
        proposal.id().clone(),
        proposal.title(),
        proposal.severity(),
        proposal.critical(),
    )
}

/// An enforcement that admitted exactly one command, for **this probe's check**.
fn admits(
    probe: &RuntimeProbe,
    program: &Path,
    arguments: &[String],
    mode: ExecutionMode,
) -> Enforcement {
    let check = check_of(probe);
    let mut plan = PermissionPlan::new(mode, FingerprintId::generate(), everything());
    plan.add(check.clone(), program, arguments.to_vec());
    Enforcement::of("the smoke", plan, std::slice::from_ref(&check))
}

/// The bounds one test's smoke check runs under.
fn limits(window: Duration, endpoint: Option<Endpoint>) -> Limits {
    limits_with(window, REQUEST, endpoint)
}

/// The same bounds with the exchange given a different bound of its own.
///
/// One test needs it — the one whose exchange is meant to outlast the service —
/// and it is a parameter rather than a second constant so that the two bounds it
/// varies, the window and the request, are visible in one place.
fn limits_with(window: Duration, request: Duration, endpoint: Option<Endpoint>) -> Limits {
    Limits::new(
        window,
        process::Limits::new(BUDGET, ROOM, ROOM),
        probe::Limits::new(request, ROOM),
        endpoint,
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

/// [`limits_with`], for the one test that is about the service's whole-life
/// budget rather than about this module's window.
///
/// A fourth parameter rather than a fourth constant, because the other three
/// tests that name a window mean [`BUDGET`], and a test whose budget is short
/// should be the only place that number appears.
fn limits_within(
    window: Duration,
    budget: Duration,
    request: Duration,
    endpoint: Option<Endpoint>,
) -> Limits {
    Limits::new(
        window,
        process::Limits::new(budget, ROOM, ROOM),
        probe::Limits::new(request, ROOM),
        endpoint,
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

/// A smoke check for `probe`, over a project rooted at `root`.
fn smoke<'a>(
    root: &Path,
    probe: &RuntimeProbe,
    enforcement: &'a Enforcement,
    window: Duration,
    endpoint: Option<Endpoint>,
) -> StartSmoke<'a> {
    smoke_with(root, probe, enforcement, limits(window, endpoint))
}

/// [`smoke`], for a test that needs bounds of its own.
fn smoke_with<'a>(
    root: &Path,
    probe: &RuntimeProbe,
    enforcement: &'a Enforcement,
    limits: Limits,
) -> StartSmoke<'a> {
    StartSmoke::of(root, probe, enforcement, limits).unwrap_or_else(|error| panic!("{error}"))
}

/// A loopback address on a port nothing is listening on **at the moment it is
/// asked**.
///
/// Bind, take the number, release. That is a race against whatever else on the
/// machine might take the port in between, and it is taken knowingly: the child
/// binds the port it was given and **says so on the stream a report quotes** if
/// it cannot, so losing the race fails the test loudly rather than quietly. The
/// alternative — having the child choose a port and report it back — cannot
/// work here, because the address is a *bound* on the smoke check and is needed
/// before the service that would report it is started.
fn free_port() -> u16 {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port must be bindable");
    let port = listener
        .local_addr()
        .expect("a bound listener has an address")
        .port();
    drop(listener);
    port
}

/// The address the smoke check asks about.
fn endpoint(port: u16) -> Endpoint {
    Endpoint::loopback(port, HEALTH).expect("a loopback endpoint")
}

// ---------------------------------------------------------------------------
// The child
// ---------------------------------------------------------------------------

/// The children below, reached by a service as `--exact <this> --ignored`.
const CHILD: &str = "child_serves_what_it_was_told_to";

/// The arguments that turn a copy of the test binary into one of the children,
/// with the payload carried in the argument vector.
///
/// The payload goes in libtest's `--skip`, which matches nothing and removes
/// nothing — the technique `service_supervisor.rs` uses, and for the reason it
/// gives there: a service is started with an inherited environment, so a parent
/// cannot hand a child a variable and cannot set one for itself either.
/// `--nocapture`, so what the child writes reaches the streams the service
/// captures rather than libtest's capture buffer.
fn child_arguments(behavior: &str, report: &Path, port: u16) -> Vec<String> {
    vec![
        "--exact".to_owned(),
        CHILD.to_owned(),
        "--ignored".to_owned(),
        "--quiet".to_owned(),
        "--nocapture".to_owned(),
        "--skip".to_owned(),
        format!("{}\n{behavior}\n{port}", report.display()),
    ]
}

/// The instructions a child was given, read out of its own argument vector.
fn child_instructions() -> (PathBuf, String, u16) {
    let mut arguments = std::env::args().skip_while(|argument| argument != "--skip");
    arguments.next();
    let payload = arguments
        .next()
        .expect("the child was given its instructions");
    let mut lines = payload.lines();
    let report = PathBuf::from(lines.next().expect("a report path on the first line"));
    let behavior = lines.next().expect("a behavior on the second").to_owned();
    let port = lines
        .next()
        .expect("a port on the third")
        .parse()
        .expect("a port number");
    (report, behavior, port)
}

/// The file a child writes when it has the port, holding the port and **the
/// directory it is running in**.
///
/// The directory is in there because it is a claim this module makes and the
/// child is the only thing that can hold it: [`StartSmoke::of`] joins the
/// probe's component onto the project root, and "in" is not visible in an
/// outcome that only carries streams.
fn bound_path(report: &Path) -> PathBuf {
    report.with_extension("bound")
}

/// The file an [`ANCHOR`] writes when a question has reached it — and, with it,
/// **how many bytes of that question it read**.
///
/// The count is the anchor's own evidence that the exchange really happened, and
/// it is the evidence for the one line in `serve` that keeps the two platforms'
/// `accept` behaviour from being observable: a socket that did not wait for the
/// request reads zero bytes and is then put down with the request still in it,
/// which the far end reports as a reset rather than as a service that was asked.
/// The assertion on it below is therefore about the exchange and not about the
/// platform — the platform's half is `set_nonblocking`, which makes the read wait
/// everywhere rather than only where it already did.
fn asked_path(report: &Path) -> PathBuf {
    report.with_extension("asked")
}

/// The file a child writes when it has the code it was meant to die with.
fn died_path(report: &Path) -> PathBuf {
    report.with_extension("died")
}

/// The file a child gives up into, when nothing ever connects to it.
fn abandoned_path(report: &Path) -> PathBuf {
    report.with_extension("abandoned")
}

/// A service that does what its payload says, and never returns on its own.
///
/// Ignored, so a normal run does not execute it, and reached only by a service
/// started with `--ignored --exact`.
#[test]
#[ignore = "started by the smoke tests, not run on its own"]
fn child_serves_what_it_was_told_to() {
    let (report, behavior, port) = child_instructions();
    match behavior.as_str() {
        // Written before the exit, so that a service which reports the code and
        // no marker is a service that ran some other child.
        DIE => {
            fs::write(died_path(&report), DIES_WITH.to_string()).expect("the marker");
            // On standard error, and after the noise: see [`NOISE`]. libtest is
            // given `--quiet` and this process never returns to it, so what the
            // service captures is exactly these lines.
            for line in 1..=NOISE {
                eprintln!("starting up, step {line}");
            }
            eprintln!("{REFUSED_TO_COME_UP}");
            std::process::exit(DIES_WITH);
        }
        // The one behavior that is not about a port: this process is a launcher,
        // and the port belongs to the process it starts.
        HANDOVER => hand_over(&report, port),
        _ => serve(&report, port, &behavior),
    }
}

/// Start a second process to hold the port, and end once that process has been
/// asked a question.
///
/// The second process is what makes the ending *later than the question* — see
/// the module comment. It is started with every stream on the null device, and
/// that is not tidiness: a child holding the service's pipes open would keep the
/// runner collecting them after the service is gone, and the run would report
/// streams that never finished rather than the ending it is here to report.
///
/// Nothing waits for the second process to exit. It is not this child's to
/// clean up: it gives up on its own, and it is the process that outlives this
/// one by design.
fn hand_over(report: &Path, port: u16) -> ! {
    let program = std::env::current_exe().expect("the binary this child is running");
    let started = Command::new(program)
        .args(child_arguments(ANCHOR, report, port))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Err(error) = started {
        // On the stream the module quotes, like a port that could not be taken:
        // a launcher that could not start its server is a start that failed, and
        // this test should read that way rather than as a service that never
        // ended.
        eprintln!("{REFUSED_TO_COME_UP}: {error}");
        std::process::exit(GAVE_UP_WITH);
    }
    if !wait_for(&asked_path(report), ABANDONED) {
        fs::write(
            abandoned_path(report),
            b"no question ever arrived, so this service was never asked",
        )
        .expect("the give-up file");
        std::process::exit(GAVE_UP_WITH);
    }
    fs::write(died_path(report), QUITS_WITH.to_string()).expect("the marker");
    std::process::exit(QUITS_WITH);
}

/// Bind the port, say so, and answer whatever arrives until something stops it.
fn serve(report: &Path, port: u16, behavior: &str) -> ! {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap_or_else(|error| {
        // On the stream the module quotes, so a child that could not take the
        // port — the race [`free_port`] documents — fails its test with the
        // operating system's own words instead of looking like a service that
        // never came up.
        eprintln!("{REFUSED_TO_COME_UP}: {error}");
        std::process::exit(GAVE_UP_WITH);
    });
    // Non-blocking, so that a child nobody ever connects to still has a way out
    // and leaves the machine rather than waiting on it.
    listener
        .set_nonblocking(true)
        .expect("a non-blocking listener");

    let directory = std::env::current_dir().expect("a working directory");
    fs::write(
        bound_path(report),
        format!("port\t{port}\ncwd\t{}\n", directory.display()),
    )
    .expect("the bound marker");
    println!("{SAID_ON_STDOUT}");
    eprintln!("{SAID_ON_STDERR}");

    // **Whether anything ever connected**, which is the whole of what the give-up
    // file is about: a child that was asked has no business writing *nothing ever
    // connected*. The [`ANCHOR`] returns to this loop once it has answered, so
    // without this line it wrote that file about thirty seconds into every run
    // that worked — into the directory the test asserts about, where it outlives
    // the run that made it and becomes evidence about whichever run is handed
    // that name next.
    let mut connected = false;
    let deadline = Instant::now() + ABANDONED;
    loop {
        if Instant::now() >= deadline {
            if !connected {
                fs::write(
                    abandoned_path(report),
                    b"nothing ever connected, and nothing stopped this",
                )
                .expect("the give-up file");
            }
            std::process::exit(GAVE_UP_WITH);
        }
        match listener.accept() {
            Ok((stream, _peer)) => {
                connected = true;
                // **A socket accepted from a non-blocking listener is
                // non-blocking on Windows and blocking on Unix**, and this line
                // is what stops the difference being observable. It was found by
                // failing, and the chain is worth writing down because it is
                // three steps long: the run reported *the exchange could not be
                // made* over a **connection reset**; a reset is an *abortive*
                // close, which the operating system sends only when the closing
                // side still had unread bytes in the socket; the request was
                // still unread, so the read that was supposed to take it had
                // returned without it — and a blocking read cannot return before
                // the bytes arrive. So the accepted socket was not blocking. The
                // child had come up and read its connection before the request
                // reached it, which is a race the far end never saw, because on
                // Unix `accept` would have handed back a socket that waited.
                let _ = stream.set_nonblocking(false);
                answer_or_not(stream, behavior, report)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => std::thread::sleep(POLL),
            // A connection that could not be accepted is not a reason to end: a
            // service that is up stays up.
            Err(_) => std::thread::sleep(POLL),
        }
    }
}

/// What the child does with the connection it was given.
fn answer_or_not(mut stream: TcpStream, behavior: &str, report: &Path) {
    match behavior {
        // The second of the two processes, and the only one that outlives the
        // service. It holds the question open and never answers it — what it is
        // here for is the *connection*, so that the exchange is still open when
        // the service ends — and it lets go only once the service is gone, by
        // its own marker rather than by a clock.
        ANCHOR => {
            let read = read_request(&mut stream);
            fs::write(asked_path(report), format!("read\t{read}\n")).expect("the asked marker");
            if wait_for(&died_path(report), ABANDONED) {
                std::thread::sleep(LINGER);
            }
            // Closing rather than answering, and the close is what ends the
            // exchange at the far end. A clean close needs the request read
            // first, which the line above does: a socket put down with unread
            // bytes in it is reset, and a reset is a different report.
        }
        // Held open rather than dropped, so that the probe's own deadline is
        // what ends the read and not the child closing the connection.
        SILENT => {
            read_request(&mut stream);
            std::thread::sleep(HOLD);
        }
        _ => {
            read_request(&mut stream);
            let _ = stream.write_all(ANSWERED_WITH);
            let _ = stream.flush();
        }
    }
}

/// Read until the blank line that ends the request, or until the peer stops,
/// and say how many bytes that took.
fn read_request(stream: &mut TcpStream) -> usize {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    while let Ok(read) = stream.read(&mut chunk) {
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    buffer.len()
}

/// Wait for a marker to appear, up to a bound.
///
/// Polling a file rather than waiting on the processes, because the two children
/// cannot see each other: each was started by a different parent, and the file is
/// the only thing they share besides the port.
fn wait_for(path: &Path, within: Duration) -> bool {
    let deadline = Instant::now() + within;
    loop {
        if path.exists() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL);
    }
}

/// What a child reported, as `key -> value`.
fn report_of(path: &Path) -> Vec<(String, String)> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
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
// The claims
// ---------------------------------------------------------------------------

#[test]
fn a_service_that_comes_up_and_answers_is_a_pass_that_quotes_the_exchange() {
    // The first acceptance sentence, all three verbs: started by SURE, asked a
    // question by SURE, stopped by SURE. The status is the pass and the reason
    // is what makes it a claim rather than a colour — it names the request that
    // was answered, so the report says what was established rather than that
    // something was.
    let fixture = Fixture::new("comes-up-and-answers");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let port = free_port();
    let report = fixture.report("answer");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(ANSWER, &report, port),
        ExecutionMode::HostConfirmed,
    );
    let smoke = smoke(
        &fixture.project,
        probe,
        &enforcement,
        WINDOW,
        Some(endpoint(port)),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert_eq!(
        result.status,
        CheckStatus::Pass,
        "a service that came up and answered the request was not a pass: {}",
        result.reason
    );
    assert_eq!(
        result.evidence_class,
        EvidenceClass::ObservedFact,
        "a pass here was observed by starting a process and reading its answer"
    );
    assert!(
        result.reason.contains("SURE asked it GET /health HTTP/1.1"),
        "the reason does not name the request that was made, so a reader cannot \
         tell what was established: {}",
        result.reason
    );
    assert!(
        result.reason.contains("204 No Content") && result.reason.contains("3 seconds"),
        "the reason does not carry the answer and the window it was given: {}",
        result.reason
    );
    assert!(
        result.reason.contains("it wrote on standard error")
            && result.reason.contains(SAID_ON_STDERR),
        "the reason does not quote what the service wrote when it came up, or \
         quotes it as though it had arrived on the other stream: {}",
        result.reason
    );
    assert_eq!(
        result.title, "start the project",
        "the result's title is the check's and not the probe verdict's, because \
         the probe's names only the request it answered"
    );
    // The fingerprint the command was admitted under, and not one this module
    // generated: a verdict that carries a fingerprint of its own describes a
    // project state the decision to run was never made about.
    assert_eq!(
        result.project_fingerprint,
        enforcement.check_plan().fingerprint,
        "the result carries a fingerprint that is not the plan's"
    );
    assert!(
        aggregate(std::slice::from_ref(&result)).is_green(),
        "a critical check that passed did not aggregate to green: {}",
        result.reason
    );

    // And the service really was the thing that answered: the child wrote the
    // port it bound before anything could connect to it, and it never gave up.
    let bound = report_of(&bound_path(&report));
    assert_eq!(value(&bound, "port"), Some(port.to_string()));
    assert!(
        value(&bound, "cwd").is_some(),
        "the child did not report where it ran"
    );
    assert!(
        !abandoned_path(&report).exists(),
        "the child gave up rather than being stopped, so nothing was listening \
         when the probe looked and the pass came from somewhere else"
    );
}

#[test]
fn a_service_that_comes_up_and_answers_nothing_is_not_a_pass() {
    // The same service again, and the one line that separates the two rows: it
    // accepts the connection and says nothing. `P3-T010`'s own distinction,
    // carried all the way through this module — a port that is open is evidence
    // that something is listening and no evidence at all about what it does, so
    // the probe's `unknown` arrives as an `unknown` here rather than being
    // flattened into either of the verdicts it is not.
    let fixture = Fixture::new("answers-nothing");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let port = free_port();
    let report = fixture.report("silent");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(SILENT, &report, port),
        ExecutionMode::HostConfirmed,
    );
    let smoke = smoke(
        &fixture.project,
        probe,
        &enforcement,
        WINDOW,
        Some(endpoint(port)),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert_eq!(
        result.status,
        CheckStatus::Unknown,
        "a port that said nothing was reported as something SURE knows: {}",
        result.reason
    );
    assert!(
        result.reason.contains("an open port is not an answer"),
        "the reason does not say what was and was not established: {}",
        result.reason
    );
    assert!(
        bound_path(&report).exists(),
        "the child never bound its port, so nothing was listening and this is \
         not the case the test is about"
    );
    let verdict = aggregate(std::slice::from_ref(&result));
    assert!(result.blocks_green() && !verdict.is_green());
    assert_eq!(
        verdict.severity,
        AggregateSeverity::NotEnoughChecked,
        "a critical check SURE could not conclude anything from leaves the run \
         without a verdict rather than with a mild one: {}",
        result.reason
    );
}

#[test]
fn a_service_that_runs_past_its_own_budget_is_stopped_and_the_budget_is_named() {
    // The third of the three ways a run ends, and the one where **this module's
    // window is not the bound that closed it**. Reaching it takes a particular
    // ordering: the service has to outlive the window, or SURE never asks it
    // anything and the run ends by one of the other two doors; and the exchange
    // has to still be open when the service's own deadline arrives, which means
    // the service is silent — the same service as the warning below, with the
    // exchange bound pushed well past the budget so that the budget is what ends
    // it.
    //
    // What the test holds is that the sentence names **which** clock ran out.
    // "SURE stopped it" is true of every run this module ends and cannot tell a
    // service that used its whole budget from one that was asked and then
    // stopped, which are different things to read in a report.
    let fixture = Fixture::new("runs-past-its-budget");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let port = free_port();
    let report = fixture.report("budget");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(SILENT, &report, port),
        ExecutionMode::HostConfirmed,
    );
    let smoke = smoke_with(
        &fixture.project,
        probe,
        &enforcement,
        limits_within(
            SHORT_WINDOW,
            SHORT_BUDGET,
            OUTLASTS_THE_BUDGET,
            Some(endpoint(port)),
        ),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert!(
        result
            .reason
            .contains("ran until its own budget of 3.5 seconds ran out"),
        "the reason does not say that the service's own budget is what ended the \
         run, which is the one thing this row is about: {}",
        result.reason
    );
    assert!(
        result.reason.contains("SURE asked it GET /health HTTP/1.1"),
        "the reason does not say that the question was asked, which is what \
         separates this row from the warning for a service SURE has no address \
         for: {}",
        result.reason
    );
    // **What the probe made of the exchange is the platform's story, and the
    // difference was found here rather than assumed.** This is the one row where
    // SURE's own deadline lands in the middle of an open exchange, and the two
    // systems report that differently: on Windows a socket is aborted when the
    // process holding it is force-terminated, so the probe sees a reset and
    // reports *the exchange could not be made*; on Unix the kernel closes the
    // socket, so the probe sees the connection end with nothing said and reports
    // *an open port is not an answer*. Both are what the probe saw — this module
    // carries the probe's verdict rather than replacing it — and neither is a
    // pass. They are named as a set rather than pinned to this machine.
    assert!(
        matches!(result.status, CheckStatus::Unknown | CheckStatus::Error),
        "a service that used its whole budget without answering was reported as \
         {:?}: {}",
        result.status,
        result.reason
    );
    assert!(
        result.blocks_green(),
        "a service that never answered did not block a green aggregate: {}",
        result.reason
    );
    // The instrument: the service really did come up and really was given the
    // budget — a child that never bound its port would have been refused by the
    // probe and reported on a different row entirely.
    assert!(
        bound_path(&report).exists(),
        "the child never bound its port, so nothing was listening and the question \
         was answered by a port rather than by the service"
    );
    assert!(
        !abandoned_path(&report).exists(),
        "the child gave up rather than being stopped by its budget, so the run \
         ended for a reason this test is not about"
    );
}

#[test]
fn a_service_that_comes_up_with_nothing_to_ask_is_a_warning_and_never_green() {
    // The row of the table a reader is most likely to misread as a pass. The
    // service is the same one as above, doing the same thing, and the only
    // difference is that SURE was given no address — which is the honest state
    // of a project whose port nothing declares. What SURE knows is that
    // something came up; what it does not know is whether the feature works, and
    // a warning says exactly that.
    let fixture = Fixture::new("nothing-to-ask");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let port = free_port();
    let report = fixture.report("warning");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(ANSWER, &report, port),
        ExecutionMode::HostConfirmed,
    );
    let smoke = smoke(&fixture.project, probe, &enforcement, WINDOW, None);

    let result = smoke.run(&process::Cancellation::default());

    assert_eq!(
        result.status,
        CheckStatus::Warning,
        "a service that came up with no address to ask was not a warning: {}",
        result.reason
    );
    assert!(
        result.reason.contains("there was no address to ask it at"),
        "the reason does not say why nothing was asked, which is the whole of \
         what this verdict is about: {}",
        result.reason
    );
    assert!(
        result.reason.contains("still running after 3 seconds"),
        "the reason does not say what was established about the service: {}",
        result.reason
    );
    // A warning **does not block** — the domain classifies one on a critical
    // check as `CriticalState::Passed`, so it degrades a verdict instead of
    // stopping it — and it is still not a green. Both halves are asserted: a
    // test that checked only the second would pass against a run that had failed
    // the project, and one that checked only the first would pass against a run
    // that reported no problem at all.
    let verdict = aggregate(std::slice::from_ref(&result));
    assert!(
        !result.blocks_green(),
        "a warning is not a failure, so it cannot block a verdict: {}",
        result.reason
    );
    assert!(!verdict.is_green(), "a warning produced a green verdict");
    assert_eq!(
        verdict.severity,
        AggregateSeverity::NeedsAttention,
        "a service SURE knows came up and cannot vouch for is something to look \
         at before hand-off, and the verdict has to say so: {}",
        result.reason
    );
    assert!(
        bound_path(&report).exists(),
        "the child never bound its port, so this warning is about a service that \
         never came up and not about one that did"
    );
}

#[test]
fn a_service_that_ends_by_itself_inside_the_window_fails_and_says_why() {
    // The acceptance's second sentence. A start that failed is the case where a
    // check quietly becomes a pass, so this asserts the status **and** that the
    // reason carries the program's own last words: "it failed to start" is not
    // something a person can act on, and "it could not come up, and here is what
    // happened" is.
    let fixture = Fixture::new("dies-inside-the-window");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let report = fixture.report("die");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(DIE, &report, free_port()),
        ExecutionMode::HostConfirmed,
    );
    let smoke = smoke(
        &fixture.project,
        probe,
        &enforcement,
        WINDOW,
        Some(endpoint(free_port())),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert_eq!(
        result.status,
        CheckStatus::Fail,
        "a service that ended by itself was not a failure: {}",
        result.reason
    );
    assert!(
        result.reason.contains("ended by itself")
            && result.reason.contains(&format!("exit code {DIES_WITH}")),
        "the reason does not say that the service ended and what it ended with: {}",
        result.reason
    );
    // **Which** ending it was, in the words the row above does not use: this
    // service never outlived anything, so a reason that says it did is a reason
    // about a different row of the table.
    assert!(
        result.reason.contains("before SURE could ask it anything")
            && !result.reason.contains("still running after"),
        "the reason describes this service as one that outlived the window, and \
         it ended before the window closed: {}",
        result.reason
    );
    // And no question was asked of it. A question put to a service that has
    // already ended is a reading of a dead port, which is evidence about the
    // port and not about the project — see the module's ordering rule.
    assert!(
        !result.reason.contains("SURE asked it"),
        "SURE asked its question of a service that had already ended: {}",
        result.reason
    );
    assert!(
        result.reason.contains(REFUSED_TO_COME_UP),
        "the reason does not quote what the program wrote before it went, so \
         nobody reading the report learns why it failed: {}",
        result.reason
    );
    // **The tail and not the head**, held against a real process rather than
    // against a hand-built outcome: this child writes [`NOISE`] lines before the
    // sentence, and the quote keeps the last [`QUOTED_LINES`] of them.
    assert!(
        result
            .reason
            .contains(&format!("and {DROPPED} earlier lines were not kept")),
        "the reason does not say how much of what the program wrote it left out, \
         so a reader cannot tell a whole stream from its end: {}",
        result.reason
    );
    assert!(
        result
            .reason
            .contains(&format!("starting up, step {}", DROPPED + 1))
            && !result.reason.contains("starting up, step 1"),
        "the quote is the head of the stream rather than its tail: {}",
        result.reason
    );
    assert!(
        result.blocks_green(),
        "a failed start did not block a green aggregate: {}",
        result.reason
    );
    assert_eq!(
        fs::read_to_string(died_path(&report)).expect("the marker the dying child writes"),
        DIES_WITH.to_string(),
        "the child that ran is not the child that was meant to"
    );
    assert!(
        !bound_path(&report).exists(),
        "the child bound a port before dying, so this is not a start that failed"
    );
}

#[test]
fn a_service_that_outlives_the_window_and_then_ends_is_still_a_failure() {
    // The one row of the table that has two ways in, and the harder one: the
    // window closed with the service alive, so SURE asked it its question — and
    // by the time SURE went to stop it, it had ended by itself. **It is a
    // failure even though the question was asked**, because "start" is the one
    // script whose whole meaning is *this keeps running*.
    //
    // The whole of why this takes two processes is in the module comment. What
    // matters here is the order the four steps happen in, because every one of
    // them is the *cause* of the next and none of them is a clock: SURE asks,
    // the anchor reports the question, the service writes the marker it ended
    // with and goes, the anchor waits [`LINGER`] and lets the connection go.
    // The stop therefore arrives at a run that is already over — by a margin
    // that is a hundred times the poll that has to notice it, rather than by the
    // microseconds that separate the ask from the stop.
    let fixture = Fixture::new("outlives-then-ends");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let port = free_port();
    let report = fixture.report("handover");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(HANDOVER, &report, port),
        ExecutionMode::HostConfirmed,
    );
    // A window this test is about, so it is short: it has to be long enough for
    // a fresh child to start a second process and no longer, because SURE waits
    // the whole window before it asks anything — and the service, which ends
    // when it is asked, cannot end before then. One second is several times what
    // the two processes need, and the assertions below name the failure if it
    // was not enough.
    let smoke = smoke_with(
        &fixture.project,
        probe,
        &enforcement,
        limits_with(Duration::from_secs(1), ENDS_DURING, Some(endpoint(port))),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert_eq!(
        result.status,
        CheckStatus::Fail,
        "a service that was still running when the window closed and then ended \
         was not a failure: {}",
        result.reason
    );
    assert!(
        result
            .reason
            .contains("still running after 1 second, and had ended by itself"),
        "the reason does not lead with the fact that the service was up and then \
         was not, which is the difference between this row and the one above: {}",
        result.reason
    );
    assert!(
        result.reason.contains(&format!("exit code {QUITS_WITH}")),
        "the reason does not carry the code the service ended with: {}",
        result.reason
    );
    assert!(
        result.reason.contains("SURE asked it GET /health HTTP/1.1"),
        "the question really was asked — the service outlived the window — and a \
         reason that does not say so understates what SURE did: {}",
        result.reason
    );
    assert!(
        result.blocks_green(),
        "a failed start did not block a green aggregate: {}",
        result.reason
    );
    let verdict = aggregate(std::slice::from_ref(&result));
    assert_eq!(
        verdict.severity,
        AggregateSeverity::NotReady,
        "a service that came up and then went left the run with no verdict: {}",
        result.reason
    );

    // The instrument, checked as three separate claims, because any one of them
    // failing means this test was about something other than the row it names.
    assert!(
        bound_path(&report).exists(),
        "nothing ever held the port, so the exchange this test is about never \
         happened"
    );
    let asked = report_of(&asked_path(&report));
    let read: usize = value(&asked, "read")
        .expect("the anchor reported what it read")
        .parse()
        .expect("a number of bytes");
    assert!(
        read > 0,
        "the anchor read none of the question, so the exchange was not the \
         exchange this test means to hold open — see the non-blocking note in \
         `serve`"
    );
    assert_eq!(
        fs::read_to_string(died_path(&report)).expect("the marker the ending child writes"),
        QUITS_WITH.to_string(),
        "the service that ran is not the service that was meant to, or it ended \
         somewhere other than the question"
    );
    assert!(
        !abandoned_path(&report).exists(),
        "one of the two processes gave up rather than doing its part, so nothing \
         in this run was released the way the test says it was"
    );
}

#[test]
fn the_service_runs_in_the_directory_the_probe_names() {
    // `StartSmoke::of` joins the probe's component onto the project root, and a
    // workspace member is the case where getting that wrong is invisible: the
    // command is the same command and the outcome is the same outcome. The child
    // is what holds the claim, because it reports the directory it is running
    // in — and a start that ran in the root would be a start that read a
    // different manifest's project.
    let fixture = Fixture::new("member-directory");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "packages/web");
    let port = free_port();
    let report = fixture.report("member");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(ANSWER, &report, port),
        ExecutionMode::HostConfirmed,
    );
    let smoke = smoke(
        &fixture.project,
        probe,
        &enforcement,
        WINDOW,
        Some(endpoint(port)),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert_eq!(
        result.status,
        CheckStatus::Pass,
        "the member's service did not come up and answer: {}",
        result.reason
    );
    let bound = report_of(&bound_path(&report));
    let reported = value(&bound, "cwd").expect("the child reported its directory");
    // Compared as **resolved paths rather than as strings**, the idiom
    // `service_supervisor.rs` was taught by a macOS failure: the child reports
    // `std::env::current_dir()`, which is the directory with every symlink
    // resolved, and the path this file built is not.
    assert_eq!(
        fs::canonicalize(Path::new(&reported)).expect("the child's directory exists"),
        fs::canonicalize(fixture.project.join("packages").join("web"))
            .expect("the member's directory exists"),
        "a smoke check for a workspace member ran somewhere else: the child \
         reported {reported}"
    );
}

#[test]
fn a_command_admitted_for_another_check_is_not_the_one_that_runs() {
    // [`StartSmoke::of`] looks its command up by **the probe's own check id**,
    // and the enforcement here holds two admitted commands: one for a check the
    // probe knows nothing about, and one for the probe's. Both were planned and
    // both were admitted — the mode allows them, and the plan holds them in this
    // order — so the only thing that can tell them apart is the lookup, which is
    // what makes this test about the lookup rather than about the plan.
    //
    // The decoy is admitted **first** on purpose: a lookup that took the first
    // admitted command would take it, and a lookup that took the last would not.
    // With the decoy anywhere else, half of this claim would go unheld.
    let fixture = Fixture::new("not-this-checks-command");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let port = free_port();
    let report = fixture.report("real");
    let decoy_report = fixture.report("decoy");
    let python = fixture.python();
    let check = check_of(probe);
    let decoy = PlannedCheck::new(
        CheckId::generate(),
        "a check this smoke is not",
        Severity::MustFix,
        true,
    );
    let mut permissions = PermissionPlan::new(
        ExecutionMode::HostConfirmed,
        FingerprintId::generate(),
        everything(),
    );
    permissions.add(
        decoy.clone(),
        &python,
        child_arguments(DIE, &decoy_report, free_port()),
    );
    permissions.add(
        check.clone(),
        &python,
        child_arguments(ANSWER, &report, port),
    );
    let enforcement = Enforcement::of("the smoke", permissions, &[decoy, check]);
    let smoke = smoke(
        &fixture.project,
        probe,
        &enforcement,
        WINDOW,
        Some(endpoint(port)),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert!(
        !died_path(&decoy_report).exists(),
        "the command that ran was the one admitted for a different check, so this \
         smoke ran work that was allowed for a check it is not"
    );
    assert_eq!(
        result.status,
        CheckStatus::Pass,
        "the probe's own command is the one that should have run: {}",
        result.reason
    );
    let bound = report_of(&bound_path(&report));
    assert_eq!(
        value(&bound, "port"),
        Some(port.to_string()),
        "the service that came up was not the one this check's command starts"
    );
}

#[test]
fn a_program_that_is_not_there_is_an_error_and_not_a_failure_or_a_pass() {
    // The row the table calls *nothing ran, and nothing about the project was
    // learned*. It would be easy to report this as a failure of the project —
    // SURE could not start it, so surely the project is broken — and that is the
    // false accusation this row exists to avoid: a command that was never
    // executed is evidence about the command and not about the code.
    let fixture = Fixture::new("no-such-program");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let absent = fixture.missing_python();
    assert!(
        !absent.exists(),
        "this test is about a program that is not there"
    );
    let enforcement = admits(
        probe,
        &absent,
        &child_arguments(ANSWER, &fixture.report("never"), free_port()),
        ExecutionMode::HostConfirmed,
    );
    let smoke = smoke(
        &fixture.project,
        probe,
        &enforcement,
        WINDOW,
        Some(endpoint(free_port())),
    );

    let result = smoke.run(&process::Cancellation::default());

    assert_eq!(
        result.status,
        CheckStatus::Error,
        "a program that could not be started was reported as something the \
         project did: {}",
        result.reason
    );
    assert!(
        result.reason.contains("SURE could not start it"),
        "the reason does not say that nothing ran: {}",
        result.reason
    );
    assert!(
        result.blocks_green(),
        "a check that never ran did not block a green aggregate: {}",
        result.reason
    );
    assert_eq!(
        result.evidence_class,
        EvidenceClass::Unknown,
        "nothing was observed, so the evidence is not an observed fact"
    );
}

#[test]
fn a_command_the_mode_stopped_cannot_be_started_and_the_plans_own_result_answers_for_it() {
    // The rule `enforce.rs` states and this module inherits: a runner takes what
    // it launches from `Enforcement::admitted`, so a mode that stopped a check
    // leaves nothing to run. **The result a report needs is not built here** —
    // it is already in `Enforcement::stopped`, written by the layer that made
    // the decision, and this test checks that the refusal names the same check
    // that result does.
    let fixture = Fixture::new("stopped-by-the-mode");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Always,
        CheckPreference::Never,
    ));
    let probe = serve_probe(&plan, "");
    let report = fixture.report("stopped");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(ANSWER, &report, free_port()),
        ExecutionMode::InspectOnly,
    );

    let refused = StartSmoke::of(
        &fixture.project,
        probe,
        &enforcement,
        limits(WINDOW, Some(endpoint(free_port()))),
    )
    .expect_err("a mode that runs no project code must leave nothing to start");

    match &refused {
        SmokeRefused::NotAdmitted { check } => assert_eq!(
            check,
            probe.proposal().id(),
            "the refusal names a check that is not the one the probe is about"
        ),
        other => panic!("a serve probe under a mode that stops it was refused as {other:?}"),
    }
    assert!(
        refused
            .to_string()
            .contains(&probe.proposal().id().to_string()),
        "the refusal does not name the check it is about: {refused}"
    );

    let stopped = enforcement.stopped();
    assert_eq!(
        stopped.len(),
        1,
        "the plan answered for {} checks and the probe is one of them: {stopped:?}",
        stopped.len()
    );
    assert_eq!(
        stopped[0].id,
        *probe.proposal().id(),
        "the plan's own not-run result is not about the probe's check"
    );
    assert_eq!(
        stopped[0].status,
        CheckStatus::Skipped,
        "a check the mode stopped was not reported as one that did not run"
    );
    assert!(
        stopped[0].blocks_green(),
        "a critical check that did not run did not block a green aggregate"
    );
    assert!(
        !bound_path(&report).exists(),
        "a child was started for a check a mode stopped"
    );
}

#[test]
fn a_probe_that_is_not_a_serve_probe_is_refused_by_name() {
    // `ProbeKind::Interface` is `P5-T004`'s, and its action is `BrowserProbe`:
    // what a user granted for it is the permission to look at a browser, not the
    // permission to run the project's code. **The enforcement here admits it**,
    // so the refusal cannot be about the command: the only thing that can refuse
    // this is the kind, which is what makes the test about the kind.
    let fixture = Fixture::new("browser-probe");
    workspace(&fixture);
    let plan = fixture.plan(&preferences(
        CheckPreference::Never,
        CheckPreference::Always,
    ));
    let probe = probe_for(&plan, ProbeKind::Interface, "");
    let python = fixture.python();
    let enforcement = admits(
        probe,
        &python,
        &child_arguments(ANSWER, &fixture.report("interface"), free_port()),
        ExecutionMode::HostConfirmed,
    );

    let refused = StartSmoke::of(
        &fixture.project,
        probe,
        &enforcement,
        limits(WINDOW, Some(endpoint(free_port()))),
    )
    .expect_err("a browser probe is not something this module can start");

    match &refused {
        SmokeRefused::NotAServeProbe { kind } => assert_eq!(
            *kind,
            ProbeKind::Interface,
            "the refusal names the wrong kind"
        ),
        other => panic!("a browser probe was refused as {other:?}"),
    }
    assert!(
        refused
            .to_string()
            .contains("check the interface in a browser"),
        "the refusal does not say in plain language what the probe would do: \
         {refused}"
    );
}

#[test]
fn a_window_that_cannot_close_before_the_service_budget_does_is_refused() {
    // The bounds that are this module's own, checked from outside the crate
    // because they are the one place a caller can hand a smoke check a pair of
    // budgets that can never produce a verdict. **Both halves are named**: a
    // window of zero, and a window the service's own deadline would pre-empt —
    // and the second is checked at the boundary as well as above it, because
    // the boundary is where the two clocks meet.
    let refused = |window: Duration| {
        Limits::new(
            window,
            process::Limits::new(BUDGET, ROOM, ROOM),
            probe::Limits::new(REQUEST, ROOM),
            None,
        )
        .expect_err("a pair of budgets that can never produce a verdict")
    };
    assert_eq!(refused(Duration::ZERO), LimitsError::ZeroWindow);
    assert_eq!(
        refused(BUDGET),
        LimitsError::WindowOutlastsTheBudget,
        "a window exactly as long as the budget is already one that cannot close first"
    );
    assert_eq!(
        refused(BUDGET + Duration::from_secs(1)),
        LimitsError::WindowOutlastsTheBudget
    );
    // And the boundary the rule is *not* about: a window one millisecond short
    // of the budget is accepted, because the refusal is about a check that can
    // never ask its question and not about a short one.
    assert!(limits(BUDGET - Duration::from_millis(1), None).window() < BUDGET);
}

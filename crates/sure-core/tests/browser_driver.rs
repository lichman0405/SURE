//! `P5-T004`'s acceptance, against a real browser and a real socket.
//!
//! The task's two sentences are:
//!
//! > *Browser check can navigate supported local app and capture
//! > runtime/console errors.*
//! > *Unavailable browser never reports pass.*
//!
//! Both are behavioural, and both are about a **whole run** rather than about a
//! value: a page is served by a socket this file opened, a browser this machine
//! has is started by the product's own search, and the answer is read out of the
//! [`Report`] the interface hands back. Nothing here builds an
//! [`Observation`] by hand and calls it a capture — that is what
//! `src/browser_driver/session.rs`'s unit tests are for, and they do it over
//! fabricated protocol messages, which is exactly the thing that cannot show
//! that a real browser sends those messages.
//!
//! # Neither branch is allowed to be vacuous
//!
//! **A machine with no browser is a supported machine**, and the second
//! acceptance sentence is about it — so these tests cannot require a browser to
//! be installed, and cannot skip when there is none. What they do instead is ask
//! [`find_installed_browser`] *before* interpreting the report and then assert
//! the contract that belongs to the machine they are on:
//!
//! | the machine | the report must be | and what is asserted |
//! | --- | --- | --- |
//! | has a browser | an observation | the page was reached, its status and title are right, the errors are in it, and a clean page is green |
//! | has none | an absence | `NoDriverInstalled`, `skipped`, never green, and a critical one still blocks |
//!
//! **The two mismatches are failures rather than skips**, and the difference
//! between them is the point:
//!
//! * *no browser was found and one was driven anyway* is impossible unless the
//!   search and the driver disagree, which would mean a report about a program
//!   nobody can point at.
//! * *a browser was found and the page was not opened* is [`AbsenceReason::DriverWouldNotStart`],
//!   and **it fails here rather than passing as an absence.** A machine that has
//!   a browser but cannot be driven by this build is the one state in which the
//!   first acceptance sentence is never exercised, and a test that treated that
//!   as a pass would be the false green this repository is built against: the
//!   suite would be green on every machine where the adapter does not work.
//!
//! # The instrument
//!
//! [`Site`] binds `127.0.0.1:0` and serves a fixed table of paths from a
//! detached thread, so the port is the operating system's choice, two runs
//! cannot collide, and a machine with something on `3000` cannot make these
//! tests lie. It answers one request per connection and closes, because the
//! point is to be a page rather than to be a server.
//!
//! Two of its details are Windows and Linux traps rather than decoration:
//!
//! * **A socket accepted from a non-blocking listener is non-blocking too**, and
//!   a response written to one can fail part-way through a header. Each accepted
//!   socket is set blocking, the same thing `probe_local_service.rs` does.
//! * **A browser opens connections that never make a request.** Chromium
//!   preconnects to an origin it is about to need and holds the socket, so a
//!   server that read until the client spoke would stop there and the page would
//!   never load. Reads are bounded by [`CLIENT_PATIENCE`] and a socket that says
//!   nothing is closed rather than waited on.
//!
//! # What is not claimed here
//!
//! **Nothing about what a *user* sees.** No pixels are read, no screenshot is
//! taken, and a page that renders white on white passes everything below.
//!
//! **Nothing about a hostile page.** Every server here is written by this file;
//! what a real project's page does to a browser is what the sandbox in
//! `browser_driver/launch.rs` is for, and it is not measured by serving
//! `hello`.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sure_core::browser::{
    AbsenceReason, BrowserDriver, Limits, Observation, ProblemKind, Report, Target,
};
use sure_core::browser_driver::{Browser, find_installed_browser};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::CheckStatus;

/// The look budget for a page that is expected to load. Launching a browser is
/// the slowest thing in this file and a loaded CI machine is not this one, so
/// this is generous on purpose: a budget tight enough to be interesting is a
/// budget that fails for reasons that are not the code's.
///
/// **Forty-five rather than thirty, and the figure that bought it did not measure
/// what it was read as measuring.** One budget pays for two things in sequence —
/// the launch, and the look that follows it — so a launch that takes twenty-five
/// seconds leaves twenty for a page that was going to settle in three. On the
/// windows job of `35351409364` the two tests that need to look at a page were
/// handed `29.999205` and `29.999266` seconds and this budget was raised from
/// thirty to forty-five on that evidence.
///
/// **What those two figures are, read from the source rather than reasoned
/// about:** `session.rs:838-843` hands the launch what is left of *the caller's*
/// budget — thirty seconds less the sub-millisecond search for a browser — and
/// `launch.rs` polls to a deadline set from that figure and reports it back, so
/// it is bounded above by the budget by construction and would read the same for
/// a browser a millisecond from writing its port and for one that was never
/// going to. It is SURE's wait, not the browser's launch. See [`BRIEF`], where
/// `P15-T027` took that apart and left both numbers alone.
///
/// The rest of what this comment used to say is still true and still worth
/// keeping: the same tests drove the same browser well inside this budget on the
/// two runs either side of that one (`35351293152` and `35349989431` before it,
/// `35352560694` and `35357365360` after, all four with a green windows job), so
/// what ran out was the runner's headroom and not the project's; and raising the
/// number does not make anything pass that should fail, because a browser that
/// genuinely cannot be driven still reports `DriverWouldNotStart` and a look that
/// still runs out still stops early.
const PATIENT: Duration = Duration::from_secs(45);

/// The look budget for the page that never arrives. Smaller than [`PATIENT`]
/// because the look runs to its deadline by design there — there is nothing to
/// watch — and still comfortably more than a cold browser start plus an attach,
/// which is all that has to happen before the navigation is refused.
///
/// # This budget was raised twice, and `P15-T027` did not raise it a third time
///
/// **Ten, then twenty, then thirty** (`90a7bc6`, then `853d7de`), each time
/// because a launch was reported as having spent the whole budget on a loaded CI
/// runner: `35196110213` at `19.999018`, `35365324422` at `19.999964`,
/// `35351409364` at `19.999577`, and then `35407533960` at `29.999916` and
/// `35430022867` at `29.999912`. The third raise was not taken, because the
/// report those decisions rested on does not measure the browser. What was
/// measured, in the order it decides the question:
///
/// * **The figure is the budget.** Three calls through this crate's own
///   interface, on one machine, against one unchanging `chrome.exe`, with
///   budgets of 1 ms, 150 ms and 2 s, reported `0.050`, `0.150` — and then
///   *opened the page*. The same browser opened when it was given two seconds,
///   so the figure tracked the budget and not the browser. This test file now
///   pins that behaviour in
///   `a_wait_that_runs_out_says_the_browser_was_still_running_and_not_how_long_it_took`.
/// * **The browser was still running when the wait ran out**, and that is
///   observed rather than assumed: the loop asks `try_wait` on every pass and a
///   browser that had stopped is reported as `Exited` with its status. So none
///   of the recorded failures was a crash or an exit, and **none of them says
///   whether the browser was a millisecond from writing its port or was never
///   going to**.
/// * **The runner that failed was driving browsers at that moment.** In
///   `35407533960` the failing test was one of the three here that launch a
///   browser — they run concurrently, which `35351409364` shows directly: two
///   tests with thirty-second budgets failed 68 ms apart — and the other two
///   launched and passed on that same job. The browser's own stderr in the
///   failing one carries Chromium timestamps, and the last of them is 0.32 s
///   before the line that reports the test failing. "That runner needed more
///   seconds" is therefore not what the logs establish, and on the windows
///   occurrences the browser said nothing at all on its error stream for the
///   whole wait, where a healthy launch on the development machine writes its
///   `DevTools listening on ws://…` line in under 0.35 s.
/// * **The two candidate replacements were measured too, and neither works.** A
///   rule that waits "while the browser is still making progress" has no
///   progress signal to key on here: the port file went from absent to complete
///   in under a millisecond in six cold starts polled every millisecond, so a
///   half-written file is not a state the loop's 25 ms poll can observe, and the
///   only other signal is the liveness it already reports. A larger number is
///   not a rule at all.
///
/// **A red here is true, which is why it is left standing.** A machine on which
/// SURE cannot drive a browser inside the budget is a machine on which the
/// acceptance sentence *the check can navigate a local app* genuinely was not
/// exercised, and `opened()` reports that as a failure rather than passing it
/// over. What changed is what the failure says: it now reports the browser's
/// liveness and its profile directory's state, and states plainly that how close
/// it was to reporting a port is not known, instead of a figure that read like a
/// launch duration.
const BRIEF: Duration = Duration::from_secs(30);

/// More than the broken page produces, since the bound is not what the test is
/// about.
const PROBLEMS_KEPT: usize = 64;

/// How long the accept loop sleeps when there is nothing to accept.
///
/// Shorter than the twenty-five milliseconds the rest of the suite polls at,
/// because this server answers every request a page load makes and each poll is
/// paid by the browser waiting: at twenty-five the whole page would be slowed by
/// a multiple of that, and the settle window would be spent waiting for this
/// file rather than for the page.
const ACCEPT_POLL: Duration = Duration::from_millis(2);

/// How long a connected client has to say something before its socket is closed.
///
/// A browser preconnects and says nothing, which is the case this exists for.
const CLIENT_PATIENCE: Duration = Duration::from_millis(250);

/// A page with nothing wrong with it.
///
/// It asks for nothing — **not even an icon** — so that the only request the
/// browser makes on its own is the `/favicon.ico` Chromium fetches for every
/// origin it opens. That request is a four-hundred-and-four answer on this
/// server, and the test asserts this page is *green anyway*: the browser's own
/// request is not the page's, and reporting it would make nearly every healthy
/// local page fail.
const CLEAN: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>a page with nothing wrong</title></head><body><h1>hello</h1></body></html>";

/// A page that does all three of the things the acceptance names, on one load.
///
/// A subresource that four-hundred-and-fours, a console error, and an uncaught
/// error — the three kinds of [`ProblemKind`], each of them asked for by the
/// page's own markup, so that what the driver reports can be traced back to a
/// line of this constant.
const BROKEN: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>a page that broke</title>\
<script src=\"/missing.js\"></script></head><body><script>\
console.error(\"SURE-CONSOLE-MARKER: the page said this\");\
throw new Error(\"SURE-RUNTIME-MARKER: and then it stopped\");\
</script></body></html>";

/// One answer a [`Site`] can give.
#[derive(Clone)]
struct Response {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: String,
}

impl Response {
    /// A page, served normally.
    fn html(body: &str) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "text/html; charset=utf-8",
            body: body.to_owned(),
        }
    }

    /// Everything this server does not have.
    fn not_found() -> Self {
        Self {
            status: 404,
            reason: "Not Found",
            content_type: "text/plain; charset=utf-8",
            body: String::new(),
        }
    }

    /// The answer as bytes, headers and body.
    fn bytes(&self) -> Vec<u8> {
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
             Cache-Control: no-store\r\nConnection: close\r\n\r\n",
            self.status,
            self.reason,
            self.content_type,
            self.body.len()
        );
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(self.body.as_bytes());
        bytes
    }
}

/// A loopback site that serves a table of paths until it is dropped.
struct Site {
    port: u16,
    running: Arc<AtomicBool>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Site {
    /// Binds a port and serves `routes`, with a four-hundred-and-four for
    /// everything else.
    fn serving(routes: &[(&str, Response)]) -> Self {
        let table: HashMap<String, Response> = routes
            .iter()
            .map(|(path, response)| ((*path).to_owned(), response.clone()))
            .collect();

        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port must be bindable");
        let port = listener
            .local_addr()
            .expect("a bound listener has an address")
            .port();
        listener
            .set_nonblocking(true)
            .expect("a listener can be asked not to block");

        let running = Arc::new(AtomicBool::new(true));
        let asked = Arc::new(Mutex::new(Vec::new()));
        {
            let running = Arc::clone(&running);
            let asked = Arc::clone(&asked);
            // Detached, like every other server in this suite: it ends when the
            // test that owns it drops the flag, and a test that leaves one
            // holding a socket open does not hold the binary open with it.
            thread::spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _peer)) => {
                            // See the header: an accepted socket inherits the
                            // listener's non-blocking mode, and a write to one
                            // can fail in the middle of a header.
                            let _ = stream.set_nonblocking(false);
                            let _ = stream.set_read_timeout(Some(CLIENT_PATIENCE));
                            if let Some(path) = read_request(&mut stream, &asked) {
                                let answer = table
                                    .get(&path)
                                    .cloned()
                                    .unwrap_or_else(Response::not_found);
                                let _ = stream.write_all(&answer.bytes());
                                let _ = stream.flush();
                            }
                        }
                        Err(error) if error.kind() == ErrorKind::WouldBlock => {
                            thread::sleep(ACCEPT_POLL);
                        }
                        // A listener that has stopped accepting is the end of
                        // this thread either way; the test's own assertions are
                        // what report it.
                        Err(_) => break,
                    }
                }
            });
        }

        Self {
            port,
            running,
            asked,
        }
    }

    /// The address this site is served from, as a browser would write it.
    fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Every path a client has asked for, in the order the server saw them.
    fn asked(&self) -> Vec<String> {
        self.asked
            .lock()
            .expect("the request log is not poisoned")
            .clone()
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        // The thread notices within one poll and ends, which releases the port.
        // Nothing here waits for it: a test that is finishing has nothing left
        // to learn from the server, and a join on a detached thread would turn a
        // slow machine into a hung test.
        self.running.store(false, Ordering::SeqCst);
    }
}

/// The path of the request `stream` carries, or `None` if it never made one.
///
/// Logged before it is returned, so that the log says what the server *saw*
/// rather than what the test expected it to see.
fn read_request(stream: &mut TcpStream, asked: &Mutex<Vec<String>>) -> Option<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        // A read that fails or returns nothing is the ordinary end of a
        // preconnected socket, and not an error worth reporting: the caller
        // closes the connection and the loop accepts the next one.
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
        }
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") || buffer.len() > 64 * 1024 {
            break;
        }
    }

    let text = String::from_utf8_lossy(&buffer);
    let path = text.lines().next()?.split_whitespace().nth(1)?.to_owned();
    asked
        .lock()
        .expect("the request log is not poisoned")
        .push(path.clone());
    Some(path)
}

/// Asks SURE to open `path` on `port` with `budget`, through the interface.
///
/// The driver is a `Box<dyn BrowserDriver>` over [`Browser::system`], so the
/// whole product path is exercised: the search for a browser, the launch, the
/// protocol, the folding, and the report. Nothing here names a program, because
/// naming one is what a caller does when it has a browser the search cannot
/// find, and that is a different test — in the module's own unit tests.
fn observe(
    port: u16,
    path: &str,
    budget: Duration,
    cancellation: &sure_core::process::Cancellation,
) -> Report {
    let target = Target::local(port, path).expect("a loopback target");
    let limits = Limits::new(budget, PROBLEMS_KEPT).expect("a budget and room for problems");
    let driver: Box<dyn BrowserDriver> = Box::new(Browser::system());
    driver.observe(&target, &limits, cancellation)
}

/// The report, refused unless it is the shape this machine should have produced.
///
/// This is the whole never-vacuous design in one function: the caller says what
/// it expected of the machine, and a report of the other shape is a failure with
/// the machine's state in the message rather than a quiet pass.
///
/// **The branch taken is printed**, so that a reader of a run's output can tell
/// which contract was checked on the machine that produced it. Without that
/// line the two branches are indistinguishable in a log: a green run would say
/// nothing about whether a browser was driven or whether the check was skipped,
/// and *which of the two happened* is the first thing anybody looking at a
/// browser test needs to know.
fn opened(report: &Report, program: Option<std::path::PathBuf>) -> &Observation {
    match (report, program) {
        (Report::Observed(observation), Some(program)) => {
            println!(
                "a browser was driven: {} opened {}{}",
                program.display(),
                observation.landing_url,
                if observation.complete {
                    ""
                } else {
                    " (and the look stopped early)"
                }
            );
            observation
        }
        (Report::Absent(absence), Some(program)) => panic!(
            "a browser was found at {} and could not be driven: {absence:?}. \
             This is the one machine state in which the acceptance sentence \
             *the check can navigate a local app* is never exercised, so it is \
             reported as a failure rather than passed over as an absence.",
            program.display()
        ),
        (Report::Observed(_), None) => panic!(
            "no browser was found and one was driven anyway, which means the \
             search and the driver disagree about what is on this machine"
        ),
        (Report::Absent(absence), None) => panic!(
            "no browser was found, so this test cannot say anything about \
             opening a page: {absence:?}"
        ),
    }
}

/// The absence this machine should have produced, refused unless it is one.
fn absent(report: &Report, program: Option<std::path::PathBuf>) -> &sure_core::browser::Absence {
    match (report, program) {
        (Report::Absent(absence), None) => absence,
        (Report::Observed(_), None) => panic!(
            "no browser was found and a page was reported anyway, which is the \
             false green the second acceptance sentence is about: {report:?}"
        ),
        (_, Some(program)) => panic!(
            "a browser was found at {} and the test compared against the \
             contract for a machine that has none",
            program.display()
        ),
    }
}

/// **A healthy local page is opened and comes back green** — the first half of
/// the acceptance, and the half that is easy to get wrong in the direction
/// nobody notices.
///
/// The assertion that matters most is the one about a page with *nothing* wrong
/// with it. Chromium asks for `/favicon.ico` on its own for every origin it
/// opens, this server answers 404, and a driver that reported every failed
/// request would call this page broken — which was measured, not imagined, and
/// is why `response_arrived` skips the browser's own request type. A green page
/// whose server answered a 404 is therefore the shape that proves the filter is
/// still there.
///
/// The title is asserted because it is the one field that can only be right if
/// the page was really loaded and really asked about afterwards: a driver
/// reporting an empty title for a page it never rendered would satisfy every
/// other assertion here.
#[test]
fn a_healthy_local_page_is_opened_and_comes_back_green() {
    let site = Site::serving(&[("/clean", Response::html(CLEAN))]);
    let found = find_installed_browser();

    if found.is_none() {
        // The machine branch, asserted in full before the test says what it
        // could not do — so that this is a checked absence rather than a skip.
        let report = observe(
            site.port,
            "/clean",
            PATIENT,
            &sure_core::process::Cancellation::default(),
        );
        let absence = absent(&report, found);
        assert_eq!(absence.reason, AbsenceReason::NoDriverInstalled);
        assert_eq!(report.status(), CheckStatus::Skipped);
        assert!(!report.status().is_green());
        assert!(
            report
                .verdict(
                    CheckId::generate(),
                    &Target::local(site.port, "/clean").expect("a loopback target"),
                    Severity::MustFix,
                    true,
                    FingerprintId::generate(),
                )
                .blocks_green(),
            "a critical browser check that could not run must keep the run out of green"
        );
        println!(
            "no browser on this machine, so nothing was opened: {}",
            absence.plain_explanation()
        );
        return;
    }

    let report = observe(
        site.port,
        "/clean",
        PATIENT,
        &sure_core::process::Cancellation::default(),
    );
    let observation = opened(&report, found);

    assert!(
        observation.landing_url.starts_with(&site.origin()),
        "the page landed at {:?}, which is not this site",
        observation.landing_url
    );
    assert_eq!(
        observation.document_status,
        Some(200),
        "the page was served a 200 and the driver reported {:?}",
        observation.document_status
    );
    assert_eq!(observation.title, "a page with nothing wrong");
    assert!(
        observation.complete,
        "the look stopped early, so nothing below is a statement about the page"
    );
    assert!(observation.reached_a_page());

    assert!(
        observation.problems.is_empty(),
        "a page with nothing wrong with it reported problems: {}. The most \
         likely one is the browser's own /favicon.ico request, which this server \
         answers 404 and which is not the page's.",
        observation.problems_described()
    );
    assert_eq!(report.status(), CheckStatus::Pass);

    assert!(
        site.asked().iter().any(|path| path == "/clean"),
        "the server never saw a request for the page, so whatever was reported \
         did not come from here. Asked for: {:?}",
        site.asked()
    );
}

/// **A page that throws, logs an error and asks for something missing comes
/// back with all three** — the second half of the first acceptance sentence, and
/// the one that would otherwise be satisfied by a driver that opens a page and
/// reports nothing about it.
///
/// All three problems are asserted by kind rather than by count, because the
/// count is the browser's business: an uncaught error can be reported by more
/// than one of the events the driver folds, and a test that pinned the number
/// would be pinning which of them arrived first.
#[test]
fn a_page_that_throws_and_asks_for_something_missing_reports_all_of_it() {
    let site = Site::serving(&[("/broken", Response::html(BROKEN))]);
    let found = find_installed_browser();

    if found.is_none() {
        let report = observe(
            site.port,
            "/broken",
            PATIENT,
            &sure_core::process::Cancellation::default(),
        );
        let absence = absent(&report, found);
        assert_eq!(absence.reason, AbsenceReason::NoDriverInstalled);
        assert_eq!(report.status(), CheckStatus::Skipped);
        assert!(!report.status().is_green());
        println!(
            "no browser on this machine, so the broken page was never opened: {}",
            absence.plain_explanation()
        );
        return;
    }

    let report = observe(
        site.port,
        "/broken",
        PATIENT,
        &sure_core::process::Cancellation::default(),
    );
    let observation = opened(&report, found);

    assert_eq!(observation.title, "a page that broke");
    assert_eq!(observation.document_status, Some(200));

    let said = |kind: ProblemKind| -> Vec<&str> {
        observation
            .problems
            .iter()
            .filter(|problem| problem.kind == kind)
            .map(|problem| problem.message.as_str())
            .collect()
    };

    let console = said(ProblemKind::ConsoleError);
    assert!(
        console
            .iter()
            .any(|line| line.contains("SURE-CONSOLE-MARKER")),
        "the page's console.error was not captured: {}. Everything reported: {}",
        console.join(" | "),
        observation.problems_described()
    );

    let runtime = said(ProblemKind::RuntimeError);
    assert!(
        runtime
            .iter()
            .any(|line| line.contains("SURE-RUNTIME-MARKER")),
        "the page's uncaught error was not captured: {}. Everything reported: {}",
        runtime.join(" | "),
        observation.problems_described()
    );

    let request = observation
        .problems
        .iter()
        .filter(|problem| problem.kind == ProblemKind::FailedRequest)
        .find(|problem| problem.source.contains("/missing.js"));
    let request = request.unwrap_or_else(|| {
        panic!(
            "the four-hundred-and-four on /missing.js was not captured. \
             Everything reported: {}",
            observation.problems_described()
        )
    });
    assert!(
        request.message.contains("404"),
        "the failed request does not say what it was answered with: {:?}",
        request.message
    );

    // The page was served its 200 and still fails, which is the whole reason
    // `document_status` is not a verdict on its own.
    assert_eq!(report.status(), CheckStatus::Fail);
    assert!(!report.status().is_green());

    let asked = site.asked();
    for path in ["/broken", "/missing.js"] {
        assert!(
            asked.iter().any(|seen| seen == path),
            "the server never saw a request for {path}, so the problem reported \
             for it is not about this site. Asked for: {asked:?}"
        );
    }
}

/// **A page that never arrives is a failure of the project, not an absence of a
/// browser** — the boundary the whole adapter is arranged around.
///
/// The two are one keystroke apart and mean opposite things to a reader: *the
/// tool would not start* is a sentence about the machine, and *your page did not
/// open* is a sentence about the project. A driver that reported the second as
/// the first would make a broken project look like an unsupported computer, and
/// nothing a user could do to the project would change the answer.
///
/// The port is bound and released first so that the operating system chooses a
/// number nothing else is using, and **the release is checked rather than
/// assumed**: if something is listening there after all, this test would be
/// about whatever that is, and it says so instead of reporting a false result
/// either way.
///
/// The run that first exercised this reported the landing address as
/// `chrome-error://chromewebdata/`, because **Chromium commits an error page of
/// its own** rather than leaving the browser nowhere. That is why nothing below
/// asserts on the landing address: the address SURE asked for is on the problem
/// this test reads, and the verdict holds whether or not the browser shows a
/// page of its own.
#[test]
fn a_page_that_never_arrives_is_a_failure_of_the_project_and_not_an_absence() {
    let port = a_port_nothing_is_listening_on();
    let found = find_installed_browser();

    if found.is_none() {
        let report = observe(
            port,
            "/",
            PATIENT,
            &sure_core::process::Cancellation::default(),
        );
        let absence = absent(&report, found);
        assert_eq!(absence.reason, AbsenceReason::NoDriverInstalled);
        assert_eq!(report.status(), CheckStatus::Skipped);
        println!(
            "no browser on this machine, so a refused connection was never \
             navigated to: {}",
            absence.plain_explanation()
        );
        return;
    }

    let report = observe(
        port,
        "/",
        BRIEF,
        &sure_core::process::Cancellation::default(),
    );
    let observation = opened(&report, found);

    let opened_failure = observation
        .problems
        .iter()
        .find(|problem| problem.kind == ProblemKind::NavigationFailed);
    let opened_failure = opened_failure.unwrap_or_else(|| {
        panic!(
            "a page on a port nothing is listening on was reported without a \
             word about the navigation: {observation:?}"
        )
    });
    assert!(
        !opened_failure.message.trim().is_empty(),
        "the navigation failure carries no reason from the browser"
    );
    assert!(
        opened_failure.source.contains(&port.to_string()),
        "the navigation failure does not name the address that failed: {:?}",
        opened_failure.source
    );

    assert_eq!(
        report.status(),
        CheckStatus::Fail,
        "a page that could not be opened is a failure of the project and not \
         something to skip: {observation:?}"
    );
    assert!(!report.status().is_green());
}

/// **A wait that runs out says what was observed, and never how long the
/// browser took** — the sentence a reader of a red CI run is handed, checked
/// against a real browser on the machine running it.
///
/// # What this pins, and why it is here rather than beside the message
///
/// The sentence is built in `browser_driver::launch` and its wording is pinned
/// there by a unit test that needs no browser. What that test cannot show is
/// that the sentence is what a **caller** is handed: this one goes through
/// [`BrowserDriver`], through the product's own search for a browser, through a
/// real launch, and reads the absence the caller actually gets. Both halves are
/// needed, because the message a person reads is the one thing about this
/// failure that the repository decided to change.
///
/// # The budget is the smallest this code can be given
///
/// [`Limits::new`] refuses a zero budget, and `session` floors what it passes on
/// at `NO_LESS_THAN` — fifty milliseconds — so a one-millisecond budget is the
/// **shortest wait this code can be asked for**, and nothing shorter exists to
/// try. Measured, a cold Chrome needs more than that by a wide margin: six cold
/// starts on the development machine took 0.184 s to 0.349 s to write their port
/// file, and the recorded CI failures are on machines that took twenty and
/// thirty seconds. If a machine ever did start a browser inside fifty
/// milliseconds this test fails and says so, which is the safe direction — the
/// alternative, treating *a browser was driven* as a pass here, is the false
/// green this file is written against.
#[test]
fn a_wait_that_runs_out_says_the_browser_was_still_running_and_not_how_long_it_took() {
    let Some(program) = find_installed_browser() else {
        println!(
            "no browser on this machine, so there is no launch to run out of \
             budget — the contract for that machine is checked by the other \
             tests here"
        );
        return;
    };

    let target = Target::local(a_port_nothing_is_listening_on(), "/")
        .expect("a loopback target on any port");
    let limits = Limits::new(Duration::from_millis(1), PROBLEMS_KEPT)
        .expect("a budget and room for problems");
    let driver: Box<dyn BrowserDriver> = Box::new(Browser::with_program(program.clone()));
    let report = driver.observe(
        &target,
        &limits,
        &sure_core::process::Cancellation::default(),
    );

    let Report::Absent(absence) = &report else {
        panic!(
            "this machine started {} inside the fifty milliseconds this code \
             floors its launch budget at, so this test cannot reach the failure \
             it is about. That is worth knowing and is not a pass: {report:?}",
            program.display()
        );
    };
    assert_eq!(
        absence.reason,
        AbsenceReason::DriverWouldNotStart,
        "a launch that ran out of budget is not a reason about the project: {absence:?}"
    );

    let detail = &absence.detail;
    assert!(
        detail.contains("the program was still running"),
        "the report does not say what the loop observed about the process: {detail}"
    );
    assert!(
        detail.contains("how close it was to reporting one is not known"),
        "the report does not say what it does not know: {detail}"
    );
    assert!(
        !detail.contains("ran for"),
        "the report claims the program ran for the figure, which is the caller's \
         budget and not a measurement of the browser: {detail}"
    );
    println!("a launch that ran out of budget reported: {detail}");
}

/// A loopback port with nothing behind it, checked rather than assumed.
///
/// **The check is the point and it stays.** A port is returned only after a
/// connection to it has been refused, or has ended at the bound without an
/// answer — the two are one `Err` here, and `http_routes.rs` records which
/// platform does which — so nothing below is ever about whatever else the
/// machine happens to be running.
///
/// **Two things changed, and it took a red run to see either.** The numbers
/// used to come from the operating system's dynamic range, because a port asked
/// for as `0` is handed out of it: 49152 and up on this machine, 32768 and up by
/// default on the platforms this has to port to. That is the same range every
/// other `bind(0)` on the host draws from, including the ones inside the other
/// copies of this binary when a whole gate runs, and two asks at the same moment
/// can be handed the same number. `runtime_start.rs`'s `free_port` removed
/// exactly that and says why; this walk is the same one, and it is here because
/// these two helpers are the ones that assert. And a candidate the machine took
/// is now *replaced* rather than asserted against: on `ubuntu-latest` another
/// job's process took the drawn port in the gap between releasing the listener
/// and testing it, which fired the guard below, and the guard was right — the
/// port really had been taken — so what was wrong was asking the machine for a
/// single number and treating a collision as a fact about SURE.
///
/// Every candidate is still required to be empty before it is returned. The
/// panic at the end is the assertion that got moved, not one that got weakened.
fn a_port_nothing_is_listening_on() -> u16 {
    /// Below every default dynamic range, and past the well-known and registered
    /// ports that a machine's own services are actually likely to be on.
    const LOWEST: u16 = 10_000;
    const HIGHEST: u16 = 32_000;
    /// A range this wide cannot be full of listeners, so the walk ends long
    /// before this on any machine.
    const TRIES: u16 = 1_000;

    static NEXT: AtomicU32 = AtomicU32::new(0);
    let span = HIGHEST - LOWEST;
    // A per-process start and a per-test step, the same shape `runtime_start.rs`
    // uses: two copies of this binary started together do not walk the same
    // numbers in the same order.
    let start = (std::process::id() % u32::from(span)) as u16;
    let step = (NEXT.fetch_add(1, Ordering::Relaxed) % u32::from(span)) as u16;

    for attempt in 0..TRIES {
        let port = LOWEST + (start.wrapping_add(step).wrapping_add(attempt) % span);
        let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, port)) else {
            // Taken, reserved or excluded: skipped rather than handed on to a
            // caller that would then be testing something else.
            continue;
        };
        drop(listener);

        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        if TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_err() {
            return port;
        }
    }

    panic!(
        "all {TRIES} loopback ports this test walked between {LOWEST} and {HIGHEST} had something \
         listening on them by the time they were tested, so no port could be shown to have nothing \
         behind it and every reading below would be about whatever that was. That is a fact about \
         this machine's load and not about SURE."
    );
}

/// **The program SURE would run is not one the project could have written.**
///
/// Everything SURE checks is written by a coding agent, and this module decides
/// what SURE executes — so the one outcome that matters is that the browser is
/// not a file inside the directory being inspected. `installed.rs` is arranged
/// for that in two ways, and this is the end of them that can be observed from
/// outside the crate: the search reads a path table compiled into this binary
/// and the absolute entries of `PATH`, and **neither can name a file in the
/// project**.
///
/// The unit tests beside the search cover the rule in the source — that a
/// relative `PATH` entry produces no candidate at all, and that every candidate
/// is absolute. What is checked here is the outcome a caller would see: whatever
/// the search returns is outside the checkout, so a `chrome.exe` planted in the
/// project is never what gets run.
#[test]
fn the_program_sure_would_run_is_not_one_the_project_could_have_written() {
    let Some(program) = find_installed_browser() else {
        println!(
            "no browser on this machine, so there is no program to check the \
             location of"
        );
        return;
    };

    let root = sure_testkit::repository_root();
    assert!(
        program.is_absolute(),
        "the browser SURE would run is a relative path, which is resolved \
         against whatever directory SURE happens to be in: {}",
        program.display()
    );
    assert!(
        !program.starts_with(&root),
        "the browser SURE would run is inside the project being checked, so a \
         project could supply the program that inspects it: {} is under {}",
        program.display(),
        root.display()
    );
    assert!(
        !program.starts_with(std::env::temp_dir()),
        "the browser SURE would run is in the temporary directory, where a \
         project's own setup could have put it: {}",
        program.display()
    );
}

/// **A browser check that is cancelled before it starts is skipped**, and the
/// cancellation path is wired through the public API.
#[test]
fn a_browser_check_that_is_cancelled_before_it_starts_is_skipped() {
    let cancelled = sure_core::process::Cancellation::default();
    cancelled.cancel();
    let site = Site::serving(&[("/clean", Response::html(CLEAN))]);
    let report = observe(site.port, "/clean", PATIENT, &cancelled);
    let Report::Absent(absence) = &report else {
        panic!("a cancelled check produced an observation: {report:?}");
    };
    assert_eq!(
        absence.reason,
        AbsenceReason::DriverWouldNotStart,
        "a cancelled check is reported as the driver not starting"
    );
    assert_eq!(report.status(), CheckStatus::Skipped);
    assert!(!report.status().is_green());
}

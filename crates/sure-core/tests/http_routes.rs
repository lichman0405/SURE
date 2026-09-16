//! `P5-T003`'s acceptance, checked from outside the crate.
//!
//! The task's sentence is two sentences:
//!
//! > *Known local routes can be probed safely.*
//! > *Response evidence is bound to run/fingerprint.*
//!
//! Both are claims about a **project on disk**, and neither can be checked from
//! inside the module: the reading half is a claim about a file's lines, and the
//! asking half is a claim about what arrived at a socket. So every fixture here
//! is a real workspace and every exchange here is a real one.
//!
//! # Why this file has no child process, when `runtime_start.rs` has five
//!
//! `runtime_start-<hash>.exe` is a copy of its own test binary answering requests
//! as a child, and the reason is that a **service** is a process SURE starts,
//! supervises and stops — an inherited environment cannot carry a payload into
//! it, so the payload goes in the argument vector, and the whole instrument
//! exists to get *a service* in front of the module under test.
//!
//! A route probe starts nothing. It connects to an address the caller already
//! has, sends one request line, and reads what comes back — so the honest
//! instrument is a [`TcpListener`] on a thread of **this** process, and it is a
//! better one here for a reason beyond simplicity: the thread is the only thing
//! that can say what actually arrived. `P5-T003`'s safety claim is *one `GET`
//! per route, at the path the project declared, and nothing else*, and that claim
//! is about bytes on a socket. A test that asserted it by reading the module's
//! own intent would be checking the sentence against itself.
//!
//! # What the anchor test is for
//!
//! [`RouteReading`] is the answer to `runtime_probes.rs`'s stated absence — *a
//! probe that asked whether `/api/health` answers would have to name where SURE
//! read that a route exists* — so the anchor is not decoration on this reading,
//! it **is** the thing that was missing. `a_route_read_from_a_file_is_anchored_at_the_line_that_states_it`
//! therefore reads the file back off disk and asserts that the line the anchor
//! names contains the route the anchor names. A `line` that were off by one, or a
//! `declared_in` naming the wrong file, would leave every other test in this file
//! passing and the product's central claim false.
//!
//! # What is not claimed here
//!
//! **Three stacks, and only in the shapes `http_routes.rs` documents.** A route
//! whose path is computed, whose router is built in a loop, or whose mount point
//! is in another file is not read at all — that is the module's stated
//! narrowness, and `docs/product/MVP_SPEC.md` is where it comes from. The
//! direction that matters is tested: what SURE cannot place is a [`NotProbed`]
//! with a reason, and never a route asked at a path the project does not serve.
//!
//! **Nothing about a browser**, which is `P5-T004`.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sure_core::consent::{PermissionPlan, PlannedCheck};
use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::enforce::Enforcement;
use sure_core::http_routes::{
    Limits, LimitsError, NotProbedBecause, RouteMethod, RouteReading, RouteSmoke,
};
use sure_core::probe::Limits as ProbeLimits;
use sure_core::references::ReferenceOptions;
use sure_core::schedule::CheckReason;
use sure_domain::evidence::{AnchorSubject, EvidenceClass};
use sure_domain::execution::{ActionKind, ExecutionMode, ExecutionPermissions, Permission};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;
use sure_domain::status::{CheckStatus, aggregate};

/// What one exchange is allowed to take, on both sides of it.
///
/// The probe's own bound and the service thread's read bound are the same
/// number, because they bound the same exchange from its two ends: a request that
/// has not arrived within this is a request that is not coming, and the failure
/// either end reports is a fact about the run rather than about the machine.
/// Generous — a loopback exchange takes microseconds — because a bound that a
/// loaded machine can overshoot turns a real result into a timeout.
const EXCHANGE: Duration = Duration::from_secs(2);

/// How long the service thread waits for the requests it was promised.
///
/// **Not the same number as [`EXCHANGE`], and the difference is what makes a
/// bug a failure instead of a hang.** A route the smoke never asks, or a
/// connection it opens and abandons, leaves this thread waiting; without a
/// deadline of its own it would wait forever and the test would be killed by the
/// harness rather than fail. The window is several exchanges wide, so reaching it
/// means the requests are not coming.
const SERVICE_WINDOW: Duration = Duration::from_secs(10);

/// How long the service thread sleeps between polls of a non-blocking listener.
const POLL: Duration = Duration::from_millis(2);

/// How much of a request the service keeps, and what a probe may read back.
/// More than anything below writes.
const ROOM: usize = 64 * 1024;

/// How many routes one smoke is allowed to ask, where the test is not about the
/// budget.
const ENOUGH: usize = 32;

/// A directory name that is ordinary on both platforms and mangles easily.
const AWKWARD: &str = "a route directory with \u{00e9}\u{4e2d}\u{6587}";

/// The two responses the service gives.
///
/// A complete response and nothing else: `Connection: close` is what the probe
/// asked for, the end of the connection is what ends its read, and `204` is a
/// status it maps to a pass without a body to parse.
const NO_CONTENT: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";
const NOT_FOUND: &[u8] =
    b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

// ---------------------------------------------------------------------------
// The fixture
// ---------------------------------------------------------------------------

/// A scratch workspace that removes itself.
///
/// Under the workspace's own `target/`, on the same volume as the checkout, with
/// a space and a character outside ASCII in the path — the discipline
/// `runtime_start.rs`, `runtime_probes.rs` and `service_supervisor.rs` all
/// follow, and for the same reason: a path is the first thing that breaks a
/// program that reads files, and this is the cheapest way to have one.
struct Fixture {
    scratch: PathBuf,
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{test}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let scratch = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join(AWKWARD)
            .join(unique);
        let project = scratch.join("project");
        fs::create_dir_all(&project)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", project.display()));
        Self { scratch, project }
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

    /// Read a file back, by the project-relative path a route carries.
    fn read(&self, relative: &Path) -> String {
        let full = self.project.join(relative);
        fs::read_to_string(&full)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", full.display()))
    }

    fn discovery(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// What SURE reads out of this project.
    fn reading(&self) -> RouteReading {
        RouteReading::of(&self.discovery())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        drop(fs::remove_dir_all(&self.scratch));
    }
}

// ---------------------------------------------------------------------------
// The service
// ---------------------------------------------------------------------------

/// A loopback service on a thread of this process, and the request lines that
/// reached it.
struct Service {
    address: SocketAddr,
    answering: Option<JoinHandle<Vec<String>>>,
    /// The lines seen so far, shared with the thread.
    ///
    /// A second copy of what the thread will return, because a test that needs to
    /// know what arrived *while* the smoke is still running — the budget test —
    /// cannot wait for a join it would have to cause first.
    seen: Arc<Mutex<Vec<String>>>,
}

impl Service {
    /// Answer `expected` requests, deciding each response from the request line.
    ///
    /// The listener is non-blocking so that reaching [`SERVICE_WINDOW`] is a
    /// failure the test reports rather than a thread that never comes back.
    fn start(expected: usize, answer: fn(&str) -> &'static [u8]) -> Self {
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port must be bindable");
        let address = listener
            .local_addr()
            .expect("a bound listener has an address");
        listener
            .set_nonblocking(true)
            .expect("a listener can be made non-blocking");

        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let answering = thread::spawn(move || {
            let deadline = Instant::now() + SERVICE_WINDOW;
            let mut lines: Vec<String> = Vec::new();
            while lines.len() < expected && Instant::now() < deadline {
                match listener.accept() {
                    Ok((stream, _peer)) => {
                        // **The accepted socket is put back into blocking mode,
                        // and that line is not tidiness.** Windows hands an
                        // accepted socket the listener's non-blocking mode and
                        // Unix does not, so without it this thread would read
                        // `WouldBlock` on one platform and wait on the other —
                        // and a request that had arrived would be closed on top
                        // of, which the far end reports as a reset rather than as
                        // a service that answered. `runtime_start.rs` lost a test
                        // to exactly this before the line was written there.
                        stream
                            .set_nonblocking(false)
                            .expect("an accepted socket can be made blocking");
                        let line = exchange(stream, answer);
                        recorded
                            .lock()
                            .expect("the record is not poisoned")
                            .push(line.clone());
                        lines.push(line);
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => thread::sleep(POLL),
                    Err(error) => panic!("the service could not accept: {error}"),
                }
            }
            lines
        });

        Self {
            address,
            answering: Some(answering),
            seen,
        }
    }

    /// Wait for the thread and answer with every request line it read.
    ///
    /// A join that returned fewer lines than the service was promised is a
    /// failure with the number in it: the smoke asked something that never
    /// arrived, which is a different defect from asking the wrong thing and the
    /// only one that a count can tell apart.
    fn finish(mut self) -> Vec<String> {
        self.answering
            .take()
            .expect("the service is joined once")
            .join()
            .expect("the answering thread did not panic")
    }

    /// What has arrived so far, without waiting for anything.
    fn seen(&self) -> Vec<String> {
        self.seen
            .lock()
            .expect("the record is not poisoned")
            .clone()
    }
}

/// Read one request, answer it, and put the connection down.
///
/// The request is read until the blank line that ends its headers, because the
/// answer is chosen from the request line and a service that answered before it
/// had one would be answering the wrong question. The read is bounded so that a
/// connection SURE opened and did not use cannot hold this thread past
/// [`SERVICE_WINDOW`] on its own.
fn exchange(mut stream: TcpStream, answer: fn(&str) -> &'static [u8]) -> String {
    stream
        .set_read_timeout(Some(EXCHANGE))
        .expect("a socket can be given a read bound");
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !buffer.windows(4).any(|window| window == b"\r\n\r\n") {
        if buffer.len() >= ROOM {
            break;
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(_) => break,
        }
    }

    let request = String::from_utf8_lossy(&buffer).into_owned();
    let line = request
        .lines()
        .next()
        .unwrap_or_default()
        .trim_end_matches('\r')
        .to_owned();
    let path = line
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned();

    let _ = stream.write_all(answer(&path));
    let _ = stream.flush();
    // Dropping the stream closes the connection, and the close is what ends the
    // probe's read: it asked for `Connection: close` and does not parse a length.
    drop(stream);
    line
}

/// The address of a port nothing is listening on **at the moment it is asked**.
///
/// Bind, take the number, release. That is a race against whatever else on the
/// machine might take the port in between, and it is taken knowingly — the same
/// way `runtime_start.rs` takes it. The alternative is a fixed port, which is a
/// race against every other run of this suite.
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

/// Everything answered with `204`, which is the service a working project has.
fn answers_everything(_path: &str) -> &'static [u8] {
    NO_CONTENT
}

/// Nothing answered, which is a project whose routes are not there.
fn answers_nothing(_path: &str) -> &'static [u8] {
    NOT_FOUND
}

// ---------------------------------------------------------------------------
// The enforcement
// ---------------------------------------------------------------------------

/// Every permission granted, so that a refusal in a test is the *plan's* answer
/// and not the permission set's.
fn everything() -> ExecutionPermissions {
    let mut permissions = ExecutionPermissions::inspect_only();
    for permission in Permission::ALL {
        permissions.set(*permission, true);
    }
    permissions
}

/// An enforcement whose plan schedules exactly the checks a reading proposes.
///
/// **The reading's own proposals**, turned into the checks a plan is built from,
/// because the fingerprint this produces is the one every result has to carry and
/// a plan built for other checks would make that assertion vacuous.
fn enforcement(reading: &RouteReading) -> Enforcement {
    let scheduled: Vec<PlannedCheck> = reading
        .checks()
        .iter()
        .map(|check| {
            let proposal = check.proposal();
            PlannedCheck::new(
                proposal.id().clone(),
                proposal.title(),
                proposal.severity(),
                proposal.critical(),
            )
        })
        .collect();
    let plan = PermissionPlan::new(
        ExecutionMode::HostConfirmed,
        FingerprintId::generate(),
        everything(),
    );
    Enforcement::of("the route reading", plan, &scheduled)
}

/// The bounds one smoke runs under.
fn limits(most_routes: usize) -> Limits {
    limits_with(EXCHANGE, most_routes)
}

/// [`limits`], for the one test whose exchanges are meant to end by timing out
/// and which would otherwise spend the whole bound four times over.
fn limits_with(request: Duration, most_routes: usize) -> Limits {
    Limits::new(ProbeLimits::new(request, ROOM), most_routes)
        .unwrap_or_else(|error| panic!("{error}"))
}

/// A smoke over a reading, asking the service at `address`.
fn smoke<'a>(
    reading: &RouteReading,
    address: SocketAddr,
    enforcement: &'a Enforcement,
    limits: Limits,
) -> RouteSmoke<'a> {
    RouteSmoke::of(reading, address, enforcement, limits)
}

// ---------------------------------------------------------------------------
// The project
// ---------------------------------------------------------------------------

/// A project that declares routes in all three stacks SURE reads.
///
/// **Five files, and four of them are the shapes the reading has to refuse.** A
/// fixture of nothing but askable routes would leave the whole of *known means
/// read* untested: what makes this module safe is that a route it cannot place is
/// not asked, so the project here contains the four ways that happens — a route
/// on a router rather than on the application, a decorator on an object that is
/// not one, a route on a chain with no name, and a router mounted under a prefix.
///
/// | file | what it is for |
/// | --- | --- |
/// | `src/app.js` | an Express app **built inside a function**, so every route line is indented, and a router bound in the same file |
/// | `api/main.py` | a Flask app, its decorators, and one on an object that is not a Flask app |
/// | `src/router.js` | an Express router, which declares routes at a path only its mount knows |
/// | `src/main.rs` | an axum app: a route that is a check and three that are not |
/// | `src/built.rs` | an axum file whose mount is a call, so every route in it is withheld |
///
/// The indentation in `src/app.js` is not decoration either: an application built
/// by a `createApp()` function is the ordinary way an Express project is written,
/// and a reading that only saw a route starting in column one would report *this
/// project declares none* for most of them.
fn project(fixture: &Fixture) -> &Fixture {
    fixture
        .write(
            "src/app.js",
            "const express = require('express');\n\
             \n\
             function createApp() {\n\
             \x20 const app = express();\n\
             \x20 app.get('/health', (request, response) => response.send('ok'));\n\
             \x20 app.post('/orders', (request, response) => response.send('placed'));\n\
             \x20 app.get('/items/:id', (request, response) => response.send('item'));\n\
             \x20 const router = express.Router();\n\
             \x20 router.get('/admin', (request, response) => response.send('admin'));\n\
             \x20 app.use('/api', router);\n\
             \x20 app.listen(3000);\n\
             \x20 return app;\n\
             }\n",
        )
        .write(
            "src/router.js",
            "const express = require('express');\n\
             const router = express.Router();\n\
             router.get('/admin', (request, response) => response.send('admin'));\n\
             router.get('/health', (request, response) => response.send('router health'));\n",
        )
        .write(
            "api/main.py",
            "from flask import Flask\n\
             from cache import Cache\n\
             app = Flask(__name__)\n\
             cache = Cache(app)\n\
             \n\
             @app.route('/status', methods=['GET', 'HEAD'])\n\
             def status():\n\
             \x20   return 'ok'\n\
             \n\
             @app.route('/report')\n\
             def report():\n\
             \x20   return 'ok'\n\
             \n\
             @cache.route('/cached')\n\
             def cached():\n\
             \x20   return 'ok'\n",
        )
        .write(
            "src/main.rs",
            "use axum::{routing::get, Router};\n\
             \n\
             fn main() {\n\
             \x20   let app = Router::new().route(\"/ping\", get(ping));\n\
             \x20   Router::new().route(\"/bare\", get(bare));\n\
             \x20   let api = Router::new().route(\"/health\", get(health));\n\
             \x20   let mounted = Router::new().nest(\"/api\", api);\n\
             \x20   let extra = Router::new().route(\"/extra\", get(extra));\n\
             \x20   let merged = Router::new().merge(extra);\n\
             \x20   axum::serve(listener, app);\n\
             }\n",
        )
        .write(
            "src/built.rs",
            "use axum::{routing::get, Router};\n\
             \n\
             pub fn all() -> Router {\n\
             \x20   let app = Router::new().route(\"/built\", get(built));\n\
             \x20   Router::new().nest(\"/x\", make_router())\n\
             }\n",
        );
    fixture
}

/// The routes this reading is expected to find, and it is written out rather
/// than derived from the reading so that a reading which lost one is a failing
/// assertion instead of a smaller list.
const DECLARED: &[(&str, RouteMethod, &str)] = &[
    ("src/app.js", RouteMethod::Get, "/health"),
    ("src/app.js", RouteMethod::Post, "/orders"),
    ("src/app.js", RouteMethod::Get, "/items/:id"),
    ("api/main.py", RouteMethod::Get, "/status"),
    ("api/main.py", RouteMethod::Head, "/status"),
    ("api/main.py", RouteMethod::Get, "/report"),
    ("src/main.rs", RouteMethod::Get, "/ping"),
    ("src/main.rs", RouteMethod::Get, "/bare"),
    ("src/main.rs", RouteMethod::Get, "/health"),
    ("src/main.rs", RouteMethod::Get, "/extra"),
    ("src/built.rs", RouteMethod::Get, "/built"),
];

/// The four of those that are checks, which are the four SURE may ask.
///
/// **Four and not eleven, and the difference is the point of this task.** The
/// other seven are routes SURE read and will not ask, each for a reason it
/// states. A project whose route list were read and asked wholesale would have
/// eleven here, and would ask a router's `/admin` at the application's root and
/// an axum file's `/built` at a path nothing serves.
const ASKED: &[(&str, RouteMethod, &str)] = &[
    ("src/app.js", RouteMethod::Get, "/health"),
    ("api/main.py", RouteMethod::Get, "/status"),
    ("api/main.py", RouteMethod::Get, "/report"),
    ("src/main.rs", RouteMethod::Get, "/ping"),
];

/// The routes a project declares that SURE never reads at all.
///
/// **Not the same list as the ones it reads and does not ask**, and the
/// difference is a claim: a route here is on a receiver that is not the
/// application object, so the path it is served at is not the path it is
/// declared with — and a reading that cannot place a route does not claim it
/// exists. `src/router.js`'s `/health` is the one that would do damage: read
/// naively it is a second `GET /health` for `src/app.js`'s, and asking it would
/// report the router's answer as the application's. **`src/app.js`'s `/admin` is
/// the same rule inside the file that holds the application, and it is the
/// fixture that makes the rule load-bearing.** `src/router.js` cannot: it binds
/// nothing to `express(` at all, so the whole file is skipped before the receiver
/// is ever consulted, and a reading that dropped the receiver test would still
/// not read its routes. The two routes from `src/router.js` are in this list
/// because the rule they state is one a reader has to be able to see; the one in
/// `src/app.js` is here because a file holding both the application and a router
/// is the only place where dropping the rule changes what SURE does.
const NOT_READ: &[(&str, RouteMethod, &str)] = &[
    ("src/router.js", RouteMethod::Get, "/admin"),
    ("src/router.js", RouteMethod::Get, "/health"),
    ("src/app.js", RouteMethod::Get, "/admin"),
    ("api/main.py", RouteMethod::Get, "/cached"),
];

// ---------------------------------------------------------------------------
// The reading
// ---------------------------------------------------------------------------

#[test]
fn a_route_read_from_a_file_is_anchored_at_the_line_that_states_it() {
    // The claim `runtime_probes.rs` said was missing: a probe that asks whether a
    // route exists has to name where SURE read that it does. So the check is the
    // anchor against the file — the line the anchor names, read off disk, has to
    // contain the route the anchor names.
    let fixture = Fixture::new("anchored-at-the-line");
    project(&fixture);
    let reading = fixture.reading();

    assert!(
        reading.is_complete(),
        "the reading did not read every source file: {:?}",
        reading.unread()
    );
    // Compared as `Path`s and not as strings: `declared_in` is a path relative
    // to the project root in the platform's own spelling, and what is compared
    // for equality is the path, which is what a user reads a file by. Every
    // *spelling* of it in a report goes through `scan::display_path`, and the
    // anchor assertions below are where that is checked.
    let mut read: Vec<(PathBuf, RouteMethod, String)> = reading
        .checks()
        .iter()
        .map(|check| {
            let route = check.route();
            (
                route.declared_in().to_path_buf(),
                route.method(),
                route.path().to_owned(),
            )
        })
        .collect();
    let mut expected: Vec<(PathBuf, RouteMethod, String)> = ASKED
        .iter()
        .map(|(file, method, path)| (PathBuf::from(file), *method, (*path).to_owned()))
        .collect();
    read.sort();
    expected.sort();
    assert_eq!(read, expected, "the reading is not the project's routes");

    for check in reading.checks() {
        let route = check.route();
        let text = fixture.read(route.declared_in());
        let line = text
            .lines()
            .nth(route.line() - 1)
            .unwrap_or_else(|| panic!("the file has no line {}", route.line()));
        assert!(
            line.contains(route.path()),
            "the anchor names line {} of {}, and that line does not contain the \
             path the route claims: {line:?}",
            route.line(),
            route.declared_in().display()
        );

        // The anchor is the same claim in both places it is made, and it is a
        // line range rather than a subject invented for routes — the vocabulary
        // has twelve subjects and none of them is a route.
        let reason = check.proposal().reason();
        let CheckReason::RouteDeclared {
            declared_in,
            route: spelled,
            line: numbered,
        } = reason
        else {
            panic!("a route's check carries {reason:?}");
        };
        assert_eq!(
            declared_in,
            &sure_core::scan::display_path(route.declared_in())
        );
        assert_eq!(spelled, &route.spelling());
        assert_eq!(*numbered, route.line());
        let anchor = reason
            .anchor()
            .unwrap_or_else(|| panic!("a route whose line is known has no anchor"));
        assert!(
            anchor.is_checkable(),
            "an anchor a reader cannot follow is not an anchor: {anchor:?}"
        );
        assert_eq!(anchor.subject, AnchorSubject::LineRange);
        assert_eq!(
            anchor.location,
            sure_core::scan::display_path(route.declared_in()),
            "the anchor's location is a spelling a person reads, and a report \
             that named a file with the platform's separator is a report whose \
             paths do not match the ones the rest of SURE prints"
        );
        assert!(
            anchor.locator.contains(&format!("line {}", route.line())),
            "the anchor does not name the line: {anchor:?}"
        );
        assert_eq!(
            anchor,
            route.anchor(),
            "the reason's anchor and the route's anchor are two spellings of one \
             claim, and a reader sent to either has to arrive at the same line"
        );

        // And the weight, which is a product decision written where it is made.
        let proposal = check.proposal();
        assert_eq!(proposal.severity(), Severity::MustFix);
        assert!(proposal.critical());
        assert_eq!(proposal.evidence_class(), EvidenceClass::ObservedFact);
        assert_eq!(
            proposal.title(),
            format!("the route `{}` answers", route.spelling())
        );
    }
}

#[test]
fn a_route_sure_cannot_place_is_read_and_says_why_it_was_not_asked() {
    // The other half of the reading, and the one that decides whether the
    // product is honest: everything SURE read and will not ask is a value with a
    // reason, so a reader never has to take a missing row for an absent problem.
    let fixture = Fixture::new("cannot-place-it");
    project(&fixture);
    fixture.write(
        "src/nested.rs",
        "use axum::{routing::get, Router};\n\
         fn build() {\n\
         \x20   let api = Router::new().route(\"/health\", get(health));\n\
         \x20   let app = Router::new().nest(\"/api\", api);\n\
         }\n",
    );
    let reading = fixture.reading();

    let mut not_probed: Vec<(PathBuf, RouteMethod, String, NotProbedBecause)> = reading
        .not_probed()
        .iter()
        .map(|entry| {
            let route = entry.route();
            (
                route.declared_in().to_path_buf(),
                route.method(),
                route.path().to_owned(),
                entry.because().clone(),
            )
        })
        .collect();
    let mut expected: Vec<(PathBuf, RouteMethod, String, NotProbedBecause)> = vec![
        // A `POST` is a claim about the project: asking it would change
        // something, and a smoke check is not a reason to change it.
        (
            PathBuf::from("src/app.js"),
            RouteMethod::Post,
            "/orders".to_owned(),
            NotProbedBecause::WouldChangeSomething(RouteMethod::Post),
        ),
        // A slot is a claim about the reading: the project named a shape, and
        // SURE has no value to put in it.
        (
            PathBuf::from("src/app.js"),
            RouteMethod::Get,
            "/items/:id".to_owned(),
            NotProbedBecause::HasASlot,
        ),
        // A `HEAD` is a claim about **SURE**: it changes nothing, and the probe
        // this build has sends `GET` and nothing else. Reporting it as
        // `WouldChangeSomething` would make SURE's own limitation read as a fact
        // about the project.
        (
            PathBuf::from("api/main.py"),
            RouteMethod::Head,
            "/status".to_owned(),
            NotProbedBecause::NotTheReadSureAsks(RouteMethod::Head),
        ),
        // The three axum cases, and each is a different sentence because each is
        // a different thing SURE saw.
        //
        // A chain with no name: `Router::new().route(...)` is a route and nothing
        // says what it is on, so nothing says whether something mounts it.
        (
            PathBuf::from("src/main.rs"),
            RouteMethod::Get,
            "/bare".to_owned(),
            NotProbedBecause::OnSomethingSureCannotName,
        ),
        // A mount under a prefix: the path this is served at is not the path it
        // is declared with, so asking `/health` would report a working route as
        // missing.
        (
            PathBuf::from("src/main.rs"),
            RouteMethod::Get,
            "/health".to_owned(),
            NotProbedBecause::MountedUnder {
                prefix: "/api".to_owned(),
            },
        ),
        // A `merge`, which is a mount with an empty prefix — recorded as one
        // rather than special-cased, because what moved is the point and not
        // where it went.
        (
            PathBuf::from("src/main.rs"),
            RouteMethod::Get,
            "/extra".to_owned(),
            NotProbedBecause::MountedUnder {
                prefix: String::new(),
            },
        ),
        // And a mount SURE cannot read at all, which withholds **every** route
        // in that file: `nest("/x", make_router())` mounts a value, so a reading
        // that could not tell which name moved has to assume it moved the one
        // being asked about. The cost is a route SURE did not check; the
        // alternative is a route checked at a path the project does not serve.
        (
            PathBuf::from("src/built.rs"),
            RouteMethod::Get,
            "/built".to_owned(),
            NotProbedBecause::MountedInAWaySureCannotRead,
        ),
        // The same mount, written in a file of its own so that the three above
        // are read rather than withheld with it.
        (
            PathBuf::from("src/nested.rs"),
            RouteMethod::Get,
            "/health".to_owned(),
            NotProbedBecause::MountedUnder {
                prefix: "/api".to_owned(),
            },
        ),
    ];
    not_probed.sort_by(|left, right| left.0.cmp(&right.0).then(left.2.cmp(&right.2)));
    expected.sort_by(|left, right| left.0.cmp(&right.0).then(left.2.cmp(&right.2)));
    assert_eq!(
        not_probed, expected,
        "a route SURE would not ask is missing or misspelled"
    );

    for entry in reading.not_probed() {
        let sentence = entry.plain_description();
        assert!(
            sentence.contains(entry.route().path())
                && sentence.contains(&entry.because().to_string()),
            "a reader is given a route and a reason, and this says neither: {sentence}"
        );

        // **A mount is the one reason whose whole point is that the path in the
        // sentence is not where the route is served**, and the assertion above
        // cannot see that: it compares the sentence against the reason it is
        // built from, so a reason that dropped the clause about the path moving
        // would satisfy it. Both halves are asserted — the prefix, and the fact
        // that the served path is a different one — because a reader given only
        // the prefix would try `/api` and be wrong in a new way.
        if let NotProbedBecause::MountedUnder { prefix } = entry.because() {
            assert!(
                sentence.contains(&format!("`{prefix}`"))
                    && sentence.contains("is not the path it is declared with"),
                "a mounted route is reported with the prefix it moved under and \
                 with the fact that its path moved: {sentence}"
            );
        }
    }

    // **Nothing the project declares is missing from the reading**, which is the
    // claim the two lists make together and neither makes alone: a route read as
    // a check and a route read as not-asked are both routes SURE saw, and a
    // reading that dropped one would leave every assertion above passing.
    let mut read: Vec<(String, RouteMethod, &Path)> = reading
        .checks()
        .iter()
        .map(|check| check.route())
        .chain(reading.not_probed().iter().map(|entry| entry.route()))
        .filter(|route| route.declared_in() != Path::new("src/nested.rs"))
        .map(|route| (route.path().to_owned(), route.method(), route.declared_in()))
        .collect();
    let mut declared: Vec<(String, RouteMethod, &Path)> = DECLARED
        .iter()
        .map(|(file, method, path)| ((*path).to_owned(), *method, Path::new(file)))
        .collect();
    read.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    declared.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    assert_eq!(
        read, declared,
        "the reading is missing a route the project declares, or read one it does \
         not declare"
    );

    // And the other direction: three routes the project declares that SURE does
    // not read **at all**, which is a different fact from the two lists above
    // and is why they are asserted against a third list rather than left out.
    // `src/router.js`'s `/health` is the one that would do damage — a reading
    // that took a router's routes for the application's would ask `/health` and
    // report the router's answer as the application's.
    for (file, method, path) in NOT_READ {
        let appeared = reading
            .checks()
            .iter()
            .map(|check| check.route())
            .chain(reading.not_probed().iter().map(|entry| entry.route()))
            .any(|route| {
                route.declared_in() == Path::new(file)
                    && route.method() == *method
                    && route.path() == *path
            });
        assert!(
            !appeared,
            "SURE read `{method} {path}` out of {file}, which is a route it cannot \
             place: the receiver is not the application object, so the path it is \
             served at is not the path it is declared with"
        );
    }
}

#[test]
fn a_check_read_from_a_file_is_named_by_the_route_and_not_only_by_the_file() {
    // The identifier is what joins a result from one run to the check that
    // proposed it in another — `CHECK_PIPELINE.md`'s repair and re-check steps
    // cannot look a check up if the second run named it something else — and its
    // two components are the file the route was declared in and the route. **A
    // component that were only the file is the failure this test exists for**:
    // every route a file declares would share one identity, two of them would be
    // stored as one, and the report would show one row where the project has two.
    let fixture = Fixture::new("one-identifier");
    project(&fixture);
    // A second application declaring a route the first one also declares, so
    // that the *other* direction is a claim too: the same route in two files is
    // two checks, because the two files are two places a reader has to go.
    fixture.write(
        "src/worker.js",
        "const express = require('express');\n\
         const app = express();\n\
         app.get('/health', (request, response) => response.send('worker'));\n",
    );
    let reading = fixture.reading();

    let mut identified: Vec<(String, String)> = reading
        .checks()
        .iter()
        .map(|check| {
            (
                check.route().check_id().as_str().to_owned(),
                check.route().spelling(),
            )
        })
        .collect();
    identified.sort();
    let mut ids: Vec<String> = identified.iter().map(|(id, _s)| id.clone()).collect();
    ids.dedup();
    assert_eq!(
        ids.len(),
        identified.len(),
        "two checks read from this project share an identifier, and the identifier \
         is what a stored result is looked up by: {identified:?}"
    );

    // Both halves of the component, each asserted where it is the only thing
    // that can make the two identifiers differ.
    let id_of = |file: &str, spelling: &str| {
        let check = reading
            .checks()
            .iter()
            .find(|check| {
                check.route().declared_in() == Path::new(file)
                    && check.route().spelling() == spelling
            })
            .unwrap_or_else(|| panic!("no check for `{spelling}` in {file}"));
        check.route().check_id().as_str().to_owned()
    };
    assert_eq!(
        reading
            .checks()
            .iter()
            .filter(|check| check.route().declared_in() == Path::new("api/main.py"))
            .count(),
        2,
        "this test is about two checks in one file and the fixture no longer has them"
    );
    assert_ne!(
        id_of("api/main.py", "GET /status"),
        id_of("api/main.py", "GET /report"),
        "two routes one file declares are two checks"
    );
    assert_ne!(
        id_of("src/app.js", "GET /health"),
        id_of("src/worker.js", "GET /health"),
        "one route two files declare is two checks, because two files are two \
         anchors and a reader sent to either has to arrive at the line it is in"
    );

    // And the identifier does not depend on where the project is on disk. A
    // result written by one run has to join a plan built by another, and those
    // two runs do not share a checkout directory: a path that reached the digest
    // without being made relative would make every result unjoinable.
    let elsewhere = Fixture::new("one-identifier-elsewhere");
    project(&elsewhere);
    elsewhere.write(
        "src/worker.js",
        "const express = require('express');\n\
         const app = express();\n\
         app.get('/health', (request, response) => response.send('worker'));\n",
    );
    let mut here_and_there: Vec<(String, String)> = elsewhere
        .reading()
        .checks()
        .iter()
        .map(|check| {
            (
                check.route().check_id().as_str().to_owned(),
                check.route().spelling(),
            )
        })
        .collect();
    here_and_there.sort();
    assert_eq!(
        identified, here_and_there,
        "the same project read from two directories is two sets of identifiers"
    );
}

#[test]
fn a_file_this_reading_could_not_read_is_named_rather_than_counted_as_empty() {
    // An unread file is a place routes could be. A reading that counted it as a
    // file with no routes would answer *this project declares none* from a file
    // it never looked at, which is the shape every honesty rule in this workspace
    // is written against.
    let fixture = Fixture::new("could-not-read-it");
    project(&fixture);
    fixture.write(
        "src/enormous.js",
        &format!(
            "const app = express();\nconst filler = '{}';\napp.get('/huge', (q, s) => s.send('ok'));\n",
            "x".repeat(4096)
        ),
    );

    let options = ReferenceOptions::default().with_max_file_bytes(512);
    let reading = RouteReading::with_options(&fixture.discovery(), &options);

    assert!(
        reading.unread().contains(&PathBuf::from("src/enormous.js")),
        "the file over the budget is not in the unread list: {:?}",
        reading.unread()
    );
    assert!(!reading.is_complete());
    assert!(
        !reading
            .checks()
            .iter()
            .any(|check| check.route().path() == "/huge"),
        "a route was read out of a file the budget excluded"
    );
    assert!(
        !reading.is_empty(),
        "the reading found nothing, so this test is not about the unread list"
    );
}

#[test]
fn a_route_check_is_a_local_probe_and_needs_nothing_but_inspection() {
    // *Can be probed safely* is a claim about what the check asks for before it
    // runs. Everything here is refused at the vocabulary level rather than by
    // this module's care: the action is `LocalProbe` and the only permission it
    // needs is `Inspect`, which is granted unconditionally — so a route check is
    // not something a user is ever asked to authorize, and there is no path by
    // which one becomes a command that runs the project's code.
    let fixture = Fixture::new("a-local-probe");
    project(&fixture);
    let reading = fixture.reading();

    assert!(!reading.checks().is_empty());
    for check in reading.checks() {
        let requirements = check.proposal().requirements();
        assert_eq!(
            requirements.actions(),
            &[ActionKind::LocalProbe],
            "a route check asks for something other than a local probe"
        );
        assert_eq!(
            requirements.permissions_needed(),
            vec![Permission::Inspect],
            "a route check needs a permission beyond inspection"
        );
        assert!(
            !requirements.runs_project_code(),
            "a route check claims to run the project's code"
        );
    }
}

#[test]
fn an_address_that_is_not_this_machine_is_refused_by_every_route() {
    // The safety claim at the boundary a caller can reach: the address is the
    // caller's, so the one thing this module cannot assume is that it is
    // loopback. It does not have to assume it — `Endpoint` refuses anything that
    // is not — and what that refusal has to be is a route SURE did not ask, with
    // a reason that is true of what was refused.
    let fixture = Fixture::new("not-this-machine");
    project(&fixture);
    let reading = fixture.reading();
    let enforcement = enforcement(&reading);
    let elsewhere = SocketAddr::new(IpAddr::from([10, 0, 0, 1]), 80);

    let smoke = smoke(&reading, elsewhere, &enforcement, limits(ENOUGH));

    assert!(
        smoke.asked().is_empty(),
        "SURE would ask a machine that is not this one: {:?}",
        smoke
            .asked()
            .iter()
            .map(|route| route.spelling())
            .collect::<Vec<_>>()
    );
    // The reading's own refusals stay in the list — that list is the whole of
    // *what SURE did not look at* — so what is counted here is the ones this
    // refusal added.
    let refused: Vec<&NotProbedBecause> = smoke
        .not_probed()
        .iter()
        .map(|entry| entry.because())
        .filter(|because| matches!(because, NotProbedBecause::NotOnThisMachine { .. }))
        .collect();
    assert_eq!(
        refused.len(),
        reading.checks().len(),
        "a route was neither asked nor reported as not asked: {:?}",
        smoke.not_probed()
    );
    for because in refused {
        assert_eq!(
            because,
            &NotProbedBecause::NotOnThisMachine {
                address: IpAddr::from([10, 0, 0, 1])
            },
            "the reason does not name the address that was refused"
        );
        assert!(
            !because.to_string().contains("request line"),
            "the reason claims the path could not be written, which is false of \
             a route whose path is a path SURE would otherwise ask: {because}"
        );
    }
    assert!(
        smoke.run().is_empty(),
        "a refused address still produced results"
    );
}

#[test]
fn a_smoke_allowed_to_ask_nothing_is_refused() {
    // A check allowed to ask nothing can never answer its question, so the
    // constructor refuses rather than rounding up — and the refusal is a value
    // rather than a check that quietly runs zero times.
    assert_eq!(
        Limits::new(ProbeLimits::new(EXCHANGE, ROOM), 0),
        Err(LimitsError::ZeroRoutes)
    );
    assert!(limits(1).most_routes() == 1);
    assert_eq!(limits(ENOUGH).request().timeout(), EXCHANGE);
}

// ---------------------------------------------------------------------------
// The asking
// ---------------------------------------------------------------------------

#[test]
fn every_route_the_project_declares_is_asked_once_and_the_answers_carry_the_fingerprint() {
    // Both acceptance sentences. *Known local routes can be probed*: four routes
    // read out of two files and one more, each asked at the path the project
    // declared — which the service's own record is what proves. *Bound to
    // run/fingerprint*: every result carries the plan's fingerprint and not one
    // this module generated.
    let fixture = Fixture::new("asked-once");
    project(&fixture);
    let reading = fixture.reading();
    let enforcement = enforcement(&reading);
    let service = Service::start(ASKED.len(), answers_everything);
    let smoke = smoke(&reading, service.address, &enforcement, limits(ENOUGH));

    assert_eq!(
        smoke.asked().len(),
        ASKED.len(),
        "the smoke is not asking every route SURE read as askable"
    );
    let results = smoke.run();
    let delivered = service.finish();

    assert_eq!(
        results.len(),
        ASKED.len(),
        "one result per asked route, and a route with no result is a row a reader \
         takes for a route with no problem"
    );
    for result in &results {
        assert_eq!(
            result.status,
            CheckStatus::Pass,
            "a route the service answered was not a pass: {}",
            result.reason
        );
        assert_eq!(result.evidence_class, EvidenceClass::ObservedFact);
        assert!(
            result.reason.contains("204 No Content"),
            "the reason does not carry what the service answered: {}",
            result.reason
        );
        assert_eq!(
            result.project_fingerprint,
            enforcement.check_plan().fingerprint,
            "the result carries a fingerprint that is not the plan's"
        );
    }
    // Which route each result is about, as a set rather than in a row order:
    // the reading sorts by file and a report is free to sort again, so a test
    // that zipped the two lists would be asserting an order neither of them
    // promises — and would fail on a project whose files sort differently from
    // the order this file happens to list them in.
    let mut titles: Vec<String> = results.iter().map(|result| result.title.clone()).collect();
    let mut expected_titles: Vec<String> = ASKED
        .iter()
        .map(|(_file, method, path)| format!("local probe: {method} {path} HTTP/1.1 answered"))
        .collect();
    titles.sort();
    expected_titles.sort();
    assert_eq!(
        titles, expected_titles,
        "the results are not one per declared route, each naming the request that \
         was made"
    );
    assert!(
        aggregate(&results).is_green(),
        "critical checks that all passed did not aggregate to green"
    );

    // And the routes really were the ones asked, once each, and each one a GET.
    let mut asked: Vec<String> = delivered;
    asked.sort();
    let mut expected: Vec<String> = ASKED
        .iter()
        .map(|(_file, method, path)| format!("{method} {path} HTTP/1.1"))
        .collect();
    expected.sort();
    assert_eq!(
        asked, expected,
        "what arrived at the service is not one request per declared route — a \
         route asked twice, a route asked at another path, or a method SURE does \
         not send would all show up here"
    );
    for line in &asked {
        assert!(
            line.starts_with("GET "),
            "SURE sent something other than a GET: {line}"
        );
    }
}

#[test]
fn a_route_the_service_does_not_have_is_a_failure_and_not_a_missing_row() {
    // A project that declares a route and does not serve it is the thing this
    // task exists to catch. The failure has to be a *result*, because a route
    // that produced no row would read as a route with no problem.
    let fixture = Fixture::new("does-not-have-it");
    project(&fixture);
    let reading = fixture.reading();
    let enforcement = enforcement(&reading);
    let service = Service::start(ASKED.len(), answers_nothing);
    let smoke = smoke(&reading, service.address, &enforcement, limits(ENOUGH));

    let results = smoke.run();
    let delivered = service.finish();

    assert_eq!(delivered.len(), ASKED.len(), "the asks did not all arrive");
    assert_eq!(results.len(), ASKED.len());
    for result in &results {
        assert_eq!(
            result.status,
            CheckStatus::Fail,
            "a route the service does not have was not a failure: {}",
            result.reason
        );
        assert!(
            result.reason.contains("404 Not Found"),
            "the failure does not carry what came back: {}",
            result.reason
        );
        assert!(
            !result.status.is_green(),
            "a failure reported itself as green"
        );

        // **The weight of the failure, which nothing held until the mutation set
        // said so.** This file asserts a route check's severity and its critical
        // flag where the *proposal* is built — what the plan says a route is worth
        // — and that assertion cannot see these, because `RouteSmoke::run` passes
        // its own two constants into `verdict`: a result's weight is a **second
        // decision made in a different place**, and the row that changes it
        // survived a full run of the thirty-four. What it costs is not decoration.
        // `blocks_green` returns `false` the moment `critical` is `false`, so a
        // route the project declares and does not serve would stop blocking. The
        // run would still not be a plain pass — a failed check is never green —
        // but the one field that says *the project cannot be trusted for hand-off
        // until this is fixed* would say the opposite of the truth, which is a
        // false green one field to the left of the status. That is the class of
        // defect this whole task exists to prevent, so it is asserted here rather
        // than left to the proposal's own assertion one file away.
        assert_eq!(
            result.severity,
            Severity::MustFix,
            "a route the service does not have was reported as less than something \
             that must be fixed"
        );
        assert!(
            result.critical,
            "a route the service does not have was not marked critical"
        );
        assert!(
            result.blocks_green(),
            "a route the service does not have did not block a green verdict, so a \
             project that declares a route it does not serve could be handed off"
        );
    }
    assert!(
        !aggregate(&results).is_green(),
        "a run whose routes are all missing aggregated to green"
    );
}

#[test]
fn a_port_nothing_is_listening_on_is_never_a_pass_and_the_reason_says_what_happened() {
    // The other way an ask goes wrong: nothing answered at all, rather than
    // something answering that the route is not there. **What the answer is
    // depends on the platform and the test asserts the set**, which is the
    // discipline this branch already applies to its other socket-level row. On
    // Unix a connect to a closed loopback port is refused and the probe reports
    // `nothing accepted the connection`, which is a `Fail`; on Windows — measured
    // here, on a port `free_port` had just released — the connect is not refused
    // and the exchange ends at the bound with `the exchange could not be made
    // (connection timed out)`, which is an `Error`, because a probe that could
    // not make the exchange is a check that could not run rather than a route
    // that is broken. `probe.rs` decides both and this module passes them
    // through; what is asserted here is the part that is this module's, and the
    // part no platform may change: a route with no service behind it is never
    // green, every route still produces its own row, and each row still carries
    // the run's fingerprint.
    //
    // A short bound, because this is the one test whose exchanges are *meant* to
    // end by timing out.
    const WAITING: Duration = Duration::from_millis(400);

    let fixture = Fixture::new("nothing-listening");
    project(&fixture);
    let reading = fixture.reading();
    let enforcement = enforcement(&reading);
    let port = free_port();
    let smoke = smoke(
        &reading,
        SocketAddr::new(IpAddr::from(Ipv4Addr::LOCALHOST), port),
        &enforcement,
        limits_with(WAITING, ENOUGH),
    );

    let results = smoke.run();

    assert_eq!(
        results.len(),
        ASKED.len(),
        "a route that could not be asked produced no row, so a reader sees a \
         missing row rather than a check that could not run"
    );
    for result in &results {
        assert!(
            !result.status.is_green(),
            "a route with nothing behind it was green: {}",
            result.reason
        );
        assert_ne!(
            result.status,
            CheckStatus::Pass,
            "a route with nothing behind it passed: {}",
            result.reason
        );
        assert!(
            result.reason.contains("nothing accepted the connection")
                || result.reason.contains("the exchange could not be made"),
            "the reason does not say that no service answered: {}",
            result.reason
        );
        assert!(
            !result.reason.contains("404") && !result.reason.contains("200"),
            "the reason reports an HTTP status nothing sent: {}",
            result.reason
        );
        assert_eq!(
            result.project_fingerprint,
            enforcement.check_plan().fingerprint
        );
    }
    assert!(
        !aggregate(&results).is_green(),
        "a run whose routes could not be asked aggregated to green"
    );
}

#[test]
fn the_budget_decides_how_many_routes_are_asked_and_what_was_left_out_is_reported() {
    // The count is what makes *SURE checked your routes* a bounded promise. A
    // route the budget reached is a `NotProbed` and not a silence, and the
    // service's own record is what shows the bound was on the traffic and not
    // only on the reporting.
    let fixture = Fixture::new("the-budget");
    project(&fixture);
    let reading = fixture.reading();
    let enforcement = enforcement(&reading);
    let service = Service::start(1, answers_everything);
    let smoke = smoke(&reading, service.address, &enforcement, limits(1));

    assert_eq!(
        smoke.asked().len(),
        1,
        "the budget did not bound what SURE would ask"
    );
    let results = smoke.run();

    assert_eq!(results.len(), 1);
    assert_eq!(
        smoke.limits().most_routes(),
        1,
        "the smoke reports a budget other than the one it was given"
    );

    let beyond: Vec<&str> = smoke
        .not_probed()
        .iter()
        .filter(|entry| entry.because() == &NotProbedBecause::BeyondTheBudget)
        .map(|entry| entry.route().path())
        .collect();
    assert_eq!(
        beyond.len(),
        ASKED.len() - 1,
        "the routes the budget left out are not reported: {:?}",
        smoke.not_probed()
    );

    // Waiting for the window rather than for a join: if a second request were
    // coming it would arrive long before this, and a join would have to be told
    // how many to expect — which is the number under test.
    thread::sleep(EXCHANGE);
    let delivered = service.seen();
    assert_eq!(
        delivered.len(),
        1,
        "the budget bounded the report and not the traffic: {delivered:?}"
    );
    let _ = service.finish();
}

/// A project cannot write a control byte into SURE's own report.
///
/// **This is the defect class this repository calls the most serious one** — a
/// false green in the terminal rather than in a verdict — and it is the shape
/// `P5-T002` found in `runtime_start.rs`, reached here through a different door.
/// A route's path is read out of a project's source by `quoted`, which returns the
/// characters between the quotes unchanged, so a file that holds a real escape byte
/// inside a route's string carries that byte into every sentence SURE composes
/// about the route: `ESC [ 2K` clears the line it is printed on, and the route SURE
/// is reporting on could erase the report of itself.
///
/// **A route is read rather than dropped, and that is what makes the sentence
/// reachable.** A path carrying a control byte cannot go into a request line, so
/// this route is a [`NotProbedBecause::NotARequestTarget`] and never a check —
/// which puts it in the one list whose whole job is to name the route it did not
/// ask.
///
/// **The assertion is the invariant and not the spelling**: no sentence SURE
/// composes about this route may contain a control character at all, which is a
/// claim about every byte a project could choose rather than about the one that
/// prompted the test. The escape is asserted separately, because a fix that
/// deleted the byte would satisfy the invariant and leave a reader unable to see
/// what the project had written.
#[test]
fn a_control_byte_a_project_wrote_cannot_reach_a_sentence_sure_prints() {
    let fixture = Fixture::new("a-control-byte-in-a-path");
    fixture.write(
        "api/odd.py",
        // The escape between the quotes is a real byte. Written as the four
        // characters `\u{1b}` it would be harmless, and this test would be about
        // nothing — which is why the byte is spelled with a Rust escape here and
        // reaches the file as one byte.
        "from flask import Flask\n\
         app = Flask(__name__)\n\
         \n\
         @app.route('/x\u{1b}[2Ky')\n\
         def odd():\n\
         \x20   return 'odd'\n",
    );
    let reading = fixture.reading();

    assert!(
        reading.checks().is_empty(),
        "a path that cannot go into a request line became a check: {:?}",
        reading.checks()
    );
    let refused = reading.not_probed();
    assert_eq!(refused.len(), 1, "the reading is not about one route");
    assert_eq!(
        refused[0].because(),
        &NotProbedBecause::NotARequestTarget,
        "the route SURE read is not the route this test is about"
    );

    let route = refused[0].route();
    let anchor = route.anchor();
    for (what, sentence) in [
        ("the spelling", route.spelling()),
        ("the description", route.plain_description()),
        ("the line a reader is given", refused[0].plain_description()),
        ("the anchor's location", anchor.location.clone()),
        ("the anchor's locator", anchor.locator.clone()),
    ] {
        assert!(
            !sentence.chars().any(char::is_control),
            "{what} carries a byte out of the project: {sentence:?}"
        );
    }

    assert!(
        route.spelling().contains("\\u{001b}"),
        "{}",
        route.spelling()
    );
    assert_eq!(
        route.path(),
        "/x\u{1b}[2Ky",
        "the escape reached the path SURE would ask for"
    );
}

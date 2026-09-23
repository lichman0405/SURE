//! A project that declares a local service, checked by the real CLI on this
//! machine.
//!
//! `crates/sure-core/src/service_plan.rs` holds the plan one declaration becomes
//! and its unit tests hold the plan in isolation. What none of them can hold is
//! the whole road: a `sure.yaml` on disk, a user's own settings file that
//! authorises it, the `node` this machine has, the browser this machine has, and
//! the rows that come back from `sure check`. That road is what this file
//! measures, and it is the only place where a check that *starts a program* on
//! the machine running the tests is measured end to end.
//!
//! # The measurement of 2026-09-23, what it found, and the repair
//!
//! **This file was written against a defect and then found it; the defect is
//! fixed, and the record is here because the fix is one variable in one file.**
//! The first run of these cases under `host_confirmed` with `run_project_code`
//! planned the service check, admitted it, started it, and reported:
//!
//! ```text
//! start api   fail   the service ended by itself after 65 milliseconds, before
//!                    SURE could ask it anything (exit code 134); it wrote on
//!                    standard error:  3: ... node::Start+160 ...
//! ```
//!
//! the process aborting with no request ever reaching it. **The row keeps the
//! tail of that stack and not its header** — the excerpt it carries begins at
//! frame 3 — so the line that says what aborted came from starting the same
//! binary outside SURE with a cleared environment block, which is also how the
//! `SystemRoot` measurement below was taken. The cause was one line
//! of `crates/sure-core/src/service_plan.rs`: `declared_service` built the
//! command as `Environment::only(Vec::new())` — ADR 0014's decision 11, carried
//! out as an *empty* environment where what the decision asks for is an
//! *explicit* one — and the `node` this machine has cannot initialize without
//! `SystemRoot`. Run with nothing in its environment, node aborts before it
//! reads `server.js` at all:
//!
//! ```text
//! Assertion failed: ncrypto::CSPRNG(nullptr, 0)
//! ... node::InitializeOncePerProcessInternal(...) at src\node.cc:1204
//! ```
//!
//! That was measured outside SURE first, by starting this fixture's own
//! `server.js` with a process whose environment block was cleared: exit code
//! 134, about seventy milliseconds, no output on either stream but that stack.
//! Adding back **one** variable — `SystemRoot=C:\Windows` — and nothing else
//! made the same file start and serve, and `WINDIR`, `SystemDrive`, `TEMP`,
//! `TMP`, `PATH` and `USERPROFILE` were each tried alone and each still aborted.
//! So it is `SystemRoot` or nothing, and that is what the file now does:
//! `service_plan.rs`'s `service_environment` passes where Windows is installed,
//! read from SURE's own process when the plan is made, and passes nothing at all
//! on the platforms no measurement covers. The empty list that stood in its
//! place is gone rather than defaulted, and `service_environment`'s own doc
//! comment carries this measurement and the test beside it asserts the value, so
//! the next person to consider emptying it again has to argue with both.
//!
//! `node` v25.8.1, `C:\Program Files\nodejs\node.exe`, the machine's only
//! `node`, measured at `f614dc4` and repaired after it. **No assertion below was
//! weakened for it and none was weakened by the repair**: a case that says a
//! service must be running said so while the run was red, with the failing row
//! as its evidence, and the red proof of the repair is the same case run with
//! the environment emptied again, which puts the exit-134 row back. Nothing here
//! works around the defect either: the environment goes to the program SURE
//! starts, every launcher kind in this build *is* that program, and a fixture
//! that avoided the crash would have been a fixture checking something else.
//!
//! # What is on disk, and where
//!
//! Every fixture is one scratch directory (`sure_testkit::scratch::directory`,
//! under the checkout's `target/tmp`) holding a project whose name has spaces and
//! non-ASCII characters in it, three directories beside it, and one file:
//!
//! - `project with spaces/` — the project being checked: `sure.yaml`, `server.js`
//!   and `helper.js`. Nothing else: no manifest, no `node_modules`, and no
//!   dependency of any kind, so nothing is installed and nothing is fetched.
//! - `markers/` — everything the fixture service writes while it runs: what it
//!   started, that it is alive, what was asked of it, and how it ended. The test
//!   reads these *after* the CLI has returned, which is why they are files rather
//!   than anything the service says.
//! - `store/` — `--store-dir`.
//! - `user-settings.yaml` — `--settings-file`, the file outside the project, and
//!   the only layer that can grant anything (`crates/sure-cli/src/grants.rs` says
//!   why that is the whole design).
//!
//! **The markers are outside the project deliberately.** A file written inside
//! the checked project during a run moves the project's fingerprint, and a run
//! that moved it withdraws its own runtime passes at stage 10. That is true and
//! it is one of the cases below; it is not something every success case should
//! have to work around, so the fixture's own evidence is written where the
//! fingerprint does not see it.
//!
//! # The port
//!
//! Each fixture binds `127.0.0.1:0`, reads the port the operating system chose
//! and **drops the listener** before writing that number into `sure.yaml`. It has
//! to be written into the file and cannot be passed some other way: the command
//! SURE runs is `node server.js` — one argument, an environment that is the
//! operating system's own directory rather than SURE's (see the measurement
//! above), and no argv of SURE's making. Dropping the listener leaves a window
//! between choosing the port and the run using it, and nothing here closes that
//! window; a machine that takes the port in that window makes the fixture service
//! fail to bind, and the check reports that failure as the failure it is rather
//! than as a pass.
//!
//! # What every case asserts on
//!
//! Rows rather than sentences wherever a row exists. `details.check_results` is
//! the run's own list of [`CheckResult`]s and each row is found by its `id`,
//! which is `checks::check_id(name, tag)` — `chk_`, the tag with everything but
//! `[a-z0-9]` dropped, then sixteen hexadecimal characters. The digest half is
//! never written down here: a test that hardcoded one would be asserting its own
//! arithmetic about a digest rather than the row the run produced.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use serde_json::Value;

const SURE: &str = env!("CARGO_BIN_EXE_sure");

/// The name the fixture's one declaration gives its service, and the title its
/// service row carries.
///
/// **The browser row is titled by its verdict and not by the plan**, so there is
/// no second constant beside this one. `ServicePlan` proposes that check as
/// `check api in a browser` — asserted where it is decided, in `service_plan.rs`'s
/// own unit tests — and the row in `details.check_results` reads
/// `browser probe: <url>` instead, because that is what
/// `crates/sure-core/src/browser.rs` titles a verdict with. The case below
/// asserts the second, address and all, because that is the one the report a
/// person reads carries.
const NAME: &str = "api";
const SERVICE_TITLE: &str = "start api";

/// The readable half of the two rows' identifiers. See `service_plan.rs`'s
/// `SERVICE_TAG` and `SERVICE_PAGE_TAG` for why they are two tags.
const SERVICE_TAG: &str = "service";
const PAGE_TAG: &str = "servicepage";

/// How long the stop case watches a marker after the CLI has returned.
///
/// **Longer than the fixture's own heartbeat interval by more than an order of
/// magnitude**, so a process that is still running is certain to have written
/// again: it beats every fifty milliseconds, and a second and a fifth of silence
/// is not something a live one produces.
const SILENCE: Duration = Duration::from_millis(1_200);

/// The user's own settings file that authorises nothing.
const INSPECT_ONLY: &str = "execution:\n  mode: inspect_only\n";

/// The user's own settings file that authorises the service and the look.
///
/// `analysis.provider` is the one setting in either file that asks for
/// `connect_service` — a model service is a service, and `ProjectRequest::
/// ExternalAnalysis` is what maps onto that permission — and it is where the
/// browser row's permission comes from. **Nothing consults the provider**: stage
/// 8 reports `not_part_of_work` for a configured provider because no check in
/// this build asks for model work, so this file starts no model process and opens
/// no connection. What it does is put `run_project_code` and `connect_service` in
/// force, which is the smallest honest way to ask for both.
const AUTHORISED: &str = "execution:\n  mode: host_confirmed\nanalysis:\n  provider: claude_cli\n";

/// The same settings with the mode in a container, which this build cannot run.
const CONTAINER: &str = "execution:\n  mode: container\nanalysis:\n  provider: claude_cli\n";

/// The service, as a file the machine's `node` can start with no arguments.
///
/// It listens on loopback only, and on one port, which is substituted here
/// because the declaration has nowhere else to put it. Everything it writes goes
/// to `../markers`, which is outside the project. Its heartbeat is a counter
/// rather than a timestamp, so that "this file has moved since the run recorded
/// it" is a comparison of two numbers the test can print.
fn server_js(port: u16) -> String {
    SERVER_JS.replace("__PORT__", &port.to_string())
}

const SERVER_JS: &str = r#"'use strict';
// The service this crate's tests start. Every import is a `node:` builtin, so
// there is nothing to install and nothing to fetch, and the only address it
// listens on is the loopback one.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const { spawn } = require('node:child_process');

const PORT = __PORT__;
const markers = path.resolve(__dirname, '..', 'markers');

// **A marker is written whole or it is not written at all.** `writeFileSync` opens with
// `O_TRUNC`, so it empties the file before the bytes go in: a reader that arrives inside
// that window sees an empty file, and a writer that is *killed* inside it leaves one
// behind for good. That is not a theory — `taskkill /T /F` ends this process outright, and
// on CI the stop landed exactly while this function was writing `heartbeat`, so the test
// read an empty counter and failed with `the heartbeat is a number: ParseIntError { kind:
// Empty }`. Writing under a scratch name and renaming it into place makes the replacement
// a single step, so a reader sees the previous value or the next one and never a
// half-written file, and a kill leaves the previous complete value rather than a hole.
//
// **The rename is refused, not queued, while another process holds the target open.** On
// Windows `MoveFileEx` answers `EPERM` for that, and inside a timer callback a refusal is
// an uncaught throw that ends this service — a collision with a reader would kill the
// thing being measured. Measured, not assumed: `target/tmp/rename-alone.mjs` renames five
// bytes over themselves 5469 times in two seconds with no reader and is refused **zero**
// times, while `target/tmp/marker-write-probe.mjs` refuses the same rename from a reader
// polling without pause on **20 rounds of 20**. So the rename is retried for half the beat
// interval below — 25 ms of the 50 ms between beats, because a beat that waits longer than
// that is behind the next one anyway — and only then written in place: that is the old
// spelling and it can be caught mid-truncation, which is worse than the retry gives and
// much better than a beat that never lands or a service that throws.
const park = new Int32Array(new SharedArrayBuffer(4));

function note(name, text) {
  const target = path.join(markers, name);
  const scratch = `${target}.writing`;
  fs.writeFileSync(scratch, text);
  const until = Date.now() + 25;
  for (;;) {
    try {
      fs.renameSync(scratch, target);
      return;
    } catch (error) {
      if (error.code !== 'EPERM' && error.code !== 'EACCES') throw error;
      if (Date.now() >= until) break;
      Atomics.wait(park, 0, 0, 1);
    }
  }
  fs.writeFileSync(target, text);
  fs.rmSync(scratch, { force: true });
}

// A switch the test sets before the run and never inside the project: a file in
// the project would move the fingerprint, which is one of the cases below.
function asked(name) {
  return fs.existsSync(path.join(markers, name));
}

function stamp(line) {
  fs.appendFileSync(path.join(markers, 'requests.log'), `${Date.now()} ${line}\n`);
}

let beats = 0;

const server = http.createServer((request, response) => {
  stamp(`${request.method} ${request.url}`);
  if (request.url === '/readyz') {
    response.writeHead(200, { 'content-type': 'text/plain' });
    response.end('ready');
    return;
  }
  if (request.url === '/error') {
    response.writeHead(500, { 'content-type': 'text/plain' });
    response.end('this route is meant to be an error');
    return;
  }
  if (request.url === '/') {
    response.writeHead(200, { 'content-type': 'text/html' });
    response.end('<!doctype html><title>the fixture page</title><h1>up</h1>');
    return;
  }
  response.writeHead(404, { 'content-type': 'text/plain' });
  response.end('not found');
});

// A bind that cannot happen is loud rather than silent. The port-taken case
// reads this exit code and the line the service wrote, and neither is a service
// that quietly was not there.
server.on('error', (error) => {
  process.stderr.write(`the fixture service could not start: ${error}\n`);
  process.exit(2);
});

// Evidence for a run that ends by itself, and evidence of nothing at all for one
// SURE stops: `taskkill /T /F` ends the tree outright and runs no handler, so a
// stop leaves no `exiting` behind. The pass cases assert exactly that.
process.on('exit', () => note('exiting', 'the service ended by itself'));
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => process.exit(0));
}

server.listen({ host: '127.0.0.1', port: PORT, exclusive: true }, () => {
  // Written before the first beat, so the counter this file records is always
  // behind the counter the heartbeat file ends at.
  note('started.json', JSON.stringify({ pid: process.pid, port: PORT, beat: beats }));
  setInterval(() => {
    beats += 1;
    note('heartbeat', String(beats));
  }, 50);
  if (asked('exit-immediately')) {
    note('exiting', 'the service ended by itself');
    process.exit(0);
  }
  if (asked('write-into-project')) {
    fs.writeFileSync(path.join(__dirname, 'wrote-during-the-run.txt'), `${Date.now()}\n`);
  }
  spawn(process.execPath, [path.join(__dirname, 'helper.js')], { stdio: 'ignore' });
});
"#;

/// The descendant of the service, which listens on nothing.
///
/// **It exists so that a stop can be measured rather than trusted.** A stop that
/// reached only the process SURE started leaves this one running and beating, and
/// the stop case below reads its counter before and after the stop to catch
/// exactly that — on the platform whose stop reaches the whole tree. Off Windows
/// it goes on beating and nothing reads it, which the stop case names as a gap
/// rather than passing over in silence.
///
/// **It ends itself after five minutes**, and that is not a detail about the
/// service: on a platform whose stop reaches one process, this descendant is
/// still running when the test ends, and a test cannot clean up a process it
/// cannot reach. Five minutes is far longer than any run here and short enough
/// that nothing outlives the suite.
const HELPER_JS: &str = r#"'use strict';
const fs = require('node:fs');
const path = require('node:path');

const markers = path.resolve(__dirname, '..', 'markers');
// Whole or not at all, for the reason the service's own `note` gives — and refused, not
// queued, while a reader holds the file open, which is why the rename is retried there
// too. This file is written by a process that outlives the run on platforms whose stop
// reaches one process, so a kill landing inside `writeFileSync`'s truncate window would
// leave it empty and the reading would be about the kill rather than about the counter.
const park = new Int32Array(new SharedArrayBuffer(4));

function note(name, text) {
  const target = path.join(markers, name);
  const scratch = `${target}.writing`;
  fs.writeFileSync(scratch, text);
  const until = Date.now() + 25;
  for (;;) {
    try {
      fs.renameSync(scratch, target);
      return;
    } catch (error) {
      if (error.code !== 'EPERM' && error.code !== 'EACCES') throw error;
      if (Date.now() >= until) break;
      Atomics.wait(park, 0, 0, 1);
    }
  }
  fs.writeFileSync(target, text);
  fs.rmSync(scratch, { force: true });
}
let beats = 0;
const beat = setInterval(() => {
  beats += 1;
  note('helper-heartbeat', String(beats));
}, 50);
setTimeout(() => {
  clearInterval(beat);
  process.exit(0);
}, 5 * 60 * 1000);
"#;

fn write(path: &Path, text: &str) {
    std::fs::write(path, text)
        .unwrap_or_else(|error| panic!("could not write {}: {error}", path.display()));
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()))
}

/// The declaration as `sure.yaml` spells it, with the browser preference it
/// needs to become two rows.
///
/// `checks.browser_probe` defaults to `auto`, and `auto` does not plan a page
/// check — a declaration has to be looked at because somebody asked, not because
/// a file mentioned a page — so the fixture asks.
fn declaration_yaml(port: u16, entry: &str, readiness: &str, page: Option<&str>) -> String {
    format!(
        "checks:\n  browser_probe: always\n{}",
        services_yaml(port, entry, readiness, page)
    )
}

/// The `services:` list on its own, at the indentation `checks:` wants.
///
/// Split out from the file above because the two cases about a preference that
/// *left a check out* need the same declaration under a different `checks:`
/// block — one setting turned off, one left at its default — and a declaration
/// written twice is a second thing to keep in step.
fn services_yaml(port: u16, entry: &str, readiness: &str, page: Option<&str>) -> String {
    let page = page.map_or_else(String::new, |page| format!("      page: {page}\n"));
    format!(
        "  services:\n    - name: {NAME}\n      launcher:\n\
         \x20       kind: node_entry\n        entry: {entry}\n      port: {port}\n\
         \x20     readiness: {readiness}\n{page}"
    )
}

/// The same file with an `execution:` block, which is what a project writes when
/// it asks for something it cannot grant itself.
fn with_project_execution(declaration: &str, mode: &str) -> String {
    format!("execution:\n  mode: {mode}\n{declaration}")
}

/// A port nothing is listening on, chosen by the operating system.
fn free_port() -> u16 {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).expect("a loopback port the operating system chooses");
    let port = listener
        .local_addr()
        .expect("the address the listener was given")
        .port();
    drop(listener);
    port
}

/// One project, and the four things around it that a run must not confuse it
/// with.
struct Fixture {
    project: PathBuf,
    markers: PathBuf,
    store: PathBuf,
    settings: PathBuf,
    port: u16,
}

impl Fixture {
    fn new(pool: &str) -> Self {
        let root = sure_testkit::scratch::directory(pool, "declared service");
        let project = root.join("project with spaces");
        let markers = root.join("markers");
        let store = root.join("store");
        for directory in [&project, &markers, &store] {
            std::fs::create_dir_all(directory).expect("a scratch directory");
        }
        let port = free_port();
        write(&project.join("server.js"), &server_js(port));
        write(&project.join("helper.js"), HELPER_JS);
        let fixture = Self {
            project,
            markers,
            store,
            settings: root.join("user-settings.yaml"),
            port,
        };
        fixture.declare(port, "server.js", "/readyz", Some("/"));
        fixture.settings(INSPECT_ONLY);
        fixture
    }

    /// An empty directory beside the project, for the one case that has to give
    /// a run a place with no browser in it.
    ///
    /// **Made when it is asked for, and asked for on Windows only** —
    /// `a_machine_with_no_browser_reports_an_absence_and_never_a_pass` is where
    /// the question can be asked at all, and its documentation says why. A
    /// platform that cannot ask it does not carry the directory either.
    #[cfg(windows)]
    fn nowhere(&self) -> PathBuf {
        let directory = self
            .project
            .parent()
            .expect("the scratch root above the project")
            .join("nowhere a browser lives");
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        directory
    }

    /// Write the project's own `sure.yaml`, replacing whatever was there.
    fn write_project(&self, text: &str) {
        write(&self.project.join("sure.yaml"), text);
    }

    /// Write the declaration this fixture started from, or a variant of it.
    fn declare(&self, port: u16, entry: &str, readiness: &str, page: Option<&str>) {
        self.write_project(&declaration_yaml(port, entry, readiness, page));
    }

    /// Write the user's own settings file, which is outside the project.
    fn settings(&self, text: &str) {
        write(&self.settings, text);
    }

    fn marker(&self, name: &str) -> PathBuf {
        self.markers.join(name)
    }

    /// A marker the fixture service reads at startup, set before a run.
    fn ask_for(&self, name: &str) {
        write(&self.marker(name), "");
    }

    /// Run the CLI over this project, with this fixture's store and settings.
    ///
    /// The environment is the test's own, minus `CARGO_NET_OFFLINE`'s absence —
    /// `sure` never invokes Cargo, and the variable is set here because every
    /// other CLI test in this crate sets it: the fixture has no network need, and
    /// a test that accidentally acquired one should fail rather than fetch.
    fn run(&self, format: &str) -> Output {
        Command::new(SURE)
            .arg("--store-dir")
            .arg(&self.store)
            .arg("--settings-file")
            .arg(&self.settings)
            .args(["--format", format, "check"])
            .arg(&self.project)
            .env("CARGO_NET_OFFLINE", "true")
            .env_remove("CARGO_TARGET_DIR")
            .stdin(Stdio::null())
            .output()
            .expect("the CLI starts")
    }

    /// The same run, in the JSON format, with the frame it produced.
    fn frame(&self) -> Value {
        let output = self.run("json");
        let stdout = String::from_utf8(output.stdout).expect("the JSON report is UTF-8");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.code().is_some(),
            "the CLI did not finish normally: {stdout}\n{stderr}"
        );
        serde_json::from_str(&stdout).expect("one JSON frame")
    }
}

/// Whether `id` is the row `checks::check_id(name, tag)` produces for this
/// fixture's service.
///
/// `chk_`, the tag, then exactly sixteen hexadecimal characters. **The tag is
/// matched against the whole of what follows `chk_` and not as a prefix**, which
/// is what keeps the `service` row and the `servicepage` row apart: one is a
/// prefix of the other, and a `starts_with` would find three rows where the run
/// produced two.
fn is_row(id: &str, tag: &str) -> bool {
    let Some(body) = id.strip_prefix("chk_") else {
        return false;
    };
    let Some(digest) = body.strip_prefix(tag) else {
        return false;
    };
    digest.len() == 16
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Every row of the run that is this fixture's `tag` row.
fn rows<'a>(frame: &'a Value, tag: &str) -> Vec<&'a Value> {
    frame["details"]["check_results"]
        .as_array()
        .unwrap_or_else(|| panic!("the run has no check rows: {frame}"))
        .iter()
        .filter(|row| row["id"].as_str().is_some_and(|id| is_row(id, tag)))
        .collect()
}

/// The one row of the run that is this fixture's `tag` row.
fn row<'a>(frame: &'a Value, tag: &str) -> &'a Value {
    let found = rows(frame, tag);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one {tag} row, found {}: {found:?}",
        found.len()
    );
    found[0]
}

/// What a row says its status is.
fn status(row: &Value) -> &str {
    row["status"]
        .as_str()
        .unwrap_or_else(|| panic!("a row with no status: {row}"))
}

/// A row's plain-language line, which is where a refusal is explained.
fn reason(row: &Value) -> String {
    row["reason"]
        .as_str()
        .unwrap_or_else(|| panic!("a row with no reason: {row}"))
        .to_owned()
}

/// The stage-4 sentence, which is where a declaration that could not become a
/// check is reported.
fn plan_detail(frame: &Value) -> String {
    frame["details"]["stages"]
        .as_array()
        .unwrap_or_else(|| panic!("the frame lists its stages: {frame}"))
        .iter()
        .find(|stage| stage["stage"] == "plan")
        .and_then(|stage| stage["detail"].as_str())
        .unwrap_or_else(|| panic!("no plan stage in {frame}"))
        .to_owned()
}

/// The run finished rather than stopping part way through a stage.
fn finished(frame: &Value) {
    assert_eq!(frame["details"]["state"], "finished", "{frame}");
    assert_eq!(frame["details"]["stopped_at"], Value::Null, "{frame}");
}

/// Nothing the fixture service would have written is there.
fn nothing_started(fixture: &Fixture, frame: &Value) {
    for marker in [
        "started.json",
        "heartbeat",
        "helper-heartbeat",
        "requests.log",
    ] {
        assert!(
            !fixture.marker(marker).exists(),
            "a run that should have started nothing wrote markers/{marker}: {frame}"
        );
    }
}

/// The declaration this file is about, as the project writes it, parses.
///
/// **This is the assertion that the documented shape is a shape.** The first
/// draft of `Launcher` was an externally tagged enum written as a nested
/// mapping, and `serde_yaml_ng` refuses that spelling outright — *invalid type:
/// map, expected a YAML tag starting with '!'* — so a `sure.yaml` written the way
/// the documentation showed it did not load at all. Every unit test in that
/// round built the struct in Rust, none of them read the format, and nothing
/// failed. This reads the format: the block below is the one
/// `sure.example.yaml` and `docs/architecture/CONFIG_REFERENCE.md` print, and it
/// goes through the same `Config::from_yaml` the product loads a project's file
/// with.
#[test]
fn the_documented_declaration_parses_through_the_product_loader() {
    let config = sure_core::config::Config::from_yaml(&declaration_yaml(
        4310,
        "server.js",
        "/health",
        Some("/"),
    ))
    .unwrap_or_else(|error| panic!("the documented declaration does not load: {error}"));
    let declared = &config.checks.services;
    assert_eq!(declared.len(), 1, "{declared:?}");
    assert_eq!(declared[0].name, NAME);
    assert_eq!(declared[0].directory, None);
    assert_eq!(declared[0].port, 4310);
    assert_eq!(declared[0].readiness, "/health");
    assert_eq!(declared[0].page.as_deref(), Some("/"));
    assert_eq!(
        declared[0].launcher,
        sure_core::config::Launcher::NodeEntry {
            entry: "server.js".to_owned()
        }
    );
}

/// Nothing runs under the default mode, and both rows say so rather than
/// disappearing.
///
/// Then the same project asks for execution authority in its own file, which is
/// the one thing a project file cannot have: the run is unchanged, the mode in
/// force is still `inspect_only`, and the refusal is a value in the frame rather
/// than an absence.
#[test]
fn nothing_runs_under_the_default_mode_and_both_checks_say_so() {
    let fixture = Fixture::new("declared service é中文");
    let frame = fixture.frame();

    finished(&frame);
    assert_eq!(frame["details"]["grants"]["execution_mode"], "inspect_only");
    for tag in [SERVICE_TAG, PAGE_TAG] {
        let row = row(&frame, tag);
        assert_ne!(status(row), "pass", "{row}");
        assert_eq!(
            row["not_checked_reason"], "execution_not_authorized",
            "{row}"
        );
    }
    assert_eq!(status(row(&frame, SERVICE_TAG)), "skipped", "{frame}");
    assert_eq!(status(row(&frame, PAGE_TAG)), "skipped", "{frame}");
    nothing_started(&fixture, &frame);

    // The same declaration, and a project authorising itself. Nothing else
    // changes: the mode is the user's to set, and the project's file is one more
    // place to state a request.
    fixture.write_project(&with_project_execution(
        &declaration_yaml(fixture.port, "server.js", "/readyz", Some("/")),
        "host_confirmed",
    ));
    let frame = fixture.frame();

    finished(&frame);
    assert_eq!(frame["details"]["grants"]["execution_mode"], "inspect_only");
    assert!(
        frame["details"]["grants"]["refused"]
            .as_array()
            .expect("the frame lists what was refused")
            .iter()
            .any(|refusal| refusal["request"] == "run_project_code"
                && refusal["asked_by"] == serde_json::json!(["project"])),
        "a project's own file asking for execution was not recorded as refused: {frame}"
    );
    for tag in [SERVICE_TAG, PAGE_TAG] {
        let row = row(&frame, tag);
        assert!(
            row["not_checked_reason"] == "execution_not_authorized",
            "a project's own request for authority must change nothing at all: {row}"
        );
    }
    nothing_started(&fixture, &frame);
}

/// A run a user granted `run_project_code` may not fall back to the host when
/// the mode says container.
///
/// This build cannot run a check in a container, and the honest answer is a row
/// that says so — never the host execution the user asked to be spared. The
/// grant is real here (the mode in force is `container`, which is not what a
/// user gets by granting nothing), so what stops the check is the mode and
/// nothing else.
#[test]
fn container_mode_does_not_fall_back_to_the_host() {
    let fixture = Fixture::new("declared service 容器");
    fixture.settings(CONTAINER);
    let frame = fixture.frame();

    finished(&frame);
    assert_eq!(frame["details"]["grants"]["execution_mode"], "container");
    assert!(
        frame["details"]["grants"]["permissions"]
            .as_array()
            .expect("the frame lists the permissions in force")
            .contains(&Value::String("run_project_code".to_owned())),
        "the user's grant must be real for this to be a measurement of the mode: {frame}"
    );
    for tag in [SERVICE_TAG, PAGE_TAG] {
        let row = row(&frame, tag);
        assert_ne!(status(row), "pass", "{row}");
        assert_eq!(
            row["not_checked_reason"], "container_execution_unavailable",
            "{row}"
        );
    }
    nothing_started(&fixture, &frame);
}

/// A run the user authorised starts the service, asks it, looks at the page, and
/// reports both rows against the state it checked.
#[test]
fn an_authorised_run_starts_the_service_and_looks_at_its_page() {
    let fixture = Fixture::new("declared service 浏览器");
    fixture.settings(AUTHORISED);
    let frame = fixture.frame();

    finished(&frame);
    assert_eq!(
        frame["details"]["grants"]["execution_mode"],
        "host_confirmed"
    );

    // The row first, because it is the run's own statement about what it did: a
    // marker that is not there says only that a file is missing, and the row
    // says what happened to the process that should have written it.
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(service["title"], SERVICE_TITLE, "{service}");
    assert_eq!(
        rows(&frame, SERVICE_TAG).len(),
        1,
        "one declaration is one service row: {:?}",
        rows(&frame, SERVICE_TAG)
    );
    assert_eq!(service["critical"], true, "{service}");
    assert_eq!(status(service), "pass", "{service}");

    // What the process did, from the outside. The pid is the machine's own
    // answer about which process ran: it is not the test's, so the file is not
    // one an already-running test could have left.
    let started: Value = serde_json::from_str(&read(&fixture.marker("started.json")))
        .expect("the service recorded what it started");
    assert_ne!(
        started["pid"].as_u64(),
        Some(u64::from(std::process::id())),
        "the marker is the service's own: {started}"
    );
    assert_eq!(started["port"].as_u64(), Some(u64::from(fixture.port)));
    let beat_when_started = started["beat"].as_u64().expect("the beat it started at");
    let beat_at_the_end: u64 = read(&fixture.marker("heartbeat"))
        .trim()
        .parse()
        .expect("the heartbeat is a number");
    assert!(
        beat_at_the_end > beat_when_started,
        "the heartbeat did not move after the service recorded {beat_when_started}: it ended at \
         {beat_at_the_end}"
    );

    // The page. **The branch this machine is on is the point**: on a machine
    // with a browser, a row that came back `skipped` or `unknown` would be a
    // check that never happened described as a look; on a machine without one,
    // an absence is the answer and a `pass` would be a fabricated one.
    let page = row(&frame, PAGE_TAG);
    // **The row's title is the verdict's, not the plan's.** `ServicePlan`
    // proposes this check as `check api in a browser` — asserted in
    // `service_plan.rs`'s own unit tests, where it is decided — and the row a
    // report carries is titled by the browser verdict instead, which names the
    // instrument and the address. Asserting the address as well is the stronger
    // claim of the two: a row titled with another fixture's port would be a look
    // at the wrong page.
    assert_eq!(
        page["title"],
        format!("browser probe: http://127.0.0.1:{}/", fixture.port),
        "{page}"
    );
    match sure_core::browser_driver::find_installed_browser() {
        None => {
            eprintln!("no browser on this machine, so the page row is an absence");
            assert_ne!(status(page), "pass", "{page}");
            assert_eq!(page["not_checked_reason"], "tool_unavailable", "{page}");
            assert!(
                reason(page).contains("needs a browser tool on this computer"),
                "a missing browser is not explained as one: {page}"
            );
        }
        Some(browser) => {
            eprintln!("the browser this machine offered: {}", browser.display());
            assert!(
                matches!(status(page), "pass" | "fail"),
                "a browser at {} was driven, so the row is what the look found and never {}: \
                 {page}",
                browser.display(),
                status(page)
            );
        }
    }

    // Both rows are about the state the run read, and a pass about no state at
    // all would be a pass nobody can carry forward.
    let fingerprint = frame["details"]["report"]["project_fingerprint"].clone();
    assert_ne!(fingerprint, Value::Null, "{frame}");
    for row in [service, page] {
        assert_eq!(row["project_fingerprint"], fingerprint, "{row}");
    }

    // A service SURE stopped runs no exit handler — see the fixture — so this
    // file is evidence that the run ended by stopping it and not by it ending.
    assert!(
        !fixture.marker("exiting").exists(),
        "the service ended by itself: {}",
        read(&fixture.marker("exiting"))
    );

    let human = fixture.run("human");
    let stdout = String::from_utf8(human.stdout).expect("the human report is UTF-8");
    assert!(human.status.code().is_some(), "{stdout}");
    assert!(stdout.contains("pass: start api"), "{stdout}");
}

/// The page check is not covered by the service check's authorisation.
///
/// A run the user granted `run_project_code` and not the service connection
/// starts the service and does not open a page on it: the page row says which
/// permission stopped it, and no browser fetched anything. The two permissions
/// are two, and a run that held one of them would otherwise look like a run that
/// held both.
#[test]
fn the_page_is_not_covered_by_the_service_authorisation() {
    let fixture = Fixture::new("declared service 一个权限");
    fixture.settings("execution:\n  mode: host_confirmed\n");
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(status(service), "pass", "{service}");
    let page = row(&frame, PAGE_TAG);
    assert_eq!(status(page), "skipped", "{page}");
    assert_eq!(
        page["not_checked_reason"], "execution_not_authorized",
        "{page}"
    );

    // The measurement behind the row: the service was asked its readiness
    // question, and nothing fetched the page.
    let log = read(&fixture.marker("requests.log"));
    assert!(log.contains("/readyz"), "{log}");
    assert!(
        !log.lines().any(|line| line.ends_with("GET /")),
        "a page was fetched by a run that was not permitted to look at one: {log}"
    );
}

/// The page is fetched only after the readiness route has answered.
///
/// *A service is up* and *a page on it is there* are two questions, and the
/// order they are asked in is the difference between a look and a guess. The
/// fixture logs every request with the moment it arrived, so this is read off
/// what the service saw rather than inferred from what SURE reported.
#[test]
fn the_opened_page_is_fetched_after_the_readiness_route_answered() {
    let fixture = Fixture::new("declared service 顺序");
    fixture.settings(AUTHORISED);
    let frame = fixture.frame();
    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(status(service), "pass", "{service}");

    let log = read(&fixture.marker("requests.log"));
    let asked = |wanted: &str| {
        log.lines()
            .position(|line| line.split_whitespace().nth(2) == Some(wanted))
            .unwrap_or_else(|| panic!("the service was never asked for {wanted}: {log}"))
    };
    let readiness = asked("/readyz");
    let page = asked("/");
    assert!(
        readiness < page,
        "the page was fetched at line {page} and the readiness route answered at line \
         {readiness}: {log}"
    );
}

/// The process SURE started, and the child it started, are both stopped.
///
/// The fixture beats every fifty milliseconds while it is alive, so *this file
/// has not moved in over a second* is a measurement of a stopped process rather
/// than a reading of SURE's own claim to have stopped one.
#[test]
fn the_started_process_and_its_descendant_are_stopped() {
    // No page: this case is about the stop, and a declaration that named one
    // would put a second service start in front of the one being measured.
    let fixture = Fixture::new("declared service 停止");
    fixture.declare(fixture.port, "server.js", "/readyz", None);
    fixture.settings(AUTHORISED);
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(status(service), "pass", "{service}");
    assert!(
        fixture.marker("started.json").exists(),
        "the service never wrote the marker it writes once it is listening: {service}"
    );

    // The descendant ran at all. That is the whole of what the non-Windows legs
    // say about it, and it is what makes the Windows comparison below mean
    // something: a helper that never started leaves no file, and a missing file
    // cannot be one that stopped moving.
    let helper = fixture.marker("helper-heartbeat");
    assert!(
        helper.exists(),
        "the service's own child never wrote its heartbeat, so there is nothing whose stopping \
         could be measured"
    );

    let heartbeat = read(&fixture.marker("heartbeat"));
    // **Read only where it is compared.** The comparison below is `#[cfg(windows)]`
    // because this platform's stop reaches the whole tree while the others stop the one
    // process SURE started, and off Windows the binding would be a read nobody uses —
    // which `-D warnings` refuses, and did: both non-Windows legs went red on
    // *unused variable: `helper_beat`* before either of them reached a single test.
    #[cfg(windows)]
    let helper_beat = read(&helper);
    std::thread::sleep(SILENCE);
    assert_eq!(
        read(&fixture.marker("heartbeat")),
        heartbeat,
        "the service SURE started was still running after the run returned"
    );

    // **The descendant is the half only Windows reaches, and only Windows
    // reads.** `crates/sure-core/src/process/terminate.rs` documents the
    // mechanism and the difference: this platform runs `taskkill /T`, which
    // reaches everything the service started, and every other platform runs
    // `Child::kill` on the one process SURE holds — reporting `Stop::ProcessOnly`
    // for exactly that reason.
    //
    // So on those platforms the helper is still beating when this test returns.
    // Nothing here reads it after the stop to find that out: asserting a frozen
    // helper there would contradict the mechanism written down one file over, and
    // reading it merely to have read it leaves a binding no comparison uses,
    // which `-D warnings` refuses — and did, on both non-Windows legs, before
    // either of them reached a single test. *The helper's survival is unmeasured
    // off Windows*, and the existence check above is the whole of what those legs
    // say about this descendant. That is a named gap rather than a passed check,
    // and it is why `HELPER_JS` ends itself after five minutes: no test on those
    // platforms can reach a process that outlives the run.
    #[cfg(windows)]
    assert_eq!(
        read(&helper),
        helper_beat,
        "the service's child outlived the run on a platform whose stop reaches the whole tree"
    );
}

/// A declaration SURE will not act on is refused with a sentence and does not
/// stop the run.
///
/// The entry file is deleted and the declaration still names it. The run
/// finishes, stage 4 says what is wrong with the declaration, no service row
/// exists to pass — and nothing was started.
#[test]
fn a_declaration_naming_a_file_that_is_not_there_is_refused() {
    let fixture = Fixture::new("declared service 没有入口");
    fixture.settings(AUTHORISED);
    std::fs::remove_file(fixture.project.join("server.js")).expect("the entry is deleted");
    let frame = fixture.frame();

    finished(&frame);
    assert!(
        rows(&frame, SERVICE_TAG).is_empty(),
        "a service SURE will not start still has a row: {:?}",
        rows(&frame, SERVICE_TAG)
    );
    assert!(
        rows(&frame, PAGE_TAG).is_empty(),
        "a page on a service nobody starts was planned: {:?}",
        rows(&frame, PAGE_TAG)
    );
    let detail = plan_detail(&frame);
    assert!(
        detail.contains("declared service(s) could not become a check")
            && detail.contains("the declared entry server.js is not a file that is there"),
        "{detail}"
    );
    // The run did its other work: stage 4 is not the end of it.
    assert_eq!(
        frame["details"]["stages"].as_array().map(Vec::len),
        Some(12),
        "the run did not walk its stages: {frame}"
    );
    nothing_started(&fixture, &frame);
}

/// A path that cannot be written into a request line is refused before anything
/// is started.
///
/// A backslash is the case this platform produces by accident: a person writing
/// `\health` means a separator, the request-target grammar does not, and SURE
/// refuses the path rather than asking for something nobody named. The refusal
/// comes from the endpoint constructor, so nothing reaches the filesystem or the
/// network on the way.
///
/// **`//evil.example/` is not this case**, although it looks like one: the
/// endpoint's rule accepts any graphic ASCII path that starts with `/` and
/// carries no backslash, so a protocol-relative spelling is a *legal* path on
/// loopback and not a refused one. What it can never be is a request to another
/// host — the endpoint has no host field — and that is a property of the type
/// rather than a case here.
#[test]
fn a_readiness_path_that_is_not_a_request_line_is_refused() {
    let fixture = Fixture::new("declared service 反斜杠");
    fixture.settings(AUTHORISED);
    // Single-quoted YAML, so the backslash reaches the loader as one: a
    // double-quoted scalar would refuse `\h` as an unknown escape and the test
    // would be measuring YAML rather than the endpoint.
    fixture.declare(fixture.port, "server.js", "'\\health'", None);
    let frame = fixture.frame();

    finished(&frame);
    assert!(
        rows(&frame, SERVICE_TAG).is_empty(),
        "a path the endpoint refuses still became a check: {:?}",
        rows(&frame, SERVICE_TAG)
    );
    let detail = plan_detail(&frame);
    assert!(
        detail.contains("declares the path")
            && detail.contains("cannot be written into a request line")
            && detail.contains("\"\\\\health\""),
        "{detail}"
    );
    nothing_started(&fixture, &frame);
}

/// A declaration that says port zero is refused, and SURE picks no port in its
/// place.
#[test]
fn a_declaration_that_names_port_zero_is_refused() {
    let fixture = Fixture::new("declared service 端口零");
    fixture.settings(AUTHORISED);
    fixture.declare(0, "server.js", "/readyz", None);
    let frame = fixture.frame();

    finished(&frame);
    assert!(
        rows(&frame, SERVICE_TAG).is_empty(),
        "{:?}",
        rows(&frame, SERVICE_TAG)
    );
    let detail = plan_detail(&frame);
    assert!(
        detail.contains("declares port 0")
            && detail.contains("nothing was picked in the project's place"),
        "{detail}"
    );
    nothing_started(&fixture, &frame);
}

/// A port something else is holding is reported, and is never a pass.
///
/// The test holds the port itself, which is the only honest way to be sure
/// something else has it: `exclusive` in the fixture's listen call means the
/// service's own bind fails rather than sharing the address, so what the run
/// sees is the failure a real port collision produces.
#[test]
fn a_port_something_else_is_holding_is_reported_and_never_passed() {
    let fixture = Fixture::new("declared service 端口被占");
    fixture.settings(AUTHORISED);
    let held = TcpListener::bind(("127.0.0.1", fixture.port))
        .expect("the test can hold the port its declaration names");
    let frame = fixture.frame();
    drop(held);

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_ne!(status(service), "pass", "{service}");
    // The service did start — it is the bind that failed — so this is a failure
    // of the service and not a plan that produced nothing. And it is *this*
    // failure: a row that is merely not a pass would be this case passing for
    // whatever reason the service happened to fail for, which is how a test
    // about a port conflict goes on being green while measuring nothing.
    assert_eq!(status(service), "fail", "{service}");
    assert!(
        reason(service).contains("EADDRINUSE"),
        "the row must be about the port the test is holding, and this one does not name it — \
         whatever it says instead is a different failure wearing this case's clothes: {service}"
    );
    assert!(
        !fixture.marker("heartbeat").exists(),
        "a service that never bound its port was beating"
    );
}

/// A readiness route that answers 404 is a failure, and the service is stopped
/// all the same.
///
/// `/nope` is a path the endpoint accepts and the fixture does not serve, so what
/// comes back is a real answer from a real service: *not found*. A 404 is not an
/// absence of evidence — SURE asked and was answered — and it is not a pass.
#[test]
fn a_readiness_route_that_answers_404_is_never_passed() {
    // No page: the declaration's readiness path is the one question here, and a
    // page would add a second service start whose own readiness question is
    // answered 404 as well.
    let fixture = Fixture::new("declared service 找不到");
    fixture.settings(AUTHORISED);
    fixture.declare(fixture.port, "server.js", "/nope", None);
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_ne!(status(service), "pass", "{service}");
    assert_eq!(status(service), "fail", "{service}");
    assert!(
        reason(service).contains("404"),
        "the readiness route answered 404 from a service that was running, and the row must be \
         about that answer: {service}"
    );

    // And the failure is not an excuse to leave the service running.
    assert!(fixture.marker("started.json").exists(), "{service}");
    let beat = read(&fixture.marker("heartbeat"));
    std::thread::sleep(SILENCE);
    assert_eq!(
        read(&fixture.marker("heartbeat")),
        beat,
        "a service that failed its readiness question was left running"
    );
}

/// A service that ends by itself is a failure, and nothing of it is left alive.
///
/// The fixture writes `exiting` and stops, which is the one way a service can end
/// that is not SURE stopping it. The row says so, and the heartbeat — written
/// only while the process is alive — stops moving on its own.
#[test]
fn a_service_that_ends_by_itself_is_never_passed() {
    let fixture = Fixture::new("declared service 自己结束");
    fixture.settings(AUTHORISED);
    fixture.declare(fixture.port, "server.js", "/readyz", None);
    fixture.ask_for("exit-immediately");
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_ne!(status(service), "pass", "{service}");
    assert_eq!(status(service), "fail", "{service}");
    assert!(
        fixture.marker("exiting").exists(),
        "the fixture did not take the branch this case is about — its `exit-immediately` switch \
         is read once the service is listening, and no service got that far: {service}"
    );
    assert!(
        !fixture.marker("heartbeat").exists(),
        "a service that stopped at startup was beating: {service}"
    );
}

/// A page that answers an error is not a pass.
///
/// The readiness route answers, so the service is up and the look happens; what
/// the browser finds is a 500. *The service is running* and *the page works* are
/// two results, and only one of them is green here.
#[test]
fn a_page_that_answers_an_error_is_never_passed() {
    let fixture = Fixture::new("declared service 页面报错");
    fixture.settings(AUTHORISED);
    fixture.declare(fixture.port, "server.js", "/readyz", Some("/error"));
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(status(service), "pass", "{service}");
    let page = row(&frame, PAGE_TAG);
    assert_ne!(status(page), "pass", "{page}");
    match sure_core::browser_driver::find_installed_browser() {
        None => {
            // No browser, so the look could not happen and the row says which
            // fact stopped it rather than reporting the page as broken.
            assert_eq!(page["not_checked_reason"], "tool_unavailable", "{page}");
        }
        Some(browser) => {
            eprintln!("the browser this machine offered: {}", browser.display());
            assert_eq!(status(page), "fail", "{page}");
            assert!(reason(page).contains("500"), "{page}");
        }
    }
}

/// A project that changes while the run is working leaves no pass behind that is
/// about the state it checked.
///
/// The fixture writes a file inside the project once it is listening, which is
/// the ordinary accident this rule exists for: the run read the project at stage
/// 3 and the evidence it has is about a project that is no longer there. **The
/// two rows are the case.** Both of them were established by starting the
/// project's own code and watching it — the page row starts the same service —
/// so both are evidence about an execution of a state that has moved.
#[test]
fn a_project_that_changes_while_the_run_works_withdraws_its_runtime_evidence() {
    let fixture = Fixture::new("declared service 运行中改动");
    fixture.settings(AUTHORISED);
    fixture.ask_for("write-into-project");
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    let page = row(&frame, PAGE_TAG);
    assert!(
        fixture.project.join("wrote-during-the-run.txt").exists(),
        "the fixture did not change the project during the run — its `write-into-project` switch \
         is read once the service is listening, so this case measured nothing: {service}"
    );

    // Stage 10 says what it noticed, in the vocabulary's own words.
    let aggregate = frame["details"]["stages"]
        .as_array()
        .expect("the frame lists its stages")
        .iter()
        .find(|stage| stage["stage"] == "aggregate")
        .expect("an aggregate stage");
    let detail = aggregate["detail"].as_str().expect("a detail");
    assert!(detail.contains("project changed"), "{detail}");

    for row in [service, page] {
        assert_ne!(
            status(row),
            "pass",
            "a pass about a state that is no longer there: {row}"
        );
    }
    assert_eq!(
        status(service),
        "unknown",
        "the service row ran project code and its pass must be withdrawn: {service}"
    );
    // The sentence is read out of the product rather than typed here, for the
    // reason the two gap cases read `plain_description`: a reason a report shows
    // a person is the domain's wording, and a test that retyped it would go on
    // passing after the product stopped saying it. **The first version of this
    // assertion looked for `"SupersededByLaterChange"`** — the Rust variant's own
    // name — which appears in no sentence SURE writes, so it failed against a
    // reason that was exactly right. That is the same mistake as a documented
    // `sure.yaml` block no test ever parsed: an assertion about a spelling
    // nobody read back from the thing that produces it.
    assert!(
        reason(service).contains(
            sure_core::evidence::StalenessReason::SupersededByLaterChange.plain_explanation()
        ),
        "the service row is withdrawn for the vocabulary's own reason and not a paraphrase of \
         it: {service}"
    );
    // **The row this case was written for.** The first run of it was red here:
    // `invalidate_runtime_passes` withdrew the passes of proposals whose *actions
    // run project code*, and the browser row's only action is `BrowserProbe` — so
    // the page row kept a pass that had been established by starting the very
    // service the change moved, beside a service row that was correctly
    // withdrawn. `pipeline.rs` now asks `CheckOperation::starts_a_process`, which
    // is the question this rule is about, and both rows go the same way.
    assert_eq!(
        status(page),
        "unknown",
        "the page row's evidence came from starting and watching the project's own service too, \
         so its pass is about an execution of a state that moved: {page}"
    );
}

/// A machine with no browser gets an absence and a sentence, not a pass.
///
/// **The question is a Windows one and is asked only there.** On macOS one of
/// `browser_driver::installed`'s roots is `/Applications`, and on Linux its
/// `ABSOLUTE` list names `/usr/bin/chromium` and `/snap/bin/chromium`; neither is
/// read from the environment, so where one of them is a file there is no
/// environment that hides it — the case would measure the runner's image, not
/// the search. The Linux runner is exactly that machine, and this repository has
/// the measurement: `.github/workflows/release-dry-run.yml` records
/// `/usr/bin/chromium` on `ubuntu-latest` aborting at `ZygoteHostImpl::Init`
/// with *No usable sandbox!*. On Windows every entry in the table is one of the
/// three roots crossed with a browser's own directory and every root is an
/// environment variable, which is what makes the question askable here.
///
/// **This machine has a browser, so this run is given the machine a browserless
/// one is.** `installed` reads exactly two things — that table and `PATH` — so
/// the run is given a block of its own in which the three roots point at an
/// empty directory and `PATH` holds no directory a browser's executable is in:
/// the answer a machine with no browser in either place gives. That is a
/// faithful way to ask the question and not a way around it — the product reads
/// those variables and nothing else, and nothing this run hands it is a browser.
///
/// **The block is cleared and rebuilt rather than overridden, and that is a
/// measurement.** With the parent's block inherited, an override of one of these
/// names is not reliably delivered on this machine: `.env("LOCALAPPDATA", …)`
/// and `.env("ProgramFiles(x86)", …)` landed and `.env("PROGRAMFILES", …)` did
/// not, so the child kept `C:\Program Files` and the earlier version of this
/// case found the Chrome under it and reported a pass where it meant to measure
/// an absence — the names it set and the names the search reads were both right
/// and the value simply did not arrive. Measured 2026-09-23 with a two-binary
/// probe that prints its own block: inherited plus those three overrides shows
/// `PROGRAMFILES = C:\Program Files`, while `env_clear()` plus the same three
/// names shows all three at the empty directory and five variables in the whole
/// block. `env_remove` of the same name does not take either, and Git Bash's
/// `env -i` is not an empty block at all — it keeps `PROGRAMFILES`,
/// `SYSTEMROOT`, `WINDIR` and `PATH`, so a probe run through it measures the
/// shell rather than the machine.
///
/// **What the block keeps is what a machine has and this run uses.** `SystemRoot`
/// is the one variable `node` will not start without, which
/// `crates/sure-core/src/service_plan.rs`'s `service_environment` documents and
/// this file's own measurement above records; the service half of this case is
/// asserted to pass, so the block has to keep it. `PATH` is where the search
/// also looks for a browser and where the `node` this service runs is found by
/// name. Nothing else is set, and nothing else is needed: the project, the store
/// and the settings file are all given as arguments.
///
/// What the row must not be is a pass: no page was looked at, and *nobody
/// looked* is not *the page works*.
#[cfg(windows)]
#[test]
fn a_machine_with_no_browser_reports_an_absence_and_never_a_pass() {
    let fixture = Fixture::new("declared service 没有浏览器");
    fixture.settings(AUTHORISED);
    let system_root =
        std::env::var_os("SystemRoot").expect("a Windows machine names where Windows is");
    let nowhere = fixture.nowhere();
    let mut command = Command::new(SURE);
    command
        .arg("--store-dir")
        .arg(&fixture.store)
        .arg("--settings-file")
        .arg(&fixture.settings)
        .args(["--format", "json", "check"])
        .arg(&fixture.project)
        .stdin(Stdio::null())
        .env_clear()
        .env("SystemRoot", system_root)
        .env("PATH", without_a_browser())
        .env("ProgramFiles", &nowhere)
        .env("ProgramFiles(x86)", &nowhere)
        .env("LOCALAPPDATA", &nowhere);
    let output = command.output().expect("the CLI starts");
    let stdout = String::from_utf8(output.stdout).expect("the JSON report is UTF-8");
    assert!(output.status.code().is_some(), "{stdout}");
    let frame: Value = serde_json::from_str(&stdout).expect("one JSON frame");

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(
        status(service),
        "pass",
        "the service check does not depend on a browser, so it still runs: {service}"
    );
    let page = row(&frame, PAGE_TAG);
    assert_ne!(status(page), "pass", "{page}");
    assert_eq!(status(page), "skipped", "{page}");
    assert_eq!(page["not_checked_reason"], "tool_unavailable", "{page}");
    assert!(
        reason(page).contains("needs a browser tool on this computer"),
        "the absence is not explained as one: {page}"
    );
    // The service itself was still looked at as far as it can be: the readiness
    // question was asked, so this is an absent browser and not an absent service.
    assert!(
        read(&fixture.marker("requests.log")).contains("/readyz"),
        "the service was never asked anything"
    );
}

/// `PATH` as this machine has it, minus the entries a browser lives in.
///
/// The names are `browser_driver::installed`'s own list of programs, lowercased
/// and matched anywhere in an entry: a directory called
/// `...\Google\Chrome\Application` is a browser's directory however it got onto
/// the list, and matching the program names rather than a fixed path is what
/// keeps this working on a machine whose browser was installed somewhere else.
///
/// **What is left is a `PATH` no search finds a browser on, and it is also the
/// `PATH` the service's own interpreter is found through** — so this drops
/// entries by what they are, and it must not drop the one `node` is in. It does
/// not: this build looks for a browser by name and `node`'s directory is not
/// named after one, and the case that uses this asserts the service half passes.
#[cfg(windows)]
fn without_a_browser() -> std::ffi::OsString {
    let names = [
        "chrome", "chromium", "msedge", "edge", "brave", "vivaldi", "opera",
    ];
    let kept: Vec<std::ffi::OsString> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .filter(|entry| {
                let lowered = entry.to_string_lossy().to_lowercase();
                !names.iter().any(|name| lowered.contains(name))
            })
            .map(PathBuf::into_os_string)
            .collect();
    std::env::join_paths(kept).expect("the remaining entries are paths")
}

/// The implementation before the plan existed could not have produced this.
///
/// `0bf130a` is the commit that made a declaration into checks: before it,
/// `crates/sure-core/src/pipeline.rs` had no proposer for
/// `CheckOperation::Service` and no proposer for `CheckOperation::Browser`, so
/// nothing a project could write produced a service check and nothing here could
/// happen. What this test measures is the discriminating property rather than the
/// commit: under settings that authorise both, **the service starts and keeps
/// running** — a file with the process's own id in it, and a beat that has moved
/// since. An implementation that planned nothing produces neither, and this test
/// fails on the first marker instead of passing on an empty plan.
#[test]
fn the_old_implementation_could_not_have_passed_this() {
    let fixture = Fixture::new("declared service 红证明");
    fixture.settings(AUTHORISED);
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(status(service), "pass", "{service}");
    let started: Value = serde_json::from_str(&read(&fixture.marker("started.json")))
        .expect("a service SURE started wrote what it was");
    assert_ne!(started["pid"].as_u64(), Some(u64::from(std::process::id())));
    let beat: u64 = read(&fixture.marker("heartbeat"))
        .trim()
        .parse()
        .expect("the heartbeat is a number");
    assert!(
        beat > started["beat"].as_u64().expect("a starting beat"),
        "the service started and did not run"
    );
}

/// A project that turns its declared services off is told which setting did it.
///
/// **This is not a refusal and must not be reported like one.** The declaration
/// is well formed, SURE would have started it, and what decided is the project's
/// own `checks.start_local_services: never`. A plan that simply held no service
/// row would leave the person who wrote the declaration looking for a mistake in
/// their file, so the decision is recorded as a value
/// (`ServicePlan::gaps`), carried into the run's plan
/// (`crates/sure-core/src/pipeline.rs`), and read here out of stage 4's own
/// `detail` — the one place all three meet.
#[test]
fn a_project_that_switches_its_services_off_is_told_that_it_did() {
    let fixture = Fixture::new("declared service 关掉开关");
    fixture.settings(AUTHORISED);
    fixture.write_project(&format!(
        "checks:\n  start_local_services: never\n  browser_probe: always\n{}",
        services_yaml(fixture.port, "server.js", "/readyz", Some("/"))
    ));
    let frame = fixture.frame();

    finished(&frame);
    assert!(
        rows(&frame, SERVICE_TAG).is_empty(),
        "a project that turned its services off still got a service row: {:?}",
        rows(&frame, SERVICE_TAG)
    );
    assert!(
        rows(&frame, PAGE_TAG).is_empty(),
        "a page was looked at on a service this project declined to start: {:?}",
        rows(&frame, PAGE_TAG)
    );
    // The setting was in force and not merely mentioned: a run that planned
    // nothing must also have started nothing.
    nothing_started(&fixture, &frame);

    let detail = plan_detail(&frame);
    // The sentence is the product's own, built here rather than retyped: a
    // paraphrase would pass while the wording a person reads said something else.
    let sentence = sure_core::service_plan::ServiceGap::LocalServicesDisabled {
        name: NAME.to_owned(),
    }
    .plain_description();
    assert!(
        detail.contains(&sentence),
        "the plan stage does not say the project switched its own service off: it was looking \
         for `{sentence}` and said `{detail}`"
    );
}

/// A declaration whose page is not opened under the default preference says so.
///
/// `checks.browser_probe` defaults to `auto`, and `auto` does not open a page:
/// nothing in a project's shape says it has an interface, so a declaration that
/// names one gets a service row and a sentence rather than a browser row on a
/// guess. **The service row is planned and running** — the gap is about the page
/// and not about the service — which is what makes the sentence the only thing
/// standing between a reader and *my page was never checked and nobody said
/// why*.
#[test]
fn a_page_the_default_preference_does_not_open_is_still_explained() {
    let fixture = Fixture::new("declared service 默认偏好");
    fixture.settings(AUTHORISED);
    fixture.write_project(&format!(
        "checks:\n{}",
        services_yaml(fixture.port, "server.js", "/readyz", Some("/"))
    ));
    let frame = fixture.frame();

    finished(&frame);
    let service = row(&frame, SERVICE_TAG);
    assert_eq!(status(service), "pass", "{service}");
    assert!(
        rows(&frame, PAGE_TAG).is_empty(),
        "`browser_probe: auto` planned a browser row: {:?}",
        rows(&frame, PAGE_TAG)
    );

    let detail = plan_detail(&frame);
    let sentence = sure_core::service_plan::ServiceGap::AutoDoesNotLookAtAPage {
        name: NAME.to_owned(),
    }
    .plain_description();
    assert!(
        detail.contains(&sentence),
        "the plan stage does not say why no browser looked at the declared page: it was looking \
         for `{sentence}` and said `{detail}`"
    );
}

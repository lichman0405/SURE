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
//! # The measurement of 2026-09-23, which is why cases here are red
//!
//! **Every case below that needs the service to be *running* fails on this
//! machine, and what it fails on is not in this file.** A run under
//! `host_confirmed` with `run_project_code` planned the service check, admitted
//! it, started it, and reported:
//!
//! ```text
//! start api   fail   the service ended by itself after 65 milliseconds, before
//!                    SURE could ask it anything (exit code 134); it wrote on
//!                    standard error:  3: ... node::Start+160 ...
//! ```
//!
//! the process aborting with no request ever reaching it. The cause is one line
//! of `crates/sure-core/src/service_plan.rs`: `declared_service` builds the
//! command as `Environment::only(Vec::new())` — ADR 0014's decision 11, carried
//! out where the plan is made — and the `node` this machine has cannot
//! initialize without `SystemRoot`. Run with nothing in its environment, node
//! aborts before it reads `server.js` at all:
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
//! makes the same file start and serve. `WINDIR`, `SystemDrive`, `TEMP`, `TMP`,
//! `PATH` and `USERPROFILE` were each tried alone and each still aborts, so this
//! is not a variable to add to a list: it is `SystemRoot` or nothing.
//!
//! `node` v25.8.1, `C:\Program Files\nodejs\node.exe`, the machine's only
//! `node`, measured at `f614dc4`. **None of the assertions below were weakened
//! for it**: a case that says a service must be running goes on saying so, and
//! the failing row carries the evidence. No fixture can work around it, because
//! the empty environment is passed to the program and every launcher kind in
//! this build is this program.
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
//! SURE runs is `node server.js` — one argument, an empty environment, and no
//! argv of SURE's making. Dropping the listener leaves a window between choosing
//! the port and the run using it, and nothing here closes that window; a machine
//! that takes the port in that window makes the fixture service fail to bind, and
//! the check reports that failure as the failure it is rather than as a pass.
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

/// The name the fixture's one declaration gives its service, and the two titles
/// the two rows it becomes carry.
const NAME: &str = "api";
const SERVICE_TITLE: &str = "start api";
const PAGE_TITLE: &str = "check api in a browser";

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

function note(name, text) {
  fs.writeFileSync(path.join(markers, name), text);
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
/// reached only the process SURE started leaves this one running, and this one
/// says so for as long as it is alive — which is the Windows-specific half of the
/// stop case below.
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
let beats = 0;
const beat = setInterval(() => {
  beats += 1;
  fs.writeFileSync(path.join(markers, 'helper-heartbeat'), String(beats));
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
    let page = page.map_or_else(String::new, |page| format!("      page: {page}\n"));
    format!(
        "checks:\n  browser_probe: always\n  services:\n    - name: {NAME}\n      launcher:\n\
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
    assert_eq!(page["title"], PAGE_TITLE, "{page}");
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

    // The descendant ran at all, which is what makes the assertion below about
    // it mean something: a helper that never started leaves no file, and a
    // missing file cannot be one that stopped moving.
    let helper = fixture.marker("helper-heartbeat");
    assert!(
        helper.exists(),
        "the service's own child never wrote its heartbeat, so there is nothing whose stopping \
         could be measured"
    );

    let heartbeat = read(&fixture.marker("heartbeat"));
    let helper_beat = read(&helper);
    std::thread::sleep(SILENCE);
    assert_eq!(
        read(&fixture.marker("heartbeat")),
        heartbeat,
        "the service SURE started was still running after the run returned"
    );

    // **The descendant is the half only Windows reaches**, and it is asserted
    // only here. `crates/sure-core/src/process/terminate.rs` documents the
    // mechanism and the difference: this platform runs `taskkill /T`, which
    // reaches the whole tree, and every other platform stops the one process
    // SURE started — so on those, the helper above is still running and saying
    // so. A test that asserted a frozen helper there would fail for a reason
    // that is already written down where the mechanism is, and a test that
    // asserted nothing would be a silently omitted assertion. What runs on those
    // platforms is the helper's own heartbeat having moved, above, and this
    // comment.
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
    assert!(
        reason(service).contains("SupersededByLaterChange"),
        "{service}"
    );
    // **This assertion states what must happen, and it is expected to be red.**
    // `invalidate_runtime_passes` withdraws the passes of proposals whose
    // actions run project code, and the browser row's only action is
    // `BrowserProbe` — so the page row keeps a pass that was established by
    // starting the very service the change moved. *A pass about a state that is
    // no longer there* is what this rule exists to prevent, and the case is here
    // as a failing measurement rather than as a missing one.
    assert_eq!(
        status(page),
        "unknown",
        "the page row's evidence came from starting and watching the project's own service too, \
         so its pass is about an execution of a state that moved: {page}"
    );
}

/// A machine with no browser gets an absence and a sentence, not a pass.
///
/// **This machine has one, so the branch is reached by making this run's machine
/// answer be `no`.** `browser_driver::installed` reads exactly two things — a
/// table of paths compiled into the binary and `PATH` — and the table's roots are
/// `ProgramFiles`, `ProgramFiles(x86)` and `LOCALAPPDATA`. With those three
/// unset and the `PATH` entries that name a browser removed, `find` answers
/// `None` for this process, which is the same answer a machine with no browser
/// gives. That is a faithful way to ask the question and not a way around it:
/// the product reads those variables and nothing else, and this run gives it the
/// values a browserless machine has.
///
/// What the row must not be is a pass: no page was looked at, and *nobody
/// looked* is not *the page works*.
#[test]
fn a_machine_with_no_browser_reports_an_absence_and_never_a_pass() {
    let fixture = Fixture::new("declared service 没有浏览器");
    fixture.settings(AUTHORISED);
    let output = Command::new(SURE)
        .arg("--store-dir")
        .arg(&fixture.store)
        .arg("--settings-file")
        .arg(&fixture.settings)
        .args(["--format", "json", "check"])
        .arg(&fixture.project)
        .env_remove("ProgramFiles")
        .env_remove("ProgramFiles(x86)")
        .env_remove("LOCALAPPDATA")
        .env("PATH", without_a_browser())
        .stdin(Stdio::null())
        .output()
        .expect("the CLI starts");
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

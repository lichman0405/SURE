//! Driving the browser: what is asked, what is watched for, and what a page's
//! own events turn into.
//!
//! # The four domains, and the one that is deliberately not enabled
//!
//! SURE connects to the debugging port Chrome wrote into its profile, attaches
//! to the page, turns on `Page`, `Network` and `Runtime`, and navigates. Every
//! event those three produce is folded into one [`Observation`].
//!
//! **`Log` is not enabled, and that is a measurement rather than a preference.**
//! The `Log` domain overlaps the other two on purpose — a `Log.entryAdded` with
//! source `network` is the same fact as a `Network.responseReceived` with a
//! status outside the window, and one with source `console-api` or `javascript`
//! is the same fact as `Runtime.consoleAPICalled` or
//! `Runtime.exceptionThrown` — so absorbing it means either reporting every
//! console error twice or matching entries to requests by URL, and a `Log` entry
//! **carries a URL and no request identifier**, which was measured rather than
//! assumed. What `Log` alone would add is deprecation and security notices,
//! which are not what this check claims to report.
//!
//! # What a failed request is, which took three measurements to get right
//!
//! The first rule written here was *every `Network.loadingFailed` is a problem*.
//! It marks a healthy page as broken, because a page that starts a `fetch` and
//! aborts it with `AbortController` produces one, and that is ordinary modern
//! code. The measured shape of that event is `net::ERR_ABORTED`, `canceled:
//! true`, and **nothing else** — no console error, no second event — so a
//! cancelled request is not a problem here.
//!
//! The second rule written here was *every `Log.entryAdded` at `level=error` is
//! a problem*. That marks **almost every healthy local page** as broken, and it
//! was found by accident: a page with nothing wrong with it reported an error,
//! because Chromium asks for `/favicon.ico` on its own and a local app that does
//! not serve one answers 404. Worse, whether a page reports that depends on
//! whether another page already warmed the favicon cache for that origin.
//!
//! So the question became *does the browser say who asked for the request?* It
//! does, in `Network.requestWillBeSent`'s `initiator` — **and for this case the
//! answer is no**: a page's own `<link rel="icon" href="/favicon.ico">` and the
//! browser's unrequested fetch of the same path are both `type: Other` with
//! `initiator: {type: "other"}`, byte for byte. Nothing in the protocol
//! separates them.
//!
//! What does separate them is **what kind of resource it was**, and that was
//! measured by loading a page that asked for one of everything and 404'ing all
//! of it:
//!
//! | the page asked for | `type` | `initiator` |
//! | --- | --- | --- |
//! | `<link rel=stylesheet>` | `Stylesheet` | `parser` |
//! | `<script src>` | `Script` | `parser` |
//! | `<script type=module src>` | `Script` | `parser` |
//! | `<img src>` | `Image` | `parser` |
//! | `new Image().src` | `Image` | `parser` |
//! | `fetch()` | `Fetch` | `script` |
//! | `XMLHttpRequest` | `XHR` | `script` |
//! | `@font-face src` | `Font` | `parser` |
//! | **nothing — the browser's own** | **`Other`** | **`other`** |
//!
//! Every one of those eight answered `404` and **every one of them arrived on
//! `Network.responseReceived` carrying its status**, which is why the filter
//! reads one stream instead of two: `Other` is the type the browser files a
//! request under when the page's own code did not name it, and no request the
//! page's code made was ever filed there.
//!
//! **What that rule can hide, stated rather than left to be discovered**: a
//! request the browser files as `Other` although the page asked for it — a
//! declared icon, measured above to be indistinguishable from the browser's
//! own, and possibly a `preload` or `prefetch`, which were not measured. What it
//! cannot do is pass a page whose scripts, stylesheets, images, fonts, fetches
//! or XHRs failed, and those are the failures that mean the page is broken
//! rather than untidy.
//!
//! # A page that never arrives
//!
//! Asked for a port nothing is listening on, Chromium **commits an error page of
//! its own**: the end-to-end test in `tests/browser_driver.rs` reported the
//! landing address as `chrome-error://chromewebdata/`, which is where the
//! browser went rather than where SURE sent it. That is a measurement, and it
//! changed nothing here because the verdict does not rest on the landing
//! address: the address SURE asked for is recorded on the
//! [`NavigationFailed`](ProblemKind::NavigationFailed) problem, and a look with
//! a problem in it is a failure whatever page the browser ended up showing. It
//! is written down because it is the kind of fact a reader would otherwise
//! assume the other way — *a page that did not load leaves the landing address
//! empty* — and then write a rule against.
//!
//! # Lines
//!
//! CDP counts lines from **zero** and a person opening a file counts from
//! **one**, so every line this module reports is incremented. That was measured
//! rather than reasoned about, and the same message proves it twice:
//! `Runtime.exceptionThrown` for a `throw` on document line 3 carried
//! `lineNumber: 2` and, in the same payload,
//! `exception.description: "Error: … at http://…/thrown:3:15"`. Two renderings
//! of one line, one of them from the browser's own pretty-printer, disagreeing
//! by exactly one.
//!
//! **Zero is not the first line and never becomes one.** [`Problem::line`] uses
//! zero for *the browser did not say*, and after incrementing, a zero-based
//! zero becomes a one — so the two never collide.
//!
//! # The settle window, and what `complete` means here
//!
//! The page is watched until its load event and then for a further
//! [`SETTLE`], because a page's own scripts run after the document is parsed and
//! an error thrown on the last line of the bundle arrives late. The look ends
//! early only when the browser stops talking or the budget is spent.
//!
//! [`Observation::complete`] is true **only when the look ended because it had
//! nothing left to watch**. A look that ran out of budget is not complete even
//! if the page had already loaded a moment before, because the settle window is
//! part of the look and a page cannot be reported clean on the strength of a
//! window that was cut in half. This is the direction that matters:
//! [`Report::status`] turns *no problems and not complete* into `unknown`, and a
//! driver that called a truncated look complete would be a driver turning *I
//! stopped looking* into *there was nothing to find*.
//!
//! # What this module does not do
//!
//! It does not decide a verdict. It produces an [`Observation`] and hands it to
//! [`Report::observed`], and every mapping from there to a status is
//! `browser.rs`'s. There is no [`CheckStatus`](sure_domain::status::CheckStatus)
//! in this file.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::browser::{AbsenceReason, Limits, Observation, Problem, ProblemKind, Report, Target};
use crate::process::Cancellation;

use super::installed;
use super::launch;
use super::websocket::{self, WebSocket, WsError};

/// How long the page is watched after its load event.
///
/// A page's scripts run after the document is parsed, and the last line of a
/// bundle can throw microseconds after the load event that started it. Two
/// hundred and fifty milliseconds is long enough for that and short enough that
/// it does not dominate a check.
const SETTLE: Duration = Duration::from_millis(250);

/// How long the browser is given to leave after it has been asked to close.
///
/// **Outside the look's budget, deliberately.** The budget bounds looking, and
/// this is the opposite of looking — it is the part where SURE stops. A browser
/// that has been asked to close is normally gone in well under this, and a
/// browser that is not gone by the end of it is stopped by
/// [`super::launch::Launched`]'s drop.
const DEPARTURE: Duration = Duration::from_secs(2);

/// The shortest a step is given, however little budget is left.
///
/// **Not a rounding nicety.** A zero timeout is not a very short wait: on
/// Windows a zero socket timeout is refused outright, so a step handed one would
/// fail with an option error rather than by timing out, and the report would
/// blame the browser for the clock.
const NO_LESS_THAN: Duration = Duration::from_millis(50);
/// A just-opened debugging endpoint can answer before its first page exists.
/// Re-read its target list within the same look budget instead of treating that
/// transient empty list as a permanently unusable browser.
const TARGET_POLL: Duration = Duration::from_millis(25);

/// How many requests are remembered so that a failure can name its address.
///
/// `Network.loadingFailed` carries a request identifier, an error and a type,
/// and **no URL** — that was measured. The URL is in the earlier
/// `Network.requestWillBeSent` for the same identifier, so the two are joined.
/// The table is bounded because it grows with the page's own behaviour and not
/// with anything SURE chose: past this many requests a failure is still
/// reported, without the address it was made for.
const NAMED_REQUESTS: usize = 256;

/// What could not be done, while the browser was being reached.
///
/// Only ever produced **before** navigation. Everything after it is an
/// [`Observation`], because once a page has been asked for, whatever went wrong
/// is something SURE saw happen to a page rather than a reason no page existed.
#[derive(Debug)]
enum Trouble {
    /// The connection to the browser failed.
    Link(WsError),
    /// The browser answered a command with an error.
    Refused { method: String, said: String },
}

impl std::fmt::Display for Trouble {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Link(error) => write!(formatter, "{error}"),
            Self::Refused { method, said } => {
                write!(formatter, "the browser refused {method}: {said}")
            }
        }
    }
}

impl std::error::Error for Trouble {}

impl From<WsError> for Trouble {
    fn from(error: WsError) -> Self {
        Self::Link(error)
    }
}

/// The rules, as a value that can be fed messages without a browser.
///
/// **Everything that turns a browser's event into an observation is here, and
/// nothing here opens a socket.** The rules are the part of this file that has
/// measurements behind it and the part most likely to be wrong, so they are the
/// part that can be tested exhaustively — a page's worth of events can be handed
/// in as data, in any order, without a browser existing.
#[derive(Debug)]
struct Folding {
    /// The frame the address was opened in, once something has said which it is.
    main_frame: Option<String>,
    /// Where the browser ended up. The last main-frame navigation wins.
    landing_url: String,
    /// What the page request was answered with. **The last main-frame document
    /// response wins, not the first** — a redirect chain that ends on a 404
    /// would otherwise record the 302, which is inside the window
    /// [`crate::browser::a_page_could_be_shown`] calls servable, and a redirect
    /// to nothing would pass.
    document_status: Option<u16>,
    /// What went wrong, in the order the browser reported it.
    problems: Vec<Problem>,
    /// How many problems may be held.
    kept: usize,
    /// Whether more problems arrived than could be held.
    overflowed: bool,
    /// Whether the page's load event has been seen.
    settled: bool,
    /// Request identifiers, so that a failure with no URL can be named.
    requests: HashMap<String, String>,
}

impl Folding {
    /// A folding that holds up to `kept` problems.
    fn new(kept: usize) -> Self {
        Self {
            main_frame: None,
            landing_url: String::new(),
            document_status: None,
            problems: Vec::new(),
            kept,
            overflowed: false,
            settled: false,
            requests: HashMap::new(),
        }
    }

    /// Folds one message from the browser into what is known so far.
    ///
    /// **A message from another session is dropped**, and that is not
    /// belt-and-braces: `Target.attachedToTarget` and the other browser-level
    /// events arrive on the same socket with **no `sessionId` at all** — that
    /// was measured while the connection was being built — so a filter written
    /// as *messages that are not ours* would have to distinguish them from
    /// messages that are nobody's. Comparing against the session SURE attached
    /// to draws the line in the right place.
    ///
    /// A message that is not an event, or an event from a domain this driver did
    /// not turn on, is ignored rather than reported: the browser is allowed to
    /// say things this driver does not understand.
    fn absorb(&mut self, session: &str, message: &Value) {
        if message.get("sessionId").and_then(Value::as_str) != Some(session) {
            return;
        }
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            return;
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match method {
            "Page.frameNavigated" => self.frame_navigated(&params),
            "Page.loadEventFired" => self.settled = true,
            "Network.requestWillBeSent" => self.request_began(&params),
            "Network.responseReceived" => self.response_arrived(&params),
            "Network.loadingFailed" => self.loading_failed(&params),
            "Runtime.exceptionThrown" => self.exception(&params),
            "Runtime.consoleAPICalled" => self.console_call(&params),
            _ => {}
        }
    }

    /// A frame finished navigating.
    ///
    /// A frame with a parent is a subframe, and a subframe's address is not
    /// where the page is; only the frame with no parent is the page.
    fn frame_navigated(&mut self, params: &Value) {
        let Some(frame) = params.get("frame") else {
            return;
        };
        if frame
            .get("parentId")
            .is_some_and(|parent| !parent.is_null())
        {
            return;
        }
        if let Some(id) = text(frame, "id") {
            self.main_frame = Some(id.to_owned());
        }
        if let Some(url) = text(frame, "url") {
            self.landing_url = url.to_owned();
        }
    }

    /// A request began, so that a later failure can name its address.
    fn request_began(&mut self, params: &Value) {
        if self.requests.len() >= NAMED_REQUESTS {
            return;
        }
        let Some(request) = params.get("request") else {
            return;
        };
        let (Some(id), Some(url)) = (text(params, "requestId"), text(request, "url")) else {
            return;
        };
        self.requests.insert(id.to_owned(), url.to_owned());
    }

    /// A response arrived for something.
    ///
    /// This is where a failed subresource is found, and it is the only place:
    /// every kind of request a page can make arrives here with its status,
    /// including the ones that 404, which was measured across eight kinds of
    /// resource at once. `Other` is the type the browser files a request under
    /// when the page's code did not name it, and it is where the browser's own
    /// favicon lives — see the module documentation for the table.
    fn response_arrived(&mut self, params: &Value) {
        let Some(response) = params.get("response") else {
            return;
        };
        let Some(status) = number(response, "status").and_then(|status| u16::try_from(status).ok())
        else {
            return;
        };
        let kind = text(params, "type").unwrap_or_default();
        if kind == "Document" && self.is_the_page(params) {
            self.document_status = Some(status);
            if self.landing_url.is_empty()
                && let Some(url) = text(response, "url").filter(|url| !url.is_empty())
            {
                self.landing_url = url.to_owned();
            }
            return;
        }
        if kind == "Other" {
            return;
        }
        // Four hundred rather than the page window, and the difference is not
        // cosmetic: a page may only be served from 200 to 399, but a subresource
        // may perfectly well answer `204 No Content` or `304 Not Modified`, and
        // a rule borrowed from the page would call both of those failures.
        if status >= 400 {
            let url = text(response, "url").unwrap_or_default().to_owned();
            self.record(
                ProblemKind::FailedRequest,
                format!("the browser was answered with HTTP {status}"),
                url,
                0,
            );
        }
    }

    /// A request did not complete.
    ///
    /// **A cancelled one is not a problem**, and getting that wrong is how this
    /// driver first reported healthy pages as broken: a page that aborts a
    /// request it no longer needs produces `net::ERR_ABORTED` with
    /// `canceled: true` and nothing else at all. A `404` also arrives here,
    /// cancelled — which is why the *cancelled* test is not the only one, and
    /// why the response path above is what reports those.
    fn loading_failed(&mut self, params: &Value) {
        if flag(params, "canceled") {
            return;
        }
        if text(params, "type") == Some("Other") {
            return;
        }
        // A message whose type cannot be read at all is reported rather than
        // dropped. Dropping is the direction that loses a real error, and the
        // cost of the other direction is one extra line in a report.
        let said = text(params, "errorText")
            .unwrap_or("the request did not complete")
            .to_owned();
        let url = text(params, "requestId")
            .and_then(|id| self.requests.get(id))
            .cloned()
            .unwrap_or_default();
        self.record(ProblemKind::FailedRequest, said, url, 0);
    }

    /// Something was thrown and nothing caught it.
    ///
    /// **The description is preferred to the text, and the measurement is the
    /// reason**: for `throw new Error("…")` the text is the bare word
    /// `"Uncaught"`, which tells a reader nothing, while the description carries
    /// the message and the stack.
    fn exception(&mut self, params: &Value) {
        let Some(details) = params.get("exceptionDetails") else {
            return;
        };
        let message = details
            .get("exception")
            .and_then(|thrown| text(thrown, "description"))
            .or_else(|| text(details, "text"))
            .unwrap_or("an uncaught error")
            .to_owned();
        let mut source = text(details, "url")
            .filter(|url| !url.is_empty())
            .or_else(|| first_frame(details).and_then(|frame| text(frame, "url")))
            .unwrap_or_default()
            .to_owned();
        if source.is_empty() {
            // The address is what `Problem::source` documents for this case, and
            // an empty one would render as a reason line beginning with a colon.
            // The line stays zero, so the report gives the project and says the
            // browser did not give the file.
            source.clone_from(&self.landing_url);
        }
        let line = number(details, "lineNumber")
            .or_else(|| first_frame(details).and_then(|frame| number(frame, "lineNumber")))
            .map(one_based)
            .unwrap_or(0);
        self.record(ProblemKind::RuntimeError, message, source, line);
    }

    /// Something was written to the console.
    ///
    /// Only `error` is a problem. A warning is something a person decided to
    /// print, and reporting every `console.log` would make a report of a page
    /// that logs.
    fn console_call(&mut self, params: &Value) {
        if text(params, "type") != Some("error") {
            return;
        }
        let message = params
            .get("args")
            .and_then(Value::as_array)
            .map(|args| {
                args.iter()
                    .map(argument_text)
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let frame = first_frame(params);
        let mut source = frame
            .and_then(|frame| text(frame, "url"))
            .unwrap_or_default()
            .to_owned();
        if source.is_empty() {
            source.clone_from(&self.landing_url);
        }
        let line = frame
            .and_then(|frame| number(frame, "lineNumber"))
            .map(one_based)
            .unwrap_or(0);
        let message = if message.trim().is_empty() {
            "the page called console.error with nothing in it".to_owned()
        } else {
            message
        };
        self.record(ProblemKind::ConsoleError, message, source, line);
    }

    /// Whether a response belongs to the page rather than to a frame inside it.
    ///
    /// When nothing has said which frame is the page, everything is. That is not
    /// a looseness for its own sake: `Page.navigate`'s reply and the document's
    /// own response are two messages from the browser with no ordering
    /// guarantee, and a driver that required the frame to be known first would
    /// report *no page arrived* for a page that had just arrived. Before a
    /// navigation there are no subframes, so the answer is right either way.
    fn is_the_page(&self, params: &Value) -> bool {
        match self.main_frame.as_deref() {
            Some(main) => text(params, "frameId") == Some(main),
            None => true,
        }
    }

    /// Whether a failure to arrive has already been reported.
    fn said_the_navigation_failed(&self) -> bool {
        self.problems
            .iter()
            .any(|problem| problem.kind == ProblemKind::NavigationFailed)
    }

    /// Records that the address was opened and no page arrived — **unless the
    /// browser already said why, or one did arrive.**
    ///
    /// Both halves of that guard are load-bearing and they fail in opposite
    /// directions. Without the first, a page that loaded perfectly would get a
    /// problem saying it never arrived. Without the second, a page that never
    /// arrived because the connection was refused would get **two** reasons for
    /// the same fact, the second in SURE's words rather than in the browser's —
    /// and the browser's is the one that names the error.
    ///
    /// **The second half is not redundant with the first even though a refusal
    /// usually produces both**, because `Page.navigate` reports a refusal as a
    /// *successful* command carrying `errorText`: the address is where the
    /// frame was going, so it is frequently set while the page never existed.
    ///
    /// **Split out of [`Connection::look`] so that a test can reach it**, the
    /// same way [`Folding::is_the_page`] and [`on_a_path`](super::installed) are
    /// split out of the code that reads the machine: this is a decision about
    /// what has been folded so far, and a decision about a value can be put to a
    /// value. The call site itself is covered by
    /// `tests/browser_driver.rs`, which drives a real browser at an address
    /// nothing is listening on.
    fn a_page_that_never_arrived(&mut self, address: &str) {
        if !self.landing_url.trim().is_empty() || self.said_the_navigation_failed() {
            return;
        }
        self.record(
            ProblemKind::NavigationFailed,
            "the address was opened and no page arrived",
            address.to_owned(),
            0,
        );
    }

    /// Holds one problem, or records that there were more than could be held.
    ///
    /// **Going over the bound does not grow the list, and it does stop the
    /// look.** A page that throws in a loop would otherwise be watched for the
    /// whole budget to collect thousands of lines nobody will read; stopping is
    /// the point of the bound. The look then ends early, which
    /// [`Observation::complete`] reports as false, so a page that produced more
    /// errors than the driver would hold is never described as clean.
    fn record(
        &mut self,
        kind: ProblemKind,
        message: impl Into<String>,
        source: impl Into<String>,
        line: u32,
    ) {
        if self.problems.len() >= self.kept {
            self.overflowed = true;
            return;
        }
        self.problems
            .push(Problem::new(kind, message, source, line));
    }

    /// What was seen, as the driver's half of the interface.
    ///
    /// `ended_naturally` is whether the look stopped because it had nothing left
    /// to watch rather than because the budget or the browser ended it, and it
    /// is not the same question as whether the page loaded: a look cut short
    /// inside its settle window has seen the load event and has not finished
    /// looking.
    fn observation(&self, title: String, ended_naturally: bool) -> Observation {
        Observation {
            landing_url: self.landing_url.clone(),
            title,
            document_status: self.document_status,
            problems: self.problems.clone(),
            complete: ended_naturally && !self.overflowed,
        }
    }
}

/// A live conversation with the browser's page.
struct Connection {
    socket: WebSocket,
    session: String,
    next_id: u64,
    folding: Folding,
    /// How long a command waits for its reply.
    ///
    /// **Kept beside the socket because [`Connection::pump`] changes the read
    /// timeout and would otherwise leave its last slice behind.** The pump reads
    /// in fifty-millisecond slices so that it can notice its deadline and the
    /// browser going quiet; a command issued afterwards would inherit the last
    /// slice, and a browser that took longer than fifty milliseconds — which is
    /// ordinary for a page whose title is set by a script — would be reported as
    /// having stopped answering. Every command therefore sets this first.
    patience: Duration,
}

impl Connection {
    /// Connects, finds the page, attaches to it and turns the domains on.
    ///
    /// Everything here is *reaching the browser*, so every failure is a
    /// [`Trouble`] and every `Trouble` becomes
    /// [`AbsenceReason::DriverWouldNotStart`]. Nothing here can say anything
    /// about the page, because nothing here has asked for one yet — and the
    /// whole point of connecting the check to a browser is to be able to tell
    /// the difference between *the tool would not start* and *your page broke*.
    fn attach(
        browser: &launch::Launched,
        timeout: Duration,
        kept: usize,
        cancellation: &Cancellation,
    ) -> Result<Self, Trouble> {
        if cancellation.is_cancelled() {
            return Err(Trouble::Link(WsError::Cancelled));
        }

        let deadline = Instant::now() + timeout;
        let mut socket = WebSocket::connect(browser.address(), browser.browser_path(), timeout)?;
        socket.set_read_timeout(timeout)?;
        let mut connection = Self {
            socket,
            session: String::new(),
            next_id: 0,
            folding: Folding::new(kept),
            patience: timeout,
        };

        let page = await_page(deadline, cancellation, |patience| {
            connection.patience = patience;
            connection.call("Target.getTargets", json!({}))
        })?;

        // `flatten` is what makes the whole session run over this one socket:
        // every later message carries the session id this returns, and without
        // it each command would need a connection of its own.
        let attached = connection.call(
            "Target.attachToTarget",
            json!({ "targetId": page, "flatten": true }),
        )?;
        connection.session = text(&attached, "sessionId")
            .ok_or_else(|| Trouble::Refused {
                method: String::from("Target.attachToTarget"),
                said: String::from("the browser attached no session"),
            })?
            .to_owned();

        for domain in ["Page.enable", "Network.enable", "Runtime.enable"] {
            connection.call(domain, json!({}))?;
        }
        Ok(connection)
    }

    /// Sends a command and returns its result, folding the events it produces.
    ///
    /// **The events are folded rather than discarded, and that is the whole
    /// reason this is a loop instead of two lines.** A command's reply and the
    /// events it causes arrive interleaved on one socket, so a reader that
    /// waited for the reply and threw the rest away would lose every event that
    /// happened while a command was in flight — which includes the document's
    /// own response, the one the whole check reads the status from.
    fn call(&mut self, method: &str, params: Value) -> Result<Value, Trouble> {
        self.socket.set_read_timeout(self.patience)?;
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({ "id": id, "method": method, "params": params });
        if !self.session.is_empty() {
            message["sessionId"] = Value::String(self.session.clone());
        }
        self.socket.send_text(&message.to_string())?;

        loop {
            let Some(text) = self.socket.read_text()? else {
                return Err(Trouble::Link(WsError::Closed));
            };
            let Ok(reply) = serde_json::from_str::<Value>(&text) else {
                return Err(Trouble::Refused {
                    method: method.to_owned(),
                    said: format!("the browser sent {} that is not JSON", clipped(&text)),
                });
            };
            if reply.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = reply.get("error") {
                    return Err(Trouble::Refused {
                        method: method.to_owned(),
                        said: error.to_string(),
                    });
                }
                return Ok(reply.get("result").cloned().unwrap_or(Value::Null));
            }
            self.folding.absorb(&self.session, &reply);
        }
    }

    /// Opens the address, watches the page, and reports what it saw.
    ///
    /// **This does not return a `Result`, and that is the interface's shape
    /// rather than a convenience.** Once the address has been asked for, every
    /// outcome is an [`Observation`]: a browser that died mid-page saw a page
    /// that stopped being loaded, and a driver that reported *the tool would not
    /// start* for that would be answering a question about the machine when it
    /// had been asked one about the project.
    fn look(
        &mut self,
        target: &Target,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Observation {
        let address = target.url();
        let mut ended_naturally = false;

        match self.call("Page.navigate", json!({ "url": address })) {
            Ok(result) => {
                if let Some(frame) = text(&result, "frameId") {
                    self.folding.main_frame = Some(frame.to_owned());
                }
                // A navigation the browser itself refused — a connection
                // refused, a name that does not resolve — arrives as a
                // successful command carrying the reason, not as an error.
                if let Some(reason) = text(&result, "errorText").filter(|text| !text.is_empty()) {
                    self.folding.record(
                        ProblemKind::NavigationFailed,
                        reason.to_owned(),
                        address.clone(),
                        0,
                    );
                }
                ended_naturally = self.pump(deadline, cancellation);
            }
            Err(trouble) => {
                self.folding.record(
                    ProblemKind::NavigationFailed,
                    trouble.to_string(),
                    address.clone(),
                    0,
                );
            }
        }

        self.folding.a_page_that_never_arrived(&address);

        let title = self.title();
        let observation = self.folding.observation(title, ended_naturally);
        // Asked for rather than killed, so that the browser removes its own
        // profile and leaves cleanly. `Launched`'s drop is what stops it if it
        // does not, and a failure here is deliberately not reported: what the
        // page did is already recorded, and the browser's departure is SURE's
        // housekeeping and not a fact about the project.
        let _ = self.call("Browser.close", json!({}));
        observation
    }

    /// Reads events until the page has settled, the budget is spent, or the
    /// browser stops talking.
    ///
    /// Returns whether the look ended because it had nothing left to watch.
    fn pump(&mut self, deadline: Instant, cancellation: &Cancellation) -> bool {
        let mut settled_at: Option<Instant> = None;
        loop {
            if cancellation.is_cancelled() {
                return false;
            }
            if self.folding.overflowed {
                return false;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            // Floored at a millisecond because a sub-millisecond timeout is not
            // a shorter wait on Windows, it is an invalid socket option.
            let slice = remaining.min(websocket::POLL).max(Duration::from_millis(1));
            if self.socket.set_read_timeout(slice).is_err() {
                return false;
            }
            match self.socket.read_text() {
                Ok(Some(text)) => {
                    if let Ok(message) = serde_json::from_str::<Value>(&text) {
                        self.folding.absorb(&self.session, &message);
                        if self.folding.settled && settled_at.is_none() {
                            settled_at = Some(Instant::now());
                        }
                    }
                }
                // A close frame and a dropped socket are the same answer here:
                // the browser is no longer describing the page, so whatever was
                // seen is all there will be.
                Ok(None) => return false,
                Err(WsError::TimedOut) => {}
                Err(_) => return false,
            }
            if settled_at.is_some_and(|at| at.elapsed() >= SETTLE) {
                return true;
            }
        }
    }

    /// The document's title, or nothing.
    ///
    /// Asked for after the page has been watched rather than taken from the
    /// navigation, because a page's title is usually set by the scripts that run
    /// after the document is parsed — which is the same reason the settle window
    /// exists.
    fn title(&mut self) -> String {
        if self.folding.landing_url.trim().is_empty() {
            return String::new();
        }
        match self.call(
            "Runtime.evaluate",
            json!({ "expression": "document.title", "returnByValue": true }),
        ) {
            Ok(reply) => reply
                .get("result")
                .and_then(|value| text(value, "value"))
                .unwrap_or_default()
                .to_owned(),
            Err(_) => String::new(),
        }
    }
}

/// Waits for the first page within the remaining look budget. Chromium may
/// publish its debugging endpoint before it creates the initial `about:blank`
/// target; issue #15 caught that exact empty reply on Ubuntu. Only an empty
/// successful reply is retried. A protocol or socket error remains an error.
fn await_page(
    deadline: Instant,
    cancellation: &Cancellation,
    mut targets: impl FnMut(Duration) -> Result<Value, Trouble>,
) -> Result<String, Trouble> {
    loop {
        if cancellation.is_cancelled() {
            return Err(Trouble::Link(WsError::Cancelled));
        }
        let patience = deadline
            .saturating_duration_since(Instant::now())
            .max(NO_LESS_THAN);
        let reply = targets(patience)?;
        let page = reply
            .get("targetInfos")
            .and_then(Value::as_array)
            .and_then(|infos| infos.iter().find(|info| text(info, "type") == Some("page")))
            .and_then(|info| text(info, "targetId"));
        if let Some(page) = page {
            return Ok(page.to_owned());
        }
        if Instant::now() >= deadline {
            return Err(Trouble::Refused {
                method: String::from("Target.getTargets"),
                said: String::from(
                    "the browser reported no page to attach to before the look budget ended",
                ),
            });
        }
        std::thread::sleep(TARGET_POLL);
    }
}

/// Opens `target` in a browser and reports what the page did.
///
/// `program` is the browser to drive, or `None` to look for one. A caller that
/// named a program gets that program or
/// [`AbsenceReason::DriverWouldNotStart`]; a caller that did not gets
/// [`AbsenceReason::NoDriverInstalled`] when the search finds nothing, which is
/// a different fact and a different sentence.
pub(crate) fn open(
    program: Option<&Path>,
    target: &Target,
    limits: &Limits,
    cancellation: &Cancellation,
) -> Report {
    if cancellation.is_cancelled() {
        return Report::absent(
            AbsenceReason::DriverWouldNotStart,
            "the browser check was cancelled before it started",
        );
    }

    let started = Instant::now();
    let Some(program) = program.map(Path::to_path_buf).or_else(installed::find) else {
        return Report::absent(
            AbsenceReason::NoDriverInstalled,
            "no Chrome, Edge, Chromium or Brave was found where this build looks, or on PATH",
        );
    };

    // What is left of the budget rather than a fresh one, so that one look
    // cannot take longer than the caller allowed however the time is spent.
    let launch_left = limits
        .timeout()
        .saturating_sub(started.elapsed())
        .max(NO_LESS_THAN);

    let mut browser = match launch::start(&program, launch_left, cancellation) {
        Ok(browser) => browser,
        Err(error) => {
            return Report::absent(AbsenceReason::DriverWouldNotStart, error.to_string());
        }
    };

    let attach_left = limits
        .timeout()
        .saturating_sub(started.elapsed())
        .max(NO_LESS_THAN);
    let mut connection =
        match Connection::attach(&browser, attach_left, limits.problems_kept(), cancellation) {
            Ok(connection) => connection,
            Err(trouble) => {
                return Report::absent(
                    AbsenceReason::DriverWouldNotStart,
                    format!("{}: {trouble}", program.display()),
                );
            }
        };

    // Everything from here is about the page. The boundary is drawn at the
    // point where a page could first be asked for, and not at the point where
    // something goes wrong, because the answer a caller needs — *was there a
    // page to look at* — is decided by that and not by the failure.
    let deadline = started + limits.timeout();
    let observation = connection.look(target, deadline, cancellation);

    // In this order, and the order is the point: the socket closes first so the
    // browser hears nothing more, then the browser is given its moment to leave,
    // and then whatever is left is taken down. Dropping `browser` stops a tree
    // that is still running without waiting for another one to answer.
    drop(connection);
    browser.wait_for_exit(DEPARTURE);
    drop(browser);

    Report::observed(observation)
}

/// A string as a JSON field, if it is there and is a string.
fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// A number as a JSON field, if it is there and is not negative.
fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

/// A flag as a JSON field, defaulting to false when it is absent.
fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// The innermost frame a stack trace names, which is where the thing happened.
fn first_frame(value: &Value) -> Option<&Value> {
    value
        .get("stackTrace")?
        .get("callFrames")?
        .as_array()?
        .first()
}

/// A line as a person counts it, from what CDP counted.
///
/// The offset of one is measured, not assumed — see the module documentation —
/// and it is applied here rather than at each of the three places a line is
/// read, so that a fourth one added later cannot forget it.
fn one_based(zero_based: u64) -> u32 {
    u32::try_from(zero_based.saturating_add(1)).unwrap_or(u32::MAX)
}

/// One console argument, as a person would read it.
///
/// A string argument is its own value; anything else is the browser's own
/// description of it, which is what carries an object's shape and an error's
/// stack.
fn argument_text(argument: &Value) -> String {
    if let Some(value) = argument.get("value") {
        if let Some(spoken) = value.as_str() {
            return spoken.to_owned();
        }
        return value.to_string();
    }
    if let Some(described) = argument.get("description").and_then(Value::as_str) {
        return described.to_owned();
    }
    argument
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("something")
        .to_owned()
}

/// The beginning of a message that is too long to repeat, in a form that shows
/// where it was cut.
fn clipped(text: &str) -> String {
    const KEPT: usize = 120;
    let mut clipped: String = text.chars().take(KEPT).collect();
    if text.chars().count() > KEPT {
        clipped.push('…');
    }
    format!("{clipped:?}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const OURS: &str = "a-session";

    #[test]
    fn an_empty_target_list_can_precede_the_page_the_browser_then_creates() {
        let mut replies = vec![
            json!({ "targetInfos": [] }),
            json!({ "targetInfos": [{ "type": "page", "targetId": "INITIAL" }] }),
        ]
        .into_iter();
        let page = await_page(
            Instant::now() + Duration::from_secs(1),
            &Cancellation::new(),
            |_| Ok(replies.next().expect("only two target polls")),
        )
        .expect("the second reply contains the initial page");
        assert_eq!(page, "INITIAL");
        assert!(
            replies.next().is_none(),
            "the page took two polls to appear"
        );
    }

    #[test]
    fn a_browser_that_never_creates_a_page_still_fails_visibly() {
        let error = await_page(Instant::now(), &Cancellation::new(), |_| {
            Ok(json!({ "targetInfos": [] }))
        })
        .expect_err("an empty target list cannot be treated as a page");
        assert!(
            matches!(error, Trouble::Refused { .. }),
            "no page remains a browser error: {error}"
        );
    }

    /// One message, shaped the way the browser sends them.
    fn event(method: &str, params: Value) -> Value {
        json!({ "sessionId": OURS, "method": method, "params": params })
    }

    /// A folding that has seen `params`, and nothing else.
    fn folded(kept: usize, params: Value) -> Folding {
        let mut folding = Folding::new(kept);
        folding.absorb(OURS, &params);
        folding
    }

    /// The page's own frame, as `Page.frameNavigated` reports it.
    fn a_page_at(url: &str) -> Value {
        event(
            "Page.frameNavigated",
            json!({ "frame": { "id": "FRAME", "url": url } }),
        )
    }

    /// A response for the page's own request.
    fn the_page_answered(status: u16) -> Value {
        event(
            "Network.responseReceived",
            json!({
                "frameId": "FRAME",
                "type": "Document",
                "response": { "status": status, "url": "http://127.0.0.1:3000/" },
            }),
        )
    }

    /// A response for something the page asked for.
    fn a_subresource(kind: &str, status: u16, url: &str) -> Value {
        event(
            "Network.responseReceived",
            json!({
                "frameId": "FRAME",
                "type": kind,
                "response": { "status": status, "url": url },
            }),
        )
    }

    #[test]
    fn the_page_is_where_the_frame_with_no_parent_went() {
        let mut folding = Folding::new(10);
        folding.absorb(
            OURS,
            &event(
                "Page.frameNavigated",
                json!({ "frame": { "id": "INNER", "parentId": "FRAME", "url": "http://127.0.0.1:3000/iframe" } }),
            ),
        );
        assert!(
            folding.landing_url.is_empty(),
            "a subframe's address was taken for the page's"
        );

        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/app"));
        assert_eq!(folding.landing_url, "http://127.0.0.1:3000/app");
        assert_eq!(folding.main_frame.as_deref(), Some("FRAME"));
    }

    /// **The last document response is the one that counts**, which is a false
    /// green the other way round: a redirect chain that ends on a 404 would
    /// record the 302, and 302 is inside the window a page can be shown from.
    #[test]
    fn a_redirect_that_ends_on_a_missing_page_records_the_missing_page() {
        let mut folding = Folding::new(10);
        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/"));
        folding.absorb(OURS, &the_page_answered(302));
        folding.absorb(OURS, &the_page_answered(404));

        assert_eq!(
            folding.document_status,
            Some(404),
            "the first response was kept, and a 302 is inside the window a page can be shown from"
        );
        assert_eq!(
            Report::observed(folding.observation(String::new(), true)).status(),
            sure_domain::status::CheckStatus::Fail
        );
    }

    #[test]
    fn a_page_that_was_served_and_reported_nothing_is_the_only_green_shape() {
        let mut folding = Folding::new(10);
        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/"));
        folding.absorb(OURS, &the_page_answered(200));
        folding.absorb(OURS, &event("Page.loadEventFired", json!({})));

        let observation = folding.observation(String::from("Example"), true);
        assert!(observation.complete);
        assert_eq!(
            Report::observed(observation).status(),
            sure_domain::status::CheckStatus::Pass
        );
    }

    /// **The measured rule, in both directions.** A stylesheet, a script, an
    /// image, a font, a fetch and an XHR that 404 are problems; the browser's
    /// own favicon request is not, because the browser files it as `Other` and
    /// filed none of the others that way.
    #[test]
    fn a_failed_subresource_is_a_problem_and_the_browser_s_own_request_is_not() {
        for kind in ["Stylesheet", "Script", "Image", "Fetch", "XHR", "Font"] {
            let folding = folded(10, a_subresource(kind, 404, "/missing"));
            assert_eq!(
                folding.problems.len(),
                1,
                "a {kind} that 404'd was not reported"
            );
            assert_eq!(folding.problems[0].kind, ProblemKind::FailedRequest);
            assert!(folding.problems[0].message.contains("404"));
            assert_eq!(folding.problems[0].source, "/missing");
        }

        let browser_own = folded(10, a_subresource("Other", 404, "/favicon.ico"));
        assert!(
            browser_own.problems.is_empty(),
            "the browser's own request for a favicon was reported as the page's problem, \
             which is what a healthy local page was measured to produce"
        );

        // **And the same request failing before it is answered, which is the
        // half that had no test.** The assertion above is about
        // `Network.responseReceived` and says nothing about the other event a
        // favicon produces: a request that never got a response arrives on
        // `Network.loadingFailed`, with no `canceled` flag, so the *cancelled*
        // rule does not cover it and the only thing that can is the type. A
        // driver that checked the type on one path and not the other would
        // report a problem for every page on a site whose favicon 404s at the
        // connection, which is the same false alarm in a different event.
        let browser_own_failed = folded(
            10,
            event(
                "Network.loadingFailed",
                json!({
                    "requestId": "1",
                    "type": "Other",
                    "errorText": "net::ERR_CONNECTION_REFUSED",
                }),
            ),
        );
        assert!(
            browser_own_failed.problems.is_empty(),
            "the browser's own failed request was reported as the page's problem: {:?}",
            browser_own_failed.problems
        );

        // The other direction on the same path, so that the case above is not
        // satisfied by a rule that drops every failure: a request the page
        // itself made and that failed is reported.
        let page_failed = folded(
            10,
            event(
                "Network.loadingFailed",
                json!({
                    "requestId": "1",
                    "type": "Script",
                    "errorText": "net::ERR_CONNECTION_REFUSED",
                }),
            ),
        );
        assert_eq!(
            page_failed.problems.len(),
            1,
            "a script the page asked for and that failed was not reported"
        );
        assert_eq!(page_failed.problems[0].kind, ProblemKind::FailedRequest);
    }

    /// A subresource may answer with a status inside the page window and still
    /// not be a page; and one outside the *other* window — a `204`, a `304` —
    /// is not a failure. The two windows are different questions and the code
    /// must not borrow one for the other.
    #[test]
    fn a_subresource_that_was_served_is_not_a_failure_whatever_it_answered_with() {
        for status in [200, 204, 206, 301, 304, 307] {
            let folding = folded(10, a_subresource("Script", status, "/app.js"));
            assert!(
                folding.problems.is_empty(),
                "HTTP {status} for a subresource was called a failure"
            );
        }
        for status in [400, 403, 404, 500, 503] {
            let folding = folded(10, a_subresource("Script", status, "/app.js"));
            assert_eq!(folding.problems.len(), 1, "HTTP {status} was not a failure");
        }
    }

    /// **A page that aborts a request is not a page that failed one.** This is
    /// the rule that was wrong first, and the measurement is that an abort
    /// produces exactly one event and nothing else.
    #[test]
    fn a_request_the_page_itself_cancelled_is_not_a_problem() {
        let folding = folded(
            10,
            event(
                "Network.loadingFailed",
                json!({ "requestId": "1", "type": "Fetch", "errorText": "net::ERR_ABORTED", "canceled": true }),
            ),
        );
        assert!(
            folding.problems.is_empty(),
            "an AbortController abort was reported as a failed request"
        );
    }

    /// A request that failed on the way, with the address it was made for,
    /// joined from the request that began it — because `loadingFailed` carries
    /// an identifier and no URL.
    #[test]
    fn a_request_that_did_not_complete_names_the_address_it_was_made_for() {
        let mut folding = Folding::new(10);
        folding.absorb(
            OURS,
            &event(
                "Network.requestWillBeSent",
                json!({ "requestId": "7", "type": "Fetch", "request": { "url": "http://127.0.0.1:3000/api" } }),
            ),
        );
        folding.absorb(
            OURS,
            &event(
                "Network.loadingFailed",
                json!({ "requestId": "7", "type": "Fetch", "errorText": "net::ERR_CONNECTION_REFUSED" }),
            ),
        );
        assert_eq!(folding.problems.len(), 1);
        assert_eq!(folding.problems[0].kind, ProblemKind::FailedRequest);
        assert_eq!(folding.problems[0].source, "http://127.0.0.1:3000/api");
        assert!(
            folding.problems[0]
                .message
                .contains("ERR_CONNECTION_REFUSED")
        );
    }

    /// **A line a person can open.** CDP counts from zero and the report counts
    /// from one, and the measurement that settles it is that the same payload
    /// renders the same line two ways: `lineNumber: 2` beside a description
    /// saying `:3:15`.
    #[test]
    fn a_line_is_reported_the_way_the_file_counts_it() {
        let folding = folded(
            10,
            event(
                "Runtime.exceptionThrown",
                json!({
                    "exceptionDetails": {
                        "text": "Uncaught",
                        "url": "http://127.0.0.1:3000/",
                        "lineNumber": 2,
                        "columnNumber": 14,
                        "exception": {
                            "description": "Error: boom\n    at http://127.0.0.1:3000/:3:15"
                        },
                    }
                }),
            ),
        );
        assert_eq!(folding.problems.len(), 1);
        let problem = &folding.problems[0];
        assert_eq!(problem.kind, ProblemKind::RuntimeError);
        assert_eq!(problem.line, 3, "the zero-based line was not converted");
        assert!(
            problem.message.contains("boom"),
            "the description was not preferred to the bare word `Uncaught`: {:?}",
            problem.message
        );
        assert_eq!(problem.source, "http://127.0.0.1:3000/");
    }

    /// **Zero stays reserved for *no line was reported*.** A `throw` on the
    /// first line of a document has a zero-based line of zero, and it must not
    /// come out looking like a line the browser never mentioned.
    #[test]
    fn a_line_the_browser_did_not_report_is_not_the_first_line() {
        let reported = folded(
            10,
            event(
                "Runtime.exceptionThrown",
                json!({ "exceptionDetails": { "text": "Uncaught", "lineNumber": 0 } }),
            ),
        );
        assert_eq!(reported.problems[0].line, 1);
        assert!(!reported.problems[0].describe().contains("no line reported"));

        let silent = folded(
            10,
            event(
                "Runtime.exceptionThrown",
                json!({ "exceptionDetails": { "text": "Uncaught" } }),
            ),
        );
        assert_eq!(silent.problems[0].line, 0);
        assert!(silent.problems[0].describe().contains("no line reported"));
    }

    #[test]
    fn a_console_error_carries_what_was_written_to_it() {
        let folding = folded(
            10,
            event(
                "Runtime.consoleAPICalled",
                json!({
                    "type": "error",
                    "args": [{ "type": "string", "value": "could not load" }, { "type": "number", "value": 7 }],
                    "stackTrace": { "callFrames": [{ "url": "http://127.0.0.1:3000/app.js", "lineNumber": 0 }] },
                }),
            ),
        );
        assert_eq!(folding.problems.len(), 1);
        let problem = &folding.problems[0];
        assert_eq!(problem.kind, ProblemKind::ConsoleError);
        assert_eq!(problem.message, "could not load 7");
        assert_eq!(problem.source, "http://127.0.0.1:3000/app.js");
        assert_eq!(problem.line, 1);
    }

    /// A page that logs is not a page that failed, and neither is one that
    /// warns. Only `error` is a problem.
    #[test]
    fn a_console_message_that_is_not_an_error_is_not_a_problem() {
        for kind in ["log", "info", "warning", "debug", "table", "trace"] {
            let folding = folded(
                10,
                event(
                    "Runtime.consoleAPICalled",
                    json!({ "type": kind, "args": [{ "type": "string", "value": "hello" }] }),
                ),
            );
            assert!(
                folding.problems.is_empty(),
                "a console.{kind} was reported as a problem"
            );
        }
    }

    /// An event from a session SURE did not attach to is not this page's.
    #[test]
    fn a_message_from_another_session_is_not_absorbed() {
        let mut folding = Folding::new(10);
        let mut elsewhere = a_page_at("http://127.0.0.1:3000/other");
        elsewhere["sessionId"] = Value::String(String::from("somebody-else"));
        folding.absorb(OURS, &elsewhere);
        assert!(folding.landing_url.is_empty());

        // And a message with no session at all, which is how the browser sends
        // its own events — `Target.attachedToTarget` among them.
        let mut nobody = a_page_at("http://127.0.0.1:3000/nobody");
        nobody.as_object_mut().unwrap().remove("sessionId");
        folding.absorb(OURS, &nobody);
        assert!(folding.landing_url.is_empty());
    }

    /// The browser is allowed to say things this driver does not understand,
    /// and a reply that is not an event at all is the common case.
    #[test]
    fn a_message_that_is_not_an_event_this_driver_knows_is_ignored() {
        let mut folding = Folding::new(10);
        for message in [
            json!({ "id": 1, "result": {} }),
            event("Target.attachedToTarget", json!({ "sessionId": OURS })),
            json!({ "sessionId": OURS, "method": "Page.somethingNew", "params": null }),
            json!({ "sessionId": OURS }),
        ] {
            folding.absorb(OURS, &message);
        }
        assert!(folding.problems.is_empty());
        assert!(folding.landing_url.is_empty());
        assert_eq!(folding.document_status, None);
    }

    /// **Going over the bound stops the look instead of growing the list**, and
    /// the look is then not complete — so a page with more errors than the
    /// driver would hold is never described as clean.
    #[test]
    fn more_problems_than_the_bound_stops_the_look_rather_than_growing_the_list() {
        let mut folding = Folding::new(4);
        for index in 0..20 {
            folding.absorb(
                OURS,
                &event(
                    "Runtime.consoleAPICalled",
                    json!({
                        "type": "error",
                        "args": [{ "type": "string", "value": format!("error {index}") }],
                    }),
                ),
            );
        }
        assert_eq!(folding.problems.len(), 4);
        assert!(folding.overflowed);
        folding.settled = true;
        let observation = folding.observation(String::new(), true);
        assert!(!observation.complete);
        assert_eq!(
            Report::observed(observation).status(),
            sure_domain::status::CheckStatus::Fail,
            "the problems that were kept still count"
        );
    }

    /// Every problem is kept, in the order it arrived, so that the report's
    /// first three lines are the browser's first three problems.
    #[test]
    fn problems_keep_the_order_the_browser_reported_them_in() {
        let mut folding = Folding::new(10);
        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/"));
        folding.absorb(OURS, &a_subresource("Script", 404, "/first.js"));
        folding.absorb(
            OURS,
            &event(
                "Runtime.exceptionThrown",
                json!({ "exceptionDetails": { "text": "second" } }),
            ),
        );
        folding.absorb(OURS, &a_subresource("Image", 500, "/third.png"));

        let sources: Vec<&str> = folding
            .problems
            .iter()
            .map(|problem| problem.source.as_str())
            .collect();
        assert_eq!(
            sources,
            vec!["/first.js", "http://127.0.0.1:3000/", "/third.png"],
            "the middle one is the exception, which named no file and is therefore \
             attributed to the page's own address"
        );
        assert_eq!(folding.problems[1].kind, ProblemKind::RuntimeError);
    }

    /// **An error the browser gave no location for is attributed to the page's
    /// own address, and never left blank.**
    ///
    /// This is the other half of the test above, which asserts the order and
    /// would pass just as happily against an empty source. Empty is the wrong
    /// answer twice over:
    ///
    /// * `Problem::describe` prints `{source} (no line reported)` or
    ///   `{source}:{line}`, so a blank source produces a reason line beginning
    ///   with a colon — *"uncaught error: … — :7"* — which reads as a bug in the
    ///   report rather than as a missing fact about the page.
    /// * The interface already says what belongs there. `Problem.source` is
    ///   documented as *"a script URL, a file, or **the address itself**"*, and
    ///   the address is a fact about this message rather than a guess at it:
    ///   `Runtime.enable` was sent on the page's own session, so an exception
    ///   absorbed here happened in the document SURE opened, whatever file the
    ///   browser did or did not name.
    ///
    /// The line stays zero, which is printed as the absence it is. So the reader
    /// is told *where in the project* and is told *the browser did not say
    /// where in the file*, which is the honest pair.
    #[test]
    fn an_error_with_no_location_the_browser_gave_is_attributed_to_the_page() {
        let mut folding = Folding::new(10);
        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/app"));
        folding.absorb(
            OURS,
            &event(
                "Runtime.exceptionThrown",
                json!({ "exceptionDetails": { "text": "no file named" } }),
            ),
        );
        assert_eq!(folding.problems.len(), 1);
        assert_eq!(folding.problems[0].source, "http://127.0.0.1:3000/app");
        assert_eq!(folding.problems[0].line, 0);
        assert!(
            !folding.problems[0].describe().contains("— :"),
            "a blank source reached the reason line: {}",
            folding.problems[0].describe()
        );
    }

    /// **The same rule on the console path**, which is the half that had no
    /// test: a `console.error` called from something that carries no stack —
    /// an inline handler compiled at run time, an eval, a worker — arrives with
    /// no frame at all, and a driver that fell back to nothing would render a
    /// reason line whose location is a colon and a space.
    ///
    /// It is a separate test rather than a case in the one above because the
    /// two are different code paths through the same decision, and the mutation
    /// harness is what showed the difference: a mutation that removed the
    /// fallback here was caught by nothing, while the identical mutation on the
    /// exception path was caught by the test above.
    #[test]
    fn a_console_error_with_no_location_the_browser_gave_is_attributed_to_the_page() {
        let mut folding = Folding::new(10);
        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/app"));
        folding.absorb(
            OURS,
            &event(
                "Runtime.consoleAPICalled",
                json!({
                    "type": "error",
                    "args": [{ "type": "string", "value": "it broke" }],
                }),
            ),
        );
        assert_eq!(folding.problems.len(), 1);
        assert_eq!(folding.problems[0].kind, ProblemKind::ConsoleError);
        assert_eq!(folding.problems[0].source, "http://127.0.0.1:3000/app");
        assert_eq!(folding.problems[0].line, 0);
        assert!(
            !folding.problems[0].describe().contains("— :"),
            "a blank source reached the reason line: {}",
            folding.problems[0].describe()
        );
    }

    /// **An address that was opened and produced no page is a problem, and it
    /// is reported exactly once.**
    ///
    /// Three cases, and the middle two are the two halves of the guard, which
    /// fail in opposite directions: one would put a *never arrived* problem on a
    /// page that arrived, and the other would say the same thing twice about a
    /// page that did not — the second time in SURE's words, crowding out the
    /// browser's own reason.
    #[test]
    fn an_address_that_was_opened_and_never_arrived_is_reported_once() {
        let address = "http://127.0.0.1:3000/";

        // The browser said nothing at all: SURE says this.
        let mut silent = Folding::new(10);
        silent.a_page_that_never_arrived(address);
        assert_eq!(silent.problems.len(), 1);
        assert_eq!(silent.problems[0].kind, ProblemKind::NavigationFailed);
        assert_eq!(silent.problems[0].source, address);
        assert_eq!(silent.problems[0].line, 0);

        // A page that did arrive is not a page that never arrived.
        let mut arrived = Folding::new(10);
        arrived.absorb(OURS, &a_page_at(address));
        arrived.a_page_that_never_arrived(address);
        assert!(
            arrived.problems.is_empty(),
            "a page that arrived was reported as one that never did: {:?}",
            arrived.problems
        );

        // And the browser's own reason is not repeated in SURE's words.
        let mut refused = Folding::new(10);
        refused.record(
            ProblemKind::NavigationFailed,
            "net::ERR_CONNECTION_REFUSED",
            address,
            0,
        );
        refused.a_page_that_never_arrived(address);
        assert_eq!(
            refused.problems.len(),
            1,
            "the same refusal was reported twice: {:?}",
            refused.problems
        );
        assert_eq!(refused.problems[0].message, "net::ERR_CONNECTION_REFUSED");
    }

    /// **A look that spent its budget is not complete even if the page had
    /// loaded**, because the settle window is part of the look and half of one
    /// is not a reason to call a page clean.
    #[test]
    fn a_look_that_was_cut_short_is_not_complete_however_far_it_got() {
        let mut folding = Folding::new(10);
        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/"));
        folding.absorb(OURS, &the_page_answered(200));
        folding.absorb(OURS, &event("Page.loadEventFired", json!({})));
        assert!(folding.settled);

        let cut_short = folding.observation(String::new(), false);
        assert!(!cut_short.complete);
        assert_eq!(
            Report::observed(cut_short).status(),
            sure_domain::status::CheckStatus::Unknown
        );
    }

    /// The page's request is the one from the frame the address was opened in,
    /// and a subframe's document response is not the page's.
    #[test]
    fn the_page_s_status_comes_from_the_page_s_own_frame() {
        let mut folding = Folding::new(10);
        folding.absorb(OURS, &a_page_at("http://127.0.0.1:3000/"));
        folding.absorb(
            OURS,
            &event(
                "Network.responseReceived",
                json!({
                    "frameId": "INNER",
                    "type": "Document",
                    "response": { "status": 500, "url": "http://127.0.0.1:3000/iframe" },
                }),
            ),
        );
        assert_eq!(
            folding.document_status, None,
            "a subframe's document response was taken for the page's"
        );
        // It is still a failed request the page's own markup asked for.
        assert_eq!(folding.problems.len(), 1);
        assert_eq!(folding.problems[0].source, "http://127.0.0.1:3000/iframe");
    }

    /// The request table is bounded, so a page that makes a very large number
    /// of requests cannot grow it without limit; a failure past the bound is
    /// still reported, without its address.
    #[test]
    fn the_table_of_request_addresses_stops_growing() {
        let mut folding = Folding::new(10);
        for index in 0..(NAMED_REQUESTS + 100) {
            folding.absorb(
                OURS,
                &event(
                    "Network.requestWillBeSent",
                    json!({ "requestId": format!("{index}"), "request": { "url": format!("/{index}") } }),
                ),
            );
        }
        assert_eq!(folding.requests.len(), NAMED_REQUESTS);

        folding.absorb(
            OURS,
            &event(
                "Network.loadingFailed",
                json!({ "requestId": "999999", "type": "Fetch", "errorText": "net::ERR_FAILED" }),
            ),
        );
        assert_eq!(folding.problems.len(), 1);
        assert_eq!(folding.problems[0].source, "");
        assert!(folding.problems[0].message.contains("ERR_FAILED"));
    }

    #[test]
    fn one_console_argument_that_is_not_a_string_is_still_readable() {
        assert_eq!(
            argument_text(&json!({ "type": "string", "value": "a" })),
            "a"
        );
        assert_eq!(argument_text(&json!({ "type": "number", "value": 7 })), "7");
        assert_eq!(
            argument_text(&json!({ "type": "boolean", "value": false })),
            "false"
        );
        assert_eq!(
            argument_text(&json!({ "type": "object", "description": "Error: boom" })),
            "Error: boom"
        );
        assert_eq!(argument_text(&json!({ "type": "undefined" })), "undefined");
        assert_eq!(argument_text(&json!({})), "something");
    }

    #[test]
    fn a_long_message_is_cut_where_a_reader_can_see_that_it_was() {
        let short = clipped("hello");
        assert_eq!(short, "\"hello\"");
        let long = clipped(&"x".repeat(500));
        assert!(long.ends_with("…\""), "{long}");
        assert!(long.chars().count() < 200, "the clipping kept too much");
    }
}

//! The Model Context Protocol bridge: `sure mcp serve`.
//!
//! A coding harness launches this process and speaks JSON-RPC to it on standard
//! input and standard output. SURE never listens on anything: there is no
//! socket, no port, no bound address, and no code in this module that could
//! open one. The transport is the caller's own child process, which is what
//! `docs/architecture/MCP_BRIDGE.md` means by "stdio for v0.1" and "no hidden
//! network listener".
//!
//! # Where the protocol came from
//!
//! The framing and the field names here were read from the specification at
//! `modelcontextprotocol.io` on 2026-09-18, at revision `2025-11-25`, rather
//! than recalled. The pages, and what each one settled:
//!
//! | Page | What it settled |
//! | --- | --- |
//! | `.../2025-11-25/basic` | messages are JSON-RPC 2.0; a request identifier MUST NOT be null; a notification MUST NOT carry one and is never answered; an error response echoes the identifier "except in error cases where the ID could not be read due a malformed request", which is why a parse error answers with a null `id` |
//! | `.../2025-11-25/basic/transports` | stdio framing: "Messages are delimited by newlines, and **MUST NOT** contain embedded newlines"; "The server **MUST NOT** write anything to its `stdout` that is not a valid MCP message"; "The server **MAY** write UTF-8 strings to its standard error (`stderr`) for any logging purposes"; the client closes the input stream first, which is how a stdio session ends |
//! | `.../2025-11-25/basic/lifecycle` | "the initialization phase **MUST** be the first interaction between client and server"; version negotiation: if the server supports the requested version it "**MUST** respond with the same version. Otherwise, the server **MUST** respond with another protocol version it supports. This **SHOULD** be the *latest* version supported by the server" |
//! | `.../2025-11-25/server/tools` | a tool needs `name`, `description` and `inputSchema`; a tool with no parameters uses `{"type": "object", "additionalProperties": false}`; a `tools/call` result is `{content, structuredContent?, isError?}` with `content` as a list of `TextContent` blocks; an unknown tool is a *protocol error* whose example message is `Unknown tool: invalid_tool_name` with code `-32602`; "For backwards compatibility, a tool that returns structured content **SHOULD** also return the serialized JSON in a `TextContent` block" |
//!
//! Two things the specification deliberately leaves open, and which are SURE's
//! own decisions rather than facts read from it:
//!
//! 1. **The code for "not initialized yet".** JSON-RPC reserves `-32000` to
//!    `-32099` for implementations; the specification assigns nothing in it for
//!    this case, and `-32002` is what the reference implementations use. SURE
//!    uses it because a client that already handles the reference servers
//!    understands it, and says so here because it is not in the specification.
//! 2. **A size bound.** The specification sets no limit on a message, and a
//!    server that allocates whatever a caller sends is a server a caller can
//!    stop with one line. [`MAX_MESSAGE_BYTES`] is SURE's, and a message over it
//!    is refused as a parse error **and the rest of the line is discarded**, so
//!    that the next message the caller sends is read as a message rather than
//!    as the tail of the last one.
//!
//! Every other number, word and field spelling in this module is one of those
//! four pages. Where this build deviates on purpose, the deviation is written
//! down at the place it happens; the one that matters is in [`tool_result`],
//! where the text block carries the command's own prose rather than a second
//! copy of the JSON.
//!
//! The specification's stdout rule is not a deviation and has no exception.
//! The session summary is a [`Report::McpSession`] and
//! [`Format::emit`](crate::output::Format::emit) writes one of those to stderr
//! under `--format json` as well as under the default format, so nothing which
//! is not a protocol message can reach the caller's stream in any mode. That
//! decision lives in `output.rs`, because this module cannot name a stream.
//!
//! # Why every tool runs through `Command::report`
//!
//! ADR 0001 gives this program exactly one edge into the engine and exactly one
//! dispatch: [`Command::report`](crate::commands::Command::report) matches the
//! grammar exhaustively, and that is where "what SURE does about this command"
//! is decided. A bridge that checked something itself — parsed a project,
//! looked at a file, built a verdict — would be a second implementation of
//! checking, reachable only through a harness, and the day the two disagreed the
//! harness would be the one telling the agent the project was fine.
//!
//! So a tool does not check anything. It builds the `Command` the caller's
//! request corresponds to and asks the dispatch, and what comes back is put in
//! the tool result unmodified: [`Report::frame`] for a program, and
//! [`Report::human_text`] for the words, which are the same words the command
//! line would have printed. That is why a tool whose command this build cannot
//! carry out answers with that command's own refusal — `sure config is not
//! implemented in this build.` — reached through the same dispatch rather than
//! written here. A refusal written here would be a second explanation of the
//! same missing work, free to drift from the first.
//!
//! `sure_check`, `sure_recheck` and `sure_get_repair` are no longer refusals:
//! since the check pipeline runs, those three answer with the run's own result —
//! its verdict, what it checked and what it did not. The rule did not change
//! with them; the honest answer did. A tool that cannot finish (an unreadable
//! project, a history it cannot open) answers with that failure and `isError`
//! true, and a tool that finished answers `isError` false **and carries a
//! verdict that is not clean**, which is where the caller reads it.
//!
//! That has one consequence worth stating plainly, because it is the shape of
//! every answer this build gives: **the harness is told the same thing the
//! user is told.** There is no path in this module that turns "this build
//! cannot do that" into a result an agent would read as success —
//! [`tool_result`]'s `isError` is `!Report::is_an_answer`, and the test that
//! pins it is in `crates/sure-cli/tests/mcp_protocol.rs`.
//!
//! # What a caller may and may not say
//!
//! A tool call may name a project, and nothing else. Execution mode, privacy
//! settings and protection policy are read from the user's own SURE
//! configuration, on the user's own machine, and **no argument here can select
//! or override them** — `docs/architecture/MCP_BRIDGE.md` requires it, and the
//! refusal in [`Session::call_tool`] says so in those words, so a caller that
//! tried is told what happened rather than being silently ignored. A project
//! path is passed to the command unchanged, so whatever the command line does
//! with a relative, missing or unreadable path is what the tool does with it,
//! and the answer arrives as the same refusal.
//!
//! # Why nothing here names a stream
//!
//! `crates/sure-cli/tests/cli_contract.rs` scans the sources and fails if any
//! module outside [`crate::output`] refers to `stdout` or `stderr`. This module
//! writes protocol messages and is no exception: every message goes out through
//! [`Format::Json::emit`](crate::output::Format::emit) on a [`Report::Mcp`],
//! whose machine form *is* the message. That is also what keeps the
//! specification's stdout rule true mechanically rather than by review: the
//! only thing this module can put on the caller's stream is a protocol message,
//! because that is the only kind of report it can build.
//!
//! # The shape of the loop
//!
//! A frame goes in, a decision comes out, and the decision is written. The
//! decision is [`Session::handle`], which is a pure function of the session and
//! one line of input: it runs commands, it builds messages, and it writes
//! nothing. [`converse`] does the reading and the writing. That split is why
//! the tests at the bottom of this module can check the handshake, the refusals
//! and every tool result without a process and without a pipe.

use std::io::{self, BufRead, Read};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::cli::{Command, HistoryAction, McpAction};
use crate::output::Format;
use crate::report::{Failed, McpMessage, McpSession, Report};

/// The revision of the Model Context Protocol this build speaks.
///
/// One revision, and the negotiation in [`Session::initialize`] has one member
/// because of it. A build that claimed to speak several and implemented one
/// would be the CLI's version of a flag that does nothing.
pub const PROTOCOL_VERSION: &str = "2025-11-25";

/// The name SURE gives itself in the handshake.
const SERVER_NAME: &str = "sure";

/// The JSON-RPC version in every message, in both directions.
const JSONRPC: &str = "2.0";

/// The largest message SURE will hold in memory, in bytes.
///
/// The specification sets no limit; this one is SURE's, and it is generous
/// rather than tight: every legitimate message on this surface is a few hundred
/// bytes, because the largest thing a caller can send is a project path. See
/// the module comment for what happens to a caller that sends more.
///
/// Public so that the test which sends more can measure against the bound
/// rather than repeat it.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// How much of an over-long line SURE will throw away before giving up.
///
/// A caller that sends an unbounded line cannot be re-synchronised: every byte
/// after the first [`MAX_MESSAGE_BYTES`] is still part of the message SURE
/// refused, so the session can only go on once a newline arrives. Past this
/// much, SURE stops pretending it can find one and ends the session with a
/// failure rather than discarding a stream forever.
const MAX_DISCARDED_BYTES: usize = 64 * 1024 * 1024;

/// The longest `protocolVersion` SURE will accept, in bytes.
///
/// The revision is a date, `2025-11-25`. Thirty-two bytes is ten times what one
/// needs, and the bound exists because the string is kept in the session
/// summary and printed for a person: an unbounded one would be caller text
/// arriving where a user expects four numbers and two dashes.
const MAX_PROTOCOL_VERSION_BYTES: usize = 32;

/// The longest caller-supplied word SURE will repeat in a sentence.
const MAX_QUOTED_BYTES: usize = 64;

/// JSON-RPC's own error codes, and the one SURE adds.
///
/// The first four are the specification's. [`NOT_INITIALIZED`] is not: it is in
/// JSON-RPC's implementation-defined range (`-32000`..`-32099`) and is the code
/// the reference implementations use. See the module comment.
mod code {
    /// "Invalid JSON was received by the server."
    pub const PARSE_ERROR: i64 = -32700;
    /// "The JSON sent is not a valid Request object."
    pub const INVALID_REQUEST: i64 = -32600;
    /// "The method does not exist / is not available."
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// "Invalid method parameter(s)."
    pub const INVALID_PARAMS: i64 = -32602;
    /// SURE's own: a request that arrived before the handshake.
    pub const NOT_INITIALIZED: i64 = -32002;
}

/// The sentence that keeps the policy rule visible at the moment somebody tries
/// to break it.
///
/// Written once and used in full, rather than summarized at each call site: a
/// caller that sent `"execution_mode": "unrestricted"` needs to read why the
/// answer is no, and a paraphrase that lost `the user's own configuration`
/// would be a refusal that sounds like a bug.
const POLICY: &str = "Execution mode, privacy settings and protection policy come from the user's \
                      own SURE configuration on this machine. No tool argument can select or \
                      override them.";

/// Run `sure mcp`.
///
/// One arm, because there is one thing `sure mcp` can be asked to do: a bare
/// `sure mcp` is a wrong command line, refused by the grammar before this is
/// reached.
///
/// `store` is the store directory the caller named on the command line, or
/// `None` for the platform's own per-user location. A session is long-lived and
/// answers many tool calls, so the location is resolved once, here, and every
/// command in the session goes through the dispatch with that one value: a
/// bridge that let one tool call read a store and the next write another would
/// be answering about two different histories.
pub fn run(action: &McpAction, store: Option<&Path>) -> Report {
    match action {
        McpAction::Serve => serve(store),
    }
}

/// Speak the protocol on this process's standard input and output until the
/// caller closes them.
///
/// # Why this returns a report rather than writing one
///
/// The session summary is a [`Report::McpSession`] and it is written by
/// `main.rs` like every other report, through the format the user chose. That
/// is deliberate, and the summary does not travel on the protocol channel in
/// either format: [`Format::emit`](crate::output::Format::emit) sends a
/// [`Report::McpSession`] to stderr whether the caller asked for `--format
/// json` or for the default, because the specification's transport page says
/// the server "MUST NOT write anything to its `stdout` that is not a valid MCP
/// message" at revision `2025-11-25`. So `sure --format json mcp serve` is a
/// command line a caller may use: its stdout is a protocol stream and nothing
/// else, and the envelope saying how the session went is on stderr with the
/// rest of the diagnostics.
///
/// The alternative — writing the summary here — is not available: this module
/// cannot name a stream (see the module comment) and it cannot see `--format`
/// either, because [`Command::report`](crate::commands::Command::report) is
/// given nothing but the command. Neither is needed: the one decision (a
/// session summary is not a result) is made once, in `output.rs`, for both
/// formats.
fn serve(store: Option<&Path>) -> Report {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    match converse(&mut input, store) {
        Ok(session) => Report::McpSession(Box::new(session.summary())),
        // The only other way a session ends. Not a refusal and not a closed
        // pipe the user caused: the stream itself failed, which is why this is
        // a failure with SURE's own sentence and the operating system's detail.
        Err(error) => Report::Failed(Box::new(Failed {
            command: "mcp",
            what: "The session with the caller ended because its stream could not be read or \
                   written, so SURE stopped answering.",
            detail: error.to_string(),
        })),
    }
}

/// Read messages, decide, write what was decided.
fn converse(input: &mut impl BufRead, store: Option<&Path>) -> io::Result<Session> {
    let mut session = Session::at(store);
    loop {
        match read_message(input)? {
            Incoming::End => return Ok(session),
            Incoming::Line(line) => {
                if let Some(message) = session.handle(&line) {
                    session.answer(message)?;
                }
            }
            // A refused line is still a request with an identifier SURE could
            // not read, so the answer carries a null one — which is what the
            // specification says to do when the identifier could not be read.
            Incoming::TooLong => {
                let message = Session::parse_refusal(&format!(
                    "this message is longer than the {MAX_MESSAGE_BYTES} bytes SURE will hold in \
                     memory."
                ));
                session.answer(message)?;
            }
        }
    }
}

/// One message read from the caller's stream.
enum Incoming {
    /// One line, within the size bound, newline and all.
    Line(Vec<u8>),
    /// A line longer than [`MAX_MESSAGE_BYTES`], whose remainder has been
    /// discarded. The caller is answered with a parse error and the session
    /// goes on.
    TooLong,
    /// The caller closed its side. This is how a stdio session is supposed to
    /// end: the specification says the client closes the input stream first.
    End,
}

/// Read one newline-delimited message.
///
/// The read is bounded before it is made, rather than after: `take` limits what
/// is pulled off the stream, so a caller cannot make SURE allocate an unbounded
/// buffer by sending a line with no newline in it.
fn read_message(input: &mut impl BufRead) -> io::Result<Incoming> {
    let mut line = Vec::new();
    // The reborrow is explicit: `input.take(...)` would move the reference
    // itself, and `input` is needed again below to discard an over-long line.
    (&mut *input)
        .take(MAX_MESSAGE_BYTES.saturating_add(2) as u64)
        .read_until(b'\n', &mut line)?;
    if line.is_empty() {
        return Ok(Incoming::End);
    }
    // Two bytes of headroom over the bound, so that the two cases below can be
    // told apart: a line that ended inside the bound, and a line that was still
    // going when the bound ran out.
    let complete = line.last() == Some(&b'\n');
    let content = if complete {
        line.len().saturating_sub(1)
    } else {
        line.len()
    };
    if content > MAX_MESSAGE_BYTES {
        if !complete {
            drain_to_newline(input)?;
        }
        return Ok(Incoming::TooLong);
    }
    Ok(Incoming::Line(line))
}

/// Throw away the rest of an over-long line, up to [`MAX_DISCARDED_BYTES`].
///
/// Chunk by chunk, through the reader's own buffer, so that discarding a
/// hundred megabytes costs a hundred megabytes of time and none of memory.
fn drain_to_newline(input: &mut impl BufRead) -> io::Result<()> {
    let mut discarded = 0usize;
    loop {
        // The borrow `fill_buf` takes has to end before `consume` can take the
        // other one, which is why the decision is made in here and acted on
        // below rather than the other way round.
        let (taken, saw_newline) = {
            let available = input.fill_buf()?;
            match available.iter().position(|byte| *byte == b'\n') {
                Some(offset) => (offset.saturating_add(1), true),
                None => (available.len(), false),
            }
        };
        if taken == 0 {
            // The stream ended inside the line it was discarding. There is
            // nothing left to re-synchronise with, and the caller is gone.
            return Ok(());
        }
        input.consume(taken);
        if saw_newline {
            return Ok(());
        }
        discarded = discarded.saturating_add(taken);
        if discarded > MAX_DISCARDED_BYTES {
            return Err(io::Error::other(
                "the caller sent a line with no newline in it longer than SURE will discard, so \
                 there is no point in the stream where reading could start again",
            ));
        }
    }
}

/// What one session has done so far.
///
/// The handshake state and the counts, and nothing that decides anything: every
/// decision is a method, so that the state a summary describes is only ever
/// changed in one place per fact.
#[derive(Debug, Default)]
struct Session {
    /// The store every command in this session runs against — the directory the
    /// caller named on the command line, or `None` for the platform's own
    /// location.
    ///
    /// Here rather than looked up per tool call so that the whole session is
    /// about one store, and so that the only value it can hold is one the
    /// caller's own argument vector carried: a tool call cannot name a store,
    /// and neither can the project it names.
    store: Option<PathBuf>,
    /// Whether `initialize` has been answered. Set when the answer is built,
    /// which is what makes the rest of the session reachable.
    initialized: bool,
    /// The revision the caller asked for, escaped and bounded, whether or not
    /// SURE agreed to it.
    requested_protocol_version: Option<String>,
    /// The revision SURE answered with.
    agreed_protocol_version: Option<String>,
    /// Requests answered, with a result or with an error object.
    answered: u64,
    /// Notifications received. Answered with nothing at all, ever.
    notifications: u64,
    /// `tools/call` requests received, including the ones that were refused.
    tool_calls: u64,
    /// Messages answered with an error.
    errors: u64,
}

impl Session {
    /// A session whose commands run against the store the caller named, if they
    /// named one.
    ///
    /// A constructor rather than a struct literal at the call site so that the
    /// handshake's counters and this are set in one place, and so that a test
    /// can start a session with a store of its own.
    fn at(store: Option<&Path>) -> Self {
        Self {
            store: store.map(Path::to_path_buf),
            ..Self::default()
        }
    }

    /// The summary a finished session reports.
    fn summary(&self) -> McpSession {
        McpSession {
            initialized: self.initialized,
            requested_protocol_version: self.requested_protocol_version.clone(),
            agreed_protocol_version: self.agreed_protocol_version.clone(),
            answered: self.answered,
            notifications: self.notifications,
            tool_calls: self.tool_calls,
            errors: self.errors,
        }
    }

    /// Write one message and count it.
    ///
    /// The counting is here rather than at the call sites so that "answered"
    /// and "errors" are one fact each about one write, and so that both use
    /// [`McpMessage::is_error`] — the same predicate the exit status and the
    /// outcome are read from, which is what keeps a summary from saying a
    /// refusal was an answer.
    fn answer(&mut self, message: Value) -> io::Result<()> {
        let message = McpMessage { message };
        self.answered = self.answered.saturating_add(1);
        if message.is_error() {
            self.errors = self.errors.saturating_add(1);
        }
        Format::Json.emit(&Report::Mcp(Box::new(message)))
    }

    /// What SURE says about one line from the caller.
    ///
    /// `None` means nothing is written: the frame was a notification, and a
    /// notification is never answered. It does **not** mean nothing happened —
    /// the counters were updated, and a frame that is a notification is
    /// deliberately *not dispatched*, so a `tools/call` that lost its
    /// identifier cannot run anything.
    fn handle(&mut self, line: &[u8]) -> Option<Value> {
        let Ok(frame) = serde_json::from_slice::<Value>(line) else {
            return Some(Self::parse_refusal("this is not JSON."));
        };
        let Some(object) = frame.as_object() else {
            // An array (a JSON-RPC batch) lands here too. MCP has no batches,
            // and SURE answers one object per message.
            return Some(Self::parse_refusal(
                "a message must be one JSON object, and SURE does not accept a batch.",
            ));
        };

        // The identifier decides what kind of message this is, so it is read
        // first. No `id` at all is a notification; a null one is a request the
        // specification says cannot exist.
        let id = match object.get("id") {
            None => {
                self.notifications = self.notifications.saturating_add(1);
                return None;
            }
            Some(value) if value.is_string() || value.is_number() => value.clone(),
            Some(Value::Null) => {
                return Self::refuse(
                    &Value::Null,
                    code::INVALID_REQUEST,
                    "A request identifier must not be null (MCP 2025-11-25, basic protocol). SURE \
                     answered with a null identifier because it could not read this one.",
                );
            }
            Some(_) => {
                return Self::refuse(
                    &Value::Null,
                    code::INVALID_REQUEST,
                    "A request identifier must be a string or a number, so SURE could not read \
                     this one and answered with a null identifier.",
                );
            }
        };

        if object.get("jsonrpc").and_then(Value::as_str) != Some(JSONRPC) {
            return Self::refuse(
                &id,
                code::INVALID_REQUEST,
                format!("Every message must carry \"jsonrpc\": \"{JSONRPC}\"."),
            );
        }
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return Self::refuse(
                &id,
                code::INVALID_REQUEST,
                "A request must name a method. SURE never sends a request, so nothing here can be \
                 a reply to one.",
            );
        };
        // A notification that arrived with an identifier is a client that
        // cannot be answered either way: answering it breaks the rule, and
        // treating it as a request would mean a method SURE does not have.
        // Refusing says which of the two the caller got wrong.
        if method.starts_with("notifications/") {
            return Self::refuse(
                &id,
                code::INVALID_REQUEST,
                format!(
                    "\"{}\" is a notification, and a notification must not carry an identifier.",
                    quoted(method)
                ),
            );
        }

        // The initialization phase must be first. `ping` is allowed before it
        // because it is the liveness check a caller makes while it is deciding
        // whether the process it launched is alive at all.
        if !self.initialized && method != "initialize" && method != "ping" {
            return Self::refuse(
                &id,
                code::NOT_INITIALIZED,
                "This session has not been initialized. The first request must be `initialize`.",
            );
        }

        let params = object.get("params").filter(|value| !value.is_null());
        if params.is_some_and(|value| !value.is_object()) {
            return Self::refuse(
                &id,
                code::INVALID_PARAMS,
                "`params` must be one JSON object.",
            );
        }
        let params = params.and_then(Value::as_object);

        match method {
            "initialize" => self.initialize(&id, params),
            "ping" => Self::reply(&id, json!({})),
            "tools/list" => self.list_tools(&id, params),
            "tools/call" => self.call_tool(&id, params),
            _ => Self::refuse(
                &id,
                code::METHOD_NOT_FOUND,
                format!("SURE has no method named \"{}\".", quoted(method)),
            ),
        }
    }

    /// The handshake, which is also the version negotiation.
    fn initialize(&mut self, id: &Value, params: Option<&Map<String, Value>>) -> Option<Value> {
        if self.initialized {
            return Self::refuse(
                id,
                code::INVALID_REQUEST,
                "This session is already initialized. SURE answers `initialize` once, and it \
                 does not renegotiate mid-session.",
            );
        }
        let Some(version) = params
            .and_then(|params| params.get("protocolVersion"))
            .and_then(Value::as_str)
        else {
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                "`initialize` must carry params.protocolVersion, as a string.",
            );
        };
        if version.is_empty()
            || version.len() > MAX_PROTOCOL_VERSION_BYTES
            || version.chars().any(char::is_control)
        {
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                format!(
                    "`protocolVersion` must be a short printable string such as \"{PROTOCOL_VERSION}\"; \
                     this one is {} bytes.",
                    version.len()
                ),
            );
        }

        self.requested_protocol_version = Some(version.to_owned());
        // The negotiation rule, collapsed to this build: SURE supports one
        // revision, so a caller asking for it is answered with it, and a caller
        // asking for anything else is answered with the latest revision SURE
        // supports — which is also it. Answering with the caller's version
        // instead would be the lie the rule exists to prevent.
        self.agreed_protocol_version = Some(PROTOCOL_VERSION.to_owned());
        self.initialized = true;
        Self::reply(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                // No `listChanged`: the tool list is a constant of this build
                // and SURE sends no notifications, so a client that believed it
                // would wait for one that is never coming.
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")},
                "instructions": instructions(),
            }),
        )
    }

    /// The five tools, and the fact that the list is one page long.
    fn list_tools(&mut self, id: &Value, params: Option<&Map<String, Value>>) -> Option<Value> {
        if let Some(cursor) = params.and_then(|params| params.get("cursor"))
            && !cursor.is_null()
        {
            // Refused rather than ignored. SURE sends one page and therefore
            // never a `nextCursor`; a caller that followed one would be
            // following something SURE never said.
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                "SURE lists its tools in one page and never sends a nextCursor, so it accepts no \
                 cursor.",
            );
        }
        let tools: Vec<Value> = tool_set().iter().map(Tool::schema).collect();
        Self::reply(id, json!({ "tools": tools }))
    }

    /// One tool call: name it, name a project if it takes one, and nothing else.
    fn call_tool(&mut self, id: &Value, params: Option<&Map<String, Value>>) -> Option<Value> {
        self.tool_calls = self.tool_calls.saturating_add(1);

        let Some(params) = params else {
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                "`tools/call` must carry params with a tool `name`.",
            );
        };
        if let Some(key) = unexpected_key(params, &["name", "arguments"]) {
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                format!(
                    "\"{}\" is not something a `tools/call` may carry. A tool call carries a tool \
                     name and, for the tools that take one, a project. {POLICY}",
                    quoted(key)
                ),
            );
        }
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                "`tools/call` must carry params.name, as a string.",
            );
        };
        let Some(tool) = tool_set().iter().find(|tool| tool.name == name).copied() else {
            // A protocol error, with the message the specification's own
            // example uses. Not a tool result: there is no tool to have run.
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                format!("Unknown tool: {}", quoted(name)),
            );
        };

        let arguments = match params.get("arguments") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(arguments)) => arguments.clone(),
            Some(_) => {
                return Self::refuse(
                    id,
                    code::INVALID_PARAMS,
                    "`arguments` must be one JSON object.",
                );
            }
        };
        for key in arguments.keys() {
            if key == "project" {
                if tool.takes_project {
                    continue;
                }
                // Refused rather than dropped. A caller that sent a project is
                // describing a project, and answering about a different one
                // without saying so is the whole shape of a false green.
                return Self::refuse(
                    id,
                    code::INVALID_PARAMS,
                    format!(
                        "The `{}` tool runs `{}`, which takes no project and no arguments, so \
                         SURE will not accept one it would ignore.",
                        tool.name, tool.routes_to
                    ),
                );
            }
            if key == "_meta" {
                // Reserved by the protocol for extensions, so it is the one
                // key that is ignored rather than refused.
                continue;
            }
            return Self::refuse(
                id,
                code::INVALID_PARAMS,
                format!(
                    "\"{}\" is not an argument SURE accepts. A tool call may name a project and \
                     nothing else: {POLICY}",
                    quoted(key)
                ),
            );
        }
        let project = match arguments.get("project") {
            None => None,
            Some(Value::String(path)) => Some(PathBuf::from(path)),
            Some(_) => {
                return Self::refuse(
                    id,
                    code::INVALID_PARAMS,
                    "The `project` argument must be a string naming a directory.",
                );
            }
        };

        // The one line that does any work, and it does none of its own: the
        // command the caller's request corresponds to goes through the same
        // dispatch a person's command line goes through, with the store this
        // session was started with and no way for the request to change it.
        let report = (tool.command)(project).report(self.store.as_deref());
        Self::reply(id, tool_result(&tool, &report))
    }

    /// A result message.
    fn reply(id: &Value, result: Value) -> Option<Value> {
        Some(json!({"jsonrpc": JSONRPC, "id": id, "result": result}))
    }

    /// An error message.
    fn refuse(id: &Value, code: i64, message: impl Into<String>) -> Option<Value> {
        Some(json!({
            "jsonrpc": JSONRPC,
            "id": id,
            "error": {"code": code, "message": message.into()},
        }))
    }

    /// An error message about a line whose identifier could not be read.
    ///
    /// The identifier is null because there is nothing to echo, which is what
    /// the specification asks for in exactly this case.
    fn parse_refusal(why: &str) -> Value {
        json!({
            "jsonrpc": JSONRPC,
            "id": Value::Null,
            "error": {"code": code::PARSE_ERROR, "message": format!("Parse error: {why}")},
        })
    }
}

/// What a tool call answers.
///
/// Three fields, and one deliberate deviation from the specification, which
/// says a tool that returns structured content **should** also return the
/// serialized JSON in a `TextContent` block:
///
/// * `content[0].text` is [`Report::human_text`] — the words the command itself
///   would have printed. Those words are the answer, and they are what a model
///   reads. A second copy of the same answer as JSON would be a second
///   rendering of one fact, and the two would be free to drift; worse, a model
///   reading a JSON blob inside a text block learns to read the blob, and the
///   blob is where the envelope lives rather than where the sentences do.
/// * `structuredContent` is where a program looks: the command's own
///   [`Report::frame`], unmodified, exactly what `sure … --format json` would
///   have printed, plus which tool ran and which command line it ran. The
///   `SHOULD` is written for clients that predate `structuredContent`, and any
///   client that can complete a `2025-11-25` handshake postdates it by several
///   revisions.
/// * `isError` is `!Report::is_an_answer` — whether the *command* answered at
///   all, not what its answer was. A `sure doctor` that found something wrong
///   answered, so this is false and the verdict is in the frame, under
///   `outcome` and `exit_code`, in the CLI's own vocabulary. A command this
///   build cannot carry out did not answer, so this is true and the text is
///   that command's refusal. The one thing it must never do is turn "SURE
///   cannot check anything yet" into something an agent reads as success.
fn tool_result(tool: &Tool, report: &Report) -> Value {
    let mut structured = json!({
        "tool": tool.name,
        "routes_to": tool.routes_to,
        // The command's frame, which is where a program reads the outcome and
        // the status. Note what the status means here: it is what *that command
        // line* returns, not the status this server process exits with. A
        // session that answers twenty refusals and then sees its caller close
        // the stream is a session that did what it says it does.
        "sure": report.frame(),
    });
    if let Some(extra) = tool.extra
        && let Some(object) = structured.as_object_mut()
    {
        object.insert("mcp".to_owned(), extra());
    }
    json!({
        "content": [{"type": "text", "text": report.human_text()}],
        "structuredContent": structured,
        "isError": !report.is_an_answer(),
    })
}

/// One tool, as `tools/list` describes it and as `tools/call` runs it.
#[derive(Clone, Copy)]
struct Tool {
    /// The name a caller uses.
    name: &'static str,
    /// The name a person reads in a harness's interface.
    title: &'static str,
    /// What the tool answers, as the first sentence of its description.
    does: &'static str,
    /// The command name, for the check against what this build implements.
    runs: &'static str,
    /// The command line this tool runs, as a person would type it.
    routes_to: &'static str,
    /// Whether the tool accepts a `project` argument.
    takes_project: bool,
    /// The command this tool runs.
    command: fn(Option<PathBuf>) -> Command,
    /// Facts about the bridge itself, for the one tool that reports on SURE.
    extra: Option<fn() -> Value>,
}

impl Tool {
    /// The tool as `tools/list` carries it.
    fn schema(&self) -> Value {
        json!({
            "name": self.name,
            "title": self.title,
            "description": self.description(),
            "inputSchema": self.input_schema(),
        })
    }

    /// The description, which says what the tool does in this build.
    ///
    /// Whether the command behind the tool is implemented is read out of
    /// [`crate::commands::IMPLEMENTED`] rather than written here, so the day
    /// `sure check` runs, the sentence that says it does not disappears by
    /// itself. A description that promised a check this build cannot make would
    /// be an agent told to expect a verdict that never arrives.
    fn description(&self) -> String {
        let mut text = format!(
            "{}. It runs `{}` — the same command a person types — and answers with that \
             command's own words and that command's own frame.",
            self.does, self.routes_to
        );
        if !crate::commands::IMPLEMENTED.contains(&self.runs) {
            text.push_str(&format!(
                " This build does not implement `sure {}` yet, so every call answers with `sure \
                 {}`'s refusal, in its own words, marked as an error: never a result, never an \
                 empty success, and never a verdict about the project.",
                self.runs, self.runs
            ));
        }
        text
    }

    /// The input schema: a project, or nothing at all.
    ///
    /// Closed (`additionalProperties: false`) in both cases, because a schema
    /// that admits a key the server refuses is a schema that teaches a caller
    /// to send it. No tool in this list has a property that could select an
    /// execution mode, a privacy setting or a protection policy; the refusal in
    /// [`Session::call_tool`] is the second line of that defence, for a caller
    /// that never read the schema.
    fn input_schema(&self) -> Value {
        if !self.takes_project {
            return json!({"type": "object", "additionalProperties": false});
        }
        json!({
            "type": "object",
            "properties": {
                "project": {
                    "type": "string",
                    "description": "The project to act on. Default: the directory `sure mcp serve` \
                                    was started in. Passed to the command unchanged, so a path \
                                    the command line would refuse is refused here in the same \
                                    words.",
                },
            },
            "additionalProperties": false,
        })
    }
}

/// The five tools `docs/architecture/MCP_BRIDGE.md` names, in its order.
fn tool_set() -> [Tool; 5] {
    [
        Tool {
            name: "sure_check",
            title: "Check a project",
            does: "Check a project and report what was found",
            runs: "check",
            routes_to: "sure check",
            takes_project: true,
            command: |project| Command::Check {
                path: project,
                // Deliberately not exposed. `--goal` is a channel only the user
                // can write, and a goal a caller could set is a goal the
                // project's own author could set — which is exactly what
                // `cli.rs` says the flag exists to avoid.
                goal: None,
            },
            extra: None,
        },
        Tool {
            name: "sure_get_report",
            title: "Show what SURE has recorded",
            does: "Show what SURE has recorded on this machine",
            runs: "history",
            routes_to: "sure history list",
            // No project: the command behind this tool takes no argument, and
            // accepting one to ignore it would be a promise SURE does not keep.
            takes_project: false,
            command: |_| Command::History {
                action: Some(HistoryAction::List {
                    limit: crate::history::DEFAULT_LIMIT,
                }),
            },
            extra: None,
        },
        Tool {
            name: "sure_get_repair",
            title: "Turn findings into repair instructions",
            does: "Turn what was found into instructions a coding agent can act on",
            runs: "repair",
            routes_to: "sure repair",
            takes_project: true,
            command: |project| Command::Repair { path: project },
            extra: None,
        },
        Tool {
            name: "sure_recheck",
            title: "Check a project again",
            does: "Check a project again, after a repair, and compare what it finds with last time",
            runs: "recheck",
            routes_to: "sure recheck",
            takes_project: true,
            command: |project| Command::Recheck { path: project },
            extra: None,
        },
        Tool {
            name: "sure_status",
            title: "Report on this SURE build",
            does: "Report on this SURE build and this machine: the version, the protocol it \
                   speaks, and what it can actually do",
            runs: "doctor",
            routes_to: "sure doctor",
            takes_project: false,
            command: |_| Command::Doctor,
            extra: Some(status_facts),
        },
    ]
}

/// What SURE can say about itself, over and above what `sure doctor` says.
///
/// The same rule as `sure doctor`: every fact here is about this build, and
/// none of them is a claim about a project. Which commands are implemented is
/// read from [`crate::commands::IMPLEMENTED`], so this cannot drift from what
/// the dispatch actually does.
fn status_facts() -> Value {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "server": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")},
        "build": sure_core::version_string(),
        "commands_implemented": crate::commands::IMPLEMENTED,
        "tools": tool_set().iter().map(|tool| tool.name).collect::<Vec<_>>(),
    })
}

/// What the handshake tells a caller about this server.
///
/// One paragraph, and it says what this build can do rather than what SURE is
/// for. An agent that reads "SURE checks projects" and calls `sure_check` will
/// get a refusal, and this is where it should have found that out.
fn instructions() -> String {
    format!(
        "SURE — Software Understanding & Reality Evaluation. AI says it's done. Be SURE. This \
         build (version {}) speaks Model Context Protocol revision {PROTOCOL_VERSION} and \
         implements these commands: {}. Every tool answers with the words of the command it runs, \
         so a tool whose command this build cannot carry out answers with that command's refusal \
         rather than with a verdict, and marks the result as an error. SURE never listens on a \
         network; this process speaks to its caller and to nothing else.",
        env!("CARGO_PKG_VERSION"),
        crate::commands::IMPLEMENTED.join(", ")
    )
}

/// The first key of `object` that SURE does not accept.
///
/// `_meta` is skipped everywhere: the protocol reserves it for extensions, so
/// it is the one key SURE ignores rather than refuses.
fn unexpected_key<'a>(object: &'a Map<String, Value>, accepted: &[&str]) -> Option<&'a str> {
    object
        .keys()
        .map(String::as_str)
        .find(|key| *key != "_meta" && !accepted.contains(key))
}

/// A caller-supplied word, safe to put inside one of SURE's sentences.
///
/// Escaped and bounded, because this text is the caller's: a method name with a
/// newline in it would otherwise let a caller write a second line into an error
/// message, and one with a megabyte in it would let a caller choose how much
/// memory SURE spends answering. `sure_core::redact` already renders control
/// characters as escapes for exactly this reason; this adds the bound.
fn quoted(word: &str) -> String {
    let mut kept = String::new();
    let mut rest = word.chars();
    for character in rest.by_ref().take(MAX_QUOTED_BYTES) {
        kept.push(character);
    }
    let escaped = sure_core::redact::escape_control_characters(&kept);
    if rest.next().is_some() {
        format!("{escaped}…")
    } else {
        escaped
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// One line in, one message out.
    fn ask(session: &mut Session, frame: Value) -> Option<Value> {
        session.handle(frame.to_string().as_bytes())
    }

    /// A session that has completed the handshake, writing to a store of its
    /// own.
    ///
    /// The store matters here even though this is a unit test of the bridge: a
    /// tool call is a real `Command::report`, and `sure_check` over a project
    /// writes what it found to the store. Reading or writing the store on the
    /// machine running the suite is the defect `--store-dir` exists to remove,
    /// so every session here names one, under the workspace's git-ignored
    /// `target/tmp`. Made unique by `create_dir` rather than by the name, so
    /// that two sessions in one process cannot share a store and see each
    /// other's records.
    fn a_store_of_our_own() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("sure mcp");
        std::fs::create_dir_all(&base)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
        for _ in 0..1_000 {
            let candidate = base.join(format!("store-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
            match std::fs::create_dir(&candidate) {
                Ok(()) => return candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create {}: {error}", candidate.display()),
            }
        }
        panic!("no free store directory under {}", base.display());
    }

    /// A session that has completed the handshake.
    fn started() -> Session {
        let mut session = Session::at(Some(&a_store_of_our_own()));
        let answer = ask(
            &mut session,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": PROTOCOL_VERSION},
            }),
        );
        assert!(answer.is_some_and(|message| message.get("error").is_none()));
        session
    }

    fn call(session: &mut Session, name: &str, arguments: Value) -> Value {
        ask(
            session,
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments},
            }),
        )
        .expect("a tool call is answered")
    }

    fn error_code(message: &Value) -> i64 {
        message["error"]["code"]
            .as_i64()
            .expect("an error object carries a number")
    }

    fn tools(session: &mut Session) -> Vec<Value> {
        let message = ask(
            session,
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        )
        .expect("tools/list is answered");
        message["result"]["tools"]
            .as_array()
            .expect("a result carries a list")
            .clone()
    }

    #[test]
    fn the_handshake_names_the_revision_this_build_speaks() {
        let mut session = Session::default();
        let message = ask(
            &mut session,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": {}},
            }),
        )
        .expect("initialize is answered");
        assert_eq!(message["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(message["result"]["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(
            message["result"]["capabilities"]["tools"]["listChanged"],
            false
        );
        assert!(session.initialized);
        assert_eq!(
            session.summary().agreed_protocol_version.as_deref(),
            Some(PROTOCOL_VERSION)
        );
    }

    #[test]
    fn a_caller_that_asks_for_another_revision_is_answered_with_sures_own() {
        // The negotiation rule, in the one form this build can be in: SURE
        // supports one revision, so it always answers with that one — and it
        // remembers what it was asked for, because "which version did it want"
        // is the first question a person has when the two sides disagree.
        let mut session = Session::default();
        let message = ask(
            &mut session,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": "1999-01-01"},
            }),
        )
        .expect("initialize is answered");
        assert_eq!(message["result"]["protocolVersion"], PROTOCOL_VERSION);
        let summary = session.summary();
        assert_eq!(
            summary.requested_protocol_version.as_deref(),
            Some("1999-01-01")
        );
        assert_eq!(
            summary.agreed_protocol_version.as_deref(),
            Some(PROTOCOL_VERSION)
        );
    }

    #[test]
    fn a_request_before_the_handshake_is_refused_rather_than_answered() {
        let mut session = Session::default();
        let message = ask(
            &mut session,
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/list"}),
        )
        .expect("a request is answered");
        assert_eq!(error_code(&message), code::NOT_INITIALIZED);
        assert_eq!(message["id"], 4);
    }

    #[test]
    fn a_second_handshake_is_refused() {
        let mut session = started();
        let message = ask(
            &mut session,
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "initialize",
                "params": {"protocolVersion": PROTOCOL_VERSION},
            }),
        )
        .expect("initialize is answered");
        assert_eq!(error_code(&message), code::INVALID_REQUEST);
        assert_eq!(
            session.summary().requested_protocol_version.as_deref(),
            Some(PROTOCOL_VERSION)
        );
    }

    #[test]
    fn a_protocol_version_that_is_not_a_short_printable_string_is_refused() {
        for params in [
            json!({}),
            json!({"protocolVersion": 20251125}),
            json!({"protocolVersion": ""}),
            json!({"protocolVersion": "x".repeat(MAX_PROTOCOL_VERSION_BYTES + 1)}),
            json!({"protocolVersion": "2025-11-25\nnot a version"}),
        ] {
            let mut session = Session::default();
            let message = ask(
                &mut session,
                json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": params}),
            )
            .expect("initialize is answered");
            assert_eq!(error_code(&message), code::INVALID_PARAMS, "{params}");
            assert!(!session.initialized, "{params}");
        }
    }

    #[test]
    fn every_tool_the_bridge_documents_is_listed_with_a_closed_schema() {
        let mut session = started();
        let listed = tools(&mut session);
        let names: Vec<&str> = listed
            .iter()
            .map(|tool| tool["name"].as_str().expect("a tool has a name"))
            .collect();
        assert_eq!(
            names,
            [
                "sure_check",
                "sure_get_report",
                "sure_get_repair",
                "sure_recheck",
                "sure_status"
            ]
        );
        for tool in &listed {
            assert!(
                tool["description"]
                    .as_str()
                    .is_some_and(|text| !text.is_empty()),
                "{tool}"
            );
            assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
            assert_eq!(
                tool["inputSchema"]["additionalProperties"], false,
                "a schema admits a key SURE refuses: {tool}"
            );
            let properties = tool["inputSchema"]["properties"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let accepted: Vec<&str> = properties.keys().map(String::as_str).collect();
            // The bound on what a caller may say, checked against the schemas
            // rather than against the list they were written from.
            assert!(
                accepted.iter().all(|key| *key == "project"),
                "{tool} exposes an argument that is not a project: {accepted:?}"
            );
        }
    }

    #[test]
    fn a_tool_the_bridge_lists_runs_a_command_of_its_own() {
        // The list and the commands behind it are one fact each; this is the
        // check that no tool is listed with a command that does not exist on
        // this surface.
        for tool in tool_set() {
            assert_eq!((tool.command)(None).name(), tool.runs, "{}", tool.name);
        }
    }

    #[test]
    fn an_argument_that_is_not_a_project_is_refused_with_the_policy_rule() {
        let mut session = started();
        for (name, key) in [
            ("sure_check", "execution_mode"),
            ("sure_check", "privacy"),
            ("sure_check", "protection"),
            ("sure_status", "mode"),
        ] {
            let message = call(&mut session, name, json!({key: "anything"}));
            assert_eq!(error_code(&message), code::INVALID_PARAMS, "{name}/{key}");
            let text = message["error"]["message"]
                .as_str()
                .expect("an error message is a string");
            assert!(text.contains(POLICY), "{text}");
        }
    }

    #[test]
    fn a_project_on_a_tool_that_takes_none_is_refused_rather_than_ignored() {
        let mut session = started();
        let message = call(&mut session, "sure_get_report", json!({"project": "."}));
        assert_eq!(error_code(&message), code::INVALID_PARAMS);
        let text = message["error"]["message"]
            .as_str()
            .expect("an error message is a string");
        assert!(text.contains("sure history list"), "{text}");
    }

    #[test]
    fn a_project_that_is_not_a_string_is_refused() {
        let mut session = started();
        let message = call(&mut session, "sure_check", json!({"project": 7}));
        assert_eq!(error_code(&message), code::INVALID_PARAMS);
    }

    #[test]
    fn an_unknown_tool_is_a_protocol_error_naming_it() {
        let mut session = started();
        let message = call(&mut session, "sure_chekc", json!({}));
        assert_eq!(error_code(&message), code::INVALID_PARAMS);
        let text = message["error"]["message"]
            .as_str()
            .expect("an error message is a string");
        assert_eq!(text, "Unknown tool: sure_chekc");
    }

    #[test]
    fn an_unknown_method_is_refused_by_name() {
        let mut session = started();
        let message = ask(
            &mut session,
            json!({"jsonrpc": "2.0", "id": 3, "method": "resources/list"}),
        )
        .expect("a request is answered");
        assert_eq!(error_code(&message), code::METHOD_NOT_FOUND);
    }

    #[test]
    fn a_method_name_cannot_write_a_second_line_into_a_refusal() {
        let mut session = started();
        let message = ask(
            &mut session,
            json!({"jsonrpc": "2.0", "id": 3, "method": "bad\nmethod\nname"}),
        )
        .expect("a request is answered");
        let text = message["error"]["message"]
            .as_str()
            .expect("an error message is a string");
        assert!(!text.contains('\n'), "{text}");
        assert!(text.contains("bad\\nmethod"), "{text}");
    }

    #[test]
    fn a_line_that_is_not_json_is_a_parse_error_with_a_null_identifier() {
        let mut session = started();
        let message = session
            .handle(b"{ not json")
            .expect("a parse error is answered");
        assert_eq!(error_code(&message), code::PARSE_ERROR);
        assert!(message["id"].is_null());
        // And the session survives it: the next request is answered.
        assert!(
            ask(
                &mut session,
                json!({"jsonrpc": "2.0", "id": 1, "method": "ping"})
            )
            .is_some()
        );
    }

    #[test]
    fn a_batch_is_refused_rather_than_answered_one_by_one() {
        let mut session = started();
        let message = session
            .handle(br#"[{"jsonrpc":"2.0","id":1,"method":"ping"}]"#)
            .expect("a batch is answered");
        assert_eq!(error_code(&message), code::PARSE_ERROR);
    }

    #[test]
    fn a_notification_is_never_answered_and_never_dispatched() {
        // Both halves matter. The first is the protocol: a notification must
        // not be answered. The second is SURE's: a `tools/call` that lost its
        // identifier must not run anything, because a caller that gets no
        // answer could otherwise have had work done for it.
        let mut session = started();
        let before = session.summary().tool_calls;
        assert!(
            ask(
                &mut session,
                json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
            )
            .is_none()
        );
        assert!(
            ask(
                &mut session,
                json!({"jsonrpc": "2.0", "method": "tools/call",
                       "params": {"name": "sure_check", "arguments": {"project": "/"}}})
            )
            .is_none()
        );
        assert_eq!(session.summary().tool_calls, before);
        assert_eq!(session.summary().notifications, 2);
    }

    #[test]
    fn a_null_identifier_is_refused() {
        let mut session = started();
        let message = ask(
            &mut session,
            json!({"jsonrpc": "2.0", "id": null, "method": "ping"}),
        )
        .expect("a request with a null identifier is answered");
        assert_eq!(error_code(&message), code::INVALID_REQUEST);
        assert!(message["id"].is_null());
    }

    #[test]
    fn the_identifier_a_caller_sent_is_the_identifier_it_gets_back() {
        let mut session = started();
        for id in [json!(1), json!("abc"), json!(1.5)] {
            let message = ask(
                &mut session,
                json!({"jsonrpc": "2.0", "id": id, "method": "ping"}),
            )
            .expect("ping is answered");
            assert_eq!(&message["id"], &id);
        }
    }

    #[test]
    fn the_wrong_jsonrpc_version_is_refused() {
        let mut session = started();
        let message = ask(
            &mut session,
            json!({"jsonrpc": "1.0", "id": 1, "method": "ping"}),
        )
        .expect("a request is answered");
        assert_eq!(error_code(&message), code::INVALID_REQUEST);
    }

    #[test]
    fn a_cursor_is_refused_because_sure_never_sends_one() {
        let mut session = started();
        let message = ask(
            &mut session,
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {"cursor": "x"}}),
        )
        .expect("tools/list is answered");
        assert_eq!(error_code(&message), code::INVALID_PARAMS);
    }

    #[test]
    fn a_command_this_build_cannot_run_answers_as_a_tool_error_in_its_own_words() {
        // The heart of the honesty rule: a tool whose command this build cannot
        // carry out answers with a tool result marked `isError`, in the
        // command's own sentence — never a result, never an empty object.
        //
        // This used to be driven through `sure_get_report`, which ran `sure
        // history` and was the one tool left in that shape. `sure history` is
        // carried out now, so **no tool is**, and the rule is driven where it
        // actually lives: `tool_result`, the one function every tool's answer
        // goes through. Driving it with a refusal written here rather than
        // through a tool is honest about what is being checked — the rule, not
        // one tool's luck — and the test below ties every tool's command to
        // `commands::IMPLEMENTED`, so the day one of them goes back to refusing,
        // the two meet again.
        let tool = tool_set()
            .into_iter()
            .find(|tool| tool.name == "sure_get_report")
            .expect("the bridge lists the tool that reports on the history");
        let result = tool_result(
            &tool,
            &Report::Unavailable(crate::report::NotYet {
                command: "history export",
                does: "write the history out as JSON",
                instead: "Use `sure --format json history`.",
            }),
        );

        assert_eq!(result["isError"], true, "{result}");
        let text = result["content"][0]["text"]
            .as_str()
            .expect("a tool result carries text");
        assert!(text.contains("is not implemented in this build."), "{text}");
        assert!(
            text.contains("history export"),
            "the tool does not name the command it could not run: {text}"
        );
        assert_eq!(
            result["structuredContent"]["sure"]["outcome"],
            "unavailable"
        );
        assert_eq!(result["structuredContent"]["sure"]["exit_code"], 3);
        // The tool that was asked is still named, even though the command it ran
        // is not: a caller reading this has to be able to tell which of its calls
        // came back this way.
        assert_eq!(result["structuredContent"]["tool"], "sure_get_report");
        assert_eq!(
            result["structuredContent"]["routes_to"],
            "sure history list"
        );
    }

    #[test]
    fn a_tool_that_ran_carries_its_verdict_and_not_an_error() {
        // The other half of the honesty rule, and the half that matters now
        // that the check pipeline runs: a tool that answered is not an error,
        // and what an agent must not read as success is the *verdict* — which
        // is in the frame, not in `isError`. A `sure_check` that came back
        // clean in this build would be the false green told to a harness.
        // An absolute path, because SURE refuses a relative project root rather
        // than resolving it against wherever the process happens to be — and the
        // crate this test lives in is a real project with checks to plan. The
        // three tools are asked about *this* crate rather than about nothing,
        // which is what makes the answer a verdict rather than a refusal.
        let project = env!("CARGO_MANIFEST_DIR");
        let mut session = started();
        for (name, command) in [
            ("sure_check", "check"),
            ("sure_recheck", "recheck"),
            ("sure_get_repair", "repair"),
        ] {
            let message = call(&mut session, name, json!({"project": project}));
            let result = &message["result"];
            assert_eq!(result["isError"], false, "{name}: {message}");
            let sure = &result["structuredContent"]["sure"];
            // The name is the command's, not the tool's: a tool answers what the
            // command line answers, and a caller comparing the two must find them
            // the same.
            assert_eq!(sure["command"], command);
            assert_ne!(
                sure["outcome"], "ok",
                "{name} told a harness the project was clean: {message}"
            );
            assert_eq!(
                sure["details"]["green"], false,
                "{name} reported a run with un-run stages as green: {message}"
            );
        }
    }

    #[test]
    fn no_tool_result_reports_success_for_a_project_that_was_never_checked() {
        // The failure this program exists to prevent, in its MCP shape: an
        // agent reads a tool result and concludes the project is fine.
        //
        // `isError` is not the field that carries that any more — a check that
        // ran and found the project not clean is a successful *call*. What
        // carries it is `outcome`, which is the CLI's own vocabulary and can
        // never be `ok` for a run that did not establish a clean project. So
        // this asks the stronger question of every tool, including the ones
        // whose command answered: **does anything here say `ok`?**
        // `sure_status` is deliberately not in this list, and not because it is
        // exempt: it answers about this *build*, not about a project (`doctor`
        // may legitimately be `ok`), and it has a test of its own —
        // `sure_status_answers_about_this_build_and_never_about_a_project`.
        //
        // `sure_get_report` is not in it either now, and for `sure_status`'s
        // reason rather than by exemption: `sure history` is carried out in this
        // build, it answers about this *machine* rather than about a project, and
        // "there is nothing recorded" is a true answer that exits 0. Requiring it
        // to be non-`ok` would force it to lie about what it did. What it must
        // never do is read as a statement about a project, and that is what
        // `sure_get_report_never_answers_a_question_about_a_project` asks
        // instead.
        let mut session = started();
        for name in ["sure_check", "sure_recheck", "sure_get_repair"] {
            let message = call(&mut session, name, json!({}));
            let sure = &message["result"]["structuredContent"]["sure"];
            assert_ne!(
                sure["outcome"], "ok",
                "{name} reported success for work that did not happen: {message}"
            );
            assert_ne!(
                sure["exit_code"], 0,
                "{name} exited zero for work that did not happen: {message}"
            );
        }
    }

    #[test]
    fn sure_get_report_never_answers_a_question_about_a_project() {
        // The tool that reads the history is the one tool whose `ok` is honest —
        // it says what this machine has recorded, and it can be right. What
        // would not be honest is an agent reading that `ok` as a verdict about
        // the project it is working on. So the answer must carry a history and
        // no green: no `green` field, no stage, nothing a caller could mistake
        // for a check. Compared against the list of tools that do answer about a
        // project, which is derived from `takes_project` rather than retyped.
        let answering_about_projects: Vec<&str> = tool_set()
            .iter()
            .filter(|tool| tool.takes_project)
            .map(|tool| tool.name)
            .collect();

        let mut session = started();
        let message = call(&mut session, "sure_get_report", json!({}));
        let result = &message["result"];
        assert_eq!(result["isError"], false, "{message}");
        let sure = &result["structuredContent"]["sure"];
        assert_eq!(sure["command"], "history");
        assert_eq!(sure["outcome"], "ok");
        assert_eq!(sure["exit_code"], 0);
        assert!(
            sure["details"]["green"].is_null(),
            "the history answered with a verdict about a project: {message}"
        );
        assert!(
            !answering_about_projects.contains(&"sure_get_report"),
            "this test says the history never answers about a project, and the tool set \
             says it takes one: {answering_about_projects:?}"
        );
        // And the text a model reads says which machine it is about, so that a
        // model quoting it cannot drop the subject.
        let text = result["content"][0]["text"]
            .as_str()
            .expect("a tool result carries text");
        assert!(text.contains("this machine"), "{text}");
    }

    #[test]
    fn a_tool_result_carries_the_commands_own_frame_unmodified() {
        let mut session = started();
        let message = call(&mut session, "sure_check", json!({"project": "somewhere"}));
        let expected = Command::Check {
            path: Some(PathBuf::from("somewhere")),
            goal: None,
        }
        // The same store the session is using, because the frame is what the
        // command answered and a store the session named is part of that.
        .report(session.store.as_deref())
        .frame();
        assert_eq!(&message["result"]["structuredContent"]["sure"], &expected);
    }

    #[test]
    fn a_tool_whose_command_is_implemented_is_not_marked_as_an_error() {
        let mut session = started();
        let message = call(&mut session, "sure_status", json!({}));
        assert_eq!(message["result"]["isError"], false);
        assert_eq!(
            message["result"]["structuredContent"]["mcp"]["protocol_version"],
            PROTOCOL_VERSION
        );
        assert_eq!(
            message["result"]["structuredContent"]["sure"]["command"],
            "doctor"
        );
        assert_eq!(session.summary().errors, 0);
    }

    #[test]
    fn the_tools_take_their_shape_from_what_this_build_implements() {
        // Not a hardcoded sentence: the description and the instructions are
        // derived from `commands::IMPLEMENTED`, so the day one of these
        // commands runs, the text that says it does not stops saying it.
        for tool in tool_set() {
            let described = tool.description().contains("does not implement");
            assert_eq!(
                described,
                !crate::commands::IMPLEMENTED.contains(&tool.runs),
                "{}",
                tool.name
            );
        }
        let instructions = instructions();
        for command in crate::commands::IMPLEMENTED {
            assert!(instructions.contains(command), "{instructions}");
        }
    }

    #[test]
    fn every_tool_and_method_sure_answers_is_reachable_after_one_handshake() {
        // A session that has handshaken can use both request methods and all
        // five tools without a second handshake. What `answer` counts is not
        // visible from here — `handle` decides and writes nothing — so the
        // counts are checked over a pipe, in `tests/mcp_protocol.rs`.
        let mut session = started();
        let pong = ask(
            &mut session,
            json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
        )
        .expect("ping is answered");
        assert!(pong.get("error").is_none(), "{pong}");
        assert_eq!(tools(&mut session).len(), 5);
        assert_eq!(session.summary().notifications, 0);
        assert_eq!(session.summary().tool_calls, 0);
    }

    #[test]
    fn a_message_that_names_no_method_and_no_identifier_is_not_dispatched() {
        let mut session = started();
        assert!(ask(&mut session, json!({"jsonrpc": "2.0", "result": {}})).is_none());
        assert_eq!(session.summary().notifications, 1);
        assert_eq!(session.summary().tool_calls, 0);
    }
}

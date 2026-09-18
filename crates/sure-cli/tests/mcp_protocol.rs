//! The MCP bridge as a caller meets it: a process, a pipe, and JSON.
//!
//! # Why this file runs the binary
//!
//! `src/mcp.rs` decides what to answer and its tests check those decisions. The
//! claims this file checks are the ones only a process can make:
//!
//! 1. the handshake and the version negotiation happen on the wire;
//! 2. the protocol messages are the *only* thing on stdout, and the session
//!    summary is not one of them;
//! 3. every tool answers with what the command line behind it answers — the
//!    same frame for a program, the same words for a person — which is the
//!    check that has to survive the day those commands start working;
//! 4. no tool reports success for a project that was not checked;
//! 5. a caller cannot reach around the CLI to change the execution mode, the
//!    privacy settings or the protection policy;
//! 6. the process ends when its caller closes the stream, with the status the
//!    session report says it returns.
//!
//! The server is driven the way a harness drives it: the test writes
//! newline-delimited JSON to its standard input and reads lines back from its
//! standard output. Every read has a deadline, so a server that stopped
//! answering fails this test instead of hanging it.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use sure_cli::mcp::{MAX_MESSAGE_BYTES, PROTOCOL_VERSION};

/// The binary this package builds, as cargo hands it to its integration tests.
const SURE: &str = env!("CARGO_BIN_EXE_sure");

/// How long one answer may take before the test calls it a hang.
///
/// Generous: the point is to fail rather than block forever, not to measure
/// anything. A server that has said nothing within this is not slow, it is
/// stuck.
const DEADLINE: Duration = Duration::from_secs(30);

/// A live `sure mcp serve`, and the pipes the test talks to it on.
struct Session {
    child: Child,
    /// `None` once the caller has closed its side.
    input: Option<ChildStdin>,
    answers: Receiver<String>,
    diagnostics: Receiver<String>,
}

/// What a finished session left behind.
struct Finished {
    status: i32,
    /// Lines on stdout that no test read as an answer.
    unread: Vec<String>,
    stderr: String,
}

impl Session {
    fn open(args: &[&str]) -> Session {
        let mut child = Command::new(SURE)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("could not run {SURE}: {error}"));
        let input = child.stdin.take().expect("stdin is piped");
        let answers = lines(child.stdout.take().expect("stdout is piped"));
        let diagnostics = lines(child.stderr.take().expect("stderr is piped"));
        Session {
            child,
            input: Some(input),
            answers,
            diagnostics,
        }
    }

    /// A server with the default format, which is what a harness gets.
    fn serve() -> Session {
        Session::open(&["mcp", "serve"])
    }

    /// A server with the machine format, for the tests that read the envelope.
    fn serve_json() -> Session {
        Session::open(&["--format", "json", "mcp", "serve"])
    }

    fn send(&mut self, line: &str) {
        let mut bytes = line.as_bytes().to_vec();
        bytes.push(b'\n');
        self.send_bytes(&bytes);
    }

    fn send_bytes(&mut self, bytes: &[u8]) {
        let input = self.input.as_mut().expect("the caller has not closed yet");
        input
            .write_all(bytes)
            .and_then(|()| input.flush())
            .unwrap_or_else(|error| panic!("could not write to the server: {error}"));
    }

    /// The next line on stdout, as JSON.
    fn read(&mut self) -> Value {
        let line = self.read_line();
        serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("a line on stdout was not JSON ({error}): {line}"))
    }

    fn read_line(&mut self) -> String {
        match self.answers.recv_timeout(DEADLINE) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => {
                panic!("the server did not answer within {DEADLINE:?}")
            }
            Err(RecvTimeoutError::Disconnected) => {
                panic!("the server closed its output without answering")
            }
        }
    }

    /// Send one request and read its answer.
    fn ask(&mut self, request: Value) -> Value {
        self.send(&request.to_string());
        self.read()
    }

    /// The handshake, which every request after it depends on.
    fn start(&mut self) {
        let answer = self.ask(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": {}},
        }));
        assert!(answer.get("error").is_none(), "{answer}");
    }

    /// Close the caller's side and collect what the process did.
    fn finish(mut self) -> Finished {
        let _ = self.input.take();
        let status = self
            .child
            .wait()
            .expect("the server exited")
            .code()
            .expect("the server exited on its own");
        Finished {
            status,
            unread: drain(&self.answers),
            stderr: drain(&self.diagnostics).join("\n"),
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // A test that panicked leaves a server reading a stream nobody writes
        // to any more. Killing it here is what keeps a failure from leaving a
        // process behind; after `finish` the child is already gone and this is
        // a no-op.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Read lines from a pipe on a thread of its own.
///
/// The threads are what stop the two pipes from deadlocking the test: a server
/// that wrote more than a pipe buffer holds would block until somebody read it,
/// and the test is the only somebody there is.
fn lines(pipe: impl Read + Send + 'static) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(pipe).lines() {
            let Ok(line) = line else { return };
            if sender.send(line).is_err() {
                return;
            }
        }
    });
    receiver
}

/// Everything left in a channel once its pipe closed.
fn drain(channel: &Receiver<String>) -> Vec<String> {
    let mut collected = Vec::new();
    loop {
        match channel.recv_timeout(DEADLINE) {
            Ok(line) => collected.push(line),
            Err(RecvTimeoutError::Disconnected) => return collected,
            Err(RecvTimeoutError::Timeout) => {
                panic!("a stream stayed open after the server exited")
            }
        }
    }
}

/// What one run of the binary did.
struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Run {
    let output = Command::new(SURE)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("could not run {SURE}: {error}"));
    Run {
        status: output.status.code().expect("the process exited on its own"),
        stdout: String::from_utf8(output.stdout).expect("stdout is utf-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is utf-8"),
    }
}

/// The frame the command line prints for one command, as a value.
fn frame_of(args: &[&str]) -> Value {
    let mut with_format = vec!["--format", "json"];
    with_format.extend_from_slice(args);
    let run = run(&with_format);
    serde_json::from_str(&run.stdout).unwrap_or_else(|error| {
        panic!(
            "`sure {}` did not print one frame ({error}): {}",
            args.join(" "),
            run.stdout
        )
    })
}

/// A tool call, as a harness sends it.
fn call(id: u32, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments},
    })
}

fn result_of(answer: &Value) -> &Value {
    assert!(answer.get("error").is_none(), "{answer}");
    &answer["result"]
}

fn error_code(answer: &Value) -> i64 {
    answer["error"]["code"]
        .as_i64()
        .unwrap_or_else(|| panic!("this is not an error message: {answer}"))
}

fn text_of(result: &Value) -> &str {
    result["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("a tool result carries one text block: {result}"))
}

/// One tool, the command line behind it, and where that command's words go.
struct Route {
    tool: &'static str,
    /// The command line the tool's result says it runs.
    routes_to: &'static str,
    arguments: Value,
    /// The same command, as the test runs it.
    command: &'static [&'static str],
    /// Whether the command's human form is an answer (stdout) or a complaint
    /// (stderr). `Report::is_an_answer` is the same decision inside the binary.
    answers_on_stdout: bool,
}

/// The four tools whose command is a fixed function of their arguments.
///
/// `sure_status` is checked on its own, because the two sides run `sure doctor`
/// at different moments and a doctor report contains the state of this machine.
fn routes() -> [Route; 4] {
    [
        Route {
            tool: "sure_check",
            routes_to: "sure check",
            arguments: json!({"project": "somewhere"}),
            command: &["check", "somewhere"],
            answers_on_stdout: false,
        },
        Route {
            tool: "sure_recheck",
            routes_to: "sure recheck",
            arguments: json!({"project": "somewhere"}),
            command: &["recheck", "somewhere"],
            answers_on_stdout: false,
        },
        Route {
            tool: "sure_get_repair",
            routes_to: "sure repair",
            arguments: json!({"project": "somewhere"}),
            command: &["repair", "somewhere"],
            answers_on_stdout: false,
        },
        Route {
            tool: "sure_get_report",
            routes_to: "sure history list",
            arguments: json!({}),
            command: &["history", "list"],
            answers_on_stdout: false,
        },
    ]
}

#[test]
fn the_handshake_names_the_revision_this_build_speaks_and_what_it_can_do() {
    let mut session = Session::serve();
    let answer = session.ask(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "mcp_protocol.rs", "version": "0"},
        },
    }));
    let result = result_of(&answer).clone();
    assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(result["serverInfo"]["name"], "sure");
    assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
    // What this build can do, said in the handshake rather than discovered by
    // calling a tool and reading a refusal.
    let instructions = result["instructions"].as_str().expect("instructions");
    for command in sure_cli::commands::IMPLEMENTED {
        assert!(
            instructions.contains(command),
            "the handshake does not mention `sure {command}`: {instructions}"
        );
    }
    let finished = session.finish();
    assert_eq!(finished.status, 0, "{}", finished.stderr);
    assert!(finished.unread.is_empty(), "{:?}", finished.unread);
    assert!(
        finished.stderr.contains(PROTOCOL_VERSION),
        "{}",
        finished.stderr
    );
}

#[test]
fn a_caller_that_asks_for_another_revision_is_still_told_what_sure_speaks() {
    // The negotiation rule: SURE answers with a revision it supports, and the
    // caller decides from the answer whether it can go on.
    let mut session = Session::serve();
    let answer = session.ask(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"protocolVersion": "1999-01-01"},
    }));
    assert_eq!(result_of(&answer)["protocolVersion"], PROTOCOL_VERSION);
    // And the session remembers both versions, because "which one did it ask
    // for" is the first question a person has when two sides disagree.
    let finished = session.finish();
    assert_eq!(finished.status, 0);
    assert!(
        finished.stderr.contains("1999-01-01"),
        "{}",
        finished.stderr
    );
}

#[test]
fn tools_list_is_the_five_tools_the_bridge_documents_with_closed_schemas() {
    let mut session = Session::serve();
    session.start();
    let answer = session.ask(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let tools = result_of(&answer)["tools"]
        .as_array()
        .expect("a list of tools")
        .clone();
    let names: Vec<&str> = tools
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
    for tool in &tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert_eq!(tool["inputSchema"]["additionalProperties"], false, "{tool}");
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        for key in properties.keys() {
            assert_eq!(
                key, "project",
                "a tool exposes an argument that could select a policy: {tool}"
            );
        }
        // Every tool but `sure_status` runs a command this build refuses, and
        // says so before it is called.
        let described = tool["description"]
            .as_str()
            .is_some_and(|text| text.contains("does not implement"));
        assert_eq!(
            described,
            tool["name"] != "sure_status",
            "a tool's description disagrees with what its command does: {tool}"
        );
    }
    assert_eq!(session.finish().status, 0);
}

#[test]
fn every_tool_answers_what_the_command_line_behind_it_answers() {
    // The single-path test. Not a hardcoded sentence: the tool result is
    // compared with what this same binary prints for the command the tool
    // stands for, so the day `sure check` runs, this test still holds and the
    // assertions about refusals below are the only ones that have to change.
    let mut session = Session::serve();
    session.start();
    for (index, route) in routes().iter().enumerate() {
        let answer = session.ask(call(10 + index as u32, route.tool, route.arguments.clone()));
        let result = result_of(&answer).clone();
        let expected = frame_of(route.command);
        assert_eq!(
            &result["structuredContent"]["sure"],
            &expected,
            "{} does not answer with `sure {}`'s frame",
            route.tool,
            route.command.join(" ")
        );
        let human = run(route.command);
        let words: &str = if route.answers_on_stdout {
            &human.stdout
        } else {
            &human.stderr
        };
        assert_eq!(
            text_of(&result),
            words,
            "{} does not answer with `sure {}`'s words",
            route.tool,
            route.command.join(" ")
        );
        assert_eq!(
            result["structuredContent"]["routes_to"].as_str(),
            Some(route.routes_to),
            "{}",
            route.tool
        );
        assert_eq!(
            human.status,
            3,
            "this test compares refusals, and `sure {}` answered",
            route.command.join(" ")
        );
    }
    assert_eq!(session.finish().status, 0);
}

#[test]
fn sure_status_answers_about_this_build_and_never_about_a_project() {
    let mut session = Session::serve();
    session.start();
    let answer = session.ask(call(20, "sure_status", json!({})));
    let result = result_of(&answer).clone();
    assert_eq!(result["isError"], false, "{answer}");
    assert_eq!(result["structuredContent"]["sure"]["command"], "doctor");
    assert_eq!(
        result["structuredContent"]["mcp"]["protocol_version"],
        PROTOCOL_VERSION
    );
    assert_eq!(
        &result["structuredContent"]["mcp"]["commands_implemented"],
        &json!(sure_cli::commands::IMPLEMENTED)
    );
    // Agreement with the command line, on the fields a doctor report cannot
    // change between two runs of the same build on the same machine.
    let frame = frame_of(&["doctor"]);
    for field in ["command", "outcome", "exit_code"] {
        assert_eq!(
            result["structuredContent"]["sure"][field], frame[field],
            "`sure doctor`'s {field} disagrees with the tool's"
        );
    }
    // And the words are doctor's own.
    assert_eq!(text_of(&result), run(&["doctor"]).stdout);
    assert_eq!(session.finish().status, 0);
}

#[test]
fn no_tool_reports_success_for_a_project_that_was_never_checked() {
    // The false green, in its MCP shape: an agent reads `isError: false` and
    // tells the user the project is fine.
    let mut session = Session::serve();
    session.start();
    for (index, route) in routes().iter().enumerate() {
        let result =
            result_of(&session.ask(call(30 + index as u32, route.tool, route.arguments.clone())))
                .clone();
        assert_eq!(
            result["isError"], true,
            "{} reported success for work that did not happen: {result}",
            route.tool
        );
        assert!(
            text_of(&result).contains("is not implemented in this build."),
            "{}: {}",
            route.tool,
            text_of(&result)
        );
        assert_eq!(
            result["structuredContent"]["sure"]["outcome"],
            "unavailable"
        );
        assert_eq!(result["structuredContent"]["sure"]["exit_code"], 3);
        assert_ne!(result["structuredContent"]["sure"]["outcome"], "ok");
    }
    assert_eq!(session.finish().status, 0);
}

#[test]
fn no_argument_can_reach_the_execution_privacy_or_protection_policy() {
    let mut session = Session::serve();
    session.start();
    let attempts = [
        (
            "sure_check",
            "execution_mode",
            json!({"execution_mode": "unrestricted"}),
        ),
        ("sure_check", "privacy", json!({"privacy": "off"})),
        ("sure_recheck", "mode", json!({"mode": "unrestricted"})),
        ("sure_status", "trust", json!({"trust": "everything"})),
        // Beside a project, which is the one argument a tool does accept.
        (
            "sure_check",
            "force",
            json!({"project": "somewhere", "force": true}),
        ),
    ];
    for (index, (tool, key, arguments)) in attempts.iter().enumerate() {
        let answer = session.ask(call(40 + index as u32, tool, arguments.clone()));
        assert_eq!(error_code(&answer), -32602, "{answer}");
        let message = answer["error"]["message"].as_str().expect("a message");
        assert!(message.contains(key), "{message}");
        assert!(
            message.contains("the user's own SURE configuration"),
            "the refusal does not say where the policy comes from: {message}"
        );
    }
    // And the list said so before the call did: no tool offers such a key.
    let listed = result_of(&session.ask(json!({
        "jsonrpc": "2.0", "id": 50, "method": "tools/list"
    })))
    .clone();
    for tool in listed["tools"].as_array().expect("tools") {
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        assert!(properties.len() <= 1, "{tool}");
    }
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_project_on_a_tool_that_takes_none_is_refused_rather_than_ignored() {
    let mut session = Session::serve();
    session.start();
    let answer = session.ask(call(60, "sure_get_report", json!({"project": "somewhere"})));
    assert_eq!(error_code(&answer), -32602);
    let message = answer["error"]["message"].as_str().expect("a message");
    assert!(message.contains("sure history list"), "{message}");
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_relative_project_path_is_refused_the_way_the_command_line_refuses_it() {
    let mut session = Session::serve();
    session.start();
    let result =
        result_of(&session.ask(call(70, "sure_check", json!({"project": "relative-dir"})))).clone();
    assert_eq!(result["isError"], true);
    assert_eq!(text_of(&result), run(&["check", "relative-dir"]).stderr);
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_project_path_with_spaces_and_non_ascii_characters_round_trips() {
    // Windows engineering discipline: a path with a space and a path that is
    // not ASCII are the two that break a bridge written with a shell in mind.
    let project = "C:\\Users\\someone\\ünïcode dir\\проект";
    let mut session = Session::serve();
    session.start();
    let result =
        result_of(&session.ask(call(80, "sure_check", json!({"project": project})))).clone();
    assert_eq!(result["isError"], true);
    assert_eq!(text_of(&result), run(&["check", project]).stderr);
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_request_before_the_handshake_is_refused_and_the_handshake_still_works() {
    let mut session = Session::serve();
    let answer = session.ask(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(error_code(&answer), -32002);
    session.start();
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_second_handshake_is_refused_and_the_session_goes_on() {
    let mut session = Session::serve();
    session.start();
    let answer = session.ask(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "initialize",
        "params": {"protocolVersion": PROTOCOL_VERSION},
    }));
    assert_eq!(error_code(&answer), -32600);
    let pong = session.ask(json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
    assert!(pong.get("result").is_some(), "{pong}");
    assert_eq!(session.finish().status, 0);
}

#[test]
fn an_unknown_method_is_refused_by_name_and_the_session_goes_on() {
    let mut session = Session::serve();
    session.start();
    let answer = session.ask(json!({"jsonrpc": "2.0", "id": 2, "method": "resources/list"}));
    assert_eq!(error_code(&answer), -32601);
    assert!(
        answer["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("resources/list")),
        "{answer}"
    );
    let pong = session.ask(json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
    assert!(pong.get("result").is_some(), "{pong}");
    assert_eq!(session.finish().status, 0);
}

#[test]
fn an_unknown_tool_is_refused_as_a_protocol_error() {
    let mut session = Session::serve();
    session.start();
    let answer = session.ask(call(2, "sure_chekc", json!({})));
    assert_eq!(error_code(&answer), -32602);
    assert_eq!(
        answer["error"]["message"].as_str(),
        Some("Unknown tool: sure_chekc")
    );
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_line_that_is_not_json_is_refused_with_a_null_identifier_and_the_session_goes_on() {
    let mut session = Session::serve();
    session.start();
    session.send("{ this is not json");
    let answer = session.read();
    assert_eq!(error_code(&answer), -32700);
    assert!(answer["id"].is_null(), "{answer}");
    // A blank line is not a message either.
    session.send("");
    let answer = session.read();
    assert_eq!(error_code(&answer), -32700);
    let pong = session.ask(json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
    assert!(pong.get("result").is_some(), "{pong}");
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_line_longer_than_sure_will_hold_is_refused_and_the_next_line_is_read_as_a_message() {
    // The size bound is SURE's (the specification sets none), and so is the
    // recovery: the rest of the line is discarded rather than read as the
    // beginning of the next message.
    let mut session = Session::serve();
    session.start();
    let mut oversized = vec![b'x'; MAX_MESSAGE_BYTES + 1024];
    oversized.push(b'\n');
    session.send_bytes(&oversized);
    let answer = session.read();
    assert_eq!(error_code(&answer), -32700);
    assert!(answer["id"].is_null(), "{answer}");
    let pong = session.ask(json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
    assert!(pong.get("result").is_some(), "{pong}");
    assert_eq!(session.finish().status, 0);
}

#[test]
fn a_notification_is_never_answered_and_never_runs_anything() {
    let mut session = Session::serve_json();
    session.start();
    session.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string());
    session.send(
        &json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {"name": "sure_check", "arguments": {"project": "somewhere"}},
        })
        .to_string(),
    );
    // The proof that nothing was answered and nothing ran: the next line on
    // stdout is the answer to the ping, and the session envelope says that no
    // tool call went past.
    let pong = session.ask(json!({"jsonrpc": "2.0", "id": 9, "method": "ping"}));
    assert_eq!(pong["id"], 9, "{pong}");
    let finished = session.finish();
    assert_eq!(finished.status, 0);
    let envelope: Value = serde_json::from_str(
        finished
            .unread
            .first()
            .expect("the session envelope is the only thing left on stdout"),
    )
    .expect("the session envelope is one frame");
    assert_eq!(finished.unread.len(), 1, "{:?}", finished.unread);
    assert_eq!(envelope["command"], "mcp");
    assert_eq!(envelope["details"]["tool_calls"], 0, "{envelope}");
    assert_eq!(envelope["details"]["notifications"], 2, "{envelope}");
    assert_eq!(envelope["details"]["errors"], 0, "{envelope}");
    assert_eq!(envelope["details"]["initialized"], true, "{envelope}");
    assert_eq!(envelope["details"]["answered"], 2, "{envelope}");
}

#[test]
fn stdout_carries_protocol_messages_and_the_session_summary_goes_to_stderr() {
    // The specification's rule: the server must not write anything to stdout
    // that is not a valid MCP message. The summary of a session is not one, so
    // with the default format it is a diagnostic.
    let mut session = Session::serve();
    session.start();
    let _ = session.ask(call(2, "sure_check", json!({})));
    let finished = session.finish();
    assert_eq!(finished.status, 0);
    assert!(
        finished.unread.is_empty(),
        "stdout carried something that is not a protocol message: {:?}",
        finished.unread
    );
    assert!(
        finished.stderr.contains("handshake") && finished.stderr.contains(PROTOCOL_VERSION),
        "{}",
        finished.stderr
    );
    assert!(
        finished
            .stderr
            .contains("the bridge itself checked nothing"),
        "the summary does not say what it is not: {}",
        finished.stderr
    );
}

#[test]
fn the_machine_format_puts_one_envelope_on_stdout_after_the_last_message() {
    // A wart, and a deliberate one: `--format json` means "everything SURE says
    // goes to stdout as one frame". A caller that reads this stream as a
    // protocol stream must not ask for the machine format.
    // `docs/architecture/MCP_BRIDGE.md` says so.
    let mut session = Session::serve_json();
    session.start();
    let finished = session.finish();
    assert_eq!(finished.status, 0);
    assert_eq!(finished.unread.len(), 1, "{:?}", finished.unread);
    let envelope: Value = serde_json::from_str(&finished.unread[0]).expect("one frame");
    assert_eq!(envelope["command"], "mcp");
    assert_eq!(envelope["outcome"], "ok");
    assert_eq!(envelope["exit_code"], 0);
    assert_eq!(envelope["details"]["answered"], 1, "{envelope}");
    // The human summary is a diagnostic, so this run's stderr is not a copy of
    // it: the machine form went to stdout instead.
    assert!(
        !finished.stderr.contains("sure_version"),
        "{}",
        finished.stderr
    );
}

#[test]
fn a_caller_that_never_handshakes_gets_a_summary_that_says_so() {
    let mut session = Session::serve_json();
    session.ask(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    let finished = session.finish();
    assert_eq!(finished.status, 0);
    let envelope: Value = serde_json::from_str(&finished.unread[0]).expect("one frame");
    assert_eq!(envelope["details"]["initialized"], false, "{envelope}");
    assert_eq!(envelope["details"]["answered"], 1, "{envelope}");
    assert_eq!(envelope["details"]["errors"], 1, "{envelope}");
    assert!(
        envelope["details"]["agreed_protocol_version"].is_null(),
        "{envelope}"
    );
}

#[test]
fn the_bridge_writes_nothing_to_stdout_when_the_caller_says_nothing() {
    let session = Session::serve();
    let finished = session.finish();
    assert_eq!(finished.status, 0);
    assert!(finished.unread.is_empty(), "{:?}", finished.unread);
    assert!(
        finished.stderr.contains("never completed one"),
        "{}",
        finished.stderr
    );
}

#[test]
fn the_grammar_refuses_a_bare_sure_mcp() {
    // `serve` is not optional, for `sure hook`'s reason: a harness that read
    // "SURE is listening" out of a process that had already exited would be
    // told about a bridge that was not there.
    let run = run(&["mcp"]);
    assert_eq!(run.status, 2, "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{:?}", run.stdout);
}

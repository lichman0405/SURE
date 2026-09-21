//! What `sure_core::probe` claims, checked against real sockets.
//!
//! The acceptance is *"Probe records concrete response/outcome."* and *"Open
//! port alone is not feature completeness."* Both are claims about a boundary
//! rather than about a type, so nothing here calls `classify` or builds a
//! `ProbeOutcome` by hand and calls that a record: every outcome asserted below
//! came out of a socket that a server on this machine actually wrote to.
//!
//! # The instrument, and why it is a thread rather than a file
//!
//! [`serve`] binds `127.0.0.1:0`, hands back the port the operating system
//! chose, and runs one accepting thread. The port is never hardcoded, so two
//! runs of this file — or two tests in the same run — cannot collide, and a
//! machine with something already on `3000` cannot make these tests lie. The
//! thread reads the request and sends it back over a channel, which is what
//! makes the *request* half of the claim checkable: a test can assert on the
//! bytes the probe actually put on the wire rather than on the format string it
//! was built from.
//!
//! # The test the acceptance is about
//!
//! [`an_open_port_that_says_nothing_does_not_aggregate_to_green`] and
//! [`the_same_open_port_that_answers_does_aggregate_to_green`] are a pair, and
//! neither means anything without the other: the first alone would be satisfied
//! by a probe that never returns green, and the second alone by the false green
//! this product exists to prevent. What separates them is one line of server
//! behavior and the assertion goes through
//! [`aggregate`](sure_domain::status::aggregate) rather than through this
//! module's own enum — the claim being checked is about the verdict, not about
//! which variant a function returned.
//!
//! # What is not claimed here
//!
//! **Nothing about whether the feature works.** A `pass` here means a request
//! was answered, and the tests assert that the title says so — see
//! [`a_pass_names_the_request_it_made_and_not_the_feature`]. A body is never
//! matched against anything, because this module does not read bodies.
//!
//! **Nothing about a hostile peer.** Every server here is written by this file.
//! What a service does with a request it dislikes, and what a peer that lies in
//! its headers can make a reader do, are not in scope for a probe that parses no
//! header at all.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread;
use std::time::{Duration, Instant};

use sure_core::probe::{Endpoint, EndpointError, Limits, Probe, ProbeOutcome};
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckStatus, aggregate};

/// How long a server here holds a connection open when the point of the test is
/// that the probe stops reading first. Comfortably longer than [`SHORT`], so
/// "the deadline fired" and "the server closed" cannot be confused.
const HOLD: Duration = Duration::from_secs(5);

/// The probe budget for anything that is expected to finish on its own. Loopback
/// answers in microseconds; this is seconds, because a loaded CI machine is not
/// this one and a timeout that is tight enough to be interesting is a timeout
/// that fails for reasons that are not the code's.
const GENEROUS: Duration = Duration::from_secs(10);

/// A budget small enough that a silent server cannot outlast it, and large
/// enough that the connect itself succeeds. Used only where the test *wants* the
/// deadline to fire.
const SHORT: Duration = Duration::from_millis(250);

/// More than any response below, so the byte bound is never what stopped a read
/// unless a test says so.
const ROOMY: usize = 64 * 1024;

/// Binds a loopback port, serves exactly one connection, and reports the request.
///
/// Returns the port and a receiver carrying the request bytes **as the server
/// read them** — through `\r\n\r\n`, which is as much as the probe ever sends.
/// The thread is detached: it ends when the connection does, and a test that
/// leaves a server holding a socket open does not hold the binary open with it.
fn serve(behavior: impl FnOnce(&mut TcpStream) + Send + 'static) -> (u16, Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port must be bindable");
    let port = listener
        .local_addr()
        .expect("a bound listener has an address")
        .port();
    let (sender, receiver) = channel();

    thread::spawn(move || {
        let Ok((mut stream, _peer)) = listener.accept() else {
            return;
        };
        let request = read_request(&mut stream);
        // A send that fails means the test is gone; the server has nothing left
        // to say either way.
        let _ = sender.send(request);
        behavior(&mut stream);
    });

    (port, receiver)
}

/// A loopback port nothing is listening on **at the moment it is handed over**,
/// checked rather than assumed.
///
/// This is the instrument `a_port_with_nothing_behind_it_is_refused_rather_than_unreachable`
/// was missing, and the test is the fourth copy of the shape `P17-T001` repaired
/// in `http_routes.rs`, `runtime_start.rs` and `browser_driver.rs`. What it
/// replaces was three lines long: bind `127.0.0.1:0`, take the number, drop the
/// listener, hand the number to the probe and assert a refusal. A port asked for
/// as `0` comes out of the machine's **dynamic range** — 49152 to 65535 on this
/// one, from `netsh int ipv4 show dynamicport tcp`, and 32768 and up by default
/// on the platforms this has to port to — which is the same range every other
/// `bind(0)` on the host draws from, including the ones inside the other copies
/// of this binary when a gate runs. Something else can therefore be handed the
/// number in the window between the release and the probe, and the test would
/// then be reading whatever answered.
///
/// So the numbers here come from below that range, are walked forwards one at a
/// time from a starting point no other live copy of this binary walks, and are
/// each tried with `bind` before being released — a port that is taken, reserved
/// or excluded is skipped rather than handed to the probe as if it were empty.
///
/// **The re-check with `connect_timeout` is kept here, and it is the half that
/// matters for this test.** `http_routes.rs`'s copy carries it and
/// `runtime_start.rs`'s does not; the difference is what each one does with the
/// number. `runtime_start.rs` hands its port to a child that binds it and
/// reports whether it could, so a failed `bind` there is answered by the child
/// rather than by the test. This test asks the opposite question — it asserts
/// that *nothing* answers — and the claim `bind` supports is only "no listener
/// was bound when it was tried". A connection attempt that ends without an
/// answer is the property itself rather than a proxy for it, so the port is
/// returned only once one has ended that way. On a Unix build that is a refusal
/// (and on Windows the connect ends at the bound); both are an `Err` here, and
/// both mean nothing answered.
///
/// **The severity is a false red, and it is stated at the strength it was
/// measured.** A process that takes the port in the remaining window — between
/// the check above and the probe's own connect, which is microseconds — makes
/// the assertion on [`ProbeOutcome::Refused`] fail, so what this walk narrows is
/// a *flake*, not a hidden green: the test fails loudly rather than passing
/// while the intended refusal path was never exercised. No scheme on this side
/// can remove that residue, which is why it is written down rather than
/// claimed away.
fn free_port() -> u16 {
    /// Below every default dynamic range, and past the well-known and registered
    /// ports a machine's own services are actually likely to be on.
    const LOWEST: u16 = 10_000;
    const HIGHEST: u16 = 32_000;
    /// How far the walk goes before giving up. A range this wide cannot be full
    /// of listeners, so the walk ends long before this on any machine.
    const TRIES: u16 = 1_000;

    static NEXT: AtomicU32 = AtomicU32::new(0);
    let span = HIGHEST - LOWEST;
    // A per-process start and a per-test step, the same shape the fixture names
    // use: two copies of this binary started together do not walk the same
    // numbers in the same order.
    let start = (std::process::id() % u32::from(span)) as u16;
    let step = (NEXT.fetch_add(1, Ordering::Relaxed) % u32::from(span)) as u16;

    for attempt in 0..TRIES {
        let port = LOWEST + (start.wrapping_add(step).wrapping_add(attempt) % span);
        let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, port)) else {
            // Taken, reserved or excluded: skipped rather than handed to a probe
            // that would then be asking about something else.
            continue;
        };
        drop(listener);

        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        if TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_err() {
            return port;
        }
    }

    panic!(
        "all {TRIES} loopback ports this test walked between {LOWEST} and {HIGHEST} answered a \
         connection attempt or refused to be bound, so no port could be shown to be one nothing \
         answers on, and `a_port_with_nothing_behind_it_is_refused_rather_than_unreachable` would \
         have been reading something else. That is a fact about this machine's load and not about \
         SURE."
    );
}

/// Reads until the blank line that ends the request, or until the peer stops.
fn read_request(stream: &mut TcpStream) -> String {
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
    String::from_utf8_lossy(&buffer).into_owned()
}

/// The request a server received, or a failure naming what happened instead.
fn request_seen(receiver: &Receiver<String>) -> String {
    match receiver.recv_timeout(GENEROUS) {
        Ok(request) => request,
        Err(RecvTimeoutError::Timeout) => {
            panic!("the server never received a request within {GENEROUS:?}")
        }
        Err(RecvTimeoutError::Disconnected) => {
            panic!("the server thread ended without reporting a request")
        }
    }
}

/// A probe with the given budget and a byte bound no response here reaches.
fn probe_under(timeout: Duration) -> Probe {
    Probe::new(Limits::new(timeout, ROOMY))
}

/// A critical check built from an outcome, aggregated, and reported as green or
/// not.
fn aggregates_green(outcome: &ProbeOutcome, endpoint: &Endpoint) -> bool {
    let result = outcome.verdict(
        CheckId::generate(),
        endpoint,
        Severity::MustFix,
        true,
        FingerprintId::generate(),
    );
    aggregate(&[result]).is_green()
}

// ---------------------------------------------------------------------------
// The pair the acceptance is about
// ---------------------------------------------------------------------------

#[test]
fn an_open_port_that_says_nothing_does_not_aggregate_to_green() {
    // The port really is open: the server accepts the connection and holds it.
    // Whatever this test proves, it may not prove it by the connect failing.
    let (port, _receiver) = serve(|stream| {
        thread::sleep(HOLD);
        let _ = stream.shutdown(std::net::Shutdown::Both);
    });
    let endpoint = Endpoint::loopback(port, "/health").unwrap();

    let started = Instant::now();
    let outcome = probe_under(SHORT).get(&endpoint);
    let elapsed = started.elapsed();

    assert_eq!(
        outcome,
        ProbeOutcome::NoAnswer { bytes_read: 0 },
        "a server that accepted and said nothing is the case this variant exists for"
    );
    assert!(
        outcome.opened_a_connection(),
        "the port was open, and the outcome has to say so"
    );
    assert!(
        !outcome.is_an_answer(),
        "an open port is not an answer, which is the whole of the acceptance"
    );
    assert_eq!(outcome.status(), CheckStatus::Unknown);
    assert_eq!(
        outcome.reason(),
        "the port is open and nothing was said \u{2014} an open port is not an answer",
        "the sentence a report prints here is the acceptance, and it has to say what was \
         established rather than only what was not"
    );
    assert!(
        elapsed < HOLD,
        "the probe returned after {elapsed:?}, which means the server closing is what ended \
         the read rather than the probe's own {SHORT:?} deadline"
    );
    assert!(
        !aggregates_green(&outcome, &endpoint),
        "a critical check that established only that something is listening must not aggregate \
         to green; the outcome was {outcome:?}"
    );
}

#[test]
fn the_same_open_port_that_answers_does_aggregate_to_green() {
    // The same server, one line different: it answers. Without this, the test
    // above would pass against a probe that never returns green at all.
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n");
    });
    let endpoint = Endpoint::loopback(port, "/health").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);

    assert!(
        outcome.is_an_answer(),
        "the server wrote a status line and a blank line; the probe saw {outcome:?}"
    );
    assert_eq!(outcome.status(), CheckStatus::Pass);
    assert!(
        aggregates_green(&outcome, &endpoint),
        "an answered request is what a pass is for; the outcome was {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// The request, as the server received it
// ---------------------------------------------------------------------------

#[test]
fn the_probe_sends_a_request_line_naming_the_path_and_a_host_naming_the_socket() {
    let (port, receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
    });
    let endpoint = Endpoint::loopback(port, "/api/health?deep=1").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);
    assert!(
        outcome.is_an_answer(),
        "the server answered; got {outcome:?}"
    );

    let request = request_seen(&receiver);
    let mut lines = request.split("\r\n");
    assert_eq!(
        lines.next(),
        Some("GET /api/health?deep=1 HTTP/1.1"),
        "the request the server received was:{request}"
    );
    assert_eq!(
        lines.next(),
        Some(format!("Host: 127.0.0.1:{port}").as_str()),
        "the Host header must name the socket that was connected to, not a name that \
         resolves to it; the request was:{request}"
    );
    assert!(
        request.contains("Connection: close\r\n"),
        "the end of the response is the end of the connection, and the request is what asks \
         for that; the request was:{request}"
    );
    assert!(
        request.ends_with("\r\n\r\n"),
        "the request must end with a blank line, or the server is still waiting; the request \
         was:{request}"
    );
}

#[test]
fn the_response_is_recorded_as_a_status_line_and_a_body_count() {
    let body = "x".repeat(1234);
    let (port, _receiver) = serve(move |stream| {
        let response = format!("HTTP/1.1 201 Created\r\nContent-Length: 1234\r\n\r\n{body}");
        let _ = stream.write_all(response.as_bytes());
    });
    let endpoint = Endpoint::loopback(port, "/items").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);

    assert_eq!(
        outcome,
        ProbeOutcome::Answered {
            status: 201,
            status_line: "HTTP/1.1 201 Created".to_owned(),
            body_bytes: 1234,
            truncated: false,
        },
        "the outcome has to record what was said concretely enough to quote in a report"
    );
    assert_eq!(outcome.status(), CheckStatus::Pass);
}

#[test]
fn a_response_that_arrives_in_two_pieces_is_still_read_as_one_response() {
    // The header block is searched for in what has accumulated, not in whatever
    // the last read returned. The pause is what makes it likely the two writes
    // stay apart; **it does not guarantee it**, since a client descheduled for
    // longer than the pause would read both at once and this test would pass
    // without exercising the split. What it guarantees is the outcome, and the
    // split is what it exercises.
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n");
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(150));
        let _ = stream.write_all(b"Content-Length: 5\r\n\r\nhello");
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);

    assert_eq!(
        outcome,
        ProbeOutcome::Answered {
            status: 200,
            status_line: "HTTP/1.1 200 OK".to_owned(),
            body_bytes: 5,
            truncated: false,
        },
        "a response split across packets is one response"
    );
}

#[test]
fn a_status_line_with_a_four_digit_code_is_not_read_as_a_status() {
    // `HTTP/1.1 2000 OK` is not a status line, and the plausible-but-wrong read
    // is `200`. A parser that took the first three digits would report a pass
    // for a service that said something no HTTP parser accepts.
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 2000 OK\r\n\r\n");
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);

    assert_eq!(
        outcome,
        ProbeOutcome::NotHttp {
            first_line: "HTTP/1.1 2000 OK".to_owned(),
        },
        "a code that is not exactly three digits is not a code"
    );
    assert_eq!(outcome.status(), CheckStatus::Fail);
}

// ---------------------------------------------------------------------------
// The four ways of not answering
// ---------------------------------------------------------------------------

#[test]
fn something_other_than_http_that_speaks_first_is_recorded_as_not_http() {
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(b"SSH-2.0-OpenSSH_9.6\r\n");
        let _ = stream.shutdown(std::net::Shutdown::Both);
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);

    assert_eq!(
        outcome,
        ProbeOutcome::NotHttp {
            first_line: "SSH-2.0-OpenSSH_9.6".to_owned(),
        },
        "a service that spoke first and not in HTTP is a concrete finding, not a parse error"
    );
    assert_eq!(
        outcome.status(),
        CheckStatus::Fail,
        "HTTP was asked for, and this is not HTTP"
    );
    assert!(
        outcome.reason().contains("SSH-2.0-OpenSSH_9.6"),
        "the reason has to carry what was actually said: {}",
        outcome.reason()
    );
}

#[test]
fn a_port_with_nothing_behind_it_is_refused_rather_than_unreachable() {
    // The port comes from [`free_port`] and not from the operating system's
    // allocator. The version this replaces bound `127.0.0.1:0`, took the number,
    // released it and then asserted a refusal — and the comment above it read
    // "the operating system says so with a refusal", which is the one thing that
    // version **cannot** support: a number released out of the dynamic range can
    // be handed to something else in the window before the probe asks, and the
    // probe would then be observing that other thing's answer (or its silence).
    // What is claimed here is that a port **measured** to have nothing behind it
    // is reported as a refusal rather than as unreachable, and the measurement
    // is what makes the claim about the project rather than about the machine.
    let port = free_port();
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);

    assert!(
        matches!(outcome, ProbeOutcome::Refused { .. }),
        "a closed loopback port refuses the connection; the probe reported {outcome:?}"
    );
    assert!(
        !outcome.opened_a_connection(),
        "nothing was listening, so no connection was opened"
    );
    assert_eq!(outcome.status(), CheckStatus::Fail);
}

#[test]
fn a_service_that_starts_an_answer_and_stops_is_recorded_with_how_much_arrived() {
    // The header block never completes, so this is not an answer — but it is not
    // silence either, and the count is what separates a port that said nothing
    // from a service that began and stalled.
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Leng");
        let _ = stream.flush();
        thread::sleep(HOLD);
        let _ = stream.shutdown(std::net::Shutdown::Both);
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = probe_under(SHORT).get(&endpoint);

    assert_eq!(
        outcome,
        ProbeOutcome::NoAnswer {
            bytes_read: "HTTP/1.1 200 OK\r\nContent-Leng".len(),
        },
        "a partial response is not an answer, and the count is the evidence"
    );
    assert_eq!(
        outcome.status(),
        CheckStatus::Unknown,
        "half a status line establishes nothing either way"
    );
    assert!(
        outcome.reason().contains("29 bytes"),
        "the reason has to carry the count: {}",
        outcome.reason()
    );
    assert!(
        !aggregates_green(&outcome, &endpoint),
        "a stalled service must not aggregate to green"
    );
}

#[test]
fn a_response_still_arriving_when_the_probe_stops_reading_is_marked_truncated() {
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\nstart");
        let _ = stream.flush();
        thread::sleep(HOLD);
        let _ = stream.shutdown(std::net::Shutdown::Both);
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let started = Instant::now();
    let outcome = probe_under(SHORT).get(&endpoint);
    let elapsed = started.elapsed();

    match outcome {
        ProbeOutcome::Answered {
            status: 200,
            truncated: true,
            body_bytes,
            ..
        } => assert_eq!(
            body_bytes, 5,
            "the body count is what had arrived, not what was declared"
        ),
        other => panic!("expected a truncated answer, got {other:?}"),
    }
    assert!(
        elapsed < HOLD,
        "the deadline is what ended this read, not the server: {elapsed:?}"
    );
}

#[test]
fn the_byte_bound_is_what_is_kept_exactly_and_stops_the_read() {
    // The header block this server sends, whose length is subtracted below
    // rather than written out, so the arithmetic cannot drift from the bytes.
    const HEAD: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";

    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(HEAD);
        // Far more than the bound. A read chunk is 4 KiB, so a bound that were
        // only checked between reads would let this through eightfold.
        for _ in 0..64 {
            if stream.write_all(&[b'z'; 1024]).is_err() {
                return;
            }
        }
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = Probe::new(Limits::new(GENEROUS, 512)).get(&endpoint);

    match outcome {
        ProbeOutcome::Answered {
            status: 200,
            truncated,
            body_bytes,
            ..
        } => {
            assert!(truncated, "the bound is what stopped this read");
            assert_eq!(
                body_bytes,
                512 - HEAD.len(),
                "the bound is what is kept, so a 512-byte bound with a {}-byte header keeps \
                 {} body bytes — not the bound plus one read chunk",
                HEAD.len(),
                512 - HEAD.len()
            );
        }
        other => panic!("expected an answer cut off by the bound, got {other:?}"),
    }
}

#[test]
fn a_reply_that_could_not_be_http_from_its_first_byte_is_not_http() {
    // The TLS case, which is the one a real user hits: an HTTPS port answered a
    // plaintext request with a binary record. It contains no CRLF, so a reader
    // that waited for the end of a header block would wait out the whole
    // deadline and then report a port that said nothing.
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(&[0x15, 0x03, 0x03, 0x00, 0x02, 0x02, 0x46]);
        let _ = stream.flush();
        thread::sleep(HOLD);
        let _ = stream.shutdown(std::net::Shutdown::Both);
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let started = Instant::now();
    let outcome = probe_under(GENEROUS).get(&endpoint);
    let elapsed = started.elapsed();

    let status = outcome.status();
    match outcome {
        ProbeOutcome::NotHttp { first_line } => assert!(
            first_line.starts_with('\u{15}'),
            "the first line is the bytes as they arrived, lossily decoded: {first_line:?}"
        ),
        other => panic!("expected not-HTTP, got {other:?}"),
    }
    assert_eq!(status, CheckStatus::Fail);
    assert!(
        elapsed < HOLD,
        "the outcome is decidable from the first byte, so the deadline is not what produced \
         it: {elapsed:?}"
    );
}

#[test]
fn a_first_line_that_never_ends_is_cut_rather_than_kept_whole() {
    // "The first line" of a binary reply is whatever arrived, so the field has to
    // be bounded or a report can carry the whole byte bound of lossy-decoded
    // binary. The cut is marked, so a cut line cannot read as a whole one.
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(&[0xff; 3000]);
        let _ = stream.flush();
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);

    match outcome {
        ProbeOutcome::NotHttp { first_line } => {
            assert!(
                first_line.ends_with('\u{2026}'),
                "a cut first line has to say it was cut: {first_line:?}"
            );
            assert!(
                first_line.chars().count() <= 121,
                "120 bytes plus the mark; got {} characters",
                first_line.chars().count()
            );
        }
        other => panic!("expected not-HTTP, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The endpoint, and what it refuses to be
// ---------------------------------------------------------------------------

#[test]
fn an_endpoint_that_is_not_on_this_machine_cannot_be_built() {
    // TEST-NET-1: reserved for documentation by RFC 5737 and routed nowhere, so
    // a constructor that let this through would be a probe that reaches off the
    // machine for permission it does not hold.
    let address = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));

    let refused = Endpoint::at(address, 80, "/");

    assert_eq!(refused, Err(EndpointError::NotLoopback { address }));
}

#[test]
fn every_loopback_address_is_local_and_a_named_host_is_not_expressible() {
    // `is_loopback`, not `== LOCALHOST`: the whole 127/8 block is this machine.
    // And `localhost` cannot be written at all, because the host is an `IpAddr`
    // and a name resolved through a hosts file anything may have edited is not
    // the same claim.
    assert!(Endpoint::loopback(8080, "/").is_ok());
    assert!(Endpoint::at(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 5)), 8080, "/").is_ok());
    assert!(Endpoint::at(IpAddr::V6(Ipv6Addr::LOCALHOST), 8080, "/").is_ok());
    assert!(Endpoint::at(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080, "/").is_err());
}

#[test]
fn a_path_that_would_add_a_header_or_split_the_request_line_is_refused() {
    // The first case is the one that matters: a CRLF in the path does not
    // lengthen the path, it ends the request line and starts headers, chosen by
    // whatever produced the string.
    for path in [
        "/health\r\nX-Injected: 1",
        "/health\r\n\r\nGET /admin HTTP/1.1\r\nHost: x",
        "/health\nX-Injected: 1",
        "/two words",
        "no-leading-slash",
        "",
        "/back\\slash",
        "/\u{65e5}\u{672c}",
    ] {
        assert_eq!(
            Endpoint::loopback(8080, path),
            Err(EndpointError::UnsafePath {
                path: path.to_owned()
            }),
            "{path:?} must not become a request line"
        );
    }
}

#[test]
fn an_already_encoded_path_passes_through_byte_for_byte() {
    let endpoint = Endpoint::loopback(8080, "/a%20b/c?d=%2F").unwrap();

    assert_eq!(endpoint.path(), "/a%20b/c?d=%2F");
    assert_eq!(endpoint.request_line(), "GET /a%20b/c?d=%2F HTTP/1.1");
    assert_eq!(endpoint.address().port(), 8080);
}

#[test]
fn a_budget_of_zero_makes_no_attempt_and_cannot_report_a_pass() {
    // The tempting implementation — round the zero up, or hand it to
    // `connect_timeout` — either reports a green check out of a budget of zero
    // or blames the platform for the caller's mistake. So this is a probe
    // failure, and the server below never sees a connection.
    let (port, receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
    });
    let endpoint = Endpoint::loopback(port, "/").unwrap();

    let outcome = Probe::new(Limits::new(Duration::ZERO, ROOMY)).get(&endpoint);

    // The detail, not just the variant: `connect_timeout` also fails on a zero,
    // and a probe that let the platform reject it would report the same variant
    // with the operating system's wording — blaming the machine for the
    // caller's misconfiguration, and losing the one fact that explains it.
    assert!(
        matches!(&outcome, ProbeOutcome::Unreachable { detail } if detail.contains("budget is zero")),
        "a zero budget is a probe failure, and the outcome has to name the budget; got \
         {outcome:?}"
    );
    assert_eq!(outcome.status(), CheckStatus::Error);
    assert_eq!(
        outcome.reason(),
        "the exchange could not be made (no exchange was attempted because the probe's \
         budget is zero)"
    );
    assert!(
        receiver.recv_timeout(Duration::from_millis(250)).is_err(),
        "no attempt means no connection, so the server has no request to report"
    );
}

// ---------------------------------------------------------------------------
// What a verdict says, as opposed to what it means
// ---------------------------------------------------------------------------

#[test]
fn a_pass_names_the_request_it_made_and_not_the_feature() {
    let (port, _receiver) = serve(|stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
    });
    let endpoint = Endpoint::loopback(port, "/health").unwrap();

    let outcome = probe_under(GENEROUS).get(&endpoint);
    let result = outcome.verdict(
        CheckId::generate(),
        &endpoint,
        Severity::MustFix,
        true,
        FingerprintId::generate(),
    );

    assert_eq!(
        result.title, "local probe: GET /health HTTP/1.1 answered",
        "the title has to name what was asked, because *the feature works* is not what was \
         established"
    );
    assert!(
        !result.title.contains("works") && !result.title.contains("ready"),
        "a title claiming readiness would be the false green this acceptance forbids: {}",
        result.title
    );
}

#[test]
fn the_status_mapping_and_the_verdict_agree_on_every_variant() {
    // `status()` exists so a caller can ask what an outcome is worth without
    // naming a check, a severity and a fingerprint. It must not be a second
    // answer that can drift from the one `verdict` gives.
    let endpoint = Endpoint::loopback(1, "/").unwrap();
    let answer = |status: u16, line: &str| ProbeOutcome::Answered {
        status,
        status_line: line.to_owned(),
        body_bytes: 0,
        truncated: false,
    };

    // The whole mapping, written out rather than derived from the same
    // comparison the code makes — a test that recomputed `200..400` would agree
    // with a wrong boundary as happily as with the right one. The last four are
    // the boundary itself: the answer to "is a redirect a pass" and to "where
    // does the success class stop" are both decisions, and both are here.
    let cases = [
        (
            ProbeOutcome::Refused {
                detail: "connection refused".to_owned(),
            },
            CheckStatus::Fail,
        ),
        (
            ProbeOutcome::Unreachable {
                detail: "broken pipe".to_owned(),
            },
            CheckStatus::Error,
        ),
        (
            ProbeOutcome::NoAnswer { bytes_read: 0 },
            CheckStatus::Unknown,
        ),
        (
            ProbeOutcome::NoAnswer { bytes_read: 29 },
            CheckStatus::Unknown,
        ),
        (
            ProbeOutcome::NotHttp {
                first_line: "SSH-2.0-x".to_owned(),
            },
            CheckStatus::Fail,
        ),
        (answer(200, "HTTP/1.1 200 OK"), CheckStatus::Pass),
        (answer(399, "HTTP/1.1 399 Whatever"), CheckStatus::Pass),
        (answer(400, "HTTP/1.1 400 Bad Request"), CheckStatus::Fail),
        (answer(302, "HTTP/1.1 302 Found"), CheckStatus::Pass),
        (answer(404, "HTTP/1.1 404 Not Found"), CheckStatus::Fail),
        (
            answer(500, "HTTP/1.1 500 Internal Server Error"),
            CheckStatus::Fail,
        ),
    ];

    for (outcome, expected) in &cases {
        assert_eq!(
            outcome.status(),
            *expected,
            "the status mapping for {outcome:?} is wrong"
        );

        let result = outcome.verdict(
            CheckId::generate(),
            &endpoint,
            Severity::MustFix,
            true,
            FingerprintId::generate(),
        );
        assert_eq!(
            result.status, *expected,
            "verdict() and status() disagree about {outcome:?}"
        );
        assert_eq!(
            result.reason,
            outcome.reason(),
            "the reason on the result must be the outcome's own: {outcome:?}"
        );
        assert!(
            !result.reason.is_empty(),
            "every outcome needs a reason a report can print: {outcome:?}"
        );
    }

    // And the one that must never be reachable by accident: a critical check
    // built from a port that said nothing, aggregated.
    for outcome in [
        ProbeOutcome::NoAnswer { bytes_read: 0 },
        ProbeOutcome::NoAnswer { bytes_read: 29 },
    ] {
        assert!(
            !aggregates_green(&outcome, &endpoint),
            "{outcome:?} must not aggregate to green"
        );
    }
}

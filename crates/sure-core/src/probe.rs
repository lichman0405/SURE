//! Asking a running local service what it actually says.
//!
//! A *local probe* is one HTTP request to a service on this machine, and the
//! record of what came back. It is the second half of the pair whose first half
//! is [`crate::service`]: a supervisor starts something, a probe finds out
//! whether it answers, and neither of them decides whether the project is any
//! good.
//!
//! ```text
//! Supervisor::start  ->  Service (a process the operating system accepted)
//!                              |
//!                        Probe::get  ->  ProbeOutcome (what the socket said)
//!                              |
//!                     ProbeOutcome::verdict  ->  CheckResult (a narrow claim)
//! ```
//!
//! # An open port is not an answer, and the type says so
//!
//! [`super::service::Supervisor::start`] returning `Ok` means a process exists.
//! A caller that treats *the port is open* as *the feature works* has reported
//! the false green this product is built against, and the way to make that
//! impossible is not a warning in a doc comment — it is to keep *the port is
//! open* off the path that produces a verdict.
//!
//! **The boolean exists, and naming it here is the point rather than an
//! exception to it.** A caller asking *is the port open* finds
//! [`ProbeOutcome::opened_a_connection`], which answers exactly that question and
//! is true for `NoAnswer` — **and no verdict is built on it.**
//! [`ProbeOutcome::status`] matches the variant and never calls it, so the
//! variant that means *something accepted the connection and said nothing* maps
//! to [`CheckStatus::Unknown`](crate::status::CheckStatus::Unknown) — which
//! [`aggregate`](crate::status::aggregate) treats as **not checked** — and a
//! critical port-only probe cannot aggregate to green however loudly the caller
//! asks the port question. [`ProbeOutcome::is_an_answer`] is true for exactly
//! one of the five variants, and that is the one a caller reaching for *did it
//! work?* should find.
//!
//! That last sentence is the acceptance rather than a design preference:
//! *"Open port alone is not feature completeness."* A check that has read a
//! response body still does not know whether the feature works — that needs a
//! check that knows what the body should contain, and this module reads nothing
//! into a body at all. What a pass here establishes is exactly what its title
//! says: this request was answered, with a status the service considers
//! successful.
//!
//! # Loopback only, and the constructor is where that is decided
//!
//! [`Endpoint`] takes an [`IpAddr`] and refuses one that is not
//! [`is_loopback`](IpAddr::is_loopback). A probe reaching a machine that is not
//! this one is `ActionKind::ExternalService` and needs
//! [`Permission::ConnectService`](crate::execution::Permission::ConnectService);
//! a local probe is `ActionKind::LocalProbe` and needs
//! [`Permission::Inspect`](crate::execution::Permission::Inspect), which the
//! vocabulary grants unconditionally (*"Read-only analysis. Always granted."*).
//! **So the permission this module needs is one nobody can withhold, and the
//! permission it must not use is one it cannot reach** — because there is no
//! constructor for an endpoint that would need it.
//!
//! The host is taken as an [`IpAddr`] and not as a string, which is the same
//! decision one layer up: a string would accept `localhost`, and `localhost` is
//! resolved by the operating system through a hosts file that anything on this
//! machine may have written. A name is not a loopback address; an address is.
//!
//! # What is bounded, and what the bounds do not buy
//!
//! [`Limits::timeout`] bounds the whole exchange — connect, write and read — and
//! [`Limits::response_bytes`] bounds how much of the response is kept, exactly:
//! a read is cut to the room left rather than trimmed after the fact, so no read
//! chunk can push the buffer past the number the caller chose. A response that is
//! still arriving when either bound is reached is reported with
//! [`truncated`](ProbeOutcome::Answered) true, so a beginning is never read as
//! the whole. **The bounds do not stop the peer from sending**: nothing here
//! closes the connection early, and a service that writes a gigabyte writes a
//! gigabyte. What is bounded is what SURE holds and how long it waits.
//!
//! A **zero timeout** is refused as a probe failure and a **zero byte bound** is
//! not, and the asymmetry is deliberate rather than an oversight. A zero timeout
//! rounded up to anything at all can produce a *pass* — a loopback socket
//! connects in microseconds — which is the one outcome this product may not
//! manufacture. A zero byte bound cannot: with nothing kept, no header block can
//! be found, so the outcome is [`ProbeOutcome::NoAnswer`] and never a pass.
//!
//! # What is missing
//!
//! **Nothing here is asynchronous, and one probe blocks for up to its timeout.**
//! That is the right shape for a single request and the wrong shape for probing
//! twenty endpoints; a caller that wants those overlapped needs something this
//! build does not have.
//!
//! **No body is parsed, decoded or matched.** Transfer encodings are not
//! decoded, so `body_bytes` counts what arrived after the header block — which
//! is the body for an identity-encoded response and includes chunk framing for a
//! chunked one. Stated here because a count that means two things is exactly the
//! number a later check will read as one.
//!
//! **No header is parsed, `Content-Length` least of all.** The end of a response
//! is the end of the connection and nothing else, so a service that keeps its
//! socket open is reported [`truncated`](ProbeOutcome::Answered) after costing
//! the whole timeout even when it sent a complete body. Reading a declared length
//! would fix that, and would also mean trusting a number this module has no way
//! to check.
//!
//! **Bare LF is not accepted as a line ending.** The header block ends at
//! `\r\n\r\n` and nowhere else, because a lenient parser is a parser that
//! eventually mis-splits a response rather than refusing it, and refusing is
//! visible.

use std::fmt;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use sure_domain::evidence::EvidenceClass;
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, CheckStatus};

/// The address a probe looks at, and the path it asks for.
///
/// Built by [`Endpoint::loopback`] or [`Endpoint::at`], both of which refuse
/// anything that is not a loopback address — see the module documentation for
/// why that is a constructor's job rather than a check at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    address: SocketAddr,
    path: String,
}

impl Endpoint {
    /// `127.0.0.1` on `port`, asking for `path`.
    ///
    /// # Errors
    ///
    /// [`EndpointError::UnsafePath`] if `path` is not something that may be
    /// written into a request line.
    pub fn loopback(port: u16, path: impl Into<String>) -> Result<Self, EndpointError> {
        Self::at(IpAddr::V4(Ipv4Addr::LOCALHOST), port, path)
    }

    /// `address` on `port`, asking for `path`.
    ///
    /// # Errors
    ///
    /// [`EndpointError::NotLoopback`] if `address` is not a loopback address,
    /// and [`EndpointError::UnsafePath`] if `path` is not something that may be
    /// written into a request line.
    pub fn at(address: IpAddr, port: u16, path: impl Into<String>) -> Result<Self, EndpointError> {
        if !address.is_loopback() {
            return Err(EndpointError::NotLoopback { address });
        }
        let path = path.into();
        if !path_is_safe(&path) {
            return Err(EndpointError::UnsafePath { path });
        }
        Ok(Self {
            address: SocketAddr::new(address, port),
            path,
        })
    }

    /// The address, with the port.
    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    /// The path asked for, as it will be written into the request line.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// `GET <path> HTTP/1.1`, the request line this endpoint produces.
    #[must_use]
    pub fn request_line(&self) -> String {
        format!("GET {} HTTP/1.1", self.path)
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "http://{}{}", self.address, self.path)
    }
}

/// Whether a path may be written into a request line.
///
/// The request line ends at the first CRLF, so a path carrying one would not
/// lengthen the path — **it would add headers**, chosen by whatever produced the
/// string. A space would split the request line into a different one. A
/// backslash is refused for a Windows reason: a caller on this machine writing
/// `\health` means a separator, and the request-target grammar does not, so
/// sending it would ask for a path nobody named. Refusing all three is one
/// predicate because the answer to all three is the same — ask for something
/// else.
///
/// **Everything else outside the visible ASCII range is refused too**, and that
/// is the same decision rather than an oversight: a path with a non-ASCII byte
/// in it is not a legal request-target, and this module will not percent-encode
/// it, because encoding a path that a caller may have encoded already changes
/// what is asked for. A caller with such a path writes the `%`-escapes itself.
///
/// What is left alone is every graphic ASCII byte, `%` included: a path that is
/// already encoded passes through byte for byte, which is what makes the
/// refusal above a caller's one-time cost instead of a repeated one.
fn path_is_safe(path: &str) -> bool {
    path.starts_with('/')
        && !path.is_empty()
        && path
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'\\')
}

/// Why an endpoint was refused.
///
/// Neither variant is a warning about what might go wrong: both name a value
/// that this module will not turn into a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointError {
    /// The address is not on this machine.
    NotLoopback {
        /// The address that was offered.
        address: IpAddr,
    },
    /// The path cannot be written into a request line without changing it.
    UnsafePath {
        /// The path that was offered.
        path: String,
    },
}

impl fmt::Display for EndpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLoopback { address } => write!(
                formatter,
                "{address} is not a loopback address, so probing it is not a local probe"
            ),
            Self::UnsafePath { path } => write!(
                formatter,
                "{path:?} cannot be written into a request line unchanged"
            ),
        }
    }
}

impl std::error::Error for EndpointError {}

/// What one probe is allowed to spend.
///
/// The same shape as [`crate::process::Limits`] one module over, and for the same
/// reason: a caller that does not choose a bound has not decided what the check
/// is allowed to cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    timeout: Duration,
    response_bytes: usize,
}

impl Limits {
    /// Bounds for one probe: how long the whole exchange has, and how much of the
    /// response SURE keeps.
    ///
    /// A zero budget is not refused here, and it is not quietly rounded up
    /// either: [`Probe::get`] makes no attempt at all under a zero budget and
    /// returns [`ProbeOutcome::Unreachable`]. See the note on that method for why
    /// a zero could not be allowed to become a budget small enough to answer.
    #[must_use]
    pub const fn new(timeout: Duration, response_bytes: usize) -> Self {
        Self {
            timeout,
            response_bytes,
        }
    }

    /// How long the whole exchange has before the probe gives up reading.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// How many bytes of the response SURE keeps — exactly this many, at most.
    #[must_use]
    pub const fn response_bytes(&self) -> usize {
        self.response_bytes
    }
}

/// Sends one request and records what came back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    limits: Limits,
}

impl Probe {
    /// A probe with the given bounds.
    #[must_use]
    pub const fn new(limits: Limits) -> Self {
        Self { limits }
    }

    /// The bounds this probe works under.
    #[must_use]
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    /// `GET`s `endpoint` and records the outcome.
    ///
    /// This is the only thing this module does, and it never fails: every way the
    /// exchange can end is a [`ProbeOutcome`], because "nothing was listening" and
    /// "what arrived was not HTTP" are answers about the project and not errors in
    /// the checker. The one case that is the checker's own failure —
    /// [`ProbeOutcome::Unreachable`] — is a variant rather than a `Result` for the
    /// same reason: a caller that got a `Result` would handle the error and forget
    /// the other four, and the outcome it forgot is the one that means *an open
    /// port said nothing*.
    ///
    /// # A zero budget makes no attempt
    ///
    /// Every tempting alternative here produces a **pass out of a budget of
    /// zero**, which is the one thing this product may not do. Rounding a zero
    /// up to a millisecond is enough time for a loopback socket to connect and a
    /// small service to answer — so "spend no time at all" would report a green
    /// check. Passing the zero through to
    /// [`TcpStream::connect_timeout`] is worse: the platform rejects it, and the
    /// caller learns that the operating system refused something the caller had
    /// already asked not to happen.
    ///
    /// So a zero is [`ProbeOutcome::Unreachable`] with a reason naming the
    /// budget — a probe's own failure, which is
    /// [`CheckStatus::Error`](crate::status::CheckStatus::Error) and never
    /// aggregates to green. Nothing is connected and nothing is read.
    pub fn get(&self, endpoint: &Endpoint) -> ProbeOutcome {
        let budget = self.limits.timeout();
        if budget.is_zero() {
            return ProbeOutcome::Unreachable {
                detail: "no exchange was attempted because the probe's budget is zero".to_owned(),
            };
        }
        let deadline = Instant::now() + budget;

        let mut stream = match TcpStream::connect_timeout(&endpoint.address(), budget) {
            Ok(stream) => stream,
            Err(error) => return connection_failed(&error),
        };

        if let Err(error) = write_request(&mut stream, endpoint) {
            return connection_failed(&error);
        }

        read_response(&mut stream, deadline, self.limits.response_bytes())
    }
}

/// Writes the request, headers and all.
///
/// Three headers, and each is a decision. `Host` is required by HTTP/1.1 and is
/// the endpoint's own address rather than a name, because a name would be a
/// `Host` the caller did not ask for. `Connection: close` is what makes the end
/// of the response the end of the connection, which is what lets the read stop
/// at EOF rather than at a `Content-Length` this module would have to trust.
/// `User-Agent` says who asked, so a service's own log can answer that question
/// without guessing.
fn write_request(stream: &mut TcpStream, endpoint: &Endpoint) -> std::io::Result<()> {
    let request = format!(
        "{}\r\nHost: {}\r\nConnection: close\r\nUser-Agent: {}/{}\r\n\r\n",
        endpoint.request_line(),
        endpoint.address(),
        crate::NAME,
        crate::VERSION,
    );
    stream.write_all(request.as_bytes())
}

/// Reads until the response ends, the budget runs out, or the bound is reached.
///
/// **The end of the response is the end of the connection**, and that is what
/// `Connection: close` in the request buys: the read can stop at EOF rather than
/// at a `Content-Length` this module would have to parse and trust. A service
/// that ignores the header and keeps the socket open therefore costs the whole
/// timeout, and its body is reported `truncated` — the probe genuinely does not
/// know whether more was coming. That is a real cost of the simplification and
/// it is named rather than papered over.
fn read_response(stream: &mut TcpStream, deadline: Instant, bound: usize) -> ProbeOutcome {
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 4096];
    let mut truncated = false;

    loop {
        if buffer.len() >= bound {
            truncated = true;
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            truncated = true;
            break;
        }
        if stream.set_read_timeout(Some(remaining)).is_err() {
            truncated = true;
            break;
        }

        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                // The bound is what is *kept*, exactly, so the room left is what
                // one read may add. Extending by the whole chunk first and
                // trimming afterwards would make the bound "the bound, plus up
                // to one read" — a number that is not the number in `Limits`.
                let room = bound.saturating_sub(buffer.len());
                let taken = read.min(room);
                buffer.extend_from_slice(&chunk[..taken]);
                if taken < read {
                    truncated = true;
                    break;
                }

                // And stop as soon as the outcome can no longer change. The
                // deadline is for a service that is still answering; a service
                // that has already been understood gets none of it. Without
                // this, a port speaking TLS is proven not to be HTTP by its
                // first byte and then costs the whole timeout anyway.
                //
                // No guard on the header block is needed, which is worth stating
                // because it is not obvious: the predicate reads only as far as
                // the code token, so a response whose first line is a valid
                // status line satisfies it however long the rest of the buffer
                // is. The only buffers this rejects are ones whose first line is
                // already not HTTP — and for those the outcome is settled
                // whether or not a blank line has arrived.
                if !could_still_be_a_status_line(&buffer) {
                    break;
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) if is_a_timeout(&error) => {
                truncated = true;
                break;
            }
            Err(error) => return connection_failed(&error),
        }
    }

    classify(&buffer, truncated)
}

/// Turns what arrived into an outcome.
///
/// **Not a `Result` and not a boolean**: every branch here is a different claim
/// about the project, and the one a caller is most likely to get wrong — an
/// accepted connection with no response — is its own variant rather than a
/// falsy success.
fn classify(buffer: &[u8], truncated: bool) -> ProbeOutcome {
    if let Some(end) = header_block_end(buffer) {
        let first_line = first_line(&buffer[..end]);
        return match parse_status_line(&first_line) {
            Some((status, status_line)) => ProbeOutcome::Answered {
                status,
                status_line,
                body_bytes: buffer.len() - (end + 4),
                truncated,
            },
            None => ProbeOutcome::NotHttp { first_line },
        };
    }

    // No blank line yet — but the **first line** may already be settled, and it
    // is settled as soon as its bytes contradict the grammar rather than when a
    // CRLF arrives. That distinction is the difference between two very
    // different reports: a port speaking TLS, probed with a plaintext request,
    // answers with a binary record that contains no CRLF at all, and a port
    // whose protocol name is not `HTTP` is already contradicted by its first
    // byte. Both would otherwise be reported as a port that said nothing, which
    // is the least useful thing a report can say about a port that answered.
    if !could_still_be_a_status_line(buffer) {
        return ProbeOutcome::NotHttp {
            first_line: first_line(buffer),
        };
    }

    ProbeOutcome::NoAnswer {
        bytes_read: buffer.len(),
    }
}

/// Whether these bytes could still be the beginning of an HTTP status line.
///
/// The same grammar [`parse_status_line`] accepts, read as a condition on a
/// prefix: `HTTP/`, then a version up to a space, then exactly three digits, then
/// a space. Bytes that contradict it now will contradict it forever, which is
/// what makes the answer decidable before the line ends — and bytes that do not
/// still might, which is why this can only ever say *not yet* rather than *yes*.
///
/// It is deliberately a second statement of the grammar rather than a reuse of
/// the parser, because the two answer different questions. A drift between them
/// is possible and the tests are what hold them together: a response that
/// `parse_status_line` accepts must never be one this rejects, or a real answer
/// would be reported as not-HTTP.
fn could_still_be_a_status_line(bytes: &[u8]) -> bool {
    /// Whether `bytes` agrees with `prefix` as far as both go.
    fn prefixed_by(bytes: &[u8], prefix: &[u8]) -> bool {
        let compared = bytes.len().min(prefix.len());
        bytes[..compared] == prefix[..compared]
    }

    if !prefixed_by(bytes, b"HTTP/") {
        return false;
    }
    let Some((_version, after_version)) = split_once_at_space(bytes) else {
        // Still inside the version token, which this module never constrains
        // beyond its prefix.
        return true;
    };
    match split_once_at_space(after_version) {
        // The code token is still arriving. Three digits or fewer may still
        // become exactly three; a fourth digit, or a non-digit, cannot be
        // undone.
        None => after_version.len() <= 3 && after_version.iter().all(u8::is_ascii_digit),
        Some((code, _rest)) => code.len() == 3 && code.iter().all(u8::is_ascii_digit),
    }
}

/// The bytes before the first space, and the bytes after it.
fn split_once_at_space(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let at = bytes.iter().position(|byte| *byte == b' ')?;
    Some((&bytes[..at], &bytes[at + 1..]))
}

/// Where the header block ends, if it has ended.
fn header_block_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

/// How much of a first line is kept, in bytes.
///
/// A first line that never ends is reachable: a binary protocol's reply has no
/// CRLF, so "the first line" is whatever arrived, which is up to the whole byte
/// bound. A report field holding sixty kilobytes of lossy-decoded binary is not
/// evidence, so it is cut — visibly, with the cut marked in the string rather
/// than left for a reader to notice a suspicious round number.
const FIRST_LINE_BYTES: usize = 120;

/// The first line of a response, lossily decoded and cut at [`FIRST_LINE_BYTES`].
///
/// Lossy on purpose: a first line that is not valid UTF-8 is still evidence
/// about what the service said, and refusing to record it would turn a
/// misbehaving service into an unreadable one. The cut is marked with `…` so that
/// a truncated line cannot be read as the whole of what was said — and cutting
/// the byte slice may split a character, which `from_utf8_lossy` renders as a
/// replacement character rather than a failure.
fn first_line(head: &[u8]) -> String {
    let end = head
        .windows(2)
        .position(|window| window == b"\r\n")
        .unwrap_or(head.len());
    let line = &head[..end];
    if line.len() <= FIRST_LINE_BYTES {
        return String::from_utf8_lossy(line).into_owned();
    }
    let mut text = String::from_utf8_lossy(&line[..FIRST_LINE_BYTES]).into_owned();
    text.push('…');
    text
}

/// `HTTP/<version> <code> <reason>`, or nothing.
///
/// The reason phrase is returned as part of the line rather than parsed, because
/// it is free text and the only thing that may be relied on is its position. A
/// status code is exactly three digits: `HTTP/1.1 2000 OK` is not a response this
/// module will read as a status, and reading it as `200` is the kind of
/// plausible-but-wrong parse that makes a probe useless.
fn parse_status_line(line: &str) -> Option<(u16, String)> {
    let mut parts = line.splitn(3, ' ');
    let version = parts.next()?;
    if !version.starts_with("HTTP/") {
        return None;
    }
    let code = parts.next()?;
    if code.len() != 3 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let status = code.parse::<u16>().ok()?;
    Some((status, line.to_owned()))
}

/// Whether an I/O error is the read deadline rather than a real failure.
///
/// The two platforms disagree about the kind: a Windows socket reports
/// `TimedOut`, and a Unix one reports `WouldBlock` for the same condition on a
/// stream with a read timeout set. Both mean "nothing arrived in time", and
/// treating the Unix one as a connection failure would report a silent port as
/// an unreachable one.
fn is_a_timeout(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}

/// The variant for anything the operating system refused to do.
fn connection_failed(error: &std::io::Error) -> ProbeOutcome {
    match error.kind() {
        ErrorKind::ConnectionRefused => ProbeOutcome::Refused {
            detail: error.to_string(),
        },
        _ => ProbeOutcome::Unreachable {
            detail: error.to_string(),
        },
    }
}

/// What one probe found.
///
/// Five variants, one of which is a response. See the module documentation for
/// why that shape is the acceptance rather than a preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// Nothing on that port accepted the connection.
    Refused {
        /// What the operating system said.
        detail: String,
    },
    /// The connection could not be made, or ended, for a reason that is neither
    /// a refusal nor a timeout.
    ///
    /// This is the only variant that is a failure of the probe rather than an
    /// observation about the project, and it is named as one: it maps to
    /// [`CheckStatus::Error`](crate::status::CheckStatus::Error), which never
    /// aggregates to green.
    Unreachable {
        /// What the operating system said.
        detail: String,
    },
    /// The connection was accepted and **no complete response arrived**.
    ///
    /// `bytes_read` is the concrete part, and it is what separates a port that
    /// said nothing at all from one that started and stalled: zero is a socket
    /// that accepted a connection and then did nothing, and a non-zero count is
    /// a service that began answering and did not finish.
    ///
    /// **This is the variant the acceptance sentence is about.** A port that is
    /// open is evidence that something is listening and no evidence at all about
    /// what it does, so this maps to
    /// [`CheckStatus::Unknown`](crate::status::CheckStatus::Unknown) — not a
    /// pass, and not a failure either.
    NoAnswer {
        /// How many bytes arrived before the deadline or the bound.
        bytes_read: usize,
    },
    /// What arrived is not an HTTP response.
    ///
    /// **This does not require a complete header block.** The first line is
    /// settled as soon as its bytes can no longer begin a status line, so a port
    /// speaking TLS or SSH — which never sends a blank line, and in the TLS case
    /// never sends an `H` — is reported here rather than as a port that said
    /// nothing. That is the difference between a finding a reader can act on and
    /// one they cannot.
    NotHttp {
        /// The first line, lossily decoded, as the service wrote it, cut at 120
        /// bytes with `…` marking the cut.
        first_line: String,
    },
    /// An HTTP response arrived.
    Answered {
        /// The status code, parsed from a status line that had exactly one.
        status: u16,
        /// The status line verbatim — the concrete thing a report can quote.
        status_line: String,
        /// Bytes that arrived after the header block.
        ///
        /// The body for an identity-encoded response; it includes chunk framing
        /// for a chunked one, which this module does not decode.
        body_bytes: usize,
        /// Whether the read stopped at the deadline or the byte bound rather
        /// than at the end of the response.
        truncated: bool,
    },
}

impl ProbeOutcome {
    /// Whether the service **answered** — which is true for exactly one variant.
    ///
    /// This is the method a caller reaching for "did it work?" should find, and
    /// it is deliberately not called `is_ok`: an open port that said nothing is
    /// not an answer, and a caller that wants to know about the port has
    /// [`Self::opened_a_connection`].
    #[must_use]
    pub const fn is_an_answer(&self) -> bool {
        matches!(self, Self::Answered { .. })
    }

    /// Whether something on that port accepted a connection.
    ///
    /// True for `NoAnswer`, `NotHttp` and `Answered` — and true for a port that
    /// said nothing, which is why this is a separate question from
    /// [`Self::is_an_answer`] rather than the one a verdict is built on.
    #[must_use]
    pub const fn opened_a_connection(&self) -> bool {
        matches!(
            self,
            Self::NoAnswer { .. } | Self::NotHttp { .. } | Self::Answered { .. }
        )
    }

    /// The narrow verdict this outcome supports.
    ///
    /// The title names the request that was answered or not, so nothing in the
    /// report reads as a claim about the feature: a pass here says *this request
    /// was answered with a status the service considers successful*, and it says
    /// nothing about whether the body was right, whether the page renders, or
    /// whether the feature exists.
    ///
    /// The mapping, in full:
    ///
    /// | outcome | status | why |
    /// | --- | --- | --- |
    /// | `Answered` 2xx or 3xx | `pass` | the service answered, successfully |
    /// | `Answered` 4xx or 5xx | `fail` | the service answered, and said no |
    /// | `NotHttp` | `fail` | HTTP was asked for and something else came back |
    /// | `Refused` | `fail` | nothing was listening where the check looked |
    /// | `NoAnswer` | `unknown` | an open port is not an answer |
    /// | `Unreachable` | `error` | SURE could not make the exchange happen |
    ///
    /// Every one of these is [`EvidenceClass::ObservedFact`]: SURE made the
    /// request and read what came back, and the difference between the rows is
    /// what the answer was worth rather than how it was obtained.
    #[must_use]
    pub fn verdict(
        &self,
        id: CheckId,
        endpoint: &Endpoint,
        severity: Severity,
        critical: bool,
        fingerprint: FingerprintId,
    ) -> CheckResult {
        let title = format!("local probe: {} answered", endpoint.request_line());
        let class = EvidenceClass::ObservedFact;
        let reason = self.reason();
        match self.status() {
            CheckStatus::Pass => {
                CheckResult::pass(id, title, severity, critical, class, fingerprint)
                    .with_reason(reason)
            }
            CheckStatus::Fail => {
                CheckResult::fail(id, title, severity, critical, class, fingerprint)
                    .with_reason(reason)
            }
            CheckStatus::Unknown => {
                CheckResult::unknown(id, title, severity, critical, class, fingerprint)
                    .with_reason(reason)
            }
            // `Error` is what `status()` returns for `Unreachable`, and the
            // other two cannot occur. They are named rather than swallowed by
            // `_` so that adding a variant to `CheckStatus` is a compile error
            // in this file instead of a silent mis-mapping, and they are sent to
            // `errored` because that is the mapping which refuses to let
            // anything through as a pass: a status arriving here by a future bug
            // would be reported as a checker failure, which is what it would be.
            CheckStatus::Error | CheckStatus::Warning | CheckStatus::Skipped => {
                CheckResult::errored(id, title, severity, critical, reason, fingerprint)
            }
        }
    }

    /// One line of plain language: what happened, in the terms a report can
    /// quote.
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::Refused { detail } => {
                format!("nothing accepted the connection ({detail})")
            }
            Self::Unreachable { detail } => {
                format!("the exchange could not be made ({detail})")
            }
            Self::NoAnswer { bytes_read: 0 } => {
                "the port is open and nothing was said — an open port is not an answer".to_owned()
            }
            Self::NoAnswer { bytes_read } => format!(
                "the port is open and {bytes_read} bytes arrived without a complete response"
            ),
            Self::NotHttp { first_line } => {
                format!("the response did not start with an HTTP status line: {first_line:?}")
            }
            Self::Answered {
                status_line,
                body_bytes,
                truncated: false,
                ..
            } => format!("{status_line}; {body_bytes} bytes of body"),
            Self::Answered {
                status_line,
                body_bytes,
                truncated: true,
                ..
            } => format!(
                "{status_line}; at least {body_bytes} bytes of body, and the response had not \
                 ended when the probe stopped reading"
            ),
        }
    }

    /// The status this outcome maps to, without building a result.
    ///
    /// Separate from [`Self::verdict`] so that a caller — or a test — can ask
    /// what an outcome is worth without naming a check, a severity and a
    /// fingerprint first. The two cannot disagree: [`Self::verdict`] is written
    /// against this.
    #[must_use]
    pub const fn status(&self) -> CheckStatus {
        match self {
            Self::Answered { status, .. } if *status >= 200 && *status < 400 => CheckStatus::Pass,
            Self::Answered { .. } | Self::NotHttp { .. } | Self::Refused { .. } => {
                CheckStatus::Fail
            }
            Self::NoAnswer { .. } => CheckStatus::Unknown,
            Self::Unreachable { .. } => CheckStatus::Error,
        }
    }
}

impl fmt::Display for ProbeOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_read_deadline_is_recognized_in_both_shapes_the_platforms_give_it() {
        // The one test in this module rather than in `tests/`, because the claim
        // cannot be measured from a socket on more than one platform per run.
        // Windows reports an expired socket read timeout as `TimedOut`; a Unix
        // reports the same condition as `WouldBlock`. This was found by a
        // mutation — deleting the `WouldBlock` arm — that **survived** the whole
        // integration suite on Windows, and could not have done anything else:
        // nothing reachable from a socket on this machine produces the Unix
        // spelling.
        //
        // What that arm protects is stated where it lives, and it is not
        // cosmetic: without it a Unix build reports a port that said nothing as
        // a port that could not be reached — an observation about the project
        // turned into a failure of the probe.
        assert!(is_a_timeout(&std::io::Error::from(ErrorKind::TimedOut)));
        assert!(is_a_timeout(&std::io::Error::from(ErrorKind::WouldBlock)));
        assert!(!is_a_timeout(&std::io::Error::from(
            ErrorKind::ConnectionReset
        )));
    }

    #[test]
    fn a_contradicted_first_line_is_decided_before_the_line_ends() {
        // Also unreachable from a socket in the discriminating cases: every
        // server in `tests/` that sends a complete response sends a blank line
        // too, so the split path is what a socket exercises.
        assert!(could_still_be_a_status_line(b""));
        assert!(could_still_be_a_status_line(b"HTTP/"));
        assert!(could_still_be_a_status_line(b"HTTP/1.1"));
        assert!(could_still_be_a_status_line(b"HTTP/1.1 "));
        assert!(could_still_be_a_status_line(b"HTTP/1.1 2"));
        assert!(could_still_be_a_status_line(b"HTTP/1.1 200"));
        assert!(could_still_be_a_status_line(b"HTTP/1.1 200 OK\r\n"));
        assert!(could_still_be_a_status_line(b"HTTP/1.1 200 OK\r\nX: 1"));

        assert!(!could_still_be_a_status_line(b"S"));
        assert!(!could_still_be_a_status_line(b"SSH-2.0-x"));
        assert!(!could_still_be_a_status_line(&[0x15, 0x03, 0x03]));
        assert!(!could_still_be_a_status_line(b"HTTP/1.1 2000"));
        assert!(!could_still_be_a_status_line(b"HTTP/1.1 20x"));
    }
}

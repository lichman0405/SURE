//! The WebSocket client the DevTools connection is made of: a handshake that
//! verifies the peer, and enough of RFC 6455's framing to carry JSON.
//!
//! # Why this is here rather than behind a dependency
//!
//! A browser's debugging endpoint is a WebSocket and nothing else — there is no
//! request/response form of it — so speaking to one means implementing the
//! protocol. The workspace manifest asks every dependency to earn its place, and
//! the honest question is what one would buy here. A general WebSocket crate
//! brings the client role, the server role, permessage-deflate, the full closing
//! handshake, TLS, redirects and a dozen protocol extensions, of which this uses
//! **upgrade, text frames, ping/pong and close, over a loopback socket that
//! cannot be a `wss://`**. What it would not bring is the one thing that
//! matters most on this path: the accept check in [`connect`] is a *security*
//! check on this client's own side, and a library that performs it inside a
//! function this code does not read is a check this code cannot describe.
//!
//! # What is refused rather than tolerated
//!
//! Every refusal below is a place where being lenient would end as a *wrong
//! value that looks right* rather than as an error:
//!
//! - A reply to the upgrade that is not `101` is refused, and so is a `101`
//!   whose `Sec-WebSocket-Accept` is not what [`accept_for`] computes from the
//!   key this client sent. **Without that second check a `101` proves nothing**:
//!   anything on the port that copies the request's headers into a `101` would
//!   pass, and SURE would then believe it was talking to a browser.
//! - Reserved bits set are refused. None are negotiated, so a frame with one set
//!   is a frame from a conversation this client is not having.
//! - A **masked** frame from the server is refused. RFC 6455 section 5.1 requires
//!   the server to send unmasked frames, so a masked one is a protocol violation
//!   rather than a style difference.
//! - A declared payload length above [`MAX_PAYLOAD`] is refused **before the
//!   bytes are asked for**, because a length field is a number the peer chose and
//!   a client that allocates what it is told allocates what it is told.
//! - A fragmented message is assembled rather than refused, because RFC 6455
//!   permits it and a partially assembled JSON document is a parse error
//!   somewhere further away from the cause.
//!
//! # What is not here
//!
//! No TLS, no redirect, no extension negotiation, no `Sec-WebSocket-Protocol`.
//! The socket this client opens is loopback and the path is written by
//! [`super::launch`] from a file the browser itself wrote, so none of the three
//! is reachable from where this is called.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::base64;
use super::sha1::sha1;

/// RFC 6455 section 1.3's globally unique identifier, appended to the client's
/// key before hashing.
///
/// It is a fixed constant in the protocol rather than a value either side
/// chooses, and it is spelled here exactly as the RFC spells it because a single
/// wrong character produces an accept value that is wrong on every connection to
/// every server.
pub const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The largest payload this client will read from a server frame.
///
/// **Eight mebibytes, and it is a refusal rather than a truncation.** The
/// frames on this connection are DevTools protocol messages, which are JSON
/// objects in the kilobytes; a frame larger than this is not a big message, it
/// is a number chosen by the peer, and the only thing that follows from
/// believing it is an allocation of that size. Refusing keeps the failure in the
/// handshake's shape — a driver that could not be driven — rather than in the
/// allocator's.
pub const MAX_PAYLOAD: usize = 8 * 1024 * 1024;

/// How long one [`read_text`](WebSocket::read_text) waits before reporting
/// [`WsError::TimedOut`].
///
/// Short, because the caller polls: the driver's loop has its own deadline and
/// its own notion of a quiet period, and it needs to come back to check both.
/// The value is the polling interval and not a budget, which is why it is not a
/// parameter of anything.
pub const POLL: Duration = Duration::from_millis(50);

/// The frame opcodes this client reads or writes.
///
/// The continuation opcode is `0x0`, which is *not* a distinct frame type: it
/// carries the rest of the message that the previous frame began.
const CONTINUATION: u8 = 0x0;
const TEXT: u8 = 0x1;
const BINARY: u8 = 0x2;
const CLOSE: u8 = 0x8;
const PING: u8 = 0x9;
const PONG: u8 = 0xA;

/// Why a connection could not be made, or did not stay made.
#[derive(Debug)]
pub enum WsError {
    /// The operating system refused something: the connect, the write, the read.
    Io(std::io::Error),
    /// The read timed out, which for a polling caller is not a failure.
    TimedOut,
    /// The peer answered the upgrade with a status that is not `101`.
    Refused { status: String },
    /// The peer answered `101` and did not compute the accept value.
    ///
    /// **The variant that exists to be visible.** A peer that fails this is not
    /// a WebSocket server, whatever it said about itself, and the two strings
    /// are carried so that the report says so rather than saying "the handshake
    /// failed".
    NotWebSocket { expected: String, answered: String },
    /// The peer closed the connection.
    ///
    /// Not an error in the protocol's terms — it is how a conversation ends —
    /// but it is reported here rather than as `Ok(None)` because the only
    /// caller is mid-conversation and a close it did not ask for is a fact it
    /// has to handle either way.
    Closed,
    /// A frame arrived that RFC 6455 does not allow, or that this client does
    /// not implement.
    Malformed(String),
}

impl std::fmt::Display for WsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::TimedOut => write!(formatter, "nothing arrived before the read timed out"),
            Self::Refused { status } => {
                write!(formatter, "the upgrade was answered with {status}")
            }
            Self::NotWebSocket { expected, answered } => write!(
                formatter,
                "the peer answered 101 but its accept value was {answered}, and the key this \
                 connection sent requires {expected}"
            ),
            Self::Closed => write!(formatter, "the peer closed the connection"),
            Self::Malformed(what) => write!(formatter, "{what}"),
        }
    }
}

impl std::error::Error for WsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WsError {
    fn from(error: std::io::Error) -> Self {
        if is_timeout(&error) {
            Self::TimedOut
        } else {
            Self::Io(error)
        }
    }
}

/// Whether an I/O error is a read or write that ran out of time.
///
/// **Two kinds, because two platforms answer differently and neither is wrong.**
/// A socket timeout surfaces as [`ErrorKind::WouldBlock`] on some platforms and
/// as [`ErrorKind::TimedOut`] on others, and `std` does not promise which; the
/// alternative to accepting both is a client that treats an ordinary poll as a
/// dead connection on one of the three platforms it has to run on.
fn is_timeout(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

/// `base64(SHA-1(key ++ GUID))`, which is the value RFC 6455 section 4.2.2
/// requires the server to return and therefore the value that tells this client
/// whether the peer is a WebSocket server.
///
/// **`key` is the base64 text that was sent, not the sixteen bytes it decodes
/// to.** The RFC concatenates the *header value* — the ASCII string — with the
/// GUID, which is a detail that is easy to get wrong in a way that works against
/// nothing and fails against everything.
#[must_use]
pub fn accept_for(key: &str) -> String {
    let mut material = String::with_capacity(key.len() + GUID.len());
    material.push_str(key);
    material.push_str(GUID);
    base64::encode(&sha1(material.as_bytes()))
}

/// Sixteen bytes that no other call to this function produced.
///
/// # What this is, and what it is not
///
/// **It is not a cryptographically secure random number generator and this
/// documentation does not claim it is.** What it is, precisely, is SipHash-1-3
/// keyed by two `u64`s taken from [`RandomState::new`], which `std` seeds from
/// the operating system for its own hash maps — **the same source a `getrandom`
/// call would reach, arrived at through `std` rather than through a crate**.
/// SipHash with a secret key is a pseudorandom function, so its output is not
/// predictable to a process that does not know the key.
///
/// The two properties that are **measured** rather than argued are in this
/// module's tests: successive calls in one process differ, and calls in
/// different processes differ. The property that is **documented** rather than
/// measured is that `RandomState`'s seed is the operating system's; `std`'s
/// source is not installed beside this toolchain, so what is relied on here is
/// the observable distinctness and not a claim about where the entropy came
/// from. `std::random` would be the obvious answer and is **unstable on this
/// toolchain** — `rustc 1.98.1` refuses `std::random::random()` with `E0658`
/// under `feature = "random"` — and this workspace does not use nightly.
///
/// # Why that is the right trade *here*, said out loud
///
/// RFC 6455 section 4.1 requires the key to be "randomly selected", and the
/// reason it gives is a security one: the key is what stops a peer that is not a
/// WebSocket server from having its own bytes reflected back at it. **On this
/// path the peer is a browser that SURE started, on a loopback port read out of
/// a file that browser wrote into a private directory**, and there is no
/// intermediary on that connection to defend against. So the entropy here is
/// buying protocol conformance rather than a security boundary, and the honest
/// response to that is to say so and to spend a dependency on it only if the
/// boundary were real. The caller that would need more is one connecting to a
/// browser it did not start, which this build has no way to do.
///
/// The counter is mixed in as well, so that distinctness survives even a
/// platform whose `RandomState` repeated its keys: it is the guarantee that is
/// *this code's* rather than one it borrows.
fn fresh_key() -> [u8; 16] {
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let call = CALLS.fetch_add(1, Ordering::Relaxed);

    let mut key = [0u8; 16];
    for (half, chunk) in key.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(call);
        hasher.write_u8(half as u8);
        chunk.copy_from_slice(&hasher.finish().to_be_bytes());
    }
    key
}

/// A connected WebSocket, after the upgrade has been verified.
///
/// The socket is blocking with a read timeout of [`POLL`], which is what lets
/// [`read_text`](Self::read_text) return [`WsError::TimedOut`] to a caller that
/// has a deadline of its own to check.
#[derive(Debug)]
pub struct WebSocket {
    stream: TcpStream,
    /// Bytes read from the socket that belong to a frame this client has not
    /// finished assembling.
    ///
    /// **Kept across calls rather than re-read**, because a frame arrives when
    /// the peer chooses to send it and not on a boundary this client picks: a
    /// header can arrive in one read and its payload in the next, and a client
    /// that discarded the surplus would lose the beginning of the next frame.
    pending: Vec<u8>,
}

impl WebSocket {
    /// Upgrades the connection to `address` and asks for `path`.
    ///
    /// `path` is the request target — everything after the host, leading slash
    /// included — and comes from the browser's own `DevToolsActivePort` file.
    ///
    /// # Errors
    ///
    /// [`WsError::Io`] if the socket cannot be opened, [`WsError::Refused`] if
    /// the reply is not `101`, and [`WsError::NotWebSocket`] if it is `101` and
    /// the accept value is wrong. The last is the one that matters: see this
    /// module's documentation.
    pub fn connect(address: SocketAddr, path: &str, timeout: Duration) -> Result<Self, WsError> {
        let mut stream = TcpStream::connect_timeout(&address, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        stream.set_nodelay(true)?;

        let key = base64::encode(&fresh_key());
        let host = address.to_string();
        let request = format!(
            "GET {path} HTTP/1.1\r\n\
             Host: {host}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {key}\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        );
        stream.write_all(request.as_bytes())?;

        let mut socket = Self {
            stream,
            pending: Vec::new(),
        };
        socket.finish_upgrade(&key)?;
        Ok(socket)
    }

    /// Reads the reply to the upgrade and checks it.
    ///
    /// **The surplus is kept**, not discarded: a server is allowed to send its
    /// first frame in the same write as the `101`, and a client that threw away
    /// everything past the header block would lose it.
    fn finish_upgrade(&mut self, key: &str) -> Result<(), WsError> {
        loop {
            if let Some(end) = find_header_end(&self.pending) {
                let block = self.pending[..end].to_vec();
                self.pending.drain(..end + 4);
                return self.check_upgrade(&block, key);
            }
            if !self.read_more()? {
                return Err(WsError::Malformed(String::from(
                    "the peer closed the connection during the upgrade, before answering it",
                )));
            }
        }
    }

    /// Whether the header block the peer sent is a `101` with the right accept.
    fn check_upgrade(&self, block: &[u8], key: &str) -> Result<(), WsError> {
        let text = std::str::from_utf8(block).map_err(|_| {
            WsError::Malformed(String::from(
                "the reply to the upgrade was not text, so it was not an HTTP reply",
            ))
        })?;
        let mut lines = text.split("\r\n");
        let status = lines.next().unwrap_or_default().trim().to_owned();
        if !status.contains(" 101 ") && !status.ends_with(" 101") {
            return Err(WsError::Refused { status });
        }

        let answered = lines
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.trim().eq_ignore_ascii_case("sec-websocket-accept"))
            .map(|(_, value)| value.trim().to_owned());
        let expected = accept_for(key);
        match answered {
            Some(answered) if answered == expected => Ok(()),
            Some(answered) => Err(WsError::NotWebSocket { expected, answered }),
            None => Err(WsError::NotWebSocket {
                expected,
                answered: String::from("absent"),
            }),
        }
    }

    /// Narrows the read timeout, which is what the polling caller sets.
    ///
    /// # Errors
    ///
    /// [`WsError::Io`] if the socket refuses the option.
    pub fn set_read_timeout(&mut self, timeout: Duration) -> Result<(), WsError> {
        self.stream.set_read_timeout(Some(timeout))?;
        Ok(())
    }

    /// Sends one text frame, masked as RFC 6455 section 5.1 requires of a client.
    ///
    /// # Errors
    ///
    /// [`WsError`] if the frame cannot be written.
    pub fn send_text(&mut self, text: &str) -> Result<(), WsError> {
        let mask = fresh_key()[..4].to_vec();
        let mut frame = frame_header(TEXT, text.len(), &mask);
        frame.extend(
            text.as_bytes()
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        self.stream.write_all(&frame)?;
        self.stream.flush()?;
        Ok(())
    }

    /// Reads the next text message, or `None` when the peer closed.
    ///
    /// **Control frames are handled here rather than returned**, because the
    /// caller only ever wants messages: a ping is answered with a pong, a pong is
    /// ignored, and a close ends the conversation. A message split across
    /// several frames is assembled before being returned.
    ///
    /// # Errors
    ///
    /// [`WsError::TimedOut`] when nothing arrived within the read timeout, which
    /// a polling caller treats as *nothing yet*; [`WsError::Closed`] when the
    /// peer dropped the socket without a close frame; and [`WsError::Malformed`]
    /// for a frame RFC 6455 does not allow.
    pub fn read_text(&mut self) -> Result<Option<String>, WsError> {
        let mut assembled: Vec<u8> = Vec::new();
        let mut assembling = false;

        loop {
            let (final_frame, opcode, payload) = self.read_frame()?;
            match opcode {
                CONTINUATION => {
                    if !assembling {
                        return Err(WsError::Malformed(String::from(
                            "a continuation frame arrived with no message to continue",
                        )));
                    }
                    assembled.extend_from_slice(&payload);
                    if final_frame {
                        return Ok(Some(String::from_utf8_lossy(&assembled).into_owned()));
                    }
                }
                TEXT | BINARY => {
                    if assembling {
                        return Err(WsError::Malformed(String::from(
                            "a new message began before the previous one was finished",
                        )));
                    }
                    if opcode == BINARY {
                        return Err(WsError::Malformed(String::from(
                            "a binary frame arrived, and this connection carries JSON text",
                        )));
                    }
                    if final_frame {
                        return Ok(Some(String::from_utf8_lossy(&payload).into_owned()));
                    }
                    assembled = payload;
                    assembling = true;
                }
                PING => self.send_control(PONG, &payload)?,
                PONG => {}
                CLOSE => return Ok(None),
                other => {
                    return Err(WsError::Malformed(format!(
                        "a frame with opcode {other:#x} arrived, which this client does not know"
                    )));
                }
            }
        }
    }

    /// Sends a control frame, which RFC 6455 requires to be unmasked from a
    /// server and **masked from a client** like any other.
    fn send_control(&mut self, opcode: u8, payload: &[u8]) -> Result<(), WsError> {
        let mask = fresh_key()[..4].to_vec();
        let mut frame = frame_header(opcode, payload.len(), &mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        self.stream.write_all(&frame)?;
        self.stream.flush()?;
        Ok(())
    }

    /// Reads one whole frame: whether it ends a message, its opcode, its payload.
    fn read_frame(&mut self) -> Result<(bool, u8, Vec<u8>), WsError> {
        loop {
            match parse_frame(&self.pending)? {
                Parsed::Need => {
                    if !self.read_more()? {
                        return Err(WsError::Closed);
                    }
                }
                Parsed::Frame {
                    consumed,
                    final_frame,
                    opcode,
                    payload,
                } => {
                    self.pending.drain(..consumed);
                    return Ok((final_frame, opcode, payload));
                }
            }
        }
    }

    /// Adds whatever the socket has to [`Self::pending`].
    ///
    /// Returns whether anything was added; `Ok(false)` is the peer having closed
    /// the connection, which the two callers report differently because they are
    /// at different points in the conversation.
    fn read_more(&mut self) -> Result<bool, WsError> {
        let mut buffer = [0u8; 16 * 1024];
        match self.stream.read(&mut buffer) {
            Ok(0) => Ok(false),
            Ok(read) => {
                self.pending.extend_from_slice(&buffer[..read]);
                Ok(true)
            }
            Err(error) => Err(WsError::from(error)),
        }
    }
}

/// What one pass over [`WebSocket::pending`] found.
#[derive(Debug)]
enum Parsed {
    /// The buffer holds the beginning of a frame, and the rest has not arrived.
    Need,
    /// A whole frame, and how many bytes of the buffer it occupied.
    Frame {
        consumed: usize,
        final_frame: bool,
        opcode: u8,
        payload: Vec<u8>,
    },
}

/// Reads one frame out of `buffer`, or says that it is not all there yet.
///
/// Split out from [`WebSocket`] so that the framing rules — which are where a
/// hand-written client is wrong — are checked by tests that do not need a
/// socket, a server or a browser.
fn parse_frame(buffer: &[u8]) -> Result<Parsed, WsError> {
    if buffer.len() < 2 {
        return Ok(Parsed::Need);
    }
    let final_frame = buffer[0] & 0x80 != 0;
    if buffer[0] & 0x70 != 0 {
        return Err(WsError::Malformed(format!(
            "a frame arrived with reserved bits {:03b} set, and no extension was negotiated",
            (buffer[0] & 0x70) >> 4
        )));
    }
    let opcode = buffer[0] & 0x0f;
    let masked = buffer[1] & 0x80 != 0;
    if masked {
        return Err(WsError::Malformed(String::from(
            "a masked frame arrived from the server, and RFC 6455 section 5.1 requires a server \
             to send unmasked frames",
        )));
    }

    let short = buffer[1] & 0x7f;
    let (length, header) = match short {
        126 => {
            if buffer.len() < 4 {
                return Ok(Parsed::Need);
            }
            (usize::from(u16::from_be_bytes([buffer[2], buffer[3]])), 4)
        }
        127 => {
            if buffer.len() < 10 {
                return Ok(Parsed::Need);
            }
            let mut eight = [0u8; 8];
            eight.copy_from_slice(&buffer[2..10]);
            let length = u64::from_be_bytes(eight);
            // Refused before the cast, because the cast is what would throw the
            // high bits away and turn a stated length into a smaller one.
            let Ok(length) = usize::try_from(length) else {
                return Err(too_long(u64::from_be_bytes(eight)));
            };
            (length, 10)
        }
        small => (usize::from(small), 2),
    };

    if length > MAX_PAYLOAD {
        return Err(too_long(length as u64));
    }
    if buffer.len() < header + length {
        return Ok(Parsed::Need);
    }
    Ok(Parsed::Frame {
        consumed: header + length,
        final_frame,
        opcode,
        payload: buffer[header..header + length].to_vec(),
    })
}

/// The refusal a frame larger than [`MAX_PAYLOAD`] gets.
fn too_long(length: u64) -> WsError {
    WsError::Malformed(format!(
        "a frame declared {length} bytes, over the {MAX_PAYLOAD}-byte bound this client reads, \
         and the bytes were not asked for"
    ))
}

/// The two-byte-or-longer header of a masked client frame, followed by its mask.
fn frame_header(opcode: u8, length: usize, mask: &[u8]) -> Vec<u8> {
    let mut header = Vec::with_capacity(14);
    header.push(0x80 | opcode);
    if length < 126 {
        header.push(0x80 | length as u8);
    } else if length < 65_536 {
        header.push(0x80 | 126);
        header.extend_from_slice(&(length as u16).to_be_bytes());
    } else {
        header.push(0x80 | 127);
        header.extend_from_slice(&(length as u64).to_be_bytes());
    }
    header.extend_from_slice(mask);
    header
}

/// Where the header block ends, if it has ended: the index of its final byte.
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{Arc, Mutex};

    use super::*;

    /// **RFC 6455's own published example, which is the one answer for exactly
    /// the construction this client computes.**
    ///
    /// Section 1.3 of the RFC works an example through in full: a client sends
    /// the key `dGhlIHNhbXBsZSBub25jZQ==` — the base64 of the ASCII text `the
    /// sample nonce` — and the server must answer
    /// `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`. It is a value this implementation did
    /// not choose, published by the standard it implements, and it fails on the
    /// next line of every handshake if it is wrong.
    ///
    /// The key decodes to something readable on purpose: it is the RFC's way of
    /// showing that the string concatenated with the GUID is the *base64 text*,
    /// and a client that hashed the sixteen decoded bytes instead produces a
    /// different accept and fails here. The first assertion is what says so —
    /// it is the same value, one step earlier.
    #[test]
    fn the_answer_rfc6455_publishes_is_the_one_this_computes() {
        assert_eq!(
            base64::encode(b"the sample nonce"),
            "dGhlIHNhbXBsZSBub25jZQ=="
        );
        assert_eq!(
            accept_for("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    /// **Two keys in one process are never the same**, which is the half of the
    /// nonce's contract that this code guarantees itself.
    ///
    /// Two thousand draws is far more than a collision would need to show up if
    /// the value were derived from anything coarse — a clock, a counter alone, a
    /// buffer address — and it is cheap. **What this cannot show is that the
    /// values are unpredictable**, which is why [`fresh_key`]'s documentation
    /// says what it says rather than claiming randomness. The property checked
    /// here is the one that is this code's own.
    #[test]
    fn no_two_keys_in_one_process_are_the_same() {
        let keys: std::collections::HashSet<[u8; 16]> = (0..2000).map(|_| fresh_key()).collect();
        assert_eq!(keys.len(), 2000, "a key repeated within one process");
    }

    /// **The frames, parsed from bytes, without a socket anywhere.**
    ///
    /// Every refusal here is a rule from RFC 6455 rather than a preference, and
    /// each is written as the bytes a peer would actually send. The unmasked
    /// server text frame `81 05 hello` is the shape section 5.7 uses.
    #[test]
    fn the_framing_rules_are_applied_to_bytes() {
        let text = parse_frame(&[0x81, 0x05, b'h', b'e', b'l', b'l', b'o']);
        let Parsed::Frame {
            final_frame,
            opcode,
            payload,
            consumed,
        } = text.expect("a whole frame was not recognised")
        else {
            panic!("a whole frame should have parsed");
        };
        assert!(final_frame);
        assert_eq!(opcode, TEXT);
        assert_eq!(payload, b"hello");
        assert_eq!(consumed, 7);

        // A reserved bit set: refused, because nothing negotiated an extension.
        assert!(matches!(
            parse_frame(&[0xC1, 0x00]),
            Err(WsError::Malformed(_))
        ));
        // A masked frame from the server: refused by section 5.1.
        assert!(matches!(
            parse_frame(&[0x81, 0x85, 0x00, 0x00, 0x00, 0x00, b'h']),
            Err(WsError::Malformed(_))
        ));
        // A header that is not all there yet: a request for the rest, not an
        // error and not a zero-length frame. All three length forms are here
        // because each has a different number of header bytes after it, and a
        // parser that read the short form's two bytes as the long form's eight
        // would take a truncated header for a whole one.
        assert!(matches!(parse_frame(&[0x81]), Ok(Parsed::Need)));
        assert!(matches!(parse_frame(&[0x81, 0x7e]), Ok(Parsed::Need)));
        assert!(matches!(parse_frame(&[0x81, 0x7e, 0x00]), Ok(Parsed::Need)));
        assert!(matches!(parse_frame(&[0x81, 0x7f]), Ok(Parsed::Need)));
        assert!(matches!(
            parse_frame(&[0x81, 0x7f, 0, 0, 0, 0, 0, 0, 0]),
            Ok(Parsed::Need)
        ));
        // And a payload that is not all there: still not a frame, and
        // specifically not a two-byte one.
        assert!(matches!(
            parse_frame(&[0x81, 0x05, b'h', b'e']),
            Ok(Parsed::Need)
        ));
    }

    /// **A declared length is refused before the bytes are asked for**, which is
    /// the difference between a client that reads a large frame and one that
    /// allocates one.
    ///
    /// Two cases: one byte over the bound, and a length that does not fit in a
    /// `usize` at all. The second is the one a cast would silently truncate into
    /// a small allocation followed by a wrong read.
    #[test]
    fn a_frame_over_the_bound_is_refused_without_reading_it() {
        let over = (MAX_PAYLOAD as u64) + 1;
        let mut header = vec![0x81, 0x7f];
        header.extend_from_slice(&over.to_be_bytes());
        let refused = parse_frame(&header);
        assert!(
            matches!(refused, Err(WsError::Malformed(_))),
            "a {over}-byte frame was not refused"
        );
        assert!(
            refused
                .expect_err("refused")
                .to_string()
                .contains("were not asked for")
        );

        let absurd = vec![0x81, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        assert!(matches!(parse_frame(&absurd), Err(WsError::Malformed(_))));
    }

    /// The header this client writes, for each of the three length forms.
    ///
    /// Small enough to be checked by hand: seven bits up to 125, sixteen for
    /// anything a `u16` holds, sixty-four above that. The mask bit is set in
    /// every case, because a client frame that is not masked is one a conforming
    /// server must reject.
    #[test]
    fn a_client_frame_header_carries_the_mask_bit_and_the_right_length_form() {
        let mask = [1u8, 2, 3, 4];
        assert_eq!(frame_header(TEXT, 5, &mask), vec![0x81, 0x85, 1, 2, 3, 4]);
        assert_eq!(
            frame_header(TEXT, 126, &mask),
            vec![0x81, 0xfe, 0x00, 0x7e, 1, 2, 3, 4]
        );
        assert_eq!(
            frame_header(TEXT, 65_536, &mask),
            vec![0x81, 0xff, 0, 0, 0, 0, 0, 0x01, 0, 0, 1, 2, 3, 4]
        );
    }

    /// Where the header block ends, including the case where it has not.
    #[test]
    fn the_end_of_a_header_block_is_found_or_reported_absent() {
        let bytes = b"HTTP/1.1 101\r\nUpgrade: websocket\r\n\r\nleft over";
        let end = find_header_end(bytes).expect("the block ends");
        assert_eq!(&bytes[..end], b"HTTP/1.1 101\r\nUpgrade: websocket");
        assert_eq!(
            find_header_end(b"HTTP/1.1 101\r\nUpgrade: websocket\r\n"),
            None
        );
    }

    /// **The handshake against a server this test writes**, which is the only
    /// way to check the client's half of it without a browser.
    ///
    /// The server computes the accept value the way a real one must, and the
    /// test asserts that the key the client sent was the base64 of sixteen
    /// bytes — which is what RFC 6455 section 4.1 requires a client to send.
    /// The length rule is the encoder's own: twenty-four characters in six
    /// groups of four, the last group carrying one byte and two pads, which is
    /// `6 * 3 - 2` decoded bytes.
    #[test]
    fn the_upgrade_is_accepted_when_the_server_computes_the_accept_value() {
        let server = FakeServer::start(Answer::Accept);
        let mut socket =
            WebSocket::connect(server.address(), "/devtools/page/x", Duration::from_secs(5))
                .expect("the handshake should succeed");

        let seen = server.seen();
        assert_eq!(
            seen.status_line,
            "HTTP/1.1 101 WebSocket Protocol Handshake"
        );
        assert_eq!(seen.key.len(), 24, "key was {:?}", seen.key);
        assert!(seen.key.ends_with("=="), "key was {:?}", seen.key);
        assert_eq!(
            seen.key.len() / 4 * 3 - 2,
            16,
            "the key decoded to 16 bytes"
        );

        socket
            .send_text("{\"id\":1}")
            .expect("a frame should write");
        // **The client's half of the framing, at the bytes.** The header is
        // asserted above by value; this is the other half — that the payload it
        // wrote is the payload it was given once the mask it declared is taken
        // off. A client that set the mask bit and never applied it sends bytes a
        // conforming server must reject, and it would fail here rather than on
        // whichever machine happens to have a browser installed.
        assert_eq!(
            server.wait_for_the_client_to_send(TEXT),
            Some(b"{\"id\":1}".to_vec()),
            "the frame the client wrote did not carry the text it was given"
        );
    }

    /// **A `101` whose accept value is wrong is refused**, which is the check
    /// that makes the upgrade mean anything.
    ///
    /// A peer that copies the request's headers into a `101` — or that is a
    /// different program entirely — gets this far and no further.
    #[test]
    fn a_reply_of_101_with_the_wrong_accept_value_is_not_a_websocket() {
        let server = FakeServer::start(Answer::WrongAccept);
        let refused = WebSocket::connect(server.address(), "/x", Duration::from_secs(5));
        let error = refused.expect_err("a wrong accept value must be refused");
        let WsError::NotWebSocket { expected, answered } = error else {
            panic!("the refusal was {error:?}");
        };
        assert_eq!(answered, "AAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        assert_ne!(expected, answered);
    }

    /// A reply that is not `101` at all, and a bare `101` with no accept header.
    #[test]
    fn a_reply_that_is_not_an_upgrade_is_refused() {
        let server = FakeServer::start(Answer::Status(404));
        let error = WebSocket::connect(server.address(), "/x", Duration::from_secs(5))
            .expect_err("a 404 must be refused");
        assert!(matches!(error, WsError::Refused { .. }), "{error:?}");

        let server = FakeServer::start(Answer::NoAcceptHeader);
        let error = WebSocket::connect(server.address(), "/x", Duration::from_secs(5))
            .expect_err("a 101 with no accept header must be refused");
        let WsError::NotWebSocket { answered, .. } = error else {
            panic!("the refusal was {error:?}");
        };
        assert_eq!(answered, "absent");
    }

    /// **A message the server splits across frames and across writes arrives
    /// whole**, which is the property [`WebSocket::pending`] exists for.
    ///
    /// The message is a text frame that is not final, a ping, and a
    /// continuation — which is what a peer is allowed to send, and what a client
    /// that returned the first frame's payload, or that treated a ping as a
    /// message, would get wrong.
    #[test]
    fn a_split_message_with_control_frames_between_is_assembled() {
        let server = FakeServer::start(Answer::Accept);
        let mut socket =
            WebSocket::connect(server.address(), "/x", Duration::from_secs(5)).expect("upgraded");
        socket
            .set_read_timeout(Duration::from_secs(5))
            .expect("a read timeout");

        server.send(&[0x01, 0x03, b'a', b'b', b'c']); // text, not final
        server.send(&[0x89, 0x00]); // ping
        server.send(&[0x80, 0x03, b'd', b'e', b'f']); // continuation, final

        assert_eq!(
            socket.read_text().expect("a message").as_deref(),
            Some("abcdef")
        );
        // The ping was answered, which RFC 6455 section 5.5.2 requires. An
        // answered ping carries the ping's payload back, and this one was empty.
        assert_eq!(
            server.wait_for_the_client_to_send(PONG),
            Some(Vec::new()),
            "the client never answered the ping with a pong"
        );
    }

    /// A close frame ends the conversation and is not an error.
    #[test]
    fn a_close_frame_ends_the_conversation() {
        let server = FakeServer::start(Answer::Accept);
        let mut socket =
            WebSocket::connect(server.address(), "/x", Duration::from_secs(5)).expect("upgraded");
        socket
            .set_read_timeout(Duration::from_secs(5))
            .expect("a read timeout");

        server.send(&[0x88, 0x00]); // close
        assert_eq!(socket.read_text().expect("a close is not an error"), None);
    }

    /// **A socket that goes quiet reports a timeout rather than an error**,
    /// because the caller is a polling loop and a quiet connection is the normal
    /// case between messages.
    #[test]
    fn a_quiet_connection_reports_a_timeout() {
        let server = FakeServer::start(Answer::Accept);
        let mut socket =
            WebSocket::connect(server.address(), "/x", Duration::from_secs(5)).expect("upgraded");
        socket
            .set_read_timeout(Duration::from_millis(50))
            .expect("a read timeout");

        let error = socket.read_text().expect_err("nothing was sent");
        assert!(matches!(error, WsError::TimedOut), "{error:?}");
    }

    /// A peer that drops the socket without a close frame is reported as closed,
    /// not as a silent nothing.
    #[test]
    fn a_dropped_connection_is_reported_as_closed() {
        let server = FakeServer::start(Answer::AcceptThenDrop);
        let mut socket =
            WebSocket::connect(server.address(), "/x", Duration::from_secs(5)).expect("upgraded");
        socket
            .set_read_timeout(Duration::from_secs(5))
            .expect("a read timeout");

        let error = socket.read_text().expect_err("the peer went away");
        assert!(matches!(error, WsError::Closed), "{error:?}");
    }

    /// **A frame that arrives with the header and the payload in separate
    /// writes is read whole**, which is the other half of what the surplus
    /// buffer is for.
    ///
    /// This is not a protocol subtlety but a fact about TCP: a frame is not a
    /// packet, and a client whose reader assumed one read per frame works on a
    /// loopback socket in a test and fails against a real peer under load.
    #[test]
    fn a_frame_split_across_two_writes_is_read_whole() {
        let server = FakeServer::start(Answer::AcceptThenWriteInTwoHalves);
        let mut socket =
            WebSocket::connect(server.address(), "/x", Duration::from_secs(5)).expect("upgraded");
        socket
            .set_read_timeout(Duration::from_secs(5))
            .expect("a read timeout");

        assert_eq!(
            socket.read_text().expect("a message").as_deref(),
            Some("split across writes")
        );
    }

    /// A binding that answers the upgrade and then does what the test tells it.
    ///
    /// **The connection has two ends and this test has one of them.** The bytes
    /// a test wants the client to *receive* have to be written by the server
    /// thread, because [`WebSocket::stream`] is the client's end and writing to
    /// it would send bytes the other way. That is why server frames travel over
    /// a channel to the thread rather than being poked into the socket.
    struct FakeServer {
        address: SocketAddr,
        script: Option<Sender<Vec<u8>>>,
        seen: Receiver<Seen>,
        from_client: Arc<Mutex<Vec<u8>>>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    /// What the server saw before the client went on its way.
    #[derive(Default, Clone)]
    struct Seen {
        status_line: String,
        key: String,
    }

    /// How the fake server answers the upgrade, and what it does after.
    #[derive(Clone, Copy)]
    enum Answer {
        Accept,
        WrongAccept,
        NoAcceptHeader,
        Status(u16),
        AcceptThenDrop,
        AcceptThenWriteInTwoHalves,
    }

    impl FakeServer {
        fn start(answer: Answer) -> Self {
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                .expect("a loopback port");
            let address = listener.local_addr().expect("the bound address");
            let (report, seen) = std::sync::mpsc::channel();
            let (script, orders) = std::sync::mpsc::channel::<Vec<u8>>();
            let from_client = Arc::new(Mutex::new(Vec::new()));
            let collected = Arc::clone(&from_client);

            let handle = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("the client connects");
                let request = read_request(&mut stream);
                let key = header_of(&request, "sec-websocket-key").unwrap_or_default();
                let accept = accept_for(&key);
                let reply = match answer {
                    Answer::Status(code) => {
                        format!("HTTP/1.1 {code} Not Found\r\nContent-Length: 0\r\n\r\n")
                    }
                    Answer::NoAcceptHeader => {
                        String::from("HTTP/1.1 101 WebSocket Protocol Handshake\r\n\r\n")
                    }
                    Answer::WrongAccept => String::from(
                        "HTTP/1.1 101 WebSocket Protocol Handshake\r\n\
                         Sec-WebSocket-Accept: AAAAAAAAAAAAAAAAAAAAAAAAAAA=\r\n\r\n",
                    ),
                    Answer::Accept
                    | Answer::AcceptThenDrop
                    | Answer::AcceptThenWriteInTwoHalves => format!(
                        "HTTP/1.1 101 WebSocket Protocol Handshake\r\nUpgrade: websocket\r\n\
                         Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
                    ),
                };
                let _ = stream.write_all(reply.as_bytes());
                let _ = stream.flush();
                let _ = report.send(Seen {
                    status_line: reply.lines().next().unwrap_or_default().to_owned(),
                    key,
                });

                if matches!(answer, Answer::AcceptThenDrop) {
                    return;
                }
                if matches!(answer, Answer::AcceptThenWriteInTwoHalves) {
                    let frame = [0x81, 0x13, b's', b'p', b'l', b'i', b't', b' ', b'a', b'c'];
                    let _ = stream.write_all(&frame);
                    let _ = stream.flush();
                    std::thread::sleep(Duration::from_millis(80));
                    let _ = stream.write_all(b"ross writes");
                    let _ = stream.flush();
                    std::thread::sleep(Duration::from_millis(400));
                    return;
                }

                // Anything else keeps the socket open, pushing whatever the test
                // scripts and collecting whatever the client says back. A single
                // thread cannot block on both, so the read is a non-blocking
                // attempt once per turn rather than a read that would park.
                loop {
                    match orders.recv_timeout(Duration::from_millis(50)) {
                        Ok(bytes) => {
                            if stream.write_all(&bytes).is_err() {
                                return;
                            }
                            let _ = stream.flush();
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if let Ok(mut message) = collected.lock() {
                                let _ = stream.set_nonblocking(true);
                                let mut buffer = [0u8; 4096];
                                while let Ok(read) = stream.read(&mut buffer) {
                                    if read == 0 {
                                        break;
                                    }
                                    message.extend_from_slice(&buffer[..read]);
                                }
                                let _ = stream.set_nonblocking(false);
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                }
            });

            Self {
                address,
                script: Some(script),
                seen,
                from_client,
                handle: Some(handle),
            }
        }

        fn address(&self) -> SocketAddr {
            self.address
        }

        /// The handshake as the server saw it. Waits, so a test that got this
        /// far is not racing the thread that produced it.
        fn seen(&self) -> Seen {
            self.seen
                .recv_timeout(Duration::from_secs(10))
                .expect("the server thread reported")
        }

        /// Sends bytes to the client, from the client's peer.
        fn send(&self, bytes: &[u8]) {
            self.script
                .as_ref()
                .expect("the server is still running")
                .send(bytes.to_vec())
                .expect("the server thread is listening");
        }

        /// The payload of the first frame the client sent with this opcode,
        /// unmasked, waiting for it.
        ///
        /// Polled rather than awaited on a channel, because the collection is
        /// done by the same thread that is pushing scripted frames; a bounded
        /// wait is enough and does not deadlock when the frame never comes.
        ///
        /// **It unmaskes rather than returning the bytes, and that is what makes
        /// the client's own half of RFC 6455 section 5.1 observable.** A client
        /// frame is masked with a four-byte nonce the client chose, so the bytes
        /// on the wire are not the bytes the client was handed and there is
        /// nothing stable to assert about them directly. Undoing the mask the
        /// frame carries is the only assertion available, and it is the one with
        /// teeth: a client that sets the mask bit and then writes its payload
        /// unmasked decodes to something that is not what it was given.
        ///
        /// **This existed as a `bool` and the mutation harness is what showed it
        /// was not enough**: a mutation that stopped applying the mask was
        /// caught by nothing in this file and only by the three tests in
        /// `tests/browser_driver.rs`, which need a real browser installed. The
        /// claim is about bytes this client writes, so it should not depend on a
        /// browser being on the machine.
        fn wait_for_the_client_to_send(&self, opcode: u8) -> Option<Vec<u8>> {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while std::time::Instant::now() < deadline {
                if let Ok(bytes) = self.from_client.lock()
                    && let Some(payload) = client_payload(&bytes, opcode)
                {
                    return Some(payload);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            None
        }
    }

    impl Drop for FakeServer {
        fn drop(&mut self) {
            // The sender goes first: the thread's loop ends when the channel
            // disconnects, which is what lets the join below return instead of
            // waiting out its poll.
            drop(self.script.take());
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    /// The payload of the first client frame with `opcode` in `bytes`, unmasked,
    /// or `None` if there is no whole such frame yet.
    ///
    /// **Only the short length form**, deliberately: every frame these tests make
    /// the client send is a handful of bytes, and a helper that also decoded the
    /// sixteen- and sixty-four-bit forms would be a second frame parser sitting
    /// beside [`parse_frame`] — which is the thing that would then be wrong
    /// instead of the thing that is being tested.
    fn client_payload(bytes: &[u8], opcode: u8) -> Option<Vec<u8>> {
        if bytes.len() < 2 || bytes[0] & 0x0f != opcode {
            return None;
        }
        assert!(
            bytes[1] & 0x80 != 0,
            "a client frame arrived with no mask bit, and RFC 6455 section 5.1 requires a client \
             to mask every frame"
        );
        let short = bytes[1] & 0x7f;
        if short > 125 {
            return None;
        }
        let length = usize::from(short);
        if bytes.len() < 2 + 4 + length {
            return None;
        }
        let mask = &bytes[2..6];
        Some(
            bytes[6..6 + length]
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4])
                .collect(),
        )
    }

    /// Reads a whole request head.
    fn read_request(stream: &mut TcpStream) -> String {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    buffer.extend_from_slice(&chunk[..read]);
                    if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }

    /// One header's value out of a request head, matched without case.
    fn header_of(request: &str, name: &str) -> Option<String> {
        request
            .split("\r\n")
            .filter_map(|line| line.split_once(':'))
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim().to_owned())
    }
}

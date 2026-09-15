//! What came of a run.
//!
//! An [`Outcome`] exists for every request the operating system let SURE start,
//! including the ones that ended badly, and that is the point: the output of a
//! process that was stopped at its deadline is the evidence for *why* the
//! deadline passed, and an error would throw it away.
//!
//! # There is no `succeeded()`
//!
//! [`std::process::ExitStatus::success`] answers one question — "was the exit
//! code zero" — and for a timed-out or cancelled run there is no exit code at
//! all, so anything that reduces an outcome to a `bool` has to decide what a
//! non-exit means. Whichever way it decides, the caller loses the difference
//! between "the command failed" and "SURE stopped the command", which are
//! different problems with different fixes: one is the project's, and one is
//! SURE's.
//!
//! So there is no method here that returns a `bool`, and [`Termination`] has to
//! be matched. A caller who genuinely wants "did it exit zero" writes that
//! match, in one line, having seen the other three arms.

use std::ffi::OsString;
use std::time::{Duration, SystemTime};

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    /// The process ran and ended by itself.
    Exited {
        /// The exit code, when there was one.
        ///
        /// `None` on a process the operating system ended with a signal, which
        /// is a Unix fact and has no Windows equivalent. It is `None` rather
        /// than a made-up number, because a fabricated code would be read as
        /// the program's own.
        code: Option<i32>,
    },
    /// The process ran and was stopped because its deadline passed.
    TimedOut {
        /// How much of it was stopped.
        stopped: Stop,
    },
    /// The process ran and was stopped because the caller asked for it.
    Cancelled {
        /// How much of it was stopped.
        stopped: Stop,
    },
    /// The process was never started, because the caller had already asked for
    /// it to stop.
    ///
    /// A separate answer from [`Self::Cancelled`], and the difference is not
    /// bookkeeping. A caller that cancels a `cargo test` and one that cancels a
    /// `service start` care about opposite things: the first cares that it
    /// stopped, the second cares that it never began. Reporting "cancelled"
    /// for both would leave the second unable to tell whether a service is
    /// running.
    CancelledBeforeStart,
}

/// How much of a process tree a stop reached.
///
/// A program SURE starts may start other programs, and stopping only the one
/// SURE holds a handle to leaves the rest running — with the project's files
/// open, or a port bound, or a lock held. That is a difference worth carrying
/// rather than smoothing over, so a stop says which of the two happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// The process and the programs it started were stopped.
    ///
    /// On Windows this is what `taskkill /T /F` reporting success means, and
    /// that is the whole strength of the claim: it is the operating system's
    /// own account of the tree, not a survey SURE made of it.
    WholeTree,
    /// The process itself was stopped, and nothing else was reached.
    ///
    /// Anything it started may still be running. This is what a Unix build
    /// gives in this version, and the module documentation records why: SURE
    /// cannot put the child in its own process group without `unsafe`, which
    /// this workspace forbids, and killing a process group it did not create
    /// would reach SURE's own process.
    ProcessOnly,
}

/// What a process wrote to one of its output streams.
///
/// Two different things can make this less than the whole stream, and they are
/// kept apart because they mean opposite things to a reader: bytes SURE
/// **chose** not to keep, and bytes SURE **could not** read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedOutput {
    bytes: Vec<u8>,
    discarded: u64,
    unfinished: Option<String>,
}

impl CapturedOutput {
    /// A stream with nothing in it, read to the end.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            bytes: Vec::new(),
            discarded: 0,
            unfinished: None,
        }
    }

    pub(crate) fn new(bytes: Vec<u8>, discarded: u64, unfinished: Option<String>) -> Self {
        Self {
            bytes,
            discarded,
            unfinished,
        }
    }

    /// A stream SURE has nothing from, and a reason why.
    ///
    /// Named apart from [`Self::unfinished`], which reads the same field: one
    /// builds the answer and one asks for it, and a type whose constructor and
    /// accessor are spelled the same is one a reader has to check twice.
    pub(crate) fn never_finished(reason: impl Into<String>) -> Self {
        Self {
            bytes: Vec::new(),
            discarded: 0,
            unfinished: Some(reason.into()),
        }
    }

    /// The bytes SURE kept, in the order the process wrote them.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The bytes SURE kept, as text, with anything that is not valid UTF-8
    /// replaced.
    ///
    /// The name says what it does. SURE reads output from programs written by
    /// anyone, on a platform where a path is not required to be text, so
    /// decoding is a decision and this is the lossy one. A caller that needs to
    /// know whether the bytes were text checks with
    /// [`std::str::from_utf8`] on [`Self::bytes`] instead.
    #[must_use]
    pub fn text_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    /// How many bytes the process wrote that SURE did not keep.
    #[must_use]
    pub const fn discarded_bytes(&self) -> u64 {
        self.discarded
    }

    /// Whether SURE kept less than the process wrote.
    ///
    /// True means [`Self::bytes`] is a **beginning**, not a whole. A reader
    /// that treats a truncated stream as the complete output is reading a
    /// partial answer as a full one, which is the shape of failure this
    /// product exists to prevent — so the flag is here to be checked rather
    /// than inferred from a length.
    #[must_use]
    pub const fn was_truncated(&self) -> bool {
        self.discarded > 0
    }

    /// Why SURE stopped reading before the stream ended, when it did.
    ///
    /// `None` means SURE read this stream to end-of-file. It does **not** mean
    /// the bytes are all of them — see [`Self::was_truncated`], which is a
    /// separate question with a separate answer.
    ///
    /// `Some` has two causes, and the text says which: a read from the pipe
    /// failed, or the stream was still open when the runner gave up waiting for
    /// it. The second happens when a program SURE started left another program
    /// running that holds the same pipe — the pipe stays open until that one
    /// exits too, and a stopped process's output would otherwise be a wait with
    /// no end.
    #[must_use]
    pub fn unfinished(&self) -> Option<&str> {
        self.unfinished.as_deref()
    }
}

/// What came of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    program: OsString,
    termination: Termination,
    stdout: CapturedOutput,
    stderr: CapturedOutput,
    started_at: SystemTime,
    took: Duration,
}

impl Outcome {
    pub(crate) fn new(
        program: OsString,
        termination: Termination,
        stdout: CapturedOutput,
        stderr: CapturedOutput,
        started_at: SystemTime,
        took: Duration,
    ) -> Self {
        Self {
            program,
            termination,
            stdout,
            stderr,
            started_at,
            took,
        }
    }

    pub(crate) fn never_started(program: OsString) -> Self {
        Self {
            program,
            termination: Termination::CancelledBeforeStart,
            stdout: CapturedOutput::empty(),
            stderr: CapturedOutput::empty(),
            started_at: SystemTime::now(),
            took: Duration::ZERO,
        }
    }

    /// The program this outcome is about.
    #[must_use]
    pub fn program(&self) -> &std::ffi::OsStr {
        &self.program
    }

    /// How the run ended. Always one of the four; there is no "other".
    #[must_use]
    pub const fn termination(&self) -> Termination {
        self.termination
    }

    /// What the process wrote to standard output, as far as SURE read it.
    #[must_use]
    pub const fn stdout(&self) -> &CapturedOutput {
        &self.stdout
    }

    /// What the process wrote to standard error, as far as SURE read it.
    #[must_use]
    pub const fn stderr(&self) -> &CapturedOutput {
        &self.stderr
    }

    /// When the process was started, by SURE's clock.
    ///
    /// Evidence metadata: a run is a thing that happened at a time, and the
    /// time is part of what a later reader needs to line it up with everything
    /// else that happened.
    #[must_use]
    pub const fn started_at(&self) -> SystemTime {
        self.started_at
    }

    /// How long the run took, from just before the process was started to the
    /// moment it was known to be over.
    ///
    /// For a run that was stopped, this is the time to the stop and not the
    /// time the program would have taken. It is measured either way.
    #[must_use]
    pub const fn took(&self) -> Duration {
        self.took
    }
}

//! Everything a run is made of, and nothing it is not.
//!
//! A [`ProcessRequest`] is a description of one process SURE means to run, with
//! every part of the description named: the program, its arguments, the
//! directory it runs in, what it can see of SURE's environment, how long it has,
//! whether anyone may stop it, and how much of what it says SURE will keep.
//!
//! # There is no command line, and no door that takes one
//!
//! [`ProcessRequest::new`] takes a program and [`ProcessRequest::with_arguments`]
//! takes arguments, one per argument. **Nothing here takes a string to be
//! split.** A caller holding `npm install && npm test` — which is what a
//! README, a `package.json` script or an agent's summary hands over — has a
//! `String` and nowhere in this module to put it. The type has no
//! `from_command_line`, no `shell`, and no field that is a command line.
//!
//! This is a shape rather than a rule the code follows, and it is the same
//! shape `sure_core::documents` uses for the same reason. Splitting shell text
//! into a program and its arguments is *itself* the act of starting a shell,
//! and it is where a path with a space in it stops being one path. Making it
//! unrepresentable is stronger than remembering not to do it, because the code
//! that would do it does not exist to be called.
//!
//! # What is not here
//!
//! **There is no default for any of it.** [`Limits`] has no `Default`, and a
//! request cannot be built without stating a timeout, an output bound, and a
//! cancellation. A default would be a value nobody chose, chosen on the day the
//! caller was least thinking about it — and for a timeout, the default nobody
//! wants is the one that never fires.
//!
//! **There is no shell.** See above. What that does and does not cover is
//! recorded at [`ProcessError::NotStarted`](super::ProcessError::NotStarted):
//! SURE builds no command line, and Windows starts a command interpreter anyway
//! for a `.cmd` or a `.bat` that a caller names.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// A request to cancel a run, shared with the caller that may make it.
///
/// One type rather than a pair, because the caller's half and the runner's half
/// are the same fact: this is a handle to "somebody has asked for this to stop"
/// and it is [`Clone`], so the caller keeps one and the request holds another.
/// Setting it through either clone is visible through the other, which is what
/// [`Arc`] is for.
///
/// ```no_run
/// # use sure_core::process::{Cancellation, Limits, ProcessRequest, run};
/// # use std::path::PathBuf;
/// # use std::time::Duration;
/// let stop = Cancellation::new();
/// let request = ProcessRequest::new(
///     "cargo",
///     PathBuf::from("/somewhere"),
///     Limits::new(Duration::from_secs(30), 64 * 1024, 64 * 1024),
///     stop.clone(),
/// )
/// .with_arguments(["test"]);
///
/// // From another thread, when the user asks SURE to stop:
/// stop.cancel();
/// # let _ = (request, run);
/// ```
///
/// # What this is not
///
/// **It is cooperative in one direction only.** The runner notices a
/// cancellation and stops the process; nothing here interrupts a process that
/// is not being watched. A request that is being run is cancelled; a request
/// that has finished is not.
///
/// **A request whose clone the caller dropped cannot be cancelled.** The runner
/// holds the only handle and will never set it, so the run simply has no
/// cancellation — and the runner cannot tell that apart from a caller who is
/// holding theirs and has not used it. This is the one part of the "explicit"
/// rule that a type cannot carry, so it is said here instead.
#[derive(Debug, Clone, Default)]
pub struct Cancellation {
    cancelled: Arc<AtomicBool>,
}

impl Cancellation {
    /// A cancellation nobody has asked for yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Ask the run to stop.
    ///
    /// Returns immediately, and says nothing about whether the run noticed.
    /// There is no answer to give at this point: the run may be mid-spawn, may
    /// be about to finish on its own, or may have finished already. What
    /// happened is in the [`Outcome`](super::Outcome) the runner returns.
    ///
    /// Setting it twice is the same as setting it once.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Whether a stop has been asked for.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// The bounds a run is held to.
///
/// All three are required by [`Limits::new`], and there is no `Default`. That
/// is deliberate: a run with no deadline is a check that hangs, and a check
/// that hangs is indistinguishable from one that is still working — which is
/// the failure `docs/architecture/FINGERPRINTING.md` names in those words about
/// a test that had to be moved onto a worker thread to stop it hanging.
///
/// # The output bounds do not stop the process writing
///
/// A bound is on what SURE **keeps**, not on what the process may write. The
/// runner goes on reading past the bound and throws the rest away, because the
/// alternative — closing the pipe — makes the process fail on a broken pipe for
/// the crime of being talkative, and reports the failure as though the command
/// were at fault.
///
/// So a program that writes a gigabyte takes as long as it takes to write a
/// gigabyte. What the bound buys is that SURE holds 64 KiB of it in memory
/// rather than all of it, and that
/// [`CapturedOutput::was_truncated`](super::CapturedOutput::was_truncated) is
/// true, so nothing downstream reads a beginning as though it were the whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    timeout: Duration,
    stdout_bytes: usize,
    stderr_bytes: usize,
}

impl Limits {
    /// Bounds for one run: how long it has, and how much of each stream SURE
    /// keeps.
    ///
    /// A stream with a bound of zero is still read to the end and still
    /// counted; none of it is kept. That is a way to say "this one does not
    /// matter", and it is a different request from not capturing the stream —
    /// which this module does not offer, because a stream nobody is reading is
    /// a pipe that fills and a process that stops.
    #[must_use]
    pub const fn new(timeout: Duration, stdout_bytes: usize, stderr_bytes: usize) -> Self {
        Self {
            timeout,
            stdout_bytes,
            stderr_bytes,
        }
    }

    /// How long the process has before it is stopped.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// How many bytes of standard output SURE keeps.
    #[must_use]
    pub const fn stdout_bytes(&self) -> usize {
        self.stdout_bytes
    }

    /// How many bytes of standard error SURE keeps.
    #[must_use]
    pub const fn stderr_bytes(&self) -> usize {
        self.stderr_bytes
    }
}

/// What the child sees of SURE's own environment.
///
/// The project being checked is untrusted by assumption, and a variable SURE
/// carries for its own reasons is a fact about the machine that the project did
/// not ask for. So there is a choice, it is named at the call site, and there
/// is no default.
///
/// # Neither variant is a sandbox, and this is worth being plain about
///
/// [`Environment::Only`] with an empty list does not confine the process. A
/// child can still open any file the user can open, reach the network, and read
/// this machine's other state, because an environment is a list of strings
/// passed to a program and not a boundary around it. What the choice decides is
/// what the program **starts with**, which matters for reproducibility and for
/// not handing secrets to code that has no business having them — and neither
/// of those is isolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    /// SURE's environment is not passed on at all. The child gets exactly this
    /// list.
    Only(Vec<(OsString, OsString)>),
    /// SURE's environment is passed on, less the named variables, plus these.
    Inherited {
        /// Variable names the child does not get, even though SURE has them.
        without: Vec<OsString>,
        /// Variables set for the child, which replace SURE's own if the names
        /// collide.
        with: Vec<(OsString, OsString)>,
    },
}

impl Environment {
    /// Nothing is passed on: the child gets exactly these variables.
    #[must_use]
    pub fn only<I>(variables: I) -> Self
    where
        I: IntoIterator<Item = (OsString, OsString)>,
    {
        Self::Only(variables.into_iter().collect())
    }

    /// Everything SURE has is passed on, unchanged.
    ///
    /// This is what a program needs when it has to find other programs by name:
    /// on Windows `CreateProcess` searches `PATH` for the program it starts, so
    /// a child started with no environment cannot start anything either.
    #[must_use]
    pub fn inherited() -> Self {
        Self::Inherited {
            without: Vec::new(),
            with: Vec::new(),
        }
    }

    /// Keep `name` from the child, even though SURE has it.
    ///
    /// Does nothing to [`Environment::Only`], which has nothing to remove.
    #[must_use]
    pub fn without(mut self, name: impl Into<OsString>) -> Self {
        if let Self::Inherited { without, .. } = &mut self {
            without.push(name.into());
        }
        self
    }

    /// Set `name` for the child, replacing SURE's own value if it has one.
    #[must_use]
    pub fn with(mut self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        let pair = (name.into(), value.into());
        match &mut self {
            Self::Only(variables) => variables.push(pair),
            Self::Inherited { with, .. } => with.push(pair),
        }
        self
    }
}

/// A process SURE means to run.
///
/// Built by [`ProcessRequest::new`] and [`ProcessRequest::with_arguments`], and
/// read by [`run`](super::run). The accessors exist so that a caller — or a
/// test, or the check plan `P3-T005` will build — can read back exactly what is
/// about to be run without running it.
/// # There is no `PartialEq`, and its absence is a decision
///
/// Everything a request describes could be compared — same program, same
/// arguments, same directory, same environment, same bounds — except the one
/// field that is a **live handle** rather than a value. A cancellation is a
/// shared flag, and two requests with identical commands and different
/// cancellations are not the same request in any sense a caller means.
///
/// So the choice was between comparing the flag's identity, which would make
/// `a == b` false for two structurally identical requests and read as "these
/// are different commands", and not offering `==` at all. The second is the
/// honest one. A caller that wants to know whether two requests would run the
/// same command reads the accessors and says so itself, in the open, rather
/// than getting an answer that silently depended on something else.
#[derive(Debug, Clone)]
pub struct ProcessRequest {
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
    environment: Environment,
    limits: Limits,
    cancellation: Cancellation,
}

impl ProcessRequest {
    /// A request to run `program` in `working_directory`, held to `limits`, and
    /// stoppable through `cancellation`.
    ///
    /// `program` is **one executable, not a command line**. It is looked for
    /// the way the operating system looks for a program: a name containing a
    /// separator is a path and is not searched for, and a name with no extension
    /// is completed with `.exe` on Windows and looked for as it stands on the
    /// platforms that mark a program with an execute bit instead. So
    /// `"npm install"` is not a program and will not be found as one, and a bare
    /// `npm` is not found either: the completion stops at `.exe`, and the file
    /// on a machine with npm installed is `npm.cmd`. Naming that file is a
    /// different question with a different answer, and
    /// [`ProcessError::NotStarted`] has both — including the shell Windows
    /// starts for a batch file, which this module neither asks for nor prevents.
    ///
    /// No arguments, and SURE's whole environment is passed on. Both are the
    /// choices that change nothing, which is what makes them the right starting
    /// point for [`Self::with_arguments`] and [`Self::with_environment`] to
    /// change.
    ///
    /// The working directory is checked when the request is run, not here: a
    /// request describes something that is going to happen, and a directory can
    /// be removed between the describing and the doing.
    #[must_use]
    pub fn new(
        program: impl Into<OsString>,
        working_directory: impl Into<PathBuf>,
        limits: Limits,
        cancellation: Cancellation,
    ) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
            working_directory: working_directory.into(),
            environment: Environment::inherited(),
            limits,
            cancellation,
        }
    }

    /// The arguments, in order, one element per argument.
    #[must_use]
    pub fn with_arguments<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments = arguments.into_iter().map(Into::into).collect();
        self
    }

    /// What the child sees of SURE's environment.
    #[must_use]
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    /// The program, as it will be handed to the operating system.
    #[must_use]
    pub fn program(&self) -> &OsStr {
        &self.program
    }

    /// The arguments, as they will be handed to the operating system.
    #[must_use]
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    /// The directory the program will run in.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// What the child will see of SURE's environment.
    #[must_use]
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// The bounds the run will be held to.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// The cancellation this run will watch.
    #[must_use]
    pub fn cancellation(&self) -> &Cancellation {
        &self.cancellation
    }

    /// The operating-system command this request describes.
    ///
    /// The one place a [`Command`] is built from a request, so that what the
    /// accessors report and what actually runs cannot drift apart.
    ///
    /// Three things are set here that a request has no field for, because there
    /// is no version of them a caller would want to choose:
    ///
    /// - **Stdin is null.** Nobody is there to answer a question. A program
    ///   that stops to ask one would otherwise wait for input that is never
    ///   coming, which is a run that hangs rather than a run that fails. This is
    ///   the same reasoning `fingerprint/git` records for `GIT_TERMINAL_PROMPT`.
    /// - **Both output streams are piped**, because a bound on what SURE keeps
    ///   is only meaningful if SURE is the one reading.
    /// - **Nothing is written to a shell.** See the module documentation: there
    ///   is no string anywhere in this path that gets split.
    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        // One element per argument. `args` does not join them and does not
        // quote them; the operating system receives the vector.
        command.args(&self.arguments);
        command.current_dir(&self.working_directory);
        match &self.environment {
            Environment::Only(variables) => {
                command.env_clear();
                command.envs(
                    variables
                        .iter()
                        .map(|(name, value)| (name.as_os_str(), value.as_os_str())),
                );
            }
            Environment::Inherited { without, with } => {
                for name in without {
                    command.env_remove(name);
                }
                command.envs(
                    with.iter()
                        .map(|(name, value)| (name.as_os_str(), value.as_os_str())),
                );
            }
        }
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command
    }
}

//! Why a process could not be run.
//!
//! Every one of these means **the process did not run**, and none of them means
//! "here is what it printed before something went wrong". That distinction is
//! the whole reason this type exists: a caller that receives output has to be
//! able to trust that it is *all* the output of a process that *finished*, and
//! a run that could not be started at all is a different fact from a run that
//! started and was stopped.
//!
//! The second fact is not an error. A process that ran and was stopped at its
//! deadline is [`Outcome`](super::Outcome) with
//! [`Termination::TimedOut`](super::Termination::TimedOut) — it has output, and
//! the output is the evidence for why the deadline passed. Turning that into an
//! error would throw away the only thing that explains it.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// Why a process could not be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    /// The working directory was given as a relative path.
    ///
    /// The same rule as [`crate::scan::ScanError::NotAbsolute`], and for the
    /// same reason. A child started with a relative working directory is
    /// started in whatever directory *SURE* happened to be in, so the same
    /// request would run a different command depending on where it was made —
    /// and a command that reads or writes files would do it somewhere else.
    ///
    /// There is no repair for this in the runner. Resolving it against the
    /// process's own directory is exactly the behaviour being refused.
    WorkingDirectoryNotAbsolute {
        /// The working directory as it was given.
        working_directory: PathBuf,
    },
    /// There is nothing at the working directory, or what is there is not a
    /// directory.
    ///
    /// Checked before the process is started, rather than left to the operating
    /// system, because the error the operating system gives for it is the same
    /// one it gives for a missing program. A caller told "the program was not
    /// found" would go looking for an installation problem when the working
    /// directory was the thing that moved.
    ///
    /// Both cases are one variant because they are one fact to a caller: this
    /// is not somewhere a program can be run. `message` says which it was —
    /// the operating system's own words when there is nothing there, and SURE's
    /// when what is there is a file.
    WorkingDirectoryUnusable {
        /// The working directory as it was given.
        working_directory: PathBuf,
        /// Why it cannot be run in.
        message: String,
    },
    /// The program could not be started.
    ///
    /// # On Windows, this is what a missing program looks like
    ///
    /// [`std::process::Command`] reaches `CreateProcess`, which completes a name
    /// that has **no** extension with `.exe` and nothing else. So `npm`, `yarn`,
    /// `pnpm` and `gradlew` are not found on a machine where all four are
    /// installed: none of them is a `.exe`, and the search never tries `.cmd` or
    /// `.bat`. The message will be the operating system's "the system cannot
    /// find the file specified", because there is no `npm.exe` — and that is
    /// also why the search stops there even when `npm.cmd` is on `PATH`.
    ///
    /// **The other side of it is not what "no shell" would lead a reader to
    /// expect, so it is written down rather than left to be discovered**: a
    /// `.cmd` or a `.bat` named *with its extension* does start, and the process
    /// that runs is `cmd.exe` — Windows starts a command interpreter for a batch
    /// file itself. A `.ps1` does not start at all: `CreateProcess` starts
    /// images, and a script is not one, so what comes back is "not a valid Win32
    /// application" rather than "not found". All of that is measured, and
    /// `tests/process_runner.rs` holds each case as a test.
    ///
    /// None of it is repaired here, and it is not a defect to be worked around
    /// inside this module. A name with no extension is the operating system's
    /// rule and a batch file's interpreter is the operating system's doing;
    /// SURE's part is that it never *builds* a command line for either — a
    /// caller names a file, or there is nothing to run. Whether a caller may
    /// name a batch file is a question about running project-controlled shell
    /// text, and the module documentation records whose it is.
    NotStarted {
        /// The program SURE tried to run.
        program: OsString,
        /// The directory it was to be run in.
        working_directory: PathBuf,
        /// What the operating system said.
        message: String,
    },
    /// The process was started and could not be followed.
    ///
    /// [`std::process::Child::try_wait`] or [`std::process::Child::wait`]
    /// failed, which means the operating system would not say what happened to
    /// a process SURE started. Nothing is known about its output or its exit,
    /// so nothing is reported as though something were.
    CouldNotBeWatched {
        /// The program SURE was running.
        program: OsString,
        /// What the operating system said.
        message: String,
    },
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkingDirectoryNotAbsolute { working_directory } => write!(
                f,
                "SURE was asked to run a program in \"{}\", which is not a full path.\n\n\
                 A short path would be resolved against whatever directory SURE happened to be \
                 started in, so the same request would run the program against a different \
                 directory depending on where it was made — and a program that reads or writes \
                 files would read and write somewhere else.\n\n\
                 Give the whole path to the directory the program should run in.",
                working_directory.display()
            ),
            Self::WorkingDirectoryUnusable {
                working_directory,
                message,
            } => write!(
                f,
                "SURE was asked to run a program in \"{}\", and that is not somewhere a program \
                 can run.\n\n\
                 {message}\n\n\
                 The directory is checked before the program is started, because the error the \
                 operating system gives for a missing directory is the same one it gives for a \
                 missing program. Without this check, a directory that had moved would be \
                 reported as a program that was not installed.\n\n\
                 Check that the directory exists, or point SURE at the directory it is in now.",
                working_directory.display()
            ),
            Self::NotStarted {
                program,
                working_directory,
                message,
            } => write!(
                f,
                "SURE could not start \"{}\" in \"{}\".\n\n\
                 The operating system said: {message}\n\n\
                 SURE runs programs directly and builds no command line of its own, so the program \
                 has to be one the operating system starts by itself. On Windows a name with no \
                 extension is completed with `.exe` and nothing else: `npm` is not found even where \
                 `npm.cmd` is installed, and a `.ps1` cannot be started at all. If the file is \
                 really a `.cmd` or a `.bat`, naming it with its extension does start it — Windows \
                 supplies the command interpreter for a batch file — and SURE neither asks for that \
                 nor prevents it.\n\n\
                 Check that the program is installed and that the path to it is right, and see the \
                 module documentation for `sure_core::process`.",
                program.to_string_lossy(),
                working_directory.display()
            ),
            Self::CouldNotBeWatched { program, message } => write!(
                f,
                "SURE started \"{}\" and then could not find out what happened to it.\n\n\
                 The operating system said: {message}\n\n\
                 Nothing is reported about this run: not its output, and not how it ended. SURE \
                 could not follow the process, so it does not know whether it finished, and \
                 anything it said about that would be a guess.",
                program.to_string_lossy()
            ),
        }
    }
}

impl std::error::Error for ProcessError {}

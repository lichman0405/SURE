//! Stopping a process that did not stop.
//!
//! A request carries a deadline and a cancellation, and both of them promise
//! the same thing: when the answer is "stop", the process stops. Everything in
//! this file is what makes that promise true rather than intended.
//!
//! # Why this is not `Child::kill`
//!
//! [`std::process::Child::kill`] stops one process: the one SURE holds a handle
//! to. A program SURE started may have started others — a build that runs a
//! compiler, a test command that runs a runner, a `sh -c` that runs two
//! commands — and those are not reached by it. They keep the project's files
//! open, keep its port bound, keep its lock held, and go on writing to a pipe
//! nobody is reading. Stopping the parent and reporting "stopped" would be a
//! claim SURE cannot hold, so what was actually reached is reported as
//! [`Stop`](super::Stop).
//!
//! # What each platform gets
//!
//! **Windows** reaches the whole tree, through `taskkill /T /F`, which is the
//! operating system's own way of saying "this process and everything it
//! started". This is the platform `CLAUDE.md` calls the primary one, and the
//! one where process-tree cancellation is a first-class requirement.
//!
//! **Everything else** reaches the process itself and nothing below it, and
//! reports [`Stop::ProcessOnly`](super::Stop::ProcessOnly) so that no caller
//! can read a narrower stop as a wider one. The reason is not a preference:
//! stopping a process *group* is how this is done on Unix, the group has to be
//! created when the child is started, and creating it means `setpgid` — an
//! `unsafe` call in a workspace that forbids `unsafe` in every crate. Doing it
//! to a group SURE did not create is worse than not doing it, because SURE is
//! in that group too.

use std::process::Child;

use super::outcome::Stop;

/// Stop `child`, reaching as much of what it started as this platform allows.
///
/// The process itself is always killed, whether or not the wider attempt
/// worked: a tree kill that failed must not leave SURE's own child running
/// while it reports that something was stopped.
///
/// # Errors
///
/// Whatever [`Child::wait`] returns. A process that could not be reaped is one
/// whose end SURE does not know, and the caller turns that into
/// [`ProcessError::CouldNotBeWatched`](super::ProcessError::CouldNotBeWatched)
/// rather than reporting a stop it cannot confirm.
pub(crate) fn stop(child: &mut Child) -> std::io::Result<Stop> {
    let whole_tree = reaches_the_whole_tree(child);
    // Ignore the error: by the time this runs the process may already be gone,
    // which is the outcome this call was after. The `wait` below is what
    // establishes the truth, and it is the one allowed to fail.
    let _ = child.kill();
    child.wait()?;
    Ok(if whole_tree {
        Stop::WholeTree
    } else {
        Stop::ProcessOnly
    })
}

/// Whether this platform can stop a whole process tree, and whether it did.
///
/// # `taskkill` is the one program this module runs, and it is not bounded
///
/// It is started with [`Stdio::null`](std::process::Stdio::null) on all three
/// streams so it can neither block on input nor put its own output into streams
/// a caller is reading, and it is found through **SURE's own** environment
/// rather than the child's — a request that clears the environment for its
/// child does not affect how SURE starts `taskkill` itself.
///
/// What it is *not* is bounded by a deadline of its own. If `taskkill` hung,
/// the stop would hang with it. Bounding it would mean running it through this
/// module, which would mean a runner that calls itself to stop itself, and the
/// added shape is worse than the risk: `taskkill` is a local program that
/// terminates processes, and the case where it hangs is the case where the
/// operating system is already not answering. Recorded rather than guarded.
#[cfg(windows)]
fn reaches_the_whole_tree(child: &Child) -> bool {
    use std::process::{Command, Stdio};

    Command::new("taskkill")
        // The process and everything it started.
        .arg("/T")
        // End it, rather than asking it to close. A console program is sent
        // `WM_CLOSE` without this, and a program that ignores it — which is
        // most of them, and every program with no window — is not stopped at
        // all. A deadline that can be declined is not a deadline.
        .arg("/F")
        .arg("/PID")
        .arg(child.id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Unix reaches the process and reports that it did.
///
/// See the module documentation for why, and for what it would take to reach
/// further. The `false` here is the report: it does not mean the attempt
/// failed, it means no attempt was made, and the caller turns it into
/// [`Stop::ProcessOnly`](super::Stop::ProcessOnly).
#[cfg(not(windows))]
fn reaches_the_whole_tree(_child: &Child) -> bool {
    false
}

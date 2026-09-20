#![forbid(unsafe_code)]
//! A program at a path, put there without ever being written there.
//!
//! A test that writes a file and then asks the operating system to run it has
//! one ordering to keep, and it is not the one that reads naturally: the file
//! at the **executed path** must be complete and closed before anything can
//! `execve` it. Writing the bytes straight to that path keeps a write
//! descriptor open on the very inode that the file is about to be.
//!
//! # The rule this rests on
//!
//! `ETXTBSY` — *Text file busy*, `os error 26` — is raised at `execve` when the
//! **inode** being executed is open for writing **by any process**, not merely
//! by the caller. `fork` duplicates the whole descriptor table into the child,
//! which is how a write descriptor held by one thread's copy comes to be
//! inherited by another thread's about-to-`execve` child; `O_CLOEXEC`, which
//! Rust sets on everything it opens, closes that descriptor at the child's own
//! `execve` — the instant **after** the window that matters, so it does not
//! help. The same errno is raised in the other direction too: `open` with
//! `O_WRONLY|O_CREAT|O_TRUNC` is refused when the inode it would truncate is,
//! at that moment, the text of a running process.
//!
//! # What these functions do about it
//!
//! Both write to a name that nothing can be executing — a **unique temporary
//! name in the destination's own directory**, claimed with `create_new` so two
//! callers cannot be handed the same one — close it, and only then `rename` it
//! onto the path the caller is about to execute. The descriptor an in-flight
//! write holds names the temporary file, and the temporary file is a name the
//! executing side never computes; the rename publishes a file that is already
//! complete and already closed. `rename` within one directory is atomic, so a
//! reader — including a process being started — sees either no file or the
//! whole of the finished one.
//!
//! # What it does not do
//!
//! It does not make a **shared** destination safe against being overwritten
//! while another process is running it. Writing to a unique name and renaming
//! closes the `execve` direction; the write direction is closed by the
//! destination being unique per caller rather than fixed, which is the
//! convention the rest of the tree keeps (`sure-cli`'s `a_store_of_our_own`
//! makes its directory unique with `create_dir` for the same reason). These
//! functions are the second half of that, not a substitute for the first —
//! and where a caller renames onto a path that already exists, whether that
//! rename succeeds depends on the platform and on whether the existing file is
//! running, which is why every caller here renames onto a path it has just
//! made.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// How many temporary names to try before giving up.
///
/// One is the ordinary case. A name that is already taken is a leftover from a
/// run whose process id has been recycled — the case `runtime_start.rs`'s
/// fixture documents at length — and the next counter value is another name.
const NAMES: u32 = 1_000;

/// Write `contents` at `path`, then make `path` the name of that file.
///
/// `mode` is the permission the program needs where the platform has permission
/// bits — `0o755` for a script the operating system is asked to start, `0o644`
/// for the control case that has to be *not* startable. On Windows it is
/// ignored, because there is no bit to set and a name is run by its extension.
///
/// # Errors
///
/// The error from creating, writing or renaming. A failure leaves `path` as it
/// was — untouched, or absent — and the temporary file is removed rather than
/// left behind.
pub fn write_program(path: &Path, contents: &[u8], mode: u32) -> io::Result<()> {
    let (temporary, mut file) = a_name_of_its_own(path)?;
    let written = file
        .write_all(contents)
        .and_then(|()| file.flush())
        .and_then(|()| set_mode(&file, mode));
    // **Closed here**, before the rename and before any path exists that
    // anything could be asked to execute.
    drop(file);
    match written {
        Ok(()) => place(&temporary, path),
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// Copy the file at `from` — its bytes and its permission bits — to `path`.
///
/// The bits are the reason this is not [`write_program`] with the bytes read
/// first: `fs::copy` carries the source's mode across, and the fixtures that
/// copy the test binary and then run it are executable *because* of that.
///
/// # Errors
///
/// The error from copying or renaming. A failure leaves `path` as it was.
pub fn copy_program(from: &Path, to: &Path) -> io::Result<u64> {
    let (temporary, claim) = a_name_of_its_own(to)?;
    // `fs::copy` opens the destination itself, so the claim is released and the
    // name kept: it is still a name no other caller here can be handed.
    drop(claim);
    match fs::copy(from, &temporary) {
        Ok(copied) => {
            place(&temporary, to)?;
            Ok(copied)
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// Rename `temporary` onto `path`, removing the temporary file if the rename
/// does not happen.
fn place(temporary: &Path, path: &Path) -> io::Result<()> {
    match fs::rename(temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(temporary);
            Err(error)
        }
    }
}

/// A name in `path`'s own directory that is claimed, by `create_new`, to be
/// this call's — and the open file that proves it.
///
/// `create_new` is `O_CREAT|O_EXCL`, so the claim is the same kind of atomic
/// one `create_dir` makes, and for the same reason: a name that is already
/// taken is skipped rather than adopted.
fn a_name_of_its_own(path: &Path) -> io::Result<(PathBuf, File)> {
    static NEXT: AtomicU32 = AtomicU32::new(0);

    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no directory to write in", path.display()),
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no file name to write under", path.display()),
        )
    })?;

    for _ in 0..NAMES {
        // The destination's directory rather than a temporary directory of its
        // own: a rename is only atomic within one file system, and the name has
        // to be one the executing side never computes. `OsString` throughout,
        // so a program whose name is not text stays the name it was.
        let mut candidate = OsString::from(".");
        candidate.push(name);
        candidate.push(format!(
            ".being-written-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let candidate = directory.join(candidate);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "{NAMES} temporary names in a row beside {} were all taken, so this call has no \
             name of its own to write under",
            path.display()
        ),
    ))
}

/// Give `file` the mode `mode`, where the platform has one.
#[cfg(unix)]
fn set_mode(file: &File, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(mode))
}

/// Windows has no mode bit to set: a program is a name with an extension.
#[cfg(not(unix))]
fn set_mode(_file: &File, _mode: u32) -> io::Result<()> {
    Ok(())
}

//! Fingerprinting a project that is under version control, from what Git says.
//!
//! `docs/architecture/RUST_DESIGN.md` asks for one abstraction through which
//! system Git is invoked, and this is it: [`Git`] is the only type in the
//! workspace that starts Git, and [`Git::STATUS_ARGUMENTS`] is read by a test
//! that fails if a second place starts it. One place is not tidiness — it is
//! what makes "how does SURE ask Git?" a question with one answer, and the
//! answer is a list of flags with a reason beside each.
//!
//! # What the fingerprint covers, and why that is the whole design
//!
//! **A file is part of the fingerprint if and only if a check could read it.**
//!
//! That sentence decides every case that would otherwise be a judgement call.
//! A check reads the files the scan found, so the fingerprint covers the files
//! the scan would find — the same ignore tables, applied by the same comparison
//! ([`crate::scan::ignore::left_out`]). It is why a tracked file under
//! `target/` is not in the fingerprint: the walk never looks inside `target/`,
//! so no check can be affected by what changes there. It is why SURE's own
//! `.sure` directory is not in it even when a project commits it — a fingerprint
//! over SURE's own output would change when SURE ran, so checking a project
//! would change the thing being checked.
//!
//! And it is why the two halves are collected differently:
//!
//! - **Tracked paths** come from Git, which knows which ones differ from HEAD.
//!   SURE does not have to read the repository to find out, which is the whole
//!   reason a Git project gets a cheaper fingerprint than a content manifest.
//! - **Untracked paths** come from Git too, but a path Git has never been told
//!   about is a path whose relevance is a question — `node_modules` that nobody
//!   ignored would otherwise put the contents of a dependency tree into the
//!   fingerprint and mark good evidence stale every time it was reinstalled.
//!
//! # It reads the files, rather than hashing Git's diff
//!
//! The cheaper design is to hash the text of `git diff`, and it is wrong in two
//! ways that are hard to see. The text depends on configuration and on Git's
//! version — `diff.algorithm`, `.gitattributes` filters, rename similarity
//! thresholds — so the same project state can produce two different digests, and
//! a fingerprint that changes when nothing did is a fingerprint nobody trusts.
//! And a diff describes a change *from HEAD*, which says nothing about a file
//! the repository does not track.
//!
//! So SURE reads the bytes of the files Git named. Git decides *which* files
//! matter; the bytes decide *what* is in them.
//!
//! # What it does not do
//!
//! **It does not follow a link.** A link is recorded as the path it points at,
//! which is the whole of what a link is, and reading through one would read
//! whatever it names — including outside the project. This is the one place the
//! fingerprint is deliberately wider than the scan, which records
//! [`SkipReason::NotFollowed`] and moves on: a tracked link is part of the
//! repository whether or not a check reads through it, and a change to one going
//! unnoticed is the worse of the two failures.
//!
//! **It does not ask about `.gitignore`.** Git's answer to "what is untracked"
//! already accounts for it; the ignore tables are a second and different
//! question, and both apply.
//!
//! **It reads Git's word for which tracked files changed.** A file marked
//! `assume-unchanged` or `skip-worktree` is reported as unchanged, and SURE
//! believes it. See `docs/architecture/FINGERPRINTING.md`, which records that
//! and the other known gaps.
//!
//! [`SkipReason::NotFollowed`]: crate::scan::SkipReason::NotFollowed

pub mod status;

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sure_domain::vocabulary::{GitState, ProjectFingerprint};

use crate::scan::{EntryKind, ScanError, ignore, scan};

use super::FingerprintOptions;
use super::digest::{self, Digest, HashError};
use super::error::FingerprintError;

/// The name of this kind of fingerprint, and the version of what goes into it.
///
/// It is the first field of every hash computed here, so a fingerprint of this
/// kind can never equal one of another kind, and changing what goes into a
/// fingerprint cannot silently make an old value equal a new one.
const DOMAIN: &str = "sure.git-fingerprint.v1";

/// The one place SURE runs Git.
///
/// Holding the program name rather than calling `Command::new("git")` where it
/// is needed is what lets a test reach the "Git is not installed" answer on a
/// machine where Git is installed — which is otherwise a branch nobody ever
/// runs, on any machine, until the day it matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Git {
    program: OsString,
}

impl Default for Git {
    fn default() -> Self {
        Self::system()
    }
}

impl Git {
    /// How SURE asks Git to describe a working tree.
    ///
    /// Every flag here is about what the answer depends on, and none of them is
    /// decoration:
    ///
    /// - `status` — the question.
    /// - `--porcelain=v2` — the format Git documents as stable for programs,
    ///   rather than the one that changes with configuration.
    /// - `-z` — no quoting. A path with a space, a quote, a backslash or a
    ///   newline in it arrives as itself. Without this, `core.quotepath` decides
    ///   how a file name is spelled, and SURE would be hashing Git's escaping of
    ///   a name rather than the name.
    /// - `--branch` — the commit and the branch, in the same output. They are
    ///   part of what a fingerprint is, and asking for them separately would be
    ///   a second chance to read a different moment.
    /// - `--untracked-files=all` — every untracked file, not one line per
    ///   untracked directory. `normal` would name a directory and leave its
    ///   contents unaccounted for.
    /// - `--no-renames` — a fingerprint must not depend on a similarity
    ///   heuristic whose threshold is configuration and whose behaviour has
    ///   changed between Git versions. With renames off, the same state arrives
    ///   as a deletion and an addition, which are read just as well, and the
    ///   same state produces the same digest on every Git that honours the flag.
    /// - `-- .` — only this project. A repository root that contains several
    ///   projects is normal, and a fingerprint of one of them must not change
    ///   when a sibling does.
    ///
    /// `--relative` is deliberately **not** here, and it is worth knowing why.
    /// It looks like exactly the right flag: paths relative to the current
    /// directory and changes outside it excluded, which is what `-- .` plus
    /// [`Git::prefix`] achieve between them. On Git 2.55.0 it produced **no
    /// output at all** for a working tree with changes in it — silently, with a
    /// successful exit — which read as "this project is clean". A fingerprint
    /// over that is the worst failure this product can produce. It was tried,
    /// and it is recorded here so that nobody tries it again on the strength of
    /// the documentation.
    pub const STATUS_ARGUMENTS: &'static [&'static str] = &[
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        "--untracked-files=all",
        "--no-renames",
        "--",
        ".",
    ];

    /// The Git the operating system will run when asked for `git`.
    #[must_use]
    pub fn system() -> Self {
        Self {
            program: OsString::from("git"),
        }
    }

    /// The same, with a different program name.
    ///
    /// Used by the tests to reach the answers that only appear on a machine
    /// without Git. A name that cannot exist is a better fixture than a fake
    /// program: there is no script to keep in step with the real one, and no
    /// shell involved in running it.
    #[must_use]
    pub fn with_program(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
        }
    }

    /// Fingerprint the project at `root`.
    ///
    /// # Errors
    ///
    /// Every [`FingerprintError`] means there is **no** fingerprint. See that
    /// type: a partial fingerprint is a wrong answer wearing the shape of a
    /// right one, and this function does not produce one.
    pub fn fingerprint(
        &self,
        root: &Path,
        options: &FingerprintOptions,
    ) -> Result<ProjectFingerprint, FingerprintError> {
        if !root.is_absolute() {
            return Err(FingerprintError::NotAbsolute {
                root: root.to_path_buf(),
            });
        }

        // The project's own place inside the repository. Asked for rather than
        // derived from `--show-toplevel`, which returns a path whose spelling
        // and letter case need not match the caller's — and comparing two
        // Windows paths as text is how a strip that should succeed fails.
        let prefix = self.prefix(root)?;

        let output = self.stdout(
            root,
            "read the status of this working tree",
            Self::STATUS_ARGUMENTS,
        )?;
        let status = status::parse(&output)?;
        let head = status.head()?.to_owned();
        let mut records = status.records;
        // Sorted by the bytes Git printed, so the digest does not depend on the
        // order Git happened to report things in — which is a property of Git's
        // implementation and not of the project.
        records.sort_by(|a, b| a.bytes.cmp(&b.bytes));

        let mut reader = Reader::new(options);
        let mut tracked = Vec::new();
        let mut untracked = Vec::new();
        for record in &records {
            let relative = relative_to_root(&record.path, &prefix)?;
            let entry = root.join(relative);
            let found = match fs::symlink_metadata(&entry) {
                Ok(metadata) => Some(metadata),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(FingerprintError::Unreadable {
                        path: relative.to_path_buf(),
                        message: error.to_string(),
                    });
                }
            };

            // Which of the two tables the last component is looked up in. The
            // filesystem is asked first because it is the authority on what is
            // there; Git's own fields answer for a path that is gone, which is
            // the case a deletion leaves behind.
            let kind = match &found {
                Some(metadata) if metadata.is_dir() => EntryKind::Directory,
                Some(_) => EntryKind::File,
                None if record.gitlink => EntryKind::Directory,
                None => EntryKind::File,
            };
            if ignore::left_out(relative, kind, options.scan.case).is_some() {
                continue;
            }

            let hashed = reader.read(relative, &entry, found.as_ref())?;
            let record = Hashed {
                bytes: &record.bytes,
                kind: record.kind,
                contents: hashed,
            };
            if record.kind.is_tracked() {
                tracked.push(record);
            } else {
                untracked.push(record);
            }
        }

        let dirty_digest = list_digest("tracked", &tracked);
        let untracked_digest = list_digest("untracked", &untracked);
        let dirty = !tracked.is_empty();

        // The branch is reported and not hashed. Renaming a branch, or giving a
        // detached HEAD a name, changes nothing about the files — and a
        // fingerprint that changed would mark every result from before the
        // rename stale, which teaches a person to ignore staleness.
        let state = GitState {
            head: head.clone(),
            dirty,
            dirty_digest,
            untracked_digest,
            branch: status.branch.clone(),
        };

        let mut digest = Digest::new(DOMAIN);
        digest
            .field("head")
            .field(&head)
            .field("tracked")
            .optional(state.dirty_digest.as_deref())
            .field("untracked")
            .optional(state.untracked_digest.as_deref());

        Ok(ProjectFingerprint::git(digest.finish(), state))
    }

    /// Where this directory sits inside its repository, as a path.
    ///
    /// Empty when the directory *is* the repository root, which is the ordinary
    /// case. Git writes it with `/` and a trailing one.
    fn prefix(&self, root: &Path) -> Result<PathBuf, FingerprintError> {
        let output = match self.stdout(
            root,
            "find the repository this folder is in",
            &["rev-parse", "--show-prefix"],
        ) {
            Ok(output) => output,
            // This is the check that asks whether there is a repository at all,
            // so its failure *is* the answer "there is not one". Reported as the
            // raw Git failure it would be the same message Git prints — but
            // under a heading that says SURE could not describe the repository
            // it found, which is the opposite of what happened and would send a
            // person looking for a broken repository instead of a folder that
            // was never one.
            //
            // A Git that would not start is a different problem and keeps its
            // own answer: installing Git is not the same task as running SURE
            // somewhere else.
            Err(FingerprintError::GitFailed {
                status, message, ..
            }) => {
                return Err(FingerprintError::NoRepository {
                    root: root.to_path_buf(),
                    message: match status {
                        Some(code) => format!("Git exited with code {code}: {message}"),
                        None => message,
                    },
                });
            }
            Err(other) => return Err(other),
        };
        let text = output.strip_suffix(b"\n".as_slice()).unwrap_or(&output);
        // A directory name can contain a newline on every platform SURE runs on,
        // and this output has no `-z` form to disambiguate it. A remaining
        // newline means the prefix is not something SURE can read, and reading
        // it as though it were would put the wrong paths into the digest.
        if text.iter().any(|byte| *byte == b'\n' || *byte == 0) {
            return Err(status::unrecognized(&output));
        }
        let text = text.strip_suffix(b"/".as_slice()).unwrap_or(text);
        status::path_from_git_bytes(text)
    }

    /// Run one Git command inside `root` and return what it wrote to stdout.
    ///
    /// # Errors
    ///
    /// [`FingerprintError::GitUnavailable`] when the program could not be
    /// started at all, and [`FingerprintError::GitFailed`] when it ran and
    /// exited with a failure. The two are different problems with different
    /// fixes, and this is where they are told apart.
    fn stdout(
        &self,
        root: &Path,
        operation: &'static str,
        arguments: &[&str],
    ) -> Result<Vec<u8>, FingerprintError> {
        let output = Command::new(&self.program)
            // Read-only, and it says so. Without this, `status` refreshes the
            // index — writing to the repository SURE was asked to look at, and
            // possibly failing on a read-only checkout or a locked index.
            // Checking a project must not change it.
            .arg("--no-optional-locks")
            // One argument per value, never a command line built as a string. A
            // path with a space or a quote in it is one path, and the way it
            // stops being one is by being pasted into something that gets split
            // again.
            .arg("-C")
            .arg(root)
            .args(arguments)
            // Nobody is there to answer a question. A Git that stops to ask one
            // would hang a check with no output.
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .output()
            .map_err(|error| FingerprintError::GitUnavailable {
                program: self.program.clone(),
                message: error.to_string(),
            })?;

        if !output.status.success() {
            return Err(FingerprintError::GitFailed {
                operation,
                status: output.status.code(),
                message: message_from(&output.stderr),
            });
        }
        Ok(output.stdout)
    }
}

/// Fingerprint the project at `root`, using the Git the operating system finds.
///
/// # Errors
///
/// As [`Git::fingerprint`].
pub fn git_fingerprint(
    root: &Path,
    options: &FingerprintOptions,
) -> Result<ProjectFingerprint, FingerprintError> {
    Git::system().fingerprint(root, options)
}

/// What Git said, as text for a message, cut short if it said a great deal.
fn message_from(stderr: &[u8]) -> String {
    const MOST: usize = 4_000;
    let text = String::from_utf8_lossy(stderr);
    let trimmed = text.trim();
    if trimmed.chars().count() <= MOST {
        return trimmed.to_owned();
    }
    let mut cut: String = trimmed.chars().take(MOST).collect();
    cut.push_str("...");
    cut
}

/// A path relative to the repository root, as a path relative to the project.
///
/// SURE asks Git about one subtree and Git answers with the repository root's
/// view of the world, so the prefix has to come off. It is a `strip_prefix` on
/// paths and not on strings: a repository containing `app` and `app-old` would
/// defeat a string prefix, and `app-old/file.rs` would come out of one as
/// `-old/file.rs` — a file that does not exist, hashed as though it did.
fn relative_to_root<'a>(path: &'a Path, prefix: &Path) -> Result<&'a Path, FingerprintError> {
    if prefix.as_os_str().is_empty() {
        return Ok(path);
    }
    path.strip_prefix(prefix)
        .map_err(|_| FingerprintError::OutsideRoot {
            path: path.to_path_buf(),
        })
}

/// What is at one of the paths Git named.
enum Contents {
    /// A file, and the digest of its bytes.
    File(String),
    /// A directory, and the digest of everything the walk found inside it.
    Tree(String),
    /// A link, and the path it points at.
    ///
    /// The target as text rather than the file it names: reading through a link
    /// is the thing every other part of SURE refuses to do, and a link that
    /// points somewhere else is a different link whatever is at the other end.
    Link(String),
    /// Nothing. Git named a path that is not in the working tree — a deletion,
    /// or a staged change whose file has since gone.
    Gone,
}

impl Contents {
    /// Write this into a digest, with a tag so that no two kinds can collide.
    fn write(&self, digest: &mut Digest) {
        match self {
            Self::File(hex) => {
                digest.field("file").field(hex);
            }
            Self::Tree(hex) => {
                digest.field("tree").field(hex);
            }
            Self::Link(target) => {
                digest.field("link").field(target);
            }
            Self::Gone => {
                digest.field("gone");
            }
        }
    }
}

/// One path Git named, together with what was found at it.
struct Hashed<'a> {
    /// Git's own bytes for the path.
    bytes: &'a [u8],
    /// Which part of the status it came from.
    kind: status::RecordKind,
    /// What is at it.
    contents: Contents,
}

/// Write a list of changes as one digest.
fn list_digest(label: &str, records: &[Hashed<'_>]) -> Option<String> {
    if records.is_empty() {
        // `None` rather than the digest of an empty list, so that "this project
        // is clean" and "this project has an empty change list" are two states
        // and not one. They are the same state here and will not be for ever;
        // the domain type says so with an `Option` and this follows it.
        return None;
    }
    let mut digest = Digest::new(DOMAIN);
    digest.field(label);
    for record in records {
        digest.field(record.bytes).field(record.kind.as_str());
        record.contents.write(&mut digest);
    }
    Some(digest.finish())
}

/// Reads the content of the paths Git named, within the budget it was given.
///
/// The budget is spent here rather than checked afterwards because the point of
/// a limit is not to notice a project that is too big — it is to not read it.
struct Reader<'a> {
    options: &'a FingerprintOptions,
    files: usize,
    bytes: u64,
}

impl<'a> Reader<'a> {
    fn new(options: &'a FingerprintOptions) -> Self {
        Self {
            options,
            files: 0,
            bytes: 0,
        }
    }

    /// What is at `relative`, which Git named and which lives at `entry`.
    fn read(
        &mut self,
        relative: &Path,
        entry: &Path,
        found: Option<&fs::Metadata>,
    ) -> Result<Contents, FingerprintError> {
        let Some(metadata) = found else {
            return Ok(Contents::Gone);
        };
        // A link is answered before anything reads through it. `read_link`
        // returns what the link says, and does not follow it — which is the only
        // kind of reading a link gets anywhere in SURE.
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(entry).map_err(|error| FingerprintError::Unreadable {
                path: relative.to_path_buf(),
                message: error.to_string(),
            })?;
            return Ok(Contents::Link(display_path(&target)));
        }
        if metadata.is_dir() {
            return self.tree(relative, entry);
        }
        self.read_bytes(relative, entry)
    }

    /// A file's digest.
    fn read_bytes(&mut self, relative: &Path, entry: &Path) -> Result<Contents, FingerprintError> {
        self.take_file(relative)?;
        let remaining = self.options.max_bytes.saturating_sub(self.bytes);
        match digest::hash_file(entry, remaining) {
            Ok(hashed) => {
                self.bytes += hashed.bytes;
                Ok(Contents::File(hashed.hex))
            }
            Err(HashError::TooLarge) => Err(FingerprintError::TooManyBytes {
                limit: self.options.max_bytes,
                path: relative.to_path_buf(),
            }),
            Err(HashError::Io(source)) => Err(FingerprintError::Unreadable {
                path: relative.to_path_buf(),
                message: source.to_string(),
            }),
        }
    }

    /// A directory's digest: everything the walk found inside it, with the
    /// walk's own ignore tables applied.
    ///
    /// This is the case a nested repository lands in — a checkout cloned into
    /// the project without being added, which Git reports as one untracked
    /// directory and does not descend into. Its files are files a check can
    /// read, so they are files the fingerprint covers; the alternative is a
    /// fingerprint that calls an old result current when somebody edits them.
    fn tree(&mut self, relative: &Path, entry: &Path) -> Result<Contents, FingerprintError> {
        let walked = scan(entry, self.options.scan).map_err(|error| match error {
            ScanError::Unreadable { message, .. } => FingerprintError::Unreadable {
                path: relative.to_path_buf(),
                message,
            },
            other => FingerprintError::Unreadable {
                path: relative.to_path_buf(),
                message: other.to_string(),
            },
        })?;

        // A walk that lost something is not a walk whose result can be hashed.
        // The files it did not reach are files a check can still read, and
        // leaving them out is how this fingerprint would come to mean less than
        // it appears to.
        if let Some(lost) = walked.losses().next() {
            return Err(FingerprintError::IncompleteTree {
                path: relative.to_path_buf(),
                detail: lost.plain_description(),
            });
        }

        let mut files: Vec<_> = walked.files().collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));

        let mut digest = Digest::new(DOMAIN);
        digest.field("tree");
        for file in files {
            let contents = self.read_bytes(&file.path, &entry.join(&file.path))?;
            // The path is in the digest as well as the contents, so that moving
            // a file and changing nothing in it is a change. Two files swapping
            // names leaves every set of bytes exactly where it was.
            digest.field(display_path(&file.path).as_bytes());
            contents.write(&mut digest);
        }
        Ok(Contents::Tree(digest.finish()))
    }

    /// Spend one file from the budget.
    fn take_file(&mut self, relative: &Path) -> Result<(), FingerprintError> {
        if self.files >= self.options.max_files {
            return Err(FingerprintError::TooManyFiles {
                limit: self.options.max_files,
                path: relative.to_path_buf(),
            });
        }
        self.files += 1;
        Ok(())
    }
}

/// A path with `/` on every platform, for a digest.
///
/// The digest has to be the same value for the same project on Windows, macOS
/// and Linux, and `Display for Path` gives backslashes on Windows. This is the
/// same normalisation `crate::scan` applies to a path it reports, and it is
/// deliberately the same function rather than the same idea: a link target
/// recorded here and a path reported there are the same kind of thing.
fn display_path(path: &Path) -> String {
    crate::scan::display_path(path)
}
